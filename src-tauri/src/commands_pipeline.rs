//! Tauri commands for managing the local Python sidecar pipeline
//!
//! Spawns `scripts/local_pipeline.py` as a child process, pipes audio data
//! (PCM s16le) to its stdin, reads JSON results from its stdout, and emits
//! Tauri events to the frontend.
//!
//! Audio flow: Rust captures audio → writes raw PCM to Python stdin.
//! Python handles its own buffering, chunking, VAD, ASR, and translation.
//! No VAD or flush markers in Rust — Python uses a sliding window.

use crate::audio::{f32_to_pcm_s16le, mix_pcm_s16le, open_privacy_settings, SystemAudioCapture};
use crate::state::{AuralisState, PipelineState};
use auralis::domain::traits::AudioSource;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, State};

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
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../scripts/local_pipeline.py"),
        std::path::PathBuf::from("scripts/local_pipeline.py"),
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
/// Accepts an optional `source` parameter ("microphone", "system", "both").
/// Defaults to "microphone" if not specified.
#[tauri::command]
pub async fn start_local_pipeline(
    state: State<'_, AuralisState>,
    app_handle: AppHandle,
    source: Option<String>,
) -> Result<(), String> {
    let source = source.unwrap_or_else(|| "microphone".to_string());

    // --- Guard: already running? ---
    if state.is_streaming.load(Ordering::Relaxed) {
        return Err("Pipeline is already running".to_string());
    }

    // --- Guard: stale pipeline from previous session? (force-reset) ---
    {
        let mut guard = state.pipeline.lock().map_err(|e| e.to_string())?;
        if let Some(mut ps) = guard.take() {
            tracing::warn!("Stale pipeline found, killing previous process (PID={})", ps.child.id());
            drop(ps.stdin);
            let _ = ps.child.kill();
            let _ = ps.child.wait();
        }
    }

    // --- Languages from settings ---
    let source_lang = state.source_language().await;
    let target_lang = state.target_language().await;
    let translation_type = state.translation_type().await;
    let endpoint_delay = state.endpoint_delay().await;

    log(&format!(
        "start_local_pipeline: {} -> {} (audio source: {}, translation: {})",
        source_lang, target_lang, source, translation_type
    ));

    // --- Locate script & python ---
    let script_path = find_pipeline_script()?;
    let python = find_python();
    tracing::info!("Pipeline: python={}, script={}", python, script_path.display());

    let _ = app_handle.emit(
        "pipeline-status",
        serde_json::json!({"type": "status", "message": "Starting Python pipeline..."}),
    );

    // --- Spawn the Python sidecar ---
    let path_env = "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin";
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());

    let mut cmd = Command::new(&python);
    cmd.arg(&script_path)
        .arg("--source-lang")
        .arg(&source_lang)
        .arg("--target-lang")
        .arg(&target_lang);
    if translation_type == "two_way" {
        cmd.arg("--two-way");
    }
    cmd.arg("--endpoint-delay").arg(endpoint_delay.to_string());
    tracing::info!("Spawning: {:?}", cmd);
    let mut child = cmd
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

    // --- Read stdout and emit events ---
    let app_stdout = app_handle.clone();
    let stdout_ready = state.pipeline_ready.clone();
    std::thread::spawn(move || {
        use std::io::BufRead;
        let reader = std::io::BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(line) if !line.is_empty() => {
                    log(&format!("stdout: {}", &line));

                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                        let msg_type = json
                            .get("type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");

                        match msg_type {
                            "original" => {
                                let _ = app_stdout.emit("pipeline-result", &line);
                            }
                            "result" => {
                                let _ = app_stdout.emit("pipeline-result", &line);
                            }
                            "status" => {
                                let _ = app_stdout.emit("pipeline-status", &line);
                            }
                            "ready" => {
                                stdout_ready.store(true, Ordering::Relaxed);
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
                                let _ = app_stdout.emit("pipeline-result", &line);
                            }
                        }
                    } else {
                        let _ = app_stdout.emit("pipeline-result", &line);
                    }
                }
                Ok(_) => {}
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

    // --- Start audio capture based on source ---
    state.is_streaming.store(true, Ordering::Relaxed);
    state.pipeline_ready.store(false, Ordering::Relaxed);
    let pipeline = state.pipeline.clone();
    let pipeline_ready = state.pipeline_ready.clone();
    let stream_stop = state.stream_stop.clone();
    let is_streaming = state.is_streaming.clone();
    let app_audio = app_handle.clone();

    // --- Set up audio sources based on requested source type ---
    let audio_start_result: Result<_, String> = {
        let result: Result<_, String> = match source.as_str() {
            "system" => {
                tracing::info!("Starting system audio capture (ScreenCaptureKit)");
                let sys_capture = SystemAudioCapture::new();
                let receiver = match sys_capture.start() {
                    Ok(r) => r,
                    Err(e) => {
                        tracing::error!("System audio capture FAILED: {}", e);
                        open_privacy_settings("screen");
                        return Err(format!(
                            "System audio capture failed: {}. Opening Screen Recording settings...",
                            e
                        ));
                    }
                };
                tracing::info!("System audio capture started successfully");

                let _ = app_handle.emit(
                    "pipeline-status",
                    serde_json::json!({"type":"status","message":"System audio capture started"}),
                );

                Ok((None, None, None, Some(receiver), Some(sys_capture)))
            }
            "both" => {
                let audio_config = auralis::infrastructure::AudioCaptureConfig::default();
                let mut mic = match auralis::infrastructure::MicrophoneCapture::new(audio_config) {
                    Ok(c) => c,
                    Err(e) => return Err(format!("Failed to create mic capture: {}", e)),
                };
                if let Err(e) = mic.start().await {
                    open_privacy_settings("microphone");
                    return Err(format!(
                        "Mic capture failed: {}. Opening Microphone settings...",
                        e
                    ));
                }
                tracing::info!("Mic capture started for 'both' mode");

                let mic_data = mic.audio_data();
                let mic_recording = mic.is_recording_flag();

                // System audio (optional — fall back to mic-only if unavailable)
                let sys = SystemAudioCapture::new();
                let sys_result = sys.start();
                let has_sys = sys_result.is_ok();
                let sys_receiver = sys_result.ok();

                if !has_sys {
                    tracing::error!(
                        "System audio capture FAILED in 'both' mode. \
                         Falling back to mic-only. \
                         Fix: System Settings > Privacy & Security > Screen Recording > enable this app/terminal"
                    );
                }

                let status_msg = if has_sys {
                    "Audio capture started (mic + system)"
                } else {
                    "System audio unavailable, using mic only"
                };
                let _ = app_handle.emit(
                    "pipeline-status",
                    serde_json::json!({"type":"status","message": status_msg}),
                );

                Ok((
                    Some(mic_data),
                    Some(mic_recording),
                    Some(mic),
                    sys_receiver,
                    if has_sys { Some(sys) } else { None },
                ))
            }
            _ => {
                // "microphone" (default)
                let audio_config = auralis::infrastructure::AudioCaptureConfig::default();
                let mut mic = match auralis::infrastructure::MicrophoneCapture::new(audio_config) {
                    Ok(c) => c,
                    Err(e) => return Err(format!("Failed to create audio capture: {}", e)),
                };
                if let Err(e) = mic.start().await {
                    open_privacy_settings("microphone");
                    return Err(format!(
                        "Audio capture failed: {}. Opening Microphone settings...",
                        e
                    ));
                }

                let mic_data = mic.audio_data();
                let mic_recording = mic.is_recording_flag();

                let _ = app_handle.emit(
                    "pipeline-status",
                    serde_json::json!({"type":"status","message":"Audio capture started"}),
                );

                Ok((Some(mic_data), Some(mic_recording), Some(mic), None, None))
            }
        };
        result
    };

    // Handle audio capture errors — kill the Python child on failure
    let (mic_data, mic_recording, mic_keeper, sys_receiver, sys_keeper) = match audio_start_result {
        Ok(sources) => sources,
        Err(e) => {
            log(&format!("Audio capture failed, killing pipeline: {}", e));
            state.is_streaming.store(false, Ordering::Relaxed);
            let _ = child.kill();
            let _ = child.wait();
            return Err(e);
        }
    };

    // --- Single unified write loop for all audio sources ---
    tokio::spawn(async move {
        // Keep captures alive for the duration of the task
        let _mic = mic_keeper;
        let _sys = sys_keeper;

        let has_mic = mic_data.is_some();
        let has_sys = sys_receiver.is_some();
        tracing::info!("Audio write loop started: mic={}, system={}", has_mic, has_sys);

        let mut loop_count: u64 = 0;
        let mut mic_bytes_total: u64 = 0;
        let mut sys_bytes_total: u64 = 0;
        let mut flushed_stale = false;

        while !stream_stop.load(Ordering::Relaxed) {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;

            if !pipeline_ready.load(Ordering::Relaxed) {
                continue;
            }

            // --- On first ready: drain stale audio that accumulated during model loading ---
            if !flushed_stale {
                flushed_stale = true;
                if let (Some(data), Some(recording)) = (mic_data.as_ref(), mic_recording.as_ref()) {
                    let rec = recording.lock().unwrap_or_else(|e| e.into_inner());
                    if *rec {
                        let mut d = data.lock().unwrap_or_else(|e| e.into_inner());
                        let drained = d.drain(..).count();
                        tracing::info!("Flushed {} stale mic chunks from model-loading period", drained);
                    }
                }
                if let Some(ref recv) = sys_receiver {
                    let mut stale_count = 0;
                    while let Ok(_data) = recv.try_recv() {
                        stale_count += 1;
                    }
                    tracing::info!("Flushed {} stale system audio chunks from model-loading period", stale_count);
                }
                tracing::info!("Stale audio flushed, pipeline now processing real-time audio");
                continue; // Skip this iteration, start fresh next cycle
            }

            let mut all_pcm = Vec::new();

            // Collect mic PCM (if mic source is active)
            let mic_pcm: Vec<u8> = if let (Some(data), Some(recording)) = (mic_data.as_ref(), mic_recording.as_ref()) {
                let rec = recording.lock().unwrap_or_else(|e| e.into_inner());
                if *rec {
                    let mut d = data.lock().unwrap_or_else(|e| e.into_inner());
                    let chunks: Vec<Vec<f32>> = d.drain(..).collect();
                    let samples: Vec<f32> = chunks.into_iter().flatten().collect();
                    let pcm = f32_to_pcm_s16le(&samples);
                    mic_bytes_total += pcm.len() as u64;
                    pcm
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            };

            // Collect system PCM (if system source is active)
            let sys_pcm: Vec<u8> = if let Some(ref recv) = sys_receiver {
                let mut buf = Vec::new();
                while let Ok(data) = recv.try_recv() {
                    sys_bytes_total += data.len() as u64;
                    buf.extend_from_slice(&data);
                }
                buf
            } else {
                Vec::new()
            };

            // Mix or pass through audio depending on active sources
            if !mic_pcm.is_empty() && !sys_pcm.is_empty() {
                // Both sources active: mix by averaging samples
                all_pcm = mix_pcm_s16le(&mic_pcm, &sys_pcm);
            } else if !mic_pcm.is_empty() {
                all_pcm = mic_pcm;
            } else if !sys_pcm.is_empty() {
                all_pcm = sys_pcm;
            }

            if all_pcm.is_empty() {
                continue;
            }

            // Log audio flow every 5 seconds (25 iterations at 200ms)
            loop_count += 1;
            if loop_count % 25 == 0 {
                tracing::info!(
                    "Audio flow: mic={} bytes, system={} bytes, total writes={}, samples_per_200ms={}",
                    mic_bytes_total, sys_bytes_total, loop_count,
                    all_pcm.len() / 2
                );
            }

            // Write to pipeline stdin
            let write_result = {
                let mut guard = match pipeline.lock() {
                    Ok(g) => g,
                    Err(e) => {
                        tracing::error!("Pipeline mutex poisoned: {}", e);
                        break;
                    }
                };
                match guard.as_mut() {
                    Some(ps) => ps.stdin.write_all(&all_pcm).and_then(|_| ps.stdin.flush()),
                    None => break,
                }
            };
            if let Err(e) = write_result {
                tracing::error!("stdin write error: {}", e);
                break;
            }
        }

        is_streaming.store(false, Ordering::Relaxed);
        let _ = app_audio.emit(
            "pipeline-status",
            serde_json::json!({"type":"status","message":"Audio capture stopped"}),
        );
    });

    // Audio capture succeeded — now store pipeline state
    {
        let mut guard = state.pipeline.lock().map_err(|e| e.to_string())?;
        *guard = Some(PipelineState {
            child,
            stdin: stdin_handle,
        });
    }

    log("Pipeline started successfully");
    Ok(())
}

/// Stop the local (offline) translation pipeline.
#[tauri::command]
pub async fn stop_local_pipeline(
    state: State<'_, AuralisState>,
) -> Result<(), String> {
    log("stop_local_pipeline called");

    state.stream_stop.store(true, Ordering::Relaxed);

    // Take ownership of PipelineState out of the mutex before cleanup,
    // so audio writer tasks aren't blocked on the mutex during the sleep.
    let pipeline_state = {
        let mut guard = state.pipeline.lock().map_err(|e| e.to_string())?;
        guard.take()
    };

    if let Some(mut ps) = pipeline_state {
        drop(ps.stdin); // Close pipe → Python sees EOF and exits
        std::thread::sleep(std::time::Duration::from_millis(500));
        let _ = ps.child.kill();
        let _ = ps.child.wait();
        log("Pipeline process killed");
    } else {
        log("No pipeline running to stop");
    }

    state.is_streaming.store(false, Ordering::Relaxed);
    state.stream_stop.store(false, Ordering::Relaxed);
    state.pipeline_ready.store(false, Ordering::Relaxed);

    log("Pipeline stopped");
    Ok(())
}

// ---------------------------------------------------------------------------
// Offline environment setup (one-click install)
// ---------------------------------------------------------------------------

/// Check whether the offline Python environment is set up and ready.
#[tauri::command]
pub async fn check_offline_ready() -> Result<serde_json::Value, String> {
    let config_dir = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let venv_dir = config_dir.join("auralis").join("mlx-env");
    let venv_python = venv_dir.join("bin").join("python3");

    let venv_exists = venv_python.exists();

    let packages_installed = if venv_exists {
        let output = Command::new(&venv_python)
            .args(["-c", "import mlx_whisper; import transformers; import numpy; import sentencepiece; print('ok')"])
            .output()
            .map_err(|e| format!("Failed to run python check: {}", e))?;
        output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "ok"
    } else {
        false
    };

    Ok(serde_json::json!({
        "venv_exists": venv_exists,
        "packages_installed": packages_installed,
        "ready": venv_exists && packages_installed,
    }))
}

/// Set up the offline Python environment (one-click).
#[tauri::command]
pub async fn setup_offline_environment(
    app_handle: AppHandle,
) -> Result<(), String> {
    let emit_progress = |step: &str, message: &str, progress: u8| {
        let _ = app_handle.emit("offline-setup-progress", serde_json::json!({
            "step": step,
            "message": message,
            "progress": progress,
        }));
    };

    emit_progress("finding-python", "Looking for Python 3...", 5);

    let system_python = find_system_python()?;

    let version = Command::new(&system_python)
        .args(["--version"])
        .output()
        .map_err(|e| format!("Failed to check python version: {}", e))?;

    let version_str = String::from_utf8_lossy(&version.stdout).trim().to_string();
    emit_progress("finding-python", &format!("Found {}", version_str), 10);

    let config_dir = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let auralis_dir = config_dir.join("auralis");
    let venv_dir = auralis_dir.join("mlx-env");

    if venv_dir.exists() {
        emit_progress("creating-venv", "Removing old environment...", 15);
        std::fs::remove_dir_all(&venv_dir)
            .map_err(|e| format!("Failed to remove old venv: {}", e))?;
    }

    std::fs::create_dir_all(&auralis_dir)
        .map_err(|e| format!("Failed to create config dir: {}", e))?;

    emit_progress("creating-venv", "Creating virtual environment...", 20);

    let venv_result = Command::new(&system_python)
        .args(["-m", "venv", venv_dir.to_string_lossy().as_ref()])
        .output()
        .map_err(|e| format!("Failed to create venv: {}", e))?;

    if !venv_result.status.success() {
        let stderr = String::from_utf8_lossy(&venv_result.stderr);
        return Err(format!("Failed to create venv: {}", stderr));
    }

    emit_progress("creating-venv", "Virtual environment created", 30);

    let venv_python = venv_dir.join("bin").join("python3");
    let venv_pip = venv_dir.join("bin").join("pip3");

    emit_progress("upgrading-pip", "Upgrading pip...", 35);

    let _ = Command::new(&venv_python)
        .args(["-m", "pip", "install", "--upgrade", "pip"])
        .output();

    emit_progress("upgrading-pip", "pip upgraded", 40);

    let packages = ["mlx-whisper", "transformers", "numpy", "sentencepiece", "protobuf"];

    for (i, pkg) in packages.iter().enumerate() {
        let pct_start = 40 + ((i as u8) * 20);
        let pct_end = pct_start + 20;
        emit_progress(
            "installing-packages",
            &format!("Installing {}...", pkg),
            pct_start,
        );

        let mut child = Command::new(&venv_pip)
            .args(["install", "--no-cache-dir", "--progress-bar", "off", pkg])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to run pip install for {}: {}", pkg, e))?;

        let stderr = child.stderr.take().unwrap();
        let app_install = app_handle.clone();
        let pkg_name = pkg.to_string();

        let stderr_thread = std::thread::spawn(move || -> String {
            use std::io::BufRead;
            let reader = std::io::BufReader::new(stderr);
            let mut all_output = String::new();
            for line in reader.lines().flatten() {
                all_output.push_str(&line);
                all_output.push('\n');
                let short = if line.len() > 80 {
                    format!("...{}", &line[line.len() - 77..])
                } else {
                    line.clone()
                };
                let _ = app_install.emit("offline-setup-progress", serde_json::json!({
                    "step": "installing-packages",
                    "message": format!("Installing {}: {}", pkg_name, short),
                    "progress": pct_start + 10,
                }));
            }
            all_output
        });

        let status = child.wait()
            .map_err(|e| format!("pip install {} failed: {}", pkg, e))?;
        let stderr_output = stderr_thread.join().unwrap_or_default();

        if !status.success() {
            return Err(format!("Failed to install {}: {}", pkg, stderr_output));
        }

        emit_progress(
            "installing-packages",
            &format!("{} installed", pkg),
            pct_end,
        );
    }

    emit_progress("verifying", "Verifying installation...", 95);

    let check = Command::new(&venv_python)
        .args(["-c", "import mlx_whisper; import transformers; import numpy; print('ok')"])
        .output()
        .map_err(|e| format!("Verification failed: {}", e))?;

    if !check.status.success() {
        let stderr = String::from_utf8_lossy(&check.stderr);
        return Err(format!("Package verification failed: {}", stderr));
    }

    emit_progress("complete", "Offline mode is ready!", 100);
    log("Offline environment setup complete");

    Ok(())
}

/// Find a system Python 3 (not the venv one).
fn find_system_python() -> Result<String, String> {
    let candidates = [
        "/opt/homebrew/bin/python3",
        "/usr/local/bin/python3",
        "/usr/bin/python3",
    ];

    for path in &candidates {
        if std::path::Path::new(path).exists() {
            return Ok(path.to_string());
        }
    }

    let output = Command::new("which")
        .arg("python3")
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !path.is_empty() {
                return Ok(path);
            }
        }
    }

    Err("Python 3 not found. Please install Python 3 first.".to_string())
}
