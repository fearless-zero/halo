mod audio;
mod commands;
mod integrations;
mod llm;
mod models;
mod state;
mod storage;
mod transcribe;
mod types;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let base = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::env::temp_dir().join("halo"));
            if let Err(e) = storage::ensure_dirs(&base) {
                eprintln!("failed to initialise data directory: {e}");
            }
            app.manage(AppState::new(base));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_status,
            commands::get_settings,
            commands::update_settings,
            commands::list_audio_inputs,
            commands::get_models,
            commands::download_models,
            commands::get_note_styles,
            commands::save_note_style,
            commands::delete_note_style,
            commands::create_note,
            commands::start_recording,
            commands::stop_recording,
            commands::cancel_recording,
            commands::transcribe,
            commands::generate_notes,
            commands::list_notes,
            commands::get_note,
            commands::save_note,
            commands::delete_note,
            commands::export_note,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
