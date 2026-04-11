//! Tauri commands for the Auralis application
//!
//! Core commands for the dual-mode architecture (cloud via Soniox / offline via Python sidecar).
//! Audio, settings, and pipeline commands are in their respective modules.

use serde::Serialize;

/// Greet command (basic connectivity test)
#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

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

#[cfg(test)]
mod tests {
    #[test]
    fn test_greet() {
        let result = super::greet("World");
        assert_eq!(result, "Hello, World! You've been greeted from Rust!");
    }

    #[test]
    fn test_greet_empty() {
        let result = super::greet("");
        assert_eq!(result, "Hello, ! You've been greeted from Rust!");
    }
}
