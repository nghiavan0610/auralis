//! State management for the Auralis Tauri application
//!
//! This module provides the application state and settings
//! for the dual-mode (cloud/offline) streaming architecture.

use serde::{Deserialize, Serialize};
use std::process::{Child, ChildStdin};
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
    /// Translation direction: "one_way" or "two_way"
    #[serde(default = "default_translation_type")]
    pub translation_type: String,
    /// Audio source: "microphone", "system", or "both"
    #[serde(default = "default_audio_source")]
    pub audio_source: String,
    /// Window background opacity (0.3–1.0)
    #[serde(default = "default_opacity")]
    pub opacity: f64,
    /// Transcript font size (12–24)
    #[serde(default = "default_font_size")]
    pub font_size: u32,
    /// Maximum number of transcript lines/segments to display (10–200)
    #[serde(default = "default_max_lines")]
    pub max_lines: u32,
    /// Seconds of silence before finalizing a speech segment (VAD endpoint delay, 0.5–3.0)
    #[serde(default = "default_endpoint_delay")]
    pub endpoint_delay: f64,
    /// Whether TTS is enabled (speak translated text aloud)
    #[serde(default = "default_tts_enabled")]
    pub tts_enabled: bool,
}

fn default_translation_type() -> String {
    "one_way".to_string()
}

fn default_audio_source() -> String {
    "microphone".to_string()
}

fn default_opacity() -> f64 {
    0.88
}

fn default_font_size() -> u32 {
    14
}

fn default_max_lines() -> u32 {
    100
}

fn default_endpoint_delay() -> f64 {
    1.0
}

fn default_tts_enabled() -> bool {
    false
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: "cloud".to_string(),
            soniox_api_key: String::new(),
            source_language: "en".to_string(),
            target_language: "vi".to_string(),
            translation_type: default_translation_type(),
            audio_source: default_audio_source(),
            opacity: default_opacity(),
            font_size: default_font_size(),
            max_lines: default_max_lines(),
            endpoint_delay: default_endpoint_delay(),
            tts_enabled: default_tts_enabled(),
        }
    }
}

/// Holds the running Python sidecar process and its stdin handle.
pub struct PipelineState {
    pub child: Child,
    pub stdin: ChildStdin,
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
    /// Running Python pipeline process (None when stopped)
    pub pipeline: Arc<std::sync::Mutex<Option<PipelineState>>>,
    /// Set to true when the Python pipeline emits {"type": "ready"}
    pub pipeline_ready: Arc<AtomicBool>,
}

impl AuralisState {
    /// Create a new application state with defaults
    pub fn new() -> Self {
        Self {
            is_streaming: Arc::new(AtomicBool::new(false)),
            stream_stop: Arc::new(AtomicBool::new(false)),
            settings: Arc::new(Mutex::new(Settings::default())),
            pipeline: Arc::new(std::sync::Mutex::new(None)),
            pipeline_ready: Arc::new(AtomicBool::new(false)),
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
    #[allow(dead_code)]
    pub async fn mode(&self) -> String {
        self.settings.lock().await.mode.clone()
    }

    /// Get the current audio source
    #[allow(dead_code)]
    pub async fn audio_source(&self) -> String {
        self.settings.lock().await.audio_source.clone()
    }

    /// Get the current translation type
    pub async fn translation_type(&self) -> String {
        self.settings.lock().await.translation_type.clone()
    }

    /// Get the current endpoint delay (seconds)
    pub async fn endpoint_delay(&self) -> f64 {
        self.settings.lock().await.endpoint_delay
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
