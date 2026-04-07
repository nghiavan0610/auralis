//! State management for the Auralis Tauri application
//!
//! This module provides the application state, settings, and model status tracking
//! for the dual-mode (cloud/offline) streaming architecture.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;

/// Status of ML models in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStatus {
    /// Whether the STT model is available
    pub stt_available: bool,
    /// STT model name
    pub stt_model: String,
    /// Whether the translation model is available
    pub translation_available: bool,
    /// Translation model name
    pub translation_model: String,
    /// Whether the VAD model is available
    pub vad_available: bool,
    /// VAD model name
    pub vad_model: String,
    /// Overall system ready status
    pub system_ready: bool,
}

impl Default for ModelStatus {
    fn default() -> Self {
        Self {
            stt_available: false,
            stt_model: "Whisper".to_string(),
            translation_available: false,
            translation_model: "Opus-MT".to_string(),
            vad_available: false,
            vad_model: "Silero".to_string(),
            system_ready: false,
        }
    }
}

impl ModelStatus {
    /// Create a new model status
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if all models are available
    pub fn all_available(&self) -> bool {
        self.stt_available && self.translation_available && self.vad_available
    }

    /// Update STT availability
    pub fn with_stt(mut self, available: bool) -> Self {
        self.stt_available = available;
        self.system_ready = self.all_available();
        self
    }

    /// Update translation availability
    pub fn with_translation(mut self, available: bool) -> Self {
        self.translation_available = available;
        self.system_ready = self.all_available();
        self
    }

    /// Update VAD availability
    pub fn with_vad(mut self, available: bool) -> Self {
        self.vad_available = available;
        self.system_ready = self.all_available();
        self
    }
}

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
/// Manages audio streaming state, shared audio buffers, recording flags,
/// user settings, and model availability status.
pub struct AuralisState {
    /// Whether audio is currently being streamed
    pub is_streaming: Arc<AtomicBool>,
    /// Shared audio data buffer (chunks of f32 samples)
    pub audio_data: Arc<Mutex<Vec<Vec<f32>>>>,
    /// Whether the microphone is actively recording
    pub is_recording: Arc<Mutex<bool>>,
    /// Flag to signal the audio stream thread to stop
    pub stream_stop: Arc<AtomicBool>,
    /// User settings (mode, API key, languages)
    pub settings: Arc<Mutex<Settings>>,
    /// ML model availability status
    pub model_status: Arc<Mutex<ModelStatus>>,
}

impl AuralisState {
    /// Create a new application state with defaults
    pub fn new() -> Self {
        Self {
            is_streaming: Arc::new(AtomicBool::new(false)),
            audio_data: Arc::new(Mutex::new(Vec::new())),
            is_recording: Arc::new(Mutex::new(false)),
            stream_stop: Arc::new(AtomicBool::new(false)),
            settings: Arc::new(Mutex::new(Settings::default())),
            model_status: Arc::new(Mutex::new(ModelStatus::default())),
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

    /// Get the current model status
    pub async fn model_status(&self) -> ModelStatus {
        self.model_status.lock().await.clone()
    }

    /// Update model status
    pub async fn update_model_status(&self, status: ModelStatus) {
        *self.model_status.lock().await = status;
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

    #[test]
    fn test_model_status_default() {
        let status = ModelStatus::default();
        assert!(!status.stt_available);
        assert!(!status.translation_available);
        assert!(!status.vad_available);
        assert!(!status.system_ready);
        assert_eq!(status.translation_model, "Opus-MT");
    }

    #[test]
    fn test_model_status_all_available() {
        let status = ModelStatus::new()
            .with_stt(true)
            .with_translation(true)
            .with_vad(true);

        assert!(status.all_available());
        assert!(status.system_ready);
    }

    #[test]
    fn test_model_status_partial() {
        let status = ModelStatus::new()
            .with_stt(true)
            .with_translation(false)
            .with_vad(true);

        assert!(!status.all_available());
        assert!(!status.system_ready);
    }

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

    #[tokio::test]
    async fn test_auralis_state_model_status() {
        let state = AuralisState::new();

        let status = state.model_status().await;
        assert!(!status.system_ready);

        let new_status = ModelStatus::new()
            .with_stt(true)
            .with_translation(true)
            .with_vad(true);

        state.update_model_status(new_status).await;

        let status = state.model_status().await;
        assert!(status.system_ready);
    }

    #[test]
    fn test_is_streaming_initially_false() {
        let state = AuralisState::new();
        assert!(!state.is_streaming.load(Ordering::Relaxed));
    }

    #[test]
    fn test_stream_stop_initially_false() {
        let state = AuralisState::new();
        assert!(!state.stream_stop.load(Ordering::Relaxed));
    }
}
