//! Transcript persistence commands for Auralis
//!
//! Provides Tauri commands to save, list, load, and delete transcript files
//! to `~/Library/Application Support/auralis/transcripts/` (macOS).

use chrono::{DateTime, Local, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

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
