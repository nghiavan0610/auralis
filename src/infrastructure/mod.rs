//! Infrastructure layer - concrete implementations of domain interfaces
//!
//! This layer contains the actual implementations of the traits defined in the domain layer,
//! including audio capture, STT engines, translators, and VAD.

pub mod audio;
pub mod stt;
pub mod translation;
pub mod vad;
pub mod container;

pub use container::{AuralisContainer, ContainerConfig, ModelStatus};
