use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::Emitter;
use xt_core::config::AppConfig;
use xt_core::esp::parser::{EspParser, StringsFiles};
use xt_core::strings::CodepageTable;
use xt_core::translation_api::{BaiduProvider, DeepLProvider, OpenAIProvider, ProviderType, TranslationProvider, YoudaoProvider};
use xt_core::types::game_id::GameId;
use xt_core::types::params::SkyStringParams;
use xt_core::types::sky_string::SkyString;
use xt_shared::dto::{
    BatchComplete, BatchEntry, BatchFileComplete, BatchFileError, BatchProgress, BatchStatus,
};

/// 批处理运行状态
enum BatchJobState {
    Idle,
    #[allow(dead_code)]
    Running {
        job_id: String,
        job_type: String,
        started_at: std::time::Instant,
        entries: Vec<BatchEntry>,
        provider: ProviderType,
        target_lang: String,
        skip_translated: bool,
    },
    Done {
        result: BatchComplete,
    },
}

/// 批处理器 — 后台顺序处理 ESP 文件，独立于 AppState
pub struct BatchExecutor {
    state: Mutex<BatchJobState>,
    pub cancel_flag: Arc<AtomicBool>,
}

impl BatchExecutor {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(BatchJobState::Idle),
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 检查是否空闲
    #[allow(dead_code)]
    pub fn is_idle(&self) -> bool {
        self.state
            .lock()
            .map_or(false, |s| matches!(&*s, BatchJobState::Idle))
    }

    /// 获取当前批处理状态摘要
    pub fn get_status(&self) -> Option<BatchStatus> {
        let state = self.state.lock().ok()?;
        match &*state {
            BatchJobState::Idle => None,
            BatchJobState::Running {
                job_id,
                job_type,
                started_at,
                entries,
                provider: _,
                target_lang: _,
                skip_translated: _,
            } => Some(BatchStatus {
                job_id: job_id.clone(),
                job_type: job_type.clone(),
                total_files: entries.len() as u32,
                completed_files: 0,
                failed_files: 0,
                current_file: None,
                current_file_progress: 0.0,
                total_strings: 0,
                translated_strings: 0,
                is_running: true,
                is_cancelled: false,
                is_completed: false,
                is_failed: false,
                errors: Vec::new(),
                elapsed_ms: started_at.elapsed().as_millis() as u64,
            }),
            BatchJobState::Done { result } => Some(BatchStatus {
                job_id: result.job_id.clone(),
                job_type: String::new(),
                total_files: result.total_files,
                completed_files: result.success,
                failed_files: result.failed,
                current_file: None,
                current_file_progress: 1.0,
                total_strings: result.total_translated,
                translated_strings: result.total_translated,
                is_running: false,
                is_cancelled: result.is_cancelled,
                is_completed: !result.is_cancelled,
                is_failed: result.failed == result.total_files && result.total_files > 0,
                errors: result
                    .errors
                    .iter()
                    .map(|e| format!("{}: {}", e.file_path, e.message))
                    .collect(),
                elapsed_ms: result.duration_ms,
            }),
        }
    }

    /// 取消当前批处理
    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
    }

    /// 启动翻译批处理（异步）
    pub fn start_translate(
        self: &Arc<Self>,
        window: tauri::Window,
        entries: Vec<BatchEntry>,
        provider: ProviderType,
        target_lang: String,
        skip_translated: bool,
    ) -> Result<String, String> {
        // 验证所有入口
        validate_entries(&entries)?;

        {
            let mut s = self.state.lock().map_err(|e| e.to_string())?;
            if matches!(&*s, BatchJobState::Running { .. }) {
                return Err("A batch job is already running".to_string());
            }
            let job_id = format!(
                "batch_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0)
            );
            *s = BatchJobState::Running {
                job_id: job_id.clone(),
                job_type: "translate".to_string(),
                started_at: std::time::Instant::now(),
                entries: entries.clone(),
                provider,
                target_lang: target_lang.clone(),
                skip_translated,
            };
        }

        // 重置取消标志
        self.cancel_flag.store(false, Ordering::SeqCst);

        let executor = self.clone();
        let window_clone = window.clone();

        tokio::spawn(async move {
            run_batch_translate(
                executor,
                window_clone,
                entries,
                provider,
                target_lang,
                skip_translated,
            )
            .await;
        });

        // 返回 job_id
        let s = self.state.lock().map_err(|e| e.to_string())?;
        match &*s {
            BatchJobState::Running { job_id, .. } => Ok(job_id.clone()),
            _ => Err("Failed to start batch job".to_string()),
        }
    }

    /// 启动导出批处理
    pub fn start_export(
        self: &Arc<Self>,
        window: tauri::Window,
        entries: Vec<BatchEntry>,
        output_dir: String,
        export_format: String,
    ) -> Result<String, String> {
        // 验证所有入口
        validate_entries(&entries)?;

        {
            let mut s = self.state.lock().map_err(|e| e.to_string())?;
            if matches!(&*s, BatchJobState::Running { .. }) {
                return Err("A batch job is already running".to_string());
            }
            let job_id = format!(
                "batch_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0)
            );
            *s = BatchJobState::Running {
                job_id: job_id.clone(),
                job_type: "export".to_string(),
                started_at: std::time::Instant::now(),
                entries: entries.clone(),
                provider: ProviderType::OpenAI, // unused for export
                target_lang: "english".to_string(), // unused for export
                skip_translated: false,         // unused for export
            };
        }

        self.cancel_flag.store(false, Ordering::SeqCst);
        let executor = self.clone();

        tokio::spawn(async move {
            run_batch_export(executor, window, entries, output_dir, export_format).await;
        });

        let s = self.state.lock().map_err(|e| e.to_string())?;
        match &*s {
            BatchJobState::Running { job_id, .. } => Ok(job_id.clone()),
            _ => Err("Failed to start batch job".to_string()),
        }
    }
}

// ── 入口验证 ─────────────────────────────────────────────

/// 验证批处理入口：检查 ESP 文件存在、strings 目录可用
fn validate_entries(entries: &[BatchEntry]) -> Result<(), String> {
    for entry in entries {
        // 检查 ESP 文件存在
        if !std::path::Path::new(&entry.esp_path).exists() {
            return Err(format!("ESP file not found: {}", entry.esp_path));
        }

        // 检查 strings 目录（如果指定）
        if let Some(ref strings_dir) = entry.strings_dir {
            let dir = std::path::Path::new(strings_dir);
            if !dir.exists() {
                // 尝试在 ESP 所在目录的 ../Strings 查找
                if let Some(parent) = std::path::Path::new(&entry.esp_path).parent() {
                    let fallback = parent.join("..").join("Strings");
                    if !fallback.exists() {
                        return Err(format!(
                            "Strings directory not found: {} (also tried {})",
                            strings_dir,
                            fallback.display()
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

// ── 语言映射（前端全名 → API 代码） ─────────────────────────────

fn lang_to_api_code(lang: &str) -> &str {
    match lang.to_lowercase().as_str() {
        "english" => "EN",
        "chinese" => "ZH",
        "japanese" => "JA",
        "korean" => "KO",
        "french" => "FR",
        "german" => "DE",
        "spanish" => "ES",
        "italian" => "IT",
        "russian" => "RU",
        "polish" => "PL",
        "portuguese" | "brazilian" => "PT",
        "czech" => "CS",
        "hungarian" => "HU",
        // DeepL specific: PT-PT vs PT-BR
        _ => "EN",
    }
}

// ── 游戏探测 ─────────────────────────────────────────────────

fn detect_game_from_path(esp_path: &str) -> GameId {
    let lower = esp_path.to_lowercase();
    if lower.contains("fallout4") || lower.contains("fo4") {
        GameId::Fallout4
    } else if lower.contains("starfield") {
        GameId::Starfield
    } else if lower.contains("falloutnv") || lower.contains("fonv") {
        GameId::FalloutNV
    } else if lower.contains("fallout76") || lower.contains("fo76") {
        GameId::Fallout76
    } else if lower.contains("skyrim") {
        GameId::SkyrimSE
    } else {
        GameId::SkyrimSE
    }
}

fn parse_game_override(game: &str) -> Option<GameId> {
    match game.to_lowercase().as_str() {
        "skyrim" => Some(GameId::Skyrim),
        "skyrimse" | "skyrim se" => Some(GameId::SkyrimSE),
        "fallout4" | "fo4" => Some(GameId::Fallout4),
        "falloutnv" | "fonv" => Some(GameId::FalloutNV),
        "fallout76" | "fo76" => Some(GameId::Fallout76),
        "starfield" | "sf" => Some(GameId::Starfield),
        _ => None,
    }
}

// ── ESP 解析（独立于 AppState） ───────────────────────────────

fn parse_esp_for_batch(
    esp_path: &str,
    strings_dir: Option<&str>,
    language: &str,
    game_override: Option<&str>,
) -> Result<(Vec<SkyString>, u32), String> {
    let game_id = game_override
        .and_then(parse_game_override)
        .unwrap_or_else(|| detect_game_from_path(esp_path));

    let data_dir = std::path::Path::new("Data");
    let mut parser = EspParser::with_game(data_dir, game_id).unwrap_or_else(|_| EspParser::new());

    let lang = language;
    let base_name = std::path::Path::new(esp_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("skyrim");

    // Codepage
    let codepage_path = data_dir
        .join(match game_id {
            GameId::Skyrim => "Skyrim",
            GameId::SkyrimSE => "SkyrimSE",
            GameId::Fallout4 => "Fallout4",
            GameId::FalloutNV => "FalloutNV",
            GameId::Fallout76 => "Fallout76",
            GameId::Starfield => "Starfield",
        })
        .join("codepage.txt");

    let codepage_table = if codepage_path.exists() {
        CodepageTable::load_from_file(&codepage_path).ok()
    } else {
        None
    };

    // 加载 strings 文件
    let esp_dir = std::path::Path::new(esp_path)
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));

    // 优先使用指定的 strings_dir
    let strings_loaded = if let Some(dir) = strings_dir {
        let dir_path = std::path::Path::new(dir);
        if let Some(ref table) = codepage_table {
            parser.strings_files =
                StringsFiles::load_from_dir_with_language(dir_path, base_name, lang, table);
        } else {
            parser.load_strings_files(dir_path, base_name);
        }
        parser.strings_files.loaded_count()
    } else {
        0
    };

    if strings_loaded == 0 {
        if let Some(ref table) = codepage_table {
            parser.strings_files =
                StringsFiles::load_from_dir_with_language(esp_dir, base_name, lang, table);
        } else {
            parser.load_strings_files(esp_dir, base_name);
        }
    }

    // 解析 ESP
    let mut file = std::fs::File::open(esp_path)
        .map_err(|e| format!("Failed to open ESP {}: {}", esp_path, e))?;

    parser
        .parse(&mut file)
        .map_err(|e| format!("Failed to parse ESP {}: {}", esp_path, e))?;

    let total = parser.strings.len() as u32;
    Ok((parser.strings, total))
}

// ── 翻译循环 ──────────────────────────────────────────────────

async fn run_batch_translate(
    executor: Arc<BatchExecutor>,
    window: tauri::Window,
    entries: Vec<BatchEntry>,
    provider: ProviderType,
    target_lang: String,
    skip_translated: bool,
) {
    let started_at = std::time::Instant::now();
    let total_files = entries.len() as u32;
    let mut completed_files = 0u32;
    let mut failed_files = 0u32;
    let mut total_translated = 0u32;
    let mut total_errors = 0u32;
    let mut batch_errors: Vec<BatchFileError> = Vec::new();

    // 读取 API key（环境变量）
    let api_key = match provider {
        ProviderType::OpenAI => std::env::var("XT_TRANSLATE_API_KEY").unwrap_or_default(),
        ProviderType::DeepL => std::env::var("XT_DEEPL_API_KEY").unwrap_or_default(),
        ProviderType::Baidu => std::env::var("XT_BAIDU_API_APP_ID").unwrap_or_default(),
        ProviderType::Youdao => std::env::var("XT_YOUDAO_API_APP_KEY").unwrap_or_default(),
    };

    let source_api_code = "EN";
    let target_api_code = lang_to_api_code(&target_lang);

    for (i, entry) in entries.iter().enumerate() {
        // 取消检查
        if executor.cancel_flag.load(Ordering::SeqCst) {
            let _ = window.emit(
                "batch-progress",
                make_progress(
                    "batch-progress",
                    "",
                    "",
                    &format!("Cancelled after {}/{} files", completed_files, total_files),
                ),
            );
            break;
        }

        let file_name = std::path::Path::new(&entry.esp_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&entry.esp_path)
            .to_string();

        let language = entry
            .language
            .clone()
            .unwrap_or_else(|| "english".to_string());

        // 阶段1: 解析 ESP
        let _ = window.emit(
            "batch-progress",
            BatchProgress {
                job_id: String::new(),
                file_path: entry.esp_path.clone(),
                stage: "parsing".to_string(),
                current_file: (i as u32) + 1,
                total_files,
                strings_translated: total_translated,
                total_strings: 0,
                message: format!("[{}/{}] Parsing {}...", i + 1, total_files, file_name),
            },
        );

        let parse_result = tokio::task::spawn_blocking({
            let esp_path = entry.esp_path.clone();
            let strings_dir = entry.strings_dir.clone();
            let game_override = entry.game.clone();
            let lang = language.clone();
            move || {
                parse_esp_for_batch(
                    &esp_path,
                    strings_dir.as_deref(),
                    &lang,
                    game_override.as_deref(),
                )
            }
        })
        .await;

        let (mut strings, _total_count) = match parse_result {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => {
                failed_files += 1;
                batch_errors.push(BatchFileError {
                    file_path: entry.esp_path.clone(),
                    message: e.clone(),
                });
                let _ = window.emit(
                    "batch-file-complete",
                    BatchFileComplete {
                        job_id: String::new(),
                        file_path: entry.esp_path.clone(),
                        translated: 0,
                        skipped: 0,
                        errors: 1,
                        duration_ms: started_at.elapsed().as_millis() as u64,
                    },
                );
                continue;
            }
            Err(e) => {
                failed_files += 1;
                batch_errors.push(BatchFileError {
                    file_path: entry.esp_path.clone(),
                    message: format!("Task error: {}", e),
                });
                continue;
            }
        };

        // 阶段2: 翻译
        // 筛选需要翻译的字符串
        let to_translate: Vec<(usize, String)> = strings
            .iter()
            .enumerate()
            .filter(|(_, sk)| {
                if skip_translated && sk.params.is_translated() {
                    return false;
                }
                !sk.source.is_empty()
            })
            .map(|(idx, sk)| (idx, sk.source.clone()))
            .collect();

        let total_to_translate = to_translate.len() as u32;
        let mut file_translated = 0u32;
        let mut file_errors = 0u32;

        let _ = window.emit(
            "batch-progress",
            BatchProgress {
                job_id: String::new(),
                file_path: entry.esp_path.clone(),
                stage: "translating".to_string(),
                current_file: (i as u32) + 1,
                total_files,
                strings_translated: total_translated,
                total_strings: total_to_translate,
                message: format!(
                    "[{}/{}] Translating {} ({}/{})",
                    i + 1,
                    total_files,
                    file_name,
                    0,
                    total_to_translate
                ),
            },
        );

        for (j, (idx, source_text)) in to_translate.iter().enumerate() {
            // 取消检查
            if executor.cancel_flag.load(Ordering::SeqCst) {
                break;
            }

            let result = match provider {
                ProviderType::OpenAI => {
                    if api_key.is_empty() {
                        Err("OpenAI API key not set".to_string())
                    } else {
                        let proxy_config = AppConfig::load(&crate::commands::config_dir()).ok();
                        let provider = OpenAIProvider::from_key(api_key.clone());
                        provider
                            .translate(source_text, source_api_code, target_api_code, proxy_config.as_ref())
                            .await
                            .map_err(|e| e.to_string())
                    }
                }
                ProviderType::DeepL => {
                    if api_key.is_empty() {
                        Err("DeepL API key not set".to_string())
                    } else {
                        let proxy_config = AppConfig::load(&crate::commands::config_dir()).ok();
                        let provider = DeepLProvider::new(api_key.clone());
                        provider
                            .translate(source_text, source_api_code, target_api_code, proxy_config.as_ref())
                            .await
                            .map_err(|e| e.to_string())
                    }
                }
                ProviderType::Baidu => {
                    let config = AppConfig::load(&crate::commands::config_dir()).ok();
                    let app_id = config.as_ref().and_then(|c| c.baidu_app_id.clone()).unwrap_or_default();
                    let key = config.as_ref().and_then(|c| c.baidu_key.clone()).unwrap_or_default();
                    if app_id.is_empty() || key.is_empty() {
                        Err("Baidu AppId/Key not set".to_string())
                    } else {
                        let proxy_config = config;
                        let provider = BaiduProvider::new(app_id, key);
                        provider
                            .translate(source_text, source_api_code, target_api_code, proxy_config.as_ref())
                            .await
                            .map_err(|e| e.to_string())
                    }
                }
                ProviderType::Youdao => {
                    let config = AppConfig::load(&crate::commands::config_dir()).ok();
                    let app_key = config.as_ref().and_then(|c| c.youdao_app_key.clone()).unwrap_or_default();
                    let secret_key = config.as_ref().and_then(|c| c.youdao_secret_key.clone()).unwrap_or_default();
                    if app_key.is_empty() || secret_key.is_empty() {
                        Err("Youdao AppKey/SecretKey not set".to_string())
                    } else {
                        let proxy_config = config;
                        let provider = YoudaoProvider::new(app_key, secret_key);
                        provider
                            .translate(source_text, source_api_code, target_api_code, proxy_config.as_ref())
                            .await
                            .map_err(|e| e.to_string())
                    }
                }
            };

            match result {
                Ok(translated) => {
                    strings[*idx].set_translation(translated);
                    strings[*idx].params.set(SkyStringParams::TRANSLATED, true);
                    strings[*idx]
                        .params
                        .set(SkyStringParams::INCOMPLETE_TRANS, false);
                    file_translated += 1;
                    total_translated += 1;
                }
                Err(_e) => {
                    file_errors += 1;
                    total_errors += 1;
                }
            }

            // 每10条或最后一条发送进度
            if j % 10 == 0 || j == total_to_translate as usize - 1 {
                let _ = window.emit(
                    "batch-progress",
                    BatchProgress {
                        job_id: String::new(),
                        file_path: entry.esp_path.clone(),
                        stage: "translating".to_string(),
                        current_file: (i as u32) + 1,
                        total_files,
                        strings_translated: total_translated,
                        total_strings: total_to_translate,
                        message: format!(
                            "[{}/{}] Translating {} ({}/{})",
                            i + 1,
                            total_files,
                            file_name,
                            j + 1,
                            total_to_translate
                        ),
                    },
                );
            }
        }

        if executor.cancel_flag.load(Ordering::SeqCst) {
            let _ = window.emit(
                "batch-file-complete",
                BatchFileComplete {
                    job_id: String::new(),
                    file_path: entry.esp_path.clone(),
                    translated: file_translated,
                    skipped: total_to_translate - file_translated - file_errors,
                    errors: file_errors,
                    duration_ms: started_at.elapsed().as_millis() as u64,
                },
            );
            break;
        }

        // 阶段3: 保存 strings 文件
        let _ = window.emit(
            "batch-progress",
            BatchProgress {
                job_id: String::new(),
                file_path: entry.esp_path.clone(),
                stage: "saving".to_string(),
                current_file: (i as u32) + 1,
                total_files,
                strings_translated: total_translated,
                total_strings: total_to_translate,
                message: format!("[{}/{}] Saving {}...", i + 1, total_files, file_name),
            },
        );

        let save_result = tokio::task::spawn_blocking({
            let esp_path = entry.esp_path.clone();
            let strings_dir = entry.strings_dir.clone();
            let lang = language.to_string();
            let target = target_lang.clone();
            let strings_snapshot = strings.clone();
            move || {
                save_strings_for_batch(
                    &esp_path,
                    strings_dir.as_deref(),
                    &lang,
                    &target,
                    &strings_snapshot,
                )
            }
        })
        .await;

        match save_result {
            Ok(Ok(_)) => {
                completed_files += 1;
                let _ = window.emit(
                    "batch-file-complete",
                    BatchFileComplete {
                        job_id: String::new(),
                        file_path: entry.esp_path.clone(),
                        translated: file_translated,
                        skipped: total_to_translate - file_translated - file_errors,
                        errors: file_errors,
                        duration_ms: started_at.elapsed().as_millis() as u64,
                    },
                );
            }
            Ok(Err(e)) => {
                failed_files += 1;
                batch_errors.push(BatchFileError {
                    file_path: entry.esp_path.clone(),
                    message: e,
                });
                let _ = window.emit(
                    "batch-file-complete",
                    BatchFileComplete {
                        job_id: String::new(),
                        file_path: entry.esp_path.clone(),
                        translated: file_translated,
                        skipped: total_to_translate - file_translated - file_errors,
                        errors: file_errors + 1,
                        duration_ms: started_at.elapsed().as_millis() as u64,
                    },
                );
            }
            Err(e) => {
                failed_files += 1;
                batch_errors.push(BatchFileError {
                    file_path: entry.esp_path.clone(),
                    message: format!("Save task error: {}", e),
                });
            }
        }
    }

    let duration_ms = started_at.elapsed().as_millis() as u64;
    let is_cancelled = executor.cancel_flag.load(Ordering::SeqCst);

    let result = BatchComplete {
        job_id: String::new(),
        total_files,
        success: completed_files,
        failed: failed_files,
        total_translated,
        total_errors,
        duration_ms,
        is_cancelled,
        errors: batch_errors,
    };

    let _ = window.emit("batch-complete", result.clone());

    // 更新状态
    if let Ok(mut state) = executor.state.lock() {
        if matches!(&*state, BatchJobState::Running { .. }) {
            *state = BatchJobState::Done { result };
        }
    }
}

// ── 导出循环 ──────────────────────────────────────────────────

async fn run_batch_export(
    executor: Arc<BatchExecutor>,
    window: tauri::Window,
    entries: Vec<BatchEntry>,
    output_dir: String,
    export_format: String, // "xml" | "sst"
) {
    let started_at = std::time::Instant::now();
    let total_files = entries.len() as u32;
    let mut completed_files = 0u32;
    let mut failed_files = 0u32;
    let mut total_exported = 0u32;
    let mut total_errors = 0u32;
    let mut batch_errors: Vec<BatchFileError> = Vec::new();

    for (i, entry) in entries.iter().enumerate() {
        if executor.cancel_flag.load(Ordering::SeqCst) {
            break;
        }

        let file_name = std::path::Path::new(&entry.esp_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&entry.esp_path)
            .to_string();

        let language = entry
            .language
            .clone()
            .unwrap_or_else(|| "english".to_string());

        let _ = window.emit(
            "batch-progress",
            BatchProgress {
                job_id: String::new(),
                file_path: entry.esp_path.clone(),
                stage: "parsing".to_string(),
                current_file: (i as u32) + 1,
                total_files,
                strings_translated: total_exported,
                total_strings: 0,
                message: format!("[{}/{}] Parsing {}...", i + 1, total_files, file_name),
            },
        );

        let parse_result = tokio::task::spawn_blocking({
            let esp_path = entry.esp_path.clone();
            let strings_dir = entry.strings_dir.clone();
            let game_override = entry.game.clone();
            let lang = language.clone();
            move || {
                parse_esp_for_batch(
                    &esp_path,
                    strings_dir.as_deref(),
                    &lang,
                    game_override.as_deref(),
                )
            }
        })
        .await;

        let (strings, _total_count) = match parse_result {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => {
                failed_files += 1;
                batch_errors.push(BatchFileError {
                    file_path: entry.esp_path.clone(),
                    message: e,
                });
                continue;
            }
            Err(e) => {
                failed_files += 1;
                batch_errors.push(BatchFileError {
                    file_path: entry.esp_path.clone(),
                    message: format!("Task error: {}", e),
                });
                continue;
            }
        };

        match export_format.as_str() {
            "sst" => {
                let base_name_sst = std::path::Path::new(&entry.esp_path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("export")
                    .to_string();
                let sst_path =
                    std::path::Path::new(&output_dir).join(format!("{}.sst", &base_name_sst));

                let save_result = tokio::task::spawn_blocking(move || {
                    let dict = xt_core::sst::v8::SstDictionary::from_entries(strings);
                    dict.save_to_file(&sst_path)
                        .map_err(|e| format!("Failed to save SST: {}", e))
                })
                .await;

                match save_result {
                    Ok(Ok(())) => {
                        completed_files += 1;
                        total_exported += 1;
                    }
                    Ok(Err(e)) => {
                        failed_files += 1;
                        total_errors += 1;
                        batch_errors.push(BatchFileError {
                            file_path: entry.esp_path.clone(),
                            message: e,
                        });
                    }
                    Err(e) => {
                        failed_files += 1;
                        batch_errors.push(BatchFileError {
                            file_path: entry.esp_path.clone(),
                            message: format!("SST task error: {}", e),
                        });
                    }
                }
            }
            _ => {
                // XML export
                let base_name_owned = std::path::Path::new(&entry.esp_path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("export")
                    .to_string();
                let xml_path =
                    std::path::Path::new(&output_dir).join(format!("{}.xml", &base_name_owned));

                let save_result = tokio::task::spawn_blocking(move || {
                    let xml_entries = xt_core::xml::sky_strings_to_xml_entries(&strings);
                    let params = xt_core::xml::XmlExportParams {
                        addon: base_name_owned,
                        source_lang: language.to_string(),
                        dest_lang: "translated".to_string(),
                        version: 2,
                    };
                    xt_core::xml::write_xml_file(&xml_path, &params, &xml_entries)
                        .map_err(|e| format!("Failed to write XML: {}", e))?;
                    Ok(xml_entries.len())
                })
                .await;

                match save_result {
                    Ok(Ok(count)) => {
                        completed_files += 1;
                        total_exported += count as u32;
                    }
                    Ok(Err(e)) => {
                        failed_files += 1;
                        total_errors += 1;
                        batch_errors.push(BatchFileError {
                            file_path: entry.esp_path.clone(),
                            message: e,
                        });
                    }
                    Err(e) => {
                        failed_files += 1;
                        batch_errors.push(BatchFileError {
                            file_path: entry.esp_path.clone(),
                            message: format!("XML task error: {}", e),
                        });
                    }
                }
            }
        }

        let _ = window.emit(
            "batch-file-complete",
            BatchFileComplete {
                job_id: String::new(),
                file_path: entry.esp_path.clone(),
                translated: 0,
                skipped: 0,
                errors: 0,
                duration_ms: started_at.elapsed().as_millis() as u64,
            },
        );
    }

    let is_cancelled = executor.cancel_flag.load(Ordering::SeqCst);
    let result = BatchComplete {
        job_id: String::new(),
        total_files,
        success: completed_files,
        failed: failed_files,
        total_translated: total_exported,
        total_errors,
        duration_ms: started_at.elapsed().as_millis() as u64,
        is_cancelled,
        errors: batch_errors,
    };

    let _ = window.emit("batch-complete", result.clone());

    if let Ok(mut state) = executor.state.lock() {
        if matches!(&*state, BatchJobState::Running { .. }) {
            *state = BatchJobState::Done { result };
        }
    }
}

// ── Strings 保存（独立于 AppState） ────────────────────────────

fn save_strings_for_batch(
    esp_path: &str,
    strings_dir: Option<&str>,
    language: &str,
    target_lang: &str,
    strings: &[SkyString],
) -> Result<(), String> {
    let esp_dir = std::path::Path::new(esp_path)
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));

    let strings_dir = strings_dir
        .map(|d| std::path::Path::new(d).to_path_buf())
        .unwrap_or_else(|| esp_dir.to_path_buf());

    let base_name = std::path::Path::new(esp_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("skyrim");

    // 收集翻译映射
    let mut translated_map: std::collections::HashMap<(u8, i32), String> =
        std::collections::HashMap::new();
    for sk in strings.iter() {
        if !sk.translation.is_empty() {
            translated_map.insert((sk.list_index, sk.esp_ptr.str_id), sk.translation.clone());
        }
    }

    for (list_index, ext) in [(0u8, "STRINGS"), (1u8, "DLSTRINGS"), (2u8, "ILSTRINGS")] {
        let source_path =
            strings_dir.join(format!("{}_{}.{}", base_name, language, ext.to_lowercase()));

        let mut strings_file = if source_path.exists() {
            xt_core::strings::StringsFile::load_with_format(
                &source_path,
                xt_core::strings::StringsFile::detect_format(&source_path),
            )
            .unwrap_or_else(|_| xt_core::strings::StringsFile::new())
        } else {
            xt_core::strings::StringsFile::new()
        };

        for (&(li, str_id), translation) in &translated_map {
            if li == list_index {
                let id = str_id as u32;
                strings_file.strings.insert(id, translation.clone());
            }
        }

        let target_path = strings_dir.join(format!(
            "{}_{}.{}",
            base_name,
            target_lang,
            ext.to_lowercase()
        ));

        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create dir: {}", e))?;
        }

        let format = xt_core::strings::StringsFile::detect_format(&target_path);
        strings_file.format = format;
        strings_file
            .save_with_format(&target_path, format)
            .map_err(|e| format!("Failed to write {}: {}", ext, e))?;
    }

    Ok(())
}

/// 创建进度事件（简易）
fn make_progress(_job_id: &str, _file_path: &str, _stage: &str, msg: &str) -> BatchProgress {
    BatchProgress {
        job_id: String::new(),
        file_path: String::new(),
        stage: "info".to_string(),
        current_file: 0,
        total_files: 0,
        strings_translated: 0,
        total_strings: 0,
        message: msg.to_string(),
    }
}
