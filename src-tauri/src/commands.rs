//! Tauri commands for the Auralis application
//!
//! This module provides the command handlers for the frontend to interact with
//! the Rust backend. Old orchestrator-based commands have been removed in favour
//! of the dual-mode architecture (cloud via Soniox / offline via Python sidecar).

use crate::state::ModelStatus;
use tauri::{AppHandle, Emitter};

/// Greet command (basic connectivity test)
#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// Get the current model status by checking the filesystem
#[tauri::command]
pub async fn get_model_status() -> Result<ModelStatus, String> {
    use auralis::infrastructure::model_path;

    let config = model_path::ModelPathConfig::default();
    let resolution = model_path::resolve_model_paths(&config);

    Ok(ModelStatus {
        stt_available: resolution.found_models.whisper,
        stt_model: "Whisper".to_string(),
        translation_available: resolution.found_models.nllb,
        translation_model: "NLLB".to_string(),
        vad_available: resolution.found_models.silero,
        vad_model: "Silero".to_string(),
        system_ready: resolution.has_all_models(),
    })
}

/// Check if a specific model exists at the configured path
#[tauri::command]
pub fn check_model_exists(model_type: String) -> Result<bool, String> {
    use auralis::infrastructure::model_path;

    let config = model_path::ModelPathConfig::default();
    let resolution = model_path::resolve_model_paths(&config);

    let exists = match model_type.as_str() {
        "whisper" | "stt" => resolution.found_models.whisper,
        "nllb" | "madlad" | "translation" => resolution.found_models.nllb,
        "silero" | "vad" => resolution.found_models.silero,
        _ => return Err(format!("Unknown model type: {}", model_type)),
    };

    Ok(exists)
}

/// Download a specific model
#[tauri::command]
pub async fn download_model(
    app_handle: AppHandle,
    model_type: String,
) -> Result<String, String> {
    use crate::model_downloader;

    // Validate model type
    let valid_types = model_downloader::ModelRegistry::all_models();
    if !valid_types.contains(&model_type.as_str()) {
        return Err(format!(
            "Unknown model type: {}. Valid types: {:?}",
            model_type, valid_types
        ));
    }

    // Check if already downloaded
    let models_dir = model_downloader::ensure_models_dir().await?;

    // For nllb, check if the main model.bin file exists
    let already_exists = if model_type == "nllb" {
        models_dir.join("nllb").join("model.bin").exists()
    } else {
        let (_, filename) = model_downloader::ModelRegistry::get_model_info(&model_type)?;
        let target_path = models_dir.join(filename);
        target_path.exists() && std::fs::metadata(&target_path).map(|m| m.len() > 0).unwrap_or(false)
    };

    if already_exists {
        // Emit completion event so frontend can update UI
        let _ = app_handle.emit("download-progress", model_downloader::DownloadProgress {
            model: model_type.clone(),
            downloaded_bytes: 0,
            total_bytes: 0,
            progress_percent: 100,
            status: "completed".to_string(),
            error: None,
        });
        return Ok(format!("Model {} already downloaded", model_type));
    }

    // Spawn download in background
    let model_type_clone = model_type.clone();
    tokio::spawn(async move {
        match model_downloader::download_model_with_progress(&model_type_clone, &app_handle).await {
            Ok(path) => {
                tracing::info!("Model {} downloaded to {:?}", model_type_clone, path);
            }
            Err(e) => {
                tracing::error!("Failed to download model {}: {}", model_type_clone, e);
                let _ = app_handle.emit("download-progress", model_downloader::DownloadProgress {
                    model: model_type_clone.clone(),
                    downloaded_bytes: 0,
                    total_bytes: 0,
                    progress_percent: 0,
                    status: "error".to_string(),
                    error: Some(e),
                });
            }
        }
    });

    Ok(format!("Download started for {}", model_type))
}

/// Download all models that aren't already downloaded
#[tauri::command]
pub async fn download_all_models(
    app_handle: AppHandle,
) -> Result<String, String> {
    use crate::model_downloader;

    let models_dir = model_downloader::ensure_models_dir().await?;
    let mut to_download = Vec::new();

    for model_type in model_downloader::ModelRegistry::all_models() {
        let (_, filename) = model_downloader::ModelRegistry::get_model_info(model_type)?;
        let target_path = models_dir.join(filename);

        let needs_download = !target_path.exists() ||
            std::fs::metadata(&target_path).map(|m| m.len() == 0).unwrap_or(true);

        if needs_download {
            to_download.push(model_type.to_string());
        }
    }

    if to_download.is_empty() {
        return Ok("All models already downloaded".to_string());
    }

    let count = to_download.len();
    let app = app_handle.clone();
    tokio::spawn(async move {
        for model_type in &to_download {
            match model_downloader::download_model_with_progress(model_type, &app).await {
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("Failed to download {}: {}", model_type, e);
                    let _ = app.emit("download-progress", model_downloader::DownloadProgress {
                        model: model_type.clone(),
                        downloaded_bytes: 0,
                        total_bytes: 0,
                        progress_percent: 0,
                        status: "error".to_string(),
                        error: Some(e),
                    });
                }
            }
        }
    });

    Ok(format!("Downloading {} models", count))
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
