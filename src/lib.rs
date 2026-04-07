pub mod domain;
pub mod infrastructure;

pub use domain::{
    models::{STTSegment, Translation},
    traits::{AudioSource, STTEngine, Translator, VAD, AudioStream, STTStream},
    errors::{AudioError, STTError, TranslationError, VADError, ConfigError},
};

pub use infrastructure::audio::{AudioCaptureConfig, MicrophoneCapture};