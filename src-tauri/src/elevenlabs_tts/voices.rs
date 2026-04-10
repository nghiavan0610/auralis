//! ElevenLabs TTS voice listing.
//!
//! Fetches voices from the ElevenLabs REST API.
//! Falls back to a curated list if the network request fails.

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// A voice available in ElevenLabs.
#[derive(Debug, Clone, Serialize)]
pub struct ElevenLabsVoice {
    /// Unique voice ID (used in synthesize URL path)
    pub voice_id: String,
    /// Human-readable name
    pub name: String,
    /// Inferred language code (best-effort from labels)
    pub lang: String,
    /// Gender from labels (male/female/unknown)
    pub gender: String,
}

/// Raw voice entry from ElevenLabs API.
#[derive(Debug, Deserialize)]
struct ApiVoice {
    voice_id: String,
    name: String,
    labels: Option<serde_json::Value>,
}

/// Raw response from the ElevenLabs voices endpoint.
#[derive(Debug, Deserialize)]
struct VoicesResponse {
    voices: Vec<ApiVoice>,
}

/// Cached voice list (fetched once, reused until API key changes).
static VOICE_CACHE: Lazy<Mutex<Option<Vec<ElevenLabsVoice>>>> =
    Lazy::new(|| Mutex::new(None));

/// Clear the voice cache (call when API key changes).
pub fn invalidate_cache() {
    let mut cache = VOICE_CACHE.lock().unwrap();
    *cache = None;
}

/// Get the default ElevenLabs voice ID for a given language code.
pub fn default_voice_for_lang(lang: &str) -> Option<&'static str> {
    match lang {
        "en" => Some("21m00Tcm4TlvDq8ikWAM"), // Rachel
        "vi" => Some("pNInz6obpgDQGcFmaJgB"), // Adam (multilingual)
        "es" => Some("pNInz6obpgDQGcFmaJgB"), // Adam (multilingual)
        "fr" => Some("pNInz6obpgDQGcFmaJgB"),
        "de" => Some("pNInz6obpgDQGcFmaJgB"),
        "zh" => Some("pNInz6obpgDQGcFmaJgB"),
        "ja" => Some("pNInz6obpgDQGcFmaJgB"),
        "ko" => Some("pNInz6obpgDQGcFmaJgB"),
        "pt" => Some("pNInz6obpgDQGcFmaJgB"),
        "ru" => Some("pNInz6obpgDQGcFmaJgB"),
        "ar" => Some("pNInz6obpgDQGcFmaJgB"),
        "hi" => Some("pNInz6obpgDQGcFmaJgB"),
        _ => None,
    }
}

/// Get all available ElevenLabs voices.
///
/// On first call, fetches from ElevenLabs API and caches the result.
/// Falls back to a curated list if the network request fails.
pub async fn all_voices(api_key: &str) -> Vec<ElevenLabsVoice> {
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
            tracing::info!("ElevenLabs TTS: {} voices fetched", voices.len());
            let mut cache = VOICE_CACHE.lock().unwrap();
            cache.get_or_insert(voices).clone()
        }
        Err(err) => {
            tracing::warn!(
                "ElevenLabs TTS: failed to fetch voices ({}), using fallback",
                err
            );
            let fallback = fallback_voices();
            let mut cache = VOICE_CACHE.lock().unwrap();
            cache.get_or_insert(fallback).clone()
        }
    }
}

async fn fetch_voices_from_api(api_key: &str) -> Result<Vec<ElevenLabsVoice>, String> {
    let url = "https://api.elevenlabs.io/v1/voices";

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    let resp = client
        .get(url)
        .header("xi-api-key", api_key)
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

    let voices: Vec<ElevenLabsVoice> = api_response
        .voices
        .into_iter()
        .map(|v| {
            let labels = v.labels.unwrap_or(serde_json::Value::Null);
            let gender = labels
                .get("gender")
                .and_then(|g| g.as_str())
                .unwrap_or("unknown")
                .to_string();
            let lang = labels
                .get("language")
                .and_then(|l| l.as_str())
                .unwrap_or("en")
                .to_string();

            ElevenLabsVoice {
                voice_id: v.voice_id,
                name: v.name,
                lang,
                gender,
            }
        })
        .collect();

    Ok(voices)
}

/// Fallback curated list used when the API is unreachable.
fn fallback_voices() -> Vec<ElevenLabsVoice> {
    vec![
        ElevenLabsVoice {
            voice_id: "21m00Tcm4TlvDq8ikWAM".into(),
            name: "Rachel".into(),
            lang: "en".into(),
            gender: "female".into(),
        },
        ElevenLabsVoice {
            voice_id: "AZnzlk1XvdvUeBnXmlld".into(),
            name: "Domi".into(),
            lang: "en".into(),
            gender: "female".into(),
        },
        ElevenLabsVoice {
            voice_id: "EXAVITQu4vr4xnSDxMaL".into(),
            name: "Bella".into(),
            lang: "en".into(),
            gender: "female".into(),
        },
        ElevenLabsVoice {
            voice_id: "ErXwobaYiN019PkySvjV".into(),
            name: "Antoni".into(),
            lang: "en".into(),
            gender: "male".into(),
        },
        ElevenLabsVoice {
            voice_id: "MF3mGyEYCl7XYWbV9V6O".into(),
            name: "Elli".into(),
            lang: "en".into(),
            gender: "female".into(),
        },
        ElevenLabsVoice {
            voice_id: "TxGEqnHWrfWFTfGW9XjX".into(),
            name: "Josh".into(),
            lang: "en".into(),
            gender: "male".into(),
        },
        ElevenLabsVoice {
            voice_id: "VR6AewLTigWG4xSOukaG".into(),
            name: "Arnold".into(),
            lang: "en".into(),
            gender: "male".into(),
        },
        ElevenLabsVoice {
            voice_id: "pNInz6obpgDQGcFmaJgB".into(),
            name: "Adam".into(),
            lang: "en".into(),
            gender: "male".into(),
        },
        ElevenLabsVoice {
            voice_id: "yoZ06aMxZJJ28mfd3POQ".into(),
            name: "Sam".into(),
            lang: "en".into(),
            gender: "male".into(),
        },
    ]
}
