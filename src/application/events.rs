//! Event system for Auralis
//!
//! This module defines the event types and event bus for communication
//! between components and the frontend.

use crate::domain::{errors::*, models::*, traits::*};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

/// Events that can occur in the Auralis system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuralisEvent {
    /// Speech-to-text result received
    STTResult {
        segment: STTSegment,
        language: String,
    },

    /// Translation result received
    TranslationResult {
        translation: Translation,
    },

    /// Error occurred
    Error {
        component: String,
        message: String,
    },

    /// Speech activity changed
    SpeechActivityChanged {
        is_speech: bool,
        probability: f32,
    },

    /// Audio capture started/stopped
    AudioCaptureChanged {
        is_capturing: bool,
    },

    /// Status update
    StatusUpdate {
        component: String,
        status: String,
    },
}

impl AuralisEvent {
    /// Create a new STT result event
    pub fn stt_result(segment: STTSegment, language: String) -> Self {
        Self::STTResult { segment, language }
    }

    /// Create a new translation result event
    pub fn translation_result(translation: Translation) -> Self {
        Self::TranslationResult { translation }
    }

    /// Create a new error event
    pub fn error(component: String, message: String) -> Self {
        Self::Error { component, message }
    }

    /// Create a new speech activity event
    pub fn speech_activity(is_speech: bool, probability: f32) -> Self {
        Self::SpeechActivityChanged { is_speech, probability }
    }

    /// Create a new audio capture event
    pub fn audio_capture(is_capturing: bool) -> Self {
        Self::AudioCaptureChanged { is_capturing }
    }

    /// Create a new status update event
    pub fn status_update(component: String, status: String) -> Self {
        Self::StatusUpdate { component, status }
    }
}

/// Event bus for broadcasting events to subscribers
#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<AuralisEvent>,
}

impl EventBus {
    /// Create a new event bus with the given capacity
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Publish an event to all subscribers
    pub fn publish(&self, event: AuralisEvent) -> Result<(), EventBusError> {
        self.sender
            .send(event)
            .map_err(|e| EventBusError::PublishError(e.to_string()))
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<AuralisEvent> {
        self.sender.subscribe()
    }

    /// Get the number of active subscribers
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

/// Errors that can occur in the event bus
#[derive(Debug, thiserror::Error)]
pub enum EventBusError {
    #[error("Failed to publish event: {0}")]
    PublishError(String),

    #[error("Failed to subscribe to events: {0}")]
    SubscribeError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let segment = STTSegment::new("Hello".to_string(), 0.95, 0, 1000, true);
        let event = AuralisEvent::stt_result(segment, "en".to_string());

        match event {
            AuralisEvent::STTResult { segment, language } => {
                assert_eq!(segment.text, "Hello");
                assert_eq!(language, "en");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_event_serialization() {
        let translation = Translation::new(
            "en".to_string(),
            "es".to_string(),
            "Hello".to_string(),
            "Hola".to_string(),
            0.9,
        );
        let event = AuralisEvent::translation_result(translation);

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("Hello"));
        assert!(json.contains("Hola"));
    }

    #[tokio::test]
    async fn test_event_bus() {
        let bus = EventBus::new(100);

        // Test publishing
        let event = AuralisEvent::audio_capture(true);
        assert!(bus.publish(event).is_ok());

        // Test subscription
        let mut receiver = bus.subscribe();
        let event = AuralisEvent::status_update("test".to_string(), "ready".to_string());
        bus.publish(event).unwrap();

        let received = receiver.recv().await.unwrap();
        match received {
            AuralisEvent::StatusUpdate { component, status } => {
                assert_eq!(component, "test");
                assert_eq!(status, "ready");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let bus = EventBus::new(100);

        let mut receiver1 = bus.subscribe();
        let mut receiver2 = bus.subscribe();

        assert_eq!(bus.subscriber_count(), 2);

        let event = AuralisEvent::speech_activity(true, 0.85);
        bus.publish(event).unwrap();

        let received1 = receiver1.recv().await.unwrap();
        let received2 = receiver2.recv().await.unwrap();

        match received1 {
            AuralisEvent::SpeechActivityChanged { is_speech, probability } => {
                assert!(is_speech);
                assert_eq!(probability, 0.85);
            }
            _ => panic!("Wrong event type"),
        }

        match received2 {
            AuralisEvent::SpeechActivityChanged { is_speech, probability } => {
                assert!(is_speech);
                assert_eq!(probability, 0.85);
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_error_event() {
        let event = AuralisEvent::error(
            "STT".to_string(),
            "Connection failed".to_string(),
        );

        match event {
            AuralisEvent::Error { component, message } => {
                assert_eq!(component, "STT");
                assert_eq!(message, "Connection failed");
            }
            _ => panic!("Wrong event type"),
        }
    }
}
