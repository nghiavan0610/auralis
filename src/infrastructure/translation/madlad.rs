//! MADLAD translation bridge using PyO3
//!
//! This module provides a Rust wrapper around the Python MADLAD translator,
//! enabling seamless integration between Rust and Python translation libraries.

use async_trait::async_trait;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_asyncio::tokio::future_into_py;
use std::path::PathBuf;
use std::pin::Pin;
use tracing::{debug, error, info, warn};

use crate::domain::{errors::TranslationError, models::Translation, traits::Translator};

/// Configuration for the MADLAD translator
#[derive(Debug, Clone)]
pub struct MadladConfig {
    /// Path to the MADLAD model directory
    pub model_path: PathBuf,

    /// Device to use for inference ("cpu", "cuda", "auto")
    pub device: String,

    /// Compute type ("default", "int8", "int8_float16", "int16", "float16")
    pub compute_type: String,

    /// Whether to use mock mode (for testing)
    pub mock_mode: bool,

    /// Beam size for translation
    pub beam_size: usize,
}

impl Default for MadladConfig {
    fn default() -> Self {
        Self {
            model_path: PathBuf::from("models/madlad"),
            device: "cpu".to_string(),
            compute_type: "default".to_string(),
            mock_mode: false,
            beam_size: 1,
        }
    }
}

/// MADLAD translator implementation using PyO3 bridge to Python
pub struct MadladTranslator {
    config: MadladConfig,
    python_translator: Option<Py<PyAny>>,
    initialized: bool,
}

impl MadladTranslator {
    /// Create a new MADLAD translator with the given configuration
    pub fn new(config: MadladConfig) -> Self {
        Self {
            config,
            python_translator: None,
            initialized: false,
        }
    }

    /// Create a new MADLAD translator with default configuration
    pub fn with_defaults() -> Self {
        Self::new(MadladConfig::default())
    }

    /// Create a new MADLAD translator in mock mode
    pub fn mock() -> Self {
        let mut config = MadladConfig::default();
        config.mock_mode = true;
        Self::new(config)
    }

    /// Check if the Python translator module is available
    pub fn is_python_available() -> bool {
        Python::with_gil(|py| {
            // Try to import the translator module
            match py.import("translator") {
                Ok(_) => true,
                Err(e) => {
                    warn!("Python translator module not available: {}", e);
                    false
                }
            }
        })
    }

    /// Check if the model directory exists
    pub fn model_exists(&self) -> bool {
        self.config.model_path.exists() || self.config.mock_mode
    }

    /// Initialize the Python translator
    fn initialize_python(&mut self) -> Result<(), TranslationError> {
        Python::with_gil(|py| {
            // Import the translator module
            let translator_module = py
                .import("translator")
                .map_err(|e| TranslationError::ServiceError {
                    code: "PY_IMPORT_ERROR".to_string(),
                    message: format!("Failed to import translator module: {}", e),
                })?;

            // Create translator instance
            let create_fn = translator_module
                .getattr("create_translator")
                .map_err(|e| TranslationError::ServiceError {
                    code: "PY_FUNCTION_ERROR".to_string(),
                    message: format!("Failed to get create_translator function: {}", e),
                })?;

            let kwargs = PyDict::new(py);
            kwargs.set_item("model_path", &self.config.model_path).map_err(|e| {
                TranslationError::ServiceError {
                    code: "PY_PARAM_ERROR".to_string(),
                    message: format!("Failed to set model_path parameter: {}", e),
                }
            })?;

            kwargs.set_item("device", &self.config.device).map_err(|e| {
                TranslationError::ServiceError {
                    code: "PY_PARAM_ERROR".to_string(),
                    message: format!("Failed to set device parameter: {}", e),
                }
            })?;

            kwargs.set_item("compute_type", &self.config.compute_type).map_err(|e| {
                TranslationError::ServiceError {
                    code: "PY_PARAM_ERROR".to_string(),
                    message: format!("Failed to set compute_type parameter: {}", e),
                }
            })?;

            kwargs.set_item("mock_mode", self.config.mock_mode).map_err(|e| {
                TranslationError::ServiceError {
                    code: "PY_PARAM_ERROR".to_string(),
                    message: format!("Failed to set mock_mode parameter: {}", e),
                }
            })?;

            let translator = create_fn
                .call((), Some(kwargs))
                .map_err(|e| TranslationError::ServiceError {
                    code: "PY_CREATE_ERROR".to_string(),
                    message: format!("Failed to create translator: {}", e),
                })?;

            // Initialize the translator
            translator
                .call_method0("initialize")
                .map_err(|e| TranslationError::ServiceError {
                    code: "PY_INIT_ERROR".to_string(),
                    message: format!("Failed to initialize translator: {}", e),
                })?;

            self.python_translator = Some(translator.into());
            Ok(())
        })
    }
}

#[async_trait]
impl Translator for MadladTranslator {
    async fn initialize(&mut self, config: serde_json::Value) -> Result<(), TranslationError> {
        info!("Initializing MADLAD translator");

        // Parse configuration
        if let Some(model_path) = config.get("model_path").and_then(|v| v.as_str()) {
            self.config.model_path = PathBuf::from(model_path);
        }

        if let Some(device) = config.get("device").and_then(|v| v.as_str()) {
            self.config.device = device.to_string();
        }

        if let Some(compute_type) = config.get("compute_type").and_then(|v| v.as_str()) {
            self.config.compute_type = compute_type.to_string();
        }

        if let Some(mock_mode) = config.get("mock_mode").and_then(|v| v.as_bool()) {
            self.config.mock_mode = mock_mode;
        }

        if let Some(beam_size) = config.get("beam_size").and_then(|v| v.as_u64()) {
            self.config.beam_size = beam_size as usize;
        }

        // Check model availability
        if !self.model_exists() && !self.config.mock_mode {
            warn!("Model directory not found: {:?}", self.config.model_path);
            return Err(TranslationError::ConnectionError {
                service: format!("Model not found at {:?}", self.config.model_path),
            });
        }

        // Initialize Python bridge
        self.initialize_python()?;

        self.initialized = true;
        info!("MADLAD translator initialized successfully");
        Ok(())
    }

    async fn translate(
        &mut self,
        text: String,
        source_lang: String,
        target_lang: String,
    ) -> Result<Translation, TranslationError> {
        if !self.initialized {
            return Err(TranslationError::ConnectionError {
                service: "Translator not initialized".to_string(),
            });
        }

        if text.trim().is_empty() {
            return Err(TranslationError::EmptyText);
        }

        if !self.is_pair_supported(&source_lang, &target_lang) {
            return Err(TranslationError::UnsupportedLanguagePair {
                source: source_lang.clone(),
                target: target_lang.clone(),
            });
        }

        debug!(
            "Translating text from {} to {} ({} characters)",
            source_lang,
            target_lang,
            text.len()
        );

        let result = Python::with_gil(|py| {
            let translator = self
                .python_translator
                .as_ref()
                .ok_or_else(|| TranslationError::ConnectionError {
                    service: "Python translator not available".to_string(),
                })?
                .as_ref(py);

            // Call translate method
            let result = translator
                .call_method(
                    "translate",
                    (text, source_lang.clone(), target_lang.clone()),
                    None,
                )
                .map_err(|e| TranslationError::ServiceError {
                    code: "PY_TRANSLATE_ERROR".to_string(),
                    message: format!("Translation call failed: {}", e),
                })?;

            // Extract translated text and confidence
            let translated_text: String = result
                .get_item(0)
                .map_err(|e| TranslationError::ServiceError {
                    code: "PY_RESULT_ERROR".to_string(),
                    message: format!("Failed to extract translated text: {}", e),
                })?
                .extract()
                .map_err(|e| TranslationError::ServiceError {
                    code: "PY_EXTRACT_ERROR".to_string(),
                    message: format!("Failed to extract translated text: {}", e),
                })?;

            let confidence: f32 = result
                .get_item(1)
                .map_err(|e| TranslationError::ServiceError {
                    code: "PY_RESULT_ERROR".to_string(),
                    message: format!("Failed to extract confidence: {}", e),
                })?
                .extract()
                .map_err(|e| TranslationError::ServiceError {
                    code: "PY_EXTRACT_ERROR".to_string(),
                    message: format!("Failed to extract confidence: {}", e),
                })?;

            Ok::<_, TranslationError>((translated_text, confidence))
        })?;

        let (translated_text, confidence) = result;

        Ok(Translation::new(
            source_lang,
            target_lang,
            text,
            translated_text,
            confidence,
        ))
    }

    fn supported_pairs(&self) -> Vec<(String, String)> {
        // Return common language pairs
        let languages = self.supported_languages();
        let mut pairs = Vec::new();

        // Create bidirectional pairs for common languages
        for i in 0..languages.len() {
            for j in 0..languages.len() {
                if i != j {
                    pairs.push((languages[i].clone(), languages[j].clone()));
                }
            }
        }

        pairs
    }

    fn is_pair_supported(&self, source: &str, target: &str) -> bool {
        let supported = self.supported_languages();
        supported.contains(&source.to_string())
            && supported.contains(&target.to_string())
            && source != target
    }

    async fn detect_language(&self, text: String) -> Result<String, TranslationError> {
        if !self.initialized {
            return Err(TranslationError::LanguageDetectionFailed);
        }

        if text.trim().is_empty() {
            return Err(TranslationError::EmptyText);
        }

        // Simple language detection heuristic
        // In a real implementation, you would use a proper language detection library
        let detected = if text.chars().any(|c| {
            (c as u32) >= 0x4E00 && (c as u32) <= 0x9FFF // CJK Unified Ideographs
        }) {
            "zh".to_string()
        } else if text.chars().any(|c| (c as u32) >= 0x0400 && (c as u32) <= 0x04FF) {
            // Cyrillic script
            "ru".to_string()
        } else if text.chars().any(|c| (c as u32) >= 0x0590 && (c as u32) <= 0x05FF) {
            // Hebrew script
            "he".to_string()
        } else if text.chars().any(|c| (c as u32) >= 0x0600 && (c as u32) <= 0x06FF) {
            // Arabic script
            "ar".to_string()
        } else if text.chars().any(|c| (c as u32) >= 0x3040 && (c as u32) <= 0x309F) {
            // Hiragana
            "ja".to_string()
        } else if text.chars().any(|c| (c as u32) >= 0x1100 && (c as u32) <= 0x11FF) {
            // Hangul
            "ko".to_string()
        } else {
            // Default to English for Latin script
            "en".to_string()
        };

        Ok(detected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_madlad_config_default() {
        let config = MadladConfig::default();
        assert_eq!(config.device, "cpu");
        assert_eq!(config.compute_type, "default");
        assert!(!config.mock_mode);
        assert_eq!(config.beam_size, 1);
    }

    #[test]
    fn test_madlad_translator_creation() {
        let translator = MadladTranslator::with_defaults();
        assert!(!translator.initialized);
        assert!(translator.python_translator.is_none());
    }

    #[test]
    fn test_madlad_translator_mock() {
        let translator = MadladTranslator::mock();
        assert!(translator.config.mock_mode);
    }

    #[test]
    fn test_model_exists() {
        let config = MadladConfig {
            model_path: PathBuf::from("/nonexistent/path"),
            ..Default::default()
        };
        let translator = MadladTranslator::new(config);
        assert!(!translator.model_exists());

        let mock_translator = MadladTranslator::mock();
        assert!(mock_translator.model_exists()); // Mock mode always returns true
    }

    #[test]
    fn test_supported_languages() {
        let translator = MadladTranslator::mock();
        let languages = translator.supported_languages();
        assert!(languages.contains(&"en".to_string()));
        assert!(languages.contains(&"es".to_string()));
        assert!(languages.contains(&"zh".to_string()));
        assert!(languages.contains(&"ja".to_string()));
    }

    #[test]
    fn test_language_pair_support() {
        let translator = MadladTranslator::mock();

        // Test supported pairs
        assert!(translator.is_pair_supported("en", "es"));
        assert!(translator.is_pair_supported("zh", "en"));

        // Test unsupported pairs
        assert!(!translator.is_pair_supported("xx", "yy"));

        // Test same language (not supported for translation)
        assert!(!translator.is_pair_supported("en", "en"));
    }

    #[tokio::test]
    async fn test_detect_language() {
        let translator = MadladTranslator::mock();
        translator.initialized = true;

        // Test English detection
        let detected = translator.detect_language("Hello world".to_string()).await.unwrap();
        assert_eq!(detected, "en");

        // Test Chinese detection
        let detected = translator
            .detect_language("你好世界".to_string())
            .await
            .unwrap();
        assert_eq!(detected, "zh");

        // Test Japanese detection
        let detected = translator
            .detect_language("こんにちは".to_string())
            .await
            .unwrap();
        assert_eq!(detected, "ja");

        // Test empty text
        let result = translator.detect_language("".to_string()).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TranslationError::EmptyText
        ));
    }

    #[tokio::test]
    async fn test_translate_not_initialized() {
        let mut translator = MadladTranslator::mock();
        // Don't initialize

        let result = translator
            .translate("Hello".to_string(), "en".to_string(), "es".to_string())
            .await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TranslationError::ConnectionError { .. }
        ));
    }

    #[tokio::test]
    async fn test_translate_empty_text() {
        let mut translator = MadladTranslator::mock();
        translator.initialized = true;

        let result = translator
            .translate("".to_string(), "en".to_string(), "es".to_string())
            .await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TranslationError::EmptyText));
    }

    #[tokio::test]
    async fn test_translate_unsupported_pair() {
        let mut translator = MadladTranslator::mock();
        translator.initialized = true;

        let result = translator
            .translate("Hello".to_string(), "xx".to_string(), "yy".to_string())
            .await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TranslationError::UnsupportedLanguagePair { .. }
        ));
    }

    #[test]
    fn test_config_cloning() {
        let config = MadladConfig {
            model_path: "/test/path".into(),
            device: "cuda".to_string(),
            compute_type: "float16".to_string(),
            mock_mode: true,
            beam_size: 5,
        };

        let cloned = config.clone();
        assert_eq!(config.model_path, cloned.model_path);
        assert_eq!(config.device, cloned.device);
        assert_eq!(config.compute_type, cloned.compute_type);
        assert_eq!(config.mock_mode, cloned.mock_mode);
        assert_eq!(config.beam_size, cloned.beam_size);
    }
}
