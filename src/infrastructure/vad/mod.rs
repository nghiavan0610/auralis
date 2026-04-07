//! Voice Activity Detection implementations
//!
//! This module provides VAD implementations:
//! - Silero VAD model
//! - WebRTC VAD
//! - Energy-based VAD

pub mod silero;

pub use silero::{SileroConfig, SileroVAD};
