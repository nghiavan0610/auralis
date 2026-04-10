//! DRM token generation for Microsoft Edge TTS service.
//!
//! Generates the Sec-MS-GEC token required for authentication
//! with the Edge TTS WebSocket endpoint.

use sha2::{Digest, Sha256};

/// Trusted client token from Edge browser.
pub const TRUSTED_CLIENT_TOKEN: &str = "6A5AA1D4EAFF4E9FB37E23D68491D6F4";

/// Windows file time epoch offset (seconds between 1601-01-01 and 1970-01-01).
const WIN_EPOCH: u64 = 11_644_473_600;

/// Generate the Sec-MS-GEC DRM token.
///
/// Based on the current time rounded down to the nearest 5 minutes,
/// converted to Windows file time format (100-ns intervals since 1601-01-01),
/// concatenated with the trusted client token, and SHA256 hashed.
pub fn generate_sec_ms_gec() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("System time before UNIX epoch")
        .as_secs();

    // Convert to Windows file time and round down to nearest 5 minutes
    let ticks = ((now + WIN_EPOCH) / 300) * 300;
    let file_time = ticks as f64 * 1e7; // 100-ns intervals

    let str_to_hash = format!("{:.0}{}", file_time, TRUSTED_CLIENT_TOKEN);

    let mut hasher = Sha256::new();
    hasher.update(str_to_hash.as_bytes());
    format!("{:X}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drm_token_deterministic() {
        // Two calls within the same 5-minute window should produce the same token
        let token1 = generate_sec_ms_gec();
        let token2 = generate_sec_ms_gec();
        assert_eq!(token1, token2);
    }

    #[test]
    fn test_drm_token_format() {
        let token = generate_sec_ms_gec();
        // SHA256 hex digest is 64 characters, uppercase
        assert_eq!(token.len(), 64);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
