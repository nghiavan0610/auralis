//! Tauri commands for managing the local Python sidecar pipeline
//!
//! Spawns `scripts/local_pipeline.py` as a child process, pipes audio data
//! (PCM s16le) to its stdin, reads JSON results from its stdout, and emits
//! Tauri events to the frontend.

use crate::state::AuralisState;
use auralis::domain::traits::AudioSource;
use once_cell::sync::Lazy;
use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};

// ---------------------------------------------------------------------------
// Global pipeline state
// ---------------------------------------------------------------------------

/// Holds the running child process and its stdin handle so that audio data
/// can be written from a separate task.
struct PipelineState {
    child: Child,
    stdin: std::process::ChildStdin,
}

/// Global singleton so both `start` and `stop` commands can access it.
static PIPELINE: Lazy<Mutex<Option<PipelineState>>> = Lazy::new(|| Mutex::new(None));

// ---------------------------------------------------------------------------
// Logging helper
// ---------------------------------------------------------------------------

fn log(msg: &str) {
    tracing::info!("[commands_pipeline] {}", msg);
}

// ---------------------------------------------------------------------------
// Python executable discovery
// ---------------------------------------------------------------------------

/// Find a suitable Python 3 executable.
/// 1. `~/.config/auralis/mlx-env/bin/python3` (MLX virtual env)
/// 2. `/opt/homebrew/bin/python3` (Homebrew on macOS)
/// 3. Fallback to bare `python3`
fn find_python() -> String {
    if let Some(config_dir) = dirs::config_dir() {
        let venv_python = config_dir
            .join("auralis")
            .join("mlx-env")
            .join("bin")
            .join("python3");
        if venv_python.exists() {
            log(&format!("Using venv python: {:?}", venv_python));
            return venv_python.to_string_lossy().to_string();
        }
    }

    if std::path::Path::new("/opt/homebrew/bin/python3").exists() {
        log("Using Homebrew python3");
        return "/opt/homebrew/bin/python3".to_string();
    }

    log("Using system python3 (fallback)");
    "python3".to_string()
}

// ---------------------------------------------------------------------------
// Pipeline script discovery
// ---------------------------------------------------------------------------

/// Locate `scripts/local_pipeline.py` relative to the crate directory.
fn find_pipeline_script() -> Result<std::path::PathBuf, String> {
    let candidates = vec![
        // Dev: project root (when running from src-tauri/)
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../scripts/local_pipeline.py"),
        // Dev: relative to cwd
        std::path::PathBuf::from("scripts/local_pipeline.py"),
        // macOS .app bundle
        std::env::current_exe()
            .unwrap_or_default()
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("../Resources/scripts/local_pipeline.py"),
    ];

    log(&format!(
        "Searching for pipeline script: {:?}",
        candidates
            .iter()
            .map(|p| format!("{:?} exists={}", p, p.exists()))
            .collect::<Vec<_>>()
    ));

    candidates
        .into_iter()
        .find(|p| p.exists())
        .ok_or_else(|| "Pipeline script not found. Ensure scripts/local_pipeline.py exists.".to_string())
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Start the local (offline) translation pipeline.
///
/// 1. Reads source/target language from settings.
/// 2. Spawns `python3 scripts/local_pipeline.py --source-lang ... --target-lang ...`.
/// 3. Starts audio capture via `MicrophoneCapture`.
/// 4. Drains audio data, converts to PCM s16le, writes to the child's stdin.
/// 5. Reads stdout line-by-line, parses JSON, emits Tauri events:
///    - `pipeline-result`  for `{"type":"result",...}`
///    - `pipeline-status`  for `{"type":"status",...}` and `{"type":"ready"}`
#[tauri::command]
pub async fn start_local_pipeline(
    state: State<'_, AuralisState>,
    app_handle: AppHandle,
) -> Result<(), String> {
    // --- Guard: already running? ---
    {
        let guard = PIPELINE.lock().map_err(|e| e.to_string())?;
        if guard.is_some() {
            return Err("Pipeline is already running".to_string());
        }
    }

    // --- Languages from settings ---
    let source_lang = state.source_language().await;
    let target_lang = state.target_language().await;

    log(&format!(
        "start_local_pipeline: {} -> {}",
        source_lang, target_lang
    ));

    // --- Stop any orphaned processes ---
    let _ = Command::new("pkill")
        .args(["-f", "local_pipeline.py"])
        .output();
    std::thread::sleep(std::time::Duration::from_millis(300));

    // --- Locate script & python ---
    let script_path = find_pipeline_script()?;
    let python = find_python();

    let _ = app_handle.emit(
        "pipeline-status",
        serde_json::json!({"type": "status", "message": "Starting Python pipeline..."}),
    );

    // --- Spawn the Python sidecar ---
    let path_env = "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin";
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());

    let mut child = Command::new(&python)
        .arg(&script_path)
        .arg("--source-lang")
        .arg(&source_lang)
        .arg("--target-lang")
        .arg(&target_lang)
        .env("PATH", path_env)
        .env("HOME", &home)
        .env("TOKENIZERS_PARALLELISM", "false")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start pipeline: {}", e))?;

    log(&format!("Python process spawned, PID={}", child.id()));

    let _ = app_handle.emit(
        "pipeline-status",
        serde_json::json!({
            "type": "status",
            "message": format!("Python started (PID={}), loading models...", child.id())
        }),
    );

    // --- Take handles from the child ---
    let stdout = child.stdout.take().ok_or("Failed to get stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to get stderr")?;
    let stdin_handle = child.stdin.take().ok_or("Failed to get stdin")?;

    // --- Store pipeline state globally ---
    {
        let mut guard = PIPELINE.lock().map_err(|e| e.to_string())?;
        *guard = Some(PipelineState {
            child,
            stdin: stdin_handle,
        });
    }

    // --- Read stdout and emit events ---
    let app_stdout = app_handle.clone();
    std::thread::spawn(move || {
        use std::io::BufRead;
        let reader = std::io::BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(line) if !line.is_empty() => {
                    log(&format!("stdout: {}", &line));

                    // Try to parse as JSON to determine event type
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                        let msg_type = json
                            .get("type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");

                        match msg_type {
                            "result" => {
                                let _ = app_stdout.emit("pipeline-result", &line);
                            }
                            "status" => {
                                let _ = app_stdout.emit("pipeline-status", &line);
                            }
                            "ready" => {
                                let _ = app_stdout.emit(
                                    "pipeline-status",
                                    serde_json::json!({"type":"status","message":"Pipeline ready"}),
                                );
                            }
                            "done" => {
                                log("Received done signal from pipeline");
                                break;
                            }
                            _ => {
                                // Forward unknown types as pipeline-result
                                let _ = app_stdout.emit("pipeline-result", &line);
                            }
                        }
                    } else {
                        // Non-JSON line -- forward as pipeline-result anyway
                        let _ = app_stdout.emit("pipeline-result", &line);
                    }
                }
                Ok(_) => {} // empty line
                Err(e) => {
                    log(&format!("stdout read error: {}", e));
                    break;
                }
            }
        }
        log("stdout reader thread ended");
    });

    // --- Read stderr and forward as status ---
    let app_stderr = app_handle.clone();
    std::thread::spawn(move || {
        use std::io::BufRead;
        let reader = std::io::BufReader::new(stderr);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    log(&format!("stderr: {}", line));
                    let escaped = line.replace('\\', "\\\\").replace('"', "\\\"");
                    let _ = app_stderr.emit(
                        "pipeline-status",
                        serde_json::json!({"type":"status","message": escaped}),
                    );
                }
                Err(_) => break,
            }
        }
        log("stderr reader thread ended");
    });

    // --- Start audio capture ---
    let audio_config = auralis::infrastructure::AudioCaptureConfig::default();
    let mut capture = auralis::infrastructure::MicrophoneCapture::new(audio_config)
        .map_err(|e| format!("Failed to create audio capture: {}", e))?;

    capture
        .start()
        .await
        .map_err(|e| format!("Failed to start audio capture: {}", e))?;

    let audio_data = capture.audio_data();
    let is_recording = capture.is_recording_flag();
    let stream_stop = capture.stream_stop_flag();

    // Signal streaming is active
    state.is_streaming.store(true, Ordering::Relaxed);

    let _ = app_handle.emit(
        "pipeline-status",
        serde_json::json!({"type":"status","message":"Audio capture started"}),
    );

    // --- Audio writer task: drain f32 chunks -> PCM s16le -> pipeline stdin ---
    let is_streaming = state.is_streaming.clone();
    let app_audio = app_handle.clone();

    tokio::spawn(async move {
        // Keep capture alive; when this task ends, capture is dropped.
        let _capture = capture;

        while !stream_stop.load(Ordering::Relaxed) {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;

            let recording = is_recording.lock().unwrap_or_else(|e| e.into_inner());
            if !*recording {
                continue;
            }

            // Drain chunks and convert to PCM s16le
            let pcm_bytes = {
                let mut data = audio_data.lock().unwrap_or_else(|e| e.into_inner());
                let mut pcm = Vec::new();
                for chunk in data.drain(..) {
                    for sample in chunk {
                        let clamped = sample.clamp(-1.0, 1.0);
                        let s16 = (clamped * 32767.0) as i16;
                        pcm.push(s16.to_le_bytes());
                    }
                }
                pcm.concat()
            };

            if pcm_bytes.is_empty() {
                continue;
            }

            // Write PCM to the pipeline's stdin
            let write_result = {
                let mut guard = match PIPELINE.lock() {
                    Ok(g) => g,
                    Err(e) => {
                        log(&format!("Pipeline mutex poisoned: {}", e));
                        break;
                    }
                };
                match guard.as_mut() {
                    Some(ps) => {
                        ps.stdin.write_all(&pcm_bytes).and_then(|_| ps.stdin.flush())
                    }
                    None => {
                        log("Pipeline state is None, stopping audio writer");
                        break;
                    }
                }
            };

            if let Err(e) = write_result {
                log(&format!("stdin write error: {}", e));
                break;
            }
        }

        // Streaming stopped
        is_streaming.store(false, Ordering::Relaxed);

        let _ = app_audio.emit(
            "pipeline-status",
            serde_json::json!({"type":"status","message":"Audio capture stopped"}),
        );
    });

    log("Pipeline started successfully");
    Ok(())
}

/// Stop the local (offline) translation pipeline.
///
/// Closes stdin to signal the Python process to exit, then kills it and
/// stops audio capture.
#[tauri::command]
pub async fn stop_local_pipeline(
    state: State<'_, AuralisState>,
) -> Result<(), String> {
    log("stop_local_pipeline called");

    // Signal the audio stream to stop
    state.stream_stop.store(true, Ordering::Relaxed);

    // Stop the Python process
    {
        let mut guard = PIPELINE.lock().map_err(|e| e.to_string())?;
        if let Some(mut ps) = guard.take() {
            // Drop stdin to signal Python to stop
            drop(ps.stdin);

            // Give it a moment to shut down gracefully
            std::thread::sleep(std::time::Duration::from_millis(500));

            // Force kill if still alive
            let _ = ps.child.kill();
            let _ = ps.child.wait();
            log("Pipeline process killed");
        } else {
            log("No pipeline running to stop");
        }
    }

    // Ensure streaming flag is cleared
    state.is_streaming.store(false, Ordering::Relaxed);

    // Allow the stream to be restarted
    state.stream_stop.store(false, Ordering::Relaxed);

    log("Pipeline stopped");
    Ok(())
}
