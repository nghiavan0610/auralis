//! Audio streaming Tauri commands
//!
//! Provides start/stop commands for real-time audio capture and streaming
//! to the frontend via Tauri events. Audio data is captured from the microphone,
//! converted to s16le PCM bytes, and emitted as "audio-data" events.

use crate::state::AuralisState;
use auralis::domain::traits::AudioSource;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, State};

/// Start capturing audio from the microphone and streaming PCM data to the frontend.
///
/// Creates a new MicrophoneCapture, starts recording, and spawns a background
/// task that periodically reads captured audio chunks, converts them to s16le
/// PCM bytes, and emits them via the "audio-data" Tauri event.
#[tauri::command]
pub async fn start_audio_capture(
    state: State<'_, AuralisState>,
    app_handle: AppHandle,
) -> Result<String, String> {
    // Check if already streaming
    if state.is_streaming.load(Ordering::Relaxed) {
        return Err("Audio capture is already running".to_string());
    }

    // Create audio capture with default config (16kHz mono)
    let audio_config = auralis::infrastructure::AudioCaptureConfig::default();
    let mut capture = auralis::infrastructure::MicrophoneCapture::new(audio_config)
        .map_err(|e| format!("Failed to create audio capture: {}", e))?;

    // Start the microphone capture
    capture
        .start()
        .await
        .map_err(|e| format!("Failed to start audio capture: {}", e))?;

    // Clone shared references from the capture instance
    let audio_data = capture.audio_data();
    let is_recording = capture.is_recording_flag();
    let stream_stop = capture.stream_stop_flag();

    // Signal that streaming is active
    state.is_streaming.store(true, Ordering::Relaxed);

    // Emit capture started event
    let _ = app_handle.emit(
        "audio-capture",
        serde_json::json!({ "is_capturing": true }),
    );

    // Spawn background streaming task
    let app = app_handle.clone();
    let is_streaming = state.is_streaming.clone();

    tokio::spawn(async move {
        // Keep capture alive for the duration of the stream.
        // When this task ends, capture is dropped and the audio stream stops.
        let _capture = capture;

        while !stream_stop.load(Ordering::Relaxed) {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;

            // Check if we should still be recording
            let recording = is_recording.lock().unwrap_or_else(|e| e.into_inner());
            if !*recording {
                continue;
            }

            // Drain audio chunks and convert to PCM bytes
            let pcm_bytes = {
                let mut data = audio_data.lock().unwrap_or_else(|e| e.into_inner());
                let mut pcm = Vec::new();
                for chunk in data.drain(..) {
                    for sample in chunk {
                        // Convert f32 to s16le: clamp to [-1.0, 1.0] then scale
                        let clamped = sample.clamp(-1.0, 1.0);
                        let s16 = (clamped * 32767.0) as i16;
                        pcm.push(s16.to_le_bytes());
                    }
                }
                pcm.concat()
            };

            if !pcm_bytes.is_empty() {
                let _ = app.emit("audio-data", pcm_bytes);
            }
        }

        // Signal streaming stopped
        is_streaming.store(false, Ordering::Relaxed);

        let _ = app.emit(
            "audio-capture",
            serde_json::json!({ "is_capturing": false }),
        );
    });

    Ok("Audio capture started".to_string())
}

/// Stop the current audio capture stream.
///
/// Signals the streaming task to stop and waits for it to clean up.
#[tauri::command]
pub async fn stop_audio_capture(
    state: State<'_, AuralisState>,
) -> Result<String, String> {
    // Check if currently streaming
    if !state.is_streaming.load(Ordering::Relaxed) {
        return Err("Audio capture is not running".to_string());
    }

    // Signal the stream to stop
    state.stream_stop.store(true, Ordering::Relaxed);

    // Wait briefly for the streaming task to clean up
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    // Ensure the streaming flag is cleared
    state.is_streaming.store(false, Ordering::Relaxed);

    Ok("Audio capture stopped".to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_pcm_conversion_values() {
        // Verify f32 to s16le conversion math
        let sample: f32 = 0.5;
        let clamped = sample.clamp(-1.0, 1.0);
        let s16 = (clamped * 32767.0) as i16;
        assert_eq!(s16, 16383);

        let sample: f32 = -0.5;
        let clamped = sample.clamp(-1.0, 1.0);
        let s16 = (clamped * 32767.0) as i16;
        assert_eq!(s16, -16383);

        // Clamping beyond 1.0
        let sample: f32 = 2.0;
        let clamped = sample.clamp(-1.0, 1.0);
        let s16 = (clamped * 32767.0) as i16;
        assert_eq!(s16, 32767);

        // Silence
        let sample: f32 = 0.0;
        let clamped = sample.clamp(-1.0, 1.0);
        let s16 = (clamped * 32767.0) as i16;
        assert_eq!(s16, 0);
    }
}
