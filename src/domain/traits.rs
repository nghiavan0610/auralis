use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

use crate::domain::{errors::*, models::STTSegment};

/// Audio configuration parameters
#[derive(Debug, Clone)]
pub struct AudioConfig {
    pub sample_rate: u32,
    pub channels: u16,
    pub buffer_size: usize,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16000,
            channels: 1,
            buffer_size: 4096,
        }
    }
}

/// Represents a stream of audio data
pub type AudioStream = Pin<Box<dyn Stream<Item = Result<Vec<f32>, AudioError>> + Send>>;

/// Represents a stream of STT segments
pub type STTStream = Pin<Box<dyn Stream<Item = Result<STTSegment, STTError>> + Send>>;

/// Trait for audio sources (microphone, file, etc.)
#[async_trait]
pub trait AudioSource: Send + Sync {
    /// Start capturing audio
    async fn start(&mut self) -> Result<(), AudioError>;

    /// Stop capturing audio
    async fn stop(&mut self) -> Result<(), AudioError>;

    /// Get a stream of audio data
    fn stream(&self) -> Result<AudioStream, AudioError>;

    /// Get the current audio configuration
    fn config(&self) -> &AudioConfig;

    /// Check if the audio source is currently active
    fn is_active(&self) -> bool;
}

/// Trait for speech-to-text engines
#[async_trait]
pub trait STTEngine: Send + Sync {
    /// Initialize the STT engine with the given configuration
    async fn initialize(&mut self, config: serde_json::Value) -> Result<(), STTError>;

    /// Process audio data and return STT segments
    async fn process_audio(&mut self, audio_data: Vec<f32>) -> Result<Vec<STTSegment>, STTError>;

    /// Create a streaming STT session
    async fn create_stream(&self) -> Result<STTStream, STTError>;

    /// Get supported languages
    fn supported_languages(&self) -> Vec<String>;

    /// Get the current language
    fn current_language(&self) -> &str;

    /// Set the language for STT
    async fn set_language(&mut self, language: String) -> Result<(), STTError>;
}

/// Trait for translation engines
#[async_trait]
pub trait Translator: Send + Sync {
    /// Initialize the translator with the given configuration
    async fn initialize(&mut self, config: serde_json::Value) -> Result<(), TranslationError>;

    /// Translate text from source language to target language
    async fn translate(
        &mut self,
        text: String,
        source_lang: String,
        target_lang: String,
    ) -> Result<crate::domain::models::Translation, TranslationError>;

    /// Get supported language pairs
    fn supported_pairs(&self) -> Vec<(String, String)>;

    /// Check if a language pair is supported
    fn is_pair_supported(&self, source: &str, target: &str) -> bool;

    /// Detect the language of the given text
    async fn detect_language(&self, text: String) -> Result<String, TranslationError>;
}

/// Trait for voice activity detection
#[async_trait]
pub trait VAD: Send + Sync {
    /// Initialize the VAD with the given configuration
    async fn initialize(&mut self, config: serde_json::Value) -> Result<(), VADError>;

    /// Process audio data and return whether speech is detected
    async fn process_audio(&mut self, audio_data: Vec<f32>) -> Result<bool, VADError>;

    /// Get the speech probability (0.0 to 1.0)
    async fn speech_probability(&self, audio_data: Vec<f32>) -> Result<f32, VADError>;

    /// Reset the VAD state
    async fn reset(&mut self) -> Result<(), VADError>;

    /// Get the current threshold
    fn threshold(&self) -> f32;

    /// Set the threshold for speech detection
    async fn set_threshold(&mut self, threshold: f32) -> Result<(), VADError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use futures::stream;

    // Mock AudioSource for testing
    struct MockAudioSource {
        active: bool,
        config: AudioConfig,
    }

    #[async_trait]
    impl AudioSource for MockAudioSource {
        async fn start(&mut self) -> Result<(), AudioError> {
            self.active = true;
            Ok(())
        }

        async fn stop(&mut self) -> Result<(), AudioError> {
            self.active = false;
            Ok(())
        }

        fn stream(&self) -> Result<AudioStream, AudioError> {
            let audio_data = vec![0.0f32; 1024];
            let stream = stream::iter(vec![Ok(audio_data)]);
            Ok(Box::pin(stream))
        }

        fn config(&self) -> &AudioConfig {
            &self.config
        }

        fn is_active(&self) -> bool {
            self.active
        }
    }

    // Mock STTEngine for testing
    struct MockSTTEngine {
        language: String,
        initialized: bool,
    }

    #[async_trait]
    impl STTEngine for MockSTTEngine {
        async fn initialize(&mut self, _config: serde_json::Value) -> Result<(), STTError> {
            self.initialized = true;
            Ok(())
        }

        async fn process_audio(&mut self, _audio_data: Vec<f32>) -> Result<Vec<STTSegment>, STTError> {
            if !self.initialized {
                return Err(STTError::NotInitialized);
            }

            Ok(vec![STTSegment::new(
                "Test transcript".to_string(),
                0.95,
                0,
                1000,
                true,
            )])
        }

        async fn create_stream(&self) -> Result<STTStream, STTError> {
            if !self.initialized {
                return Err(STTError::NotInitialized);
            }

            let segment = STTSegment::new("Streaming text".to_string(), 0.9, 0, 500, true);
            let stream = stream::iter(vec![Ok(segment)]);
            Ok(Box::pin(stream))
        }

        fn supported_languages(&self) -> Vec<String> {
            vec!["en".to_string(), "es".to_string(), "zh".to_string()]
        }

        fn current_language(&self) -> &str {
            &self.language
        }

        async fn set_language(&mut self, language: String) -> Result<(), STTError> {
            self.language = language;
            Ok(())
        }
    }

    // Mock Translator for testing
    struct MockTranslator {
        initialized: bool,
    }

    #[async_trait]
    impl Translator for MockTranslator {
        async fn initialize(&mut self, _config: serde_json::Value) -> Result<(), TranslationError> {
            self.initialized = true;
            Ok(())
        }

        async fn translate(
            &mut self,
            text: String,
            source_lang: String,
            target_lang: String,
        ) -> Result<crate::domain::models::Translation, TranslationError> {
            if !self.initialized {
                return Err(TranslationError::EmptyText);
            }

            Ok(crate::domain::models::Translation::new(
                source_lang,
                target_lang,
                text.clone(),
                format!("{} (translated)", text),
                0.9,
            ))
        }

        fn supported_pairs(&self) -> Vec<(String, String)> {
            vec![
                ("en".to_string(), "es".to_string()),
                ("en".to_string(), "zh".to_string()),
            ]
        }

        fn is_pair_supported(&self, source: &str, target: &str) -> bool {
            self.supported_pairs().contains(&(source.to_string(), target.to_string()))
        }

        async fn detect_language(&self, _text: String) -> Result<String, TranslationError> {
            if !self.initialized {
                return Err(TranslationError::LanguageDetectionFailed);
            }
            Ok("en".to_string())
        }
    }

    // Mock VAD for testing
    struct MockVAD {
        threshold: f32,
        initialized: bool,
    }

    #[async_trait]
    impl VAD for MockVAD {
        async fn initialize(&mut self, _config: serde_json::Value) -> Result<(), VADError> {
            self.initialized = true;
            Ok(())
        }

        async fn process_audio(&mut self, audio_data: Vec<f32>) -> Result<bool, VADError> {
            if !self.initialized {
                return Err(VADError::ProcessingError("Not initialized".to_string()));
            }

            // Simple mock logic: detect speech if audio has any non-zero values
            Ok(audio_data.iter().any(|&x| x.abs() > 0.001))
        }

        async fn speech_probability(&self, _audio_data: Vec<f32>) -> Result<f32, VADError> {
            if !self.initialized {
                return Err(VADError::PredictionError("Not initialized".to_string()));
            }
            Ok(0.85)
        }

        async fn reset(&mut self) -> Result<(), VADError> {
            Ok(())
        }

        fn threshold(&self) -> f32 {
            self.threshold
        }

        async fn set_threshold(&mut self, threshold: f32) -> Result<(), VADError> {
            if threshold < 0.0 || threshold > 1.0 {
                return Err(VADError::InvalidThreshold { threshold });
            }
            self.threshold = threshold;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_audio_source() {
        let mut source = MockAudioSource {
            active: false,
            config: AudioConfig::default(),
        };

        assert!(!source.is_active());

        source.start().await.unwrap();
        assert!(source.is_active());

        let stream = source.stream();
        assert!(stream.is_ok());

        source.stop().await.unwrap();
        assert!(!source.is_active());
    }

    #[tokio::test]
    async fn test_stt_engine() {
        let mut engine = MockSTTEngine {
            language: "en".to_string(),
            initialized: false,
        };

        // Test initialization
        engine.initialize(serde_json::json!({})).await.unwrap();
        assert!(engine.initialized);

        // Test processing audio
        let audio_data = vec![0.1f32; 1024];
        let segments = engine.process_audio(audio_data).await.unwrap();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].text, "Test transcript");

        // Test streaming
        let stream = engine.create_stream().await.unwrap();
        // Stream is valid but we don't consume it in this test

        // Test language support
        assert_eq!(engine.current_language(), "en");
        assert!(engine.supported_languages().contains(&"en".to_string()));

        engine.set_language("es".to_string()).await.unwrap();
        assert_eq!(engine.current_language(), "es");
    }

    #[tokio::test]
    async fn test_translator() {
        let mut translator = MockTranslator {
            initialized: false,
        };

        translator.initialize(serde_json::json!({})).await.unwrap();

        let translation = translator
            .translate("Hello".to_string(), "en".to_string(), "es".to_string())
            .await
            .unwrap();

        assert_eq!(translation.original_text, "Hello");
        assert_eq!(translation.translated_text, "Hello (translated)");
        assert_eq!(translation.source_lang, "en");
        assert_eq!(translation.target_lang, "es");

        // Test language pair support
        assert!(translator.is_pair_supported("en", "es"));
        assert!(!translator.is_pair_supported("en", "fr"));

        // Test language detection
        let detected = translator.detect_language("test text".to_string()).await.unwrap();
        assert_eq!(detected, "en");
    }

    #[tokio::test]
    async fn test_vad() {
        let mut vad = MockVAD {
            threshold: 0.5,
            initialized: false,
        };

        vad.initialize(serde_json::json!({})).await.unwrap();

        // Test threshold
        assert_eq!(vad.threshold(), 0.5);
        vad.set_threshold(0.8).await.unwrap();
        assert_eq!(vad.threshold(), 0.8);

        // Test invalid threshold
        assert!(vad.set_threshold(1.5).await.is_err());

        // Test speech detection
        let silent_audio = vec![0.0f32; 1024];
        assert!(!vad.process_audio(silent_audio).await.unwrap());

        let speech_audio = vec![0.1f32; 1024];
        assert!(vad.process_audio(speech_audio).await.unwrap());

        // Test speech probability
        let prob = vad.speech_probability(vec![0.1f32; 1024]).await.unwrap();
        assert_eq!(prob, 0.85);

        // Test reset
        vad.reset().await.unwrap();
    }

    #[test]
    fn test_audio_config_default() {
        let config = AudioConfig::default();
        assert_eq!(config.sample_rate, 16000);
        assert_eq!(config.channels, 1);
        assert_eq!(config.buffer_size, 4096);
    }

    #[tokio::test]
    async fn test_stt_not_initialized() {
        let mut engine = MockSTTEngine {
            language: "en".to_string(),
            initialized: false,
        };

        let audio_data = vec![0.1f32; 1024];
        let result = engine.process_audio(audio_data).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), STTError::NotInitialized));
    }

    #[tokio::test]
    async fn test_vad_not_initialized() {
        let vad = MockVAD {
            threshold: 0.5,
            initialized: false,
        };

        let result = vad.speech_probability(vec![0.1f32; 1024]).await;
        assert!(result.is_err());
    }
}