/**
 * Constants and configuration values for Auralis
 *
 * This module provides a single source of truth for all magic numbers,
 * timeouts, and configuration values used throughout the frontend.
 */

// ---------------------------------------------------------------------------
// WebSocket Status Codes (RFC 6455)
// ---------------------------------------------------------------------------

export const WS_CLOSE_NORMAL = 1000;

// ---------------------------------------------------------------------------
// Timing and Latency Constants (milliseconds)
// ---------------------------------------------------------------------------

// Session management
export const SESSION_DURATION_MS = 3 * 60 * 1000; // 3 minutes
export const KEEPALIVE_INTERVAL_MS = 15 * 1000; // 15 seconds

// Subscription sync
export const SUBSCRIPTION_SYNC_INTERVAL_MS = 5 * 60 * 1000; // 5 minutes

// UI timeouts
export const ERROR_TOAST_DURATION_MS = 5000;
export const SETTINGS_DEBOUNCE_MS = 1000;

// ---------------------------------------------------------------------------
// Display Constants
// ---------------------------------------------------------------------------

export const MIN_OPACITY = 0.3;
export const MAX_OPACITY = 1.0;
export const DEFAULT_OPACITY = 0.88;

export const MIN_FONT_SIZE = 12;
export const MAX_FONT_SIZE = 24;
export const DEFAULT_FONT_SIZE = 14;

export const MIN_MAX_LINES = 10;
export const MAX_MAX_LINES = 200;
export const DEFAULT_MAX_LINES = 100;

export const MIN_ENDPOINT_DELAY = 0.5;
export const MAX_ENDPOINT_DELAY = 3.0;
export const DEFAULT_ENDPOINT_DELAY = 1.5;

// ---------------------------------------------------------------------------
// Subscription Constants
// ---------------------------------------------------------------------------

export const FREE_TIER_SUMMARY_LIMIT = 5;
export const PRO_TIER_SUMMARY_LIMIT = 500;

// ---------------------------------------------------------------------------
// Audio Constants
// ---------------------------------------------------------------------------

export const AUDIO_SAMPLE_RATE = 16000;
export const AUDIO_CHANNELS = 1;
export const AUDIO_FORMAT = 'pcm_s16le';

// ---------------------------------------------------------------------------
// Translation Constants
// ---------------------------------------------------------------------------

export const SONIOX_ENDPOINT = "wss://stt-rt.soniox.com/transcribe-websocket";
export const CONTEXT_HISTORY_CHARS = 1500;
export const MAX_RECONNECT_ATTEMPTS = 3;
export const BASE_RECONNECT_DELAY_MS = 2000;

// ---------------------------------------------------------------------------
// TTS Constants
// ---------------------------------------------------------------------------

export const MIN_TTS_RATE = 0.5;
export const MAX_TTS_RATE = 2.0;
export const DEFAULT_TTS_RATE = 1.0;

// ---------------------------------------------------------------------------
// Enums (using const objects for type safety)
// ---------------------------------------------------------------------------

export const OperatingMode = {
  CLOUD: 'cloud',
  OFFLINE: 'offline',
} as const;

export type OperatingMode = typeof OperatingMode[keyof typeof OperatingMode];

export const TranslationType = {
  ONE_WAY: 'one_way',
  TWO_WAY: 'two_way',
} as const;

export type TranslationType = typeof TranslationType[keyof typeof TranslationType];

export const AudioSource = {
  MICROPHONE: 'microphone',
  SYSTEM: 'system',
  BOTH: 'both',
} as const;

export type AudioSource = typeof AudioSource[keyof typeof AudioSource];

export const SubscriptionTier = {
  FREE: 'free',
  PRO: 'pro',
} as const;

export type SubscriptionTier = typeof SubscriptionTier[keyof typeof SubscriptionTier];

export const TtsProvider = {
  WEB_SPEECH: 'webspeech',
  EDGE: 'edge',
  GOOGLE: 'google',
  ELEVENLABS: 'elevenlabs',
} as const;

export type TtsProvider = typeof TtsProvider[keyof typeof TtsProvider];

export const ConnectionStatus = {
  CONNECTING: 'connecting',
  CONNECTED: 'connected',
  DISCONNECTED: 'disconnected',
  ERROR: 'error',
} as const;

export type ConnectionStatus = typeof ConnectionStatus[keyof typeof ConnectionStatus];

// ---------------------------------------------------------------------------
// Helper Functions
// ---------------------------------------------------------------------------

/**
 * Get the summary limit for a given subscription tier
 */
export function getSummaryLimit(tier: SubscriptionTier): number {
  return tier === SubscriptionTier.PRO ? PRO_TIER_SUMMARY_LIMIT : FREE_TIER_SUMMARY_LIMIT;
}

/**
 * Check if a value is within the valid range for opacity
 */
export function isValidOpacity(value: number): boolean {
  return value >= MIN_OPACITY && value <= MAX_OPACITY;
}

/**
 * Check if a value is within the valid range for font size
 */
export function isValidFontSize(value: number): boolean {
  return value >= MIN_FONT_SIZE && value <= MAX_FONT_SIZE;
}

/**
 * Check if a value is within the valid range for max lines
 */
export function isValidMaxLines(value: number): boolean {
  return value >= MIN_MAX_LINES && value <= MAX_MAX_LINES;
}

/**
 * Check if a value is within the valid range for endpoint delay
 */
export function isValidEndpointDelay(value: number): boolean {
  return value >= MIN_ENDPOINT_DELAY && value <= MAX_ENDPOINT_DELAY;
}

/**
 * Check if a value is within the valid range for TTS rate
 */
export function isValidTtsRate(value: number): boolean {
  return value >= MIN_TTS_RATE && value <= MAX_TTS_RATE;
}

/**
 * Clamp a value to the valid opacity range
 */
export function clampOpacity(value: number): number {
  return Math.max(MIN_OPACITY, Math.min(MAX_OPACITY, value));
}

/**
 * Clamp a value to the valid font size range
 */
export function clampFontSize(value: number): number {
  return Math.max(MIN_FONT_SIZE, Math.min(MAX_FONT_SIZE, value));
}

/**
 * Clamp a value to the valid max lines range
 */
export function clampMaxLines(value: number): number {
  return Math.max(MIN_MAX_LINES, Math.min(MAX_MAX_LINES, value));
}

/**
 * Clamp a value to the valid endpoint delay range
 */
export function clampEndpointDelay(value: number): number {
  return Math.max(MIN_ENDPOINT_DELAY, Math.min(MAX_ENDPOINT_DELAY, value));
}

/**
 * Clamp a value to the valid TTS rate range
 */
export function clampTtsRate(value: number): number {
  return Math.max(MIN_TTS_RATE, Math.min(MAX_TTS_RATE, value));
}
