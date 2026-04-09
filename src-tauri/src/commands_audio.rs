//! Audio streaming Tauri commands
//!
//! Provides start/stop commands for real-time audio capture and streaming
//! to the frontend via Tauri events. Supports three audio sources:
//! - Microphone (cpal)
//! - System audio (ScreenCaptureKit on macOS)
//! - Both (merged)

use crate::audio::{f32_to_pcm_s16le, mix_pcm_s16le, open_privacy_settings, SystemAudioCapture};
use crate::state::AuralisState;
use auralis::domain::traits::AudioSource;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use tauri::{AppHandle, Emitter, State};

/// Start capturing audio from the specified source and streaming PCM data to the frontend.
///
/// The `source` parameter controls which audio source to use:
/// - "microphone": Capture from default microphone via cpal
/// - "system": Capture system audio via ScreenCaptureKit (macOS only)
/// - "both": Capture from both sources simultaneously
#[tauri::command]
pub async fn start_audio_capture(
    state: State<'_, AuralisState>,
    app_handle: AppHandle,
    source: Option<String>,
) -> Result<String, String> {
    let source = source.unwrap_or_else(|| "microphone".to_string());

    // Check if already streaming
    if state.is_streaming.load(Ordering::Relaxed) {
        return Err("Audio capture is already running".to_string());
    }

    match source.as_str() {
        "microphone" => start_mic_capture(state, app_handle).await,
        "system" => start_system_capture(state, app_handle).await,
        "both" => start_both_capture(state, app_handle).await,
        _ => Err(format!("Unknown audio source: {}", source)),
    }
}

/// Start microphone-only capture (original behavior)
async fn start_mic_capture(
    state: State<'_, AuralisState>,
    app_handle: AppHandle,
) -> Result<String, String> {
    let audio_config = auralis::infrastructure::AudioCaptureConfig::default();
    let mut capture = auralis::infrastructure::MicrophoneCapture::new(audio_config)
        .map_err(|e| format!("Failed to create audio capture: {}", e))?;

    capture
        .start()
        .await
        .map_err(|e| {
            open_privacy_settings("microphone");
            format!("Audio capture failed: {}. Opening Microphone settings...", e)
        })?;

    let audio_data = capture.audio_data();
    let is_recording = capture.is_recording_flag();
    let stream_stop = state.stream_stop.clone();

    state.is_streaming.store(true, Ordering::Relaxed);

    let _ = app_handle.emit(
        "audio-capture",
        serde_json::json!({ "is_capturing": true }),
    );

    let app = app_handle.clone();
    let is_streaming = state.is_streaming.clone();

    tokio::spawn(async move {
        let _capture = capture;

        while !stream_stop.load(Ordering::Relaxed) {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;

            let recording = is_recording.lock().unwrap_or_else(|e| e.into_inner());
            if !*recording {
                continue;
            }

            let pcm_bytes = {
                let mut data = audio_data.lock().unwrap_or_else(|e| e.into_inner());
                let chunks: Vec<Vec<f32>> = data.drain(..).collect();
                let mut all_samples = Vec::new();
                for chunk in chunks {
                    all_samples.extend(chunk);
                }
                f32_to_pcm_s16le(&all_samples)
            };

            if !pcm_bytes.is_empty() {
                let _ = app.emit("audio-data", pcm_bytes);
            }
        }

        is_streaming.store(false, Ordering::Relaxed);
        let _ = app.emit(
            "audio-capture",
            serde_json::json!({ "is_capturing": false }),
        );
    });

    Ok("Audio capture started".to_string())
}

/// Start system audio capture via ScreenCaptureKit
async fn start_system_capture(
    state: State<'_, AuralisState>,
    app_handle: AppHandle,
) -> Result<String, String> {
    let sys_capture = SystemAudioCapture::new();
    let receiver = sys_capture.start().map_err(|e| {
        open_privacy_settings("screen");
        format!("System audio capture failed: {}. Opening Screen Recording settings...", e)
    })?;

    state.is_streaming.store(true, Ordering::Relaxed);

    let _ = app_handle.emit(
        "audio-capture",
        serde_json::json!({ "is_capturing": true }),
    );

    let app = app_handle.clone();
    let is_streaming = state.is_streaming.clone();
    let stream_stop = state.stream_stop.clone();

    tokio::spawn(async move {
        // Keep system capture alive
        let _sys = sys_capture;

        while !stream_stop.load(Ordering::Relaxed) {
            match receiver.recv_timeout(std::time::Duration::from_millis(200)) {
                Ok(data) => {
                    let data: Vec<u8> = data;
                    if !data.is_empty() {
                        let _ = app.emit("audio-data", data);
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }

        is_streaming.store(false, Ordering::Relaxed);
        let _ = app.emit(
            "audio-capture",
            serde_json::json!({ "is_capturing": false }),
        );
    });

    Ok("System audio capture started".to_string())
}

/// Start both microphone and system audio capture
async fn start_both_capture(
    state: State<'_, AuralisState>,
    app_handle: AppHandle,
) -> Result<String, String> {
    // Start microphone
    let audio_config = auralis::infrastructure::AudioCaptureConfig::default();
    let mut mic_capture = auralis::infrastructure::MicrophoneCapture::new(audio_config)
        .map_err(|e| format!("Failed to create mic capture: {}", e))?;
    mic_capture
        .start()
        .await
        .map_err(|e| {
            open_privacy_settings("microphone");
            format!("Mic capture failed: {}. Opening Microphone settings...", e)
        })?;

    let mic_data = mic_capture.audio_data();
    let mic_recording = mic_capture.is_recording_flag();

    // Start system audio (optional — fall back to mic-only if unavailable)
    let sys_capture = SystemAudioCapture::new();
    let sys_result = sys_capture.start();
    let sys_has_audio = sys_result.is_ok();
    let sys_receiver = sys_result.unwrap_or_else(|_| {
        let (_, rx) = std::sync::mpsc::channel::<Vec<u8>>();
        rx
    });

    state.is_streaming.store(true, Ordering::Relaxed);

    let _ = app_handle.emit(
        "audio-capture",
        serde_json::json!({ "is_capturing": true }),
    );

    let app = app_handle.clone();
    let is_streaming = state.is_streaming.clone();
    let stream_stop = state.stream_stop.clone();

    tokio::spawn(async move {
        let _mic = mic_capture;
        let _keep_sys_alive = if sys_has_audio { Some(sys_capture) } else { None };

        while !stream_stop.load(Ordering::Relaxed) {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;

            // Collect mic PCM
            let mic_pcm: Vec<u8> = {
                let recording = mic_recording.lock().unwrap_or_else(|e| e.into_inner());
                if *recording {
                    let mut data = mic_data.lock().unwrap_or_else(|e| e.into_inner());
                    let chunks: Vec<Vec<f32>> = data.drain(..).collect();
                    let mut all_samples = Vec::new();
                    for chunk in chunks {
                        all_samples.extend(chunk);
                    }
                    f32_to_pcm_s16le(&all_samples)
                } else {
                    Vec::new()
                }
            };

            // Collect system PCM
            let mut sys_pcm = Vec::new();
            while let Ok(data) = sys_receiver.try_recv() {
                sys_pcm.extend_from_slice(&data);
            }

            // Mix or pass through based on available sources
            let output = if !mic_pcm.is_empty() && !sys_pcm.is_empty() {
                mix_pcm_s16le(&mic_pcm, &sys_pcm)
            } else if !mic_pcm.is_empty() {
                mic_pcm
            } else if !sys_pcm.is_empty() {
                sys_pcm
            } else {
                Vec::new()
            };

            if !output.is_empty() {
                let _ = app.emit("audio-data", output);
            }
        }

        is_streaming.store(false, Ordering::Relaxed);
        let _ = app.emit(
            "audio-capture",
            serde_json::json!({ "is_capturing": false }),
        );
    });

    Ok("Audio capture started (both sources)".to_string())
}

/// Stop the current audio capture stream.
#[tauri::command]
pub async fn stop_audio_capture(
    state: State<'_, AuralisState>,
) -> Result<String, String> {
    if !state.is_streaming.load(Ordering::Relaxed) {
        return Err("Audio capture is not running".to_string());
    }

    state.stream_stop.store(true, Ordering::Relaxed);
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    state.is_streaming.store(false, Ordering::Relaxed);
    state.stream_stop.store(false, Ordering::Relaxed);

    Ok("Audio capture stopped".to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_pcm_conversion_values() {
        let sample: f32 = 0.5;
        let clamped = sample.clamp(-1.0, 1.0);
        let s16 = (clamped * 32767.0) as i16;
        assert_eq!(s16, 16383);

        let sample: f32 = -0.5;
        let clamped = sample.clamp(-1.0, 1.0);
        let s16 = (clamped * 32767.0) as i16;
        assert_eq!(s16, -16383);

        let sample: f32 = 2.0;
        let clamped = sample.clamp(-1.0, 1.0);
        let s16 = (clamped * 32767.0) as i16;
        assert_eq!(s16, 32767);

        let sample: f32 = 0.0;
        let clamped = sample.clamp(-1.0, 1.0);
        let s16 = (clamped * 32767.0) as i16;
        assert_eq!(s16, 0);
    }
}
