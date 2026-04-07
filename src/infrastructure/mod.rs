//! Infrastructure layer - concrete implementations of domain interfaces
//!
//! This layer contains the actual implementations of the traits defined in the domain layer,
//! including audio capture, logging, and model path management.

pub mod audio;
pub mod logging;
pub mod model_path;

// Re-export config types
pub use audio::AudioCaptureConfig;

// Re-export component types
pub use audio::MicrophoneCapture;

// Re-export logging
pub use logging::{LoggingConfig, init_logging, init_default_logging, init_dev_logging, init_test_logging};

// Re-export model path management
pub use model_path::{ModelPathConfig, ResolutionResult, FoundModels, LocationSource, resolve_model_paths, verify_model_file};
