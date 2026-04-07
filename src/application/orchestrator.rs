//! Orchestrator for coordinating all Auralis components
//!
//! This module provides the main orchestration logic that coordinates
//! audio capture, STT, translation, and VAD components.

use crate::application::events::{AuralisEvent, EventBus};
use crate::domain::traits::*;
use crate::domain::{errors::*, models::*};
use futures::StreamExt;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::Instant;

/// Phrase detector with configurable rules
#[derive(Debug, Clone)]
pub struct PhraseDetector {
    /// Minimum word count for a valid phrase
    min_words: usize,
    /// Maximum phrase duration in seconds
    max_duration_secs: u64,
    /// Silence timeout in milliseconds before considering a phrase complete
    silence_timeout_ms: u64,
    /// Required punctuation marks to end a phrase
    end_punctuation: Vec<char>,
    /// Minimum confidence threshold
    min_confidence: f32,
}

impl Default for PhraseDetector {
    fn default() -> Self {
        Self {
            min_words: 2,
            max_duration_secs: 30,
            silence_timeout_ms: 1500,
            end_punctuation: vec!['.', '!', '?', '。', '！', '？'],
            min_confidence: 0.7,
        }
    }
}

impl PhraseDetector {
    /// Create a new phrase detector with custom configuration
    pub fn new(
        min_words: usize,
        max_duration_secs: u64,
        silence_timeout_ms: u64,
        min_confidence: f32,
    ) -> Self {
        Self {
            min_words,
            max_duration_secs,
            silence_timeout_ms,
            end_punctuation: vec!['.', '!', '?', '。', '！', '？'],
            min_confidence,
        }
    }

    /// Set the end punctuation marks
    pub fn with_end_punctuation(mut self, punctuation: Vec<char>) -> Self {
        self.end_punctuation = punctuation;
        self
    }

    /// Check if a segment is a complete phrase
    pub fn is_complete_phrase(&self, segment: &STTSegment) -> bool {
        // Check confidence threshold
        if segment.confidence < self.min_confidence {
            return false;
        }

        // Check if it's final
        if !segment.is_final {
            return false;
        }

        // Check word count
        let word_count = segment.text.split_whitespace().count();
        if word_count < self.min_words {
            return false;
        }

        // Check duration
        let duration_secs = segment.duration() as f64 / 1000.0;
        if duration_secs > self.max_duration_secs as f64 {
            return false;
        }

        // Check for end punctuation
        let has_end_punctuation = segment
            .text
            .chars()
            .last()
            .map(|c| self.end_punctuation.contains(&c))
            .unwrap_or(false);

        has_end_punctuation
    }

    /// Check if a segment should trigger translation based on timeout
    pub fn should_trigger_translation(
        &self,
        segment: &STTSegment,
        last_speech_time: Option<Instant>,
    ) -> bool {
        // First check if it's a complete phrase
        if self.is_complete_phrase(segment) {
            return true;
        }

        // Check if silence timeout has elapsed
        if let Some(last_time) = last_speech_time {
            let elapsed = last_time.elapsed();
            if elapsed.as_millis() >= self.silence_timeout_ms as u128 {
                // Check minimum requirements
                let word_count = segment.text.split_whitespace().count();
                word_count >= self.min_words && segment.confidence >= self.min_confidence
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Get the minimum words setting
    pub fn min_words(&self) -> usize {
        self.min_words
    }

    /// Get the silence timeout setting
    pub fn silence_timeout(&self) -> Duration {
        Duration::from_millis(self.silence_timeout_ms)
    }

    /// Get the minimum confidence setting
    pub fn min_confidence(&self) -> f32 {
        self.min_confidence
    }
}

/// Main orchestrator for coordinating all components
pub struct Orchestrator<A, S, T, V>
where
    A: AudioSource,
    S: STTEngine,
    T: Translator,
    V: VAD,
{
    audio_source: Arc<Mutex<A>>,
    stt_engine: Arc<Mutex<S>>,
    translator: Arc<Mutex<T>>,
    vad: Arc<Mutex<V>>,
    event_bus: EventBus,
    phrase_detector: PhraseDetector,
    source_language: String,
    target_language: String,
    is_running: Arc<Mutex<bool>>,
    last_speech_time: Arc<Mutex<Option<Instant>>>,
}

impl<A, S, T, V> Orchestrator<A, S, T, V>
where
    A: AudioSource,
    S: STTEngine,
    T: Translator,
    V: VAD,
{
    /// Create a new orchestrator with the given components
    pub fn new(
        audio_source: A,
        stt_engine: S,
        translator: T,
        vad: V,
        source_language: String,
        target_language: String,
    ) -> Self {
        let event_bus = EventBus::new(1000);
        let phrase_detector = PhraseDetector::default();

        Self {
            audio_source: Arc::new(Mutex::new(audio_source)),
            stt_engine: Arc::new(Mutex::new(stt_engine)),
            translator: Arc::new(Mutex::new(translator)),
            vad: Arc::new(Mutex::new(vad)),
            event_bus,
            phrase_detector,
            source_language,
            target_language,
            is_running: Arc::new(Mutex::new(false)),
            last_speech_time: Arc::new(Mutex::new(None)),
        }
    }

    /// Create a new orchestrator with a custom phrase detector
    pub fn with_phrase_detector(mut self, detector: PhraseDetector) -> Self {
        self.phrase_detector = detector;
        self
    }

    /// Get the event bus for subscribing to events
    pub fn event_bus(&self) -> EventBus {
        self.event_bus.clone()
    }

    /// Start the translation pipeline
    pub async fn start(&self) -> Result<(), OrchestratorError> {
        let mut running = self.is_running.lock().await;
        if *running {
            return Err(OrchestratorError::AlreadyRunning);
        }
        *running = true;
        drop(running);

        // Start audio capture
        let mut audio = self.audio_source.lock().await;
        audio.start().await.map_err(|e| {
            let _ = self.event_bus.publish(AuralisEvent::error(
                "AudioCapture",
                format!("Failed to start: {}", e),
            ));
            OrchestratorError::AudioError(e.to_string())
        })?;

        let _ = self.event_bus.publish(AuralisEvent::audio_capture(true));

        // Spawn the processing task
        let audio_source = self.audio_source.clone();
        let stt_engine = self.stt_engine.clone();
        let translator = self.translator.clone();
        let vad = self.vad.clone();
        let event_bus = self.event_bus.clone();
        let phrase_detector = self.phrase_detector.clone();
        let source_lang = self.source_language.clone();
        let target_lang = self.target_language.clone();
        let is_running = self.is_running.clone();
        let last_speech_time = self.last_speech_time.clone();

        tokio::spawn(async move {
            let mut audio_buffer = Vec::new();
            let mut text_buffer = String::new();
            let mut segment_start_time: Option<u64> = None;

            while *is_running.lock().await {
                // Get audio stream
                let audio = audio_source.lock().await;
                let stream_result = audio.stream();
                drop(audio);

                if let Ok(mut stream) = stream_result {
                    while let Some(result) = stream.next().await {
                        if !*is_running.lock().await {
                            break;
                        }

                        let audio_data = match result {
                            Ok(data) => data,
                            Err(e) => {
                                let _ = event_bus.publish(AuralisEvent::error(
                                    "AudioStream",
                                    format!("Stream error: {}", e),
                                ));
                                continue;
                            }
                        };

                        // Process with VAD
                        let mut vad_guard = vad.lock().await;
                        let speech_prob = vad_guard.speech_probability(audio_data.clone()).await;
                        drop(vad_guard);

                        let is_speech = match speech_prob {
                            Ok(prob) => {
                                let _ = event_bus.publish(AuralisEvent::speech_activity(
                                    prob >= 0.5,
                                    prob,
                                ));
                                prob >= 0.5
                            }
                            Err(e) => {
                                let _ = event_bus.publish(AuralisEvent::error(
                                    "VAD",
                                    format!("VAD error: {}", e),
                                ));
                                false
                            }
                        };

                        if is_speech {
                            // Update last speech time
                            *last_speech_time.lock().await = Some(Instant::now());

                            // Add to audio buffer
                            audio_buffer.extend(audio_data);

                            // Process with STT
                            let mut stt = stt_engine.lock().await;
                            let segments_result = stt.process_audio(audio_buffer.clone()).await;
                            drop(stt);

                            if let Ok(segments) = segments_result {
                                for segment in segments {
                                    if segment.is_final {
                                        text_buffer.push_str(&segment.text);
                                        text_buffer.push(' ');

                                        if segment_start_time.is_none() {
                                            segment_start_time = Some(segment.start_time);
                                        }

                                        let _ = event_bus.publish(AuralisEvent::stt_result(
                                            segment.clone(),
                                            source_lang.clone(),
                                        ));

                                        // Check if we should translate
                                        let last_speech = *last_speech_time.lock().await;
                                        if phrase_detector.should_trigger_translation(
                                            &segment,
                                            last_speech,
                                        ) {
                                            // Translate the accumulated text
                                            let text_to_translate = text_buffer.trim().to_string();
                                            if !text_to_translate.is_empty() {
                                                let mut trans = translator.lock().await;
                                                let translation_result = trans
                                                    .translate(
                                                        text_to_translate.clone(),
                                                        source_lang.clone(),
                                                        target_lang.clone(),
                                                    )
                                                    .await;
                                                drop(trans);

                                                if let Ok(translation) = translation_result {
                                                    let _ = event_bus.publish(
                                                        AuralisEvent::translation_result(
                                                            translation,
                                                        ),
                                                    );
                                                }

                                                // Clear buffers
                                                text_buffer.clear();
                                                audio_buffer.clear();
                                                segment_start_time = None;
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            // Clear buffer if no speech for a while
                            let last_speech = *last_speech_time.lock().await;
                            if let Some(last) = last_speech {
                                if last.elapsed() > phrase_detector.silence_timeout() {
                                    audio_buffer.clear();
                                }
                            }
                        }
                    }
                }

                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });

        Ok(())
    }

    /// Stop the translation pipeline
    pub async fn stop(&self) -> Result<(), OrchestratorError> {
        let mut running = self.is_running.lock().await;
        if !*running {
            return Err(OrchestratorError::NotRunning);
        }
        *running = false;
        drop(running);

        // Stop audio capture
        let mut audio = self.audio_source.lock().await;
        audio.stop().await.map_err(|e| {
            let _ = self.event_bus.publish(AuralisEvent::error(
                "AudioCapture",
                format!("Failed to stop: {}", e),
            ));
            OrchestratorError::AudioError(e.to_string())
        })?;

        let _ = self.event_bus.publish(AuralisEvent::audio_capture(false));

        Ok(())
    }

    /// Check if the orchestrator is currently running
    pub async fn is_running(&self) -> bool {
        *self.is_running.lock().await
    }

    /// Set the source language
    pub async fn set_source_language(&mut self, language: String) -> Result<(), OrchestratorError> {
        let mut stt = self.stt_engine.lock().await;
        stt.set_language(language.clone()).await.map_err(|e| {
            OrchestratorError::STTError(format!("Failed to set language: {}", e))
        })?;
        self.source_language = language;
        Ok(())
    }

    /// Set the target language
    pub async fn set_target_language(&mut self, language: String) {
        self.target_language = language;
    }

    /// Get the current phrase detector configuration
    pub fn phrase_detector(&self) -> &PhraseDetector {
        &self.phrase_detector
    }

    /// Update the phrase detector configuration
    pub fn update_phrase_detector(&mut self, detector: PhraseDetector) {
        self.phrase_detector = detector;
    }
}

/// Errors that can occur in the orchestrator
#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    #[error("Orchestrator is already running")]
    AlreadyRunning,

    #[error("Orchestrator is not running")]
    NotRunning,

    #[error("Audio error: {0}")]
    AudioError(String),

    #[error("STT error: {0}")]
    STTError(String),

    #[error("Translation error: {0}")]
    TranslationError(String),

    #[error("VAD error: {0}")]
    VADError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phrase_detector_default() {
        let detector = PhraseDetector::default();
        assert_eq!(detector.min_words(), 2);
        assert_eq!(detector.min_confidence(), 0.7);
        assert_eq!(detector.silence_timeout(), Duration::from_millis(1500));
    }

    #[test]
    fn test_phrase_detector_custom() {
        let detector = PhraseDetector::new(3, 20, 2000, 0.8);
        assert_eq!(detector.min_words(), 3);
        assert_eq!(detector.min_confidence(), 0.8);
        assert_eq!(detector.silence_timeout(), Duration::from_millis(2000));
    }

    #[test]
    fn test_phrase_detector_complete_phrase() {
        let detector = PhraseDetector::default();

        // Valid complete phrase
        let segment = STTSegment::new("Hello world!".to_string(), 0.9, 0, 1000, true);
        assert!(detector.is_complete_phrase(&segment));

        // Too short
        let segment = STTSegment::new("Hi!".to_string(), 0.9, 0, 1000, true);
        assert!(!detector.is_complete_phrase(&segment));

        // Too low confidence
        let segment = STTSegment::new("Hello world!".to_string(), 0.5, 0, 1000, true);
        assert!(!detector.is_complete_phrase(&segment));

        // No end punctuation
        let segment = STTSegment::new("Hello world".to_string(), 0.9, 0, 1000, true);
        assert!(!detector.is_complete_phrase(&segment));

        // Not final
        let segment = STTSegment::new("Hello world!".to_string(), 0.9, 0, 1000, false);
        assert!(!detector.is_complete_phrase(&segment));
    }

    #[test]
    fn test_phrase_detector_timeout() {
        let detector = PhraseDetector::new(2, 30, 100, 0.7);

        let segment = STTSegment::new("Hello world".to_string(), 0.9, 0, 1000, true);

        // No timeout yet
        let last_time = Some(Instant::now());
        assert!(!detector.should_trigger_translation(&segment, last_time));

        // Timeout elapsed
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert!(detector.should_trigger_translation(&segment, last_time));
    }

    #[test]
    fn test_phrase_detector_with_punctuation() {
        let detector = PhraseDetector::default().with_end_punctuation(vec!['.']);

        // Has correct punctuation
        let segment = STTSegment::new("Hello world.".to_string(), 0.9, 0, 1000, true);
        assert!(detector.is_complete_phrase(&segment));

        // Wrong punctuation
        let segment = STTSegment::new("Hello world!".to_string(), 0.9, 0, 1000, true);
        assert!(!detector.is_complete_phrase(&segment));
    }
}
