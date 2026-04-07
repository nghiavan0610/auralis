//! Speech-to-Text engine implementations
//!
//! This module provides STT engine implementations:
//! - Whisper.cpp integration via whisper-rs
//! - Streaming STT support

pub mod whisper;

pub use whisper::{WhisperConfig, WhisperEngine};
