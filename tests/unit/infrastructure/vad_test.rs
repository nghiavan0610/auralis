//! Unit tests for Silero VAD implementation
//!
//! This module contains comprehensive tests for the Silero VAD implementation
//! including energy-based detection, mock mode, and error handling.

use auralis::infrastructure::vad::{SileroConfig, SileroVAD};
use auralis::domain::errors::VADError;
use std::path::PathBuf;

#[tokio::test]
async fn test_silero_vad_lifecycle() {
    let mut vad = SileroVAD::mock();

    // Test initialization
    let config = serde_json::json!({
        "threshold": 0.6,
        "sample_rate": 16000,
        "mock_mode": true
    });

    assert!(!vad.initialized);
    let result = vad.initialize(config).await;
    assert!(result.is_ok());
    assert!(vad.initialized);
    assert_eq!(vad.threshold(), 0.6);

    // Test processing
    let audio_data = vec![0.1f32; 1600]; // 100ms at 16kHz
    let is_speech = vad.process_audio(audio_data).await.unwrap();
    assert!(is_speech);

    // Test reset
    vad.audio_buffer.extend_from_slice(&[0.1f32; 1000]);
    vad.reset().await.unwrap();
    assert!(vad.audio_buffer.is_empty());
}

#[test]
fn test_silero_config_default() {
    let config = SileroConfig::default();
    assert_eq!(config.sample_rate, 16000);
    assert_eq!(config.threshold, 0.5);
    assert_eq!(config.window_size, 512);
    assert!(!config.mock_mode);
    assert_eq!(config.min_speech_duration_ms, 250);
    assert_eq!(config.min_silence_duration_ms, 100);
}

#[test]
fn test_silero_vad_creation() {
    let vad = SileroVAD::with_defaults();
    assert!(!vad.initialized);
    assert_eq!(vad.threshold(), 0.5);
    assert!(vad.audio_buffer.is_empty());
}

#[test]
fn test_silero_vad_mock() {
    let vad = SileroVAD::mock();
    assert!(vad.config.mock_mode);
    assert!(vad.model_exists()); // Mock mode always returns true
}

#[test]
fn test_model_detection() {
    let config = SileroConfig {
        model_path: PathBuf::from("/nonexistent/model.pt"),
        ..Default::default()
    };

    let vad = SileroVAD::new(config);
    assert!(!vad.model_exists());
    assert_eq!(vad.model_path(), &PathBuf::from("/nonexistent/model.pt"));
}

#[tokio::test]
async fn test_threshold_management() {
    let mut vad = SileroVAD::mock();
    vad.initialized = true;

    // Test default threshold
    assert_eq!(vad.threshold(), 0.5);

    // Test setting valid threshold
    vad.set_threshold(0.8).await.unwrap();
    assert_eq!(vad.threshold(), 0.8);

    vad.set_threshold(0.0).await.unwrap();
    assert_eq!(vad.threshold(), 0.0);

    vad.set_threshold(1.0).await.unwrap();
    assert_eq!(vad.threshold(), 1.0);

    // Test invalid thresholds
    assert!(vad.set_threshold(1.5).await.is_err());
    assert!(vad.set_threshold(-0.1).await.is_err());

    // Verify threshold didn't change after invalid attempt
    assert_eq!(vad.threshold(), 1.0);
}

#[tokio::test]
async fn test_energy_calculation() {
    let vad = SileroVAD::mock();
    vad.initialized = true;

    // Test with silence
    let silence = vec![0.0f32; 1000];
    let energy = vad.calculate_energy(&silence);
    assert_eq!(energy, 0.0);

    // Test with constant amplitude
    let constant = vec![0.5f32; 1000];
    let energy = vad.calculate_energy(&constant);
    assert!((energy - 0.5).abs() < 0.01);

    // Test with varying signal
    let varying = vec![0.1, 0.2, 0.3, 0.4, 0.5f32];
    let energy = vad.calculate_energy(&varying);
    assert!(energy > 0.0 && energy < 1.0);

    // Test with empty array
    let empty: Vec<f32> = vec![];
    let energy = vad.calculate_energy(&empty);
    assert_eq!(energy, 0.0);
}

#[tokio::test]
async fn test_speech_detection_energy_based() {
    let vad = SileroVAD::mock();
    vad.initialized = true;

    // Test silence detection
    let silence = vec![0.0f32; 1000];
    let (is_speech, energy) = vad.detect_speech_energy(&silence);
    assert!(!is_speech);
    assert_eq!(energy, 0.0);

    // Test very quiet audio (below threshold)
    let quiet = vec![0.0001f32; 1000];
    let (is_speech, energy) = vad.detect_speech_energy(&quiet);
    assert!(!is_speech);
    assert!(energy < vad.energy_threshold);

    // Test speech-level audio
    let speech = vec![0.1f32; 1000];
    let (is_speech, energy) = vad.detect_speech_energy(&speech);
    assert!(is_speech);
    assert!(energy > vad.energy_threshold);

    // Test loud audio
    let loud = vec![0.5f32; 1000];
    let (is_speech, energy) = vad.detect_speech_energy(&loud);
    assert!(is_speech);
    assert!(energy > vad.energy_threshold);
}

#[tokio::test]
async fn test_is_speech_method() {
    let mut vad = SileroVAD::mock();
    vad.initialized = true;

    // Test various audio levels
    let test_cases = vec![
        (vec![0.0f32; 1000], false),      // Silence
        (vec![0.0001f32; 1000], false),   // Very quiet
        (vec![0.001f32; 1000], true),     // Just above threshold
        (vec![0.1f32; 1000], true),       // Normal speech level
        (vec![0.9f32; 1000], true),       // Loud speech
    ];

    for (audio, expected) in test_cases {
        let result = vad.is_speech(&audio).unwrap();
        assert_eq!(
            result, expected,
            "Failed for audio level: {}",
            audio[0]
        );
    }

    // Test empty audio
    assert!(!vad.is_speech(&[]).unwrap());
}

#[tokio::test]
async fn test_speech_probability() {
    let mut vad = SileroVAD::mock();
    vad.initialized = true;

    // Test silence
    let prob = vad.speech_probability(vec![0.0f32; 1000]).await.unwrap();
    assert_eq!(prob, 0.0);

    // Test moderate signal
    let prob = vad.speech_probability(vec![0.3f32; 1000]).await.unwrap();
    assert!(prob > 0.0 && prob <= 1.0);

    // Test strong signal
    let prob = vad.speech_probability(vec![0.8f32; 1000]).await.unwrap();
    assert!(prob > 0.5);

    // Test empty audio
    let prob = vad.speech_probability(vec![]).await.unwrap();
    assert_eq!(prob, 0.0);
}

#[tokio::test]
async fn test_process_audio_not_initialized() {
    let mut vad = SileroVAD::mock();
    // Don't initialize

    let audio_data = vec![0.1f32; 1000];
    let result = vad.process_audio(audio_data).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        VADError::ProcessingError { .. }
    ));
}

#[tokio::test]
async fn test_process_audio_empty() {
    let mut vad = SileroVAD::mock();
    vad.initialized = true;

    let audio_data = vec![];
    let result = vad.process_audio(audio_data).await;
    assert!(result.is_ok());
    assert!(!result.unwrap()); // Empty audio should not be detected as speech
}

#[tokio::test]
async fn test_process_audio_mock_mode() {
    let mut vad = SileroVAD::mock();
    vad.config.mock_mode = true;
    vad.initialized = true;

    // Test various audio levels
    let quiet = vec![0.0001f32; 1000];
    assert!(!vad.process_audio(quiet.clone()).await.unwrap());

    let speech = vec![0.1f32; 1000];
    assert!(vad.process_audio(speech.clone()).await.unwrap());

    let loud = vec![0.5f32; 1000];
    assert!(vad.process_audio(loud.clone()).await.unwrap());
}

#[tokio::test]
async fn test_initialize_invalid_sample_rate() {
    let mut vad = SileroVAD::mock();
    let config = serde_json::json!({
        "sample_rate": 44100, // Invalid
        "mock_mode": true
    });

    let result = vad.initialize(config).await;
    assert!(result.is_err());
    if let Err(VADError::InvalidSampleRate { rate, supported }) = result {
        assert_eq!(rate, 44100);
        assert!(supported.contains("16000"));
        assert!(supported.contains("8000"));
    } else {
        panic!("Expected InvalidSampleRate error");
    }
}

#[tokio::test]
async fn test_initialize_valid_sample_rates() {
    let valid_rates = vec![16000u64, 8000u64];

    for rate in valid_rates {
        let mut vad = SileroVAD::mock();
        let config = serde_json::json!({
            "sample_rate": rate,
            "mock_mode": true
        });

        let result = vad.initialize(config).await;
        assert!(result.is_ok(), "Failed for sample rate: {}", rate);
        assert_eq!(vad.config.sample_rate, rate as u32);
    }
}

#[tokio::test]
async fn test_reset_functionality() {
    let mut vad = SileroVAD::mock();
    vad.initialized = true;

    // Add data to buffer
    vad.audio_buffer.extend_from_slice(&[0.1f32; 2000]);
    assert_eq!(vad.audio_buffer.len(), 2000);

    // Reset
    vad.reset().await.unwrap();
    assert!(vad.audio_buffer.is_empty());

    // Should still work after reset
    let result = vad.process_audio(vec![0.1f32; 1000]).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_chunk_processing() {
    let mut vad = SileroVAD::mock();
    vad.config.mock_mode = true;
    vad.config.window_size = 512;
    vad.initialized = true;

    // Process single chunk
    let chunk1 = vec![0.1f32; 512];
    let result = vad.process_chunk(&chunk1);
    assert!(result.is_ok());

    // Process multiple chunks
    for i in 0..10 {
        let chunk = vec![0.1f32; 512];
        let result = vad.process_chunk(&chunk);
        assert!(result.is_ok(), "Failed on chunk {}", i);
    }

    // Verify buffer doesn't grow unbounded
    assert!(vad.audio_buffer.len() <= vad.config.window_size * 4);
}

#[tokio::test]
async fn test_chunk_processing_not_initialized() {
    let vad = SileroVAD::mock();
    // Don't initialize

    let chunk = vec![0.1f32; 512];
    let result = vad.process_chunk(&chunk);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        VADError::ProcessingError { .. }
    ));
}

#[tokio::test]
async fn test_speech_probability_not_initialized() {
    let vad = SileroVAD::mock();
    // Don't initialize

    let result = vad.speech_probability(vec![0.1f32; 1000]).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        VADError::PredictionError { .. }
    ));
}

#[test]
fn test_config_variations() {
    // Test with custom configuration
    let config = SileroConfig {
        model_path: PathBuf::from("/custom/path/model.pt"),
        sample_rate: 8000,
        threshold: 0.7,
        window_size: 256,
        mock_mode: true,
        min_speech_duration_ms: 500,
        min_silence_duration_ms: 200,
    };

    let vad = SileroVAD::new(config);
    assert_eq!(vad.config.sample_rate, 8000);
    assert_eq!(vad.config.window_size, 256);
    assert_eq!(vad.config.threshold, 0.7);
    assert!(vad.config.mock_mode);
}

#[test]
fn test_config_cloning() {
    let config = SileroConfig {
        model_path: PathBuf::from("/test/path.pt"),
        sample_rate: 16000,
        threshold: 0.8,
        window_size: 1024,
        mock_mode: false,
        min_speech_duration_ms: 300,
        min_silence_duration_ms: 150,
    };

    let cloned = config.clone();
    assert_eq!(config.model_path, cloned.model_path);
    assert_eq!(config.sample_rate, cloned.sample_rate);
    assert_eq!(config.threshold, cloned.threshold);
    assert_eq!(config.window_size, cloned.window_size);
    assert_eq!(config.mock_mode, cloned.mock_mode);
    assert_eq!(config.min_speech_duration_ms, cloned.min_speech_duration_ms);
    assert_eq!(config.min_silence_duration_ms, cloned.min_silence_duration_ms);
}

#[tokio::test]
async fn test_varying_audio_amplitudes() {
    let mut vad = SileroVAD::mock();
    vad.initialized = true;

    let test_cases: Vec<(Vec<f32>, bool)> = vec![
        (vec![0.0f32; 100], false),
        (vec![0.00005f32; 100], false),
        (vec![0.0005f32; 100], true),
        (vec![0.005f32; 100], true),
        (vec![0.05f32; 100], true),
        (vec![0.5f32; 100], true),
        (vec![1.0f32; 100], true),
    ];

    for (audio, expected_has_speech) in test_cases {
        let is_speech = vad.is_speech(&audio).unwrap();
        assert_eq!(
            is_speech, expected_has_speech,
            "Failed for amplitude: {}",
            audio[0]
        );
    }
}

#[tokio::test]
async fn test_real_world_scenario() {
    let mut vad = SileroVAD::mock();
    vad.config.mock_mode = true;
    vad.config.window_size = 512;
    vad.initialized = true;

    // Simulate a conversation with silence, speech, and more silence
    let scenario = vec![
        (vec![0.0f32; 512], false),   // Initial silence
        (vec![0.0f32; 512], false),   // More silence
        (vec![0.1f32; 512], true),    // Speech starts
        (vec![0.15f32; 512], true),   // Speech continues
        (vec![0.08f32; 512], true),   // Speech continues
        (vec![0.0f32; 512], false),   // Silence returns
        (vec![0.0f32; 512], false),   // More silence
    ];

    for (i, (audio_chunk, expected)) in scenario.iter().enumerate() {
        let is_speech = vad.process_chunk(audio_chunk).unwrap();
        assert_eq!(
            is_speech, *expected,
            "Chunk {}: expected {}, got {}",
            i, expected, is_speech
        );
    }
}
