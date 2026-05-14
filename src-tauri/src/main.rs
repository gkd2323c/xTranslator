#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod batch;
mod commands;

use crate::batch::BatchExecutor;
use commands::{
    auto_backup_sst, batch_update_translations, build_dialog_tree, cancel_batch_job, compare_esp_files, compare_source_dest,
    check_aliases, check_pending_cache, apply_translation_cache, discard_translation_cache, compile_pex,
    toolbox_transform,
    start_string_batch_translate, cancel_string_batch_translate,
    export_xml, extract_ba2_file, extract_ba2_folder, extract_bsa_file, extract_bsa_folder,
    finalize, get_all_strings, get_batch_status, get_fuz_audio_data, get_is_dirty, get_stats,
    get_strings_chunk, get_strings_count, get_translation_providers, heuristic_search,
    import_xml, list_ba2_files, list_bsa_files, list_esp_files, load_esp, load_mcm_file,
    load_sst, load_vocabulary, mcm_compare, parse_pex_strings, query_strings_command, save_mcm_file, save_sst, save_strings,
    scan_fuz_directory, set_deepl_api_key, set_openai_api_key, set_baidu_api_key, set_yooudao_api_key, set_azure_api_key, set_translation_provider,
    start_batch_export, start_batch_translate, tcsc_convert, tcsc_batch_convert, translate_string, update_translation,
    load_config, save_config, get_api_config, load_data_configs, rtl_reverse, AppState,
    spell_check_load, spell_check_unload, spell_check_toggle, spell_check_config,
    spell_check_text, spell_check_suggestions, spell_check_ignore,
    sst_merge, export_dial_html, rtl_preview,
    header_rules_load, header_rules_list, header_rules_toggle, header_rules_apply, header_rules_save,
    header_rules_delete, header_rules_move, header_rules_update, header_rules_add,
    header_templates_list, header_templates_save, header_templates_load, header_templates_delete,
    preproc_opts_load, preproc_opts_list, preproc_opts_set,     preproc_opts_delete, preproc_opts_save,
    header_batch_process,
    save_esp, finalize_esp, delocalize_esp, decompile_pex, get_esp_header, shape_arabic, deshape_arabic,
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
            batch_update_translations,
            heuristic_search,
            translate_string,
            set_openai_api_key,
            set_deepl_api_key,
            set_baidu_api_key,
            set_yooudao_api_key,
            set_azure_api_key,
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
            decompile_pex,
            compare_esp_files,
            // MCM commands
            load_mcm_file,
            save_mcm_file,
            mcm_compare,
            // Config commands
            load_config,
            save_config,
            get_api_config,
            load_data_configs,
            scan_fuz_directory,
            get_fuz_audio_data,
            build_dialog_tree,
            tcsc_convert,
            tcsc_batch_convert,
            load_vocabulary,
            finalize,
            compare_source_dest,
            check_aliases,
            toolbox_transform,
            rtl_reverse,
            shape_arabic,
            deshape_arabic,
            // ESP write-back commands
            save_esp,
            finalize_esp,
            delocalize_esp,
            get_esp_header,
            // Translation cache commands
            check_pending_cache,
            apply_translation_cache,
            discard_translation_cache,
            // String-level batch translation
            start_string_batch_translate,
            cancel_string_batch_translate,
            // Spell check
            spell_check_load,
            spell_check_unload,
            spell_check_toggle,
            spell_check_config,
            spell_check_text,
            spell_check_suggestions,
            spell_check_ignore,
            sst_merge,
            export_dial_html,
            rtl_preview,
            // Header Processor
            header_rules_load,
            header_rules_list,
            header_rules_toggle,
            header_rules_apply,
            header_rules_save,
            header_rules_delete,
            header_rules_move,
            header_rules_update,
            header_rules_add,
            // Templates
            header_templates_list,
            header_templates_save,
            header_templates_load,
            header_templates_delete,
            // Pre-processing options
            preproc_opts_load,
            preproc_opts_list,
            preproc_opts_set,
            preproc_opts_delete,
            preproc_opts_save,
            // Header batch wizard
            header_batch_process,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
