use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use tauri::{Emitter, Manager};
use xt_core::command_processor::{
    execute_command_processor, parse_command_processor, CommandErrorPolicy, CommandProcessorGlobals,
    CommandProcessorHost, CommandRule, ProcessorCommand, ProcessorCommandKind,
};
use xt_shared::dto::{
    CommandProcessorActiveFileDto, CommandProcessorErrorPolicyDto, CommandProcessorFailureDto,
    CommandProcessorProgressDto, CommandProcessorRunRequest, CommandProcessorRunResponse,
    FinalizeRequest, SaveEspRequest, SstApplyOptionsDto, SstMatchModeDto, SstOverwriteScopeDto,
    TranslateRequest,
};

use crate::commands::{self, AppState};
use xt_core::esp::header::{GenericHeader, RecordHeaderData};
use xt_core::types::esp_pointer::split_form_id_identity;
use xt_core::types::game_id::GameId;
use xt_core::types::params::SkyStringParams;

const PROGRESS_EVENT: &str = "command-processor-progress";

pub struct TauriCommandProcessorHost {
    window: tauri::Window,
    data_dir: Option<PathBuf>,
    game: Option<String>,
    warnings: Vec<String>,
    file_context_changed: bool,
    active_file: Option<CommandProcessorActiveFileDto>,
}

impl TauriCommandProcessorHost {
    fn new(window: tauri::Window, request: &CommandProcessorRunRequest) -> Self {
        Self {
            window,
            data_dir: request.data_dir.as_deref().map(PathBuf::from),
            game: request.game.clone(),
            warnings: Vec::new(),
            file_context_changed: false,
            active_file: None,
        }
    }

    fn emit_progress(
        &self,
        stage: &str,
        rule_number: usize,
        command_number: Option<usize>,
        line: usize,
        command: Option<&str>,
        message: impl Into<String>,
    ) {
        let _ = self.window.emit(
            PROGRESS_EVENT,
            CommandProcessorProgressDto {
                stage: stage.to_string(),
                rule_number,
                command_number,
                line,
                command: command.map(str::to_string),
                message: message.into(),
            },
        );
    }

    fn add_warning(&mut self, warning: String, rule_number: usize, command: &ProcessorCommand) {
        self.emit_progress(
            "message",
            rule_number,
            None,
            command.line,
            Some(command.kind.name()),
            warning.clone(),
        );
        self.warnings.push(warning);
    }

    fn resolve_load_path(&self, rule: &CommandRule, path: &str) -> Result<PathBuf, String> {
        if !rule.use_data_dir {
            return Ok(PathBuf::from(path));
        }

        let data_dir = self.data_dir.as_ref().ok_or_else(|| {
            "UseDataDir=true requires data_dir in the command processor run request".to_string()
        })?;
        let file_name = Path::new(path)
            .file_name()
            .ok_or_else(|| format!("LoadFile path has no file name: {path}"))?;
        Ok(data_dir.join(file_name))
    }

    fn strings_dir_for_load(&self, esp_path: &Path) -> Option<String> {
        let candidate = self
            .data_dir
            .as_ref()
            .map(|dir| dir.join("Strings"))
            .or_else(|| esp_path.parent().map(|parent| parent.join("Strings")))?;
        candidate
            .is_dir()
            .then(|| candidate.to_string_lossy().into_owned())
    }

    fn resolve_import_path(
        &self,
        globals: &CommandProcessorGlobals,
        path: &str,
    ) -> Result<PathBuf, String> {
        let resolved = if let Some(folder) = globals.import_folder.as_deref() {
            let file_name = Path::new(path)
                .file_name()
                .ok_or_else(|| format!("import path has no file name: {path}"))?;
            PathBuf::from(folder).join(file_name)
        } else {
            PathBuf::from(path)
        };
        Ok(resolved)
    }

    fn resolve_apply_sst_path(
        &self,
        globals: &CommandProcessorGlobals,
        rule: &CommandRule,
        path: &str,
    ) -> Result<PathBuf, String> {
        if let Some(folder) = globals.vocab_folder.as_deref() {
            let source = rule.lang_source.as_deref().ok_or_else(|| {
                "ApplySst with Global_VocabFolder requires LangSource".to_string()
            })?;
            let dest = rule.lang_dest.as_deref().ok_or_else(|| {
                "ApplySst with Global_VocabFolder requires LangDest".to_string()
            })?;
            return Ok(PathBuf::from(folder).join(delphi_sst_filename(path, source, dest)?));
        }

        let direct = PathBuf::from(path);
        if direct.is_file() {
            return Ok(direct);
        }

        // The Delphi fallback uses its configured SSTUserFolder. The Rust rewrite does not
        // currently have an equivalent global SST folder, so allow a language-suffixed file
        // next to the command path before failing explicitly.
        if let (Some(source), Some(dest)) = (rule.lang_source.as_deref(), rule.lang_dest.as_deref()) {
            let generated_name = delphi_sst_filename(path, source, dest)?;
            let generated = direct
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .map(|parent| parent.join(&generated_name))
                .unwrap_or_else(|| PathBuf::from(generated_name));
            if generated.is_file() {
                return Ok(generated);
            }
        }

        Ok(direct)
    }

    fn loaded_file_context(&self) -> Result<LoadedFileContext, String> {
        let state = self.window.state::<Arc<AppState>>();
        let file_info = state.file_info.lock().map_err(|e| e.to_string())?;
        let info = file_info
            .as_ref()
            .ok_or_else(|| "no file is currently loaded".to_string())?;
        let esp_path = PathBuf::from(&info.esp_path);
        let strings_dir = info.strings_dir.as_deref().map(PathBuf::from);
        drop(file_info);

        let esp_file = state.esp_file.lock().map_err(|e| e.to_string())?;
        let is_localized = esp_file
            .as_ref()
            .map(|file| file.tes4.parse_fields().is_localized)
            .unwrap_or(false);

        Ok(LoadedFileContext {
            esp_path,
            strings_dir,
            is_localized,
        })
    }

    fn export_folder(
        &self,
        globals: &CommandProcessorGlobals,
        rule: &CommandRule,
        context: &LoadedFileContext,
    ) -> Result<PathBuf, String> {
        let source_base = if context.is_localized {
            context
                .strings_dir
                .clone()
                .or_else(|| context.esp_path.parent().map(Path::to_path_buf))
        } else {
            context.esp_path.parent().map(Path::to_path_buf)
        }
        .ok_or_else(|| "loaded file has no parent directory".to_string())?;

        // Delphi parses Global_ExportFolder but its current runCommands path never consumes it.
        // Honor it here as the explicit global output base; otherwise preserve Delphi's
        // addon-folder + ExportSubFolder behavior.
        let mut output = globals
            .export_folder
            .as_deref()
            .map(PathBuf::from)
            .unwrap_or(source_base);
        if let Some(subfolder) = rule.export_subfolder.as_deref() {
            output.push(subfolder);
        }
        Ok(output)
    }

    async fn load_file(&mut self, rule: &CommandRule, path: &str) -> Result<(), String> {
        let esp_path = self.resolve_load_path(rule, path)?;
        if !esp_path.is_file() {
            return Err(format!("LoadFile target does not exist: {}", esp_path.display()));
        }

        let strings_dir = self.strings_dir_for_load(&esp_path);
        let window = self.window.clone();
        let state = window.state::<Arc<AppState>>();
        let stats = commands::load_esp(
            window.clone(),
            state,
            esp_path.to_string_lossy().into_owned(),
            strings_dir.clone(),
            rule.lang_source.clone(),
            self.game.clone(),
        )
        .await?;
        self.file_context_changed = true;
        self.active_file = Some(CommandProcessorActiveFileDto {
            esp_path: esp_path.to_string_lossy().into_owned(),
            strings_dir,
            stats,
        });
        Ok(())
    }

    async fn apply_sst(
        &self,
        path: PathBuf,
        compare_option: u8,
        apply_mode: u8,
    ) -> Result<(), String> {
        if !path.is_file() {
            return Err(format!("SST file does not exist: {}", path.display()));
        }
        let options = processor_sst_options(compare_option, apply_mode)?;
        let state = self.window.state::<Arc<AppState>>();
        commands::load_sst(state, path.to_string_lossy().into_owned(), Some(options))
            .await
            .map(|_| ())
    }

    async fn import_xml(&self, path: PathBuf) -> Result<(), String> {
        if !path.is_file() {
            return Err(format!("XML file does not exist: {}", path.display()));
        }
        let window = self.window.clone();
        let state = window.state::<Arc<AppState>>();
        commands::import_xml(window.clone(), state, path.to_string_lossy().into_owned())
            .await
            .map(|_| ())
    }

    async fn finalize_rule(
        &self,
        globals: &CommandProcessorGlobals,
        rule: &CommandRule,
    ) -> Result<(), String> {
        let context = self.loaded_file_context()?;
        let output_folder = self.export_folder(globals, rule, &context)?;
        std::fs::create_dir_all(&output_folder).map_err(|e| {
            format!(
                "failed to create finalize output directory {}: {e}",
                output_folder.display()
            )
        })?;

        let file_name = context
            .esp_path
            .file_name()
            .ok_or_else(|| "loaded ESP path has no file name".to_string())?
            .to_string_lossy()
            .into_owned();
        let base_name = context
            .esp_path
            .file_stem()
            .ok_or_else(|| "loaded ESP path has no base name".to_string())?
            .to_string_lossy()
            .into_owned();

        if context.is_localized {
            let target_lang = rule
                .lang_dest
                .clone()
                .ok_or_else(|| "Finalize for a localized plugin requires LangDest".to_string())?;
            let window = self.window.clone();
            let state = window.state::<Arc<AppState>>();
            commands::finalize(
                window.clone(),
                state,
                FinalizeRequest {
                    strings_output_dir: output_folder.to_string_lossy().into_owned(),
                    target_lang,
                    base_name,
                    sst_path: None,
                    xml_path: None,
                },
            )
            .await
            .map(|_| ())
        } else {
            let state = self.window.state::<Arc<AppState>>();
            commands::save_esp(
                state,
                SaveEspRequest {
                    path: output_folder.join(file_name).to_string_lossy().into_owned(),
                    create_backup: true,
                },
            )
            .await
            .map(|_| ())
        }
    }

    fn close_loaded_file(&mut self) -> Result<(), String> {
        let state = self.window.state::<Arc<AppState>>();
        state.strings.lock().map_err(|e| e.to_string())?.clear();
        state
            .sst_old_data
            .lock()
            .map_err(|e| e.to_string())?
            .clear();
        *state.file_info.lock().map_err(|e| e.to_string())? = None;
        *state.esp_file.lock().map_err(|e| e.to_string())? = None;
        *state.codepage_table.lock().map_err(|e| e.to_string())? = None;
        *state.batch_queue.lock().map_err(|e| e.to_string())? = None;
        *state.is_dirty.lock().map_err(|e| e.to_string())? = false;
        self.file_context_changed = true;
        self.active_file = None;
        Ok(())
    }

    async fn save_dictionary(
        &self,
        globals: &CommandProcessorGlobals,
        rule: &CommandRule,
    ) -> Result<(), String> {
        let context = self.loaded_file_context()?;
        let source = rule
            .lang_source
            .as_deref()
            .ok_or_else(|| "SaveDictionary requires LangSource".to_string())?;
        let dest = rule
            .lang_dest
            .as_deref()
            .ok_or_else(|| "SaveDictionary requires LangDest".to_string())?;
        let folder = globals.vocab_folder.as_deref().ok_or_else(|| {
            "SaveDictionary requires Global_VocabFolder because the Rust rewrite has no SSTUserFolder setting yet"
                .to_string()
        })?;
        std::fs::create_dir_all(folder)
            .map_err(|e| format!("failed to create Global_VocabFolder {folder}: {e}"))?;
        let name = delphi_sst_filename(
            context
                .esp_path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| "loaded ESP path has no UTF-8 file name".to_string())?,
            source,
            dest,
        )?;
        let masters = {
            let state = self.window.state::<Arc<AppState>>();
            let esp_file = state.esp_file.lock().map_err(|e| e.to_string())?;
            esp_file
                .as_ref()
                .map(|file| file.tes4.parse_fields().masters)
                .unwrap_or_default()
        };
        let state = self.window.state::<Arc<AppState>>();
        commands::save_sst(
            state,
            PathBuf::from(folder).join(name).to_string_lossy().into_owned(),
            Some(masters),
        )
        .await
    }

    async fn api_translation(
        &mut self,
        rule_number: usize,
        rule: &CommandRule,
        command: &ProcessorCommand,
        api_id: u8,
        auto_no_trans_tag: bool,
    ) -> Result<(), String> {
        let provider = delphi_api_provider(api_id)?;
        let source_lang = rule
            .lang_source
            .clone()
            .ok_or_else(|| "ApiTranslation requires LangSource".to_string())?;
        let target_lang = rule
            .lang_dest
            .clone()
            .ok_or_else(|| "ApiTranslation requires LangDest".to_string())?;

        if auto_no_trans_tag {
            self.add_warning(
                format!(
                    "rule {rule_number}, line {} ApiTranslation:{api_id}:1 requested Delphi \
                     auto NoTranslation tagging; the Rust rewrite does not yet load the Delphi \
                     lRulesNoTransListIn/Out rule set, so translation continues without that pre-tag pass",
                    command.line
                ),
                rule_number,
                command,
            );
        }

        let candidates: Vec<(u32, String)> = {
            let state = self.window.state::<Arc<AppState>>();
            let strings = state.strings.lock().map_err(|e| e.to_string())?;
            strings
                .iter()
                .filter(|sk| {
                    // Delphi StartApiTranslationArray(false, ...) uses
                    // compareOptNoTransAndPartialsExLocked: exclude VMAD, locked,
                    // translated, incomplete and validated strings.
                    !sk.internal_params
                        .is_set(xt_core::types::params::SkyStringInternalParams::IS_VMAD_STRING)
                        && !sk.params.is_locked()
                        && !sk.params.is_translated()
                        && !sk.params.is_incomplete()
                        && !sk.params.is_validated()
                })
                .map(|sk| (sk.id, sk.source.clone()))
                .collect()
        };

        if candidates.is_empty() {
            self.emit_progress(
                "message",
                rule_number,
                None,
                command.line,
                Some(command.kind.name()),
                "ApiTranslation found no eligible strings",
            );
            return Ok(());
        }

        // Delphi's array translator reuses one returned translation for equal source
        // strings. Cache by exact source here as well, avoiding duplicate API calls.
        let mut translated_by_source: HashMap<String, String> = HashMap::new();
        let mut translated_by_id: HashMap<u32, String> = HashMap::with_capacity(candidates.len());

        for (index, (id, source)) in candidates.iter().enumerate() {
            let translation = if let Some(existing) = translated_by_source.get(source) {
                existing.clone()
            } else {
                let window = self.window.clone();
                let state = window.state::<Arc<AppState>>();
                let translated = commands::translate_string(
                    state,
                    TranslateRequest {
                        text: source.clone(),
                        source_lang: Some(source_lang.clone()),
                        target_lang: Some(target_lang.clone()),
                        provider: Some(provider.to_string()),
                    },
                )
                .await?;
                translated_by_source.insert(source.clone(), translated.clone());
                translated
            };

            translated_by_id.insert(*id, translation);

            if index == 0 || (index + 1) % 25 == 0 || index + 1 == candidates.len() {
                self.emit_progress(
                    "message",
                    rule_number,
                    None,
                    command.line,
                    Some(command.kind.name()),
                    format!(
                        "ApiTranslation {}/{} eligible strings processed via {}",
                        index + 1,
                        candidates.len(),
                        provider
                    ),
                );
            }
        }

        let state = self.window.state::<Arc<AppState>>();
        let mut strings = state.strings.lock().map_err(|e| e.to_string())?;
        let mut applied = 0usize;
        for sk in strings.iter_mut() {
            let Some(translation) = translated_by_id.get(&sk.id) else {
                continue;
            };
            sk.translation = translation.clone();
            // Delphi resetStatus([incompleteTrans]) after successful machine translation.
            sk.params = xt_core::types::params::SkyStringParams::new();
            sk.params
                .set(xt_core::types::params::SkyStringParams::INCOMPLETE_TRANS, true);
            applied += 1;
        }
        drop(strings);

        if applied > 0 {
            *state.is_dirty.lock().map_err(|e| e.to_string())? = true;
        }

        self.emit_progress(
            "message",
            rule_number,
            None,
            command.line,
            Some(command.kind.name()),
            format!("ApiTranslation applied {applied} machine translations"),
        );
        Ok(())
    }

    async fn generate_dictionaries(
        &mut self,
        globals: &CommandProcessorGlobals,
        rule_number: usize,
        rule: &CommandRule,
        command: &ProcessorCommand,
    ) -> Result<(), String> {
        let data_dir = self.data_dir.clone().ok_or_else(|| {
            "GenerateDictionaries requires the Bethesda Data directory in the run request"
                .to_string()
        })?;
        let game_name = self
            .game
            .as_deref()
            .ok_or_else(|| "GenerateDictionaries requires an explicit game workspace".to_string())?;
        let game = GameId::from_alias(game_name)
            .ok_or_else(|| format!("unknown game workspace for GenerateDictionaries: {game_name}"))?;
        let source = rule
            .lang_source
            .clone()
            .ok_or_else(|| "GenerateDictionaries requires LangSource".to_string())?;
        let dest = rule
            .lang_dest
            .clone()
            .ok_or_else(|| "GenerateDictionaries requires LangDest".to_string())?;
        let output_dir = globals.vocab_folder.as_deref().ok_or_else(|| {
            "GenerateDictionaries requires Global_VocabFolder as the Rust output folder"
                .to_string()
        })?;
        std::fs::create_dir_all(output_dir)
            .map_err(|e| format!("failed to create Global_VocabFolder {output_dir}: {e}"))?;

        let vocabulary_path = PathBuf::from("Data")
            .join(game.as_str())
            .join("vocabulary.txt");
        let names = xt_core::vocabulary::parse_vocabulary_file(&vocabulary_path).map_err(|e| {
            format!(
                "failed to read {} for GenerateDictionaries: {e}",
                vocabulary_path.display()
            )
        })?;
        if names.is_empty() {
            return Err(format!(
                "{} contains no STRINGS= entries",
                vocabulary_path.display()
            ));
        }

        let snapshot = AppStateSnapshot::capture(&self.window)?;
        let result = self
            .generate_dictionaries_inner(
                &data_dir,
                game,
                output_dir,
                &source,
                &dest,
                &names,
                rule_number,
                command,
            )
            .await;
        let restore_result = snapshot.restore(&self.window);

        match (result, restore_result) {
            (Err(error), Err(restore)) => Err(format!(
                "{error}; additionally failed to restore the previous app state: {restore}"
            )),
            (Err(error), Ok(())) => Err(error),
            (Ok(_), Err(restore)) => Err(format!(
                "dictionaries were generated but the previous app state could not be restored: {restore}"
            )),
            (Ok(count), Ok(())) => {
                self.emit_progress(
                    "message",
                    rule_number,
                    None,
                    command.line,
                    Some(command.kind.name()),
                    format!("GenerateDictionaries created {count} SST dictionaries"),
                );
                Ok(())
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn generate_dictionaries_inner(
        &mut self,
        data_dir: &Path,
        game: GameId,
        output_dir: &str,
        source: &str,
        dest: &str,
        names: &[String],
        rule_number: usize,
        command: &ProcessorCommand,
    ) -> Result<usize, String> {
        let strings_dir = data_dir.join("Strings");
        if !strings_dir.is_dir() {
            return Err(format!(
                "GenerateDictionaries requires a Strings directory under {}",
                data_dir.display()
            ));
        }

        let mut generated = 0usize;
        for (index, name) in names.iter().enumerate() {
            let Some(plugin_path) = find_game_plugin(data_dir, name) else {
                self.add_warning(
                    format!(
                        "GenerateDictionaries skipped {name}: no .esm/.esl/.esp was found in {}",
                        data_dir.display()
                    ),
                    rule_number,
                    command,
                );
                continue;
            };

            let window = self.window.clone();
            let state = window.state::<Arc<AppState>>();
            if let Err(error) = commands::load_esp(
                window.clone(),
                state,
                plugin_path.to_string_lossy().into_owned(),
                Some(strings_dir.to_string_lossy().into_owned()),
                Some(source.to_string()),
                Some(game.as_str().to_string()),
            )
            .await
            {
                self.add_warning(
                    format!("GenerateDictionaries skipped {name}: {error}"),
                    rule_number,
                    command,
                );
                continue;
            }

            let codepage = {
                let state = self.window.state::<Arc<AppState>>();
                let value = state
                    .codepage_table
                    .lock()
                    .map_err(|e| e.to_string())?
                    .clone();
                value
            };
            let fallback_codepage = xt_core::strings::CodepageTable::default();
            let target_files = xt_core::esp::parser::StringsFiles::load_from_dir_with_language(
                &strings_dir,
                name,
                dest,
                codepage.as_ref().unwrap_or(&fallback_codepage),
            );
            if target_files.loaded_count() == 0 {
                self.add_warning(
                    format!(
                        "GenerateDictionaries skipped {name}: no target-language {dest} Strings files were found"
                    ),
                    rule_number,
                    command,
                );
                continue;
            }

            let translated = {
                let state = self.window.state::<Arc<AppState>>();
                let mut strings = state.strings.lock().map_err(|e| e.to_string())?;
                let mut translated = 0usize;
                for sk in strings.iter_mut() {
                    if sk.esp_ptr.str_id < 0 {
                        continue;
                    }
                    let target = match sk.list_index {
                        0 => target_files.strings.as_ref(),
                        1 => target_files.dlstrings.as_ref(),
                        2 => target_files.ilstrings.as_ref(),
                        _ => None,
                    }
                    .and_then(|file| file.strings.get(&(sk.esp_ptr.str_id as u32)));

                    if let Some(target) = target.filter(|text| !text.is_empty()) {
                        sk.translation = target.clone();
                        sk.params = SkyStringParams::new();
                        sk.params.set(SkyStringParams::TRANSLATED, true);
                        translated += 1;
                    }
                }
                translated
            };

            if translated == 0 {
                self.add_warning(
                    format!(
                        "GenerateDictionaries skipped {name}: source and target Strings had no matching IDs"
                    ),
                    rule_number,
                    command,
                );
                continue;
            }

            let masters = {
                let state = self.window.state::<Arc<AppState>>();
                let esp_file = state.esp_file.lock().map_err(|e| e.to_string())?;
                esp_file
                    .as_ref()
                    .map(|file| file.tes4.parse_fields().masters)
                    .unwrap_or_default()
            };
            let output = PathBuf::from(output_dir).join(delphi_sst_filename(name, source, dest)?);
            let state = self.window.state::<Arc<AppState>>();
            commands::save_sst(
                state,
                output.to_string_lossy().into_owned(),
                Some(masters),
            )
            .await?;
            generated += 1;

            if index == 0 || (index + 1) % 10 == 0 || index + 1 == names.len() {
                self.emit_progress(
                    "message",
                    rule_number,
                    None,
                    command.line,
                    Some(command.kind.name()),
                    format!(
                        "GenerateDictionaries scanned {}/{} vocabulary plugins; {} generated",
                        index + 1,
                        names.len(),
                        generated
                    ),
                );
            }
        }

        if generated == 0 {
            Err("GenerateDictionaries did not produce any SST dictionaries".to_string())
        } else {
            Ok(generated)
        }
    }

    async fn load_masters(
        &mut self,
        rule_number: usize,
        command: &ProcessorCommand,
    ) -> Result<(), String> {
        let context = self.loaded_file_context()?;
        let data_dir = self.data_dir.clone().ok_or_else(|| {
            "LoadMasters requires the Bethesda Data directory in the run request".to_string()
        })?;
        let masters = {
            let state = self.window.state::<Arc<AppState>>();
            let esp_file = state.esp_file.lock().map_err(|e| e.to_string())?;
            esp_file
                .as_ref()
                .ok_or_else(|| "LoadMasters requires an ESP record tree".to_string())?
                .tes4
                .parse_fields()
                .masters
        };
        if masters.is_empty() {
            self.emit_progress(
                "message",
                rule_number,
                None,
                command.line,
                Some(command.kind.name()),
                "LoadMasters: current plugin declares no masters",
            );
            return Ok(());
        }

        let game = self
            .game
            .as_deref()
            .and_then(GameId::from_alias)
            .or_else(|| xt_core::esp::game_detect::detect_game_from_esp(&context.esp_path))
            .unwrap_or(GameId::SkyrimSE);
        let master_layout = if game == GameId::Starfield {
            Some(build_starfield_master_layout(&data_dir, &masters)?)
        } else {
            None
        };

        let wanted_by_master = {
            let state = self.window.state::<Arc<AppState>>();
            let strings = state.strings.lock().map_err(|e| e.to_string())?;
            let mut wanted = vec![HashSet::new(); masters.len()];
            for sk in strings.iter() {
                if sk.esp_ptr.edid_hash != 0 {
                    continue;
                }
                let form_id = sk.esp_ptr.form_id;
                if let Some(master_slot) = resolve_inherited_master_slot(
                    form_id,
                    game,
                    master_layout.as_ref(),
                    masters.len(),
                ) {
                    let (_, local_form_id) = split_form_id_identity(form_id);
                    wanted[master_slot].insert((local_form_id, sk.esp_ptr.record_sig));
                }
            }
            wanted
        };

        let mut master_edids: Vec<HashMap<(u32, [u8; 4]), String>> =
            Vec::with_capacity(masters.len());
        let mut loaded = 0usize;
        for (master_index, master_name) in masters.iter().enumerate() {
            if wanted_by_master[master_index].is_empty() {
                master_edids.push(HashMap::new());
                continue;
            }
            let master_path = data_dir.join(master_name);
            if !master_path.is_file() {
                self.add_warning(
                    format!(
                        "LoadMasters skipped {master_name}: file not found under {}",
                        data_dir.display()
                    ),
                    rule_number,
                    command,
                );
                master_edids.push(HashMap::new());
                continue;
            }

            match parse_master_edid_index(&master_path, &wanted_by_master[master_index]) {
                Ok(index) => {
                    loaded += 1;
                    master_edids.push(index);
                }
                Err(error) => {
                    self.add_warning(
                        format!("LoadMasters skipped {master_name}: {error}"),
                        rule_number,
                        command,
                    );
                    master_edids.push(HashMap::new());
                }
            }
        }

        let enriched = {
            let state = self.window.state::<Arc<AppState>>();
            let mut strings = state.strings.lock().map_err(|e| e.to_string())?;
            let mut enriched = 0usize;
            for sk in strings.iter_mut() {
                if sk.esp_ptr.edid_hash != 0 {
                    continue;
                }
                let form_id = sk.esp_ptr.form_id;
                let Some(master_slot) = resolve_inherited_master_slot(
                    form_id,
                    game,
                    master_layout.as_ref(),
                    masters.len(),
                ) else {
                    continue;
                };
                let (_, local_id) = split_form_id_identity(form_id);
                if let Some(edid) = master_edids[master_slot]
                    .get(&(local_id, sk.esp_ptr.record_sig))
                {
                    sk.edid = Some(edid.clone());
                    sk.esp_ptr.edid_hash = xt_core::types::esp_pointer::string_hash(edid);
                    enriched += 1;
                }
            }
            enriched
        };

        self.emit_progress(
            "message",
            rule_number,
            None,
            command.line,
            Some(command.kind.name()),
            format!(
                "LoadMasters parsed {loaded}/{} masters and enriched {enriched} inherited EDID references",
                masters.len()
            ),
        );
        Ok(())
    }
}

#[derive(Debug)]
struct LoadedFileContext {
    esp_path: PathBuf,
    strings_dir: Option<PathBuf>,
    is_localized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MasterPluginType {
    Normal,
    Light,
    Medium,
}

#[derive(Debug, Default)]
struct StarfieldMasterLayout {
    normal: Vec<usize>,
    medium: Vec<usize>,
    light: Vec<usize>,
}

fn read_master_plugin_type(path: &Path) -> Result<MasterPluginType, String> {
    let mut file = std::fs::File::open(path)
        .map_err(|e| format!("failed to open master {}: {e}", path.display()))?;
    let header = GenericHeader::read_from(&mut file)
        .map_err(|e| format!("failed to read TES4 header from {}: {e}", path.display()))?;
    if !header.is_tes4() {
        return Err(format!("{} does not start with a TES4 record", path.display()));
    }
    let record = RecordHeaderData::read_from(&mut file)
        .map_err(|e| format!("failed to read TES4 flags from {}: {e}", path.display()))?;
    if record.flags & 0x0000_0100 != 0 {
        Ok(MasterPluginType::Light)
    } else if record.flags & 0x0000_0400 != 0 {
        Ok(MasterPluginType::Medium)
    } else {
        // Delphi getPluginType treats overlay (0x200) as a separate type, but
        // buildInheritedData dispatches every non-light/non-medium plugin as normal.
        Ok(MasterPluginType::Normal)
    }
}

fn build_starfield_master_layout(
    data_dir: &Path,
    masters: &[String],
) -> Result<StarfieldMasterLayout, String> {
    let mut layout = StarfieldMasterLayout::default();
    for (raw_index, master) in masters.iter().enumerate() {
        let path = data_dir.join(master);
        if !path.is_file() {
            return Err(format!(
                "Starfield LoadMasters cannot reconstruct owner slots because declared master {master} is missing from {}",
                data_dir.display()
            ));
        }
        match read_master_plugin_type(&path)? {
            MasterPluginType::Normal => layout.normal.push(raw_index),
            MasterPluginType::Medium => layout.medium.push(raw_index),
            MasterPluginType::Light => layout.light.push(raw_index),
        }
    }
    Ok(layout)
}

fn resolve_inherited_master_slot(
    form_id: u32,
    game: GameId,
    starfield: Option<&StarfieldMasterLayout>,
    master_count: usize,
) -> Option<usize> {
    let high = (form_id >> 24) as u8;
    if game == GameId::Starfield {
        let layout = starfield?;
        let (owner_index, _) = split_form_id_identity(form_id);
        let raw_index = match high {
            0xFE => layout.light.get(owner_index).copied(),
            0xFD => layout.medium.get(owner_index).copied(),
            _ => layout.normal.get(owner_index).copied(),
        }?;
        (raw_index < master_count).then_some(raw_index)
    } else {
        let raw_index = high as usize;
        (raw_index < master_count).then_some(raw_index)
    }
}

struct AppStateSnapshot {
    strings: Vec<xt_core::types::sky_string::SkyString>,
    sst_old_data: Vec<xt_core::types::sky_string::SkyString>,
    file_info: Option<commands::EspFileInfo>,
    esp_file: Option<xt_core::esp::record_tree::EspFile>,
    codepage_table: Option<xt_core::strings::CodepageTable>,
    is_dirty: bool,
}

impl AppStateSnapshot {
    fn capture(window: &tauri::Window) -> Result<Self, String> {
        let state = window.state::<Arc<AppState>>();
        let strings = state.strings.lock().map_err(|e| e.to_string())?.clone();
        let sst_old_data = state
            .sst_old_data
            .lock()
            .map_err(|e| e.to_string())?
            .clone();
        let file_info = state.file_info.lock().map_err(|e| e.to_string())?.clone();
        let esp_file = state.esp_file.lock().map_err(|e| e.to_string())?.clone();
        let codepage_table = state
            .codepage_table
            .lock()
            .map_err(|e| e.to_string())?
            .clone();
        let is_dirty = *state.is_dirty.lock().map_err(|e| e.to_string())?;
        Ok(Self {
            strings,
            sst_old_data,
            file_info,
            esp_file,
            codepage_table,
            is_dirty,
        })
    }

    fn restore(self, window: &tauri::Window) -> Result<(), String> {
        let state = window.state::<Arc<AppState>>();
        *state.strings.lock().map_err(|e| e.to_string())? = self.strings;
        *state.sst_old_data.lock().map_err(|e| e.to_string())? = self.sst_old_data;
        *state.file_info.lock().map_err(|e| e.to_string())? = self.file_info;
        *state.esp_file.lock().map_err(|e| e.to_string())? = self.esp_file;
        *state.codepage_table.lock().map_err(|e| e.to_string())? = self.codepage_table;
        *state.is_dirty.lock().map_err(|e| e.to_string())? = self.is_dirty;
        Ok(())
    }
}

#[async_trait]
impl CommandProcessorHost for TauriCommandProcessorHost {
    async fn begin_rule(
        &mut self,
        _globals: &CommandProcessorGlobals,
        rule_number: usize,
        rule: &CommandRule,
    ) -> Result<(), String> {
        self.emit_progress(
            "rule_start",
            rule_number,
            None,
            rule.line,
            None,
            format!("Starting rule {rule_number}"),
        );
        Ok(())
    }

    async fn execute_command(
        &mut self,
        globals: &CommandProcessorGlobals,
        rule_number: usize,
        command_number: usize,
        rule: &CommandRule,
        command: &ProcessorCommand,
    ) -> Result<(), String> {
        let name = command.kind.name();
        self.emit_progress(
            "command_start",
            rule_number,
            Some(command_number),
            command.line,
            Some(name),
            format!("Executing {name}"),
        );

        let result = match &command.kind {
            ProcessorCommandKind::LoadFile { path } => self.load_file(rule, path).await,
            ProcessorCommandKind::CloseFile | ProcessorCommandKind::CloseAll => {
                self.close_loaded_file()
            }
            ProcessorCommandKind::Finalize => self.finalize_rule(globals, rule).await,
            ProcessorCommandKind::ApplySst {
                compare_option,
                apply_mode,
                path,
            } => match self.resolve_apply_sst_path(globals, rule, path) {
                Ok(path) => self.apply_sst(path, *compare_option, *apply_mode).await,
                Err(error) => Err(error),
            },
            ProcessorCommandKind::ImportSst {
                compare_option,
                apply_mode,
                path,
            } => match self.resolve_import_path(globals, path) {
                Ok(path) => self.apply_sst(path, *compare_option, *apply_mode).await,
                Err(error) => Err(error),
            },
            ProcessorCommandKind::ImportXml {
                compare_option,
                apply_mode,
                path,
            } => match self.resolve_import_path(globals, path) {
                Ok(path) => {
                    let result = self.import_xml(path).await;
                    if result.is_ok() {
                        self.add_warning(
                            format!(
                                "rule {rule_number}, line {} ImportXml:{compare_option}:{apply_mode}: \
                                 XML import currently uses the Rust T1-T4 matcher; Delphi processor \
                                 comparator modes will be closed with DP-09 XML metadata parity",
                                command.line
                            ),
                            rule_number,
                            command,
                        );
                    }
                    result
                }
                Err(error) => Err(error),
            },
            ProcessorCommandKind::SaveDictionary => self.save_dictionary(globals, rule).await,
            ProcessorCommandKind::GenerateDictionaries => {
                self.generate_dictionaries(globals, rule_number, rule, command)
                    .await
            }
            ProcessorCommandKind::LoadMasters => self.load_masters(rule_number, command).await,
            ProcessorCommandKind::ApiTranslation {
                api_id,
                auto_no_trans_tag,
            } => {
                self.api_translation(
                    rule_number,
                    rule,
                    command,
                    *api_id,
                    *auto_no_trans_tag,
                )
                .await
            }
        };

        self.emit_progress(
            "command_done",
            rule_number,
            Some(command_number),
            command.line,
            Some(name),
            match &result {
                Ok(()) => format!("{name} completed"),
                Err(error) => format!("{name} failed: {error}"),
            },
        );
        result
    }

    async fn end_rule(
        &mut self,
        _globals: &CommandProcessorGlobals,
        rule_number: usize,
        rule: &CommandRule,
    ) -> Result<(), String> {
        self.emit_progress(
            "rule_done",
            rule_number,
            None,
            rule.line,
            None,
            format!("Finished rule {rule_number}"),
        );
        Ok(())
    }
}

#[tauri::command]
pub async fn run_command_processor(
    window: tauri::Window,
    request: CommandProcessorRunRequest,
) -> Result<CommandProcessorRunResponse, String> {
    let script = parse_command_processor(&request.script)
        .map_err(|error| format!("Command processor parse error: {error}"))?;
    let error_policy = match request.error_policy {
        CommandProcessorErrorPolicyDto::Stop => CommandErrorPolicy::Stop,
        CommandProcessorErrorPolicyDto::Continue => CommandErrorPolicy::Continue,
    };

    let mut host = TauriCommandProcessorHost::new(window, &request);
    let report = execute_command_processor(&script, &mut host, error_policy).await;

    Ok(CommandProcessorRunResponse {
        rules_started: report.rules_started,
        rules_completed: report.rules_completed,
        commands_succeeded: report.commands_succeeded,
        failures: report
            .failures
            .into_iter()
            .map(|failure| CommandProcessorFailureDto {
                rule_number: failure.rule_number,
                command_number: failure.command_number,
                line: failure.line,
                command: failure.command.map(str::to_string),
                message: failure.message,
            })
            .collect(),
        warnings: host.warnings,
        file_context_changed: host.file_context_changed,
        active_file: host.active_file,
        stopped_early: report.stopped_early,
    })
}

fn processor_sst_options(
    compare_option: u8,
    apply_mode: u8,
) -> Result<SstApplyOptionsDto, String> {
    let overwrite_scope = match compare_option {
        0 => SstOverwriteScopeDto::All,
        1 => SstOverwriteScopeDto::NoTransExclusive,
        2 => SstOverwriteScopeDto::NoTransAndPartial,
        3 => SstOverwriteScopeDto::PartialOnly,
        4 => SstOverwriteScopeDto::Selection,
        value => {
            return Err(format!(
                "invalid processor compare option {value}; expected 0..=4"
            ))
        }
    };
    let match_mode = match apply_mode {
        0 => SstMatchModeDto::FormIdOnly,
        1 => SstMatchModeDto::FormIdStrictString,
        2 => SstMatchModeDto::FormIdRelaxedString,
        3 => SstMatchModeDto::StringOnly,
        value => {
            return Err(format!(
                "invalid processor apply mode {value}; expected 0..=3"
            ))
        }
    };

    Ok(SstApplyOptionsDto {
        overwrite_scope,
        match_mode,
        tag_only: false,
        reset_state: false,
        restrict_to_filter: false,
        selected_ids: None,
        filtered_ids: None,
    })
}

fn delphi_sst_filename(path: &str, source: &str, dest: &str) -> Result<String, String> {
    let stem = Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| format!("cannot derive SST name from path: {path}"))?;
    Ok(format!("{stem}_{source}_{dest}.sst"))
}

fn delphi_api_provider(api_id: u8) -> Result<&'static str, String> {
    // Delphi aApiBaseName order:
    // 0 MsTranslate, 1 Yandex, 2 Baidu, 3 Youdao, 4 freeApi,
    // 5 Google, 6 DeepL, 7 OpenAI.
    // The Rust Azure provider is the equivalent modern Microsoft Translator adapter.
    match api_id {
        0 => Ok("azure"),
        2 => Ok("baidu"),
        3 => Ok("youdao"),
        5 => Ok("google"),
        6 => Ok("deepl"),
        7 => Ok("openai"),
        1 => Err("Delphi API id 1 (Yandex) is not implemented in the Rust rewrite".to_string()),
        4 => Err("Delphi API id 4 (freeApi) is not implemented in the Rust rewrite".to_string()),
        value => Err(format!("invalid Delphi API id {value}; expected 0..=7")),
    }
}

fn find_game_plugin(data_dir: &Path, base_name: &str) -> Option<PathBuf> {
    for ext in ["esm", "esl", "esp"] {
        let candidate = data_dir.join(format!("{base_name}.{ext}"));
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    let expected = [
        format!("{base_name}.esm").to_ascii_lowercase(),
        format!("{base_name}.esl").to_ascii_lowercase(),
        format!("{base_name}.esp").to_ascii_lowercase(),
    ];
    std::fs::read_dir(data_dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| expected.contains(&name.to_ascii_lowercase()))
                .unwrap_or(false)
        })
}

fn parse_master_edid_index(
    path: &Path,
    wanted: &HashSet<(u32, [u8; 4])>,
) -> Result<HashMap<(u32, [u8; 4]), String>, String> {
    xt_core::esp::parser::scan_selected_record_edids(path, wanted)
        .map(|records| {
            records
                .into_iter()
                .map(|((form_id, sig), edid)| {
                    let (_, local_form_id) = split_form_id_identity(form_id);
                    ((local_form_id, sig), edid)
                })
                .collect()
        })
        .map_err(|e| format!("failed to scan master {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn processor_numeric_options_map_to_delphi_dp03_matrix() {
        let opts = processor_sst_options(2, 3).expect("valid Delphi options");
        assert!(matches!(
            opts.overwrite_scope,
            SstOverwriteScopeDto::NoTransAndPartial
        ));
        assert!(matches!(opts.match_mode, SstMatchModeDto::StringOnly));
        assert!(processor_sst_options(5, 1).is_err());
        assert!(processor_sst_options(0, 4).is_err());
    }

    #[test]
    fn sst_filename_matches_delphi_language_suffix_shape() {
        assert_eq!(
            delphi_sst_filename(r"C:\Games\Data\Example.esp", "english", "chinese")
                .expect("filename"),
            "Example_english_chinese.sst"
        );
    }

    #[test]
    fn delphi_api_ids_map_to_current_providers() {
        assert_eq!(delphi_api_provider(0).unwrap(), "azure");
        assert_eq!(delphi_api_provider(2).unwrap(), "baidu");
        assert_eq!(delphi_api_provider(3).unwrap(), "youdao");
        assert_eq!(delphi_api_provider(5).unwrap(), "google");
        assert_eq!(delphi_api_provider(6).unwrap(), "deepl");
        assert_eq!(delphi_api_provider(7).unwrap(), "openai");
        assert!(delphi_api_provider(1).is_err());
        assert!(delphi_api_provider(4).is_err());
        assert!(delphi_api_provider(8).is_err());
    }

    #[test]
    fn find_game_plugin_prefers_delphi_extension_order() {
        let temp = std::env::temp_dir().join(format!(
            "xtranslator-command-processor-plugin-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).unwrap();
        std::fs::write(temp.join("Example.esp"), b"").unwrap();
        std::fs::write(temp.join("Example.esm"), b"").unwrap();

        assert_eq!(
            find_game_plugin(&temp, "Example")
                .unwrap()
                .extension()
                .and_then(|ext| ext.to_str()),
            Some("esm")
        );

        let _ = std::fs::remove_dir_all(temp);
    }

    #[test]
    fn starfield_master_slots_follow_delphi_type_buckets() {
        let layout = StarfieldMasterLayout {
            normal: vec![0, 2, 5],
            medium: vec![3],
            light: vec![1, 4],
        };

        assert_eq!(
            resolve_inherited_master_slot(0x0100_1234, GameId::Starfield, Some(&layout), 6),
            Some(2)
        );
        assert_eq!(
            resolve_inherited_master_slot(0xFE00_1ABC, GameId::Starfield, Some(&layout), 6),
            Some(4)
        );
        assert_eq!(
            resolve_inherited_master_slot(0xFD00_BEEF, GameId::Starfield, Some(&layout), 6),
            Some(3)
        );
        assert_eq!(
            resolve_inherited_master_slot(0x0600_1234, GameId::Starfield, Some(&layout), 6),
            None
        );
    }

    #[test]
    fn non_starfield_master_slots_keep_delphi_high_byte_rule() {
        assert_eq!(
            resolve_inherited_master_slot(0x0200_1234, GameId::SkyrimSE, None, 4),
            Some(2)
        );
        assert_eq!(
            resolve_inherited_master_slot(0xFE00_1234, GameId::SkyrimSE, None, 4),
            None
        );
    }

    #[test]
    fn master_plugin_type_reads_delphi_tes4_flags() {
        fn write_tes4(path: &Path, flags: u32) {
            let mut bytes = Vec::new();
            bytes.extend_from_slice(b"TES4");
            bytes.extend_from_slice(&0u32.to_le_bytes());
            bytes.extend_from_slice(&flags.to_le_bytes());
            bytes.extend_from_slice(&0u32.to_le_bytes());
            bytes.extend_from_slice(&0u32.to_le_bytes());
            bytes.extend_from_slice(&0u16.to_le_bytes());
            bytes.extend_from_slice(&0u16.to_le_bytes());
            std::fs::write(path, bytes).unwrap();
        }

        let temp = std::env::temp_dir().join(format!(
            "xtranslator-master-flags-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).unwrap();
        let normal = temp.join("normal.esm");
        let light = temp.join("light.esm");
        let medium = temp.join("medium.esm");
        write_tes4(&normal, 0);
        write_tes4(&light, 0x0000_0100);
        write_tes4(&medium, 0x0000_0400);

        assert_eq!(read_master_plugin_type(&normal).unwrap(), MasterPluginType::Normal);
        assert_eq!(read_master_plugin_type(&light).unwrap(), MasterPluginType::Light);
        assert_eq!(read_master_plugin_type(&medium).unwrap(), MasterPluginType::Medium);

        let _ = std::fs::remove_dir_all(temp);
    }
}
