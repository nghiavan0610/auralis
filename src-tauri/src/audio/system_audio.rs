//! System audio capture using macOS ScreenCaptureKit
//!
//! Captures all system audio output and converts to PCM s16le 16kHz mono.
//! Uses `excludes_current_process_audio(true)` to prevent TTS feedback loops.

use screencapturekit::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;

/// Audio handler that receives CMSampleBuffer callbacks from ScreenCaptureKit
/// and sends PCM data through a channel.
struct AudioHandler {
    sender: mpsc::Sender<Vec<u8>>,
}

impl SCStreamOutputTrait for AudioHandler {
    fn did_output_sample_buffer(&self, sample: CMSampleBuffer, output_type: SCStreamOutputType) {
        match output_type {
            SCStreamOutputType::Audio => {
                if let Some(audio_buffer_list) = sample.audio_buffer_list() {
                    // Take the first audio buffer (mono at 48kHz since we request channel_count=1)
                    let mut iter = audio_buffer_list.into_iter();
                    if let Some(audio_buffer) = iter.next() {
                        let raw_data = audio_buffer.data();

                        if raw_data.is_empty() {
                            return;
                        }

                        // Interpret raw bytes as f32 samples (mono at 48kHz)
                        let f32_samples: &[f32] = unsafe {
                            std::slice::from_raw_parts(
                                raw_data.as_ptr() as *const f32,
                                raw_data.len() / 4,
                            )
                        };

                        // Downsample 48kHz -> 16kHz (take every 3rd sample)
                        let downsampled: Vec<f32> = f32_samples
                            .iter()
                            .step_by(3)
                            .copied()
                            .collect();

                        // Convert f32 [-1.0, 1.0] to i16 PCM s16le
                        let pcm_s16: Vec<u8> = downsampled
                            .iter()
                            .flat_map(|&sample| {
                                let clamped = sample.clamp(-1.0, 1.0);
                                let s16 = (clamped * 32767.0) as i16;
                                s16.to_le_bytes()
                            })
                            .collect();

                        if !pcm_s16.is_empty() {
                            let _ = self.sender.send(pcm_s16);
                        }
                    }
                }
            }
            _ => {
                // Ignore video frames
            }
        }
    }
}

/// System audio capture using ScreenCaptureKit.
///
/// Captures all system audio output and converts to PCM s16le 16kHz mono.
pub struct SystemAudioCapture {
    is_capturing: Arc<AtomicBool>,
}

impl SystemAudioCapture {
    pub fn new() -> Self {
        Self {
            is_capturing: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start capturing system audio.
    /// Returns a receiver that yields PCM s16le 16kHz mono audio chunks.
    pub fn start(&self) -> Result<mpsc::Receiver<Vec<u8>>, String> {
        if self.is_capturing.load(Ordering::SeqCst) {
            return Err("Already capturing system audio".to_string());
        }

        // Get available displays (requires Screen Recording permission)
        let content = SCShareableContent::get().map_err(|e| {
            format!(
                "Failed to get shareable content. Screen Recording permission may be needed: {}",
                e
            )
        })?;

        let display = content
            .displays()
            .into_iter()
            .next()
            .ok_or("No displays found".to_string())?;

        // Create content filter for the main display
        let filter = SCContentFilter::create()
            .with_display(&display)
            .with_excluding_windows(&[])
            .build();

        // Configure: audio only, 48kHz mono
        // Using channel_count(1) avoids stereo interleaving confusion
        // Downsampling to 16kHz happens in AudioHandler
        let config = SCStreamConfiguration::new()
            .with_width(2) // minimal video (required by API)
            .with_height(2)
            .with_captures_audio(true)
            .with_excludes_current_process_audio(true) // Prevent TTS audio feedback loop
            .with_sample_rate(48000)
            .with_channel_count(1);

        // Create channel for audio data
        let (sender, receiver) = mpsc::channel::<Vec<u8>>();

        let handler = AudioHandler { sender };

        // Create and start the stream
        let mut stream = SCStream::new(&filter, &config);
        stream.add_output_handler(handler, SCStreamOutputType::Audio);

        stream
            .start_capture()
            .map_err(|e| format!("Failed to start system audio capture: {}", e))?;

        self.is_capturing.store(true, Ordering::SeqCst);

        // Keep the stream alive in a background thread
        let is_capturing = self.is_capturing.clone();
        std::thread::spawn(move || {
            while is_capturing.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            let _ = stream.stop_capture();
        });

        Ok(receiver)
    }

    /// Stop capturing system audio.
    #[allow(dead_code)]
    pub fn stop(&self) {
        self.is_capturing.store(false, Ordering::SeqCst);
    }

    #[allow(dead_code)]
    pub fn is_capturing(&self) -> bool {
        self.is_capturing.load(Ordering::SeqCst)
    }
}

impl Default for SystemAudioCapture {
    fn default() -> Self {
        Self::new()
    }
}
