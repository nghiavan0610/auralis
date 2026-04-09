//! Tauri commands for macOS native Text-to-Speech
//!
//! Uses the macOS `say` command to speak translated text aloud.
//! Supports voice discovery (mapping language codes to macOS voices)
//! and interrupting previous speech when new text arrives.

use crate::state::AuralisState;
use std::process::Command;
use tauri::State;

// ---------------------------------------------------------------------------
// Language → macOS voice mapping
// ---------------------------------------------------------------------------

/// Preferred macOS voice names for each supported language.
/// These are standard voices available on macOS 12+ (Monterey).
/// If a voice is not installed, `say` will fail silently.
const LANG_VOICES: &[(&str, &str)] = &[
    ("en", "Samantha"),
    ("es", "Monica"),
    ("fr", "Thomas"),
    ("de", "Anna"),
    ("zh", "Ting-Ting"),
    ("ja", "Kyoko"),
    ("ko", "Yuna"),
    ("pt", "Luciana"),
    ("ru", "Milena"),
    ("ar", "Maged"),
    // Note: Vietnamese (vi) and Hindi (hi) have no standard macOS voices.
    // TTS will be skipped for these languages until Piper TTS is added.
];

/// Find the best macOS voice for a given language code.
/// Returns `None` if no voice is available for that language.
fn voice_for_language(lang: &str) -> Option<&'static str> {
    LANG_VOICES
        .iter()
        .find(|(code, _)| *code == lang)
        .map(|(_, voice)| *voice)
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Speak the given text using macOS `say` command.
///
/// If a previous speech is in progress, it is stopped first.
/// If no voice is available for the language, does nothing.
#[tauri::command]
pub async fn speak_text(
    state: State<'_, AuralisState>,
    text: String,
    language: String,
) -> Result<(), String> {
    // Check if TTS is enabled
    {
        let settings = state.settings.lock().await;
        if !settings.tts_enabled {
            return Ok(());
        }
    }

    if text.trim().is_empty() {
        return Ok(());
    }

    let voice = voice_for_language(&language);
    if voice.is_none() {
        tracing::debug!("No macOS voice available for language: {}", language);
        return Ok(());
    }
    let voice = voice.unwrap();

    // Stop any previous speech
    {
        let mut guard = state.tts_process.lock().map_err(|e| e.to_string())?;
        if let Some(ref mut child) = *guard {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    // Spawn new `say` command
    tracing::info!("TTS: speaking {} chars in {} (voice: {})", text.len(), language, voice);

    let child = Command::new("say")
        .arg("-v")
        .arg(voice)
        .arg(&text)
        .spawn()
        .map_err(|e| format!("Failed to start TTS: {}", e))?;

    // Store child process so we can kill it if interrupted.
    // No background cleanup thread: the process is reaped when the next
    // speak_text call kills it, or when stop_tts is called. If `say`
    // finishes naturally and nobody calls either, the zombie is harmless
    // and will be cleaned up on the next call or at app exit.
    {
        let mut guard = state.tts_process.lock().map_err(|e| e.to_string())?;
        *guard = Some(child);
    }

    Ok(())
}

/// Stop any currently playing TTS speech.
#[tauri::command]
pub async fn stop_tts(
    state: State<'_, AuralisState>,
) -> Result<(), String> {
    let mut guard = state.tts_process.lock().map_err(|e| e.to_string())?;
    if let Some(ref mut child) = *guard {
        let _ = child.kill();
        let _ = child.wait();
        *guard = None;
    }
    Ok(())
}

/// Discover available macOS TTS voices.
///
/// Returns a list of { language, voice_name } objects for all
/// voices installed on this system.
#[tauri::command]
pub async fn list_tts_voices() -> Result<Vec<serde_json::Value>, String> {
    let output = Command::new("say")
        .arg("-v")
        .arg("?")
        .output()
        .map_err(|e| format!("Failed to list voices: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut voices = Vec::new();

    for line in stdout.lines() {
        // Actual macOS format: "Albert    en_US    # Hello! My name is Albert."
        let parts: Vec<&str> = line.splitn(3, char::is_whitespace).collect();
        if parts.len() >= 2 {
            let voice_name = parts[0].trim();
            let locale = parts[1].trim();
            if !locale.is_empty() && !voice_name.is_empty() {
                // Extract language code from locale (e.g., "en_US" → "en")
                let lang = locale.split('-').next().unwrap_or(locale).split('_').next().unwrap_or(locale);
                voices.push(serde_json::json!({
                    "language": lang,
                    "locale": locale,
                    "voice_name": voice_name,
                }));
            }
        }
    }

    Ok(voices)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voice_for_language_known() {
        assert_eq!(voice_for_language("en"), Some("Samantha"));
        assert_eq!(voice_for_language("ja"), Some("Kyoko"));
        assert_eq!(voice_for_language("zh"), Some("Ting-Ting"));
    }

    #[test]
    fn test_voice_for_language_unknown() {
        assert_eq!(voice_for_language("vi"), None);
        assert_eq!(voice_for_language("hi"), None);
    }

    #[test]
    fn test_voice_for_language_not_in_list() {
        assert_eq!(voice_for_language("xx"), None);
    }
}
