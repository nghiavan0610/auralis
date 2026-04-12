//! Tauri commands for the Auralis application
//!
//! Core commands for the dual-mode architecture (cloud via Soniox / offline via Python sidecar).
//! Audio, settings, and pipeline commands are in their respective modules.

use serde::Serialize;

/// Platform information returned to the frontend.
#[derive(Serialize)]
pub struct PlatformInfo {
    pub os: String,
    pub system_audio_available: bool,
    pub offline_mode_available: bool,
}

/// Return platform capabilities so the frontend can adapt its UI.
#[tauri::command]
pub fn get_platform_info() -> PlatformInfo {
    PlatformInfo {
        os: std::env::consts::OS.to_string(),
        system_audio_available: cfg!(target_os = "macos"),
        offline_mode_available: cfg!(target_os = "macos"),
    }
}
