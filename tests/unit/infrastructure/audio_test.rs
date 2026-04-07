//! Unit tests for audio capture functionality

use auralis::infrastructure::audio::{AudioCaptureConfig, MicrophoneCapture};

#[test]
fn test_audio_capture_config_creation() {
    let config = AudioCaptureConfig {
        sample_rate: 16000,
        channels: 1,
        chunk_duration_ms: 100,
    };

    assert_eq!(config.sample_rate, 16000);
    assert_eq!(config.channels, 1);
    assert_eq!(config.chunk_duration_ms, 100);
}

#[test]
fn test_audio_capture_config_default_values() {
    let config = AudioCaptureConfig::default();

    assert_eq!(config.sample_rate, 16000);
    assert_eq!(config.channels, 1);
    assert_eq!(config.chunk_duration_ms, 100);
}

#[test]
fn test_microphone_capture_creation() {
    let config = AudioCaptureConfig::default();
    let capture_result = MicrophoneCapture::new(config);

    assert!(
        capture_result.is_ok(),
        "Failed to create MicrophoneCapture: {:?}",
        capture_result.err()
    );
}

#[test]
fn test_microphone_capture_default_config() {
    let capture_result = MicrophoneCapture::default_config();

    assert!(
        capture_result.is_ok(),
        "Failed to create MicrophoneCapture with default config: {:?}",
        capture_result.err()
    );

    let capture = capture_result.unwrap();
    let config = capture.config();

    assert_eq!(config.sample_rate, 16000);
    assert_eq!(config.channels, 1);
    assert_eq!(config.buffer_size, 1600); // 16000 * 1 * 100 / 1000
}

#[test]
fn test_microphone_capture_custom_config() {
    let config = AudioCaptureConfig {
        sample_rate: 48000,
        channels: 2,
        chunk_duration_ms: 50,
    };

    let capture_result = MicrophoneCapture::new(config.clone());
    assert!(capture_result.is_ok());

    let capture = capture_result.unwrap();
    let audio_config = capture.config();

    assert_eq!(audio_config.sample_rate, 48000);
    assert_eq!(audio_config.channels, 2);
    assert_eq!(audio_config.buffer_size, 4800); // 48000 * 2 * 50 / 1000
}

#[test]
fn test_microphone_capture_initially_inactive() {
    let capture = MicrophoneCapture::default_config().unwrap();
    assert!(!capture.is_active(), "MicrophoneCapture should be initially inactive");
}

#[test]
fn test_audio_capture_config_validation_zero_sample_rate() {
    let config = AudioCaptureConfig {
        sample_rate: 0,
        ..Default::default()
    };

    let capture_result = MicrophoneCapture::new(config);
    assert!(capture_result.is_err());
}

#[test]
fn test_audio_capture_config_validation_zero_channels() {
    let config = AudioCaptureConfig {
        channels: 0,
        ..Default::default()
    };

    let capture_result = MicrophoneCapture::new(config);
    assert!(capture_result.is_err());
}

#[test]
fn test_audio_capture_config_validation_zero_chunk_duration() {
    let config = AudioCaptureConfig {
        chunk_duration_ms: 0,
        ..Default::default()
    };

    let capture_result = MicrophoneCapture::new(config);
    assert!(capture_result.is_err());
}

#[test]
fn test_audio_capture_config_buffer_size_calculation() {
    let test_cases = vec![
        // (sample_rate, channels, chunk_duration_ms, expected_buffer_size)
        (16000, 1, 100, 1600),
        (48000, 1, 50, 2400),
        (44100, 2, 25, 2205),
        (22050, 1, 200, 4410),
    ];

    for (sample_rate, channels, chunk_duration_ms, expected_buffer_size) in test_cases {
        let config = AudioCaptureConfig {
            sample_rate,
            channels,
            chunk_duration_ms,
        };

        let capture = MicrophoneCapture::new(config).unwrap();
        let audio_config = capture.config();

        assert_eq!(
            audio_config.buffer_size,
            expected_buffer_size,
            "Buffer size calculation failed for sample_rate={}, channels={}, chunk_duration_ms={}",
            sample_rate, channels, chunk_duration_ms
        );
    }
}

#[test]
fn test_audio_capture_config_common_sample_rates() {
    let common_sample_rates = vec![8000, 16000, 22050, 44100, 48000];

    for sample_rate in common_sample_rates {
        let config = AudioCaptureConfig {
            sample_rate,
            ..Default::default()
        };

        let capture_result = MicrophoneCapture::new(config);
        assert!(
            capture_result.is_ok(),
            "Failed to create capture for sample rate {}: {:?}",
            sample_rate,
            capture_result.err()
        );

        let capture = capture_result.unwrap();
        assert_eq!(capture.config().sample_rate, sample_rate);
    }
}

#[test]
fn test_audio_capture_config_mono_stereo() {
    // Test mono
    let mono_config = AudioCaptureConfig {
        channels: 1,
        ..Default::default()
    };
    let mono_capture = MicrophoneCapture::new(mono_config).unwrap();
    assert_eq!(mono_capture.config().channels, 1);

    // Test stereo
    let stereo_config = AudioCaptureConfig {
        channels: 2,
        ..Default::default()
    };
    let stereo_capture = MicrophoneCapture::new(stereo_config).unwrap();
    assert_eq!(stereo_capture.config().channels, 2);
}

#[test]
fn test_audio_capture_config_various_chunk_durations() {
    let chunk_durations = vec![10, 25, 50, 100, 200, 500];

    for chunk_duration_ms in chunk_durations {
        let config = AudioCaptureConfig {
            chunk_duration_ms,
            ..Default::default()
        };

        let capture_result = MicrophoneCapture::new(config);
        assert!(
            capture_result.is_ok(),
            "Failed to create capture for chunk duration {}: {:?}",
            chunk_duration_ms,
            capture_result.err()
        );

        let capture = capture_result.unwrap();
        let audio_config = capture.config();

        let expected_buffer_size = (16000 * 1 * chunk_duration_ms) / 1000;
        assert_eq!(
            audio_config.buffer_size,
            expected_buffer_size,
            "Buffer size mismatch for chunk duration {}ms",
            chunk_duration_ms
        );
    }
}

#[test]
fn test_audio_capture_config_high_quality() {
    let config = AudioCaptureConfig {
        sample_rate: 48000,
        channels: 2,
        chunk_duration_ms: 25,
    };

    let capture = MicrophoneCapture::new(config).unwrap();
    let audio_config = capture.config();

    // High quality stereo capture at 48kHz
    assert_eq!(audio_config.sample_rate, 48000);
    assert_eq!(audio_config.channels, 2);
    assert_eq!(audio_config.buffer_size, 2400); // 48000 * 2 * 25 / 1000
}

#[test]
fn test_audio_capture_config_voice_optimized() {
    let config = AudioCaptureConfig {
        sample_rate: 16000,
        channels: 1,
        chunk_duration_ms: 100,
    };

    let capture = MicrophoneCapture::new(config).unwrap();
    let audio_config = capture.config();

    // Voice-optimized mono capture at 16kHz (common for speech recognition)
    assert_eq!(audio_config.sample_rate, 16000);
    assert_eq!(audio_config.channels, 1);
    assert_eq!(audio_config.buffer_size, 1600); // 16000 * 1 * 100 / 1000
}
