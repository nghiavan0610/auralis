//! Tauri commands for Edge TTS cloud synthesis.

use crate::edge_tts;
use base64::Engine;
use tauri::State;

use crate::state::AuralisState;

/// Synthesize text using Edge TTS and return base64-encoded MP3 audio.
#[tauri::command]
pub async fn edge_tts_synthesize(
    _state: State<'_, AuralisState>,
    text: String,
    voice: String,
    rate: f64,
    lang: String,
) -> Result<String, String> {
    let audio = edge_tts::communicate::synthesize(&text, &voice, rate, &lang).await?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&audio);
    Ok(encoded)
}

/// List available Edge TTS voices, optionally filtered by language.
#[tauri::command]
pub async fn edge_tts_list_voices(
    lang: Option<String>,
) -> Result<Vec<edge_tts::voices::EdgeVoice>, String> {
    let mut voices = edge_tts::voices::all_voices().await;
    if let Some(lang) = lang {
        let prefix = lang.to_lowercase();
        voices.retain(|v| v.lang.to_lowercase().starts_with(&prefix));
    }
    Ok(voices)
}
