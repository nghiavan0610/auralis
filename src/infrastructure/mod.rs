//! Infrastructure layer - concrete implementations of domain interfaces
//!
//! This layer contains the actual implementations of the traits defined in the domain layer,
//! including audio capture, STT engines, translators, and VAD.

pub mod audio;
pub mod stt;
pub mod translation;
pub mod vad;
pub mod container;
pub mod logging;

pub use container::{AuralisContainer, ContainerConfig, ModelStatus};
pub use logging::{LoggingConfig, init_logging, init_default_logging, init_dev_logging, init_test_logging};
