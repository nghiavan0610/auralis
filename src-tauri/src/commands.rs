//! Tauri commands for the Auralis application
//!
//! Core commands for the dual-mode architecture (cloud via Soniox / offline via Python sidecar).
//! Audio, settings, and pipeline commands are in their respective modules.

/// Greet command (basic connectivity test)
#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_greet() {
        let result = super::greet("World");
        assert_eq!(result, "Hello, World! You've been greeted from Rust!");
    }

    #[test]
    fn test_greet_empty() {
        let result = super::greet("");
        assert_eq!(result, "Hello, ! You've been greeted from Rust!");
    }
}
