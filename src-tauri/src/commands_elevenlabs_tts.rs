//! Tauri commands for ElevenLabs TTS synthesis.

use base64::Engine;
use tauri::State;

use crate::elevenlabs_tts;
use crate::state::AuralisState;

/// Synthesize text using ElevenLabs and return base64-encoded MP3 audio.
#[tauri::command]
pub async fn elevenlabs_tts_synthesize(
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
        .elevenlabs_api_key
        .clone();

    let audio = elevenlabs_tts::synthesize::synthesize(&text, &voice, rate, &lang, &api_key).await?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&audio);
    Ok(encoded)
}

/// List available ElevenLabs voices.
#[tauri::command]
pub async fn elevenlabs_tts_list_voices(
    state: State<'_, AuralisState>,
) -> Result<Vec<elevenlabs_tts::voices::ElevenLabsVoice>, String> {
    let api_key = state
        .settings
        .lock()
        .await
        .elevenlabs_api_key
        .clone();

    let voices = elevenlabs_tts::voices::all_voices(&api_key).await;
    Ok(voices)
}
