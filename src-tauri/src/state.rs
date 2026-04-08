//! State management for the Auralis Tauri application
//!
//! This module provides the application state and settings
//! for the dual-mode (cloud/offline) streaming architecture.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tokio::sync::Mutex;

/// Application settings for dual-mode operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Operating mode: "cloud" (Soniox) or "offline" (Python MLX sidecar)
    pub mode: String,
    /// Soniox API key for cloud mode
    pub soniox_api_key: String,
    /// Source language code (e.g., "en")
    pub source_language: String,
    /// Target language code (e.g., "vi")
    pub target_language: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: "cloud".to_string(),
            soniox_api_key: String::new(),
            source_language: "en".to_string(),
            target_language: "vi".to_string(),
        }
    }
}

/// Main application state for the streaming architecture
///
/// Manages audio streaming state, recording flags, and user settings.
pub struct AuralisState {
    /// Whether audio is currently being streamed
    pub is_streaming: Arc<AtomicBool>,
    /// Flag to signal the audio stream thread to stop
    pub stream_stop: Arc<AtomicBool>,
    /// User settings (mode, API key, languages)
    pub settings: Arc<Mutex<Settings>>,
}

impl AuralisState {
    /// Create a new application state with defaults
    pub fn new() -> Self {
        Self {
            is_streaming: Arc::new(AtomicBool::new(false)),
            stream_stop: Arc::new(AtomicBool::new(false)),
            settings: Arc::new(Mutex::new(Settings::default())),
        }
    }

    /// Get the current source language from settings
    pub async fn source_language(&self) -> String {
        self.settings.lock().await.source_language.clone()
    }

    /// Get the current target language from settings
    pub async fn target_language(&self) -> String {
        self.settings.lock().await.target_language.clone()
    }

    /// Get the current operating mode
    pub async fn mode(&self) -> String {
        self.settings.lock().await.mode.clone()
    }
}

impl Default for AuralisState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_settings_default() {
        let state = AuralisState::new();

        assert_eq!(state.source_language().await, "en");
        assert_eq!(state.target_language().await, "vi");
        assert_eq!(state.mode().await, "cloud");
    }

    #[tokio::test]
    async fn test_settings_update() {
        let state = AuralisState::new();

        {
            let mut settings = state.settings.lock().await;
            settings.source_language = "zh".to_string();
            settings.target_language = "fr".to_string();
            settings.mode = "offline".to_string();
        }

        assert_eq!(state.source_language().await, "zh");
        assert_eq!(state.target_language().await, "fr");
        assert_eq!(state.mode().await, "offline");
    }

    #[test]
    fn test_is_streaming_initially_false() {
        let state = AuralisState::new();
        assert!(!state.is_streaming.load(std::sync::atomic::Ordering::Relaxed));
    }

    #[test]
    fn test_stream_stop_initially_false() {
        let state = AuralisState::new();
        assert!(!state.stream_stop.load(std::sync::atomic::Ordering::Relaxed));
    }
}
