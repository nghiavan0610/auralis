//! Audio capture sources for Auralis
//!
//! Supports multiple audio sources:
//! - Microphone via cpal
//! - System audio via macOS ScreenCaptureKit (macOS only)
//! - Both (merged)

#[cfg(target_os = "macos")]
pub mod system_audio;

#[cfg(not(target_os = "macos"))]
pub mod system_audio_stub;

#[cfg(target_os = "macos")]
pub use system_audio::SystemAudioCapture;

#[cfg(not(target_os = "macos"))]
pub use system_audio_stub::SystemAudioCapture;

/// Convert f32 audio samples to PCM s16le bytes.
pub fn f32_to_pcm_s16le(samples: &[f32]) -> Vec<u8> {
    samples
        .iter()
        .flat_map(|&sample| {
            let clamped = sample.clamp(-1.0, 1.0);
            let s16 = (clamped * 32767.0) as i16;
            s16.to_le_bytes()
        })
        .collect()
}

/// Mix two PCM s16le streams by averaging corresponding i16 samples.
///
/// If the streams have different lengths, the shorter is padded with silence (zero).
/// Returns the mixed PCM s16le bytes.
pub fn mix_pcm_s16le(a: &[u8], b: &[u8]) -> Vec<u8> {
    let a_samples = bytes_to_i16(a);
    let b_samples = bytes_to_i16(b);
    let max_len = a_samples.len().max(b_samples.len());

    let mut mixed = Vec::with_capacity(max_len * 2);
    for i in 0..max_len {
        let sa = if i < a_samples.len() { a_samples[i] as i32 } else { 0 };
        let sb = if i < b_samples.len() { b_samples[i] as i32 } else { 0 };
        // Average and clamp to i16 range
        let avg = ((sa + sb) / 2).clamp(-32768, 32767) as i16;
        mixed.extend_from_slice(&avg.to_le_bytes());
    }
    mixed
}

/// Interpret a byte slice as little-endian i16 samples.
fn bytes_to_i16(bytes: &[u8]) -> Vec<i16> {
    bytes
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect()
}

/// Open system privacy settings to the relevant pane.
pub fn open_privacy_settings(pane: &str) {
    #[cfg(target_os = "macos")]
    {
        let url = match pane {
            "microphone" => "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone",
            "screen" => "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture",
            _ => return,
        };
        let _ = std::process::Command::new("open").arg(url).spawn();
    }

    #[cfg(target_os = "windows")]
    {
        let scheme = match pane {
            "microphone" => "ms-settings:privacy-microphone",
            _ => return,
        };
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", scheme])
            .spawn();
    }
}
