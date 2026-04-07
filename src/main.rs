// src/main.rs
use auralis::domain::{STTSegment, Translation};

fn main() {
    println!("Auralis - Voice across languages");

    // Demonstrate domain types
    let segment = STTSegment::new("Hello world".to_string(), 0.95, 0, 1000, true);
    println!("Created STT segment: {}", segment.text);

    let translation = Translation::new(
        "en".to_string(),
        "es".to_string(),
        "Hello".to_string(),
        "Hola".to_string(),
        0.95,
    );
    println!("Created translation: {} -> {}", translation.original_text, translation.translated_text);
}
