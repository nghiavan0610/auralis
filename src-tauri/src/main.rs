// Prevents additional console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod state;
mod commands;

use state::AuralisState;
use commands::*;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AuralisState::new())
        .invoke_handler(tauri::generate_handler![
            greet,
            start_translation,
            stop_translation,
            get_model_status,
            subscribe_events,
            set_source_language,
            set_target_language,
            is_translation_running,
            get_languages,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}
