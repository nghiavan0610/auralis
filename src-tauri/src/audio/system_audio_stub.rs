//! Stub system audio capture for non-macOS platforms
//!
//! Provides the same type signature as the macOS implementation so call sites
//! compile without changes. `start()` always returns an error on unsupported
//! platforms.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;

/// Stub system audio capture (non-macOS).
///
/// All methods are no-ops. `start()` returns an error indicating the feature
/// is not available on the current platform.
pub struct SystemAudioCapture {
    is_capturing: Arc<AtomicBool>,
}

impl SystemAudioCapture {
    pub fn new() -> Self {
        Self {
            is_capturing: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Always returns an error on non-macOS platforms.
    pub fn start(&self) -> Result<mpsc::Receiver<Vec<u8>>, String> {
        Err("System audio capture is not supported on this platform. Use microphone instead.".to_string())
    }

    #[allow(dead_code)]
    pub fn stop(&self) {
        self.is_capturing.store(false, Ordering::SeqCst);
    }

    #[allow(dead_code)]
    pub fn is_capturing(&self) -> bool {
        self.is_capturing.load(Ordering::SeqCst)
    }
}

impl Default for SystemAudioCapture {
    fn default() -> Self {
        Self::new()
    }
}
