#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod batch;
mod commands;

use crate::batch::BatchExecutor;
use commands::{
    cancel_batch_job, export_xml, get_all_strings, get_batch_status, get_is_dirty, get_stats,
    get_strings_chunk, get_strings_count, get_translation_providers, heuristic_search, import_xml,
    list_esp_files, load_esp, load_sst, query_strings_command, save_strings, save_sst,
    start_batch_export, start_batch_translate, set_deepl_api_key, set_openai_api_key,
    set_translation_provider, translate_string, update_translation, AppState,
};
use std::sync::Arc;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Arc::new(AppState::new()))
        .manage(Arc::new(BatchExecutor::new()))
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
            set_openai_api_key,
            set_deepl_api_key,
            set_translation_provider,
            get_translation_providers,
            export_xml,
            import_xml,
            get_is_dirty,
            save_strings,
            // Batch commands
            start_batch_translate,
            start_batch_export,
            get_batch_status,
            cancel_batch_job,
            list_esp_files
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
