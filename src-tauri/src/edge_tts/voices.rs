//! Edge TTS voice listing.
//!
//! Fetches the full voice catalog from Microsoft's API at runtime.
//! Falls back to a curated list if the network request fails.

use serde::{Deserialize, Serialize};
use once_cell::sync::Lazy;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize)]
pub struct EdgeVoice {
    pub name: String,
    pub lang: String,
    pub gender: String,
}

/// Raw voice entry from Microsoft's API
#[derive(Debug, Deserialize)]
struct ApiVoice {
    #[serde(rename = "ShortName")]
    short_name: String,
    #[serde(rename = "Locale")]
    locale: String,
    #[serde(rename = "Gender")]
    gender: String,
}

const VOICES_URL: &str = "https://speech.platform.bing.com/consumer/speech/synthesize/readaloud/voices/list?trustedclienttoken=6A5AA1D4EAFF4E9FB37E23D68491D6F4";

/// Cached voice list (fetched once, reused)
static VOICE_CACHE: Lazy<Mutex<Option<Vec<EdgeVoice>>>> = Lazy::new(|| Mutex::new(None));

/// Get the default Edge TTS voice for a given language code.
pub fn default_voice_for_lang(lang: &str) -> Option<&'static str> {
    match lang {
        "en" => Some("en-US-EmmaMultilingualNeural"),
        "vi" => Some("vi-VN-HoaiMyNeural"),
        "es" => Some("es-ES-ElviraNeural"),
        "fr" => Some("fr-FR-DeniseNeural"),
        "de" => Some("de-DE-KatjaNeural"),
        "zh" => Some("zh-CN-XiaoxiaoNeural"),
        "ja" => Some("ja-JP-NanamiNeural"),
        "ko" => Some("ko-KR-SunHiNeural"),
        "pt" => Some("pt-BR-FranciscaNeural"),
        "ru" => Some("ru-RU-SvetlanaNeural"),
        "ar" => Some("ar-SA-ZariyahNeural"),
        "hi" => Some("hi-IN-SwaraNeural"),
        _ => None,
    }
}

/// Languages we support in the app (STT + TTS consistency).
const SUPPORTED_LANGS: &[&str] = &[
    "en", "vi", "es", "fr", "de", "zh", "ja", "ko", "pt", "ru", "ar", "hi",
];

/// Get all available Edge TTS voices, filtered to supported languages only.
///
/// On first call, fetches from Microsoft's API and caches the result.
/// Falls back to a curated list if the network request fails.
pub async fn all_voices() -> Vec<EdgeVoice> {
    // Check cache first
    {
        let cache = VOICE_CACHE.lock().unwrap();
        if let Some(ref voices) = *cache {
            return voices.clone();
        }
    }

    // Fetch from API
    match fetch_voices_from_api().await {
        Ok(voices) => {
            // Filter to supported languages only
            let filtered: Vec<EdgeVoice> = voices
                .into_iter()
                .filter(|v| SUPPORTED_LANGS.contains(&v.lang.as_str()))
                .collect();
            tracing::info!("Edge TTS: {} voices for supported languages", filtered.len());
            let mut cache = VOICE_CACHE.lock().unwrap();
            cache.get_or_insert(filtered).clone()
        }
        Err(err) => {
            tracing::warn!("Edge TTS: failed to fetch voices from API ({}), using fallback", err);
            let fallback = fallback_voices();
            let mut cache = VOICE_CACHE.lock().unwrap();
            cache.get_or_insert(fallback).clone()
        }
    }
}

async fn fetch_voices_from_api() -> Result<Vec<EdgeVoice>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let resp = client
        .get(VOICES_URL)
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let api_voices: Vec<ApiVoice> = resp
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    // Filter to Neural voices only and map to our struct
    let voices: Vec<EdgeVoice> = api_voices
        .into_iter()
        .filter(|v| v.short_name.contains("Neural"))
        .map(|v| EdgeVoice {
            name: v.short_name,
            lang: locale_to_lang(&v.locale),
            gender: v.gender,
        })
        .collect();

    Ok(voices)
}

/// Convert full locale like "en-US" to short language code "en".
fn locale_to_lang(locale: &str) -> String {
    locale
        .split('-')
        .next()
        .unwrap_or(locale)
        .to_lowercase()
}

/// Fallback curated list used when the API is unreachable.
fn fallback_voices() -> Vec<EdgeVoice> {
    vec![
        // English
        EdgeVoice { name: "en-US-EmmaMultilingualNeural".into(), lang: "en".into(), gender: "Female".into() },
        EdgeVoice { name: "en-US-JennyMultilingualNeural".into(), lang: "en".into(), gender: "Female".into() },
        EdgeVoice { name: "en-US-GuyNeural".into(), lang: "en".into(), gender: "Male".into() },
        EdgeVoice { name: "en-US-ChristopherNeural".into(), lang: "en".into(), gender: "Male".into() },
        // Vietnamese
        EdgeVoice { name: "vi-VN-HoaiMyNeural".into(), lang: "vi".into(), gender: "Female".into() },
        EdgeVoice { name: "vi-VN-NamMinhNeural".into(), lang: "vi".into(), gender: "Male".into() },
        // Spanish
        EdgeVoice { name: "es-ES-ElviraNeural".into(), lang: "es".into(), gender: "Female".into() },
        EdgeVoice { name: "es-ES-AlvaroNeural".into(), lang: "es".into(), gender: "Male".into() },
        // French
        EdgeVoice { name: "fr-FR-DeniseNeural".into(), lang: "fr".into(), gender: "Female".into() },
        EdgeVoice { name: "fr-FR-HenriNeural".into(), lang: "fr".into(), gender: "Male".into() },
        // German
        EdgeVoice { name: "de-DE-KatjaNeural".into(), lang: "de".into(), gender: "Female".into() },
        EdgeVoice { name: "de-DE-ConradNeural".into(), lang: "de".into(), gender: "Male".into() },
        // Chinese
        EdgeVoice { name: "zh-CN-XiaoxiaoNeural".into(), lang: "zh".into(), gender: "Female".into() },
        EdgeVoice { name: "zh-CN-YunxiNeural".into(), lang: "zh".into(), gender: "Male".into() },
        // Japanese
        EdgeVoice { name: "ja-JP-NanamiNeural".into(), lang: "ja".into(), gender: "Female".into() },
        EdgeVoice { name: "ja-JP-KeitaNeural".into(), lang: "ja".into(), gender: "Male".into() },
        // Korean
        EdgeVoice { name: "ko-KR-SunHiNeural".into(), lang: "ko".into(), gender: "Female".into() },
        EdgeVoice { name: "ko-KR-InJoonNeural".into(), lang: "ko".into(), gender: "Male".into() },
        // Portuguese
        EdgeVoice { name: "pt-BR-FranciscaNeural".into(), lang: "pt".into(), gender: "Female".into() },
        EdgeVoice { name: "pt-BR-AntonioNeural".into(), lang: "pt".into(), gender: "Male".into() },
        // Russian
        EdgeVoice { name: "ru-RU-SvetlanaNeural".into(), lang: "ru".into(), gender: "Female".into() },
        EdgeVoice { name: "ru-RU-DmitryNeural".into(), lang: "ru".into(), gender: "Male".into() },
        // Arabic
        EdgeVoice { name: "ar-SA-ZariyahNeural".into(), lang: "ar".into(), gender: "Female".into() },
        EdgeVoice { name: "ar-SA-HamedNeural".into(), lang: "ar".into(), gender: "Male".into() },
        // Hindi
        EdgeVoice { name: "hi-IN-SwaraNeural".into(), lang: "hi".into(), gender: "Female".into() },
        EdgeVoice { name: "hi-IN-MadhurNeural".into(), lang: "hi".into(), gender: "Male".into() },
    ]
}
