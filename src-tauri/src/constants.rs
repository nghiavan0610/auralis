//! Constants and configuration values for Auralis
//!
//! This module provides a single source of truth for all magic numbers,
//! timeouts, and configuration values used throughout the application.

// ---------------------------------------------------------------------------
// Timing and Latency Constants
// ---------------------------------------------------------------------------

/// Audio polling interval for the local pipeline (50ms for low latency)
#[allow(dead_code)]
pub const AUDIO_POLLING_INTERVAL_MS: u64 = 50;

/// Audio polling interval for cloud mode capture (200ms)
#[allow(dead_code)]
pub const AUDIO_CAPTURE_POLLING_MS: u64 = 200;

// ---------------------------------------------------------------------------
// Subscription Constants
// ---------------------------------------------------------------------------

/// Free tier monthly summary limit
pub const FREE_TIER_SUMMARY_LIMIT: u32 = 5;

/// Pro tier monthly summary limit
pub const PRO_TIER_SUMMARY_LIMIT: u32 = 500;

// ---------------------------------------------------------------------------
// Display Constants
// ---------------------------------------------------------------------------

/// Default window opacity
pub const DEFAULT_OPACITY: f64 = 0.88;

/// Default font size in pixels
pub const DEFAULT_FONT_SIZE: u32 = 14;

/// Default transcript lines to display
pub const DEFAULT_MAX_LINES: u32 = 100;

/// Default endpoint delay in seconds
pub const DEFAULT_ENDPOINT_DELAY: f64 = 1.5;

// ---------------------------------------------------------------------------
// TTS Constants
// ---------------------------------------------------------------------------

/// Default TTS speech rate
pub const DEFAULT_TTS_RATE: f64 = 1.0;

// ---------------------------------------------------------------------------
// Enums for type-safe values (used by default functions)
// ---------------------------------------------------------------------------

/// Translation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum TranslationType {
    OneWay,
    TwoWay,
}

impl TranslationType {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::OneWay => "one_way",
            Self::TwoWay => "two_way",
        }
    }
}

/// Audio source
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum AudioSource {
    Microphone,
    System,
    Both,
}

impl AudioSource {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Microphone => "microphone",
            Self::System => "system",
            Self::Both => "both",
        }
    }
}

/// Subscription tier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SubscriptionTier {
    Free,
    Pro,
}

impl SubscriptionTier {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Pro => "pro",
        }
    }

    #[allow(dead_code)]
    pub const fn summary_limit(&self) -> u32 {
        match self {
            Self::Free => FREE_TIER_SUMMARY_LIMIT,
            Self::Pro => PRO_TIER_SUMMARY_LIMIT,
        }
    }
}

/// TTS provider
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum TtsProvider {
    WebSpeech,
    Edge,
    Google,
    ElevenLabs,
}

impl TtsProvider {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::WebSpeech => "webspeech",
            Self::Edge => "edge",
            Self::Google => "google",
            Self::ElevenLabs => "elevenlabs",
        }
    }
}

// ---------------------------------------------------------------------------
// Default value functions for serde
// ---------------------------------------------------------------------------

pub fn default_translation_type() -> String {
    TranslationType::OneWay.as_str().to_string()
}

pub fn default_audio_source() -> String {
    AudioSource::Microphone.as_str().to_string()
}

pub fn default_opacity() -> f64 {
    DEFAULT_OPACITY
}

pub fn default_font_size() -> u32 {
    DEFAULT_FONT_SIZE
}

pub fn default_max_lines() -> u32 {
    DEFAULT_MAX_LINES
}

pub fn default_endpoint_delay() -> f64 {
    DEFAULT_ENDPOINT_DELAY
}

pub fn default_tts_enabled() -> bool {
    false
}

pub fn default_tts_rate() -> f64 {
    DEFAULT_TTS_RATE
}

pub fn default_tts_provider() -> String {
    TtsProvider::WebSpeech.as_str().to_string()
}

pub fn default_subscription_tier() -> String {
    SubscriptionTier::Free.as_str().to_string()
}

pub fn default_confidence_filter_level() -> String {
    "low".to_string()
}
