//! Google Cloud TTS synthesis via REST API.
//!
//! Sends text + voice + language to Google's texttospeech.googleapis.com/v1
//! endpoint and returns MP3 audio bytes.

use base64::Engine;
use serde_json::json;

use super::voices::default_voice_for_lang;

/// Synthesize text to speech using Google Cloud TTS.
///
/// Returns MP3 audio data.
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
        return Err("Google API key is required".into());
    }

    // Pick voice: user preference > default for language
    let voice_name = if voice.is_empty() {
        default_voice_for_lang(lang)
            .unwrap_or("en-US-Neural2-C")
            .to_string()
    } else {
        voice.to_string()
    };

    // Determine language code for the API.
    // Google expects full locale like "en-US" but we store short codes like "en".
    let language_code = short_lang_to_locale(lang);

    let url = format!(
        "https://texttospeech.googleapis.com/v1/text:synthesize?key={}",
        api_key
    );

    let body = json!({
        "input": {
            "text": text
        },
        "voice": {
            "languageCode": language_code,
            "name": voice_name
        },
        "audioConfig": {
            "audioEncoding": "MP3",
            "speakingRate": rate
        }
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let error_body = resp.text().await.unwrap_or_default();
        return Err(format!("Google TTS API error (HTTP {}): {}", status, error_body));
    }

    let response_json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    let audio_content = response_json
        .get("audioContent")
        .and_then(|v| v.as_str())
        .ok_or("No audioContent in response")?;

    let audio_bytes = base64::engine::general_purpose::STANDARD
        .decode(audio_content)
        .map_err(|e| format!("Base64 decode error: {}", e))?;

    tracing::info!(
        "Google TTS: synthesized {} chars in {} -> {} bytes MP3",
        text.len(),
        lang,
        audio_bytes.len()
    );

    Ok(audio_bytes)
}

/// Map short language code to a Google-compatible locale.
fn short_lang_to_locale(lang: &str) -> &'static str {
    match lang {
        "en" => "en-US",
        "vi" => "vi-VN",
        "es" => "es-ES",
        "fr" => "fr-FR",
        "de" => "de-DE",
        "zh" => "cmn-CN",
        "ja" => "ja-JP",
        "ko" => "ko-KR",
        "pt" => "pt-BR",
        "ru" => "ru-RU",
        "ar" => "ar-XA",
        "hi" => "hi-IN",
        _ => "en-US",
    }
}
