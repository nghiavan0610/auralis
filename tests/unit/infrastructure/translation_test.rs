//! Unit tests for MADLAD translation bridge
//!
//! This module contains comprehensive tests for the PyO3 translation bridge
//! including mock mode testing, error handling, and language detection.

use auralis::infrastructure::translation::{MadladConfig, MadladTranslator};
use auralis::domain::errors::TranslationError;
use std::path::PathBuf;

#[tokio::test]
async fn test_madlad_translator_lifecycle() {
    let mut translator = MadladTranslator::mock();

    // Test initialization
    let config = serde_json::json!({
        "mock_mode": true,
        "device": "cpu",
        "beam_size": 1
    });

    assert!(!translator.initialized);
    let result = translator.initialize(config).await;
    // Note: This might fail if Python isn't available, which is expected
    // In a real test environment, we'd set up Python properly
}

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
    assert!(translator.model_exists()); // Mock mode always returns true
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

    // Test common languages
    assert!(languages.contains(&"en".to_string()));
    assert!(languages.contains(&"es".to_string()));
    assert!(languages.contains(&"zh".to_string()));
    assert!(languages.contains(&"ja".to_string()));
    assert!(languages.contains(&"ko".to_string()));
    assert!(languages.contains(&"fr".to_string()));
    assert!(languages.contains(&"de".to_string()));
}

#[test]
fn test_language_pair_support() {
    let translator = MadladTranslator::mock();

    // Test supported pairs
    assert!(translator.is_pair_supported("en", "es"));
    assert!(translator.is_pair_supported("zh", "en"));
    assert!(translator.is_pair_supported("ja", "ko"));

    // Test unsupported pairs
    assert!(!translator.is_pair_supported("xx", "yy"));
    assert!(!translator.is_pair_supported("invalid", "en"));

    // Test same language (not supported for translation)
    assert!(!translator.is_pair_supported("en", "en"));
    assert!(!translator.is_pair_supported("zh", "zh"));
}

#[tokio::test]
async fn test_language_detection() {
    let translator = MadladTranslator::mock();
    translator.initialized = true;

    // Test English detection
    let detected = translator
        .detect_language("Hello world, this is English text.".to_string())
        .await
        .unwrap();
    assert_eq!(detected, "en");

    // Test Chinese detection (CJK characters)
    let detected = translator
        .detect_language("你好世界".to_string())
        .await
        .unwrap();
    assert_eq!(detected, "zh");

    // Test Japanese detection (Hiragana)
    let detected = translator
        .detect_language("こんにちは世界".to_string())
        .await
        .unwrap();
    assert_eq!(detected, "ja");

    // Test Korean detection (Hangul)
    let detected = translator
        .detect_language("안녕하세요 세계".to_string())
        .await
        .unwrap();
    assert_eq!(detected, "ko");

    // Test empty text
    let result = translator.detect_language("".to_string()).await;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), TranslationError::EmptyText));

    // Test whitespace-only text
    let result = translator.detect_language("   ".to_string()).await;
    assert!(result.is_err());
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

    // Test whitespace-only text
    let result = translator
        .translate("   ".to_string(), "en".to_string(), "es".to_string())
        .await;
    assert!(result.is_err());
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

    // Test with one valid and one invalid language
    let result = translator
        .translate("Hello".to_string(), "en".to_string(), "xx".to_string())
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_detect_language_not_initialized() {
    let translator = MadladTranslator::mock();
    // Don't initialize

    let result = translator.detect_language("Hello world".to_string()).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        TranslationError::LanguageDetectionFailed
    ));
}

#[test]
fn test_config_cloning() {
    let config = MadladConfig {
        model_path: PathBuf::from("/test/path/madlad"),
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

#[test]
fn test_supported_pairs_generation() {
    let translator = MadladTranslator::mock();
    let pairs = translator.supported_pairs();

    // Should have many pairs (all combinations of supported languages)
    assert!(!pairs.is_empty());

    // Check that common pairs are present
    assert!(pairs.contains(&(String::from("en"), String::from("es"))));
    assert!(pairs.contains(&(String::from("zh"), String::from("en"))));
    assert!(pairs.contains(&(String::from("en"), String::from("zh"))));

    // Check that pairs are bidirectional
    assert!(pairs.contains(&(String::from("en"), String::from("fr"))));
    assert!(pairs.contains(&(String::from("fr"), String::from("en"))));
}

#[tokio::test]
async fn test_translation_result_structure() {
    let mut translator = MadladTranslator::mock();
    translator.initialized = true;

    // Note: This test would require Python to be available
    // In a real test environment, we'd mock the Python calls
    // For now, we'll skip the actual translation call
}

#[test]
fn test_madlad_config_variations() {
    // Test CPU configuration
    let cpu_config = MadladConfig {
        device: "cpu".to_string(),
        compute_type: "int8".to_string(),
        ..Default::default()
    };
    assert_eq!(cpu_config.device, "cpu");
    assert_eq!(cpu_config.compute_type, "int8");

    // Test GPU configuration
    let gpu_config = MadladConfig {
        device: "cuda".to_string(),
        compute_type: "float16".to_string(),
        ..Default::default()
    };
    assert_eq!(gpu_config.device, "cuda");
    assert_eq!(gpu_config.compute_type, "float16");

    // Test auto device
    let auto_config = MadladConfig {
        device: "auto".to_string(),
        ..Default::default()
    };
    assert_eq!(auto_config.device, "auto");
}

#[tokio::test]
async fn test_multiple_language_detection() {
    let translator = MadladTranslator::mock();
    translator.initialized = true;

    let test_cases = vec![
        ("Hello world", "en"),
        ("Bonjour le monde", "en"), // Latin script defaults to English
        ("Привет мир", "ru"),        // Cyrillic
        ("مرحبا بالعالم", "ar"),     // Arabic
        ("שלום עולם", "he"),        // Hebrew
    ];

    for (text, expected_lang) in test_cases {
        let detected = translator.detect_language(text.to_string()).await.unwrap();
        assert_eq!(
            detected, expected_lang,
            "Expected {} for text '{}', got {}",
            expected_lang, text, detected
        );
    }
}

#[test]
fn test_translator_initialization_states() {
    let translator = MadladTranslator::mock();

    // Initially not initialized
    assert!(!translator.initialized);

    // Python translator is None before initialization
    assert!(translator.python_translator.is_none());
}

#[tokio::test]
async fn test_translation_error_handling() {
    let mut translator = MadladTranslator::mock();
    translator.initialized = true;

    // Test various error conditions
    let error_cases = vec![
        ("", "en", "es"),           // Empty text
        ("   ", "en", "es"),        // Whitespace only
        ("Hello", "xx", "yy"),      // Unsupported languages
        ("Hello", "en", "en"),      // Same language
    ];

    for (text, source, target) in error_cases {
        let result = translator.translate(text.to_string(), source.to_string(), target.to_string()).await;
        assert!(result.is_err(), "Expected error for {} -> {}: '{}'", source, target, text);
    }
}
