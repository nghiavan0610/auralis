//! Unit tests for Whisper STT engine
//!
//! This module contains comprehensive tests for the Whisper STT implementation
//! including mock mode testing, error handling, and stream processing.

use auralis::infrastructure::stt::{WhisperConfig, WhisperEngine};
use auralis::domain::errors::STTError;
use futures::StreamExt;

#[tokio::test]
async fn test_whisper_engine_lifecycle() {
    let mut engine = WhisperEngine::mock();

    // Test initialization
    let config = serde_json::json!({
        "mock_mode": true,
        "language": "en"
    });

    assert!(!engine.initialized);
    let result = engine.initialize(config).await;
    assert!(result.is_ok());
    assert!(engine.initialized);

    // Test processing
    let audio_data = vec![0.1f32; 1600]; // 100ms at 16kHz
    let segments = engine.process_audio(audio_data).await.unwrap();
    assert_eq!(segments.len(), 1);
    assert!(segments[0].text.contains("mock"));

    // Test streaming
    let stream = engine.create_stream().await.unwrap();
    let stream_items: Vec<_> = stream.collect().await;
    assert_eq!(stream_items.len(), 2);
}

#[tokio::test]
async fn test_whisper_language_support() {
    let mut engine = WhisperEngine::mock();
    engine.config.mock_mode = true;
    engine.initialized = true;

    // Test supported languages
    let languages = engine.supported_languages();
    assert!(languages.contains(&"en".to_string()));
    assert!(languages.contains(&"es".to_string()));
    assert!(languages.contains(&"zh".to_string()));
    assert!(languages.contains(&"auto".to_string()));

    // Test setting language
    engine.set_language("es".to_string()).await.unwrap();
    assert_eq!(engine.current_language(), "es");

    // Test unsupported language
    let result = engine.set_language("invalid".to_string()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_whisper_error_handling() {
    let mut engine = WhisperEngine::mock();
    engine.config.mock_mode = true;

    // Test processing before initialization
    let result = engine.process_audio(vec![0.1f32; 100]).await;
    assert!(matches!(result.unwrap_err(), STTError::NotInitialized));

    // Test streaming before initialization
    let result = engine.create_stream().await;
    assert!(matches!(result.unwrap_err(), STTError::NotInitialized));

    // Initialize and test empty audio
    engine.initialized = true;
    let result = engine.process_audio(vec![]).await;
    assert!(matches!(result.unwrap_err(), STTError::AudioTooShort));
}

#[tokio::test]
async fn test_whisper_mock_mode_speech_detection() {
    let mut engine = WhisperEngine::mock();
    engine.config.mock_mode = true;
    engine.initialized = true;

    // Test with different audio energy levels
    let silence = vec![0.0f32; 1000];
    let quiet = vec![0.0005f32; 1000];
    let normal = vec![0.01f32; 1000];
    let loud = vec![0.1f32; 1000];

    // Silence should return empty
    let result = engine.process_audio(silence).await.unwrap();
    assert!(result.is_empty());

    // Quiet audio should return empty
    let result = engine.process_audio(quiet).await.unwrap();
    assert!(result.is_empty());

    // Normal audio should return transcription
    let result = engine.process_audio(normal).await.unwrap();
    assert_eq!(result.len(), 1);
    assert!(result[0].text.contains("lower energy"));

    // Loud audio should return transcription
    let result = engine.process_audio(loud).await.unwrap();
    assert_eq!(result.len(), 1);
    assert!(result[0].text.contains("higher energy"));
}

#[tokio::test]
async fn test_whisper_audio_preparation() {
    let engine = WhisperEngine::mock();

    // Test audio clipping
    let audio = vec![1.5f32, -2.0f32, 0.5f32, -0.8f32];
    let prepared = engine.prepare_audio(audio);
    assert_eq!(prepared, vec![1.0f32, -1.0f32, 0.5f32, -0.8f32]);
}

#[tokio::test]
async fn test_whisper_config_parsing() {
    let mut engine = WhisperEngine::new(WhisperConfig {
        model_path: "/path/to/model.bin".into(),
        language: "auto".to_string(),
        use_gpu: false,
        num_threads: 8,
        mock_mode: true,
    });

    let config = serde_json::json!({
        "model_path": "/new/path/model.bin",
        "language": "zh",
        "use_gpu": true,
        "num_threads": 16
    });

    engine.initialize(config).await.unwrap();

    assert_eq!(engine.config.model_path, PathBuf::from("/new/path/model.bin"));
    assert_eq!(engine.current_language(), "zh");
    assert!(engine.config.use_gpu);
    assert_eq!(engine.config.num_threads, 16);
}

#[tokio::test]
async fn test_whisper_stream_segments() {
    let mut engine = WhisperEngine::mock();
    engine.config.mock_mode = true;
    engine.initialized = true;

    let stream = engine.create_stream().await.unwrap();
    let segments: Vec<_> = stream
        .filter_map(|result| async { result.ok() })
        .collect()
        .await;

    assert_eq!(segments.len(), 2);
    assert!(segments[0].text.contains("segment 1"));
    assert!(segments[1].text.contains("segment 2"));
    assert!(!segments[0].is_final); // First segment is interim
    assert!(segments[1].is_final);  // Second segment is final
}

#[tokio::test]
async fn test_whisper_multiple_language_switching() {
    let mut engine = WhisperEngine::mock();
    engine.config.mock_mode = true;
    engine.initialized = true;

    // Test switching between multiple languages
    for lang in &["en", "es", "zh", "fr"] {
        engine.set_language(lang.to_string()).await.unwrap();
        assert_eq!(engine.current_language(), *lang);
    }

    // Test that unsupported language fails
    let result = engine.set_language("invalid_lang".to_string()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_whisper_segment_structure() {
    let mut engine = WhisperEngine::mock();
    engine.config.mock_mode = true;
    engine.initialized = true;

    let audio_data = vec![0.1f32; 1000];
    let segments = engine.process_audio(audio_data).await.unwrap();

    assert_eq!(segments.len(), 1);
    let segment = &segments[0];

    // Validate segment structure
    assert!(!segment.text.is_empty());
    assert!(segment.confidence > 0.0 && segment.confidence <= 1.0);
    assert!(segment.end_time > segment.start_time);
    assert!(segment.is_final);
}

use std::path::PathBuf;

#[test]
fn test_whisper_model_detection() {
    let config = WhisperConfig {
        model_path: PathBuf::from("/nonexistent/model.bin"),
        ..Default::default()
    };

    let engine = WhisperEngine::new(config);

    // Test model existence check
    assert!(!engine.model_exists());
    assert_eq!(engine.model_path(), PathBuf::from("/nonexistent/model.bin"));
}

#[tokio::test]
async fn test_whisper_download_not_implemented() {
    let engine = WhisperEngine::mock();

    // Download should fail for non-existent models
    let result = engine.download_model().await;
    assert!(result.is_err());

    if let Err(STTError::ServiceError { code, message }) = result {
        assert_eq!(code, "MODEL_NOT_FOUND");
        assert!(message.contains("download not implemented"));
    } else {
        panic!("Expected ServiceError");
    }
}

#[tokio::test]
async fn test_whisper_concurrent_processing() {
    let mut engine = WhisperEngine::mock();
    engine.config.mock_mode = true;
    engine.initialized = true;

    // Test processing multiple audio chunks concurrently
    let audio_chunks = vec![
        vec![0.1f32; 500],
        vec![0.05f32; 500],
        vec![0.2f32; 500],
    ];

    let mut handles = Vec::new();
    for audio in audio_chunks {
        let mut engine_ref = unsafe { &mut *(&mut engine as *mut WhisperEngine) };
        let handle = tokio::spawn(async move {
            engine_ref.process_audio(audio).await
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_whisper_stt_segment_validation() {
    let mut engine = WhisperEngine::mock();
    engine.config.mock_mode = true;
    engine.initialized = true;

    let audio_data = vec![0.1f32; 1000];
    let segments = engine.process_audio(audio_data).await.unwrap();

    // Validate each segment
    for segment in &segments {
        let validation = segment.validate();
        assert!(validation.is_ok(), "Segment validation failed: {:?}", validation);
    }
}

#[test]
fn test_whisper_config_cloning() {
    let config = WhisperConfig {
        model_path: "/test/path.bin".into(),
        language: "zh".to_string(),
        use_gpu: true,
        num_threads: 12,
        mock_mode: false,
    };

    let cloned = config.clone();
    assert_eq!(config.model_path, cloned.model_path);
    assert_eq!(config.language, cloned.language);
    assert_eq!(config.use_gpu, cloned.use_gpu);
    assert_eq!(config.num_threads, cloned.num_threads);
    assert_eq!(config.mock_mode, cloned.mock_mode);
}
