# Model Download Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace simulated model downloads with real streaming downloads from public URLs, with progress events sent to the Tauri frontend.

**Architecture:** Add `reqwest` to the Tauri app crate for HTTP streaming downloads. A new `download_model` Tauri command spawns a tokio task that streams bytes to disk and emits progress events via Tauri's `AppHandle.emit()`. The existing `ModelDownloader.svelte` component listens for these events and updates the UI in real-time.

**Tech Stack:** `reqwest` (streaming HTTP), `tokio::fs` (async file I/O), Tauri events (frontend progress), `sha2` (optional checksum verification)

---

### Task 1: Add reqwest dependency to src-tauri

**Files:**
- Modify: `src-tauri/Cargo.toml:14-21` (add reqwest to dependencies)

**Step 1: Add reqwest with streaming support**

In `src-tauri/Cargo.toml`, add after the `url` line:

```toml
reqwest = { version = "0.12", features = ["stream"] }
```

**Step 2: Build to verify dependency resolves**

Run: `cargo build -p auralis-app`
Expected: Compiles (may take a while fetching reqwest crates)

**Step 3: Commit**

```bash
git add src-tauri/Cargo.toml
git commit -m "chore: add reqwest for model downloads"
```

---

### Task 2: Create model download registry

**Files:**
- Create: `src-tauri/src/model_downloader.rs`

This module contains the model URL registry and the streaming download logic.

**Step 1: Write the model_downloader module**

```rust
//! Model download logic with streaming progress
//!
//! Downloads ML models from public URLs and reports progress via Tauri events.

use std::path::{Path, PathBuf};
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
    pub status: String, // "downloading", "completed", "error"
    pub error: Option<String>,
}

/// Registry of downloadable models
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
            "madlad" => Ok((
                "https://huggingface.co/google/madlad400-3b-mt/resolve/main/model.bin",
                "madlad/model.bin",
            )),
            _ => Err(format!("Unknown model type: {}", model_type)),
        }
    }

    /// Get all model types
    pub fn all_models() -> &'static [&'static str] {
        &["whisper", "silero", "madlad"]
    }
}

/// Get the models directory, creating it if needed
pub async fn ensure_models_dir() -> Result<PathBuf, String> {
    let models_dir = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("auralis")
        .join("models");

    tokio::fs::create_dir_all(&models_dir)
        .await
        .map_err(|e| format!("Failed to create models directory: {}", e))?;

    Ok(models_dir)
}

/// Download a model with streaming progress
pub async fn download_model_with_progress(
    model_type: &str,
    app_handle: &AppHandle,
) -> Result<PathBuf, String> {
    let (url, filename) = ModelRegistry::get_model_info(model_type)?;
    let models_dir = ensure_models_dir().await?;

    // For madlad, create the subdirectory
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

    // Start the download
    let response = reqwest::get(url).await
        .map_err(|e| format!("Failed to start download: {}", e))?;

    let total_size = response.content_length().unwrap_or(0);

    // Stream the response body to file
    let mut file = tokio::fs::File::create(&file_path).await
        .map_err(|e| format!("Failed to create file: {}", e))?;

    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut last_reported_percent: u8 = 0;

    use futures::StreamExt;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download error: {}", e))?;
        file.write_all(&chunk).await
            .map_err(|e| format!("Write error: {}", e))?;

        downloaded += chunk.len() as u64;

        // Only emit progress every 1% change to avoid flooding
        let percent = if total_size > 0 {
            ((downloaded as f64 / total_size as f64) * 100.0) as u8
        } else {
            // Unknown total size - report downloaded MB
            ((downloaded as f64 / 1_000_000.0).floor() as u8).min(99)
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
        .map_err(|e| format!("Flush error: {}", e))?;

    // Verify the file was written
    let metadata = tokio::fs::metadata(&file_path).await
        .map_err(|e| format!("Failed to verify file: {}", e))?;

    if metadata.len() == 0 {
        tokio::fs::remove_file(&file_path).await.ok();
        return Err("Downloaded file is empty".to_string());
    }

    // Emit completion
    let _ = app_handle.emit("download-progress", DownloadProgress {
        model: model_type.to_string(),
        downloaded_bytes: downloaded,
        total_bytes: total_size,
        progress_percent: 100,
        status: "completed".to_string(),
        error: None,
    });

    tracing::info!("Downloaded {} to {:?}", model_type, file_path);

    Ok(file_path)
}
```

**Step 2: Build to verify compilation**

Run: `cargo build -p auralis-app`
Expected: Compiles with no errors (warnings OK)

**Step 3: Commit**

```bash
git add src-tauri/src/model_downloader.rs
git commit -m "feat: add model download module with streaming progress"
```

---

### Task 3: Add download_model Tauri command

**Files:**
- Modify: `src-tauri/src/commands.rs` (add new command)
- Modify: `src-tauri/src/main.rs:4,40-53` (register module + command)

**Step 1: Add the module declaration in main.rs**

In `src-tauri/src/main.rs`, add after `mod commands;` (line 6):

```rust
mod model_downloader;
```

**Step 2: Add the download_model command in commands.rs**

Add at the end of commands.rs, before the `#[cfg(test)]` block:

```rust
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
    let (_, filename) = model_downloader::ModelRegistry::get_model_info(&model_type)?;
    let target_path = models_dir.join(filename);

    if target_path.exists() {
        let metadata = std::fs::metadata(&target_path)
            .map_err(|e| format!("Failed to check existing file: {}", e))?;
        if metadata.len() > 0 {
            return Ok(format!("Model {} already downloaded", model_type));
        }
    }

    // Spawn download in background so the command returns immediately
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

    // Download models sequentially to avoid overwhelming the connection
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

    Ok(format!("Downloading {} models", to_download.len()))
}
```

**Step 3: Register the new commands in main.rs**

In `src-tauri/src/main.rs`, add to the `invoke_handler` array (after `check_model_exists`):

```rust
            download_model,
            download_all_models,
```

**Step 4: Build and verify**

Run: `cargo build -p auralis-app`
Expected: Compiles with no errors

**Step 5: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/main.rs
git commit -m "feat: add download_model and download_all_models Tauri commands"
```

---

### Task 4: Update ModelDownloader.svelte to use real downloads

**Files:**
- Modify: `src/components/ModelDownloader.svelte:57-89` (replace fake download with real)

**Step 1: Replace the downloadModel function**

Replace the entire `downloadModel` function (lines 57-89) with:

```typescript
  async function downloadModel(modelName: string) {
    try {
      isDownloading = true;
      downloadProgress = downloadProgress.map((item) =>
        item.model === modelName ? { ...item, status: 'downloading' as const, progress: 0 } : item
      );

      const modelType = modelName.toLowerCase() === 'whisper' ? 'whisper'
        : modelName === 'MADLAD' ? 'madlad'
        : 'silero';

      const { listen } = await import('@tauri-apps/api/event');
      const unlisten = await listen<DownloadProgress>('download-progress', (event) => {
        if (event.payload.model !== modelType) return;

        downloadProgress = downloadProgress.map((item) => {
          if (item.model === modelName) {
            if (event.payload.status === 'error') {
              return { ...item, status: 'error' as const, progress: 0, error: event.payload.error };
            } else if (event.payload.status === 'completed') {
              return { ...item, status: 'completed' as const, progress: 100 };
            } else {
              return { ...item, status: 'downloading' as const, progress: event.payload.progress_percent };
            }
          }
          return item;
        });

        if (event.payload.status === 'completed' || event.payload.status === 'error') {
          unlisten();
          isDownloading = false;
          refreshModelStatus();
        }
      });

      await invoke('download_model', { modelType });
    } catch (error) {
      downloadProgress = downloadProgress.map((item) =>
        item.model === modelName ? { ...item, status: 'error' as const, error: String(error) } : item
      );
      isDownloading = false;
    }
  }
```

**Step 2: Add the DownloadProgress interface**

Add after the `ModelDownloadProgress` interface (after line 20):

```typescript
  interface DownloadProgress {
    model: string;
    downloaded_bytes: number;
    total_bytes: number;
    progress_percent: number;
    status: string;
    error?: string;
  }
```

**Step 3: Update model names to use lowercase for invoke calls**

No changes needed - the mapping is handled inside `downloadModel()`.

**Step 4: Test in browser via tauri:dev**

Run: `npm run tauri:dev`
Expected: Click Download on Silero (smallest, ~2MB) - should show real progress and complete

**Step 5: Commit**

```bash
git add src/components/ModelDownloader.svelte
git commit -m "feat: wire ModelDownloader to real download backend"
```

---

### Task 5: Verify with a real download test

**Files:** None (manual verification)

**Step 1: Start the app**

Run: `npm run tauri:dev`

**Step 2: Test Silero download (smallest, ~2MB)**

1. Click "Download" next to Silero
2. Verify progress bar fills up
3. Verify status changes to "Ready" after completion
4. Check file exists at `~/.cache/auralis/models/silero_vad.jit`

Run: `ls -la ~/.cache/auralis/models/silero_vad.jit`
Expected: File exists with non-zero size

**Step 3: Verify model status updates**

The status should update from "Download required models" to showing Silero as ready after the download completes.

**Step 4: Commit any fixes**

```bash
git add -A
git commit -m "fix: address download verification issues"
```
