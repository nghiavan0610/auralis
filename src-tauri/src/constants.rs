//! Constants and configuration values for Auralis
//!
//! This module provides a single source of truth for all magic numbers,
//! timeouts, and configuration values used throughout the application.

use std::time::Duration;

// ---------------------------------------------------------------------------
// Audio Processing Constants
// ---------------------------------------------------------------------------

/// Audio sample rate in Hz (16kHz for speech recognition)
pub const AUDIO_SAMPLE_RATE: u32 = 16000;

/// Audio channels (mono)
pub const AUDIO_CHANNELS: u16 = 1;

/// Audio format (PCM s16le)
pub const AUDIO_FORMAT: &str = "pcm_s16le";

// ---------------------------------------------------------------------------
// Timing and Latency Constants
// ---------------------------------------------------------------------------

/// Audio polling interval for the local pipeline (50ms for low latency)
pub const AUDIO_POLLING_INTERVAL_MS: u64 = 50;

/// Audio polling interval for cloud mode capture (200ms)
pub const AUDIO_CAPTURE_POLLING_MS: u64 = 200;

/// System audio capture keep-alive sleep interval (100ms)
pub const SYSTEM_AUDIO_KEEPALIVE_MS: u64 = 100;

/// Audio stop confirmation delay (300ms)
pub const AUDIO_STOP_DELAY_MS: u64 = 300;

/// How often to log audio flow statistics (5 seconds)
pub const AUDIO_LOG_INTERVAL_MS: u64 = 5000;

/// Convert to Duration
pub const fn audio_polling_interval() -> Duration {
    Duration::from_millis(AUDIO_POLLING_INTERVAL_MS)
}

pub const fn audio_capture_polling() -> Duration {
    Duration::from_millis(AUDIO_CAPTURE_POLLING_MS)
}

pub const fn audio_stop_delay() -> Duration {
    Duration::from_millis(AUDIO_STOP_DELAY_MS)
}

// ---------------------------------------------------------------------------
// Process Management Constants
// ---------------------------------------------------------------------------

/// Initial delay for process graceful shutdown (50ms)
pub const PROCESS_SHUTDOWN_INITIAL_DELAY_MS: u64 = 50;

/// Maximum total wait time for process graceful shutdown (1.55 seconds)
pub const PROCESS_SHUTDOWN_MAX_WAIT_MS: u64 = 1550;

/// Process shutdown exponential backoff sequence: 50, 100, 200, 400, 800 ms
pub const PROCESS_SHUTDOWN_BACKOFF_MS: [u64; 5] = [50, 100, 200, 400, 800];

pub const fn process_shutdown_initial_delay() -> Duration {
    Duration::from_millis(PROCESS_SHUTDOWN_INITIAL_DELAY_MS)
}

// ---------------------------------------------------------------------------
// Subscription Constants
// ---------------------------------------------------------------------------

/// Free tier monthly summary limit
pub const FREE_TIER_SUMMARY_LIMIT: u32 = 5;

/// Pro tier monthly summary limit
pub const PRO_TIER_SUMMARY_LIMIT: u32 = 500;

/// Default reset day of month (1st of each month)
pub const SUMMARY_RESET_DAY: u32 = 1;

// ---------------------------------------------------------------------------
// WebSocket Status Codes (RFC 6455)
// ---------------------------------------------------------------------------

/// Normal closure (1000)
pub const WS_CLOSE_NORMAL: u16 = 1000;

/// Abnormal closure (1006) - should not be sent manually
pub const WS_CLOSE_ABNORMAL: u16 = 1006;

/// Going away (1001)
pub const WS_CLOSE_GOING_AWAY: u16 = 1001;

// ---------------------------------------------------------------------------
// Soniox API Error Codes
// ---------------------------------------------------------------------------

/// Invalid API key (4001, 4003)
pub const SONIOX_ERR_INVALID_API_KEY: u16 = 4001;

/// Subscription issue (4002)
pub const SONIOX_ERR_SUBSCRIPTION: u16 = 4002;

/// Rate limit exceeded (4029)
pub const SONIOX_ERR_RATE_LIMIT: u16 = 4029;

// ---------------------------------------------------------------------------
// Display Constants
// ---------------------------------------------------------------------------

/// Minimum window opacity (30%)
pub const MIN_OPACITY: f64 = 0.3;

/// Maximum window opacity (100%)
pub const MAX_OPACITY: f64 = 1.0;

/// Default window opacity
pub const DEFAULT_OPACITY: f64 = 0.88;

/// Minimum font size in pixels
pub const MIN_FONT_SIZE: u32 = 12;

/// Maximum font size in pixels
pub const MAX_FONT_SIZE: u32 = 24;

/// Default font size in pixels
pub const DEFAULT_FONT_SIZE: u32 = 14;

/// Minimum transcript lines to display
pub const MIN_MAX_LINES: u32 = 10;

/// Maximum transcript lines to display
pub const MAX_MAX_LINES: u32 = 200;

/// Default transcript lines to display
pub const DEFAULT_MAX_LINES: u32 = 100;

/// Minimum endpoint delay in seconds
pub const MIN_ENDPOINT_DELAY: f64 = 0.5;

/// Maximum endpoint delay in seconds
pub const MAX_ENDPOINT_DELAY: f64 = 3.0;

/// Default endpoint delay in seconds
pub const DEFAULT_ENDPOINT_DELAY: f64 = 1.5;

// ---------------------------------------------------------------------------
// TTS Constants
// ---------------------------------------------------------------------------

/// Minimum TTS speech rate
pub const MIN_TTS_RATE: f64 = 0.5;

/// Maximum TTS speech rate
pub const MAX_TTS_RATE: f64 = 2.0;

/// Default TTS speech rate
pub const DEFAULT_TTS_RATE: f64 = 1.0;

// ---------------------------------------------------------------------------
// Summary Generation Constants
// ---------------------------------------------------------------------------

/// Maximum input tokens for Gemma free tier
pub const GEMMA_MAX_TOKENS_FREE: u32 = 100;

/// Maximum input tokens for Gemma with expansion
pub const GEMMA_MAX_TOKENS_EXPANDED: u32 = 200;

/// Maximum input tokens for GPT-4o-mini
pub const GPT_MAX_TOKENS: u32 = 2048;

/// Default temperature for Gemma translation
pub const GEMMA_TRANSLATION_TEMPERATURE: f32 = 0.3;

/// Default temperature for Gemma summary
pub const GEMMA_SUMMARY_TEMPERATURE: f32 = 0.4;

// ---------------------------------------------------------------------------
// Enums for type-safe values
// ---------------------------------------------------------------------------

/// Operating mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatingMode {
    Cloud,
    Offline,
}

impl OperatingMode {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Cloud => "cloud",
            Self::Offline => "offline",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "cloud" => Some(Self::Cloud),
            "offline" => Some(Self::Offline),
            _ => None,
        }
    }
}

/// Translation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "one_way" => Some(Self::OneWay),
            "two_way" => Some(Self::TwoWay),
            _ => None,
        }
    }
}

/// Audio source
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "microphone" => Some(Self::Microphone),
            "system" => Some(Self::System),
            "both" => Some(Self::Both),
            _ => None,
        }
    }
}

/// Subscription tier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "free" => Some(Self::Free),
            "pro" => Some(Self::Pro),
            _ => None,
        }
    }

    pub const fn summary_limit(&self) -> u32 {
        match self {
            Self::Free => FREE_TIER_SUMMARY_LIMIT,
            Self::Pro => PRO_TIER_SUMMARY_LIMIT,
        }
    }
}

/// TTS provider
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "webspeech" => Some(Self::WebSpeech),
            "edge" => Some(Self::Edge),
            "google" => Some(Self::Google),
            "elevenlabs" => Some(Self::ElevenLabs),
            _ => None,
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
