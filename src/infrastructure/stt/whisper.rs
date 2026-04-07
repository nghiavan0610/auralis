//! Whisper STT engine implementation using whisper-rs
//!
//! This module provides a Speech-to-Text implementation using Whisper.cpp
//! via the whisper-rs binding.

use async_trait::async_trait;
use futures::{stream, Stream, StreamExt};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use tracing::{debug, error, info, warn};

use crate::domain::{errors::STTError, models::STTSegment, traits::STTEngine};

/// Configuration for the Whisper STT engine
#[derive(Debug, Clone)]
pub struct WhisperConfig {
    /// Path to the Whisper model file (.bin)
    pub model_path: PathBuf,

    /// Language code (e.g., "en", "es", "zh") or "auto" for auto-detection
    pub language: String,

    /// Whether to use GPU acceleration (if available)
    pub use_gpu: bool,

    /// Number of threads to use for processing
    pub num_threads: usize,

    /// Whether to enable mock mode (for testing without a model)
    pub mock_mode: bool,
}

impl Default for WhisperConfig {
    fn default() -> Self {
        Self {
            model_path: PathBuf::from("models/ggml-base.bin"),
            language: "auto".to_string(),
            use_gpu: false,
            num_threads: 4,
            mock_mode: false,
        }
    }
}

/// Whisper STT engine implementation
pub struct WhisperEngine {
    config: WhisperConfig,
    context: Option<whisper_rs::WhisperContext>,
    initialized: bool,
    current_language: String,
}

impl WhisperEngine {
    /// Create a new Whisper engine with the given configuration
    pub fn new(config: WhisperConfig) -> Self {
        Self {
            config,
            context: None,
            initialized: false,
            current_language: "auto".to_string(),
        }
    }

    /// Create a new Whisper engine with default configuration
    pub fn with_defaults() -> Self {
        Self::new(WhisperConfig::default())
    }

    /// Create a new Whisper engine in mock mode
    pub fn mock() -> Self {
        let mut config = WhisperConfig::default();
        config.mock_mode = true;
        Self::new(config)
    }

    /// Check if the model file exists
    pub fn model_exists(&self) -> bool {
        self.config.model_path.exists()
    }

    /// Get the model path
    pub fn model_path(&self) -> &Path {
        &self.config.model_path
    }

    /// Download the Whisper model if it doesn't exist
    ///
    /// This is a placeholder for future implementation. In a real application,
    /// you would download the model from Hugging Face or another source.
    pub async fn download_model(&self) -> Result<PathBuf, STTError> {
        if self.model_exists() {
            return Ok(self.config.model_path.clone());
        }

        warn!(
            "Model file not found at {:?}. Download not implemented yet.",
            self.config.model_path
        );

        Err(STTError::ServiceError {
            code: "MODEL_NOT_FOUND".to_string(),
            message: format!("Model file not found and download not implemented"),
        })
    }

    /// Convert audio samples to the format required by Whisper
    ///
    /// Whisper requires 16kHz mono f32 audio samples in the range [-1.0, 1.0]
    fn prepare_audio(&self, audio_data: Vec<f32>) -> Vec<f32> {
        // Ensure audio is in the correct range
        audio_data
            .into_iter()
            .map(|sample| sample.clamp(-1.0, 1.0))
            .collect()
    }

    /// Process audio using the real Whisper context
    fn process_with_whisper(&mut self, audio_data: Vec<f32>) -> Result<Vec<STTSegment>, STTError> {
        let context = self
            .context
            .as_ref()
            .ok_or_else(|| STTError::NotInitialized)?;

        // Prepare audio data
        let audio = self.prepare_audio(audio_data);

        // Create a params object for the full decode
        let mut params = whisper_rs::WhisperParameters::new(
            self.config.num_threads as i32,
            0, // offset
            audio.len() as i32, // length
        );

        // Set language if not auto
        if self.current_language != "auto" {
            // Convert language code to whisper language id
            let lang_id = match self.current_language.as_str() {
                "en" => "en",
                "es" => "es",
                "zh" => "zh",
                "fr" => "fr",
                "de" => "de",
                "ja" => "ja",
                "ko" => "ko",
                _ => "en", // Default to English
            };
            params.set_language(lang_id);
        }

        // Run the full decode
        match context.full(params, &audio) {
            Ok(_) => {
                let mut segments = Vec::new();

                // Get the number of segments
                let num_segments = context.full_n_segments();

                for i in 0..num_segments {
                    // Get the segment text
                    let text = context
                        .full_get_segment_text(i)
                        .unwrap_or_else(|_| "".to_string())
                        .trim()
                        .to_string();

                    if text.is_empty() {
                        continue;
                    }

                    // Get the segment timing
                    let start_time = context.full_get_segment_t0(i);
                    let end_time = context.full_get_segment_t1(i);

                    // Calculate confidence (simplified approach)
                    // In a real implementation, you might use token probabilities
                    let confidence = 0.85; // Default confidence

                    segments.push(STTSegment::new(
                        text,
                        confidence,
                        start_time as u64 * 10, // Convert to milliseconds
                        end_time as u64 * 10,
                        true, // Final segment
                    ));
                }

                debug!("Processed {} Whisper segments", segments.len());
                Ok(segments)
            }
            Err(e) => {
                error!("Whisper processing error: {}", e);
                Err(STTError::ServiceError {
                    code: "WHISPER_ERROR".to_string(),
                    message: format!("Whisper processing failed: {}", e),
                })
            }
        }
    }

    /// Process audio in mock mode (for testing)
    fn process_mock(&self, audio_data: Vec<f32>) -> Result<Vec<STTSegment>, STTError> {
        // Calculate audio energy to determine if there's speech
        let energy: f32 = audio_data.iter().map(|&x| x * x).sum::<f32>() / audio_data.len() as f32;

        if energy < 0.0001 {
            // Too quiet, return empty result
            return Ok(vec![]);
        }

        // Return mock transcription
        let text = if energy > 0.01 {
            "This is a mock transcription with higher energy speech detected."
        } else {
            "This is a mock transcription with lower energy speech detected."
        };

        Ok(vec![STTSegment::new(
            text.to_string(),
            0.9,
            0,
            1000,
            true,
        )])
    }
}

#[async_trait]
impl STTEngine for WhisperEngine {
    async fn initialize(&mut self, config: serde_json::Value) -> Result<(), STTError> {
        info!("Initializing Whisper STT engine");

        // Parse configuration
        if let Some(model_path) = config.get("model_path").and_then(|v| v.as_str()) {
            self.config.model_path = PathBuf::from(model_path);
        }

        if let Some(language) = config.get("language").and_then(|v| v.as_str()) {
            self.config.language = language.to_string();
            self.current_language = self.config.language.clone();
        }

        if let Some(use_gpu) = config.get("use_gpu").and_then(|v| v.as_bool()) {
            self.config.use_gpu = use_gpu;
        }

        if let Some(num_threads) = config.get("num_threads").and_then(|v| v.as_u64()) {
            self.config.num_threads = num_threads as usize;
        }

        if let Some(mock_mode) = config.get("mock_mode").and_then(|v| v.as_bool()) {
            self.config.mock_mode = mock_mode;
        }

        // Check if we're in mock mode
        if self.config.mock_mode {
            info!("Whisper engine running in mock mode");
            self.initialized = true;
            return Ok(());
        }

        // Check if model exists
        if !self.model_exists() {
            warn!("Model file not found, attempting download");
            self.download_model().await?;
        }

        // Load the Whisper model
        match whisper_rs::WhisperContext::new_with_params(
            &self.config.model_path,
            whisper_rs::WhisperContextParameters {
                use_gpu: self.config.use_gpu,
            },
        ) {
            Ok(context) => {
                self.context = Some(context);
                self.initialized = true;
                info!("Whisper STT engine initialized successfully");
                Ok(())
            }
            Err(e) => {
                error!("Failed to load Whisper model: {}", e);
                Err(STTError::ServiceError {
                    code: "MODEL_LOAD_ERROR".to_string(),
                    message: format!("Failed to load Whisper model: {}", e),
                })
            }
        }
    }

    async fn process_audio(&mut self, audio_data: Vec<f32>) -> Result<Vec<STTSegment>, STTError> {
        if !self.initialized {
            return Err(STTError::NotInitialized);
        }

        if audio_data.is_empty() {
            return Err(STTError::AudioTooShort);
        }

        debug!("Processing audio with {} samples", audio_data.len());

        if self.config.mock_mode {
            self.process_mock(audio_data)
        } else {
            self.process_with_whisper(audio_data)
        }
    }

    async fn create_stream(&self) -> Result<Pin<Box<dyn Stream<Item = Result<STTSegment, STTError>> + Send>>, STTError> {
        if !self.initialized {
            return Err(STTError::NotInitialized);
        }

        // Create a mock streaming implementation
        // In a real implementation, this would use whisper's streaming API
        let stream = stream::iter(vec![
            Ok(STTSegment::new(
                "Streaming transcription segment 1".to_string(),
                0.9,
                0,
                500,
                false,
            )),
            Ok(STTSegment::new(
                "Streaming transcription segment 2".to_string(),
                0.85,
                500,
                1000,
                true,
            )),
        ]);

        Ok(Box::pin(stream))
    }

    fn supported_languages(&self) -> Vec<String> {
        vec![
            "auto".to_string(),
            "en".to_string(),
            "es".to_string(),
            "zh".to_string(),
            "fr".to_string(),
            "de".to_string(),
            "ja".to_string(),
            "ko".to_string(),
        ]
    }

    fn current_language(&self) -> &str {
        &self.current_language
    }

    async fn set_language(&mut self, language: String) -> Result<(), STTError> {
        if !self.supported_languages().contains(&language) {
            return Err(STTError::ServiceError {
                code: "UNSUPPORTED_LANGUAGE".to_string(),
                message: format!("Language '{}' is not supported", language),
            });
        }

        self.current_language = language;
        info!("Whisper language set to {}", self.current_language);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whisper_config_default() {
        let config = WhisperConfig::default();
        assert_eq!(config.language, "auto");
        assert!(!config.use_gpu);
        assert_eq!(config.num_threads, 4);
        assert!(!config.mock_mode);
    }

    #[test]
    fn test_whisper_engine_creation() {
        let engine = WhisperEngine::with_defaults();
        assert!(!engine.initialized);
        assert_eq!(engine.current_language(), "auto");
    }

    #[test]
    fn test_whisper_engine_mock() {
        let engine = WhisperEngine::mock();
        assert!(engine.config.mock_mode);
    }

    #[test]
    fn test_model_exists() {
        let config = WhisperConfig {
            model_path: PathBuf::from("/nonexistent/path.bin"),
            ..Default::default()
        };
        let engine = WhisperEngine::new(config);
        assert!(!engine.model_exists());
    }

    #[test]
    fn test_supported_languages() {
        let engine = WhisperEngine::mock();
        let languages = engine.supported_languages();
        assert!(languages.contains(&"en".to_string()));
        assert!(languages.contains(&"es".to_string()));
        assert!(languages.contains(&"zh".to_string()));
    }

    #[tokio::test]
    async fn test_initialize_mock_mode() {
        let mut engine = WhisperEngine::mock();
        let config = serde_json::json!({
            "mock_mode": true,
            "language": "en"
        });

        let result = engine.initialize(config).await;
        assert!(result.is_ok());
        assert!(engine.initialized);
        assert_eq!(engine.current_language(), "en");
    }

    #[tokio::test]
    async fn test_process_audio_mock() {
        let mut engine = WhisperEngine::mock();
        engine.config.mock_mode = true;
        engine.initialized = true;

        // Test with quiet audio
        let quiet_audio = vec![0.0f32; 1000];
        let result = engine.process_audio(quiet_audio).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());

        // Test with speech audio
        let speech_audio = vec![0.1f32; 1000];
        let result = engine.process_audio(speech_audio).await;
        assert!(result.is_ok());
        let segments = result.unwrap();
        assert_eq!(segments.len(), 1);
        assert!(segments[0].text.contains("mock transcription"));
    }

    #[tokio::test]
    async fn test_process_audio_not_initialized() {
        let mut engine = WhisperEngine::mock();
        // Don't initialize

        let audio_data = vec![0.1f32; 1000];
        let result = engine.process_audio(audio_data).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), STTError::NotInitialized));
    }

    #[tokio::test]
    async fn test_process_audio_empty() {
        let mut engine = WhisperEngine::mock();
        engine.initialized = true;

        let audio_data = vec![];
        let result = engine.process_audio(audio_data).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), STTError::AudioTooShort));
    }

    #[tokio::test]
    async fn test_set_language() {
        let mut engine = WhisperEngine::mock();
        engine.initialized = true;

        let result = engine.set_language("es".to_string()).await;
        assert!(result.is_ok());
        assert_eq!(engine.current_language(), "es");
    }

    #[tokio::test]
    async fn test_set_unsupported_language() {
        let mut engine = WhisperEngine::mock();
        engine.initialized = true;

        let result = engine.set_language("xx".to_string()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_stream() {
        let mut engine = WhisperEngine::mock();
        engine.initialized = true;

        let result = engine.create_stream().await;
        assert!(result.is_ok());

        let mut stream = result.unwrap();
        // Collect the first segment
        let segment = stream.next().await.unwrap().unwrap();
        assert!(segment.text.contains("streaming"));
    }

    #[tokio::test]
    async fn test_create_stream_not_initialized() {
        let engine = WhisperEngine::mock();
        // Don't initialize

        let result = engine.create_stream().await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), STTError::NotInitialized));
    }

    #[test]
    fn test_prepare_audio() {
        let engine = WhisperEngine::mock();

        // Test with normal audio
        let audio = vec![0.5f32, -0.3f32, 0.8f32];
        let prepared = engine.prepare_audio(audio);
        assert_eq!(prepared, vec![0.5f32, -0.3f32, 0.8f32]);

        // Test with clipping
        let audio = vec![1.5f32, -2.0f32, 0.5f32];
        let prepared = engine.prepare_audio(audio);
        assert_eq!(prepared, vec![1.0f32, -1.0f32, 0.5f32]);
    }
}
