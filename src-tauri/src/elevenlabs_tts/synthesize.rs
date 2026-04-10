//! ElevenLabs TTS synthesis via REST API.
//!
//! Sends text + voice_id to ElevenLabs API and returns MP3 audio bytes.

use serde_json::json;

use super::voices::default_voice_for_lang;

/// Synthesize text to speech using ElevenLabs.
///
/// Returns raw MP3 audio data.
pub async fn synthesize(
    text: &str,
    voice: &str,
    rate: f64,
    lang: &str,
    api_key: &str,
) -> Result<Vec<u8>, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("Text is empty".into());
    }

    if api_key.is_empty() {
        return Err("ElevenLabs API key is required".into());
    }

    // Pick voice: user preference > default for language
    let voice_id = if voice.is_empty() {
        default_voice_for_lang(lang)
            .unwrap_or("21m00Tcm4TlvDq8ikWAM")
            .to_string()
    } else {
        voice.to_string()
    };

    let url = format!(
        "https://api.elevenlabs.io/v1/text-to-speech/{}?output_format=mp3_44100_128",
        voice_id
    );

    let body = json!({
        "text": text,
        "model_id": "eleven_multilingual_v2",
        "voice_settings": {
            "stability": 0.5,
            "similarity_boost": 0.75,
            "speed": rate
        }
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let resp = client
        .post(&url)
        .header("xi-api-key", api_key)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let error_body = resp.text().await.unwrap_or_default();
        return Err(format!(
            "ElevenLabs API error (HTTP {}): {}",
            status, error_body
        ));
    }

    let audio_bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?
        .to_vec();

    tracing::info!(
        "ElevenLabs TTS: synthesized {} chars in {} -> {} bytes MP3",
        text.len(),
        lang,
        audio_bytes.len()
    );

    Ok(audio_bytes)
}
