use thiserror::Error;

/// Errors that can occur during audio capture and processing
#[derive(Error, Debug)]
pub enum AudioError {
    #[error("Failed to open audio device: {device}")]
    DeviceOpenError { device: String },

    #[error("Audio configuration error: {message}")]
    ConfigurationError { message: String },

    #[error("Failed to read audio data: {0}")]
    ReadError(#[from] std::io::Error),

    #[error("Audio format not supported: {format}")]
    UnsupportedFormat { format: String },

    #[error("Audio stream closed unexpectedly")]
    StreamClosed,

    #[error("Buffer overrun: audio data processing too slow")]
    BufferOverrun,

    #[error("Failed to initialize audio subsystem: {0}")]
    InitializationError(String),
}

/// Errors that can occur during speech-to-text processing
#[derive(Error, Debug)]
pub enum STTError {
    #[error("Failed to connect to STT service: {service}")]
    ConnectionError { service: String },

    #[error("STT request timeout after {seconds}s")]
    Timeout { seconds: u64 },

    #[error("Invalid audio format for STT: {format}")]
    InvalidAudioFormat { format: String },

    #[error("STT service returned an error: {code} - {message}")]
    ServiceError { code: String, message: String },

    #[error("Rate limit exceeded for STT service")]
    RateLimitExceeded,

    #[error("Invalid API key for STT service")]
    InvalidApiKey,

    #[error("Audio data too short for processing")]
    AudioTooShort,

    #[error("Failed to parse STT response: {0}")]
    ParseError(String),

    #[error("STT engine not initialized")]
    NotInitialized,
}

/// Errors that can occur during translation
#[derive(Error, Debug)]
pub enum TranslationError {
    #[error("Unsupported language pair: {source} -> {target}")]
    UnsupportedLanguagePair { source: String, target: String },

    #[error("Failed to connect to translation service: {service}")]
    ConnectionError { service: String },

    #[error("Translation request timeout after {seconds}s")]
    Timeout { seconds: u64 },

    #[error("Translation service returned an error: {code} - {message}")]
    ServiceError { code: String, message: String },

    #[error("Rate limit exceeded for translation service")]
    RateLimitExceeded,

    #[error("Invalid API key for translation service")]
    InvalidApiKey,

    #[error("Text too long for translation: {length} characters (max: {max_length})")]
    TextTooLong { length: usize, max_length: usize },

    #[error("Empty text provided for translation")]
    EmptyText,

    #[error("Failed to detect language of input text")]
    LanguageDetectionFailed,

    #[error("Translation quality below threshold: {score} (required: {required})")]
    QualityTooLow { score: f32, required: f32 },
}

/// Errors that can occur during voice activity detection
#[derive(Error, Debug)]
pub enum VADError {
    #[error("Failed to initialize VAD model: {model}")]
    InitializationError { model: String },

    #[error("VAD processing error: {0}")]
    ProcessingError(String),

    #[error("Invalid VAD threshold: {threshold} (must be between 0.0 and 1.0)")]
    InvalidThreshold { threshold: f32 },

    #[error("VAD model file not found: {path}")]
    ModelNotFound { path: String },

    #[error("VAD prediction failed: {0}")]
    PredictionError(String),

    #[error("Invalid audio sample rate: {rate} Hz (supported: {supported})")]
    InvalidSampleRate { rate: u32, supported: String },
}

/// Errors that can occur during configuration
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Configuration file not found: {path}")]
    FileNotFound { path: String },

    #[error("Failed to parse configuration file: {0}")]
    ParseError(#[from] serde_json::Error),

    #[error("Invalid configuration value for {key}: {value}")]
    InvalidValue { key: String, value: String },

    #[error("Missing required configuration key: {key}")]
    MissingKey { key: String },

    #[error("Configuration validation failed: {0}")]
    ValidationError(String),

    #[error("Failed to load configuration: {0}")]
    LoadError(String),

    #[error("Environment variable not found: {0}")]
    EnvVarNotFound(String),

    #[error("Invalid configuration format: expected {expected}, found {found}")]
    InvalidFormat { expected: String, found: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_error_display() {
        let error = AudioError::DeviceOpenError {
            device: "default".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "Failed to open audio device: default"
        );
    }

    #[test]
    fn test_stt_error_display() {
        let error = STTError::Timeout { seconds: 30 };
        assert_eq!(error.to_string(), "STT request timeout after 30s");
    }

    #[test]
    fn test_translation_error_display() {
        let error = TranslationError::UnsupportedLanguagePair {
            source: "en".to_string(),
            target: "xx".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "Unsupported language pair: en -> xx"
        );
    }

    #[test]
    fn test_vad_error_display() {
        let error = VADError::InvalidThreshold { threshold: 1.5 };
        assert_eq!(
            error.to_string(),
            "Invalid VAD threshold: 1.5 (must be between 0.0 and 1.0)"
        );
    }

    #[test]
    fn test_config_error_display() {
        let error = ConfigError::MissingKey {
            key: "api_key".to_string(),
        };
        assert_eq!(error.to_string(), "Missing required configuration key: api_key");
    }

    #[test]
    fn test_error_chain() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let audio_error = AudioError::ReadError(io_error);

        assert!(audio_error.to_string().contains("file not found"));
    }

    #[test]
    fn test_service_error() {
        let error = STTError::ServiceError {
            code: "500".to_string(),
            message: "Internal server error".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "STT service returned an error: 500 - Internal server error"
        );
    }

    #[test]
    fn test_config_parse_error() {
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json")
            .unwrap_err();
        let config_error = ConfigError::ParseError(json_error);

        assert!(config_error.to_string().contains("parse"));
    }

    #[test]
    fn test_quality_error() {
        let error = TranslationError::QualityTooLow {
            score: 0.6,
            required: 0.8,
        };
        assert_eq!(
            error.to_string(),
            "Translation quality below threshold: 0.6 (required: 0.8)"
        );
    }

    #[test]
    fn test_multiple_error_variants() {
        // Test different AudioError variants
        let errors = vec![
            AudioError::ConfigurationError {
                message: "Invalid sample rate".to_string(),
            },
            AudioError::UnsupportedFormat {
                format: "mp3".to_string(),
            },
            AudioError::StreamClosed,
            AudioError::BufferOverrun,
            AudioError::InitializationError("Driver not found".to_string()),
        ];

        for error in errors {
            assert!(!error.to_string().is_empty());
        }
    }
}