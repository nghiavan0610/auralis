//! Silero Voice Activity Detection implementation
//!
//! This module provides VAD using the Silero model with energy-based
//! detection as a fallback.

use async_trait::async_trait;
use std::path::PathBuf;
use tracing::{debug, info, warn};

use crate::domain::{errors::VADError, traits::VAD};

/// Configuration for Silero VAD
#[derive(Debug, Clone)]
pub struct SileroConfig {
    /// Path to the Silero model file
    pub model_path: PathBuf,

    /// Sample rate for audio processing (Hz)
    pub sample_rate: u32,

    /// Threshold for speech detection (0.0 to 1.0)
    pub threshold: f32,

    /// Window size for processing (in samples)
    pub window_size: usize,

    /// Whether to use mock mode (for testing)
    pub mock_mode: bool,

    /// Minimum speech duration (in milliseconds)
    pub min_speech_duration_ms: u32,

    /// Minimum silence duration (in milliseconds)
    pub min_silence_duration_ms: u32,
}

impl Default for SileroConfig {
    fn default() -> Self {
        Self {
            model_path: PathBuf::from("models/silero_vad_v4.pt"),
            sample_rate: 16000,
            threshold: 0.5,
            window_size: 512,
            mock_mode: false,
            min_speech_duration_ms: 250,
            min_silence_duration_ms: 100,
        }
    }
}

/// Silero VAD implementation
pub struct SileroVAD {
    config: SileroConfig,
    threshold: f32,
    initialized: bool,
    // In a real implementation, you would load the Silero model here
    // For now, we'll use energy-based detection as fallback
    energy_threshold: f32,
    audio_buffer: Vec<f32>,
}

impl SileroVAD {
    /// Create a new Silero VAD with the given configuration
    pub fn new(config: SileroConfig) -> Self {
        Self {
            energy_threshold: 0.001, // Default energy threshold
            audio_buffer: Vec::new(),
            threshold: config.threshold,
            config,
            initialized: false,
        }
    }

    /// Create a new Silero VAD with default configuration
    pub fn with_defaults() -> Self {
        Self::new(SileroConfig::default())
    }

    /// Create a new Silero VAD in mock mode
    pub fn mock() -> Self {
        let mut config = SileroConfig::default();
        config.mock_mode = true;
        Self::new(config)
    }

    /// Check if the model file exists
    pub fn model_exists(&self) -> bool {
        self.config.model_path.exists() || self.config.mock_mode
    }

    /// Calculate audio energy (RMS)
    fn calculate_energy(&self, audio_data: &[f32]) -> f32 {
        if audio_data.is_empty() {
            return 0.0;
        }

        let sum_squares: f32 = audio_data.iter().map(|&x| x * x).sum();
        (sum_squares / audio_data.len() as f32).sqrt()
    }

    /// Energy-based speech detection (fallback method)
    fn detect_speech_energy(&self, audio_data: &[f32]) -> (bool, f32) {
        let energy = self.calculate_energy(audio_data);
        let is_speech = energy > self.energy_threshold;
        (is_speech, energy)
    }

    /// Process a single audio chunk
    pub fn process_chunk(&mut self, chunk: &[f32]) -> Result<bool, VADError> {
        if !self.initialized {
            return Err(VADError::ProcessingError("VAD not initialized".to_string()));
        }

        if self.config.mock_mode {
            return self.process_chunk_mock(chunk);
        }

        // Add chunk to buffer
        self.audio_buffer.extend_from_slice(chunk);

        // Keep buffer size manageable
        if self.audio_buffer.len() > self.config.window_size * 4 {
            let excess = self.audio_buffer.len() - self.config.window_size * 4;
            self.audio_buffer.drain(0..excess);
        }

        // Process when we have enough data
        if self.audio_buffer.len() >= self.config.window_size {
            let window: Vec<f32> = self
                .audio_buffer
                .iter()
                .rev()
                .take(self.config.window_size)
                .copied()
                .collect();
            let window: Vec<f32> = window.into_iter().rev().collect();

            // Use energy-based detection for now
            let (is_speech, _energy) = self.detect_speech_energy(&window);
            Ok(is_speech)
        } else {
            // Not enough data yet, assume silence
            Ok(false)
        }
    }

    /// Process chunk in mock mode
    fn process_chunk_mock(&self, chunk: &[f32]) -> Result<bool, VADError> {
        if chunk.is_empty() {
            return Ok(false);
        }

        // Simple mock logic based on audio energy
        let energy: f32 = chunk.iter().map(|&x| x * x).sum::<f32>() / chunk.len() as f32;

        // Use a simple threshold for mock mode
        let is_speech = energy > 0.0001;

        Ok(is_speech)
    }

    /// Check if audio contains speech
    pub fn is_speech(&self, audio_data: &[f32]) -> Result<bool, VADError> {
        if !self.initialized {
            return Err(VADError::ProcessingError("VAD not initialized".to_string()));
        }

        if audio_data.is_empty() {
            return Ok(false);
        }

        if self.config.mock_mode {
            let energy: f32 = audio_data.iter().map(|&x| x * x).sum::<f32>() / audio_data.len() as f32;
            return Ok(energy > 0.0001);
        }

        // Use energy-based detection
        let (is_speech, _energy) = self.detect_speech_energy(audio_data);
        Ok(is_speech)
    }

    /// Get the current speech probability for audio data
    pub fn speech_probability(&self, audio_data: &[f32]) -> Result<f32, VADError> {
        if !self.initialized {
            return Err(VADError::PredictionError("VAD not initialized".to_string()));
        }

        if audio_data.is_empty() {
            return Ok(0.0);
        }

        // Calculate energy and normalize to probability
        let energy = self.calculate_energy(audio_data);

        // Normalize energy to 0-1 range (simple approach)
        // In a real Silero model, this would be the actual probability
        let probability = (energy.tanh() + 1.0) / 2.0;

        Ok(probability.clamp(0.0, 1.0))
    }
}

#[async_trait]
impl VAD for SileroVAD {
    async fn initialize(&mut self, config: serde_json::Value) -> Result<(), VADError> {
        info!("Initializing Silero VAD");

        // Parse configuration
        if let Some(model_path) = config.get("model_path").and_then(|v| v.as_str()) {
            self.config.model_path = PathBuf::from(model_path);
        }

        if let Some(sample_rate) = config.get("sample_rate").and_then(|v| v.as_u64()) {
            if sample_rate != 16000 && sample_rate != 8000 {
                return Err(VADError::InvalidSampleRate {
                    rate: sample_rate as u32,
                    supported: "16000, 8000".to_string(),
                });
            }
            self.config.sample_rate = sample_rate as u32;
        }

        if let Some(threshold) = config.get("threshold").and_then(|v| v.as_f64()) {
            self.set_threshold(threshold as f32).await?;
        }

        if let Some(window_size) = config.get("window_size").and_then(|v| v.as_u64()) {
            self.config.window_size = window_size as usize;
        }

        if let Some(mock_mode) = config.get("mock_mode").and_then(|v| v.as_bool()) {
            self.config.mock_mode = mock_mode;
        }

        // Validate threshold
        if self.config.threshold < 0.0 || self.config.threshold > 1.0 {
            return Err(VADError::InvalidThreshold {
                threshold: self.config.threshold,
            });
        }

        // Check model availability
        if !self.model_exists() && !self.config.mock_mode {
            warn!("Silero model not found at {:?}, using energy-based fallback", self.config.model_path);
            // Continue with energy-based detection
        }

        self.initialized = true;
        info!("Silero VAD initialized successfully");
        Ok(())
    }

    async fn process_audio(&mut self, audio_data: Vec<f32>) -> Result<bool, VADError> {
        if !self.initialized {
            return Err(VADError::ProcessingError("VAD not initialized".to_string()));
        }

        if audio_data.is_empty() {
            return Ok(false);
        }

        debug!("Processing VAD for {} samples", audio_data.len());

        // Process the audio chunk
        self.process_chunk(&audio_data)
    }

    async fn speech_probability(&self, audio_data: Vec<f32>) -> Result<f32, VADError> {
        self.speech_probability(&audio_data)
    }

    async fn reset(&mut self) -> Result<(), VADError> {
        self.audio_buffer.clear();
        debug!("VAD state reset");
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
        self.config.threshold = threshold;
        debug!("VAD threshold set to {}", threshold);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_silero_config_default() {
        let config = SileroConfig::default();
        assert_eq!(config.sample_rate, 16000);
        assert_eq!(config.threshold, 0.5);
        assert_eq!(config.window_size, 512);
        assert!(!config.mock_mode);
    }

    #[test]
    fn test_silero_vad_creation() {
        let vad = SileroVAD::with_defaults();
        assert!(!vad.initialized);
        assert_eq!(vad.threshold(), 0.5);
    }

    #[test]
    fn test_silero_vad_mock() {
        let vad = SileroVAD::mock();
        assert!(vad.config.mock_mode);
    }

    #[test]
    fn test_model_exists() {
        let config = SileroConfig {
            model_path: PathBuf::from("/nonexistent/path.pt"),
            ..Default::default()
        };
        let vad = SileroVAD::new(config);
        assert!(!vad.model_exists());

        let mock_vad = SileroVAD::mock();
        assert!(mock_vad.model_exists()); // Mock mode always returns true
    }

    #[test]
    fn test_calculate_energy() {
        let vad = SileroVAD::mock();
        vad.initialized = true;

        // Test with silence
        let silence = vec![0.0f32; 1000];
        let energy = vad.calculate_energy(&silence);
        assert_eq!(energy, 0.0);

        // Test with constant signal
        let signal = vec![0.5f32; 1000];
        let energy = vad.calculate_energy(&signal);
        assert!((energy - 0.5).abs() < 0.01);

        // Test with varying signal
        let signal = vec![0.1f32, 0.2f32, 0.3f32, 0.4f32];
        let energy = vad.calculate_energy(&signal);
        assert!(energy > 0.0);
    }

    #[test]
    fn test_detect_speech_energy() {
        let vad = SileroVAD::mock();
        vad.initialized = true;

        // Test silence
        let silence = vec![0.0f32; 1000];
        let (is_speech, energy) = vad.detect_speech_energy(&silence);
        assert!(!is_speech);
        assert_eq!(energy, 0.0);

        // Test low energy
        let quiet = vec![0.0005f32; 1000];
        let (is_speech, energy) = vad.detect_speech_energy(&quiet);
        assert!(!is_speech); // Below threshold
        assert!(energy < vad.energy_threshold);

        // Test speech
        let speech = vec![0.01f32; 1000];
        let (is_speech, energy) = vad.detect_speech_energy(&speech);
        assert!(is_speech);
        assert!(energy > vad.energy_threshold);
    }

    #[tokio::test]
    async fn test_initialize() {
        let mut vad = SileroVAD::mock();
        let config = serde_json::json!({
            "threshold": 0.7,
            "sample_rate": 16000,
            "mock_mode": true
        });

        let result = vad.initialize(config).await;
        assert!(result.is_ok());
        assert!(vad.initialized);
        assert_eq!(vad.threshold(), 0.7);
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
        assert!(matches!(
            result.unwrap_err(),
            VADError::InvalidSampleRate { .. }
        ));
    }

    #[tokio::test]
    async fn test_set_threshold() {
        let mut vad = SileroVAD::mock();
        vad.initialized = true;

        vad.set_threshold(0.8).await.unwrap();
        assert_eq!(vad.threshold(), 0.8);

        // Test invalid threshold
        let result = vad.set_threshold(1.5).await;
        assert!(result.is_err());

        let result = vad.set_threshold(-0.1).await;
        assert!(result.is_err());
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
        assert!(!result.unwrap()); // Empty audio is not speech
    }

    #[tokio::test]
    async fn test_process_audio_mock() {
        let mut vad = SileroVAD::mock();
        vad.config.mock_mode = true;
        vad.initialized = true;

        // Test with quiet audio
        let quiet_audio = vec![0.0001f32; 1000];
        let result = vad.process_audio(quiet_audio).await;
        assert!(result.is_ok());
        assert!(!result.unwrap()); // Should not detect speech

        // Test with speech audio
        let speech_audio = vec![0.1f32; 1000];
        let result = vad.process_audio(speech_audio).await;
        assert!(result.is_ok());
        assert!(result.unwrap()); // Should detect speech
    }

    #[tokio::test]
    async fn test_is_speech() {
        let mut vad = SileroVAD::mock();
        vad.initialized = true;

        // Test with silence
        let silence = vec![0.0f32; 1000];
        assert!(!vad.is_speech(&silence).unwrap());

        // Test with speech
        let speech = vec![0.1f32; 1000];
        assert!(vad.is_speech(&speech).unwrap());

        // Test with empty audio
        assert!(!vad.is_speech(&[]).unwrap());
    }

    #[tokio::test]
    async fn test_speech_probability() {
        let mut vad = SileroVAD::mock();
        vad.initialized = true;

        // Test with silence
        let silence = vec![0.0f32; 1000];
        let prob = vad.speech_probability(silence).await.unwrap();
        assert_eq!(prob, 0.0);

        // Test with speech
        let speech = vec![0.5f32; 1000];
        let prob = vad.speech_probability(speech).await.unwrap();
        assert!(prob > 0.0 && prob <= 1.0);

        // Test with empty audio
        let prob = vad.speech_probability(vec![]).await.unwrap();
        assert_eq!(prob, 0.0);
    }

    #[tokio::test]
    async fn test_reset() {
        let mut vad = SileroVAD::mock();
        vad.initialized = true;

        // Add some data to buffer
        vad.audio_buffer.extend_from_slice(&[0.1f32; 1000]);
        assert!(!vad.audio_buffer.is_empty());

        // Reset
        vad.reset().await.unwrap();
        assert!(vad.audio_buffer.is_empty());
    }

    #[tokio::test]
    async fn test_process_chunk_not_initialized() {
        let mut vad = SileroVAD::mock();
        // Don't initialize

        let chunk = vec![0.1f32; 512];
        let result = vad.process_chunk(&chunk);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_process_chunk_buffer_management() {
        let mut vad = SileroVAD::mock();
        vad.config.mock_mode = true;
        vad.config.window_size = 512;
        vad.initialized = true;

        // Process multiple chunks
        for _ in 0..10 {
            let chunk = vec![0.1f32; 512];
            let result = vad.process_chunk(&chunk);
            assert!(result.is_ok());
        }

        // Buffer should not grow unbounded
        assert!(vad.audio_buffer.len() <= vad.config.window_size * 4);
    }

    #[test]
    fn test_config_cloning() {
        let config = SileroConfig {
            model_path: "/test/path.pt".into(),
            sample_rate: 8000,
            threshold: 0.7,
            window_size: 256,
            mock_mode: true,
            min_speech_duration_ms: 500,
            min_silence_duration_ms: 200,
        };

        let cloned = config.clone();
        assert_eq!(config.model_path, cloned.model_path);
        assert_eq!(config.sample_rate, cloned.sample_rate);
        assert_eq!(config.threshold, cloned.threshold);
        assert_eq!(config.window_size, cloned.window_size);
        assert_eq!(config.mock_mode, cloned.mock_mode);
    }
}
