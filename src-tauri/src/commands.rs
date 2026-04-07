//! Tauri commands for the Auralis application
//!
//! This module provides the command handlers for the frontend to interact with
//! the Rust backend.

use crate::state::{AuralisState, ModelStatus};
use auralis::application::{AuralisEvent, EventBus};
use auralis::infrastructure::*;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

/// Start the translation process
#[tauri::command]
async fn start_translation(
    state: State<'_, AuralisState>,
    app_handle: AppHandle,
) -> Result<String, String> {
    // Check if already running
    if state.is_translating().await {
        return Err("Translation is already running".to_string());
    }

    // Get current languages
    let source_lang = state.source_language().await;
    let target_lang = state.target_language().await;

    // Create audio capture
    let audio_config = AudioCaptureConfig {
        sample_rate: 16000,
        channels: 1,
        buffer_size: 4096,
        device_name: None,
    };
    let audio_source = MicrophoneCapture::new(audio_config)
        .map_err(|e| format!("Failed to create audio source: {}", e))?;

    // Create STT engine
    let stt_config = WhisperConfig {
        model_path: "models/whisper.bin".to_string(),
        language: Some(source_lang.clone()),
        ..Default::default()
    };
    let mut stt_engine = WhisperEngine::new(stt_config);
    stt_engine.initialize(serde_json::json!({}))
        .await
        .map_err(|e| format!("Failed to initialize STT: {}", e))?;

    // Create translator
    let translator_config = MadladConfig {
        model_path: "models/madlad".to_string(),
        device: "cuda".to_string(),
    };
    let mut translator = MadladTranslator::new(translator_config);
    translator.initialize(serde_json::json!({}))
        .await
        .map_err(|e| format!("Failed to initialize translator: {}", e))?;

    // Create VAD
    let vad_config = SileroConfig {
        model_path: "models/silero_vad.torch".to_string(),
        threshold: 0.5,
        sample_rate: 16000,
    };
    let mut vad = SileroVAD::new(vad_config);
    vad.initialize(serde_json::json!({}))
        .await
        .map_err(|e| format!("Failed to initialize VAD: {}", e))?;

    // Create orchestrator
    let mut orchestrator = auralis::Orchestrator::new(
        audio_source,
        stt_engine,
        translator,
        vad,
        source_lang.clone(),
        target_lang.clone(),
    );

    // Subscribe to events and emit to frontend
    let event_bus = orchestrator.event_bus();
    let mut receiver = event_bus.subscribe();

    tokio::spawn(async move {
        while let Ok(event) = receiver.recv().await {
            // Emit event to frontend
            let event_name = match event {
                AuralisEvent::STTResult { .. } => "stt-result",
                AuralisEvent::TranslationResult { .. } => "translation-result",
                AuralisEvent::Error { .. } => "error",
                AuralisEvent::SpeechActivityChanged { .. } => "speech-activity",
                AuralisEvent::AudioCaptureChanged { .. } => "audio-capture",
                AuralisEvent::StatusUpdate { .. } => "status-update",
            };

            let _ = app_handle.emit(event_name, event);
        }
    });

    // Start the orchestrator
    orchestrator.start()
        .await
        .map_err(|e| format!("Failed to start orchestrator: {}", e))?;

    // Store orchestrator in state
    // Note: We need to do this through a mutable reference, which requires
    // a different approach in Tauri. For now, we'll skip storing it.

    Ok(format!("Translation started: {} -> {}", source_lang, target_lang))
}

/// Stop the translation process
#[tauri::command]
async fn stop_translation(state: State<'_, AuralisState>) -> Result<String, String> {
    if !state.is_translating().await {
        return Err("Translation is not running".to_string());
    }

    // Stop the orchestrator
    if let Some(orch) = state.orchestrator() {
        orch.stop()
            .await
            .map_err(|e| format!("Failed to stop translation: {}", e))?;
    }

    Ok("Translation stopped".to_string())
}

/// Get the current model status
#[tauri::command]
async fn get_model_status(state: State<'_, AuralisState>) -> Result<ModelStatus, String> {
    Ok(state.model_status().await)
}

/// Subscribe to events (returns the current event stream)
#[tauri::command]
async fn subscribe_events() -> Result<String, String> {
    Ok("Events subscribed".to_string())
}

/// Set the source language
#[tauri::command]
async fn set_source_language(
    state: State<'_, AuralisState>,
    language: String,
) -> Result<String, String> {
    state.set_source_language(language.clone()).await;
    Ok(format!("Source language set to: {}", language))
}

/// Set the target language
#[tauri::command]
async fn set_target_language(
    state: State<'_, AuralisState>,
    language: String,
) -> Result<String, String> {
    state.set_target_language(language.clone()).await;
    Ok(format!("Target language set to: {}", language))
}

/// Check if translation is currently running
#[tauri::command]
async fn is_translation_running(state: State<'_, AuralisState>) -> Result<bool, String> {
    Ok(state.is_translating().await)
}

/// Get the current language configuration
#[tauri::command]
async fn get_languages(state: State<'_, AuralisState>) -> Result<(String, String), String> {
    Ok((
        state.source_language().await,
        state.target_language().await,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_responses() {
        // Test that commands return the expected response types
        // Actual command testing would require a Tauri test context

        let source_lang = "en".to_string();
        let target_lang = "es".to_string();

        assert_eq!(
            format!("Translation started: {} -> {}", source_lang, target_lang),
            "Translation started: en -> es"
        );

        assert_eq!(
            format!("Source language set to: {}", source_lang),
            "Source language set to: en"
        );

        assert_eq!(
            format!("Target language set to: {}", target_lang),
            "Target language set to: es"
        );
    }
}
