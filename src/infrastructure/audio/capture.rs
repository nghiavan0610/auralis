//! Audio capture implementation using cpal
//!
//! This module provides microphone capture functionality using the cpal library,
//! which provides cross-platform audio capture support.

use crate::domain::{
    errors::AudioError,
    traits::{AudioConfig, AudioSource, AudioStream},
};
use async_trait::async_trait;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, StreamConfig,
};
use futures::stream;
use std::sync::{Arc, Mutex};

/// Configuration for audio capture
#[derive(Debug, Clone)]
pub struct AudioCaptureConfig {
    /// Sample rate in Hz (e.g., 16000, 44100, 48000)
    pub sample_rate: u32,
    /// Number of audio channels (1 = mono, 2 = stereo)
    pub channels: u16,
    /// Duration of each audio chunk in milliseconds
    pub chunk_duration_ms: u32,
}

impl Default for AudioCaptureConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16000,
            channels: 1,
            chunk_duration_ms: 100,
        }
    }
}

impl From<AudioCaptureConfig> for AudioConfig {
    fn from(config: AudioCaptureConfig) -> Self {
        let buffer_size = (config.sample_rate as usize * config.channels as usize
            * config.chunk_duration_ms as usize)
            / 1000;

        AudioConfig {
            sample_rate: config.sample_rate,
            channels: config.channels,
            buffer_size,
        }
    }
}

/// Microphone audio capture using cpal
pub struct MicrophoneCapture {
    config: AudioCaptureConfig,
    audio_config: AudioConfig,
    is_active: bool,
    is_recording: Arc<Mutex<bool>>,
    audio_data: Arc<Mutex<Vec<Vec<f32>>>>,
}

impl MicrophoneCapture {
    /// Create a new microphone capture instance
    pub fn new(config: AudioCaptureConfig) -> Result<Self, AudioError> {
        // Validate configuration
        if config.sample_rate == 0 {
            return Err(AudioError::ConfigError(
                "Sample rate must be greater than 0".to_string(),
            ));
        }
        if config.channels == 0 {
            return Err(AudioError::ConfigError(
                "Channels must be greater than 0".to_string(),
            ));
        }
        if config.chunk_duration_ms == 0 {
            return Err(AudioError::ConfigError(
                "Chunk duration must be greater than 0".to_string(),
            ));
        }

        let audio_config = AudioConfig::from(config.clone());

        Ok(Self {
            config,
            audio_config,
            is_active: false,
            is_recording: Arc::new(Mutex::new(false)),
            audio_data: Arc::new(Mutex::new(Vec::new())),
        })
    }

    /// Create a new microphone capture instance with default configuration
    pub fn default_config() -> Result<Self, AudioError> {
        Self::new(AudioCaptureConfig::default())
    }

    /// Get the default audio input device
    fn get_default_device() -> Result<Device, AudioError> {
        let host = cpal::default_host();
        host.default_input_device()
            .ok_or_else(|| AudioError::DeviceError("No audio input device found".to_string()))
    }

    /// Get available audio input devices
    pub fn available_devices() -> Result<Vec<String>, AudioError> {
        let host = cpal::default_host();
        let devices = host
            .devices()
            .map_err(|e| AudioError::DeviceError(format!("Failed to get devices: {}", e)))?;

        let device_names: Vec<String> = devices
            .filter_map(|d| d.name().ok())
            .collect();

        Ok(device_names)
    }
}

#[async_trait]
impl AudioSource for MicrophoneCapture {
    async fn start(&mut self) -> Result<(), AudioError> {
        if self.is_active {
            return Err(AudioError::ConfigError("Audio capture already active".to_string()));
        }

        let device = Self::get_default_device()?;

        // Create a supported stream configuration
        let supported = device
            .supported_input_configs()
            .map_err(|e| AudioError::DeviceError(format!("Failed to get supported configs: {}", e)))?
            .find(|config| {
                config.min_sample_rate().0 <= self.config.sample_rate
                    && config.max_sample_rate().0 >= self.config.sample_rate
                    && config.channels() == self.config.channels
            })
            .ok_or_else(|| {
                AudioError::ConfigError(format!(
                    "No supported config found for sample_rate={}, channels={}",
                    self.config.sample_rate, self.config.channels
                ))
            })?;

        let stream_config: StreamConfig = supported.with_sample_rate(self.config.sample_rate).into();

        // Setup recording flag and audio data storage
        let is_recording = self.is_recording.clone();
        let audio_data = self.audio_data.clone();
        let channels = self.config.channels;

        // Clear any existing audio data
        if let Ok(mut data) = audio_data.lock() {
            data.clear();
        }

        // Build the audio stream
        let stream = device
            .build_input_stream(
                &stream_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    // Only process audio if we're recording
                    if let Ok(recording) = is_recording.lock() {
                        if *recording {
                            // Convert interleaved data to planar if needed
                            let chunk: Vec<f32> = if channels == 2 {
                                // For stereo, just take the left channel for now
                                data.iter().step_by(2).copied().collect()
                            } else {
                                data.to_vec()
                            };

                            if let Ok(mut buffer) = audio_data.lock() {
                                buffer.push(chunk);
                            }
                        }
                    }
                },
                |err| {
                    eprintln!("Audio capture error: {:?}", err);
                },
                None, // Default buffer size
            )
            .map_err(|e| AudioError::DeviceError(format!("Failed to build input stream: {}", e)))?;

        // Start the stream
        stream
            .play()
            .map_err(|e| AudioError::DeviceError(format!("Failed to start stream: {}", e)))?;

        // Set recording flag to true
        if let Ok(mut recording) = is_recording.lock() {
            *recording = true;
        }

        self.is_active = true;

        Ok(())
    }

    async fn stop(&mut self) -> Result<(), AudioError> {
        if !self.is_active {
            return Err(AudioError::ConfigError("Audio capture not active".to_string()));
        }

        // Stop recording
        if let Ok(mut recording) = self.is_recording.lock() {
            *recording = false;
        }

        self.is_active = false;

        Ok(())
    }

    fn stream(&self) -> Result<AudioStream, AudioError> {
        if !self.is_active {
            return Err(AudioError::ConfigError("Audio capture not active".to_string()));
        }

        let audio_data = self.audio_data.clone();
        let chunk_size = self.audio_config.buffer_size;

        let stream = stream::repeat_with(move || {
            if let Ok(mut data) = audio_data.lock() {
                if data.is_empty() {
                    // Return silence if no data available yet
                    return Ok(vec![0.0f32; chunk_size]);
                }

                // Get the first chunk
                let chunk = data.remove(0);

                // If chunk is smaller than expected, pad with zeros
                if chunk.len() < chunk_size {
                    let mut padded = chunk;
                    padded.resize(chunk_size, 0.0);
                    return Ok(padded);
                }

                // If chunk is larger than expected, split it
                if chunk.len() > chunk_size {
                    let (first, rest) = chunk.split_at(chunk_size);
                    data.insert(0, rest.to_vec());
                    return Ok(first.to_vec());
                }

                Ok(chunk)
            } else {
                Ok(vec![0.0f32; chunk_size])
            }
        });

        Ok(Box::pin(stream))
    }

    fn config(&self) -> &AudioConfig {
        &self.audio_config
    }

    fn is_active(&self) -> bool {
        self.is_active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_capture_config_default() {
        let config = AudioCaptureConfig::default();
        assert_eq!(config.sample_rate, 16000);
        assert_eq!(config.channels, 1);
        assert_eq!(config.chunk_duration_ms, 100);
    }

    #[test]
    fn test_audio_capture_config_from() {
        let capture_config = AudioCaptureConfig {
            sample_rate: 44100,
            channels: 2,
            chunk_duration_ms: 50,
        };

        let audio_config: AudioConfig = capture_config.clone().into();

        assert_eq!(audio_config.sample_rate, 44100);
        assert_eq!(audio_config.channels, 2);
        // buffer_size = 44100 * 2 * 50 / 1000 = 4410
        assert_eq!(audio_config.buffer_size, 4410);
    }

    #[test]
    fn test_microphone_capture_new() {
        let config = AudioCaptureConfig::default();
        let capture = MicrophoneCapture::new(config);
        assert!(capture.is_ok());
    }

    #[test]
    fn test_microphone_capture_new_invalid_sample_rate() {
        let config = AudioCaptureConfig {
            sample_rate: 0,
            ..Default::default()
        };
        let capture = MicrophoneCapture::new(config);
        assert!(capture.is_err());
    }

    #[test]
    fn test_microphone_capture_new_invalid_channels() {
        let config = AudioCaptureConfig {
            channels: 0,
            ..Default::default()
        };
        let capture = MicrophoneCapture::new(config);
        assert!(capture.is_err());
    }

    #[test]
    fn test_microphone_capture_new_invalid_chunk_duration() {
        let config = AudioCaptureConfig {
            chunk_duration_ms: 0,
            ..Default::default()
        };
        let capture = MicrophoneCapture::new(config);
        assert!(capture.is_err());
    }

    #[test]
    fn test_microphone_capture_default_config() {
        let capture = MicrophoneCapture::default_config();
        assert!(capture.is_ok());
    }

    #[test]
    fn test_microphone_capture_config() {
        let capture = MicrophoneCapture::default_config().unwrap();
        let config = capture.config();
        assert_eq!(config.sample_rate, 16000);
        assert_eq!(config.channels, 1);
        assert_eq!(config.buffer_size, 1600); // 16000 * 1 * 100 / 1000
    }

    #[test]
    fn test_microphone_capture_not_active_initially() {
        let capture = MicrophoneCapture::default_config().unwrap();
        assert!(!capture.is_active());
    }

    #[test]
    fn test_audio_config_conversion() {
        let capture_config = AudioCaptureConfig {
            sample_rate: 48000,
            channels: 1,
            chunk_duration_ms: 25,
        };

        let audio_config: AudioConfig = capture_config.clone().into();
        assert_eq!(audio_config.sample_rate, 48000);
        assert_eq!(audio_config.channels, 1);
        assert_eq!(audio_config.buffer_size, 1200); // 48000 * 1 * 25 / 1000
    }

    #[test]
    fn test_stereo_config_buffer_size() {
        let capture_config = AudioCaptureConfig {
            sample_rate: 48000,
            channels: 2,
            chunk_duration_ms: 50,
        };

        let audio_config: AudioConfig = capture_config.clone().into();
        assert_eq!(audio_config.buffer_size, 4800); // 48000 * 2 * 50 / 1000
    }
}
