//! Settings load/save commands for Auralis
//!
//! Provides Tauri commands to persist user settings (mode, API key, languages)
//! to `~/.config/auralis/settings.json` and keep in-memory state in sync.

use crate::state::{AuralisState, Settings};
use std::fs;
use std::path::PathBuf;
use tauri::State;

/// Returns the path to the settings file: `~/.config/auralis/settings.json`
fn settings_path() -> Result<PathBuf, String> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| "Could not determine config directory".to_string())?;
    Ok(config_dir.join("auralis").join("settings.json"))
}

/// Ensure the `~/.config/auralis/` directory exists, creating it if necessary.
fn ensure_config_dir() -> Result<PathBuf, String> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| "Could not determine config directory".to_string())?;
    let auralis_dir = config_dir.join("auralis");
    if !auralis_dir.exists() {
        fs::create_dir_all(&auralis_dir)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }
    Ok(auralis_dir)
}

/// Load settings from disk.
///
/// If the settings file does not exist, returns defaults.
/// Also updates the in-memory state so subsequent reads are consistent.
#[tauri::command]
pub async fn get_settings(
    state: State<'_, AuralisState>,
) -> Result<Settings, String> {
    let path = settings_path()?;

    let settings = if path.exists() {
        let contents = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read settings file: {}", e))?;
        let parsed: Settings = serde_json::from_str(&contents)
            .map_err(|e| format!("Failed to parse settings JSON: {}", e))?;
        parsed
    } else {
        Settings::default()
    };

    // Update in-memory state
    *state.settings.lock().await = settings.clone();

    Ok(settings)
}

/// Alias for get_settings for compatibility with frontend stores
#[tauri::command]
pub async fn load_settings(
    state: State<'_, AuralisState>,
) -> Result<Settings, String> {
    get_settings(state).await
}

/// Load confidence settings from disk
#[tauri::command]
pub async fn load_confidence_settings() -> Result<serde_json::Value, String> {
    let path = settings_path()?;

    if !path.exists() {
        return Ok(serde_json::json!({
            "filter_level": "none",
            "high_threshold": 0.85,
            "medium_threshold": 0.70,
            "low_threshold": 0.50,
            "show_confidence_scores": false,
            "per_language_overrides": {}
        }));
    }

    let contents = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read settings file: {}", e))?;
    let settings: serde_json::Value = serde_json::from_str(&contents)
        .map_err(|e| format!("Failed to parse settings JSON: {}", e))?;

    Ok(serde_json::json!({
        "filter_level": settings.get("confidence_filter_level").unwrap_or(&serde_json::json!("none")),
        "high_threshold": 0.85,
        "medium_threshold": 0.70,
        "low_threshold": 0.50,
        "show_confidence_scores": false,
        "per_language_overrides": {}
    }))
}

/// Save confidence settings to disk
#[tauri::command]
pub async fn save_confidence_settings(
    state: State<'_, AuralisState>,
    settings: serde_json::Value,
) -> Result<(), String> {
    // Get current settings
    let mut current_settings = state.settings.lock().await.clone();

    // Update confidence filter level
    if let Some(filter_level) = settings.get("filter_level") {
        if let Some(level) = filter_level.as_str() {
            current_settings.confidence_filter_level = level.to_string();
        }
    }

    // Save all settings
    drop(state.settings.lock().await);
    save_settings(state, current_settings).await?;

    Ok(())
}

/// Save settings to disk and update in-memory state.
///
/// Creates the `~/.config/auralis/` directory if it does not exist.
#[tauri::command]
pub async fn save_settings(
    state: State<'_, AuralisState>,
    settings: Settings,
) -> Result<String, String> {
    // Ensure the config directory exists
    ensure_config_dir()?;

    let path = settings_path()?;

    // Serialize and write
    let json = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    fs::write(&path, json)
        .map_err(|e| format!("Failed to write settings file: {}", e))?;

    // Update in-memory state
    let old_settings = state.settings.lock().await.clone();
    *state.settings.lock().await = settings.clone();

    // Invalidate Google TTS voice cache if API key changed
    if old_settings.google_api_key != settings.google_api_key {
        crate::google_tts::voices::invalidate_cache();
    }

    // Invalidate ElevenLabs voice cache if API key changed
    if old_settings.elevenlabs_api_key != settings.elevenlabs_api_key {
        crate::elevenlabs_tts::voices::invalidate_cache();
    }

    tracing::info!(
        mode = %settings.mode,
        source = %settings.source_language,
        target = %settings.target_language,
        "Settings saved"
    );

    Ok("Settings saved".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_path_is_under_config_dir() {
        let path = settings_path().unwrap();
        assert!(path.to_string_lossy().contains("auralis"));
        assert!(path.to_string_lossy().contains("settings.json"));
    }

    #[test]
    fn test_ensure_config_dir_returns_valid_path() {
        // This should not fail even if the directory already exists
        let result = ensure_config_dir();
        assert!(result.is_ok());
        let dir = result.unwrap();
        assert!(dir.exists());
        assert!(dir.is_dir());
    }

    #[test]
    fn test_default_settings_serialize_roundtrip() {
        let original = Settings::default();
        let json = serde_json::to_string(&original).unwrap();
        let parsed: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.mode, original.mode);
        assert_eq!(parsed.soniox_api_key, original.soniox_api_key);
        assert_eq!(parsed.source_language, original.source_language);
        assert_eq!(parsed.target_language, original.target_language);
    }
}
