//! Model download logic with streaming progress
//!
//! Downloads ML models from public URLs and reports progress via Tauri events.

use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use tauri::{AppHandle, Emitter};
use serde::{Deserialize, Serialize};

/// Model download progress event payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub model: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub progress_percent: u8,
    pub status: String,
    pub error: Option<String>,
}

/// Registry of downloadable models with their URLs and filenames
pub struct ModelRegistry;

impl ModelRegistry {
    /// Get the download URL and expected filename for a model type
    pub fn get_model_info(model_type: &str) -> Result<(&'static str, &'static str), String> {
        match model_type {
            "whisper" => Ok((
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin",
                "ggml-base.en.bin",
            )),
            "silero" => Ok((
                "https://github.com/snakers4/silero-vad/raw/master/src/silero_vad/data/silero_vad.jit",
                "silero_vad.jit",
            )),
            "nllb" => Ok((
                "https://huggingface.co/JustFrederik/nllb-200-distilled-600M-ct2-float16/resolve/main/model.bin",
                "nllb/model.bin",
            )),
            _ => Err(format!("Unknown model type: {}", model_type)),
        }
    }

    /// Get all files that need to be downloaded for a model type
    pub fn get_all_files(model_type: &str) -> Result<Vec<(&'static str, &'static str)>, String> {
        match model_type {
            "whisper" | "silero" => {
                let (url, filename) = Self::get_model_info(model_type)?;
                Ok(vec![(url, filename)])
            }
            "nllb" => Ok(vec![
                ("https://huggingface.co/JustFrederik/nllb-200-distilled-600M-ct2-float16/resolve/main/model.bin", "nllb/model.bin"),
                ("https://huggingface.co/JustFrederik/nllb-200-distilled-600M-ct2-float16/resolve/main/sentencepiece.bpe.model", "nllb/sentencepiece.bpe.model"),
                ("https://huggingface.co/JustFrederik/nllb-200-distilled-600M-ct2-float16/resolve/main/shared_vocabulary.txt", "nllb/shared_vocabulary.txt"),
                ("https://huggingface.co/JustFrederik/nllb-200-distilled-600M-ct2-float16/resolve/main/config.json", "nllb/config.json"),
            ]),
            _ => Err(format!("Unknown model type: {}", model_type)),
        }
    }

    /// Get all auto-downloadable model types
    pub fn all_models() -> &'static [&'static str] {
        &["whisper", "silero", "nllb"]
    }
}

/// Get the models directory, creating it if needed
///
/// Uses the same path as model_path.rs: ~/.cache/auralis/models/
pub async fn ensure_models_dir() -> Result<PathBuf, String> {
    let models_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cache")
        .join("auralis")
        .join("models");

    tokio::fs::create_dir_all(&models_dir)
        .await
        .map_err(|e| format!("Failed to create models directory: {}", e))?;

    Ok(models_dir)
}

/// Download a model with streaming progress events
pub async fn download_model_with_progress(
    model_type: &str,
    app_handle: &AppHandle,
) -> Result<PathBuf, String> {
    let models_dir = ensure_models_dir().await?;
    let files = ModelRegistry::get_all_files(model_type)?;

    // For single-file downloads, use the original simple logic
    if files.len() == 1 {
        return download_single_file(model_type, files[0].0, files[0].1, &models_dir, app_handle).await;
    }

    // Multi-file download (e.g., nllb)
    let total_files = files.len();
    let mut last_path = None;

    for (file_idx, (url, filename)) in files.iter().enumerate() {
        let file_path = models_dir.join(filename);

        // Create parent directories
        if let Some(parent) = file_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }

        // Emit initial status for this file
        let _ = app_handle.emit("download-progress", DownloadProgress {
            model: model_type.to_string(),
            downloaded_bytes: 0,
            total_bytes: 0,
            progress_percent: ((file_idx as u64 * 100) / total_files as u64) as u8,
            status: "downloading".to_string(),
            error: None,
        });

        tracing::info!("Starting download of {} file {}/{}: {}", model_type, file_idx + 1, total_files, url);

        // Start the HTTP request
        let response = reqwest::get(*url).await
            .map_err(|e| format!("Failed to start download: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("HTTP error for {}: {}", filename, response.status()));
        }

        let total_size = response.content_length().unwrap_or(0);

        // Create the output file
        let mut file = tokio::fs::File::create(&file_path).await
            .map_err(|e| format!("Failed to create file {:?}: {}", file_path, e))?;

        // Stream the response body
        let mut stream = response.bytes_stream();
        let mut downloaded: u64 = 0;
        let mut last_reported_percent: u8 = 0;

        use futures::StreamExt;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Download stream error: {}", e))?;
            file.write_all(&chunk).await
                .map_err(|e| format!("File write error: {}", e))?;

            downloaded += chunk.len() as u64;

            // Calculate overall progress across all files
            let file_progress = if total_size > 0 {
                (downloaded as f64 / total_size as f64)
            } else {
                0.5 // Unknown size, assume 50% per file
            };

            let overall_percent = (((file_idx as f64 + file_progress) / total_files as f64) * 100.0) as u8;

            if overall_percent > last_reported_percent {
                last_reported_percent = overall_percent;
                let _ = app_handle.emit("download-progress", DownloadProgress {
                    model: model_type.to_string(),
                    downloaded_bytes: downloaded,
                    total_bytes: total_size,
                    progress_percent: overall_percent,
                    status: "downloading".to_string(),
                    error: None,
                });
            }
        }

        file.flush().await
            .map_err(|e| format!("File flush error: {}", e))?;

        // Verify the file was written
        let metadata = tokio::fs::metadata(&file_path).await
            .map_err(|e| format!("Failed to verify downloaded file: {}", e))?;

        if metadata.len() == 0 {
            let _ = tokio::fs::remove_file(&file_path).await;
            return Err(format!("Downloaded file {} is empty", filename));
        }

        tracing::info!("Successfully downloaded {} ({} bytes) to {:?}", filename, metadata.len(), file_path);
        last_path = Some(file_path);
    }

    // Emit completion
    let _ = app_handle.emit("download-progress", DownloadProgress {
        model: model_type.to_string(),
        downloaded_bytes: 0,
        total_bytes: 0,
        progress_percent: 100,
        status: "completed".to_string(),
        error: None,
    });

    last_path.ok_or_else(|| "No files were downloaded".to_string())
}

/// Download a single file with progress tracking
async fn download_single_file(
    model_type: &str,
    url: &str,
    filename: &str,
    models_dir: &PathBuf,
    app_handle: &AppHandle,
) -> Result<PathBuf, String> {
    // Create parent directories
    let file_path = models_dir.join(filename);
    if let Some(parent) = file_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    // Emit initial status
    let _ = app_handle.emit("download-progress", DownloadProgress {
        model: model_type.to_string(),
        downloaded_bytes: 0,
        total_bytes: 0,
        progress_percent: 0,
        status: "downloading".to_string(),
        error: None,
    });

    tracing::info!("Starting download of {} from {}", model_type, url);

    // Start the HTTP request
    let response = reqwest::get(url).await
        .map_err(|e| format!("Failed to start download: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let total_size = response.content_length().unwrap_or(0);
    tracing::info!("Download size: {} bytes", total_size);

    // Create the output file
    let mut file = tokio::fs::File::create(&file_path).await
        .map_err(|e| format!("Failed to create file {:?}: {}", file_path, e))?;

    // Stream the response body
    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut last_reported_percent: u8 = 0;

    use futures::StreamExt;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download stream error: {}", e))?;
        file.write_all(&chunk).await
            .map_err(|e| format!("File write error: {}", e))?;

        downloaded += chunk.len() as u64;

        // Emit progress every 1% change
        let percent = if total_size > 0 {
            ((downloaded as f64 / total_size as f64) * 100.0) as u8
        } else {
            // Unknown size - use downloaded MB as rough progress (cap at 99%)
            std::cmp::min((downloaded / 1_000_000) as u8, 99)
        };

        if percent > last_reported_percent {
            last_reported_percent = percent;
            let _ = app_handle.emit("download-progress", DownloadProgress {
                model: model_type.to_string(),
                downloaded_bytes: downloaded,
                total_bytes: total_size,
                progress_percent: percent,
                status: "downloading".to_string(),
                error: None,
            });
        }
    }

    file.flush().await
        .map_err(|e| format!("File flush error: {}", e))?;

    // Verify the file was written
    let metadata = tokio::fs::metadata(&file_path).await
        .map_err(|e| format!("Failed to verify downloaded file: {}", e))?;

    if metadata.len() == 0 {
        let _ = tokio::fs::remove_file(&file_path).await;
        return Err("Downloaded file is empty".to_string());
    }

    tracing::info!("Successfully downloaded {} ({} bytes) to {:?}", model_type, metadata.len(), file_path);

    // Emit completion
    let _ = app_handle.emit("download-progress", DownloadProgress {
        model: model_type.to_string(),
        downloaded_bytes: downloaded,
        total_bytes: total_size,
        progress_percent: 100,
        status: "completed".to_string(),
        error: None,
    });

    Ok(file_path)
}
