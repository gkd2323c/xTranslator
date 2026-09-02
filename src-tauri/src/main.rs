#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod batch;
mod commands;
mod command_processor;

use crate::batch::BatchExecutor;
use crate::command_processor::run_command_processor;
use commands::{
    apply_def_ui_generator, get_default_def_ui_options,
    get_codepage_info, reload_with_codepage, apply_add_id_offset,
    apply_translation_cache, auto_backup_sst, batch_update_translations, build_dialog_tree,
    cancel_batch_job, cancel_string_batch_translate, check_aliases, check_pending_cache,
    colab_assign, colab_filter, colab_get_labels, colab_set_label, compare_esp_files,
    compare_source_dest, compile_pex, decompile_pex, delocalize_esp, deshape_arabic,
    discard_translation_cache, export_dial_html, export_xml, extract_ba2_file, extract_ba2_folder,
    extract_bsa_file, extract_bsa_folder, finalize, finalize_esp, get_all_strings, get_api_config,
    get_batch_status, get_esp_header, get_fuz_audio_data, get_fuz_lip_data, get_is_dirty, get_stats,
    get_strings_chunk, get_strings_count, get_translation_providers, header_batch_process,
    header_rules_add, header_rules_apply, header_rules_delete, header_rules_list,
    header_rules_load, header_rules_move, header_rules_save, header_rules_toggle,
    header_rules_update, header_templates_delete, header_templates_list, header_templates_load,
    header_templates_save, heuristic_search, import_xml, inject_archive, list_ba2_files, list_bsa_files,
    list_esp_files, load_config, load_data_configs, load_esp, load_mcm_file, load_sst,
    load_vocabulary, mcm_compare, parse_pex_strings, preproc_opts_delete, preproc_opts_list,
    preproc_opts_load, preproc_opts_save, preproc_opts_set, query_strings_command, rtl_preview,
    read_text_file, rtl_reverse, save_config, save_esp, save_mcm_file, save_sst, save_strings, scan_fuz_directory,
    set_azure_api_key, set_baidu_api_key, set_deepl_api_key, set_openai_api_key,
    set_translation_provider, set_yooudao_api_key, shape_arabic, spell_check_config,
    spell_check_ignore, spell_check_load, spell_check_suggestions, spell_check_text,
    spell_check_toggle, spell_check_unload, sst_merge, start_batch_export, start_batch_translate,
    start_string_batch_translate, tcsc_batch_convert, tcsc_convert, toolbox_transform,
    toolbox_load_exception_words, toolbox_get_exception_words, translate_string,
    update_translation, write_text_file, AppState,
};
use std::sync::Arc;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let api_config = xt_core::translation_api::config::ApiTranslatorConfig::load_from_file(
        std::path::Path::new("Misc/ApiTranslator.txt"),
    )
    .unwrap_or_default();

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
            // 批量命令
            start_batch_translate,
            start_batch_export,
            get_batch_status,
            cancel_batch_job,
            list_esp_files,
            run_command_processor,
            // BSA 浏览器命令
            list_bsa_files,
            list_ba2_files,
            extract_bsa_file,
            extract_ba2_file,
            extract_bsa_folder,
            extract_ba2_folder,
            // 归档注入（DP-06）
            inject_archive,
            // PEX 命令
            parse_pex_strings,
            compile_pex,
            decompile_pex,
            compare_esp_files,
            // MCM 命令
            load_mcm_file,
            save_mcm_file,
            mcm_compare,
            // 配置命令
            load_config,
            save_config,
            get_api_config,
            load_data_configs,
            scan_fuz_directory,
            get_fuz_audio_data,
            get_fuz_lip_data,
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
            // ESP 回写命令
            save_esp,
            finalize_esp,
            delocalize_esp,
            get_esp_header,
            // 翻译缓存命令
            check_pending_cache,
            apply_translation_cache,
            discard_translation_cache,
            // 字符串级批量翻译
            start_string_batch_translate,
            cancel_string_batch_translate,
            write_text_file,
            read_text_file,
            // 拼写检查
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
            colab_get_labels,
            colab_set_label,
            colab_assign,
            colab_filter,
            // 头部处理器
            get_default_def_ui_options,
            apply_def_ui_generator,
            get_codepage_info,
            reload_with_codepage,
            apply_add_id_offset,
            header_rules_load,
            header_rules_list,
            header_rules_toggle,
            header_rules_apply,
            header_rules_save,
            header_rules_delete,
            header_rules_move,
            header_rules_update,
            header_rules_add,
            // 模板
            header_templates_list,
            header_templates_save,
            header_templates_load,
            header_templates_delete,
            // 预处理选项
            preproc_opts_load,
            preproc_opts_list,
            preproc_opts_set,
            preproc_opts_delete,
            preproc_opts_save,
            // 头部批量向导
            header_batch_process,
            // 工具箱例外词
            toolbox_load_exception_words,
            toolbox_get_exception_words,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
