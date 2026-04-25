#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;

use commands::{export_xml, get_all_strings, get_is_dirty, get_stats, get_strings_chunk, get_strings_count, heuristic_search, import_xml, load_esp, load_sst, query_strings_command, save_strings, save_sst, set_api_key, translate_string, update_translation, AppState};
use std::sync::Arc;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Arc::new(AppState::new()))
        .invoke_handler(tauri::generate_handler![
            query_strings_command,
            get_stats,
            get_all_strings,
            get_strings_chunk,
            get_strings_count,
            load_esp,
            load_sst,
            save_sst,
            update_translation,
            heuristic_search,
            translate_string,
            set_api_key,
            export_xml,
            import_xml,
            get_is_dirty,
            save_strings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
