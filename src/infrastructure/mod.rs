//! Infrastructure layer - concrete implementations of domain interfaces
//!
//! This layer contains audio capture, and logging for the dual-mode
//! (cloud Soniox / offline MLX) architecture.

pub mod audio;
pub mod logging;

// Re-export audio types
pub use audio::{AudioCaptureConfig, MicrophoneCapture};

// Re-export logging
pub use logging::{LoggingConfig, init_logging, init_default_logging, init_dev_logging, init_test_logging};
