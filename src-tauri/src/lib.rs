// Library exports for testing and external usage

pub mod state;
pub mod commands;
pub mod commands_audio;
pub mod commands_settings;
pub mod commands_pipeline;

pub use state::{AuralisState, Settings};
pub use commands::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet() {
        let result = greet("World");
        assert_eq!(result, "Hello, World! You've been greeted from Rust!");
    }

    #[test]
    fn test_greet_empty() {
        let result = greet("");
        assert_eq!(result, "Hello, ! You've been greeted from Rust!");
    }

    #[test]
    fn test_greet_special_chars() {
        let result = greet("Test-123");
        assert_eq!(result, "Hello, Test-123! You've been greeted from Rust!");
    }
}
