//! Structured logging infrastructure for Auralis
//!
//! This module provides a comprehensive logging setup using the tracing ecosystem,
//! supporting both console and file output with configurable log levels.

use tracing::{Level, Subscriber};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer, Registry,
};
use std::io;
use std::path::PathBuf;

/// Default log level for the application
pub const DEFAULT_LOG_LEVEL: Level = Level::INFO;

/// Log directory name
pub const LOG_DIR_NAME: &str = "logs";

/// Log file names
pub const MAIN_LOG_FILE: &str = "auralis.log";
pub const ERROR_LOG_FILE: &str = "errors.log";

/// Configuration for logging infrastructure
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// Minimum log level to display
    pub log_level: Level,

    /// Whether to enable console logging
    pub console: bool,

    /// Whether to enable file logging
    pub file: bool,

    /// Custom directory for log files (defaults to ./logs)
    pub log_dir: Option<PathBuf>,

    /// Whether to include span events (for debugging async operations)
    pub span_events: bool,

    /// Whether to use ANSI colors in console output
    pub ansi: bool,

    /// Whether to include target module in logs
    pub with_target: bool,

    /// Whether to include thread IDs in logs
    pub with_thread_ids: bool,

    /// Custom environment filter (overrides log_level if set)
    pub env_filter: Option<String>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            log_level: DEFAULT_LOG_LEVEL,
            console: true,
            file: true,
            log_dir: None,
            span_events: false,
            ansi: true,
            with_target: true,
            with_thread_ids: false,
            env_filter: None,
        }
    }
}

impl LoggingConfig {
    /// Create a new logging config with the specified log level
    pub fn with_level(mut self, level: Level) -> Self {
        self.log_level = level;
        self
    }

    /// Enable or disable console logging
    pub fn with_console(mut self, console: bool) -> Self {
        self.console = console;
        self
    }

    /// Enable or disable file logging
    pub fn with_file(mut self, file: bool) -> Self {
        self.file = file;
        self
    }

    /// Set a custom log directory
    pub fn with_log_dir(mut self, dir: PathBuf) -> Self {
        self.log_dir = Some(dir);
        self
    }

    /// Enable or disable span events for async debugging
    pub fn with_span_events(mut self, enable: bool) -> Self {
        self.span_events = enable;
        self
    }

    /// Enable or disable ANSI colors
    pub fn with_ansi(mut self, ansi: bool) -> Self {
        self.ansi = ansi;
        self
    }

    /// Enable or disable target module in logs
    pub fn with_target(mut self, with_target: bool) -> Self {
        self.with_target = with_target;
        self
    }

    /// Enable or disable thread IDs in logs
    pub fn with_thread_ids(mut self, with_thread_ids: bool) -> Self {
        self.with_thread_ids = with_thread_ids;
        self
    }

    /// Set a custom environment filter
    pub fn with_env_filter(mut self, filter: String) -> Self {
        self.env_filter = Some(filter);
        self
    }

    /// Initialize the logging system with this configuration
    pub fn init(self) -> Result<(), LoggingError> {
        init_logging(self)
    }
}

/// Errors that can occur during logging initialization
#[derive(Debug, thiserror::Error)]
pub enum LoggingError {
    #[error("Failed to create log directory: {0}")]
    DirectoryError(io::Error),

    #[error("Failed to create log file: {0}")]
    FileError(io::Error),

    #[error("Invalid log level: {0}")]
    InvalidLevel(String),
}

/// Initialize the logging system with the given configuration
///
/// This function sets up a comprehensive logging infrastructure including:
/// - Console logging with optional ANSI colors
/// - File logging (both general and error-specific logs)
/// - Configurable log levels via environment variables
/// - Structured formatting with tracing
///
/// # Arguments
///
/// * `config` - Logging configuration
///
/// # Returns
///
/// Returns `Ok(())` if logging was successfully initialized, or an error if setup failed.
///
/// # Example
///
/// ```no_run
/// use auralis::infrastructure::logging::init_logging;
/// use tracing::Level;
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     init_logging(auralis::infrastructure::logging::LoggingConfig::default()
///         .with_level(Level::DEBUG)
///         .with_console(true)
///         .with_file(true))?;
///
///     // Your application code here
///     Ok(())
/// }
/// ```
pub fn init_logging(config: LoggingConfig) -> Result<(), LoggingError> {
    // Create log directory if file logging is enabled
    let log_dir = if config.file {
        let dir = config.log_dir.clone().unwrap_or_else(|| PathBuf::from(LOG_DIR_NAME));
        std::fs::create_dir_all(&dir).map_err(LoggingError::DirectoryError)?;
        Some(dir)
    } else {
        None
    };

    // Build the environment filter
    let env_filter = if let Some(filter) = config.env_filter {
        EnvFilter::try_new(filter).map_err(|e| LoggingError::InvalidLevel(e.to_string()))?
    } else {
        EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(config.log_level.to_string()))
    };

    // Build the subscriber
    let subscriber = Registry::default().with(env_filter);

    // Add console layer if enabled
    let subscriber = if config.console {
        let console_layer = fmt::layer()
            .with_writer(io::stdout)
            .with_ansi(config.ansi)
            .with_target(config.with_target)
            .with_thread_ids(config.with_thread_ids);

        let console_layer = if config.span_events {
            console_layer.with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        } else {
            console_layer
        };

        subscriber.with(console_layer)
    } else {
        subscriber
    };

    // Add file layers if enabled
    let subscriber = if let Some(ref dir) = log_dir {
        // Main log file (all levels)
        let main_log_path = dir.join(MAIN_LOG_FILE);
        let main_log_file = std::fs::File::create(&main_log_path)
            .map_err(LoggingError::FileError)?;

        let main_layer = fmt::layer()
            .with_writer(main_log_file)
            .with_ansi(false)
            .with_target(config.with_target)
            .with_thread_ids(config.with_thread_ids);

        // Error log file (ERROR and WARN only)
        let error_log_path = dir.join(ERROR_LOG_FILE);
        let error_log_file = std::fs::File::create(&error_log_path)
            .map_err(LoggingError::FileError)?;

        let error_filter = EnvFilter::new("warn")
            .add_directive("auralis=error".parse().unwrap());

        let error_layer = fmt::layer()
            .with_writer(error_log_file)
            .with_ansi(false)
            .with_target(config.with_target)
            .with_thread_ids(config.with_thread_ids)
            .with_filter(error_filter);

        subscriber.with(main_layer).with(error_layer)
    } else {
        subscriber
    };

    // Initialize the global subscriber
    subscriber.init();

    Ok(())
}

/// Initialize logging with default settings
///
/// This is a convenience function that initializes logging with sensible defaults.
/// It enables both console and file logging at INFO level.
///
/// # Example
///
/// ```no_run
/// use auralis::infrastructure::logging::init_default_logging;
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     init_default_logging()?;
///     // Your application code here
///     Ok(())
/// }
/// ```
pub fn init_default_logging() -> Result<(), LoggingError> {
    LoggingConfig::default().init()
}

/// Initialize logging with debug level for development
///
/// This is a convenience function that initializes logging with debug settings,
/// suitable for development and debugging.
pub fn init_dev_logging() -> Result<(), LoggingError> {
    LoggingConfig::default()
        .with_level(Level::DEBUG)
        .with_span_events(true)
        .with_thread_ids(true)
        .init()
}

/// Initialize logging for testing
///
/// Disables file logging and uses a more verbose output format.
pub fn init_test_logging() -> Result<(), LoggingError> {
    LoggingConfig::default()
        .with_level(Level::DEBUG)
        .with_console(true)
        .with_file(false)
        .with_span_events(true)
        .with_thread_ids(true)
        .init()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing::info;

    #[test]
    fn test_logging_config_default() {
        let config = LoggingConfig::default();
        assert_eq!(config.log_level, DEFAULT_LOG_LEVEL);
        assert!(config.console);
        assert!(config.file);
        assert!(config.ansi);
        assert!(config.with_target);
        assert!(!config.with_thread_ids);
        assert!(!config.span_events);
    }

    #[test]
    fn test_logging_config_builders() {
        let config = LoggingConfig::default()
            .with_level(Level::DEBUG)
            .with_console(false)
            .with_file(false)
            .with_ansi(false)
            .with_target(false)
            .with_thread_ids(true)
            .with_span_events(true);

        assert_eq!(config.log_level, Level::DEBUG);
        assert!(!config.console);
        assert!(!config.file);
        assert!(!config.ansi);
        assert!(!config.with_target);
        assert!(config.with_thread_ids);
        assert!(config.span_events);
    }

    #[test]
    fn test_init_test_logging() {
        // This test just verifies that init_test_logging doesn't panic
        // In a real test environment, we might want to capture the output
        let result = init_test_logging();
        assert!(result.is_ok());

        // Log a test message
        info!("Test logging initialized successfully");
    }

    #[test]
    fn test_init_default_logging() {
        let result = init_default_logging();
        assert!(result.is_ok());

        info!("Default logging initialized successfully");
    }

    #[test]
    fn test_custom_log_dir() {
        let temp_dir = std::env::temp_dir();
        let custom_dir = temp_dir.join("auralis_test_logs");

        let config = LoggingConfig::default()
            .with_log_dir(custom_dir.clone())
            .with_console(false)
            .with_file(true);

        let result = config.init();
        assert!(result.is_ok());

        // Verify the directory was created
        assert!(custom_dir.exists());

        // Clean up
        let _ = std::fs::remove_dir_all(&custom_dir);
    }
}
