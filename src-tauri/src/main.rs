// Prevents additional console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod state;
mod model_downloader;
#[macro_use]
mod commands;
mod commands_audio;
mod commands_settings;
mod commands_pipeline;

use state::AuralisState;

// Import all public commands into scope
use commands::*;
use commands_audio::*;
use commands_settings::*;
use commands_pipeline::*;

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
            // Core commands
            greet,
            get_model_status,
            check_model_exists,
            download_model,
            download_all_models,
            // Audio commands
            start_audio_capture,
            stop_audio_capture,
            // Settings commands
            get_settings,
            save_settings,
            // Pipeline commands (offline mode)
            start_local_pipeline,
            stop_local_pipeline,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
