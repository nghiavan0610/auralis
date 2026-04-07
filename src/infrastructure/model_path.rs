//! Model path resolution and management
//!
//! This module handles finding and validating ML models across multiple possible locations,
//! with support for user configuration, environment variables, and system-wide locations.

use std::path::{Path, PathBuf};
use std::env;
use std::fs;
use serde::{Deserialize, Serialize};
use dirs as dirs_sys;

/// Model path resolution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPathConfig {
    /// Custom user-configured path (highest priority)
    pub custom_path: Option<String>,

    /// Whether to use default location as fallback
    pub use_default_fallback: bool,
}

impl Default for ModelPathConfig {
    fn default() -> Self {
        Self {
            custom_path: None,
            use_default_fallback: true,
        }
    }
}

/// Resolution strategy for finding models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionResult {
    /// The chosen model directory
    pub models_dir: PathBuf,

    /// Paths where models were found
    pub found_models: FoundModels,

    /// Which location was used
    pub location_source: LocationSource,

    /// Any warnings or issues
    pub warnings: Vec<String>,
}

/// Which model types exist in a location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoundModels {
    pub whisper: bool,
    pub nllb: bool,
    pub silero: bool,
    pub whisper_path: Option<PathBuf>,
    pub nllb_path: Option<PathBuf>,
    pub silero_path: Option<PathBuf>,
}

/// Where the models were found
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LocationSource {
    /// User-configured custom path
    Custom,

    /// Environment variable
    EnvironmentVariable(String),

    /// Standard cache location
    DefaultCache,

    /// System-wide location
    SystemWide,

    /// Project-relative (dev mode)
    ProjectRelative,
}

impl ResolutionResult {
    /// Check if all required models are available
    pub fn has_all_models(&self) -> bool {
        self.found_models.whisper && self.found_models.nllb && self.found_models.silero
    }

    /// Get a human-readable status message
    pub fn status_message(&self) -> String {
        if self.has_all_models() {
            format!("✅ All models found in: {}", self.models_dir.display())
        } else {
            let missing = vec![
                if !self.found_models.whisper { "Whisper" } else { "" },
                if !self.found_models.nllb { "NLLB" } else { "" },
                if !self.found_models.silero { "Silero VAD" } else { "" },
            ].into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(", ");

            format!("⚠️ Missing models: {} in {}", missing, self.models_dir.display())
        }
    }
}

/// Resolve model paths with priority ordering
pub fn resolve_model_paths(config: &ModelPathConfig) -> ResolutionResult {
    let mut warnings = Vec::new();

    // Build the priority order for finding models
    let mut resolution_order: Vec<Option<(PathBuf, LocationSource)>> = vec![
        // 1. User-configured path (highest priority)
        config.custom_path.as_ref().map(|p| (PathBuf::from(p), LocationSource::Custom)),

        // 2. Environment variable
        env::var("AURALIS_MODEL_PATH").ok()
            .map(|p| (PathBuf::from(p), LocationSource::EnvironmentVariable("AURALIS_MODEL_PATH".to_string()))),

        // 3. Standard cache location
        dirs_sys::home_dir()
            .map(|h: PathBuf| (h.join(".cache").join("auralis").join("models"), LocationSource::DefaultCache)),
    ];

    // 4. System-wide locations (platform-specific)
    #[cfg(unix)]
    {
        resolution_order.push(Some((PathBuf::from("/usr/local/share/auralis/models"), LocationSource::SystemWide)));
    }

    // 5. Project-relative (dev mode)
    if config.use_default_fallback {
        resolution_order.push(Some((PathBuf::from("models"), LocationSource::ProjectRelative)));
    } else {
        resolution_order.push(None);
    }

    // Try each location in priority order
    for (path, source) in resolution_order.into_iter().flatten() {
        if let Ok(found) = check_location(&path) {
            // At least one model found, use this location
            if found.whisper || found.nllb || found.silero {
                return ResolutionResult {
                    models_dir: path,
                    found_models: found,
                    location_source: source,
                    warnings,
                };
            }
        }
    }

    // No models found anywhere
    warnings.push("No models found in any location".to_string());

    ResolutionResult {
        models_dir: dirs_sys::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".cache")
            .join("auralis")
            .join("models"),
        found_models: FoundModels {
            whisper: false,
            nllb: false,
            silero: false,
            whisper_path: None,
            nllb_path: None,
            silero_path: None,
        },
        location_source: LocationSource::DefaultCache,
        warnings,
    }
}

/// Check what models exist in a given location (public for use by commands)
pub fn check_location(path: &Path) -> Result<FoundModels, String> {
    if !path.exists() {
        return Err(format!("Path does not exist: {}", path.display()));
    }

    let mut found = FoundModels {
        whisper: false,
        nllb: false,
        silero: false,
        whisper_path: None,
        nllb_path: None,
        silero_path: None,
    };

    // Check for Whisper model
    let whisper_names = vec![
        "ggml-base.en.bin",
        "ggml-base.bin",
        "ggml-small.bin",
        "ggml-medium.bin",
        "whisper.bin",
    ];

    for name in &whisper_names {
        let model_path = path.join("whisper").join(name);
        if model_path.exists() {
            found.whisper = true;
            found.whisper_path = Some(model_path);
            break;
        }
    }

    // Also check root of models dir for whisper
    if !found.whisper {
        for name in &whisper_names {
            let model_path = path.join(name);
            if model_path.exists() {
                found.whisper = true;
                found.whisper_path = Some(model_path);
                break;
            }
        }
    }

    // Check for NLLB model
    let nllb_indicators = vec![
        "model.bin", // CTranslate2 model file
        "config.json",
        "sentencepiece.bpe.model",
    ];

    let nllb_dir = path.join("nllb");
    if nllb_dir.exists() {
        // Check if it's a valid NLLB directory
        let has_indicator = nllb_indicators.iter()
            .any(|name| nllb_dir.join(name).exists());

        if has_indicator {
            found.nllb = true;
            found.nllb_path = Some(nllb_dir);
        }
    }

    // Check for Silero VAD model
    let silero_names = vec![
        "silero_vad.jit",
        "silero_vad.torch",
        "silero_vad",
    ];

    for name in &silero_names {
        let model_path = path.join("silero").join(name);
        if model_path.exists() {
            found.silero = true;
            found.silero_path = Some(model_path);
            break;
        }
    }

    // Also check root for silero
    if !found.silero {
        for name in &silero_names {
            let model_path = path.join(name);
            if model_path.exists() {
                found.silero = true;
                found.silero_path = Some(model_path);
                break;
            }
        }
    }

    Ok(found)
}

/// Verify that a model file is valid (basic check)
pub fn verify_model_file(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("File does not exist: {}", path.display()));
    }

    // Check file size
    let metadata = fs::metadata(path)
        .map_err(|e| format!("Cannot read file metadata: {}", e))?;

    let size = metadata.len();

    // Basic validation based on expected model sizes
    if path.to_string_lossy().contains("whisper") {
        if size < 10_000_000 { // Less than 10MB is too small for Whisper
            return Err(format!("Whisper model too small: {} bytes", size));
        }
    } else if path.to_string_lossy().contains("silero") {
        if size < 100_000 { // Less than 100KB is too small for Silero
            return Err(format!("Silero model too small: {} bytes", size));
        }
    } else if path.to_string_lossy().contains("nllb") || path.to_string_lossy().contains("model.bin") {
        if size < 1_000_000 { // Less than 1MB is too small for translation model
            return Err(format!("Translation model too small: {} bytes", size));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolution_result_creation() {
        let result = ResolutionResult {
            models_dir: PathBuf::from("/test/models"),
            found_models: FoundModels {
                whisper: true,
                nllb: false,
                silero: true,
                whisper_path: Some(PathBuf::from("/test/whisper.bin")),
                nllb_path: None,
                silero_path: Some(PathBuf::from("/test/silero.jit")),
            },
            location_source: LocationSource::Custom,
            warnings: vec![],
        };

        assert!(!result.has_all_models()); // Missing NLLB
        assert!(result.status_message().contains("Missing models: NLLB"));
    }

    #[test]
    fn test_default_config_resolution() {
        // Test that default config doesn't crash and produces a result
        let config = ModelPathConfig::default();
        let resolution = resolve_model_paths(&config);

        // Should always return a valid result, even if no models found
        assert!(!resolution.models_dir.as_os_str().is_empty());
    }

    #[test]
    fn test_custom_path_priority() {
        // Test that custom path is used when set
        let config = ModelPathConfig {
            custom_path: Some("/custom/models".to_string()),
            use_default_fallback: false,
        };

        let resolution = resolve_model_paths(&config);

        // Custom path should be the source if it exists
        // (Note: this test doesn't create the directory, so it will fall through)
    }
}
