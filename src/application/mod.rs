//! Application layer - orchestration and business logic
//!
//! This layer contains the orchestration logic that coordinates all components
//! and manages the event system for communication between components.

pub mod events;
pub mod orchestrator;

pub use events::{AuralisEvent, EventBus};
pub use orchestrator::{Orchestrator, PhraseDetector};
