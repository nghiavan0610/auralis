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
        "Xin chao".to_string(),
        0.95,
    );
    println!("Created translation: {} -> {}", translation.original_text, translation.translated_text);

    // Test that audio dependency is available (compile-time check)
    let _audio_test: Option<cpal::Device> = None;

    println!("All dependencies loaded successfully!");
}
