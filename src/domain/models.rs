use serde::{Deserialize, Serialize};

/// Represents a segment of speech-to-text output
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct STTSegment {
    /// The transcribed text
    pub text: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Start time in milliseconds
    pub start_time: u64,
    /// End time in milliseconds
    pub end_time: u64,
    /// Whether this segment is final (true) or interim (false)
    pub is_final: bool,
}

impl STTSegment {
    /// Create a new STT segment
    pub fn new(text: String, confidence: f32, start_time: u64, end_time: u64, is_final: bool) -> Self {
        Self {
            text,
            confidence,
            start_time,
            end_time,
            is_final,
        }
    }

    /// Calculate the duration of this segment in milliseconds
    pub fn duration(&self) -> u64 {
        self.end_time - self.start_time
    }

    /// Validate the segment data
    pub fn validate(&self) -> Result<(), String> {
        if self.confidence < 0.0 || self.confidence > 1.0 {
            return Err("Confidence must be between 0.0 and 1.0".to_string());
        }
        if self.end_time <= self.start_time {
            return Err("End time must be greater than start time".to_string());
        }
        if self.text.is_empty() {
            return Err("Text cannot be empty".to_string());
        }
        Ok(())
    }
}

/// Represents a translation result
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Translation {
    /// The source language code (e.g., "en", "es", "zh")
    pub source_lang: String,
    /// The target language code
    pub target_lang: String,
    /// The original text
    pub original_text: String,
    /// The translated text
    pub translated_text: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
}

impl Translation {
    /// Create a new translation
    pub fn new(
        source_lang: String,
        target_lang: String,
        original_text: String,
        translated_text: String,
        confidence: f32,
    ) -> Self {
        Self {
            source_lang,
            target_lang,
            original_text,
            translated_text,
            confidence,
        }
    }

    /// Validate the translation data
    pub fn validate(&self) -> Result<(), String> {
        if self.confidence < 0.0 || self.confidence > 1.0 {
            return Err("Confidence must be between 0.0 and 1.0".to_string());
        }
        if self.source_lang.is_empty() || self.target_lang.is_empty() {
            return Err("Language codes cannot be empty".to_string());
        }
        if self.original_text.is_empty() || self.translated_text.is_empty() {
            return Err("Text fields cannot be empty".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stt_segment_creation() {
        let segment = STTSegment::new(
            "Hello world".to_string(),
            0.95,
            0,
            1000,
            true,
        );

        assert_eq!(segment.text, "Hello world");
        assert_eq!(segment.confidence, 0.95);
        assert_eq!(segment.start_time, 0);
        assert_eq!(segment.end_time, 1000);
        assert!(segment.is_final);
    }

    #[test]
    fn test_stt_segment_duration() {
        let segment = STTSegment::new(
            "Test".to_string(),
            0.9,
            500,
            1500,
            true,
        );

        assert_eq!(segment.duration(), 1000);
    }

    #[test]
    fn test_stt_segment_validation_valid() {
        let segment = STTSegment::new(
            "Valid text".to_string(),
            0.85,
            0,
            500,
            true,
        );

        assert!(segment.validate().is_ok());
    }

    #[test]
    fn test_stt_segment_validation_invalid_confidence() {
        let segment = STTSegment::new(
            "Text".to_string(),
            1.5, // Invalid confidence
            0,
            500,
            true,
        );

        assert!(segment.validate().is_err());
    }

    #[test]
    fn test_stt_segment_validation_invalid_timing() {
        let segment = STTSegment::new(
            "Text".to_string(),
            0.9,
            1000, // Start time after end time
            500,
            true,
        );

        assert!(segment.validate().is_err());
    }

    #[test]
    fn test_stt_segment_validation_empty_text() {
        let segment = STTSegment::new(
            "".to_string(), // Empty text
            0.9,
            0,
            500,
            true,
        );

        assert!(segment.validate().is_err());
    }

    #[test]
    fn test_translation_creation() {
        let translation = Translation::new(
            "en".to_string(),
            "es".to_string(),
            "Hello".to_string(),
            "Hola".to_string(),
            0.95,
        );

        assert_eq!(translation.source_lang, "en");
        assert_eq!(translation.target_lang, "es");
        assert_eq!(translation.original_text, "Hello");
        assert_eq!(translation.translated_text, "Hola");
        assert_eq!(translation.confidence, 0.95);
    }

    #[test]
    fn test_translation_validation_valid() {
        let translation = Translation::new(
            "en".to_string(),
            "zh".to_string(),
            "Hello world".to_string(),
            "你好世界".to_string(),
            0.9,
        );

        assert!(translation.validate().is_ok());
    }

    #[test]
    fn test_translation_validation_invalid_confidence() {
        let translation = Translation::new(
            "en".to_string(),
            "es".to_string(),
            "Hello".to_string(),
            "Hola".to_string(),
            -0.1, // Invalid confidence
        );

        assert!(translation.validate().is_err());
    }

    #[test]
    fn test_translation_validation_empty_language() {
        let translation = Translation::new(
            "".to_string(), // Empty source language
            "es".to_string(),
            "Hello".to_string(),
            "Hola".to_string(),
            0.9,
        );

        assert!(translation.validate().is_err());
    }

    #[test]
    fn test_translation_validation_empty_text() {
        let translation = Translation::new(
            "en".to_string(),
            "es".to_string(),
            "".to_string(), // Empty original text
            "Hola".to_string(),
            0.9,
        );

        assert!(translation.validate().is_err());
    }

    #[test]
    fn test_translation_serialization() {
        let translation = Translation::new(
            "en".to_string(),
            "es".to_string(),
            "Hello".to_string(),
            "Hola".to_string(),
            0.95,
        );

        // Test serialization
        let json = serde_json::to_string(&translation).unwrap();
        assert!(json.contains("Hello"));
        assert!(json.contains("Hola"));

        // Test deserialization
        let deserialized: Translation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, translation);
    }

    #[test]
    fn test_stt_segment_serialization() {
        let segment = STTSegment::new(
            "Test text".to_string(),
            0.88,
            100,
            200,
            false,
        );

        // Test serialization
        let json = serde_json::to_string(&segment).unwrap();
        assert!(json.contains("Test text"));

        // Test deserialization
        let deserialized: STTSegment = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, segment);
    }
}