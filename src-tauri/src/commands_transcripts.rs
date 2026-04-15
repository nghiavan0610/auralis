//! Transcript persistence commands for Auralis
//!
//! Provides Tauri commands to save, list, load, and delete transcript files
//! to `~/Library/Application Support/auralis/transcripts/` (macOS).

use chrono::{DateTime, Local, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::BufRead;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::state::AuralisState;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// A single transcript segment produced by the translation pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub original: String,
    pub translated: String,
    pub detected_lang: String,
    pub target_lang: String,
    pub timestamp: i64, // millis since epoch
}

/// Metadata returned when listing saved transcripts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptMeta {
    pub filename: String,
    pub date: String,
    pub segment_count: usize,
    pub preview: String,
}

/// Summary data stored as sidecar .summary.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryData {
    pub version: u32,
    pub transcript_file: String,
    pub generated_at: String,
    pub model_used: String,
    pub tier: String,
    pub summary: serde_json::Value,
}

/// Metadata about an existing summary (for list preview)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryPreview {
    pub exists: bool,
    pub model_used: Option<String>,
    pub tier: Option<String>,
    pub key_points_count: Option<usize>,
    pub action_items_count: Option<usize>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns the transcripts directory: `$LOCAL_DATA_DIR/auralis/transcripts/`
fn transcripts_dir() -> Result<PathBuf, String> {
    let data_dir = dirs::data_local_dir()
        .ok_or_else(|| "Could not determine local data directory".to_string())?;
    Ok(data_dir.join("auralis").join("transcripts"))
}

/// Ensures the transcripts directory exists, creating it if necessary.
fn ensure_transcripts_dir() -> Result<PathBuf, String> {
    let dir = transcripts_dir()?;
    if !dir.exists() {
        fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create transcripts directory: {}", e))?;
    }
    Ok(dir)
}

/// Format a single segment as: `[HH:MM:SS] original (detected -> target) translated`
fn format_segment(seg: &TranscriptSegment) -> String {
    let dt: DateTime<Local> = Utc
        .timestamp_millis_opt(seg.timestamp)
        .single()
        .unwrap_or_else(|| Utc::now())
        .into();
    let time = dt.format("%H:%M:%S");
    format!(
        "[{}] {} ({} \u{2192} {}) {}",
        time, seg.original, seg.detected_lang, seg.target_lang, seg.translated
    )
}

/// Verify that `filename` resolves to a path inside the transcripts directory
/// to prevent directory-traversal attacks.
fn safe_path(filename: &str) -> Result<PathBuf, String> {
    let dir = transcripts_dir()?;
    let target = dir.join(filename);

    // Canonicalize both paths so that symlinks and `..` are resolved.
    // If the target file doesn't exist yet, canonicalize only the parent.
    let canonical_dir = dir
        .canonicalize()
        .map_err(|e| format!("Failed to resolve transcripts directory: {}", e))?;

    let canonical_target = if target.exists() {
        target
            .canonicalize()
            .map_err(|e| format!("Failed to resolve file path: {}", e))?
    } else {
        // File doesn't exist — resolve the parent and join.
        let parent = target
            .parent()
            .unwrap_or_else(|| dir.as_path())
            .canonicalize()
            .map_err(|e| format!("Failed to resolve parent path: {}", e))?;
        parent.join(
            target
                .file_name()
                .ok_or_else(|| "Invalid filename".to_string())?,
        )
    };

    if !canonical_target.starts_with(&canonical_dir) {
        return Err("Path traversal not allowed".to_string());
    }

    Ok(canonical_target)
}

/// Given "2026-04-13_14-30-15.txt", return path to "2026-04-13_14-30-15.summary.json"
fn summary_path_for(filename: &str) -> Result<PathBuf, String> {
    let base = filename.trim_end_matches(".txt");
    let summary_name = format!("{}.summary.json", base);
    safe_path(&summary_name)
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Save a transcript to disk.
///
/// Creates a timestamped `.txt` file in the transcripts directory.
#[tauri::command]
pub async fn save_transcript(segments: Vec<TranscriptSegment>) -> Result<String, String> {
    if segments.is_empty() {
        return Err("Cannot save an empty transcript".to_string());
    }

    let dir = ensure_transcripts_dir()?;

    // Derive filename from the first segment's timestamp.
    let first_ts = segments[0].timestamp;
    let dt: DateTime<Local> = Utc
        .timestamp_millis_opt(first_ts)
        .single()
        .unwrap_or_else(|| Utc::now())
        .into();
    let filename = dt.format("%Y-%m-%d_%H-%M-%S").to_string() + ".txt";
    let path = dir.join(&filename);

    let lines: Vec<String> = segments.iter().map(format_segment).collect();
    let content = lines.join("\n");

    fs::write(&path, content)
        .map_err(|e| format!("Failed to write transcript: {}", e))?;

    tracing::info!(filename = %filename, segments = segments.len(), "Transcript saved");

    Ok(filename)
}

/// List all saved transcripts, newest first.
#[tauri::command]
pub async fn list_transcripts() -> Result<Vec<TranscriptMeta>, String> {
    let dir = transcripts_dir()?;

    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries: Vec<_> = fs::read_dir(&dir)
        .map_err(|e| format!("Failed to read transcripts directory: {}", e))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .map(|ext| ext == "txt")
                .unwrap_or(false)
        })
        .collect();

    // Sort by modified time, newest first.
    entries.sort_by(|a, b| {
        let a_time = a.metadata().and_then(|m| m.modified()).ok();
        let b_time = b.metadata().and_then(|m| m.modified()).ok();
        b_time.cmp(&a_time)
    });

    let mut metas = Vec::new();
    for entry in entries {
        let filename = entry
            .file_name()
            .to_string_lossy()
            .to_string();

        let content = match fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let segment_count = content.lines().count();

        // Derive a human-readable date from the filename.
        // Filename format: YYYY-MM-DD_HH-MM-SS.txt
        let stem = filename.trim_end_matches(".txt").replace('_', " ");
        let date_display = chrono::NaiveDateTime::parse_from_str(&stem, "%Y-%m-%d %H:%M:%S")
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or(stem);

        let preview = content
            .lines()
            .next()
            .unwrap_or("")
            .chars()
            .take(80)
            .collect();

        metas.push(TranscriptMeta {
            filename,
            date: date_display,
            segment_count,
            preview,
        });
    }

    Ok(metas)
}

/// Load the content of a transcript file.
#[tauri::command]
pub async fn load_transcript(filename: String) -> Result<String, String> {
    let path = safe_path(&filename)?;

    if !path.exists() {
        return Err(format!("Transcript not found: {}", filename));
    }

    fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read transcript: {}", e))
}

/// Delete a transcript file.
#[tauri::command]
pub async fn delete_transcript(filename: String) -> Result<String, String> {
    let path = safe_path(&filename)?;

    if !path.exists() {
        return Err(format!("Transcript not found: {}", filename));
    }

    fs::remove_file(&path)
        .map_err(|e| format!("Failed to delete transcript: {}", e))?;

    tracing::info!(filename = %filename, "Transcript deleted");

    Ok(format!("Deleted {}", filename))
}

/// Rename a transcript file and its sidecar summary (if any).
/// `new_name` should NOT include the .txt extension.
#[tauri::command]
pub async fn rename_transcript(filename: String, new_name: String) -> Result<String, String> {
    let old_path = safe_path(&filename)?;
    if !old_path.exists() {
        return Err(format!("Transcript not found: {}", filename));
    }

    // Sanitize: strip extension if user included one, then add .txt
    let clean_name = new_name.trim_end_matches(".txt").trim_end_matches(".json");
    if clean_name.is_empty() {
        return Err("Name cannot be empty".to_string());
    }
    let new_filename = format!("{}.txt", clean_name);
    let new_path = safe_path(&new_filename)?;

    if new_path.exists() && new_filename != filename {
        return Err(format!("A transcript named '{}' already exists", new_filename));
    }

    // Rename transcript file
    fs::rename(&old_path, &new_path)
        .map_err(|e| format!("Failed to rename transcript: {}", e))?;

    // Rename sidecar summary if it exists
    let old_summary = summary_path_for(&filename);
    let new_summary = summary_path_for(&new_filename);
    if let (Ok(old_s), Ok(new_s)) = (old_summary, new_summary) {
        if old_s.exists() {
            if let Err(e) = fs::rename(&old_s, &new_s) {
                tracing::warn!("Failed to rename summary sidecar: {}", e);
            }
        }
    }

    tracing::info!(old = %filename, new = %new_filename, "Transcript renamed");

    Ok(new_filename)
}

// ---------------------------------------------------------------------------
// Summary commands
// ---------------------------------------------------------------------------

/// Generate a meeting summary by spawning the Python summary.py script.
///
/// The script reads the transcript file and outputs JSON lines on stdout.
/// Events are forwarded to the frontend via Tauri events.
#[tauri::command]
pub async fn generate_summary(
    app: AppHandle,
    state: State<'_, AuralisState>,
    filename: String,
    _tier: String,
) -> Result<(), String> {
    // Validate transcript exists
    let transcript_path = safe_path(&filename)?;
    if !transcript_path.exists() {
        return Err(format!("Transcript not found: {}", filename));
    }

    // Check subscription tier and enforce limits
    // IMPORTANT: Must atomically check limit AND increment to prevent race conditions
    let current_month = chrono::Utc::now().format("%Y-%m").to_string();
    let (subscription_tier, model_to_use, gpt_key_to_use) = {
        let mut settings = state.settings.lock().await;

        // Check if we need to reset the monthly counter
        let needs_reset = settings.last_summary_reset != current_month;
        if needs_reset {
            settings.summaries_this_month = 0;
            settings.last_summary_reset = current_month.clone();
        }

        let tier = settings.subscription_tier.clone();
        let current_count = settings.summaries_this_month;
        let limit = if tier == "free" {
            crate::constants::FREE_TIER_SUMMARY_LIMIT
        } else {
            crate::constants::PRO_TIER_SUMMARY_LIMIT
        };

        // Enforce limit BEFORE incrementing (optimistic increment)
        if current_count >= limit {
            drop(settings);
            if tier == "free" {
                return Err(format!(
                    "Free tier limit reached: {} summaries per month. Upgrade to Pro for up to {} summaries/month.",
                    crate::constants::FREE_TIER_SUMMARY_LIMIT,
                    crate::constants::PRO_TIER_SUMMARY_LIMIT
                ));
            } else {
                return Err(format!(
                    "Pro tier limit reached: {} summaries per month. Please contact support@auralis.app if you need more.",
                    crate::constants::PRO_TIER_SUMMARY_LIMIT
                ));
            }
        }

        // Atomically increment counter (optimistic increment)
        settings.summaries_this_month += 1;

        // Determine model and get API key based on tier
        let (model, gpt_key) = if tier == "pro" {
            ("gpt".to_string(), std::env::var("AURALIS_OPENAI_API_KEY").unwrap_or_default())
        } else {
            ("gemma".to_string(), String::new())
        };

        // Verify Pro tier has backend key configured
        if tier == "pro" && gpt_key.is_empty() {
            // Rollback the counter increment since we can't proceed
            settings.summaries_this_month -= 1;
            drop(settings);
            return Err("Pro tier is not configured. Please contact support.".to_string());
        }

        (tier, model, gpt_key)
    };

    let python = crate::commands_pipeline::find_python();

    // Locate summary.py: try resource dir first, then fallback for dev mode
    let script_path = {
        let resource_dir_result: Result<std::path::PathBuf, _> = app
            .path()
            .resource_dir()
            .map(|p| p.to_path_buf());

        let resource_attempt: Option<std::path::PathBuf> = resource_dir_result
            .ok()
            .map(|d: std::path::PathBuf| d.join("scripts/summary.py"));

        if let Some(ref p) = resource_attempt {
            if p.exists() {
                tracing::info!("Using resource-dir summary.py: {:?}", p);
                p.clone()
            } else {
                let fallback =
                    std::env::current_dir().unwrap_or_default().join("scripts/summary.py");
                tracing::info!("Resource-dir script not found, trying cwd fallback: {:?}", fallback);
                if fallback.exists() {
                    fallback
                } else {
                    // Last resort: try CARGO_MANIFEST_DIR/../scripts/
                    let manifest_fallback = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("../scripts/summary.py");
                    tracing::info!("Trying manifest fallback: {:?}", manifest_fallback);
                    manifest_fallback
                }
            }
        } else {
            let fallback =
                std::env::current_dir().unwrap_or_default().join("scripts/summary.py");
            tracing::info!("No resource dir, trying cwd fallback: {:?}", fallback);
            fallback
        }
    };

    // Build command args
    let args: Vec<String> = vec![
        script_path.to_string_lossy().to_string(),
        "--input".to_string(),
        transcript_path.to_string_lossy().to_string(),
        "--tier".to_string(),
        subscription_tier.clone(),
        "--model".to_string(),
        model_to_use.clone(),
    ];

    tracing::info!("Spawning summary process: {} {:?}", python, args);

    #[cfg(target_os = "macos")]
    let path_env = "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin".to_string();
    #[cfg(not(target_os = "macos"))]
    let path_env =
        std::env::var("PATH").unwrap_or_else(|_| "/usr/local/bin:/usr/bin:/bin".to_string());

    let mut child = Command::new(&python)
        .args(&args)
        .env("PATH", path_env)
        .env("OPENAI_API_KEY", &gpt_key_to_use)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start summary process: {}", e))?;

    let stdout = child.stdout.take().ok_or("Failed to get stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to get stderr")?;

    // Stdout reader thread
    let app_stdout = app.clone();
    let filename_owned = filename.clone();
    std::thread::spawn(move || {
        let reader = std::io::BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(line) if !line.is_empty() => {
                    tracing::info!("[summary] stdout: {}", &line);
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                        let msg_type = json
                            .get("type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");

                        match msg_type {
                            "status" => {
                                let _ = app_stdout.emit("summary-progress", &line);
                            }
                            "summary" => {
                                // Save the data field as a .summary.json sidecar file
                                if let Some(data) = json.get("data") {
                                    // Extract only the inner "summary" object from Python's output
                                    // Python sends: {version, transcript_file, generated_at, model_used, tier, summary: {key_points, ...}}
                                    let summary_content = data
                                        .get("summary")
                                        .cloned()
                                        .unwrap_or(data.clone());

                                    let summary = SummaryData {
                                        version: data
                                            .get("version")
                                            .and_then(|v| v.as_u64())
                                            .unwrap_or(1) as u32,
                                        transcript_file: filename_owned.clone(),
                                        generated_at: data
                                            .get("generated_at")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                        model_used: data
                                            .get("model_used")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("unknown")
                                            .to_string(),
                                        tier: data
                                            .get("tier")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("free")
                                            .to_string(),
                                        summary: summary_content,
                                    };

                                    match serde_json::to_string_pretty(&summary) {
                                        Ok(json_str) => {
                                            match summary_path_for(&filename_owned) {
                                                Ok(path) => {
                                                    if let Err(e) = fs::write(&path, &json_str) {
                                                        tracing::error!(
                                                            "Failed to write summary file: {}",
                                                            e
                                                        );
                                                        let _ = app_stdout.emit(
                                                            "summary-error",
                                                            serde_json::json!({
                                                                "type": "error",
                                                                "message": format!("Failed to write summary: {}", e)
                                                            }),
                                                        );
                                                    } else {
                                                        tracing::info!(
                                                            "Summary saved to {:?}",
                                                            path
                                                        );
                                                        // Counter already incremented atomically at the start
                                                    }
                                                }
                                                Err(e) => {
                                                    tracing::error!(
                                                        "Failed to resolve summary path: {}",
                                                        e
                                                    );
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!(
                                                "Failed to serialize summary: {}",
                                                e
                                            );
                                        }
                                    }
                                }
                                let _ = app_stdout.emit("summary-result", &line);
                            }
                            "error" => {
                                let _ = app_stdout.emit("summary-error", &line);
                            }
                            _ => {
                                let _ = app_stdout.emit("summary-progress", &line);
                            }
                        }
                    }
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
        tracing::info!("[summary] stdout reader ended");
    });

    // Stderr reader thread
    let app_stderr = app.clone();
    std::thread::spawn(move || {
        let reader = std::io::BufReader::new(stderr);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    tracing::warn!("[summary] stderr: {}", line);
                    let _ = app_stderr.emit(
                        "summary-progress",
                        serde_json::json!({
                            "type": "status",
                            "message": line
                        }),
                    );
                }
                Err(_) => break,
            }
        }
    });

    // Wait for the process to finish in a background thread (don't block the async runtime)
    std::thread::spawn(move || {
        let _ = child.wait();
    });

    Ok(())
}

/// Load a summary sidecar file for a transcript.
///
/// Returns `Ok(None)` if no summary exists yet.
#[tauri::command]
pub async fn load_summary(filename: String) -> Result<Option<SummaryData>, String> {
    let path = summary_path_for(&filename)?;

    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read summary: {}", e))?;

    let summary: SummaryData = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse summary: {}", e))?;

    Ok(Some(summary))
}

/// Delete a summary sidecar file.
#[tauri::command]
pub async fn delete_summary(filename: String) -> Result<String, String> {
    let path = summary_path_for(&filename)?;

    if !path.exists() {
        return Err(format!("Summary not found for {}", filename));
    }

    fs::remove_file(&path)
        .map_err(|e| format!("Failed to delete summary: {}", e))?;

    tracing::info!(filename = %filename, "Summary deleted");

    Ok(format!("Deleted summary for {}", filename))
}

/// Check whether a summary exists for a transcript and return preview metadata.
#[tauri::command]
pub async fn check_summary(filename: String) -> Result<SummaryPreview, String> {
    let path = summary_path_for(&filename)?;

    if !path.exists() {
        return Ok(SummaryPreview {
            exists: false,
            model_used: None,
            tier: None,
            key_points_count: None,
            action_items_count: None,
        });
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read summary: {}", e))?;

    let summary: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse summary: {}", e))?;

    // Extract counts from the nested summary data
    let summary_data = summary.get("summary").unwrap_or(&summary);

    let key_points_count = summary_data
        .get("key_points")
        .and_then(|v| v.as_array())
        .map(|a| a.len());

    let action_items_count = summary_data
        .get("action_items")
        .and_then(|v| v.as_array())
        .map(|a| a.len());

    Ok(SummaryPreview {
        exists: true,
        model_used: summary
            .get("model_used")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        tier: summary
            .get("tier")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        key_points_count,
        action_items_count,
    })
}

/// Get subscription status and remaining summaries for this month.
#[tauri::command]
pub async fn get_subscription_status(state: State<'_, AuralisState>) -> Result<serde_json::Value, String> {
    let current_month = chrono::Utc::now().format("%Y-%m").to_string();
    let settings = state.settings.lock().await;

    // Check if we need to reset the counter
    let needs_reset = settings.last_summary_reset != current_month;
    let (tier, remaining, reset_date) = if needs_reset {
        drop(settings);
        let mut s = state.settings.lock().await;
        s.summaries_this_month = 0;
        s.last_summary_reset = current_month.clone();
        let remaining = if s.subscription_tier == "free" { 5 } else { 500 };
        (s.subscription_tier.clone(), remaining, current_month)
    } else {
        let tier = settings.subscription_tier.clone();
        let count = settings.summaries_this_month;
        let remaining = if tier == "free" { 5 - count } else { 500 - count };
        (tier, remaining.max(0), settings.last_summary_reset.clone())
    };

    Ok(serde_json::json!({
        "tier": tier,
        "remaining_summaries": remaining,
        "reset_date": reset_date,
    }))
}

/// Handle RevenueCat webhook events to update subscription tier.
/// This endpoint is called by RevenueCat when subscription status changes.
///
/// Expected JSON payload:
/// {
///   "event": {
///     "event_type": "INITIAL_PURCHASE" | "RENEWAL" | "CANCELLATION" | "EXPIRATION",
///     "product_id": "auralis_pro_monthly",
///     "entitlement_id": "pro"
///   },
///   "api_version": "1.0"
/// }
#[tauri::command]
pub async fn handle_revenuecat_webhook(
    state: State<'_, AuralisState>,
    payload: serde_json::Value,
) -> Result<serde_json::Value, String> {
    use serde_json::json;

    // Validate webhook payload structure
    let event = payload
        .get("event")
        .and_then(|e| e.as_object())
        .ok_or("Invalid webhook payload: missing event")?;

    let event_type = event
        .get("event_type")
        .and_then(|t| t.as_str())
        .ok_or("Invalid webhook payload: missing event_type")?;

    let entitlement_id = event
        .get("entitlement_id")
        .and_then(|e| e.as_str())
        .unwrap_or("");

    tracing::info!(
        "RevenueCat webhook: event_type={}, entitlement_id={}",
        event_type,
        entitlement_id
    );

    // Determine new tier based on event type
    let new_tier = match event_type {
        "INITIAL_PURCHASE" | "RENEWAL" | "UNCANCELLATION" => {
            if entitlement_id == "pro" {
                "pro".to_string()
            } else {
                "free".to_string()
            }
        }
        "CANCELLATION" | "EXPIRATION" | "PRODUCT_CHANGE" => {
            // Downgrade to free when subscription is cancelled or expires
            "free".to_string()
        }
        _ => {
            // For other events (like TEST), don't change tier
            return Ok(json!({
                "status": "ignored",
                "message": format!("Event type '{}' ignored", event_type)
            }));
        }
    };

    // Update subscription tier in settings
    let mut settings = state.settings.lock().await;
    let old_tier = settings.subscription_tier.clone();
    settings.subscription_tier = new_tier.clone();
    let settings_to_save = settings.clone();
    drop(settings); // Release lock before I/O

    tracing::info!(
        "Subscription tier updated: {} -> {}",
        old_tier,
        new_tier
    );

    // Persist settings to disk
    let config_dir = dirs::config_dir()
        .ok_or_else(|| "Could not determine config directory".to_string())?;
    let auralis_dir = config_dir.join("auralis");
    let settings_file = auralis_dir.join("settings.json");

    // Create directory if it doesn't exist
    std::fs::create_dir_all(&auralis_dir)
        .map_err(|e| format!("Failed to create config directory: {}", e))?;

    // Write settings
    let json = serde_json::to_string_pretty(&settings_to_save)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    std::fs::write(&settings_file, json)
        .map_err(|e| format!("Failed to write settings file: {}", e))?;

    tracing::info!("Settings saved after webhook tier update");

    Ok(json!({
        "status": "success",
        "old_tier": old_tier,
        "new_tier": new_tier,
        "event_type": event_type
    }))
}

