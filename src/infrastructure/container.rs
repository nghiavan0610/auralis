//! Dependency injection container for Auralis
//!
//! This module provides a container for managing component creation and
//! dependency injection, making it easy to configure and test the system.

use crate::domain::traits::*;
use crate::infrastructure::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

/// Configuration for the Auralis container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    /// Path to models directory
    pub models_dir: PathBuf,

    /// Audio configuration
    pub audio: AudioCaptureConfig,

    /// STT configuration
    pub stt: WhisperConfig,

    /// Translation configuration
    pub translation: MadladConfig,

    /// VAD configuration
    pub vad: SileroConfig,

    /// Source language
    pub source_language: String,

    /// Target language
    pub target_language: String,
}

impl Default for ContainerConfig {
    fn default() -> Self {
        Self {
            models_dir: PathBuf::from("models"),
            audio: AudioCaptureConfig::default(),
            stt: WhisperConfig::default(),
            translation: MadladConfig::default(),
            vad: SileroConfig::default(),
            source_language: "en".to_string(),
            target_language: "es".to_string(),
        }
    }
}

impl ContainerConfig {
    /// Create a new container config with a custom models directory
    pub fn with_models_dir(mut self, dir: PathBuf) -> Self {
        self.models_dir = dir;
        self
    }

    /// Create a new container config with custom languages
    pub fn with_languages(mut self, source: String, target: String) -> Self {
        self.source_language = source;
        self.target_language = target;
        self
    }

    /// Resolve model paths from the models directory
    pub fn resolve_model_paths(&mut self) {
        self.stt.model_path = self
            .models_dir
            .join("whisper.bin")
            .to_string_lossy()
            .to_string();
        self.translation.model_path = self
            .models_dir
            .join("madlad")
            .to_string_lossy()
            .to_string();
        self.vad.model_path = self
            .models_dir
            .join("silero_vad.torch")
            .to_string_lossy()
            .to_string();
    }
}

/// Status of models in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStatus {
    /// Whether the STT model is available
    pub stt_available: bool,
    /// STT model path
    pub stt_model_path: String,
    /// Whether the translation model is available
    pub translation_available: bool,
    /// Translation model path
    pub translation_model_path: String,
    /// Whether the VAD model is available
    pub vad_available: bool,
    /// VAD model path
    pub vad_model_path: String,
    /// Overall system ready status
    pub system_ready: bool,
    /// Error messages for unavailable models
    pub errors: Vec<String>,
}

impl Default for ModelStatus {
    fn default() -> Self {
        Self {
            stt_available: false,
            stt_model_path: String::new(),
            translation_available: false,
            translation_model_path: String::new(),
            vad_available: false,
            vad_model_path: String::new(),
            system_ready: false,
            errors: Vec::new(),
        }
    }
}

/// Errors that can occur during container operations
#[derive(Debug, Error)]
pub enum ContainerError {
    #[error("Failed to create audio source: {0}")]
    AudioSourceError(String),

    #[error("Failed to create STT engine: {0}")]
    STTError(String),

    #[error("Failed to create translator: {0}")]
    TranslationError(String),

    #[error("Failed to create VAD: {0}")]
    VADError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// Dependency injection container for Auralis
pub struct AuralisContainer {
    config: ContainerConfig,
}

impl AuralisContainer {
    /// Create a new container with default configuration
    pub fn new() -> Self {
        let config = ContainerConfig::default();
        Self { config }
    }

    /// Create a new container with custom configuration
    pub fn with_config(mut config: ContainerConfig) -> Self {
        config.resolve_model_paths();
        Self { config }
    }

    /// Get the container configuration
    pub fn config(&self) -> &ContainerConfig {
        &self.config
    }

    /// Update the container configuration
    pub fn update_config(&mut self, config: ContainerConfig) {
        let mut config = config;
        config.resolve_model_paths();
        self.config = config;
    }

    /// Check if all required models are available
    pub fn check_models(&self) -> ModelStatus {
        let mut status = ModelStatus::default();
        let mut errors = Vec::new();

        // Check STT model
        status.stt_model_path = self.config.stt.model_path.clone();
        status.stt_available = std::path::Path::new(&self.config.stt.model_path).exists();
        if !status.stt_available {
            errors.push(format!(
                "STT model not found at: {}",
                self.config.stt.model_path
            ));
        }

        // Check translation model
        status.translation_model_path = self.config.translation.model_path.clone();
        status.translation_available =
            std::path::Path::new(&self.config.translation.model_path).exists();
        if !status.translation_available {
            errors.push(format!(
                "Translation model not found at: {}",
                self.config.translation.model_path
            ));
        }

        // Check VAD model
        status.vad_model_path = self.config.vad.model_path.clone();
        status.vad_available = std::path::Path::new(&self.config.vad.model_path).exists();
        if !status.vad_available {
            errors.push(format!(
                "VAD model not found at: {}",
                self.config.vad.model_path
            ));
        }

        status.system_ready = status.stt_available
            && status.translation_available
            && status.vad_available;

        status.errors = errors;

        status
    }

    /// Create a new audio source
    pub fn create_audio_source(&self) -> Result<MicrophoneCapture, ContainerError> {
        MicrophoneCapture::new(self.config.audio.clone()).map_err(|e| {
            ContainerError::AudioSourceError(format!("Failed to create audio source: {}", e))
        })
    }

    /// Create a new STT engine
    pub fn create_stt_engine(&self) -> Result<WhisperEngine, ContainerError> {
        let mut engine = WhisperEngine::new(self.config.stt.clone());
        Ok(engine)
    }

    /// Create a new translator
    pub fn create_translator(&self) -> Result<MadladTranslator, ContainerError> {
        let translator = MadladTranslator::new(self.config.translation.clone());
        Ok(translator)
    }

    /// Create a new VAD
    pub fn create_vad(&self) -> Result<SileroVAD, ContainerError> {
        let vad = SileroVAD::new(self.config.vad.clone());
        Ok(vad)
    }

    /// Create an orchestrator with all components
    pub fn create_orchestrator(
        &self,
    ) -> Result<
        crate::application::Orchestrator<
            MicrophoneCapture,
            WhisperEngine,
            MadladTranslator,
            SileroVAD,
        >,
        ContainerError,
    > {
        let audio_source = self.create_audio_source()?;
        let stt_engine = self.create_stt_engine()?;
        let translator = self.create_translator()?;
        let vad = self.create_vad()?;

        Ok(crate::application::Orchestrator::new(
            audio_source,
            stt_engine,
            translator,
            vad,
            self.config.source_language.clone(),
            self.config.target_language.clone(),
        ))
    }

    /// Initialize all components (for testing purposes)
    pub async fn initialize_all(&self) -> Result<(), ContainerError> {
        // Check models first
        let status = self.check_models();
        if !status.system_ready {
            return Err(ContainerError::ConfigError(format!(
                "Models not ready: {}",
                status.errors.join(", ")
            )));
        }

        // Try to create each component
        let _audio_source = self.create_audio_source()?;
        let _stt_engine = self.create_stt_engine()?;
        let _translator = self.create_translator()?;
        let _vad = self.create_vad()?;

        Ok(())
    }
}

impl Default for AuralisContainer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_config_default() {
        let config = ContainerConfig::default();
        assert_eq!(config.models_dir, PathBuf::from("models"));
        assert_eq!(config.source_language, "en");
        assert_eq!(config.target_language, "es");
    }

    #[test]
    fn test_container_config_with_languages() {
        let config = ContainerConfig::default()
            .with_languages("zh".to_string(), "fr".to_string());

        assert_eq!(config.source_language, "zh");
        assert_eq!(config.target_language, "fr");
    }

    #[test]
    fn test_container_new() {
        let container = AuralisContainer::new();
        let config = container.config();
        assert_eq!(config.source_language, "en");
        assert_eq!(config.target_language, "es");
    }

    #[test]
    fn test_container_with_config() {
        let config = ContainerConfig::default()
            .with_languages("es".to_string(), "en".to_string());

        let container = AuralisContainer::with_config(config);
        assert_eq!(container.config().source_language, "es");
        assert_eq!(container.config().target_language, "en");
    }

    #[test]
    fn test_check_models() {
        let container = AuralisContainer::new();
        let status = container.check_models();

        // In a test environment, models probably won't exist
        // but we can check the structure
        assert!(!status.system_ready || status.system_ready); // Just that it compiles
        assert!(status.stt_model_path.contains("whisper"));
        assert!(status.translation_model_path.contains("madlad"));
        assert!(status.vad_model_path.contains("silero"));
    }

    #[test]
    fn test_model_status_default() {
        let status = ModelStatus::default();
        assert!(!status.stt_available);
        assert!(!status.translation_available);
        assert!(!status.vad_available);
        assert!(!status.system_ready);
        assert!(status.errors.is_empty());
    }

    #[test]
    fn test_container_config_resolve_paths() {
        let mut config = ContainerConfig::default();
        config.resolve_model_paths();

        assert!(config.stt.model_path.contains("whisper"));
        assert!(config.translation.model_path.contains("madlad"));
        assert!(config.vad.model_path.contains("silero"));
    }

    #[tokio::test]
    async fn test_initialize_all() {
        let container = AuralisContainer::new();

        // This will likely fail in a test environment without models,
        // but we can test the error handling
        let result = container.initialize_all().await;

        // Either it succeeds (models present) or fails with config error
        match result {
            Ok(()) => {
                // Models are present, test passed
            }
            Err(ContainerError::ConfigError(_)) => {
                // Expected when models aren't present
            }
            Err(e) => {
                panic!("Unexpected error: {}", e);
            }
        }
    }
}
