// src/main.rs
use auralis::domain::{STTSegment, Translation};

fn main() {
    println!("Auralis - Voice across languages");

    // Demonstrate domain types
    let segment = STTSegment::new("Hello world".to_string(), 0.95, 0, 1000, true);
    println!("Created STT segment: {}", segment.text);

    let translation = Translation::new(
        "en".to_string(),
        "vi".to_string(),
        "Hello".to_string(),
        "Xin chào".to_string(),
        0.95,
    );
    println!("Created translation: {} -> {}", translation.original_text, translation.translated_text);

    // Test that new dependencies are available (compile-time check)
    let _audio_test: Option<cpal::Device> = None;
    let _python_test: Option<pyo3::Python> = None;
    let _whisper_test: Option<whisper_rs::WhisperContext> = None;

    println!("All dependencies loaded successfully!");
}
