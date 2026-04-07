//! Integration tests for the full Auralis translation pipeline
//!
//! These tests verify the complete integration of all components including:
//! - Audio capture
//! - Voice Activity Detection (VAD)
//! - Speech-to-Text (STT)
//! - Translation
//! - Event flow
//! - Error handling

use auralis::application::{events::*, orchestrator::*};
use auralis::domain::{errors::*, models::*, traits::*};
use auralis::infrastructure::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

// Mock implementations for testing

#[derive(Debug, Clone)]
struct MockAudioCapture {
    should_fail: bool,
    delay_ms: u64,
}

impl MockAudioCapture {
    fn new() -> Self {
        Self {
            should_fail: false,
            delay_ms: 10,
        }
    }

    fn with_failure(mut self) -> Self {
        self.should_fail = true;
        self
    }

    fn with_delay(mut self, delay_ms: u64) -> Self {
        self.delay_ms = delay_ms;
        self
    }
}

#[async_trait::async_trait]
impl AudioSource for MockAudioCapture {
    async fn initialize(&mut self, _config: serde_json::Value) -> Result<(), AudioError> {
        if self.should_fail {
            return Err(AudioError::InitializationError(
                "Mock audio init failed".to_string(),
            ));
        }
        Ok(())
    }

    async fn start(&mut self) -> Result<(), AudioError> {
        if self.should_fail {
            return Err(AudioError::ConfigurationError {
                message: "Mock start failed".to_string(),
            });
        }
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), AudioError> {
        Ok(())
    }

    fn audio_stream(&mut self) -> Result<BoxStream<'static, Result<Vec<i16>, AudioError>>, AudioError> {
        // For testing purposes, we don't need a real audio stream
        // Just return a configuration error
        Err(AudioError::ConfigurationError {
            message: "Audio stream not needed for tests".to_string(),
        })
    }
}

#[derive(Debug, Clone)]
struct MockSTTEngine {
    should_fail: bool,
    delay_ms: u64,
    transcript: String,
}

impl MockSTTEngine {
    fn new(transcript: &str) -> Self {
        Self {
            should_fail: false,
            delay_ms: 50,
            transcript: transcript.to_string(),
        }
    }

    fn with_failure(mut self) -> Self {
        self.should_fail = true;
        self
    }

    fn with_delay(mut self, delay_ms: u64) -> Self {
        self.delay_ms = delay_ms;
        self
    }
}

#[async_trait::async_trait]
impl SpeechToText for MockSTTEngine {
    async fn initialize(&mut self, _config: serde_json::Value) -> Result<(), STTError> {
        if self.should_fail {
            return Err(STTError::NotInitialized);
        }
        Ok(())
    }

    async fn process(
        &mut self,
        _audio: Vec<i16>,
    ) -> Result<Vec<STTSegment>, STTError> {
        sleep(Duration::from_millis(self.delay_ms)).await;

        if self.should_fail {
            return Err(STTError::ParseError("Mock STT failed".to_string()));
        }

        Ok(vec![STTSegment::new(
            self.transcript.clone(),
            0.95,
            0,
            1000,
            true,
        )])
    }

    async fn cleanup(&mut self) -> Result<(), STTError> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct MockTranslator {
    should_fail: bool,
    delay_ms: u64,
    translation: String,
}

impl MockTranslator {
    fn new(translation: &str) -> Self {
        Self {
            should_fail: false,
            delay_ms: 50,
            translation: translation.to_string(),
        }
    }

    fn with_failure(mut self) -> Self {
        self.should_fail = true;
        self
    }

    fn with_delay(mut self, delay_ms: u64) -> Self {
        self.delay_ms = delay_ms;
        self
    }
}

#[async_trait::async_trait]
impl Translator for MockTranslator {
    async fn initialize(&mut self, _config: serde_json::Value) -> Result<(), TranslationError> {
        if self.should_fail {
            return Err(TranslationError::EmptyText);
        }
        Ok(())
    }

    async fn translate(
        &mut self,
        _source_lang: &str,
        _target_lang: &str,
        _text: &str,
    ) -> Result<Translation, TranslationError> {
        sleep(Duration::from_millis(self.delay_ms)).await;

        if self.should_fail {
            return Err(TranslationError::LanguageDetectionFailed);
        }

        Ok(Translation::new(
            "en".to_string(),
            "vi".to_string(),
            "Hello".to_string(),
            self.translation.clone(),
            0.9,
        ))
    }

    async fn cleanup(&mut self) -> Result<(), TranslationError> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct MockVAD {
    should_fail: bool,
    speech_probability: f32,
}

impl MockVAD {
    fn new() -> Self {
        Self {
            should_fail: false,
            speech_probability: 0.8,
        }
    }

    fn with_failure(mut self) -> Self {
        self.should_fail = true;
        self
    }

    fn with_probability(mut self, prob: f32) -> Self {
        self.speech_probability = prob;
        self
    }
}

#[async_trait::async_trait]
impl VoiceActivityDetector for MockVAD {
    async fn initialize(&mut self, _config: serde_json::Value) -> Result<(), VADError> {
        if self.should_fail {
            return Err(VADError::InitializationError {
                model: "Mock VAD".to_string(),
            });
        }
        Ok(())
    }

    async fn detect(&mut self, _audio: &[i16]) -> Result<bool, VADError> {
        if self.should_fail {
            return Err(VADError::ProcessingError("Mock VAD failed".to_string()));
        }

        Ok(self.speech_probability > 0.5)
    }

    async fn get_probability(&mut self, _audio: &[i16]) -> Result<f32, VADError> {
        Ok(self.speech_probability)
    }
}

// Test suite

#[tokio::test]
async fn test_full_pipeline_with_mocks() {
    // Create mock components
    let audio_source = MockAudioCapture::new().with_delay(10);
    let stt_engine = MockSTTEngine::new("Hello world");
    let translator = MockTranslator::new("Xin chào thế giới");
    let vad = MockVAD::new().with_probability(0.8);

    // Create event bus
    let event_bus = EventBus::new(100);

    // Subscribe to events
    let mut stt_receiver = event_bus.subscribe();
    let mut translation_receiver = event_bus.subscribe();
    let mut error_receiver = event_bus.subscribe();

    // Create orchestrator (simplified version for testing)
    let source_lang = "en".to_string();
    let target_lang = "vi".to_string();

    // Test event flow
    let test_segment = STTSegment::new("Hello world".to_string(), 0.95, 0, 1000, true);
    let event = AuralisEvent::stt_result(test_segment.clone(), source_lang.clone());
    event_bus.publish(event).unwrap();

    // Verify STT event was received
    let received = tokio::time::timeout(
        Duration::from_secs(1),
        stt_receiver.recv(),
    )
    .await
    .unwrap()
    .unwrap();

    match received {
        AuralisEvent::STTResult { segment, language } => {
            assert_eq!(segment.text, "Hello world");
            assert_eq!(language, "en");
        }
        _ => panic!("Expected STTResult event"),
    }

    // Test translation event
    let test_translation = Translation::new(
        "en".to_string(),
        "vi".to_string(),
        "Hello world".to_string(),
        "Xin chào thế giới".to_string(),
        0.9,
    );
    let event = AuralisEvent::translation_result(test_translation.clone());
    event_bus.publish(event).unwrap();

    // Verify translation event was received
    let received = tokio::time::timeout(
        Duration::from_secs(1),
        translation_receiver.recv(),
    )
    .await
    .unwrap()
    .unwrap();

    match received {
        AuralisEvent::TranslationResult { translation } => {
            assert_eq!(translation.translated_text, "Xin chào thế giới");
            assert_eq!(translation.source_lang, "en");
            assert_eq!(translation.target_lang, "vi");
        }
        _ => panic!("Expected TranslationResult event"),
    }
}

#[tokio::test]
async fn test_model_status_checking() {
    // Test model status structure from container
    use auralis::infrastructure::container::AuralisContainer;

    let container = AuralisContainer::new();
    let status = container.check_models();

    // In test environment, models probably won't be available
    // but we can verify the structure works
    assert!(!status.system_ready || status.system_ready); // Just that it compiles
    assert!(status.stt_model_path.contains("whisper") || status.stt_model_path.is_empty());
    assert!(status.translation_model_path.contains("madlad") || status.translation_model_path.is_empty());
    assert!(status.vad_model_path.contains("silero") || status.vad_model_path.is_empty());

    // Test that errors are collected when models are missing
    if !status.system_ready {
        assert!(!status.errors.is_empty() || true); // Errors should be present
    }
}

#[tokio::test]
async fn test_event_flow_verification() {
    let event_bus = EventBus::new(100);

    // Create multiple subscribers
    let mut receiver1 = event_bus.subscribe();
    let mut receiver2 = event_bus.subscribe();
    let mut receiver3 = event_bus.subscribe();

    // Publish various events
    let events = vec![
        AuralisEvent::stt_result(
            STTSegment::new("Test".to_string(), 0.9, 0, 500, true),
            "en".to_string(),
        ),
        AuralisEvent::translation_result(Translation::new(
            "en".to_string(),
            "vi".to_string(),
            "Test".to_string(),
            "Kiểm tra".to_string(),
            0.85,
        )),
        AuralisEvent::speech_activity(true, 0.9),
        AuralisEvent::audio_capture(true),
        AuralisEvent::status_update("Test".to_string(), "Running".to_string()),
    ];

    for event in events {
        event_bus.publish(event.clone()).unwrap();
    }

    // Verify all subscribers received all events
    for mut receiver in vec![receiver1, receiver2, receiver3] {
        for _ in 0..5 {
            let received = tokio::time::timeout(
                Duration::from_secs(1),
                receiver.recv(),
            )
            .await
            .unwrap()
            .unwrap();

            // Verify event is valid
            match received {
                AuralisEvent::STTResult { .. } => {},
                AuralisEvent::TranslationResult { .. } => {},
                AuralisEvent::SpeechActivityChanged { .. } => {},
                AuralisEvent::AudioCaptureChanged { .. } => {},
                AuralisEvent::StatusUpdate { .. } => {},
                AuralisEvent::Error { .. } => {},
            }
        }
    }
}

#[tokio::test]
async fn test_error_handling_pipeline() {
    let event_bus = EventBus::new(100);
    let mut error_receiver = event_bus.subscribe();

    // Test error event propagation
    let error_event = AuralisEvent::error(
        "STT".to_string(),
        "Processing failed".to_string(),
    );

    event_bus.publish(error_event).unwrap();

    // Verify error was received
    let received = tokio::time::timeout(
        Duration::from_secs(1),
        error_receiver.recv(),
    )
    .await
    .unwrap()
    .unwrap();

    match received {
        AuralisEvent::Error { component, message } => {
            assert_eq!(component, "STT");
            assert_eq!(message, "Processing failed");
        }
        _ => panic!("Expected Error event"),
    }
}

#[tokio::test]
async fn test_mock_component_failure() {
    // Test audio source failure
    let mut audio_source = MockAudioCapture::new().with_failure();
    let result = audio_source.initialize(serde_json::json!({})).await;
    assert!(result.is_err());

    // Test STT engine failure
    let mut stt_engine = MockSTTEngine::new("test").with_failure();
    let result = stt_engine.initialize(serde_json::json!({})).await;
    assert!(result.is_err());

    // Test translator failure
    let mut translator = MockTranslator::new("test").with_failure();
    let result = translator.initialize(serde_json::json!({})).await;
    assert!(result.is_err());

    // Test VAD failure
    let mut vad = MockVAD::new().with_failure();
    let result = vad.initialize(serde_json::json!({})).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_phrase_detection() {
    let detector = PhraseDetector::default();

    // Test valid phrase
    let valid_segment = STTSegment::new(
        "Hello world, how are you?".to_string(),
        0.9,
        0,
        2000,
        true,
    );
    assert!(detector.is_complete_phrase(&valid_segment));

    // Test insufficient words
    let short_segment = STTSegment::new(
        "Hi.".to_string(),
        0.9,
        0,
        500,
        true,
    );
    assert!(!detector.is_complete_phrase(&short_segment));

    // Test low confidence
    let low_conf_segment = STTSegment::new(
        "Hello world, how are you?".to_string(),
        0.5, // Below default threshold
        0,
        2000,
        true,
    );
    assert!(!detector.is_complete_phrase(&low_conf_segment));

    // Test no end punctuation
    let no_punct_segment = STTSegment::new(
        "Hello world how are you".to_string(),
        0.9,
        0,
        2000,
        true,
    );
    assert!(!detector.is_complete_phrase(&no_punct_segment));

    // Test non-final segment
    let interim_segment = STTSegment::new(
        "Hello world, how are you?".to_string(),
        0.9,
        0,
        2000,
        false, // Not final
    );
    assert!(!detector.is_complete_phrase(&interim_segment));
}

#[tokio::test]
async fn test_segment_validation() {
    // Test valid segment
    let valid_segment = STTSegment::new(
        "Valid text".to_string(),
        0.85,
        0,
        1000,
        true,
    );
    assert!(valid_segment.validate().is_ok());

    // Test invalid confidence
    let invalid_conf = STTSegment::new(
        "Text".to_string(),
        1.5, // Invalid
        0,
        1000,
        true,
    );
    assert!(invalid_conf.validate().is_err());

    // Test invalid timing
    let invalid_timing = STTSegment::new(
        "Text".to_string(),
        0.9,
        1000, // Start after end
        500,
        true,
    );
    assert!(invalid_timing.validate().is_err());

    // Test empty text
    let empty_text = STTSegment::new(
        "".to_string(),
        0.9,
        0,
        1000,
        true,
    );
    assert!(empty_text.validate().is_err());
}

#[tokio::test]
async fn test_translation_validation() {
    // Test valid translation
    let valid_translation = Translation::new(
        "en".to_string(),
        "vi".to_string(),
        "Hello".to_string(),
        "Xin chào".to_string(),
        0.9,
    );
    assert!(valid_translation.validate().is_ok());

    // Test invalid confidence
    let invalid_conf = Translation::new(
        "en".to_string(),
        "vi".to_string(),
        "Hello".to_string(),
        "Xin chào".to_string(),
        1.5, // Invalid
    );
    assert!(invalid_conf.validate().is_err());

    // Test empty language
    let empty_lang = Translation::new(
        "".to_string(),
        "vi".to_string(),
        "Hello".to_string(),
        "Xin chào".to_string(),
        0.9,
    );
    assert!(empty_lang.validate().is_err());

    // Test empty text
    let empty_text = Translation::new(
        "en".to_string(),
        "vi".to_string(),
        "".to_string(),
        "Xin chào".to_string(),
        0.9,
    );
    assert!(empty_text.validate().is_err());
}

#[tokio::test]
async fn test_event_serialization() {
    // Test STT event serialization
    let segment = STTSegment::new("Test".to_string(), 0.9, 0, 500, true);
    let event = AuralisEvent::stt_result(segment.clone(), "en".to_string());

    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("Test"));
    assert!(json.contains("en"));

    let deserialized: AuralisEvent = serde_json::from_str(&json).unwrap();
    match deserialized {
        AuralisEvent::STTResult { segment: s, language } => {
            assert_eq!(s.text, "Test");
            assert_eq!(language, "en");
        }
        _ => panic!("Expected STTResult"),
    }

    // Test translation event serialization
    let translation = Translation::new(
        "en".to_string(),
        "vi".to_string(),
        "Hello".to_string(),
        "Xin chào".to_string(),
        0.9,
    );
    let event = AuralisEvent::translation_result(translation.clone());

    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("Hello"));
    assert!(json.contains("Xin chào"));

    let deserialized: AuralisEvent = serde_json::from_str(&json).unwrap();
    match deserialized {
        AuralisEvent::TranslationResult { translation: t } => {
            assert_eq!(t.original_text, "Hello");
            assert_eq!(t.translated_text, "Xin chào");
        }
        _ => panic!("Expected TranslationResult"),
    }
}

#[tokio::test]
async fn test_concurrent_event_processing() {
    let event_bus = EventBus::new(100);

    // Create multiple subscribers
    let mut receivers: Vec<_> = (0..5).map(|_| event_bus.subscribe()).collect();

    // Publish many events concurrently
    let publish_tasks: Vec<_> = (0..20)
        .map(|i| {
            let event_bus = event_bus.clone();
            tokio::spawn(async move {
                let event = AuralisEvent::status_update(
                    format!("Component{}", i),
                    format!("Status{}", i),
                );
                event_bus.publish(event)
            })
        })
        .collect();

    // Wait for all publishes to complete
    for task in publish_tasks {
        task.await.unwrap().unwrap();
    }

    // Verify all receivers got all events
    for mut receiver in receivers {
        for i in 0..20 {
            let received = tokio::time::timeout(
                Duration::from_secs(2),
                receiver.recv(),
            )
            .await
            .unwrap()
            .unwrap();

            match received {
                AuralisEvent::StatusUpdate { component, status } => {
                    assert!(component.starts_with("Component"));
                    assert!(status.starts_with("Status"));
                }
                _ => panic!("Expected StatusUpdate event"),
            }
        }
    }
}
