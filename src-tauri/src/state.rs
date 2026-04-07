//! State management for the Auralis Tauri application
//!
//! This module provides the application state and model status tracking.

use auralis::domain::traits::*;
use auralis::infrastructure::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
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
            translation_model: "MADLAD".to_string(),
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

/// Main application state
pub struct AuralisState {
    /// The orchestrator instance (created when translation starts)
    orchestrator: Option<Arc<auralis::Orchestrator<
        auralis::MicrophoneCapture,
        auralis::WhisperEngine,
        auralis::MadladTranslator,
        auralis::SileroVAD,
    >>>,
    /// Model status
    model_status: Arc<Mutex<ModelStatus>>,
    /// Current source language
    source_language: Arc<Mutex<String>>,
    /// Current target language
    target_language: Arc<Mutex<String>>,
}

impl AuralisState {
    /// Create a new application state
    pub fn new() -> Self {
        Self {
            orchestrator: None,
            model_status: Arc::new(Mutex::new(ModelStatus::new())),
            source_language: Arc::new(Mutex::new("en".to_string())),
            target_language: Arc::new(Mutex::new("es".to_string())),
        }
    }

    /// Get the model status
    pub async fn model_status(&self) -> ModelStatus {
        self.model_status.lock().await.clone()
    }

    /// Update model status
    pub async fn update_model_status(&self, status: ModelStatus) {
        *self.model_status.lock().await = status;
    }

    /// Get the source language
    pub async fn source_language(&self) -> String {
        self.source_language.lock().await.clone()
    }

    /// Set the source language
    pub async fn set_source_language(&self, language: String) {
        *self.source_language.lock().await = language;
    }

    /// Get the target language
    pub async fn target_language(&self) -> String {
        self.target_language.lock().await.clone()
    }

    /// Set the target language
    pub async fn set_target_language(&self, language: String) {
        *self.target_language.lock().await = language;
    }

    /// Set the orchestrator
    pub fn set_orchestrator(
        &mut self,
        orchestrator: Arc<auralis::Orchestrator<
            auralis::MicrophoneCapture,
            auralis::WhisperEngine,
            auralis::MadladTranslator,
            auralis::SileroVAD,
        >>,
    ) {
        self.orchestrator = Some(orchestrator);
    }

    /// Get the orchestrator
    pub fn orchestrator(
        &self,
    ) -> Option<
        &Arc<
            auralis::Orchestrator<
                auralis::MicrophoneCapture,
                auralis::WhisperEngine,
                auralis::MadladTranslator,
                auralis::SileroVAD,
            >,
        >,
    > {
        self.orchestrator.as_ref()
    }

    /// Check if translation is running
    pub async fn is_translating(&self) -> bool {
        if let Some(orch) = &self.orchestrator {
            orch.is_running().await
        } else {
            false
        }
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
    async fn test_auralis_state_languages() {
        let state = AuralisState::new();

        assert_eq!(state.source_language().await, "en");
        assert_eq!(state.target_language().await, "es");

        state.set_source_language("zh".to_string()).await;
        state.set_target_language("fr".to_string()).await;

        assert_eq!(state.source_language().await, "zh");
        assert_eq!(state.target_language().await, "fr");
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

    #[tokio::test]
    async fn test_auralis_state_is_translating() {
        let state = AuralisState::new();

        // No orchestrator, not translating
        assert!(!state.is_translating().await);
    }
}
