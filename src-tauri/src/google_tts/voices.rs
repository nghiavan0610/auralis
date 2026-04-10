//! Google Cloud TTS voice listing.
//!
//! Fetches voices from the Google Cloud Text-to-Speech REST API.
//! Falls back to a curated list if the network request fails.

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// A voice available in Google Cloud TTS.
#[derive(Debug, Clone, Serialize)]
pub struct GoogleVoice {
    pub name: String,
    pub lang: String,
    pub gender: String,
    /// Whether this is a WaveNet/Neural2 voice (higher quality).
    pub natural: bool,
}

/// Raw voice entry from Google's API.
#[derive(Debug, Deserialize)]
struct ApiVoice {
    name: String,
    #[serde(rename = "languageCodes")]
    language_codes: Vec<String>,
    #[serde(rename = "ssmlGender")]
    ssml_gender: String,
}

/// Raw response from the Google voices endpoint.
#[derive(Debug, Deserialize)]
struct VoicesResponse {
    voices: Vec<ApiVoice>,
}

/// Languages we support in the app (STT + TTS consistency).
const SUPPORTED_LANGS: &[&str] = &[
    "en", "vi", "es", "fr", "de", "zh", "ja", "ko", "pt", "ru", "ar", "hi",
];

/// Cached voice list (fetched once, reused until API key changes).
static VOICE_CACHE: Lazy<Mutex<Option<Vec<GoogleVoice>>>> = Lazy::new(|| Mutex::new(None));

/// Clear the voice cache (call when API key changes).
pub fn invalidate_cache() {
    let mut cache = VOICE_CACHE.lock().unwrap();
    *cache = None;
}

/// Get the default Google Cloud TTS voice for a given language code.
pub fn default_voice_for_lang(lang: &str) -> Option<&'static str> {
    match lang {
        "en" => Some("en-US-Neural2-C"),
        "vi" => Some("vi-VN-Neural2-A"),
        "es" => Some("es-ES-Neural2-A"),
        "fr" => Some("fr-FR-Neural2-A"),
        "de" => Some("de-DE-Neural2-A"),
        "zh" => Some("cmn-CN-Neural2-A"),
        "ja" => Some("ja-JP-Neural2-A"),
        "ko" => Some("ko-KR-Neural2-A"),
        "pt" => Some("pt-BR-Neural2-A"),
        "ru" => Some("ru-RU-Neural2-A"),
        "ar" => Some("ar-XA-Neural2-A"),
        "hi" => Some("hi-IN-Neural2-A"),
        _ => None,
    }
}

/// Get all available Google Cloud TTS voices, filtered to supported languages.
///
/// On first call, fetches from Google's API and caches the result.
/// Falls back to a curated list if the network request fails.
pub async fn all_voices(api_key: &str) -> Vec<GoogleVoice> {
    // No point calling the API without a key — return fallback immediately without caching
    if api_key.is_empty() {
        return fallback_voices();
    }

    // Check cache first
    {
        let cache = VOICE_CACHE.lock().unwrap();
        if let Some(ref voices) = *cache {
            return voices.clone();
        }
    }

    // Fetch from API
    match fetch_voices_from_api(api_key).await {
        Ok(voices) => {
            tracing::info!("Google TTS: {} voices for supported languages", voices.len());
            let mut cache = VOICE_CACHE.lock().unwrap();
            cache.get_or_insert(voices).clone()
        }
        Err(err) => {
            tracing::warn!(
                "Google TTS: failed to fetch voices from API ({}), using fallback",
                err
            );
            let fallback = fallback_voices();
            let mut cache = VOICE_CACHE.lock().unwrap();
            cache.get_or_insert(fallback).clone()
        }
    }
}

async fn fetch_voices_from_api(api_key: &str) -> Result<Vec<GoogleVoice>, String> {
    let url = format!(
        "https://texttospeech.googleapis.com/v1/voices?key={}",
        api_key
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let api_response: VoicesResponse = resp
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    // Filter to Neural2/WaveNet voices for supported languages only
    let voices: Vec<GoogleVoice> = api_response
        .voices
        .into_iter()
        .filter(|v| {
            // Only Neural2, WaveNet, or Studio voices
            v.name.contains("Neural2")
                || v.name.contains("WaveNet")
                || v.name.contains("Studio")
        })
        .filter_map(|v| {
            // Extract short language code from the first languageCode
            let lang_code = v.language_codes.first()?;
            let short_lang = lang_code.split('-').next()?.to_lowercase();
            if !SUPPORTED_LANGS.contains(&short_lang.as_str()) {
                return None;
            }
            Some(GoogleVoice {
                name: v.name,
                lang: short_lang,
                gender: v.ssml_gender,
                natural: true,
            })
        })
        .collect();

    Ok(voices)
}

/// Fallback curated list used when the API is unreachable.
fn fallback_voices() -> Vec<GoogleVoice> {
    vec![
        // English
        GoogleVoice { name: "en-US-Neural2-C".into(), lang: "en".into(), gender: "FEMALE".into(), natural: true },
        GoogleVoice { name: "en-US-Neural2-D".into(), lang: "en".into(), gender: "MALE".into(), natural: true },
        GoogleVoice { name: "en-US-Wavenet-C".into(), lang: "en".into(), gender: "FEMALE".into(), natural: true },
        GoogleVoice { name: "en-US-Wavenet-D".into(), lang: "en".into(), gender: "MALE".into(), natural: true },
        // Vietnamese
        GoogleVoice { name: "vi-VN-Neural2-A".into(), lang: "vi".into(), gender: "FEMALE".into(), natural: true },
        GoogleVoice { name: "vi-VN-Neural2-D".into(), lang: "vi".into(), gender: "MALE".into(), natural: true },
        GoogleVoice { name: "vi-VN-Wavenet-A".into(), lang: "vi".into(), gender: "FEMALE".into(), natural: true },
        GoogleVoice { name: "vi-VN-Wavenet-D".into(), lang: "vi".into(), gender: "MALE".into(), natural: true },
        // Spanish
        GoogleVoice { name: "es-ES-Neural2-A".into(), lang: "es".into(), gender: "FEMALE".into(), natural: true },
        GoogleVoice { name: "es-ES-Neural2-B".into(), lang: "es".into(), gender: "MALE".into(), natural: true },
        // French
        GoogleVoice { name: "fr-FR-Neural2-A".into(), lang: "fr".into(), gender: "FEMALE".into(), natural: true },
        GoogleVoice { name: "fr-FR-Neural2-B".into(), lang: "fr".into(), gender: "MALE".into(), natural: true },
        // German
        GoogleVoice { name: "de-DE-Neural2-A".into(), lang: "de".into(), gender: "FEMALE".into(), natural: true },
        GoogleVoice { name: "de-DE-Neural2-B".into(), lang: "de".into(), gender: "MALE".into(), natural: true },
        // Chinese
        GoogleVoice { name: "cmn-CN-Neural2-A".into(), lang: "zh".into(), gender: "FEMALE".into(), natural: true },
        GoogleVoice { name: "cmn-CN-Neural2-B".into(), lang: "zh".into(), gender: "MALE".into(), natural: true },
        // Japanese
        GoogleVoice { name: "ja-JP-Neural2-A".into(), lang: "ja".into(), gender: "FEMALE".into(), natural: true },
        GoogleVoice { name: "ja-JP-Neural2-B".into(), lang: "ja".into(), gender: "MALE".into(), natural: true },
        // Korean
        GoogleVoice { name: "ko-KR-Neural2-A".into(), lang: "ko".into(), gender: "FEMALE".into(), natural: true },
        GoogleVoice { name: "ko-KR-Neural2-B".into(), lang: "ko".into(), gender: "MALE".into(), natural: true },
        // Portuguese
        GoogleVoice { name: "pt-BR-Neural2-A".into(), lang: "pt".into(), gender: "FEMALE".into(), natural: true },
        GoogleVoice { name: "pt-BR-Neural2-B".into(), lang: "pt".into(), gender: "MALE".into(), natural: true },
        // Russian
        GoogleVoice { name: "ru-RU-Neural2-A".into(), lang: "ru".into(), gender: "FEMALE".into(), natural: true },
        GoogleVoice { name: "ru-RU-Neural2-B".into(), lang: "ru".into(), gender: "MALE".into(), natural: true },
        // Arabic
        GoogleVoice { name: "ar-XA-Neural2-A".into(), lang: "ar".into(), gender: "FEMALE".into(), natural: true },
        GoogleVoice { name: "ar-XA-Neural2-B".into(), lang: "ar".into(), gender: "MALE".into(), natural: true },
        // Hindi
        GoogleVoice { name: "hi-IN-Neural2-A".into(), lang: "hi".into(), gender: "FEMALE".into(), natural: true },
        GoogleVoice { name: "hi-IN-Neural2-B".into(), lang: "hi".into(), gender: "MALE".into(), natural: true },
    ]
}
