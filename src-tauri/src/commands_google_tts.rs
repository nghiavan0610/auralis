//! Tauri commands for Google Cloud TTS synthesis.

use base64::Engine;
use tauri::State;

use crate::google_tts;
use crate::state::AuralisState;

/// Synthesize text using Google Cloud TTS and return base64-encoded MP3 audio.
#[tauri::command]
pub async fn google_tts_synthesize(
    state: State<'_, AuralisState>,
    text: String,
    voice: String,
    rate: f64,
    lang: String,
) -> Result<String, String> {
    let api_key = state
        .settings
        .lock()
        .await
        .google_api_key
        .clone();

    let audio = google_tts::synthesize::synthesize(&text, &voice, rate, &lang, &api_key).await?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&audio);
    Ok(encoded)
}

/// List available Google Cloud TTS voices, optionally filtered by language.
#[tauri::command]
pub async fn google_tts_list_voices(
    state: State<'_, AuralisState>,
    lang: Option<String>,
) -> Result<Vec<google_tts::voices::GoogleVoice>, String> {
    let api_key = state
        .settings
        .lock()
        .await
        .google_api_key
        .clone();

    let mut voices = google_tts::voices::all_voices(&api_key).await;
    if let Some(lang) = lang {
        let prefix = lang.to_lowercase();
        voices.retain(|v| v.lang.to_lowercase().starts_with(&prefix));
    }
    Ok(voices)
}
