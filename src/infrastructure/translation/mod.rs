//! Translation engine implementations
//!
//! This module provides translation engine implementations:
//! - PyO3 bridge to Python translation libraries
//! - Support for multiple translation backends

pub mod madlad;

pub use madlad::{MadladConfig, MadladTranslator};
