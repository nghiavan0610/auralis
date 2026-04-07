pub mod domain;
pub mod infrastructure;
pub mod application;

pub use domain::{
    models::{STTSegment, Translation},
    errors::{AudioError, ConfigError},
};

pub use infrastructure::audio::{AudioCaptureConfig, MicrophoneCapture};
