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
use serde::{Serialize, Deserialize};
use std::sync::{Arc, Mutex};

/// Configuration for audio capture
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// The actual sample rate the device uses (may differ from requested)
    actual_sample_rate: u32,
    /// The actual channel count the device uses (may differ from requested)
    actual_channels: u16,
    /// Flag to signal the stream thread to stop
    stream_stop: Arc<std::sync::atomic::AtomicBool>,
}

impl MicrophoneCapture {
    /// Create a new microphone capture instance
    pub fn new(config: AudioCaptureConfig) -> Result<Self, AudioError> {
        // Validate configuration
        if config.sample_rate == 0 {
            return Err(AudioError::ConfigurationError {
                message: "Sample rate must be greater than 0".to_string(),
            });
        }
        if config.channels == 0 {
            return Err(AudioError::ConfigurationError {
                message: "Channels must be greater than 0".to_string(),
            });
        }
        if config.chunk_duration_ms == 0 {
            return Err(AudioError::ConfigurationError {
                message: "Chunk duration must be greater than 0".to_string(),
            });
        }

        let actual_sample_rate = config.sample_rate;
        let actual_channels = config.channels;
        let audio_config = AudioConfig::from(config.clone());

        Ok(Self {
            config,
            audio_config,
            is_active: false,
            is_recording: Arc::new(Mutex::new(false)),
            audio_data: Arc::new(Mutex::new(Vec::new())),
            actual_sample_rate,
            actual_channels,
            stream_stop: Arc::new(std::sync::atomic::AtomicBool::new(false)),
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
            .ok_or_else(|| AudioError::DeviceOpenError {
                device: "default".to_string(),
            })
    }

    /// Get a shared reference to the internal audio data buffer
    pub fn audio_data(&self) -> Arc<Mutex<Vec<Vec<f32>>>> {
        self.audio_data.clone()
    }

    /// Get a shared reference to the recording flag
    pub fn is_recording_flag(&self) -> Arc<Mutex<bool>> {
        self.is_recording.clone()
    }

    /// Get a shared reference to the stream stop flag
    pub fn stream_stop_flag(&self) -> Arc<std::sync::atomic::AtomicBool> {
        self.stream_stop.clone()
    }

    /// Get available audio input devices
    pub fn available_devices() -> Result<Vec<String>, AudioError> {
        let host = cpal::default_host();
        let devices = host
            .devices()
            .map_err(|e| AudioError::InitializationError(format!("Failed to get devices: {}", e)))?;

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
            return Err(AudioError::InitializationError("Audio capture already active".to_string()));
        }

        let device = Self::get_default_device()?;

        // Collect supported configs into a Vec so we can search multiple times
        let supported_configs: Vec<_> = device
            .supported_input_configs()
            .map_err(|e| AudioError::ConfigurationError {
                message: format!("Failed to get supported configs: {}", e),
            })?
            .collect();

        // Try exact match first (16kHz mono)
        let exact_match = supported_configs.iter().find(|config| {
            config.min_sample_rate().0 <= self.config.sample_rate
                && config.max_sample_rate().0 >= self.config.sample_rate
                && config.channels() == self.config.channels
        });

        let stream_config: StreamConfig = if let Some(supported) = exact_match {
            self.actual_sample_rate = self.config.sample_rate;
            self.actual_channels = self.config.channels;
            supported.with_sample_rate(cpal::SampleRate(self.config.sample_rate)).into()
        } else {
            // Fall back to device default config
            let default_config = device
                .default_input_config()
                .map_err(|e| AudioError::ConfigurationError {
                    message: format!("Failed to get default input config: {}", e),
                })?;

            self.actual_sample_rate = default_config.sample_rate().0;
            self.actual_channels = default_config.channels();

            tracing::info!(
                "Using device native format: {}Hz, {}ch (will resample to {}Hz {}ch for processing)",
                self.actual_sample_rate, self.actual_channels,
                self.config.sample_rate, self.config.channels
            );

            default_config.into()
        };

        // Setup recording flag and audio data storage
        let is_recording = self.is_recording.clone();
        let is_recording_for_closure = is_recording.clone();
        let audio_data = self.audio_data.clone();
        let actual_sample_rate = self.actual_sample_rate;
        let actual_channels = self.actual_channels;
        let target_sample_rate = self.config.sample_rate;

        // Clear any existing audio data
        if let Ok(mut data) = audio_data.lock() {
            data.clear();
        }

        // Build the audio stream - cpal::Stream is !Send so we must create it
        // and keep it alive entirely within a dedicated non-async thread.
        self.stream_stop.store(true, std::sync::atomic::Ordering::Relaxed);
        let stop_flag = self.stream_stop.clone();
        let started_ok = Arc::new(std::sync::Mutex::new(false));
        let started_ok_clone = started_ok.clone();
        let start_err = Arc::new(std::sync::Mutex::new(None::<String>));
        let start_err_clone = start_err.clone();

        std::thread::spawn(move || {
            let stream = match device.build_input_stream(
                &stream_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if let Ok(recording) = is_recording_for_closure.lock() {
                        if *recording {
                            let mono: Vec<f32> = if actual_channels > 1 {
                                data.iter().step_by(actual_channels as usize).copied().collect()
                            } else {
                                data.to_vec()
                            };

                            let resampled: Vec<f32> = if actual_sample_rate != target_sample_rate {
                                let ratio = target_sample_rate as f64 / actual_sample_rate as f64;
                                let new_len = (mono.len() as f64 * ratio) as usize;
                                let mut output = Vec::with_capacity(new_len);
                                for i in 0..new_len {
                                    let src_idx = i as f64 / ratio;
                                    let idx = src_idx as usize;
                                    if idx + 1 < mono.len() {
                                        let frac = src_idx - idx as f64;
                                        output.push(
                                            (mono[idx] as f64 * (1.0 - frac)
                                                + mono[idx + 1] as f64 * frac) as f32,
                                        );
                                    } else if idx < mono.len() {
                                        output.push(mono[idx]);
                                    }
                                }
                                output
                            } else {
                                mono
                            };

                            if let Ok(mut buffer) = audio_data.lock() {
                                buffer.push(resampled);
                            }
                        }
                    }
                },
                |err| {
                    eprintln!("Audio capture error: {:?}", err);
                },
                None,
            ) {
                Ok(s) => s,
                Err(e) => {
                    *start_err_clone.lock().unwrap() = Some(format!("Failed to build input stream: {}", e));
                    return;
                }
            };

            if let Err(e) = stream.play() {
                *start_err_clone.lock().unwrap() = Some(format!("Failed to start stream: {}", e));
                return;
            }

            *started_ok_clone.lock().unwrap() = true;

            // Keep stream alive until stop signal
            while stop_flag.load(std::sync::atomic::Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            // Stream dropped here
        });

        // Wait for the stream thread to start (or fail)
        std::thread::sleep(std::time::Duration::from_millis(100));

        if let Some(err) = start_err.lock().unwrap().take() {
            return Err(AudioError::InitializationError(err));
        }

        // Set recording flag to true
        if let Ok(mut recording) = is_recording.lock() {
            *recording = true;
        }

        self.is_active = true;

        Ok(())
    }

    async fn stop(&mut self) -> Result<(), AudioError> {
        if !self.is_active {
            return Err(AudioError::InitializationError("Audio capture not active".to_string()));
        }

        // Stop recording
        if let Ok(mut recording) = self.is_recording.lock() {
            *recording = false;
        }

        // Signal the stream thread to stop (drops the cpal stream)
        self.stream_stop.store(false, std::sync::atomic::Ordering::Relaxed);
        // Give it a moment to clean up
        std::thread::sleep(std::time::Duration::from_millis(200));
        self.is_active = false;

        Ok(())
    }

    fn stream(&self) -> Result<AudioStream, AudioError> {
        if !self.is_active {
            return Err(AudioError::InitializationError("Audio capture not active".to_string()));
        }

        let audio_data = self.audio_data.clone();
        let chunk_size = self.audio_config.buffer_size;

        // Use an interval-based stream that yields every 50ms to match
        // the audio capture rate, avoiding a busy-loop that starves the
        // mic callback from writing to the shared buffer.
        let stream = stream::unfold(0u64, move |_count| {
            let audio_data = audio_data.clone();
            async move {
                // Sleep ~50ms to match the audio chunk cadence
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;

                let result = if let Ok(mut data) = audio_data.lock() {
                    if data.is_empty() {
                        // No data yet - return None to skip this tick
                        None
                    } else {
                        // Collect all available chunks into one buffer
                        let mut combined: Vec<f32> = Vec::new();
                        while !data.is_empty() && combined.len() < chunk_size * 2 {
                            let chunk = data.remove(0);
                            combined.extend(chunk);
                        }

                        if combined.is_empty() {
                            None
                        } else if combined.len() < chunk_size {
                            combined.resize(chunk_size, 0.0);
                            Some(Ok(combined))
                        } else if combined.len() > chunk_size {
                            let (first, rest) = combined.split_at(chunk_size);
                            if !rest.is_empty() {
                                data.insert(0, rest.to_vec());
                            }
                            Some(Ok(first.to_vec()))
                        } else {
                            Some(Ok(combined))
                        }
                    }
                } else {
                    None
                };

                // Return Some((item, next_state)) to continue, or stop if no data
                match result {
                    Some(item) => Some((item, 0)),
                    None => {
                        // Return silence to keep the stream alive
                        Some((Ok(vec![0.0f32; chunk_size]), 0))
                    }
                }
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
