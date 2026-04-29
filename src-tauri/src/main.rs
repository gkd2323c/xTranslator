#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod batch;
mod commands;

use crate::batch::BatchExecutor;
use commands::{
    auto_backup_sst, build_dialog_tree, cancel_batch_job, compare_esp_files, compile_pex,
    export_xml, extract_ba2_file, extract_ba2_folder, extract_bsa_file, extract_bsa_folder,
    get_all_strings, get_batch_status, get_fuz_audio_data, get_is_dirty, get_stats,
    get_strings_chunk, get_strings_count, get_translation_providers, heuristic_search,
    import_xml, list_ba2_files, list_bsa_files, list_esp_files, load_esp, load_mcm_file,
    load_sst, parse_pex_strings, query_strings_command, save_mcm_file, save_sst, save_strings,
    scan_fuz_directory, set_deepl_api_key, set_openai_api_key, set_translation_provider,
    start_batch_export, start_batch_translate, translate_string, update_translation,
    load_config, save_config, AppState,
};
use std::sync::Arc;

fn main() {
    let api_config = xt_core::translation_api::config::ApiTranslatorConfig::load_from_file(
        std::path::Path::new("Misc/ApiTranslator.txt")
    ).unwrap_or_default();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Arc::new(AppState::new(api_config)))
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
            auto_backup_sst,
            export_xml,
            import_xml,
            get_is_dirty,
            save_strings,
            // Batch commands
            start_batch_translate,
            start_batch_export,
            get_batch_status,
            cancel_batch_job,
            list_esp_files,
            // BSA browser commands
            list_bsa_files,
            list_ba2_files,
            extract_bsa_file,
            extract_ba2_file,
            extract_bsa_folder,
            extract_ba2_folder,
            // PEX commands
            parse_pex_strings,
            compile_pex,
            compare_esp_files,
            // MCM commands
            load_mcm_file,
            save_mcm_file,
            // Config commands
            load_config,
            save_config,
            scan_fuz_directory,
            get_fuz_audio_data,
            build_dialog_tree
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
