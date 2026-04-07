use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

use crate::domain::errors::*;

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

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn test_audio_config_default() {
        let config = AudioConfig::default();
        assert_eq!(config.sample_rate, 16000);
        assert_eq!(config.channels, 1);
        assert_eq!(config.buffer_size, 4096);
    }
}
