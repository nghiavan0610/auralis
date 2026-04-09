// Prevents additional console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod state;
#[macro_use]
mod commands;
mod commands_audio;
mod commands_settings;
mod commands_pipeline;
mod commands_tts;

use state::AuralisState;

// Import all public commands into scope
use commands::*;
use commands_audio::*;
use commands_settings::*;
use commands_pipeline::*;
use commands_tts::*;

// Import Manager trait for window access
use tauri::Manager;
use url::Url;

fn main() {
    // Initialize logging based on build configuration
    #[cfg(debug_assertions)]
    let _ = auralis::infrastructure::logging::init_dev_logging();

    #[cfg(not(debug_assertions))]
    let _ = auralis::infrastructure::logging::init_default_logging();

    // Log application startup
    tracing::info!("Starting Auralis application");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                let url = Url::parse("http://localhost:1420").unwrap();
                let _ = window.navigate(url);
            }
            Ok(())
        })
        .manage(AuralisState::new())
        .invoke_handler(tauri::generate_handler![
            // Core
            greet,
            // Audio streaming
            start_audio_capture,
            stop_audio_capture,
            // Settings
            get_settings,
            save_settings,
            // Offline pipeline
            start_local_pipeline,
            stop_local_pipeline,
            // Offline setup
            check_offline_ready,
            setup_offline_environment,
            // TTS
            speak_text,
            stop_tts,
            list_tts_voices,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
