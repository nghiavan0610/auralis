// Prevents additional console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod state;
#[macro_use]
mod commands;
mod commands_audio;
mod commands_settings;
mod commands_pipeline;
mod commands_edge_tts;
mod commands_google_tts;
mod commands_elevenlabs_tts;
mod edge_tts;
mod google_tts;
mod elevenlabs_tts;

use state::AuralisState;

// Import all public commands into scope
use commands::*;
use commands_audio::*;
use commands_settings::*;
use commands_pipeline::*;
use commands_edge_tts::*;
use commands_google_tts::*;
use commands_elevenlabs_tts::*;

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
            #[cfg(desktop)]
            {
                app.handle().plugin(tauri_plugin_updater::Builder::new().build())?;
                app.handle().plugin(tauri_plugin_process::init())?;
            }
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
            get_platform_info,
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
            // Edge TTS
            edge_tts_synthesize,
            edge_tts_list_voices,
            // Google Cloud TTS
            google_tts_synthesize,
            google_tts_list_voices,
            // ElevenLabs TTS
            elevenlabs_tts_synthesize,
            elevenlabs_tts_list_voices,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
