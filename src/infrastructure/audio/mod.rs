//! Audio capture and processing implementations
//!
//! This module provides concrete implementations of audio sources including:
//! - Microphone capture using cpal
//! - File-based audio sources
//! - Audio stream processing

pub mod capture;

pub use capture::{AudioCaptureConfig, MicrophoneCapture};
