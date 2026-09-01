use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::Emitter;
use xt_core::batch_queue::BatchQueue;
use xt_core::cache_index::CacheIndex;
use xt_core::esp::game_detect;
use xt_core::esp::parser::{EspParser, StringsFiles, StringsLoadStrategy};
use xt_core::esp::record_tree::EspFile;
use xt_core::matching::{apply_dictionary_entries_with_policy, ApplyPolicy, DictionaryApplyEntry};
use xt_core::pex::types::PexTranslatableString;
use xt_core::sqlite_cache::SqliteCache;
use xt_core::sst::v8::SstDictionary;
use xt_core::strings::CodepageTable;
use xt_core::translation_api::config::ApiTranslatorConfig;
use xt_core::translation_api::{
    AzureProvider, BaiduProvider, DeepLProvider, GoogleProvider, OpenAIProvider, ProviderType,
    TranslationProvider, YoudaoProvider,
};
use xt_core::translation_cache::TranslationCache;
use xt_core::types::game_id::GameId;
use xt_core::types::params::SkyStringParams;
use xt_core::types::sky_string::SkyString;
use xt_core::xml::{
    import_xml_to_sky_strings, parse_xml_file, sky_strings_to_xml_entries, write_xml_file,
    XmlExportParams,
};
use xt_shared::dto::{
    ApplyCacheResponse, AutoBackupRequest, AutoBackupResponse, BatchConfig, BatchEntry,
    BatchStatus, BsaFileEntryDto, BsaFileListDto, CheckPendingCacheResponse, CtdaFuncDto,
    DataConfigsDto, DialogInfoDto, DialogTreeDto, EspComparePairDto, EspCompareResultDto,
    EspLoadProgress, FieldSizeInfoDto, FinalizeRequest, FinalizeResponse, FuzLipDataResponse,
    FuzMapping, FuzScanResponse, HeuristicMatchDTO, HeuristicSearchRequest, InjectArchiveRequest,
    InjectArchiveResponse, LipDataDto,
    LipKeyframeDto, LoadEspResponse, LoadSstResponse,
    McmComparePolicy, McmCompareRequest, McmCompareResult, McmEntryDto, McmFileDto, McmSaveRequest,
    NpcDialogDto, PexScriptDto, PexTranslatableDto, QueryRequest, QueryResponse, RecoveryInfo,
    SaveStringsRequest, SaveStringsResponse, SkyStringDTO,
    SstApplyOptionsDto, SstMatchModeDto, SstOverwriteScopeDto,
    TranslateRequest, XmlExportRequest,
    XmlImportResponse, XmlProgress,
};

use crate::batch::BatchExecutor;

/// 已加载的 ESP 文件信息
///
/// 用于追踪当前打开的 ESP 文件及其关联的 Strings 目录。
/// 当用户加载新的 ESP 文件时，此结构会被更新。
#[derive(Clone, Debug)]
pub struct EspFileInfo {
    /// ESP/ESM 文件的完整路径
    pub esp_path: String,
    /// Strings 文件所在目录（可能与 ESP 不同）
    /// 如果为 None，则使用 ESP 所在目录
    pub strings_dir: Option<String>,
    /// 字符串文件的语言标识（如 "english", "chinese"）
    pub language: String,
}

/// 应用状态：持有所有加载的文件数据
///
/// AppState 是 Tauri 应用的全局状态容器，通过 Mutex 保护所有可变字段。
/// 所有 Tauri 命令都通过 `tauri::State<'_, Arc<AppState>>` 获取此状态。
///
/// 设计要点：
/// - 使用 Mutex 而非 RwLock，因为大多数操作是快速的（不需要并发读）
/// - API Key 存储在内存中，应用关闭时丢失（不持久化到磁盘）
/// - `strings` 是主要的工作集，所有翻译操作都基于此
/// - `esp_file` 用于 ESP 回写功能（T42-T45）
pub struct AppState {
    /// 已加载的字符串列表（主要工作集）
    /// 包含 ESP 中提取的所有可翻译字符串
    pub strings: Mutex<Vec<SkyString>>,
    /// SST 字典中的旧数据条目（用于保留向后兼容性）
    /// 当 SST 加载时，未匹配的条目保存在此，导出时一并写出
    pub sst_old_data: Mutex<Vec<SkyString>>,
    /// 当前打开的 ESP 文件信息
    pub file_info: Mutex<Option<EspFileInfo>>,
    /// OpenAI 兼容 API Key（内存存储，不持久化）
    /// 支持 OpenAI 官方 API 和兼容的第三方服务（如 Azure OpenAI）
    pub openai_api_key: Mutex<Option<String>>,
    /// DeepL API Key（内存存储，不持久化）
    pub deepl_api_key: Mutex<Option<String>>,
    /// 百度翻译 AppId（内存存储，不持久化）
    pub baidu_app_id: Mutex<Option<String>>,
    /// 百度翻译 Key（内存存储，不持久化）
    pub baidu_key: Mutex<Option<String>>,
    /// 有道翻译 AppKey（内存存储，不持久化）
    pub youdao_app_key: Mutex<Option<String>>,
    /// 有道翻译 SecretKey（内存存储，不持久化）
    pub youdao_secret_key: Mutex<Option<String>>,
    /// Azure 翻译 subscription key（内存存储，不持久化）
    pub azure_key: Mutex<Option<String>>,
    /// 当前选中的翻译提供方（OpenAI / DeepL / Baidu / Youdao / Azure）
    pub current_provider: Mutex<ProviderType>,
    /// 是否有未保存的翻译修改
    /// 前端使用此标志来提示用户保存
    pub is_dirty: Mutex<bool>,
    /// ApiTranslator.txt 配置（从文件加载，包含 API 端点等）
    pub api_config: ApiTranslatorConfig,
    /// Vocabulary: source→translation 词汇对（来自游戏 Strings 文件）
    /// 用于启发式搜索和翻译建议
    pub vocabulary: Mutex<Vec<(String, String)>>,
    /// 字符串级批量翻译队列 (非文件级)
    /// 用于后台翻译任务的状态管理
    pub batch_queue: Mutex<Option<Arc<BatchQueue>>>,
    /// ESP 文件树（用于回写）
    /// 在 ESP 模式下构建，用于 save_esp / finalize_esp 等操作
    pub esp_file: Mutex<Option<xt_core::esp::record_tree::EspFile>>,
    /// Codepage 表（用于正确的字符串文件编码加载/写入）
    /// 不同游戏使用不同的代码页（如 UTF-8, Windows-1252 等）
    pub codepage_table: Mutex<Option<CodepageTable>>,
    /// 拼写检查器
    /// 用于检测翻译中的拼写错误
    pub spell_checker: Mutex<xt_core::spell::SpellChecker>,
    /// Header Processor 规则集
    /// 用于自动处理 ESP 记录头部的规则
    pub header_rules: Mutex<xt_core::header_processor::HeaderRuleSet>,
    /// Header Processor 预处理选项
    /// 用于配置头部处理的预处理选项
    pub pre_processing_opts: Mutex<xt_core::header_processor::PreProcessingOpts>,
}

impl AppState {
    pub fn new(api_config: ApiTranslatorConfig) -> Self {
        let openai_env_key = std::env::var("XT_TRANSLATE_API_KEY").ok();
        let deepl_env_key = std::env::var("XT_DEEPL_API_KEY").ok();

        let default_provider = if openai_env_key.is_some() {
            ProviderType::OpenAI
        } else if deepl_env_key.is_some() {
            ProviderType::DeepL
        } else {
            ProviderType::OpenAI
        };

        Self {
            strings: Mutex::new(Vec::new()),
            sst_old_data: Mutex::new(Vec::new()),
            file_info: Mutex::new(None),
            openai_api_key: Mutex::new(openai_env_key),
            deepl_api_key: Mutex::new(deepl_env_key),
            baidu_app_id: Mutex::new(None),
            baidu_key: Mutex::new(None),
            youdao_app_key: Mutex::new(None),
            youdao_secret_key: Mutex::new(None),
            azure_key: Mutex::new(None),
            current_provider: Mutex::new(default_provider),
            is_dirty: Mutex::new(false),
            api_config,
            vocabulary: Mutex::new(Vec::new()),
            batch_queue: Mutex::new(None),
            esp_file: Mutex::new(None),
            codepage_table: Mutex::new(None),
            spell_checker: Mutex::new(xt_core::spell::SpellChecker::new()),
            header_rules: Mutex::new(xt_core::header_processor::HeaderRuleSet::new()),
            pre_processing_opts: Mutex::new(xt_core::header_processor::PreProcessingOpts::default()),
        }
    }
}

/// 获取缓存目录路径
///
/// Windows 平台: `%LOCALAPPDATA%/xTranslator/cache/`
/// Unix 平台: `~/.cache/xTranslator/`
fn cache_dir() -> std::path::PathBuf {
    if cfg!(windows) {
        std::env::var("LOCALAPPDATA")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join("xTranslator")
            .join("cache")
    } else {
        std::env::var("HOME")
            .map(|h| std::path::PathBuf::from(h).join(".cache"))
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join("xTranslator")
    }
}

/// 获取当前 ESP 文件的缓存哈希（从 file_info 读取路径并计算 SHA-256）
fn get_esp_cache_hash(state: &AppState) -> Option<String> {
    let file_info = state.file_info.lock().ok()?;
    let info = file_info.as_ref()?;
    xt_core::cache::hash_file(std::path::Path::new(&info.esp_path)).ok()
}

/// 获取配置目录路径
///
/// Windows 平台: `%LOCALAPPDATA%/xTranslator/`
/// Unix 平台: `~/.config/xTranslator/`
pub fn config_dir() -> std::path::PathBuf {
    if cfg!(windows) {
        std::env::var("LOCALAPPDATA")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join("xTranslator")
    } else {
        std::env::var("HOME")
            .map(|h| std::path::PathBuf::from(h).join(".config"))
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .join("xTranslator")
    }
}

/// 将 SkyString 状态转为前端字符串
///
/// 约定：
/// - translated：已翻译
/// - incomplete：未完成翻译
/// - locked：不可编辑/锁定
/// - untranslated：未翻译（兜底，避免未知标志位组合被误判为 locked）
///
/// 兜底策略：未知标志位组合统一映射为 `untranslated`，
/// 比 `locked` 更不易迷惑用户（前者暗示"需要处理"，后者"不可编辑"）。
fn status_string(sk: &SkyString) -> String {
    if sk.params.is_translated() {
        "translated"
    } else if sk.params.is_incomplete() {
        "incomplete"
    } else if sk.params.is_locked() {
        "locked"
    } else {
        log::warn!("status_string: unrecognized flag combination, falling back to 'untranslated'");
        "untranslated"
    }
    .to_string()
}

/// 将 SkyString 转为 DTO
///
/// 说明：
/// - `form_id` 以十六进制字符串返回，便于前端直接展示。
/// - `list_index` 来自 ESP 解析或 SST 加载，标识 STRINGS/DLSTRINGS/ILSTRINGS 归属。
/// - `ld` 是启发式搜索的匹配数量，用于翻译建议排序。
fn sky_string_to_dto(sk: &SkyString) -> SkyStringDTO {
    SkyStringDTO {
        id: sk.id,
        source: sk.source.clone(),
        translation: sk.translation.clone(),
        record_sig: String::from_utf8_lossy(&sk.esp_ptr.record_sig).to_string(),
        field_sig: String::from_utf8_lossy(&sk.esp_ptr.field_sig).to_string(),
        edid: sk.edid.clone(),
        form_id: format!("0x{:08X}", sk.esp_ptr.form_id),
        status: status_string(sk),
        list_index: sk.list_index,
        str_id: sk.esp_ptr.str_id,
        // VMAD 字符串使用负 str_id 编码偏移量
        is_vmad: sk.esp_ptr.str_id < 0,
        ld: sk.ld_found.min(255) as u8,
    }
}

/// 将 SST 旧数据条目追加到字符串列表
///
/// SST 加载时，未匹配的条目被保存为 "oldData"。
/// 导出时需要将这些条目一并写出，以保持向后兼容性。
fn append_old_data_entries(entries: &mut Vec<SkyString>, old_data: &[SkyString]) {
    for old in old_data {
        let mut sk = old.clone();
        sk.params.set(SkyStringParams::OLD_DATA, true);
        entries.push(sk);
    }
}

/// 加载 ESP/ESM 文件并构建内存中的字符串列表。
///
/// 这是应用的核心命令，负责：
/// 1. 解析 ESP/ESM 二进制文件
/// 2. 加载关联的 Strings 文件（.STRINGS / .DLSTRINGS / .ILSTRINGS）
/// 3. 构建 ESP 记录树（用于后续的回写操作）
/// 4. 缓存解析结果以加速重复加载
///
/// 行为要点：
/// - 解析会覆盖当前 `AppState.strings`（相当于重新打开文件）。
/// - 若提供 `strings_dir`，会尝试加载对应语言的 STRINGS 文件。
/// - 返回值中的统计信息用于前端侧边栏和加载反馈。
/// - 先检查本地缓存（基于 ESP 文件 SHA-256 哈希），命中则直接返回。
/// - 通过 Tauri 事件系统实时发送进度更新。
///
/// 参数：
/// - `esp_path`: ESP/ESM 文件的完整路径
/// - `strings_dir`: Strings 文件所在目录（可选，默认使用 ESP 所在目录）
/// - `language`: 字符串文件的语言标识（可选，默认 "english"）
/// - `game`: 游戏类型（可选，用于加载正确的 record_defs）
///
/// 返回：
/// - `LoadEspResponse`: 包含解析统计和缓存状态
///
/// 错误处理：
/// - 文件不存在或无读权限 → 返回错误
/// - 文件格式无效 → 返回错误
/// - Strings 文件加载失败 → 继续（不中断主流程）
#[tauri::command]
pub async fn load_esp(
    window: tauri::Window,
    state: tauri::State<'_, Arc<AppState>>,
    esp_path: String,
    strings_dir: Option<String>,
    language: Option<String>,
    game: Option<String>,
    strings_strategy: Option<String>,
) -> Result<LoadEspResponse, String> {
    let esp_path_clone = esp_path.clone();
    let strings_dir_clone = strings_dir.clone();
    let language_clone = language.clone();
    let game_clone = game.clone();
    let strategy = StringsLoadStrategy::from_str_value(
        strings_strategy.as_deref().unwrap_or("disk"),
    )
    .ok_or_else(|| format!("invalid strings strategy: {:?}", strings_strategy))?;

    let c_dir = cache_dir();

    // ESP 解析是 CPU 密集型任务，放到阻塞线程池里执行，避免卡住异步运行时。
    let result = tokio::task::spawn_blocking(
        move || -> Result<(Vec<SkyString>, LoadEspResponse, Option<EspFile>), String> {
            let start = std::time::Instant::now();

            // ── 缓存检查阶段 ──
            let cache = SqliteCache::new(c_dir.clone());
            let esp_path_ref = std::path::Path::new(&esp_path_clone);

            let mut cache_index = CacheIndex::load(&c_dir);

            // 快速路径：通过 mtime+size 查找避免完整文件的 SHA-256 读取
            let file_hash = cache_index
                .lookup(esp_path_ref)
                .or_else(|| xt_core::cache::hash_file(esp_path_ref).ok());

            if let Some(ref hash) = file_hash {
                if let Some(cached) = cache.lookup(hash) {
                    let _ = window.emit(
                        "esp-load-progress",
                        EspLoadProgress {
                            stage: "cached".to_string(),
                            current: 100,
                            total: 100,
                            percentage: 100,
                            message: format!(
                                "Loaded from cache ({} strings)",
                                cached.strings.len()
                            ),
                        },
                    );

                    let total = cached.strings.len() as u32;
                    let record_counts = cache.compute_record_counts(hash).unwrap_or_default();

                    // 缓存命中：字符串已就绪，但 ESP 树仍需构建（write-back 需要）
                    // 在阻塞线程内解析文件结构以构建记录树
                    let mut tree_parser = EspParser::new();
                    tree_parser.enable_esp_mode();
                    tree_parser.set_build_search_index(false); // 不需要搜索索引，只要记录树
                    let esp_file = if let Ok(mut f) = std::fs::File::open(esp_path_ref) {
                        let _ = tree_parser.parse(&mut f);
                        tree_parser.build_esp_file()
                    } else {
                        None
                    };

                    // Cache hits still need game context for downstream game-specific tools.
                    // Reuse the TES4 header already parsed for the write-back tree.
                    let detected_game_id = esp_file.as_ref().and_then(|f| {
                        game_detect::game_from_form_version(f.tes4.record_header_data.f_version)
                    });
                    let (resolved_game_id, game_source) = game_detect::resolve_game_id(
                        game_clone.as_deref(),
                        detected_game_id,
                        GameId::SkyrimSE,
                    );

                    return Ok((
                        cached.strings,
                        LoadEspResponse {
                            total,
                            compressed_records: cached.compressed_records,
                            strings_loaded: cached.strings_loaded,
                            parse_time_ms: 0,
                            record_counts,
                            cached: true,
                            esp_hash: hash.clone(),
                            game_id: resolved_game_id.as_str().to_string(),
                            detected_game_id: detected_game_id.map(|g| g.as_str().to_string()),
                            game_source: game_source.as_str().to_string(),
                            // 缓存命中：无法回溯每个文件的来源，标记为缓存来源
                            strings_sources: vec!["cache".to_string(); 3],
                        },
                        esp_file,
                    ));
                }
            }

            // ── 缓存未命中，完整解析 ──

            // 阶段 1：加载 record_defs
            let _ = window.emit(
                "esp-load-progress",
                EspLoadProgress {
                    stage: "reading_defs".to_string(),
                    current: 0,
                    total: 100,
                    percentage: 0,
                    message: "Loading record definitions...".to_string(),
                },
            );

            // Prefer an explicit workspace, otherwise detect from TES4 Form Version.
            // A compatibility fallback remains explicitly marked as untrusted.
            let detected_game_id = game_detect::detect_game_from_esp(esp_path_ref);
            let (game_id, game_source) = game_detect::resolve_game_id(
                game_clone.as_deref(),
                detected_game_id,
                GameId::SkyrimSE,
            );

            // 优先加载对应游戏的 record_defs；失败时回退到内置默认定义。
            let data_dir = std::path::Path::new("Data");
            let mut parser =
                EspParser::with_game(data_dir, game_id).unwrap_or_else(|_| EspParser::new());

            // 启用 ESP 模式以构建记录树（用于回写支持）
            parser.enable_esp_mode();

            let _ = window.emit(
                "esp-load-progress",
                EspLoadProgress {
                    stage: "reading_defs".to_string(),
                    current: 100,
                    total: 100,
                    percentage: 5,
                    message: "Record definitions loaded".to_string(),
                },
            );

            // 阶段 2：加载 Strings 文件
            let _ = window.emit(
                "esp-load-progress",
                EspLoadProgress {
                    stage: "loading_strings".to_string(),
                    current: 0,
                    total: 100,
                    percentage: 5,
                    message: "Loading strings files...".to_string(),
                },
            );

            let lang = language_clone.as_deref().unwrap_or("english");
            let base_name = std::path::Path::new(&esp_path_clone)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("skyrim");

            let codepage_path = data_dir.join(game_id.as_str()).join("codepage.txt");
            let codepage_table = if codepage_path.exists() {
                CodepageTable::load_from_file(&codepage_path).ok()
            } else {
                None
            };

            let mut strings_loaded = 0u8;

            if strategy == StringsLoadStrategy::Manual {
                // Manual 策略：调用方负责提供已解析的 StringsFiles，此处保持空集
                parser.strings_files = StringsFiles::default();
            } else if let Some(ref dir) = strings_dir_clone {
                let dir_path = std::path::Path::new(dir);
                if let Some(ref table) = codepage_table {
                    parser.strings_files = StringsFiles::load_from_dir_with_strategy(
                        dir_path,
                        base_name,
                        lang,
                        Some(table),
                        strategy,
                    );
                } else {
                    parser.strings_files = StringsFiles::load_from_dir_with_strategy(
                        dir_path,
                        base_name,
                        lang,
                        None,
                        strategy,
                    );
                }
                strings_loaded = parser.strings_files.loaded_count() as u8;
            }

            if strings_loaded == 0 && strategy != StringsLoadStrategy::Manual {
                let esp_dir = std::path::Path::new(&esp_path_clone)
                    .parent()
                    .unwrap_or_else(|| std::path::Path::new("."));
                if let Some(ref table) = codepage_table {
                    parser.strings_files = StringsFiles::load_from_dir_with_strategy(
                        esp_dir,
                        base_name,
                        lang,
                        Some(table),
                        strategy,
                    );
                } else {
                    parser.strings_files = StringsFiles::load_from_dir_with_strategy(
                        esp_dir,
                        base_name,
                        lang,
                        None,
                        strategy,
                    );
                }
                strings_loaded = parser.strings_files.loaded_count() as u8;
            }

            let _ = window.emit(
                "esp-load-progress",
                EspLoadProgress {
                    stage: "loading_strings".to_string(),
                    current: 100,
                    total: 100,
                    percentage: 15,
                    message: "Strings files loaded".to_string(),
                },
            );

            // 阶段 3：解析 ESP/ESM
            let _ = window.emit(
                "esp-load-progress",
                EspLoadProgress {
                    stage: "parsing".to_string(),
                    current: 0,
                    total: 100,
                    percentage: 15,
                    message: "Parsing ESP file...".to_string(),
                },
            );

            let mut file = std::fs::File::open(&esp_path_clone)
                .map_err(|e| format!("Failed to open ESP: {}", e))?;

            let file_size = file.metadata().map(|m| m.len()).unwrap_or(1);

            let window_clone = window.clone();
            let file_size_for_callback = file_size;
            parser.set_progress_callback(move |current_bytes| {
                let percentage =
                    ((current_bytes as f64 / file_size_for_callback as f64) * 80.0) as u8 + 15;
                let _ = window_clone.emit(
                    "esp-load-progress",
                    EspLoadProgress {
                        stage: "parsing".to_string(),
                        current: current_bytes,
                        total: file_size_for_callback,
                        percentage: percentage.min(95),
                        message: format!(
                            "Parsing... {:.1}%",
                            (current_bytes as f64 / file_size_for_callback as f64) * 100.0
                        ),
                    },
                );
            });

            parser
                .parse(&mut file)
                .map_err(|e| format!("Failed to parse ESP: {}", e))?;

            let _ = window.emit(
                "esp-load-progress",
                EspLoadProgress {
                    stage: "finalizing".to_string(),
                    current: 100,
                    total: 100,
                    percentage: 95,
                    message: "Storing cache...".to_string(),
                },
            );

            let parse_time_ms = start.elapsed().as_millis() as u64;
            let total = parser.strings.len() as u32;

            let mut record_counts: HashMap<String, usize> = HashMap::new();
            for sk in &parser.strings {
                let sig = String::from_utf8_lossy(&sk.esp_ptr.record_sig).to_string();
                *record_counts.entry(sig).or_insert(0) += 1;
            }

            let compressed_records = parser.compressed_records;

            // 构建 ESP 文件树（用于回写支持）
            let esp_file = parser.build_esp_file();

            // 存储解析结果到 SQLite 缓存（静默失败，不影响主流程）
            if let Some(ref hash) = file_hash {
                let cache_payload = xt_core::sqlite_cache::CachePayload {
                    version: 2,
                    strings: parser.strings.clone(),
                    compressed_records,
                    strings_loaded,
                };
                let _ = cache.store(hash, &cache_payload);
                // 更新索引：下次加载时通过 mtime+size 直接获取哈希
                cache_index.store(esp_path_ref, hash);
                cache_index.save(&c_dir);
            }

            let _ = window.emit(
                "esp-load-progress",
                EspLoadProgress {
                    stage: "finalizing".to_string(),
                    current: 100,
                    total: 100,
                    percentage: 100,
                    message: "Complete".to_string(),
                },
            );

            Ok((
                parser.strings,
                LoadEspResponse {
                    total,
                    compressed_records,
                    strings_loaded,
                    parse_time_ms,
                    record_counts,
                    cached: false,
                    esp_hash: file_hash.unwrap_or_default(),
                    game_id: game_id.as_str().to_string(),
                    detected_game_id: detected_game_id.map(|g| g.as_str().to_string()),
                    game_source: game_source.as_str().to_string(),
                    strings_sources: parser
                        .strings_files
                        .sources
                        .iter()
                        .map(|s| s.as_str().to_string())
                        .collect(),
                },
                esp_file,
            ))
        },
    )
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .map_err(|e| e)?;

    let mut strings = state.strings.lock().map_err(|e| e.to_string())?;
    *strings = result.0;
    drop(strings);

    let mut old_data = state.sst_old_data.lock().map_err(|e| e.to_string())?;
    old_data.clear();

    let mut file_info = state.file_info.lock().map_err(|e| e.to_string())?;
    *file_info = Some(EspFileInfo {
        esp_path: esp_path.clone(),
        strings_dir,
        language: language.unwrap_or_else(|| "english".to_string()),
    });

    // 复用解析时构建的 ESP 树（避免重复解析）
    {
        let mut esp_file_lock = state.esp_file.lock().map_err(|e| e.to_string())?;
        *esp_file_lock = result.2;
    }

    *state.is_dirty.lock().map_err(|e| e.to_string())? = false;

    // Store the codepage table for save/finalize using the already-resolved game context.
    let codepage_table = {
        let resolved_game_id = GameId::from_alias(&result.1.game_id).unwrap_or(GameId::SkyrimSE);
        let codepage_path = std::path::Path::new("Data")
            .join(resolved_game_id.as_str())
            .join("codepage.txt");
        if codepage_path.exists() {
            CodepageTable::load_from_file(&codepage_path).ok()
        } else {
            None
        }
    };
    *state.codepage_table.lock().map_err(|e| e.to_string())? = codepage_table;

    Ok(result.1)
}

/// 加载 SST 字典并合并到当前内存字符串。
///
/// 使用共享字典匹配引擎，按 exact / EDID / normalized / vocab 顺序应用，或根据高级选项应用。
/// 该命令仅更新匹配成功的条目，不会新增行。
#[tauri::command]
pub async fn load_sst(
    state: tauri::State<'_, Arc<AppState>>,
    sst_path: String,
    options: Option<SstApplyOptionsDto>,
) -> Result<LoadSstResponse, String> {
    // 读取 SST 字典
    let dict = SstDictionary::load_from_file(&sst_path)
        .map_err(|e| format!("Failed to load SST: {}", e))?;

    let mut strings = state.strings.lock().map_err(|e| e.to_string())?;
    let apply_entries: Vec<DictionaryApplyEntry> = dict
        .entries
        .iter()
        .map(DictionaryApplyEntry::from_sst_entry)
        .collect();

    let policy = match options {
        Some(opts) => {
            let core_scope = match opts.overwrite_scope {
                SstOverwriteScopeDto::All => xt_core::matching::SstOverwriteScope::All,
                SstOverwriteScopeDto::NoTransExclusive => {
                    xt_core::matching::SstOverwriteScope::NoTransExclusive
                }
                SstOverwriteScopeDto::NoTransAndPartial => {
                    xt_core::matching::SstOverwriteScope::NoTransAndPartial
                }
                SstOverwriteScopeDto::PartialOnly => {
                    xt_core::matching::SstOverwriteScope::PartialOnly
                }
                SstOverwriteScopeDto::Selection => {
                    xt_core::matching::SstOverwriteScope::Selection
                }
            };
            let core_mode = match opts.match_mode {
                SstMatchModeDto::FormIdOnly => xt_core::matching::SstMatchMode::FormIdOnly,
                SstMatchModeDto::FormIdStrictString => {
                    xt_core::matching::SstMatchMode::FormIdStrictString
                }
                SstMatchModeDto::FormIdRelaxedString => {
                    xt_core::matching::SstMatchMode::FormIdRelaxedString
                }
                SstMatchModeDto::StringOnly => xt_core::matching::SstMatchMode::StringOnly,
            };

            ApplyPolicy::sst_load_with_options(xt_core::matching::SstApplyOptions {
                overwrite_scope: core_scope,
                match_mode: core_mode,
                tag_only: opts.tag_only,
                reset_state: opts.reset_state,
                restrict_to_filter: opts.restrict_to_filter,
                selected_ids: opts.selected_ids,
                filtered_ids: opts.filtered_ids,
            })
        }
        None => ApplyPolicy::sst_load(),
    };

    let result = apply_dictionary_entries_with_policy(&mut strings, &apply_entries, policy);

    let old_data_entries: Vec<SkyString> = result
        .old_data_entries
        .iter()
        .map(DictionaryApplyEntry::to_old_data_sky_string)
        .collect();
    let mut old_data = state.sst_old_data.lock().map_err(|e| e.to_string())?;
    *old_data = old_data_entries;

    *state.is_dirty.lock().map_err(|e| e.to_string())? = true;

    Ok(LoadSstResponse {
        matched: result.total_matched(),
        unmatched: result.unmatched,
        updated_ids: result.updated_ids,
        tier_exact: result.tier_exact,
        tier_edid: result.tier_edid,
        tier_normalized: result.tier_normalized,
        tier_vocab: result.tier_vocab,
        ambiguous: result.ambiguous,
        pending_skipped: result.pending_skipped,
        old_data_preserved: result.old_data_preserved,
        warning: result.warning,
        big_warning: result.big_warning,
    })
}

/// 将当前内存字符串导出为 SST 字典文件。
///
/// 当提供 `masters` 时，会写入带主文件信息的 SST 头部。
#[tauri::command]
pub async fn save_sst(
    state: tauri::State<'_, Arc<AppState>>,
    sst_path: String,
    masters: Option<Vec<String>>,
) -> Result<(), String> {
    let strings = state.strings.lock().map_err(|e| e.to_string())?;
    let old_data = state.sst_old_data.lock().map_err(|e| e.to_string())?;
    let mut entries = strings.clone();
    append_old_data_entries(&mut entries, &old_data);

    let dict = if let Some(masters) = masters {
        SstDictionary::from_entries_with_masters(entries, masters)
    } else {
        SstDictionary::from_entries(entries)
    };

    dict.save_to_file(&sst_path)
        .map_err(|e| format!("Failed to save SST: {}", e))?;

    // 保存成功后清除脏标记
    *state.is_dirty.lock().map_err(|e| e.to_string())? = false;

    Ok(())
}

/// 将另一个 SST 字典的翻译合并到当前字典
///
/// 按 (str_id, record_sig, field_sig) 三元组匹配。
/// overwrite=true 时用来源译文覆盖已有译文，否则保留。
#[tauri::command]
pub async fn sst_merge(
    state: tauri::State<'_, crate::AppState>,
    source_path: String,
    overwrite: bool,
) -> Result<MergeStatsDto, String> {
    let source = SstDictionary::load_from_file(&source_path)
        .map_err(|e| format!("Failed to load source SST: {}", e))?;

    let mut strings = state.strings.lock().map_err(|e| e.to_string())?;
    let mut dict = SstDictionary::from_entries(strings.clone());

    let stats = dict.merge_from(&source, overwrite);

    *strings = dict.entries;

    Ok(MergeStatsDto {
        added: stats.added,
        updated: stats.updated,
        overwritten: stats.overwritten,
        conflicts_skipped: stats.conflicts_skipped,
    })
}

/// 导出当前对话树为 HTML
#[tauri::command]
pub async fn export_dial_html(
    state: tauri::State<'_, crate::AppState>,
    title: String,
) -> Result<String, String> {
    let strings = state.strings.lock().map_err(|e| e.to_string())?;
    let tree = xt_core::dial_html::build_dial_tree(&strings);
    let html = xt_core::dial_html::dial_tree_to_html(&tree, &title);
    Ok(html)
}

/// RTL 实时预览：对文本应用 RTL 处理并返回结果
#[tauri::command]
pub async fn rtl_preview(
    text: String,
    apply_reverse: bool,
    apply_shape: bool,
    line_width: u32,
) -> Result<Vec<String>, String> {
    use xt_core::rtl;

    if text.is_empty() {
        return Ok(vec![]);
    }

    let mut processed = text;

    if apply_shape {
        processed = rtl::shape_arabic(&processed);
    }
    if apply_reverse {
        if let Some(reversed) = rtl::reverse_rtl(&processed) {
            processed = reversed;
        }
    }

    // 按指定宽度换行
    let lines: Vec<String> = if line_width > 0 && line_width < processed.len() as u32 {
        processed
            .chars()
            .collect::<Vec<_>>()
            .chunks(line_width as usize)
            .map(|c| c.iter().collect())
            .collect()
    } else {
        vec![processed]
    };

    Ok(lines)
}

// ── 协作标签系统 ──────────────────────────────────────────────────

/// 获取所有协作标签
#[tauri::command]
pub async fn colab_get_labels(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<(u32, String)>, String> {
    let strings = state.strings.lock().map_err(|e| e.to_string())?;
    // 从 SST 旧数据或当前字符串中重建标签
    let old = state.sst_old_data.lock().map_err(|e| e.to_string())?;
    let mut labels: std::collections::BTreeMap<u32, String> = std::collections::BTreeMap::new();
    for sk in old.iter().chain(strings.iter()) {
        if sk.colab_id > 0 {
            labels
                .entry(sk.colab_id as u32)
                .or_insert_with(|| format!("Slot {}", sk.colab_id));
        }
    }
    Ok(labels.into_iter().collect())
}

/// 更新协作标签名称
#[tauri::command]
pub async fn colab_set_label(
    state: tauri::State<'_, crate::AppState>,
    slot_id: u32,
    _label: String,
) -> Result<(), String> {
    let strings = state.strings.lock().map_err(|e| e.to_string())?;
    // 更新当前字符串中匹配 slot 的标签（标签不存储在 SkyString 中，
    // 但 colab_id 指向此 slot。实际标签名存储在上层，这里只做验证）
    let has = strings.iter().any(|s| s.colab_id as u32 == slot_id);
    if !has && slot_id > 0 {
        return Err(format!("Slot {} is not used by any string", slot_id));
    }
    // 标签名存储在应用层，通过 SST colab_labels 持久化
    Ok(())
}

/// 分配协作标签到选中字符串
#[tauri::command]
pub async fn colab_assign(
    state: tauri::State<'_, crate::AppState>,
    ids: Vec<u32>,
    slot_id: u32,
) -> Result<usize, String> {
    let mut strings = state.strings.lock().map_err(|e| e.to_string())?;
    let mut count = 0;
    for &id in &ids {
        if let Some(sk) = strings.iter_mut().find(|s| s.id == id) {
            sk.colab_id = slot_id as u8;
            count += 1;
        }
    }
    Ok(count)
}

/// 按协作槽位过滤字符串（三态：0=关闭, 1=仅包含, 2=排除）
#[tauri::command]
pub async fn colab_filter(
    state: tauri::State<'_, crate::AppState>,
    slot_id: u32,
    mode: u8,
) -> Result<Vec<u32>, String> {
    let strings = state.strings.lock().map_err(|e| e.to_string())?;
    if mode == 0 || slot_id == 0 {
        return Ok(strings.iter().map(|s| s.id).collect());
    }
    let filtered: Vec<u32> = strings
        .iter()
        .filter(|s| {
            let match_slot = s.colab_id as u32 == slot_id;
            match mode {
                1 => match_slot,  // include
                2 => !match_slot, // exclude
                _ => true,
            }
        })
        .map(|s| s.id)
        .collect();
    Ok(filtered)
}

/// 按内部 `id` 更新单条翻译文本。
///
/// 注意：这里使用内部行 ID，而不是 `str_id`（两者语义不同）。
#[tauri::command]
pub async fn update_translation(
    state: tauri::State<'_, Arc<AppState>>,
    id: u32,
    translation: String,
) -> Result<(), String> {
    let (esp_hash, trans_clone) = {
        let mut strings = state.strings.lock().map_err(|e| e.to_string())?;

        // 用内部自增 ID 定位条目，避免筛选/排序后索引漂移问题。
        let found = strings.iter_mut().find(|sk| sk.id == id);
        let sk = match found {
            Some(s) => s,
            None => return Err(format!("String with id {} not found", id)),
        };

        sk.set_translation(translation);

        // 更新状态：有译文=已翻译；空译文=未完成（与前端状态语义一致）。
        if !sk.translation.is_empty() {
            sk.params.set(SkyStringParams::TRANSLATED, true);
            sk.params.set(SkyStringParams::INCOMPLETE_TRANS, false);
        } else {
            sk.params.set(SkyStringParams::TRANSLATED, false);
            sk.params.set(SkyStringParams::INCOMPLETE_TRANS, true);
        }

        // 标记有未保存的修改
        *state.is_dirty.lock().map_err(|e| e.to_string())? = true;

        // 获取 esp_hash 用于 SQLite 缓存更新
        let esp_hash = get_esp_cache_hash(&state);
        (esp_hash, sk.translation.clone())
    };

    // 异步更新 SQLite 缓存（静默失败）
    if let Some(hash) = esp_hash {
        let cache = SqliteCache::new(cache_dir());
        let _ = cache.update_translation(&hash, id, &trans_clone);
    }

    Ok(())
}

/// 批量更新翻译 — 单次 IPC 调用更新多条翻译，避免逐条 IPC 开销。
#[tauri::command]
pub async fn batch_update_translations(
    state: tauri::State<'_, Arc<AppState>>,
    updates: Vec<(u32, String)>,
) -> Result<u32, String> {
    let (esp_hash, changed_pairs) = {
        let mut strings = state.strings.lock().map_err(|e| e.to_string())?;
        let mut changed: u32 = 0;
        let mut pairs: Vec<(u32, String)> = Vec::new();

        for (id, translation) in updates {
            if let Some(sk) = strings.iter_mut().find(|sk| sk.id == id) {
                sk.set_translation(translation);
                if !sk.translation.is_empty() {
                    sk.params.set(SkyStringParams::TRANSLATED, true);
                    sk.params.set(SkyStringParams::INCOMPLETE_TRANS, false);
                } else {
                    sk.params.set(SkyStringParams::TRANSLATED, false);
                    sk.params.set(SkyStringParams::INCOMPLETE_TRANS, true);
                }
                pairs.push((id, sk.translation.clone()));
                changed += 1;
            }
        }

        if changed > 0 {
            *state.is_dirty.lock().map_err(|e| e.to_string())? = true;
        }

        let esp_hash = get_esp_cache_hash(&state);
        (esp_hash, pairs)
    };

    // 批量更新 SQLite 缓存（静默失败）
    if let Some(hash) = esp_hash {
        let cache = SqliteCache::new(cache_dir());
        for (id, translation) in &changed_pairs {
            let _ = cache.update_translation(&hash, *id, translation);
        }
    }

    Ok(changed_pairs.len() as u32)
}

/// 查询字符串分页结果（后端筛选/排序/分页一体化）。
///
/// 该命令主要用于服务端查询模式；全量虚拟滚动模式可走分块接口。
#[tauri::command]
pub async fn query_strings_command(
    state: tauri::State<'_, Arc<AppState>>,
    request: QueryRequest,
) -> Result<QueryResponse, String> {
    let data = state.strings.lock().map_err(|e| e.to_string())?;

    let start = std::time::Instant::now();
    let total = data.len() as u32;

    // 1) 先做状态筛选，后续文本筛选/排序/分页都在更小的数据集上执行。
    let status_filtered: Vec<&SkyString> = if let Some(ref sf) = request.status_filter {
        match sf.as_str() {
            "translated" => data.iter().filter(|sk| sk.params.is_translated()).collect(),
            "incomplete" => data
                .iter()
                .filter(|sk| sk.params.is_incomplete() && !sk.params.is_translated())
                .collect(),
            "locked" => data
                .iter()
                .filter(|sk| !sk.params.is_translated() && !sk.params.is_incomplete())
                .collect(),
            _ => data.iter().collect(),
        }
    } else {
        data.iter().collect()
    };

    // 2) 大小写不敏感的文本筛选：匹配 source / translation / record_sig。
    // 注意：当前实现对每条记录都会做 to_lowercase，后续可按需优化为预计算字段。
    let mut filtered_data: Vec<&SkyString> = if let Some(ref filter_text) = request.filter {
        let ft = filter_text.to_lowercase();
        status_filtered
            .into_iter()
            .filter(|sk| {
                sk.source.to_lowercase().contains(&ft)
                    || sk.translation.to_lowercase().contains(&ft)
                    || String::from_utf8_lossy(&sk.esp_ptr.record_sig)
                        .to_lowercase()
                        .contains(&ft)
            })
            .collect()
    } else {
        status_filtered
    };

    let filtered = filtered_data.len() as u32;

    // 3) 仅对筛选结果排序；未指定排序时保持解析时的原始顺序。
    if let Some(ref field) = request.sort_field {
        let is_asc = request.sort_dir.as_deref() != Some("desc");
        match field.as_str() {
            "id" => {
                if is_asc {
                    filtered_data.sort_by_key(|sk| sk.id);
                } else {
                    filtered_data.sort_by_key(|sk| std::cmp::Reverse(sk.id));
                }
            }
            "source" => {
                if is_asc {
                    filtered_data.sort_by(|a, b| a.source.cmp(&b.source));
                } else {
                    filtered_data.sort_by(|a, b| b.source.cmp(&a.source));
                }
            }
            "record_sig" => {
                if is_asc {
                    filtered_data.sort_by(|a, b| a.esp_ptr.record_sig.cmp(&b.esp_ptr.record_sig));
                } else {
                    filtered_data.sort_by(|a, b| b.esp_ptr.record_sig.cmp(&a.esp_ptr.record_sig));
                }
            }
            _ => {} // 默认不排序
        }
    }

    // 4) 最后执行 offset/limit 分页，仅返回当前视口所需数据。
    // 即使 offset 超过长度也会安全返回空数组（skip/take 语义保证）。
    let offset_usize = request.offset as usize;
    let limit_usize = request.limit as usize;
    let page: Vec<&SkyString> = filtered_data
        .into_iter()
        .skip(offset_usize)
        .take(limit_usize)
        .collect();

    let elapsed_ms = start.elapsed().as_millis() as u64;

    let dtos: Vec<SkyStringDTO> = page.iter().map(|sk| sky_string_to_dto(sk)).collect();

    Ok(QueryResponse {
        total,
        filtered,
        items: dtos,
        offset: request.offset,
        elapsed_ms,
    })
}

/// 返回当前加载数据的概览统计文本。
#[tauri::command]
pub async fn get_stats(state: tauri::State<'_, Arc<AppState>>) -> Result<String, String> {
    let data = state.strings.lock().map_err(|e| e.to_string())?;
    let translated = data.iter().filter(|sk| sk.params.is_translated()).count();
    let incomplete = data.iter().filter(|sk| sk.params.is_incomplete()).count();
    let locked = data.len() - translated - incomplete;
    Ok(format!(
        "Total: {} items | Translated: {} | Incomplete: {} | Locked: {} | Memory: ~{} MB",
        data.len(),
        translated,
        incomplete,
        locked,
        data.len() * 256 / 1024 / 1024
    ))
}

/// 对给定源文本执行启发式相似匹配，返回候选译文。
/// 候选集包括当前已翻译字符串 + vocabulary 词汇对。
#[tauri::command]
pub async fn heuristic_search(
    state: tauri::State<'_, Arc<AppState>>,
    request: HeuristicSearchRequest,
) -> Result<Vec<HeuristicMatchDTO>, String> {
    let data = state.strings.lock().map_err(|e| e.to_string())?;
    let vocab = state.vocabulary.lock().map_err(|e| e.to_string())?;

    // 候选集仅来自"已翻译"条目；未翻译条目没有可用目标文本。
    let mut candidates: Vec<(String, String)> = data
        .iter()
        .filter(|sk| sk.params.is_translated() && !sk.source.is_empty())
        .map(|sk| (sk.source.clone(), sk.translation.clone()))
        .collect();

    // 合并词汇对（来自游戏 Strings 文件的额外 source→translation 语料）
    candidates.extend(vocab.iter().cloned());

    if candidates.is_empty() {
        return Ok(Vec::new());
    }

    let max_res = request.max_results.unwrap_or(5);

    let matches = xt_core::heuristic::find_similar_delphi(&request.source, &candidates, max_res);

    let dtos: Vec<HeuristicMatchDTO> = matches
        .into_iter()
        .map(|m| HeuristicMatchDTO {
            source: m.source,
            translation: m.translation,
            similarity: m.similarity,
            levenshtein: m.levenshtein,
            lcs_len: m.lcs_len,
        })
        .collect();

    Ok(dtos)
}

/// 调用翻译提供方翻译单条文本。
///
/// 支持 OpenAI 兼容 API 和 DeepL API。
/// API Key 来自 `set_openai_api_key`/`set_deepl_api_key` 或环境变量。
#[tauri::command]
pub async fn translate_string(
    state: tauri::State<'_, Arc<AppState>>,
    request: TranslateRequest,
) -> Result<String, String> {
    // 确定使用哪个 provider（请求参数优先，否则用当前默认）
    let provider_type = request
        .provider
        .map(|p| ProviderType::from_str(&p))
        .unwrap_or_else(|| {
            *state
                .current_provider
                .lock()
                .unwrap_or_else(|e| e.into_inner())
        });

    // 获取对应 API Key
    let api_key = match provider_type {
        ProviderType::OpenAI => {
            let key = state.openai_api_key.lock().map_err(|e| e.to_string())?;
            key.clone().ok_or_else(||
                "OpenAI API key not set. Please set via Settings or XT_TRANSLATE_API_KEY env var".to_string()
            )?
        }
        ProviderType::DeepL => {
            let key = state.deepl_api_key.lock().map_err(|e| e.to_string())?;
            key.clone().ok_or_else(|| {
                "DeepL API key not set. Please set via Settings or XT_DEEPL_API_KEY env var"
                    .to_string()
            })?
        }
        ProviderType::Baidu => {
            // 百度不使用单一 API Key；凭证在下方检查
            String::new()
        }
        ProviderType::Youdao => {
            // 有道不使用单一 API Key；凭证在下方检查
            String::new()
        }
        ProviderType::Azure => {
            // 使用订阅密钥；在下方检查
            String::new()
        }
        ProviderType::Google => {
            // 无需 API Key
            String::new()
        }
    };

    // 保持默认语言兜底，避免前端漏传参数导致请求失败。
    let source_lang = request.source_lang.unwrap_or_else(|| "EN".to_string());
    let target_lang = request.target_lang.unwrap_or_else(|| "ZH".to_string());

    // 通过 ApiTranslator.txt 配置解析语言代码
    let provider_name = provider_type.to_string();
    let resolved_source = state.api_config.resolve_lang(&provider_name, &source_lang);
    let resolved_target = state.api_config.resolve_lang(&provider_name, &target_lang);

    // 从磁盘加载代理配置
    let proxy_config = xt_core::config::AppConfig::load(&config_dir()).ok();

    let text = request.text;

    let result = match provider_type {
        ProviderType::OpenAI => {
            let provider = OpenAIProvider::from_key(api_key).with_config(state.api_config.clone());
            provider
                .translate(
                    &text,
                    &resolved_source,
                    &resolved_target,
                    proxy_config.as_ref(),
                )
                .await
                .map_err(|e| e.to_string())?
        }
        ProviderType::DeepL => {
            let provider = DeepLProvider::new(api_key);
            provider
                .translate(
                    &text,
                    &resolved_source,
                    &resolved_target,
                    proxy_config.as_ref(),
                )
                .await
                .map_err(|e| e.to_string())?
        }
        ProviderType::Baidu => {
            let app_id = state
                .baidu_app_id
                .lock()
                .map_err(|e| e.to_string())?
                .clone()
                .ok_or_else(|| "Baidu AppId not set".to_string())?;
            let key = state
                .baidu_key
                .lock()
                .map_err(|e| e.to_string())?
                .clone()
                .ok_or_else(|| "Baidu Key not set".to_string())?;
            let provider = BaiduProvider::new(app_id, key);
            provider
                .translate(
                    &text,
                    &resolved_source,
                    &resolved_target,
                    proxy_config.as_ref(),
                )
                .await
                .map_err(|e| e.to_string())?
        }
        ProviderType::Youdao => {
            let app_key = state
                .youdao_app_key
                .lock()
                .map_err(|e| e.to_string())?
                .clone()
                .ok_or_else(|| "Youdao AppKey not set".to_string())?;
            let secret_key = state
                .youdao_secret_key
                .lock()
                .map_err(|e| e.to_string())?
                .clone()
                .ok_or_else(|| "Youdao SecretKey not set".to_string())?;
            let provider = YoudaoProvider::new(app_key, secret_key);
            provider
                .translate(
                    &text,
                    &resolved_source,
                    &resolved_target,
                    proxy_config.as_ref(),
                )
                .await
                .map_err(|e| e.to_string())?
        }
        ProviderType::Azure => {
            let key = state
                .azure_key
                .lock()
                .map_err(|e| e.to_string())?
                .clone()
                .ok_or_else(|| "Azure subscription key not set".to_string())?;
            let provider = AzureProvider::new(key);
            provider
                .translate(
                    &text,
                    &resolved_source,
                    &resolved_target,
                    proxy_config.as_ref(),
                )
                .await
                .map_err(|e| e.to_string())?
        }
        ProviderType::Google => {
            let provider = GoogleProvider::new();
            provider
                .translate(
                    &text,
                    &resolved_source,
                    &resolved_target,
                    proxy_config.as_ref(),
                )
                .await
                .map_err(|e| e.to_string())?
        }
    };

    Ok(result)
}

/// 设置（或清空）运行期 OpenAI API Key。
///
/// 仅写入内存，不做磁盘持久化。
#[tauri::command]
pub async fn set_openai_api_key(
    state: tauri::State<'_, Arc<AppState>>,
    api_key: String,
) -> Result<(), String> {
    let mut key = state.openai_api_key.lock().map_err(|e| e.to_string())?;
    *key = if api_key.is_empty() {
        None
    } else {
        Some(api_key)
    };
    Ok(())
}

/// 设置（或清空）运行期 DeepL API Key。
///
/// 仅写入内存，不做磁盘持久化。
/// 自动根据 key 是否以 `:fx` 结尾切换免费/专业版端点。
#[tauri::command]
pub async fn set_deepl_api_key(
    state: tauri::State<'_, Arc<AppState>>,
    api_key: String,
) -> Result<(), String> {
    let mut key = state.deepl_api_key.lock().map_err(|e| e.to_string())?;
    *key = if api_key.is_empty() {
        None
    } else {
        Some(api_key)
    };
    Ok(())
}

/// 设置（或清空）运行期百度翻译 AppId 和 Key。
#[tauri::command]
pub async fn set_baidu_api_key(
    state: tauri::State<'_, Arc<AppState>>,
    app_id: String,
    key: String,
) -> Result<(), String> {
    let mut aid = state.baidu_app_id.lock().map_err(|e| e.to_string())?;
    *aid = if app_id.is_empty() {
        None
    } else {
        Some(app_id)
    };
    let mut k = state.baidu_key.lock().map_err(|e| e.to_string())?;
    *k = if key.is_empty() { None } else { Some(key) };
    Ok(())
}

/// 设置（或清空）运行期有道翻译 AppKey 和 SecretKey。
#[tauri::command]
pub async fn set_yooudao_api_key(
    state: tauri::State<'_, Arc<AppState>>,
    app_key: String,
    secret_key: String,
) -> Result<(), String> {
    let mut ak = state.youdao_app_key.lock().map_err(|e| e.to_string())?;
    *ak = if app_key.is_empty() {
        None
    } else {
        Some(app_key)
    };
    let mut sk = state.youdao_secret_key.lock().map_err(|e| e.to_string())?;
    *sk = if secret_key.is_empty() {
        None
    } else {
        Some(secret_key)
    };
    Ok(())
}

/// 设置（或清空）运行期 Azure subscription key。
#[tauri::command]
pub async fn set_azure_api_key(
    state: tauri::State<'_, Arc<AppState>>,
    api_key: String,
) -> Result<(), String> {
    let mut key = state.azure_key.lock().map_err(|e| e.to_string())?;
    *key = if api_key.is_empty() {
        None
    } else {
        Some(api_key)
    };
    Ok(())
}

/// 设置当前默认翻译提供方。
#[tauri::command]
pub async fn set_translation_provider(
    state: tauri::State<'_, Arc<AppState>>,
    provider: String,
) -> Result<(), String> {
    let mut current = state.current_provider.lock().map_err(|e| e.to_string())?;
    *current = ProviderType::from_str(&provider);
    Ok(())
}

/// 获取当前翻译提供方和可用列表。
///
/// # 返回元组序列语义
/// `(current_provider, providers_list, openai_set, deepl_set, baidu_set, youdao_set, azure_set, google_set)`
/// - `current_provider`: 当前选中的提供方名称
/// - `providers_list`: 所有可用提供方名称列表
/// - `*_set`: 各提供方是否已配置密钥（`true` = 已配置）
#[tauri::command]
pub async fn get_translation_providers(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(String, Vec<String>, bool, bool, bool, bool, bool, bool), String> {
    let current = state.current_provider.lock().map_err(|e| e.to_string())?;
    let openai_set = state
        .openai_api_key
        .lock()
        .map_err(|e| e.to_string())?
        .is_some();
    let deepl_set = state
        .deepl_api_key
        .lock()
        .map_err(|e| e.to_string())?
        .is_some();
    let baidu_set = state
        .baidu_app_id
        .lock()
        .map_err(|e| e.to_string())?
        .is_some()
        && state.baidu_key.lock().map_err(|e| e.to_string())?.is_some();
    let youdao_set = state
        .youdao_app_key
        .lock()
        .map_err(|e| e.to_string())?
        .is_some()
        && state
            .youdao_secret_key
            .lock()
            .map_err(|e| e.to_string())?
            .is_some();
    let azure_set = state.azure_key.lock().map_err(|e| e.to_string())?.is_some();
    let google_set = true; // Google 无需密钥，始终返回 true

    Ok((
        current.to_string(),
        ProviderType::all()
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
        openai_set,
        deepl_set,
        baidu_set,
        youdao_set,
        azure_set,
        google_set,
    ))
}

/// 向前端广播 XML 导出进度事件
///
/// `stage` 为阶段名称（如 "preparing"、"exporting"），全体广播到前端的 `xml-progress` 事件通道。
fn emit_xml_progress(window: &tauri::Window, stage: &str, current: u64, total: u64, message: &str) {
    let percentage = if total > 0 {
        ((current as f64 / total as f64) * 100.0) as u8
    } else {
        0
    };
    let _ = window.emit(
        "xml-progress",
        XmlProgress {
            stage: stage.to_string(),
            current,
            total,
            percentage,
            message: message.to_string(),
        },
    );
}

/// 返回值为实际导出的条目数。
#[tauri::command]
pub async fn export_xml(
    window: tauri::Window,
    state: tauri::State<'_, Arc<AppState>>,
    request: XmlExportRequest,
) -> Result<u32, String> {
    let strings = state.strings.lock().map_err(|e| e.to_string())?;
    let file_info = state.file_info.lock().map_err(|e| e.to_string())?;

    emit_xml_progress(&window, "preparing", 0, 3, "Preparing XML export...");

    let addon = file_info
        .as_ref()
        .map(|fi| {
            std::path::Path::new(&fi.esp_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown")
                .to_string()
        })
        .unwrap_or_else(|| "Unknown".to_string());

    let source_lang = file_info
        .as_ref()
        .map(|fi| fi.language.clone())
        .unwrap_or_else(|| "english".to_string());

    // 仅导出"已有译文"的条目，行为与 Delphi 版本保持一致。
    let entries = sky_strings_to_xml_entries(&strings);
    let exported_count = entries.len() as u32;

    emit_xml_progress(
        &window,
        "collecting",
        1,
        3,
        &format!("Collected {} entries...", exported_count),
    );

    let params = XmlExportParams {
        addon,
        source_lang,
        dest_lang: request.dest_lang,
        version: 2,
    };

    emit_xml_progress(&window, "writing", 2, 3, "Writing XML file...");

    let path = std::path::Path::new(&request.path);
    write_xml_file(path, &params, &entries).map_err(|e| format!("Failed to write XML: {}", e))?;

    emit_xml_progress(&window, "done", 3, 3, "Export complete");

    // 导出成功后清除脏标记
    *state.is_dirty.lock().map_err(|e| e.to_string())? = false;

    Ok(exported_count)
}

/// 导入 XML 翻译并合并到当前内存字符串。
///
/// 使用增强多层级匹配策略，返回各层级统计及被更新的内部 ID 列表。
#[tauri::command]
pub async fn import_xml(
    window: tauri::Window,
    state: tauri::State<'_, Arc<AppState>>,
    xml_path: String,
) -> Result<XmlImportResponse, String> {
    emit_xml_progress(&window, "parsing", 0, 2, "Parsing XML file...");

    // 先解析 XML，再通过共享字典匹配引擎写回内存数据。
    let (_, xml_entries) = parse_xml_file(std::path::Path::new(&xml_path))
        .map_err(|e| format!("Failed to parse XML: {}", e))?;

    let total = xml_entries.len() as u32;

    emit_xml_progress(
        &window,
        "merging",
        1,
        2,
        &format!("Merging {} entries...", total),
    );

    let mut strings = state.strings.lock().map_err(|e| e.to_string())?;
    let result = import_xml_to_sky_strings(&mut strings, &xml_entries);

    emit_xml_progress(&window, "done", 2, 2, "Import complete");

    // 导入修改了数据，标记为脏
    *state.is_dirty.lock().map_err(|e| e.to_string())? = true;

    Ok(XmlImportResponse {
        matched: result.total_matched(),
        unmatched: result.unmatched,
        total,
        updated_ids: result.updated_ids,
        tier_exact: result.tier_exact,
        tier_edid: result.tier_edid,
        tier_vocab: result.tier_vocab,
        tier_normalized: result.tier_normalized,
        ambiguous: result.ambiguous,
        pending_skipped: result.pending_skipped,
        old_data_preserved: result.old_data_preserved,
        warning: result.warning,
        big_warning: result.big_warning,
    })
}

/// 分块返回 DTO，避免触发 WebView2 的 IPC 负载大小限制。
/// 一般每批约 1 万条（约 2MB JSON）。
#[tauri::command]
pub async fn get_strings_chunk(
    state: tauri::State<'_, Arc<AppState>>,
    offset: u32,
    limit: u32,
) -> Result<Vec<SkyStringDTO>, String> {
    let data = state.strings.lock().map_err(|e| e.to_string())?;
    let offset_usize = offset as usize;
    let limit_usize = limit as usize;

    let dtos: Vec<SkyStringDTO> = data
        .iter()
        .skip(offset_usize)
        .take(limit_usize)
        .map(sky_string_to_dto)
        .collect();

    Ok(dtos)
}

/// 返回总条数，供前端计算分块拉取批次。
#[tauri::command]
pub async fn get_strings_count(state: tauri::State<'_, Arc<AppState>>) -> Result<u32, String> {
    let data = state.strings.lock().map_err(|e| e.to_string())?;
    Ok(data.len() as u32)
}

/// 查询当前是否有未保存的翻译修改
#[tauri::command]
pub async fn get_is_dirty(state: tauri::State<'_, Arc<AppState>>) -> Result<bool, String> {
    let dirty = state.is_dirty.lock().map_err(|e| e.to_string())?;
    Ok(*dirty)
}

/// 一次性返回全部 DTO（用于前端全量虚拟滚动模式）。
/// 大数据集优先使用 `get_strings_chunk`，避免 IPC 负载过大。
#[tauri::command]
pub async fn get_all_strings(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<SkyStringDTO>, String> {
    let data = state.strings.lock().map_err(|e| e.to_string())?;
    let dtos: Vec<SkyStringDTO> = data.iter().map(sky_string_to_dto).collect();
    Ok(dtos)
}

/// 将已翻译的字符串写入目标语言的 Strings 文件。
///
/// 策略（与 Delphi 原版一致）：
/// 1. 加载源语言 Strings 文件作为基础（保留未翻译的原始条目）
/// 2. 用已翻译条目覆盖对应 str_id 的文本
/// 3. 按 list_index 分组写入 .STRINGS / .DLSTRINGS / .ILSTRINGS
/// 4. ESP 文件本身不修改
#[tauri::command]
pub async fn save_strings(
    state: tauri::State<'_, Arc<AppState>>,
    request: SaveStringsRequest,
) -> Result<SaveStringsResponse, String> {
    let strings = state.strings.lock().map_err(|e| e.to_string())?;
    let file_info = state.file_info.lock().map_err(|e| e.to_string())?;
    let codepage_table = state.codepage_table.lock().map_err(|e| e.to_string())?;
    let codepage_table_ref: Option<&CodepageTable> = codepage_table.as_ref();

    let source_lang = file_info
        .as_ref()
        .map(|fi| fi.language.clone())
        .unwrap_or_else(|| "english".to_string());
    let strings_dir = file_info
        .as_ref()
        .and_then(|fi| fi.strings_dir.clone())
        .unwrap_or_default();

    // 收集翻译映射：按 list_index 和 str_id 分组
    // 每个 SkyString 可能是源语言或目标语言的条目
    let mut translated_map: std::collections::HashMap<(u8, i32), String> =
        std::collections::HashMap::new();
    for sk in strings.iter() {
        // 跳过 VMAD 字符串（负 str_id），VMAD 不写入 .STRINGS 文件
        if !sk.translation.is_empty() && sk.esp_ptr.str_id >= 0 {
            translated_map.insert((sk.list_index, sk.esp_ptr.str_id), sk.translation.clone());
        }
    }

    let output_dir = std::path::Path::new(&request.output_dir);
    let base_name = &request.base_name;
    let target_lang = &request.target_lang;

    let mut strings_count = 0u32;
    let mut dlstrings_count = 0u32;
    let mut ilstrings_count = 0u32;
    let mut translated_count = 0u32;

    // 对每种 Strings 格式：加载源语言 → 覆盖翻译 → 写入目标语言
    for (list_index, ext, count_ref) in [
        (0u8, "STRINGS", &mut strings_count),
        (1u8, "DLSTRINGS", &mut dlstrings_count),
        (2u8, "ILSTRINGS", &mut ilstrings_count),
    ] {
        let source_path = std::path::Path::new(&strings_dir).join(format!(
            "{}_{}.{}",
            base_name,
            source_lang,
            ext.to_lowercase()
        ));

        // 加载源语言文件作为基础（保留所有未翻译条目）
        let mut strings_file = if source_path.exists() {
            if let Some(ref table) = codepage_table_ref {
                xt_core::strings::StringsFile::load_with_codepage_table(&source_path, table)
            } else {
                xt_core::strings::StringsFile::load_with_format(
                    &source_path,
                    xt_core::strings::StringsFile::detect_format(&source_path),
                )
            }
            .unwrap_or_else(|_| xt_core::strings::StringsFile::new())
        } else {
            xt_core::strings::StringsFile::new()
        };

        // 覆盖已翻译的条目
        for (&(li, str_id), translation) in &translated_map {
            if li == list_index {
                let id = str_id as u32;
                // 只有当 str_id 在源文件中存在时才覆盖；否则添加新条目
                strings_file.strings.insert(id, translation.clone());
                translated_count += 1;
            }
        }

        // 写入目标语言文件
        let target_path = output_dir.join(format!(
            "{}_{}.{}",
            base_name,
            target_lang,
            ext.to_lowercase()
        ));

        // 确保输出目录存在
        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create output dir: {}", e))?;
        }

        // 设置正确的格式
        let format = xt_core::strings::StringsFile::detect_format(&target_path);
        strings_file.format = format;

        strings_file
            .save_with_format(&target_path, format)
            .map_err(|e| format!("Failed to write {}: {}", ext, e))?;

        *count_ref = strings_file.strings.len() as u32;
    }

    // 保存成功后清除脏标记
    *state.is_dirty.lock().map_err(|e| e.to_string())? = false;

    Ok(SaveStringsResponse {
        strings_count,
        dlstrings_count,
        ilstrings_count,
        translated_count,
    })
}

// ── 批处理 IPC 命令 ──────────────────────────────────────────

/// 启动批处理翻译
#[tauri::command]
pub async fn start_batch_translate(
    window: tauri::Window,
    executor: tauri::State<'_, Arc<BatchExecutor>>,
    config: BatchConfig,
) -> Result<String, String> {
    let entries = config.entries.clone();
    let provider = config
        .provider
        .as_deref()
        .map(ProviderType::from_str)
        .unwrap_or(ProviderType::OpenAI);
    let target_lang = config.target_lang.unwrap_or_else(|| "chinese".to_string());
    let skip_translated = config.skip_translated.unwrap_or(true);

    executor.start_translate(window, entries, provider, target_lang, skip_translated)
}

/// 启动批处理导出
#[tauri::command]
pub async fn start_batch_export(
    window: tauri::Window,
    executor: tauri::State<'_, Arc<BatchExecutor>>,
    entries: Vec<BatchEntry>,
    output_dir: String,
    export_format: String,
) -> Result<String, String> {
    executor.start_export(window, entries, output_dir, export_format)
}

/// 获取当前批处理状态
#[tauri::command]
pub async fn get_batch_status(
    executor: tauri::State<'_, Arc<BatchExecutor>>,
) -> Result<Option<BatchStatus>, String> {
    Ok(executor.get_status())
}

/// 取消当前批处理
#[tauri::command]
pub async fn cancel_batch_job(
    executor: tauri::State<'_, Arc<BatchExecutor>>,
) -> Result<(), String> {
    executor.cancel();
    Ok(())
}

/// 扫描目录中的 ESP/ESM 文件
#[tauri::command]
pub async fn list_esp_files(dir: String) -> Result<Vec<String>, String> {
    let mut entries = Vec::new();
    let mut read_dir =
        std::fs::read_dir(&dir).map_err(|e| format!("Failed to read directory: {}", e))?;
    while let Some(entry) = read_dir
        .next()
        .transpose()
        .map_err(|e| format!("Read error: {}", e))?
    {
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "esp" || ext == "esm" {
                if let Some(s) = path.to_str() {
                    entries.push(s.to_string());
                }
            }
        }
    }
    entries.sort();
    Ok(entries)
}

#[tauri::command]
pub async fn auto_backup_sst(
    state: tauri::State<'_, Arc<AppState>>,
    request: AutoBackupRequest,
) -> Result<AutoBackupResponse, String> {
    let is_dirty = *state.is_dirty.lock().map_err(|e| e.to_string())?;
    if !is_dirty {
        return Ok(AutoBackupResponse {
            backup_path: None,
            total_backups: 0,
        });
    }

    let sst_path = std::path::Path::new(&request.sst_path);
    let parent = sst_path
        .parent()
        .ok_or_else(|| "Invalid SST path: no parent directory".to_string())?;
    let backup_dir = parent.join("backups");
    std::fs::create_dir_all(&backup_dir)
        .map_err(|e| format!("Failed to create backup dir: {}", e))?;

    let stem = sst_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("backup");
    // 从 UNIX 纪元秒数生成时间戳
    let epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let backup_name = format!("{}_{}.sst", stem, epoch);
    let backup_path = backup_dir.join(&backup_name);

    // 从当前字符串构建 SST
    let strings = state.strings.lock().map_err(|e| e.to_string())?;
    let old_data = state.sst_old_data.lock().map_err(|e| e.to_string())?;
    let mut entries = strings.clone();
    append_old_data_entries(&mut entries, &old_data);
    let sst = xt_core::sst::v8::SstDictionary::from_entries(entries);
    sst.save_to_file(backup_path.to_str().ok_or("Invalid backup path")?)
        .map_err(|e| format!("Failed to save backup: {}", e))?;

    // 轮转：保留最新的 max_backups 个备份
    let max_backups = request.max_backups.unwrap_or(10) as usize;
    let mut backup_files: Vec<std::path::PathBuf> = Vec::new();
    let mut read_dir =
        std::fs::read_dir(&backup_dir).map_err(|e| format!("Failed to read backup dir: {}", e))?;
    while let Some(entry) = read_dir
        .next()
        .transpose()
        .map_err(|e| format!("Read error: {}", e))?
    {
        let p = entry.path();
        if p.extension().and_then(|e| e.to_str()) == Some("sst") {
            backup_files.push(p);
        }
    }

    // 按修改时间降序排列（最新的在前）
    backup_files.sort_by(|a, b| {
        let ma = a.metadata().ok();
        let mb = b.metadata().ok();
        let ta = ma
            .and_then(|m| m.modified().ok())
            .unwrap_or(std::time::UNIX_EPOCH);
        let tb = mb
            .and_then(|m| m.modified().ok())
            .unwrap_or(std::time::UNIX_EPOCH);
        tb.cmp(&ta)
    });

    // 删除多余的旧备份
    for old in backup_files.iter().skip(max_backups) {
        let _ = std::fs::remove_file(old);
    }

    let total_backups = std::cmp::min(backup_files.len(), max_backups) as u32;

    Ok(AutoBackupResponse {
        backup_path: Some(backup_path.to_str().unwrap_or("").to_string()),
        total_backups,
    })
}

// ── BSA/BA2 Browser Commands ─────────────────────────────────────────

#[tauri::command]
pub async fn list_bsa_files(bsa_path: String) -> Result<BsaFileListDto, String> {
    let bsa = xt_core::bsa::BsaArchive::open(&bsa_path)
        .map_err(|e| format!("Failed to open BSA: {}", e))?;

    let archive_name = bsa.archive_name().unwrap_or("unknown.bsa").to_string();

    let folders: Vec<String> = bsa.folder_names().iter().map(|s| s.to_string()).collect();

    let files: Vec<BsaFileEntryDto> = bsa
        .list_all_files()
        .iter()
        .map(|e| BsaFileEntryDto {
            path: e.path.clone(),
            size: e.size,
            compressed: e.compressed,
            folder: e.folder.clone(),
        })
        .collect();

    Ok(BsaFileListDto {
        archive_name,
        version: bsa.version(),
        total_files: files.len() as u32,
        folders,
        files,
    })
}

#[tauri::command]
pub async fn list_ba2_files(ba2_path: String) -> Result<BsaFileListDto, String> {
    let ba2 = xt_core::ba2::Ba2Archive::open(&ba2_path)
        .map_err(|e| format!("Failed to open BA2: {}", e))?;

    let archive_name = ba2.archive_name().unwrap_or("unknown.ba2").to_string();

    let folders: Vec<String> = ba2.folder_names().iter().map(|s| s.to_string()).collect();

    let files: Vec<BsaFileEntryDto> = ba2
        .list_all_files()
        .iter()
        .map(|e| BsaFileEntryDto {
            path: e.path.clone(),
            size: e.size,
            compressed: e.compressed,
            folder: e.folder.clone(),
        })
        .collect();

    Ok(BsaFileListDto {
        archive_name,
        version: ba2.version(),
        total_files: files.len() as u32,
        folders,
        files,
    })
}

#[tauri::command]
pub async fn extract_bsa_file(
    bsa_path: String,
    file_path: String,
    output_dir: String,
) -> Result<String, String> {
    let bsa = xt_core::bsa::BsaArchive::open(&bsa_path)
        .map_err(|e| format!("Failed to open BSA: {}", e))?;

    let data = bsa
        .extract_file(&file_path)
        .map_err(|e| format!("Failed to extract '{}': {}", file_path, e))?;

    // 从文件路径确定输出文件名
    let file_name = std::path::Path::new(&file_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("extracted.bin");

    let output_path = std::path::Path::new(&output_dir).join(file_name);
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create output dir: {}", e))?;
    }

    std::fs::write(&output_path, &data).map_err(|e| format!("Failed to write output: {}", e))?;

    Ok(output_path.to_str().unwrap_or("").to_string())
}

#[tauri::command]
pub async fn extract_bsa_folder(
    bsa_path: String,
    folder: String,
    output_dir: String,
) -> Result<Vec<String>, String> {
    let bsa = xt_core::bsa::BsaArchive::open(&bsa_path)
        .map_err(|e| format!("Failed to open BSA: {}", e))?;

    let mut extracted: Vec<String> = Vec::new();

    for entry in bsa.list_all_files() {
        if entry.folder == folder {
            match bsa.extract_file(&entry.path) {
                Ok(data) => {
                    let file_name = std::path::Path::new(&entry.path)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown");
                    let output_path = std::path::Path::new(&output_dir).join(file_name);
                    if let Some(parent) = output_path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if let Err(e) = std::fs::write(&output_path, &data) {
                        log::warn!("Failed to write {}: {}", entry.path, e);
                    } else {
                        extracted.push(output_path.to_str().unwrap_or("").to_string());
                    }
                }
                Err(e) => {
                    log::warn!("Failed to extract {}: {}", entry.path, e);
                }
            }
        }
    }

    Ok(extracted)
}

#[tauri::command]
pub async fn extract_ba2_file(
    ba2_path: String,
    file_path: String,
    output_dir: String,
) -> Result<String, String> {
    let ba2 = xt_core::ba2::Ba2Archive::open(&ba2_path)
        .map_err(|e| format!("Failed to open BA2: {}", e))?;

    let data = ba2
        .extract_file(&file_path)
        .map_err(|e| format!("Failed to extract '{}': {}", file_path, e))?;

    let file_name = std::path::Path::new(&file_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("extracted.bin");

    let output_path = std::path::Path::new(&output_dir).join(file_name);
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create output dir: {}", e))?;
    }

    std::fs::write(&output_path, &data).map_err(|e| format!("Failed to write output: {}", e))?;

    Ok(output_path.to_str().unwrap_or("").to_string())
}

#[tauri::command]
pub async fn extract_ba2_folder(
    ba2_path: String,
    folder: String,
    output_dir: String,
) -> Result<Vec<String>, String> {
    let ba2 = xt_core::ba2::Ba2Archive::open(&ba2_path)
        .map_err(|e| format!("Failed to open BA2: {}", e))?;

    let mut extracted: Vec<String> = Vec::new();

    for entry in ba2.list_all_files() {
        if entry.folder == folder {
            match ba2.extract_file(&entry.path) {
                Ok(data) => {
                    let file_name = std::path::Path::new(&entry.path)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown");
                    let output_path = std::path::Path::new(&output_dir).join(file_name);
                    if let Some(parent) = output_path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if let Err(e) = std::fs::write(&output_path, &data) {
                        eprintln!("Failed to write {}: {}", entry.path, e);
                    } else {
                        extracted.push(output_path.to_str().unwrap_or("").to_string());
                    }
                }
                Err(e) => {
                    log::warn!("Failed to extract {}: {}", entry.path, e);
                }
            }
        }
    }

    Ok(extracted)
}

// ── Archive Injection（DP-06）─────────────────────────────────────

/// 替换归档内已存在文件（BSA/BA2 replacement injection）。
///
/// 安全流程：临时文件 → 重开校验 → 备份 → 原子替换。
#[tauri::command]
pub async fn inject_archive(
    request: InjectArchiveRequest,
) -> Result<InjectArchiveResponse, String> {
    use base64::Engine;

    let replacements: std::collections::HashMap<String, Vec<u8>> = request
        .replacements
        .iter()
        .map(|(k, v)| {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(v)
                .map_err(|e| format!("invalid base64 for {}: {}", k, e))?;
            Ok((k.clone(), bytes))
        })
        .collect::<Result<_, String>>()?;

    let result = xt_core::archive_inject::inject_archive(
        std::path::Path::new(&request.archive_path),
        &replacements,
        request.create_backup,
    )
    .map_err(|e| format!("Archive injection failed: {}", e))?;

    Ok(InjectArchiveResponse {
        injected: result.injected,
        not_found: result.not_found,
        backup_path: result.backup_path.map(|p| p.to_string_lossy().into_owned()),
        output_size: result.output_size,
    })
}

// ── PEX Commands ────────────────────────────────────────────────────

/// 解析 PEX 文件并提取可翻译的字符串
#[tauri::command]
pub async fn parse_pex_strings(
    pex_path: String,
    game: String,
) -> Result<PexScriptDto, String> {
    let mut file =
        std::fs::File::open(&pex_path).map_err(|e| format!("Failed to open PEX: {}", e))?;

    let script = xt_core::pex::parser::parse_pex(&mut file)
        .map_err(|e| format!("Failed to parse PEX: {}", e))?;

    let script_name = std::path::Path::new(&pex_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Load pexNoTransProc.txt from the explicit global game context; no silent fallback.
    let game_id = GameId::from_alias(&game).ok_or_else(|| format!("Unknown game: {}", game))?;
    let no_trans_procs = load_no_trans_procs(game_id);

    let translatable: Vec<PexTranslatableDto> = script
        .translatable
        .iter()
        .filter(|t| {
            // 过滤掉不可翻译的过程中的字符串
            if !t.function_name.is_empty() {
                let fn_lower = t.function_name.to_lowercase();
                if no_trans_procs.contains(&fn_lower) {
                    return false;
                }
            }
            true
        })
        .map(|t| PexTranslatableDto {
            object_name: t.object_name.clone(),
            state_name: t.state_name.clone(),
            function_name: t.function_name.clone(),
            string_type: t.string_type.clone(),
            source_text: t.source_text.clone(),
            translation: t.translation.clone(),
        })
        .collect();

    Ok(PexScriptDto {
        script_name,
        game_id: script.header.game_id,
        major_version: script.header.major_version,
        minor_version: script.header.minor_version,
        string_count: script.string_table.len() as u32,
        translatable,
    })
}

/// 反编译 PEX 文件为类 Papyrus 的伪代码
#[tauri::command]
pub async fn decompile_pex(
    pex_path: String,
) -> Result<xt_shared::dto::DecompilePexResponse, String> {
    let data = std::fs::read(&pex_path).map_err(|e| format!("Failed to read PEX: {}", e))?;

    let decompiled = xt_core::pex::decompile::decompile_pex(&data)
        .map_err(|e| format!("Failed to decompile PEX: {}", e))?;

    let script_name = decompiled
        .objects
        .first()
        .map(|o| o.name.clone())
        .unwrap_or_else(|| {
            std::path::Path::new(&pex_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string()
        });

    let object_count = decompiled.objects.len() as u32;
    let function_count = decompiled
        .objects
        .iter()
        .map(|o| o.states.iter().map(|s| s.functions.len()).sum::<usize>())
        .sum::<usize>() as u32;
    let instruction_count = decompiled
        .objects
        .iter()
        .flat_map(|o| o.states.iter())
        .flat_map(|s| s.functions.iter())
        .map(|f| f.instructions.len())
        .sum::<usize>() as u32;

    let pseudocode = xt_core::pex::decompile::emit_pseudocode(&decompiled);

    Ok(xt_shared::dto::DecompilePexResponse {
        script_name,
        object_count,
        function_count,
        instruction_count,
        pseudocode,
    })
}

/// 加载指定游戏的 pexNoTransProc.txt 过滤器，返回一个
/// 应排除在翻译之外的小写过程（procedure）名称集合。
fn load_no_trans_procs(game: GameId) -> std::collections::HashSet<String> {
    let path = std::path::Path::new("Data")
        .join(game.as_str())
        .join("pexNoTransProc.txt");
    if !path.exists() {
        return std::collections::HashSet::new();
    }
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return std::collections::HashSet::new(),
    };
    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim().to_lowercase();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                None
            } else {
                Some(trimmed)
            }
        })
        .collect()
}

/// 使用更新后的翻译编译 PEX 文件
///
/// 接受原始 PEX 脚本和翻译字符串列表，
/// 写入一个带有更新后字符串表的新 PEX 文件。
#[tauri::command]
pub async fn compile_pex(
    pex_path: String,
    output_path: String,
    translations: Vec<PexTranslatableDto>,
) -> Result<String, String> {
    use std::fs::File;
    use xt_core::pex::compile::compile_pex;

    // 解析原始 PEX
    let mut file = File::open(&pex_path).map_err(|e| format!("Failed to open PEX: {}", e))?;
    let script = xt_core::pex::parser::parse_pex(&mut file)
        .map_err(|e| format!("Failed to parse PEX: {}", e))?;

    // 将 DTO 转换为 PexTranslatableString
    let pex_translations: Vec<PexTranslatableString> = translations
        .iter()
        .map(|t| PexTranslatableString {
            object_name: t.object_name.clone(),
            state_name: t.state_name.clone(),
            function_name: t.function_name.clone(),
            string_type: t.string_type.clone(),
            source_text: t.source_text.clone(),
            translation: t.translation.clone(),
        })
        .collect();

    // 使用实际翻译编译
    let result = compile_pex(&script, &pex_translations, &output_path)
        .map_err(|e| format!("Failed to compile PEX: {}", e))?;

    Ok(result.path)
}

// ── ESP Compare Commands ───────────────────────────────────────────

use xt_core::esp::compare::{self, CompareEntry, EspComparison};

/// 将内部对比结果转换为 DTO
fn comparison_to_dto(comp: EspComparison) -> EspCompareResultDto {
    let sig_to_str = |sig: &[u8; 4]| String::from_utf8_lossy(sig).to_string();

    let old_by_id: HashMap<u32, &CompareEntry> =
        comp.old_strings.iter().map(|e| (e.id, e)).collect();
    let new_by_id: HashMap<u32, &CompareEntry> =
        comp.new_strings.iter().map(|e| (e.id, e)).collect();

    let to_pair = |new_id: u32, old_id: u32| -> EspComparePairDto {
        let new_e = new_by_id.get(&new_id).copied();
        let old_e = old_by_id.get(&old_id).copied();
        let record_sig = new_e
            .or(old_e)
            .map(|e| sig_to_str(&e.record_sig))
            .unwrap_or_default();
        let field_sig = new_e
            .or(old_e)
            .map(|e| sig_to_str(&e.field_sig))
            .unwrap_or_default();

        EspComparePairDto {
            new_id,
            old_id,
            source: new_e.map(|e| e.source.clone()).unwrap_or_default(),
            record_sig,
            field_sig,
            old_source: old_e.map(|e| e.source.clone()).unwrap_or_default(),
            new_source: new_e.map(|e| e.source.clone()).unwrap_or_default(),
        }
    };

    let identical: Vec<EspComparePairDto> = comp
        .matched_pairs
        .iter()
        .map(|(&new_id, &old_id)| to_pair(new_id, old_id))
        .collect();

    let added: Vec<EspComparePairDto> = comp
        .added
        .iter()
        .map(|&new_id| to_pair(new_id, 0))
        .collect();

    let removed: Vec<EspComparePairDto> = comp
        .removed
        .iter()
        .map(|&old_id| {
            let mut pair = to_pair(0, old_id);
            pair.source = pair.old_source.clone();
            pair
        })
        .collect();

    let modified: Vec<EspComparePairDto> = comp
        .modified_pairs
        .iter()
        .map(|(&new_id, &old_id)| to_pair(new_id, old_id))
        .collect();

    EspCompareResultDto {
        identical_count: identical.len(),
        added_count: added.len(),
        removed_count: removed.len(),
        modified_count: modified.len(),
        identical,
        added,
        removed,
        modified,
    }
}

/// 对比两个 ESP/ESM 文件并返回字符串对映射
#[tauri::command]
pub async fn compare_esp_files(
    old_esp_path: String,
    new_esp_path: String,
    data_dir: Option<String>,
    game: Option<String>,
) -> Result<EspCompareResultDto, String> {
    let game_id = match game.as_deref() {
        Some("Skyrim") => GameId::Skyrim,
        Some("SkyrimSE") => GameId::SkyrimSE,
        Some("Fallout4") => GameId::Fallout4,
        Some("FalloutNV") => GameId::FalloutNV,
        Some("Fallout76") => GameId::Fallout76,
        Some("Starfield") => GameId::Starfield,
        None => GameId::SkyrimSE,
        Some(g) => return Err(format!("Unknown game: {}", g)),
    };
    tokio::task::spawn_blocking(move || {
        let comp =
            compare::compare_esp_files(&old_esp_path, &new_esp_path, data_dir.as_deref(), game_id)
                .map_err(|e| format!("Failed to compare ESP files: {}", e))?;
        Ok(comparison_to_dto(comp))
    })
    .await
    .map_err(|e| format!("ESP comparison task failed: {}", e))?
}

// ── MCM Commands ────────────────────────────────────────────────────

use xt_core::mcm::{self, types::McmEncoding};

/// 将内部 McmFile 转换为 DTO
fn mcm_file_to_dto(file: &xt_core::mcm::McmFile) -> McmFileDto {
    McmFileDto {
        path: file.path.clone(),
        entry_count: file.entries.len() as u32,
        encoding: match &file.encoding {
            McmEncoding::Utf16Le => "UTF-16LE".to_string(),
            McmEncoding::Utf16Be => "UTF-16BE".to_string(),
            McmEncoding::Utf8 => "UTF-8".to_string(),
            McmEncoding::Ansi(cp) => format!("windows-{}", cp),
        },
        entries: file
            .entries
            .iter()
            .map(|e| McmEntryDto {
                id: e.id.clone(),
                source: e.source.clone(),
                translation: e.translation.clone(),
                line_index: e.line_index as u32,
                byte_offset: e.byte_offset as u32,
            })
            .collect(),
    }
}

/// 加载并解析 MCM 翻译文件
#[tauri::command]
pub async fn load_mcm_file(mcm_path: String) -> Result<McmFileDto, String> {
    let file =
        mcm::parse_mcm_file(&mcm_path).map_err(|e| format!("Failed to parse MCM file: {}", e))?;
    Ok(mcm_file_to_dto(&file))
}

/// 保存带有更新翻译的 MCM 文件
#[tauri::command]
pub async fn save_mcm_file(request: McmSaveRequest) -> Result<(), String> {
    // 需要原始 McmFile 以保留编码和 normalized_lines。
    // 加载它，从请求中应用翻译，然后保存。
    let mut file = mcm::parse_mcm_file(&request.path)
        .map_err(|e| format!("Failed to open MCM file for save: {}", e))?;

    for dto_entry in &request.entries {
        if let Some(entry) = file
            .entries
            .iter_mut()
            .find(|e| e.line_index as u32 == dto_entry.line_index)
        {
            entry.translation = dto_entry.translation.clone();
        }
    }

    mcm::save_mcm_file(&request.path, &file)
        .map_err(|e| format!("Failed to save MCM file: {}", e))?;
    Ok(())
}

/// 将当前的 MCM 条目与参考 MCM 文件进行对比，并根据指定的覆盖策略应用翻译。
#[tauri::command]
pub async fn mcm_compare(request: McmCompareRequest) -> Result<McmCompareResult, String> {
    use std::collections::HashMap;

    // 解析参考 MCM 文件
    let reference_file = mcm::parse_mcm_file(&request.reference_path)
        .map_err(|e| format!("Failed to parse reference MCM file: {}", e))?;

    // 从参考条目构建 HashMap（按 id 索引，O(1) 查找）
    let reference_by_id: HashMap<&str, &xt_core::mcm::McmEntry> = reference_file
        .entries
        .iter()
        .map(|e| (e.id.as_str(), e))
        .collect();

    let mut matched: u32 = 0;
    let mut unmatched: u32 = 0;
    let mut updated_entries: Vec<McmEntryDto> = Vec::new();

    // 启发式规则：翻译为"部分翻译"的条件是非空、与源文本不同，
    // 且明显短于源文本（不足源文本长度的 30%）
    fn is_partial(source: &str, translation: &str) -> bool {
        if translation.is_empty() {
            return false;
        }
        if translation == source {
            return false;
        }
        let ratio = translation.len() as f32 / source.len().max(1) as f32;
        ratio < 0.3
    }

    fn should_update(current_trans: &str, policy: &McmComparePolicy, source: &str) -> bool {
        match policy {
            McmComparePolicy::All => true,
            McmComparePolicy::NoTrans => current_trans.is_empty(),
            McmComparePolicy::NoTransAndPartial => {
                current_trans.is_empty() || is_partial(source, current_trans)
            }
            McmComparePolicy::PartialOnly => is_partial(source, current_trans),
        }
    }

    for entry in &request.entries {
        if let Some(ref_entry) = reference_by_id.get(entry.id.as_str()) {
            matched += 1;

            if should_update(&entry.translation, &request.policy, &entry.source) {
                // 从参考条目复制翻译
                updated_entries.push(McmEntryDto {
                    id: entry.id.clone(),
                    source: entry.source.clone(),
                    translation: ref_entry.translation.clone(),
                    line_index: entry.line_index,
                    byte_offset: entry.byte_offset,
                });
            }
        } else {
            unmatched += 1;
        }
    }

    Ok(McmCompareResult {
        matched,
        unmatched,
        updated_entries,
    })
}

// ── Data Config Commands ─────────────────────────────────────────────

use xt_core::data_config::{
    parse_ctda_func, parse_dial_sub_type, parse_emote_definition, parse_field_size_ref,
};

/// Load and parse Data/<Game>/ 配置文件
#[tauri::command]
pub async fn load_data_configs(game: String) -> Result<DataConfigsDto, String> {
    let data_dir = std::path::Path::new("Data");
    let game_id = GameId::from_alias(&game).ok_or_else(|| format!("Unknown game: {}", game))?;
    let game_dir = data_dir.join(game_id.as_str());

    // 解析 ctdaFunc.txt
    let ctda_funcs: Vec<CtdaFuncDto> = {
        let path = game_dir.join("ctdaFunc.txt");
        if path.exists() {
            parse_ctda_func(&path)
                .into_iter()
                .map(|(id, func)| CtdaFuncDto {
                    id,
                    name: func.name,
                    params: func.params,
                })
                .collect()
        } else {
            Vec::new()
        }
    };

    // 解析 fieldSizeRef.txt
    let field_size_ref: HashMap<String, FieldSizeInfoDto> = {
        let path = game_dir.join("fieldSizeRef.txt");
        if path.exists() {
            parse_field_size_ref(&path)
                .into_iter()
                .map(|(key, info)| {
                    (
                        key,
                        FieldSizeInfoDto {
                            max_size: info.max_size,
                            can_wrap: info.can_wrap,
                        },
                    )
                })
                .collect()
        } else {
            HashMap::new()
        }
    };

    // 解析 DialSubType.txt
    let dial_sub_type: HashMap<String, String> = {
        let path = game_dir.join("DialSubType.txt");
        if path.exists() {
            parse_dial_sub_type(&path)
                .into_iter()
                .map(|(id, name)| (format!("{:08X}", id), name))
                .collect()
        } else {
            HashMap::new()
        }
    };

    // 解析 EmoteDefinition.txt
    let emote_definition: HashMap<String, String> = {
        let path = game_dir.join("EmoteDefinition.txt");
        if path.exists() {
            parse_emote_definition(&path)
                .into_iter()
                .map(|(id, name)| (format!("{:08X}", id), name))
                .collect()
        } else {
            HashMap::new()
        }
    };

    Ok(DataConfigsDto {
        ctda_funcs,
        field_size_ref,
        dial_sub_type,
        emote_definition,
    })
}

// ── FUZ Commands ────────────────────────────────────────────────────

#[tauri::command]
pub async fn scan_fuz_directory(
    state: tauri::State<'_, Arc<AppState>>,
    voice_dir: String,
) -> Result<FuzScanResponse, String> {
    let strings = state.strings.lock().map_err(|e| e.to_string())?;

    let mut mappings: Vec<FuzMapping> = Vec::new();
    let mut fuz_paths: Vec<std::path::PathBuf> = Vec::new();

    let walk_dir = std::path::Path::new(&voice_dir);
    if !walk_dir.is_dir() {
        return Err(format!("Not a directory: {}", voice_dir));
    }

    fn collect_fuz(
        dir: &std::path::Path,
        out: &mut Vec<std::path::PathBuf>,
    ) -> std::io::Result<()> {
        if dir.is_dir() {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    collect_fuz(&path, out)?;
                } else if path.extension().map_or(false, |e| e == "fuz") {
                    out.push(path);
                }
            }
        }
        Ok(())
    }

    collect_fuz(walk_dir, &mut fuz_paths).map_err(|e| format!("Failed to scan: {}", e))?;

    let total_fuz = fuz_paths.len() as u32;

    for fuz_path in &fuz_paths {
        let stem = fuz_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let parts: Vec<&str> = stem.split('_').collect();
        if parts.is_empty() {
            continue;
        }

        if let Ok(resp_id) = u32::from_str_radix(parts[0], 16) {
            let parsed = xt_core::fuz::FuzFile::parse(
                &mut std::fs::File::open(fuz_path).map_err(|e| format!("Failed: {}", e))?,
            );
            let parse_ok = parsed.is_ok();
            let dur = parsed.as_ref().map(|f| f.duration_secs).unwrap_or(0.0);
            let has_lip = parsed
                .as_ref()
                .map(|f| f.lip_data.is_some())
                .unwrap_or(false);

            let dialog_text = strings
                .iter()
                .find(|s| s.esp_ptr.str_id == resp_id as i32)
                .map(|s| s.source.clone())
                .unwrap_or_default();

            if !dialog_text.is_empty() || dur > 0.0 {
                mappings.push(FuzMapping {
                    response_id: resp_id,
                    dialog_text,
                    fuz_file: fuz_path.to_str().unwrap_or("").to_string(),
                    duration_secs: dur,
                    has_lip,
                    parse_ok,
                });
            }
        }
    }

    mappings.sort_by_key(|m| m.response_id);
    Ok(FuzScanResponse {
        fuz_mappings: mappings,
        total_fuz_files: total_fuz,
    })
}

#[tauri::command]
pub async fn get_fuz_audio_data(fuz_path: String) -> Result<Vec<u8>, String> {
    let mut file =
        std::fs::File::open(&fuz_path).map_err(|e| format!("Failed to open FUZ: {}", e))?;
    let fuz =
        xt_core::fuz::FuzFile::parse(&mut file).map_err(|e| format!("Failed to parse: {}", e))?;
    Ok(fuz.wav_data)
}

#[tauri::command]
pub async fn get_fuz_lip_data(fuz_path: String) -> Result<FuzLipDataResponse, String> {
    let mut file =
        std::fs::File::open(&fuz_path).map_err(|e| format!("Failed to open FUZ: {}", e))?;
    let fuz =
        xt_core::fuz::FuzFile::parse(&mut file).map_err(|e| format!("Failed to parse: {}", e))?;

    let lip_data = fuz.lip_data.map(|ld| LipDataDto {
        version: ld.version,
        keyframes: ld
            .keyframes
            .into_iter()
            .map(|kf| LipKeyframeDto {
                time: kf.time,
                shape: kf.shape,
            })
            .collect(),
    });

    Ok(FuzLipDataResponse {
        lip_data,
        duration_secs: fuz.duration_secs,
        sample_rate: fuz.sample_rate,
        channels: fuz.channels,
    })
}

// ── Dialog Tree Commands ─────────────────────────────────────────────

#[tauri::command]
pub async fn build_dialog_tree(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<DialogTreeDto, String> {
    let strings = state.strings.lock().map_err(|e| e.to_string())?;

    // 按父 DIAL FormID 分组 INFO 字符串
    let mut npc_groups: std::collections::HashMap<String, Vec<DialogInfoDto>> =
        std::collections::HashMap::new();

    for s in strings.iter() {
        let record_sig = String::from_utf8_lossy(&s.record_sig);

        // 聚焦 INFO 记录（对话回复）和 NPC_ 记录（用于名称关联）
        if record_sig == "INFO" && !s.source.is_empty() {
            let parent_form_id = s.parent_form_id;

            // 构建对话条目
            let entry = DialogInfoDto {
                id: s.id,
                form_id: parent_form_id,
                source: s.source.clone(),
                translation: s.translation.clone(),
                dialog_text: s.source.clone(),
            };

            // 按父 DIAL form_id 字符串键分组
            let key = format!("DIAL_{:08X}", parent_form_id);
            npc_groups.entry(key).or_default().push(entry);
        }
    }

    // 同时关联 NPC_ 记录字符串（名称）与对话
    let mut npc_names: std::collections::HashMap<u32, String> = std::collections::HashMap::new();
    for s in strings.iter() {
        let sig = String::from_utf8_lossy(&s.record_sig);
        if sig == "NPC_" {
            npc_names.insert(s.esp_ptr.form_id, s.source.clone());
        }
    }

    // 构建响应
    let npcs: Vec<NpcDialogDto> = npc_groups
        .into_iter()
        .map(|(key, dialogues)| {
            let edid = key.clone();
            NpcDialogDto {
                npc_edid: edid,
                dialogues,
            }
        })
        .collect();

    Ok(DialogTreeDto { npcs })
}

// ── TCSC Commands ───────────────────────────────────────────────────

use xt_core::tcsc;

/// 从 vocabulary.txt 以及游戏 Strings 文件中加载词汇表。
///
/// 返回源文本→翻译对的数量，并使它们可用于启发式搜索增强。
#[tauri::command]
pub async fn load_vocabulary(
    state: tauri::State<'_, Arc<AppState>>,
    strings_dir: String,
    source_lang: String,
    target_lang: String,
    game: String,
) -> Result<VocabularyInfo, String> {
    let game_id = GameId::from_alias(&game).ok_or_else(|| format!("Unknown game: {}", game))?;

    let state_clone = state.inner().clone();
    let result = tokio::task::spawn_blocking(move || {
        let data_dir = std::path::Path::new("Data");
        let game_dir = data_dir.join(game_id.as_str());

        let vocab_path = game_dir.join("vocabulary.txt");
        if !vocab_path.exists() {
            return Ok(VocabularyInfo {
                pair_count: 0,
                base_names: vec![],
            });
        }

        let names = xt_core::vocabulary::parse_vocabulary_file(&vocab_path)
            .map_err(|e| format!("Failed to parse vocabulary.txt: {}", e))?;
        let base_names = names.clone();

        let codepage_path = game_dir.join("codepage.txt");
        let codepage_table = if codepage_path.exists() {
            CodepageTable::load_from_file(&codepage_path).ok()
        } else {
            None
        };

        let strings_dir_path = std::path::Path::new(&strings_dir);
        let vocab = xt_core::vocabulary::Vocabulary::load(
            &names,
            strings_dir_path,
            &source_lang,
            &target_lang,
            codepage_table.as_ref(),
        );

        let pair_count = vocab.len();
        let pairs = vocab.pairs().to_vec();

        // 将词汇存储到 AppState 中，用于启发式搜索增强
        *state_clone.vocabulary.lock().map_err(|e| e.to_string())? = pairs;

        Ok(VocabularyInfo {
            pair_count,
            base_names,
        })
    })
    .await
    .map_err(|e| format!("Vocabulary loading task failed: {}", e))?
    .map_err(|e: String| e)?;

    Ok(result)
}

/// Result of loading a vocabulary
#[derive(serde::Serialize)]
pub struct VocabularyInfo {
    pub pair_count: usize,
    pub base_names: Vec<String>,
}

/// 在简体中文与繁体中文之间转换文本
#[tauri::command]
pub async fn tcsc_convert(text: String, direction: String) -> Result<String, String> {
    let result = match direction.as_str() {
        "to_simplified" => tcsc::to_simplified(&text),
        "to_traditional" => tcsc::to_traditional(&text),
        _ => return Err("Invalid direction: use 'to_simplified' or 'to_traditional'".into()),
    };
    Ok(result)
}

/// 翻转从右到左（RTL）的文本，用于阿拉伯语/希伯来语的显示。
///
/// 逐行处理文本：翻转阿拉伯字符块并镜像括号符号。返回翻转后的文本，如果未找到阿拉伯字符则返回错误。
#[tauri::command]
pub async fn rtl_reverse(text: String) -> Result<String, String> {
    xt_core::rtl::reverse_rtl_multiline(&text)
        .ok_or_else(|| "No Arabic characters found in text".into())
}

/// 整形阿拉伯语文本：将逻辑顺序的字符转换为呈现形式。
#[tauri::command]
pub async fn shape_arabic(text: String) -> Result<String, String> {
    Ok(xt_core::rtl::shape_arabic(&text))
}

/// 还原阿拉伯语文本：将呈现形式重新转换为逻辑基础字符。
#[tauri::command]
pub async fn deshape_arabic(text: String) -> Result<String, String> {
    Ok(xt_core::rtl::deshape_arabic(&text))
}

/// 批量转换所有（或指定的）字符串的译文。
///
/// 原地转换每个匹配字符串的 `translation` 字段。
/// 返回更新后的字符串 ID 列表。
#[tauri::command]
pub async fn tcsc_batch_convert(
    state: tauri::State<'_, Arc<AppState>>,
    direction: String,
    ids: Option<Vec<u32>>,
) -> Result<Vec<u32>, String> {
    let dir_fn: fn(&str) -> String = match direction.as_str() {
        "to_simplified" => tcsc::to_simplified,
        "to_traditional" => tcsc::to_traditional,
        _ => return Err("Invalid direction: use 'to_simplified' or 'to_traditional'".into()),
    };

    let mut strings = state.strings.lock().map_err(|e| e.to_string())?;
    let mut updated = Vec::new();

    for sk in strings.iter_mut() {
        // 如果指定了 ID，仅转换指定的；否则转换所有非空翻译
        if let Some(ref filter_ids) = ids {
            if !filter_ids.contains(&sk.id) {
                continue;
            }
        }
        if sk.translation.is_empty() {
            continue;
        }
        let converted = dir_fn(&sk.translation);
        if converted != sk.translation {
            sk.translation = converted;
            sk.params.set(SkyStringParams::TRANSLATED, true);
            updated.push(sk.id);
        }
    }

    if !updated.is_empty() {
        *state.is_dirty.lock().map_err(|e| e.to_string())? = true;
    }

    Ok(updated)
}

/// 对比源文本与目标（翻译）文本字符串。
///
/// 模式：
/// - "diff"：将源文本 != 翻译文本（哈希不匹配）的字符串标记为未完成
/// - "same"：将源文本 == 翻译文本（哈希匹配）的字符串标记为未完成
///
/// 仅影响当前已翻译或已验证的字符串。
/// 返回被标记的字符串数量。
#[tauri::command]
pub async fn compare_source_dest(
    state: tauri::State<'_, Arc<AppState>>,
    mode: String,
) -> Result<u32, String> {
    let is_diff = match mode.as_str() {
        "diff" => true,
        "same" => false,
        _ => return Err("Invalid mode: use 'diff' or 'same'".into()),
    };

    let mut strings = state.strings.lock().map_err(|e| e.to_string())?;
    let mut count = 0u32;

    for sk in strings.iter_mut() {
        // 仅处理已翻译或已验证的字符串（与 Delphi 行为一致）
        if !sk.params.is_translated() && !sk.params.is_validated() {
            continue;
        }
        let matches = if is_diff {
            sk.hash != sk.hash_trans
        } else {
            sk.hash == sk.hash_trans
        };
        if matches {
            sk.params.set(SkyStringParams::INCOMPLETE_TRANS, true);
            sk.params.set(SkyStringParams::TRANSLATED, false);
            sk.params.set(SkyStringParams::VALIDATED, false);
            count += 1;
        }
    }

    if count > 0 {
        *state.is_dirty.lock().map_err(|e| e.to_string())? = true;
    }

    Ok(count)
}

/// 对选定的字符串应用工具箱文本转换。
///
/// `tool`：其一为 "uppercase_all"、"lowercase_all"、"uppercase_first"、"title_case"、
///         "fix_alias"、"add_header"、"trim"
/// `target`："source" | "translation" | "both"
/// `ids`：要操作的字符串 ID（为空 = 所有字符串）
/// `header_text`："add_header" 工具的前缀文本
#[tauri::command]
pub async fn toolbox_transform(
    state: tauri::State<'_, Arc<AppState>>,
    tool: String,
    target: String,
    ids: Vec<u32>,
    header_text: Option<String>,
) -> Result<u32, String> {
    let tool_type = xt_core::toolbox::ToolType::from_str(&tool)
        .ok_or_else(|| format!("Unknown tool: {}", tool))?;

    let mut strings = state.strings.lock().map_err(|e| e.to_string())?;
    let mut count = 0u32;

    let apply_to_source = target == "source" || target == "both";
    let apply_to_translation = target == "translation" || target == "both";

    let id_set: Option<std::collections::HashSet<u32>> = if ids.is_empty() {
        None
    } else {
        Some(ids.iter().copied().collect())
    };

    for sk in strings.iter_mut() {
        if let Some(ref id_set) = id_set {
            if !id_set.contains(&sk.id) {
                continue;
            }
        }
        if sk.params.is_locked() {
            continue;
        }

        let mut modified = false;

        if apply_to_source {
            let new_source = xt_core::toolbox::apply_tool(
                tool_type,
                &sk.source,
                &sk.source,
                header_text.as_deref(),
            );
            if new_source != sk.source {
                sk.source = new_source;
                modified = true;
            }
        }

        if apply_to_translation {
            let new_trans = xt_core::toolbox::apply_tool(
                tool_type,
                &sk.translation,
                &sk.source,
                header_text.as_deref(),
            );
            if new_trans != sk.translation {
                sk.translation = new_trans;
                sk.params.set(SkyStringParams::TRANSLATED, true);
                sk.params.set(SkyStringParams::INCOMPLETE_TRANS, false);
                modified = true;
            }
        }

        if modified {
            count += 1;
        }
    }

    if count > 0 {
        *state.is_dirty.lock().map_err(|e| e.to_string())? = true;
    }

    Ok(count)
}

// ── Toolbox Exception Words ──────────────────────────────────────

use xt_core::toolbox::{get_exception_words, load_exception_words};

/// 从配置中加载工具箱例外词，并应用到运行时。
/// 在应用程序启动且加载配置后调用。
#[tauri::command]
pub async fn toolbox_load_exception_words(
    words: Option<String>,
) -> Result<String, String> {
    if let Some(ref w) = words {
        load_exception_words(w);
    }
    Ok("ok".to_string())
}

/// 获取例外词列表。
#[tauri::command]
pub async fn toolbox_get_exception_words() -> Result<Vec<String>, String> {
    Ok(get_exception_words())
}

// ── Spell Check Commands ───────────────────────────────────────────

use xt_shared::dto::{MergeStatsDto, SpellCheckConfigDto, SpellCheckResultDto, SpellFaultDto};

/// 加载 Hunspell DLL 和词典以进行拼写检查。
#[tauri::command]
pub async fn spell_check_load(
    state: tauri::State<'_, Arc<AppState>>,
    dll_path: String,
    dict_dir: String,
    dict_name: String,
) -> Result<SpellCheckConfigDto, String> {
    let mut checker = state.spell_checker.lock().map_err(|e| e.to_string())?;
    checker.load(&dll_path, &dict_dir, &dict_name)?;
    checker.config.active = true;

    let config = checker.config.clone();
    Ok(SpellCheckConfigDto {
        available_dictionaries: xt_core::spell::SpellChecker::scan_dictionaries(&dict_dir),
        current_dictionary: config.current_dictionary,
        active: config.active,
        loaded: config.loaded,
    })
}

/// 卸载拼写检查器。
#[tauri::command]
pub async fn spell_check_unload(state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    let mut checker = state.spell_checker.lock().map_err(|e| e.to_string())?;
    checker.unload();
    Ok(())
}

/// 切换拼写检查的启用状态。
#[tauri::command]
pub async fn spell_check_toggle(state: tauri::State<'_, Arc<AppState>>) -> Result<bool, String> {
    let mut checker = state.spell_checker.lock().map_err(|e| e.to_string())?;
    checker.config.active = !checker.config.active;
    Ok(checker.config.active)
}

/// 获取拼写检查配置（可用词典、状态）。
#[tauri::command]
pub async fn spell_check_config(
    state: tauri::State<'_, Arc<AppState>>,
    dict_dir: String,
) -> Result<SpellCheckConfigDto, String> {
    let checker = state.spell_checker.lock().map_err(|e| e.to_string())?;
    Ok(SpellCheckConfigDto {
        available_dictionaries: xt_core::spell::SpellChecker::scan_dictionaries(&dict_dir),
        current_dictionary: checker.config.current_dictionary.clone(),
        active: checker.config.active,
        loaded: checker.config.loaded,
    })
}

/// 分析文本的拼写错误。返回错误单词的位置。
#[tauri::command]
pub async fn spell_check_text(
    state: tauri::State<'_, Arc<AppState>>,
    text: String,
) -> Result<SpellCheckResultDto, String> {
    let mut checker = state.spell_checker.lock().map_err(|e| e.to_string())?;
    let active = checker.is_active();
    let result = checker.analyze(&text);
    Ok(SpellCheckResultDto {
        faults: result
            .fault_words
            .iter()
            .map(|w| SpellFaultDto {
                word: w.word.clone(),
                start_byte: w.start_byte,
                end_byte: w.end_byte,
            })
            .collect(),
        total_words: result.total_words,
        fault_ratio_locked: result.fault_ratio_locked,
        active,
    })
}

/// 获取单词的拼写建议。
#[tauri::command]
pub async fn spell_check_suggestions(
    state: tauri::State<'_, Arc<AppState>>,
    word: String,
) -> Result<Vec<String>, String> {
    let checker = state.spell_checker.lock().map_err(|e| e.to_string())?;
    Ok(checker.suggestions(&word))
}

/// 将单词添加到拼写检查的忽略列表中。
#[tauri::command]
pub async fn spell_check_ignore(
    state: tauri::State<'_, Arc<AppState>>,
    word: String,
    ignore_path: String,
) -> Result<(), String> {
    let mut checker = state.spell_checker.lock().map_err(|e| e.to_string())?;
    let resolved_ignore_path = checker.resolved_ignore_path(&ignore_path);
    checker.add_ignore(&word);
    checker
        .save_ignore_list(&resolved_ignore_path.to_string_lossy())
        .map_err(|e| e.to_string())
}

// ── Header Processor Commands ──────────────────────────────────────

use xt_core::header_processor::{HeaderApplyResult, HeaderRuleDto};

/// 从 INI 文件加载头部处理规则。
#[tauri::command]
pub async fn header_rules_load(
    state: tauri::State<'_, Arc<AppState>>,
    path: String,
) -> Result<Vec<HeaderRuleDto>, String> {
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read rules file: {}", e))?;
    let rule_set = xt_core::header_processor::HeaderRuleSet::from_ini_text(&text);
    let dtos: Vec<HeaderRuleDto> = rule_set
        .rules
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let mut d = HeaderRuleDto::from(r);
            d.index = i;
            d
        })
        .collect();
    *state.header_rules.lock().map_err(|e| e.to_string())? = rule_set;
    Ok(dtos)
}

/// 获取当前已加载的规则。
#[tauri::command]
pub async fn header_rules_list(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<HeaderRuleDto>, String> {
    let rules = state.header_rules.lock().map_err(|e| e.to_string())?;
    Ok(rules
        .rules
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let mut d = HeaderRuleDto::from(r);
            d.index = i;
            d
        })
        .collect())
}

/// 通过索引切换规则’启用状态。
#[tauri::command]
pub async fn header_rules_toggle(
    state: tauri::State<'_, Arc<AppState>>,
    index: usize,
    enabled: bool,
) -> Result<(), String> {
    let mut rules = state.header_rules.lock().map_err(|e| e.to_string())?;
    if let Some(rule) = rules.rules.get_mut(index) {
        rule.enabled = enabled;
    }
    Ok(())
}

/// 将所有启用的头部规则应用到已加载的字符串中。
/// 返回匹配了至少一条规则的字符串数量。
#[tauri::command]
pub async fn header_rules_apply(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<HeaderApplyResult, String> {
    let rules = state.header_rules.lock().map_err(|e| e.to_string())?;
    let mut strings = state.strings.lock().map_err(|e| e.to_string())?;

    // 从记录树构建 EDID 查找表（如果可用）
    let edid_map = {
        let esp_file = state.esp_file.lock().map_err(|e| e.to_string())?;
        if let Some(ref ef) = *esp_file {
            let mut map = std::collections::HashMap::new();
            fn collect_edids(
                groups: &[xt_core::esp::record_tree::EspGrup],
                map: &mut std::collections::HashMap<u32, String>,
            ) {
                for grup in groups {
                    for rec in &grup.records {
                        if let Some(ref edid) = rec.editor_id {
                            map.insert(rec.form_id, edid.clone());
                        }
                    }
                    collect_edids(&grup.children, map);
                }
            }
            collect_edids(&ef.top_level_grups, &mut map);
            map
        } else {
            std::collections::HashMap::new()
        }
    };

    let total = rules.rules.len();
    let enabled = rules.rules.iter().filter(|r| r.enabled).count();
    let mut matched = 0u32;

    for sk in strings.iter_mut() {
        if sk.params.is_locked() || sk.params.is_translated() {
            continue;
        }
        let record_sig = String::from_utf8_lossy(&sk.record_sig).to_string();
        let field_sig = String::from_utf8_lossy(&sk.field_sig).to_string();
        let form_id = sk.esp_ptr.form_id;
        let edid = edid_map.get(&form_id).cloned().unwrap_or_default();

        if let Some(new_text) = rules.apply_rules(
            &record_sig,
            &field_sig,
            &edid,
            form_id,
            &[],
            &sk.translation,
            &sk.source,
        ) {
            sk.translation = new_text;
            sk.params.set(SkyStringParams::INCOMPLETE_TRANS, true);
            matched += 1;
        }
    }

    if matched > 0 {
        *state.is_dirty.lock().map_err(|e| e.to_string())? = true;
    }

    Ok(HeaderApplyResult {
        total_rules: total,
        enabled_rules: enabled,
        strings_matched: matched,
    })
}

// ── Header Batch Wizard Command ────────────────────────────────────

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct HeaderBatchConfig {
    pub source_dir: String,
    pub game_id: String,
    pub data_dir: String,
    pub create_backup: bool,
}

#[derive(Clone, serde::Serialize)]
pub struct HeaderBatchProgress {
    pub current: usize,
    pub total: usize,
    pub file_path: String,
    pub strings_matched: u32,
    pub stage: String,
    #[serde(default)]
    pub detail_count: Option<usize>,
    pub message: String,
}

#[derive(Clone, serde::Serialize)]
pub struct HeaderBatchComplete {
    pub total_files: usize,
    pub success: usize,
    pub failed: usize,
    pub total_strings_matched: u32,
    pub duration_ms: u64,
    pub is_cancelled: bool,
    pub errors: Vec<String>,
}

/// 使用当前加载的头部规则处理目录中的所有 ESP 文件。
#[tauri::command]
pub async fn header_batch_process(
    window: tauri::Window,
    state: tauri::State<'_, Arc<AppState>>,
    config: HeaderBatchConfig,
) -> Result<HeaderBatchComplete, String> {
    let start = std::time::Instant::now();

    // 收集 ESP 文件
    let dir = std::path::Path::new(&config.source_dir);
    if !dir.is_dir() {
        return Err("Source directory does not exist".into());
    }
    let mut esp_files: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| format!("Read dir error: {}", e))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
            let ext_lower = ext.to_lowercase();
            ext_lower == "esp" || ext_lower == "esm"
        })
        .collect();
    esp_files.sort();

    let game_id: xt_core::types::game_id::GameId = match config.game_id.as_str() {
        "Skyrim" => xt_core::types::game_id::GameId::Skyrim,
        "SkyrimSE" => xt_core::types::game_id::GameId::SkyrimSE,
        "Fallout4" => xt_core::types::game_id::GameId::Fallout4,
        "FalloutNV" => xt_core::types::game_id::GameId::FalloutNV,
        "Fallout76" => xt_core::types::game_id::GameId::Fallout76,
        "Starfield" => xt_core::types::game_id::GameId::Starfield,
        _ => return Err(format!("Unknown game ID: {}", config.game_id)),
    };
    let rules = state
        .header_rules
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    let total = esp_files.len();
    let mut success = 0usize;
    let mut failed = 0usize;
    let mut total_matched = 0u32;
    let mut errors: Vec<String> = Vec::new();

    for (i, path) in esp_files.iter().enumerate() {
        let fname = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let _ = window.emit(
            "header-batch-progress",
            HeaderBatchProgress {
                current: i + 1,
                total,
                file_path: fname.clone(),
                strings_matched: total_matched,
                stage: "parsing".into(),
                detail_count: None,
                message: String::new(),
            },
        );

        // 解析 ESP
        let esp_data =
            std::fs::read(path).map_err(|e| format!("Failed to read {}: {}", fname, e))?;

        let mut parser = match xt_core::esp::parser::EspParser::with_game(
            std::path::Path::new(&config.data_dir),
            game_id,
        ) {
            Ok(p) => p,
            Err(e) => {
                failed += 1;
                let err = format!("Parser init failed {}: {}", fname, e);
                errors.push(err.clone());
                continue;
            }
        };
        parser.enable_esp_mode();

        if let Err(e) = parser.parse(&mut std::io::Cursor::new(&esp_data)) {
            failed += 1;
            let err = format!("Parse failed {}: {}", fname, e);
            errors.push(err.clone());
            let _ = window.emit(
                "header-batch-progress",
                HeaderBatchProgress {
                    current: i + 1,
                    total,
                    file_path: fname.clone(),
                    strings_matched: total_matched,
                    stage: "error".into(),
                    detail_count: None,
                    message: err,
                },
            );
            continue;
        }

        let _ = window.emit(
            "header-batch-progress",
            HeaderBatchProgress {
                current: i + 1,
                total,
                file_path: fname.clone(),
                strings_matched: total_matched,
                stage: "applying".into(),
                detail_count: Some(parser.strings.len()),
                message: String::new(),
            },
        );

        // 从记录树构建 EDID 映射
        let edid_map: std::collections::HashMap<u32, String> = {
            let mut map = std::collections::HashMap::new();
            fn collect_edids(
                grups: &[xt_core::esp::record_tree::EspGrup],
                map: &mut std::collections::HashMap<u32, String>,
            ) {
                for grup in grups {
                    for rec in &grup.records {
                        if let Some(ref edid) = rec.editor_id {
                            map.insert(rec.form_id, edid.clone());
                        }
                    }
                    collect_edids(&grup.children, map);
                }
            }
            collect_edids(&parser.record_tree, &mut map);
            map
        };

        // 应用规则
        let mut file_matched = 0u32;
        for sk in &parser.strings {
            if sk.params.is_locked() || sk.params.is_translated() {
                continue;
            }
            let record_sig = String::from_utf8_lossy(&sk.record_sig).to_string();
            let field_sig = String::from_utf8_lossy(&sk.field_sig).to_string();
            let form_id = sk.esp_ptr.form_id;
            let edid = edid_map.get(&form_id).cloned().unwrap_or_default();

            if let Some(_new_text) = rules.apply_rules(
                &record_sig,
                &field_sig,
                &edid,
                form_id,
                &[],
                &sk.translation,
                &sk.source,
            ) {
                file_matched += 1;
            }
        }

        total_matched += file_matched;

        let _ = window.emit(
            "header-batch-progress",
            HeaderBatchProgress {
                current: i + 1,
                total,
                file_path: fname.clone(),
                strings_matched: total_matched,
                stage: "complete".into(),
                detail_count: Some(file_matched as usize),
                message: String::new(),
            },
        );

        success += 1;
    }

    let duration_ms = start.elapsed().as_millis() as u64;
    let result = HeaderBatchComplete {
        total_files: total,
        success,
        failed,
        total_strings_matched: total_matched,
        duration_ms,
        is_cancelled: false,
        errors,
    };

    let _ = window.emit("header-batch-complete", result.clone());
    Ok(result)
}

/// 将当前规则保存到 INI 文件。
#[tauri::command]
pub async fn header_rules_save(
    state: tauri::State<'_, Arc<AppState>>,
    path: String,
) -> Result<(), String> {
    let rules = state.header_rules.lock().map_err(|e| e.to_string())?;
    let text = rules.to_ini_text();
    std::fs::write(&path, text).map_err(|e| format!("Failed to save rules: {}", e))
}

/// 删除给定索引处的规则。返回更新后的规则列表。
#[tauri::command]
pub async fn header_rules_delete(
    state: tauri::State<'_, Arc<AppState>>,
    index: usize,
) -> Result<Vec<HeaderRuleDto>, String> {
    let mut rules = state.header_rules.lock().map_err(|e| e.to_string())?;
    if index < rules.rules.len() {
        rules.rules.remove(index);
    }
    Ok(rules
        .rules
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let mut d = HeaderRuleDto::from(r);
            d.index = i;
            d
        })
        .collect())
}

/// 向上或向下移动规则。返回更新后的规则列表。
#[tauri::command]
pub async fn header_rules_move(
    state: tauri::State<'_, Arc<AppState>>,
    index: usize,
    direction: String,
) -> Result<Vec<HeaderRuleDto>, String> {
    let mut rules = state.header_rules.lock().map_err(|e| e.to_string())?;
    if direction == "up" && index > 0 && index < rules.rules.len() {
        rules.rules.swap(index, index - 1);
    } else if direction == "down" && index + 1 < rules.rules.len() {
        rules.rules.swap(index, index + 1);
    }
    Ok(rules
        .rules
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let mut d = HeaderRuleDto::from(r);
            d.index = i;
            d
        })
        .collect())
}

/// 更新给定索引处规则的字段。返回更新后的规则列表。
#[tauri::command]
pub async fn header_rules_update(
    state: tauri::State<'_, Arc<AppState>>,
    index: usize,
    field: String,
    value: String,
) -> Result<Vec<HeaderRuleDto>, String> {
    let mut rules = state.header_rules.lock().map_err(|e| e.to_string())?;
    if let Some(rule) = rules.rules.get_mut(index) {
        match field.as_str() {
            "header" => rule.header = value,
            "r_sig" => rule.r_sig = value,
            "f_sig" => rule.f_sig = value,
            "in_edid" => {
                rule.in_edid = value
                    .split('|')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            }
            "ex_edid" => {
                rule.ex_edid = value
                    .split('|')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            }
            "regex" => rule.regex = if value.is_empty() { None } else { Some(value) },
            "full_replace" => rule.full_replace = value == "true",
            "pre_process" => rule.pre_process = value == "true",
            _ => {}
        }
    }
    Ok(rules
        .rules
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let mut d = HeaderRuleDto::from(r);
            d.index = i;
            d
        })
        .collect())
}

/// 添加一条新的空白规则。返回更新后的规则列表。
#[tauri::command]
pub async fn header_rules_add(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<HeaderRuleDto>, String> {
    let mut rules = state.header_rules.lock().map_err(|e| e.to_string())?;
    rules
        .rules
        .push(xt_core::header_processor::HeaderRule::default());
    Ok(rules
        .rules
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let mut d = HeaderRuleDto::from(r);
            d.index = i;
            d
        })
        .collect())
}

// ── Template Manager Commands ───────────────────────────────────────

use xt_core::header_processor::{PreProcessingOpts, TemplateInfo, TemplateManager};

/// 列出目录中可用的模板。
#[tauri::command]
pub async fn header_templates_list(dir: String) -> Result<Vec<TemplateInfo>, String> {
    TemplateManager::list_templates(&dir)
}

/// 将当前规则保存为命名模板。
#[tauri::command]
pub async fn header_templates_save(
    state: tauri::State<'_, Arc<AppState>>,
    dir: String,
    name: String,
) -> Result<(), String> {
    let rules = state.header_rules.lock().map_err(|e| e.to_string())?;
    TemplateManager::save_template(&dir, &name, &rules)
}

/// 加载命名模板（替换当前规则）。返回更新后的规则列表。
#[tauri::command]
pub async fn header_templates_load(
    state: tauri::State<'_, Arc<AppState>>,
    dir: String,
    name: String,
) -> Result<Vec<HeaderRuleDto>, String> {
    let rule_set = TemplateManager::load_template(&dir, &name)?;
    let dtos: Vec<HeaderRuleDto> = rule_set
        .rules
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let mut d = HeaderRuleDto::from(r);
            d.index = i;
            d
        })
        .collect();
    *state.header_rules.lock().map_err(|e| e.to_string())? = rule_set;
    Ok(dtos)
}

/// Delete a named template.
#[tauri::command]
pub async fn header_templates_delete(dir: String, name: String) -> Result<(), String> {
    TemplateManager::delete_template(&dir, &name)
}

// ── Pre-Processing Options Commands ─────────────────────────────────

#[derive(serde::Serialize)]
pub struct PreProcOptsDto {
    pub options: Vec<(String, String)>,
}

/// 从 INI 文件加载预处理选项。
#[tauri::command]
pub async fn preproc_opts_load(
    state: tauri::State<'_, Arc<AppState>>,
    path: String,
) -> Result<PreProcOptsDto, String> {
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read opts file: {}", e))?;
    let opts = xt_core::header_processor::PreProcessingOpts::from_ini_text(&text);
    let mut sorted: Vec<_> = opts.options.into_iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    *state
        .pre_processing_opts
        .lock()
        .map_err(|e| e.to_string())? = PreProcessingOpts {
        options: sorted.iter().cloned().collect(),
    };
    Ok(PreProcOptsDto { options: sorted })
}

/// 获取当前预处理选项。
#[tauri::command]
pub async fn preproc_opts_list(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<PreProcOptsDto, String> {
    let opts = state
        .pre_processing_opts
        .lock()
        .map_err(|e| e.to_string())?;
    let mut sorted: Vec<_> = opts
        .options
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(PreProcOptsDto { options: sorted })
}

/// 更新预处理选项的键值对。
#[tauri::command]
pub async fn preproc_opts_set(
    state: tauri::State<'_, Arc<AppState>>,
    key: String,
    value: String,
) -> Result<PreProcOptsDto, String> {
    let mut opts = state
        .pre_processing_opts
        .lock()
        .map_err(|e| e.to_string())?;
    opts.set(&key, &value);
    let mut sorted: Vec<_> = opts
        .options
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(PreProcOptsDto { options: sorted })
}

/// 删除预处理选项的键。
#[tauri::command]
pub async fn preproc_opts_delete(
    state: tauri::State<'_, Arc<AppState>>,
    key: String,
) -> Result<PreProcOptsDto, String> {
    let mut opts = state
        .pre_processing_opts
        .lock()
        .map_err(|e| e.to_string())?;
    opts.options.remove(&key);
    let mut sorted: Vec<_> = opts
        .options
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(PreProcOptsDto { options: sorted })
}

/// 保存预处理选项到 INI 文件。
#[tauri::command]
pub async fn preproc_opts_save(
    state: tauri::State<'_, Arc<AppState>>,
    path: String,
) -> Result<(), String> {
    let opts = state
        .pre_processing_opts
        .lock()
        .map_err(|e| e.to_string())?;
    let text = opts.to_ini_text();
    std::fs::write(&path, text).map_err(|e| format!("Failed to save opts: {}", e))
}

/// 检查源文本与译文之间的别名一致性。
///
/// 从源文本和译文中提取 `<Alias=...>` 样式的标签，
/// 返回不匹配的信息以便前端展示。
#[tauri::command]
pub async fn check_aliases(
    state: tauri::State<'_, Arc<AppState>>,
    id: u32,
) -> Result<AliasCheckResult, String> {
    let strings = state.strings.lock().map_err(|e| e.to_string())?;
    let sk = strings
        .iter()
        .find(|s| s.id == id)
        .ok_or_else(|| format!("String with id {} not found", id))?;

    let source_aliases = extract_aliases(&sk.source);
    let trans_aliases = extract_aliases(&sk.translation);

    // 检查：源文本中存在但翻译中缺失的别名
    let missing_in_trans: Vec<String> = source_aliases
        .iter()
        .filter(|a| !trans_aliases.iter().any(|t| t.eq_ignore_ascii_case(a)))
        .cloned()
        .collect();

    // 检查：翻译中存在但源文本中没有的别名
    let extra_in_trans: Vec<String> = trans_aliases
        .iter()
        .filter(|a| !source_aliases.iter().any(|t| t.eq_ignore_ascii_case(a)))
        .cloned()
        .collect();

    let has_mismatch = !missing_in_trans.is_empty() || !extra_in_trans.is_empty();

    Ok(AliasCheckResult {
        source_aliases,
        trans_aliases,
        missing_in_trans,
        extra_in_trans,
        has_mismatch,
    })
}

/// 别名一致性检查的结果
#[derive(serde::Serialize)]
pub struct AliasCheckResult {
    pub source_aliases: Vec<String>,
    pub trans_aliases: Vec<String>,
    pub missing_in_trans: Vec<String>,
    pub extra_in_trans: Vec<String>,
    pub has_mismatch: bool,
}

/// 从文本中提取别名样式标签。
/// 匹配 Delphi 的 rxPatternAliasStrict：`<alias...>`、`<global...>`、`<relat...>`、
/// `<basename...>`、`<token...>`、`<repetitions>`、`</?font...>`、`<mag>`、`<dur>`
fn extract_aliases(text: &str) -> Vec<String> {
    let re = regex::Regex::new(
        r"<alias[^>]*>|<global[^>]*>|<relat[^>]*>|<basename[^>]*>|<token[^>]*>|<repetitions>|</?font[^>]*>|<mag>|<dur>"
    ).unwrap();
    re.find_iter(text).map(|m| m.as_str().to_string()).collect()
}

// ── Config Commands ─────────────────────────────────────────────────

use xt_shared::dto::AppConfigDto;

fn config_to_dto(cfg: &xt_core::config::AppConfig) -> AppConfigDto {
    AppConfigDto {
        openai_api_key: cfg.openai_api_key.clone(),
        deepl_api_key: cfg.deepl_api_key.clone(),
        baidu_app_id: cfg.baidu_app_id.clone(),
        baidu_key: cfg.baidu_key.clone(),
        youdao_app_key: cfg.youdao_app_key.clone(),
        youdao_secret_key: cfg.youdao_secret_key.clone(),
        azure_key: cfg.azure_key.clone(),
        current_provider: cfg.current_provider.clone(),
        theme: cfg.theme.clone(),
        language: cfg.language.clone(),
        last_game: cfg.last_game.clone(),
        game_selection_mode: cfg.game_selection_mode.clone(),
        strings_strategy: cfg.strings_strategy.clone(),
        proxy_server: cfg.proxy_server.clone(),
        proxy_port: cfg.proxy_port,
        proxy_username: cfg.proxy_username.clone(),
        proxy_password: cfg.proxy_password.clone(),
        esp_mode: cfg.esp_mode,
        spellcheck_dictionary: cfg.spellcheck_dictionary.clone(),
        spellcheck_active: cfg.spellcheck_active,
        spellcheck_loaded: cfg.spellcheck_loaded,
        word_exception_list: cfg.word_exception_list.clone(),
    }
}

fn dto_to_config(dto: &AppConfigDto) -> xt_core::config::AppConfig {
    xt_core::config::AppConfig {
        openai_api_key: dto.openai_api_key.clone(),
        deepl_api_key: dto.deepl_api_key.clone(),
        baidu_app_id: dto.baidu_app_id.clone(),
        baidu_key: dto.baidu_key.clone(),
        youdao_app_key: dto.youdao_app_key.clone(),
        youdao_secret_key: dto.youdao_secret_key.clone(),
        azure_key: dto.azure_key.clone(),
        current_provider: dto.current_provider.clone(),
        theme: dto.theme.clone(),
        language: dto.language.clone(),
        last_game: dto.last_game.clone(),
        game_selection_mode: dto.game_selection_mode.clone(),
        strings_strategy: dto.strings_strategy.clone(),
        proxy_server: dto.proxy_server.clone(),
        proxy_port: dto.proxy_port,
        proxy_username: dto.proxy_username.clone(),
        proxy_password: dto.proxy_password.clone(),
        esp_mode: dto.esp_mode,
        spellcheck_dictionary: dto.spellcheck_dictionary.clone(),
        spellcheck_active: dto.spellcheck_active,
        spellcheck_loaded: dto.spellcheck_loaded,
        word_exception_list: dto.word_exception_list.clone(),
    }
}

#[tauri::command]
pub async fn load_config() -> Result<AppConfigDto, String> {
    let dir = config_dir();
    let cfg = xt_core::config::AppConfig::load(&dir)
        .map_err(|e| format!("Failed to load config: {}", e))?;
    Ok(config_to_dto(&cfg))
}

#[tauri::command]
pub async fn save_config(config: AppConfigDto) -> Result<(), String> {
    let dir = config_dir();
    let mut existing = xt_core::config::AppConfig::load(&dir).unwrap_or_default();
    existing.apply(&dto_to_config(&config));
    existing
        .save(&dir)
        .map_err(|e| format!("Failed to save config: {}", e))
}

// ── ESP Write-back Commands ──────────────────────────────────────────

/// 从加载的 ESP 文件中获取解析后的 TES4 文件头信息。
#[tauri::command]
pub async fn get_esp_header(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<xt_shared::dto::EspHeaderInfoDto, String> {
    let esp_file_lock = state.esp_file.lock().map_err(|e| e.to_string())?;
    let esp_file = esp_file_lock.as_ref().ok_or_else(|| {
        "No ESP file loaded. Enable ESP mode and load an ESP file first.".to_string()
    })?;

    let info = esp_file.tes4.parse_fields();

    Ok(xt_shared::dto::EspHeaderInfoDto {
        version: info.version,
        num_records: info.num_records,
        next_object_id: info.next_object_id,
        author: info.author,
        description: info.description,
        masters: info.masters,
        overridden_count: info.overridden_forms.len() as u32,
        is_master: info.is_master,
        is_localized: info.is_localized,
    })
}

/// 直接保存 ESP（去本地化的 ESP 回写）。
///
/// 当处于 ESP 模式时，将翻译写回 ESP 文件的字段缓冲区中，
/// 重建记录、重新压缩并序列化到磁盘。
#[tauri::command]
pub async fn save_esp(
    state: tauri::State<'_, Arc<AppState>>,
    request: xt_shared::dto::SaveEspRequest,
) -> Result<xt_shared::dto::SaveEspResponse, String> {
    let esp_file_lock = state.esp_file.lock().map_err(|e| e.to_string())?;
    let esp_file = esp_file_lock
        .as_ref()
        .ok_or_else(|| "No ESP file loaded or ESP mode not enabled".to_string())?;

    let strings = state.strings.lock().map_err(|e| e.to_string())?;

    // 构建索引：非 VMAD 字段用单值，(form_id, record_sig, field_sig) → &SkyString
    // VMAD 字段用 Vec 容纳多个字符串
    let mut string_index: HashMap<(u32, [u8; 4], [u8; 4]), &SkyString> = HashMap::new();
    let mut vmad_index: HashMap<(u32, [u8; 4]), Vec<&SkyString>> = HashMap::new();
    for sk in strings.iter() {
        if !sk.translation.is_empty() && sk.translation != sk.source {
            if sk.esp_ptr.field_sig == *b"VMAD" && sk.esp_ptr.str_id < 0 {
                vmad_index
                    .entry((sk.esp_ptr.form_id, sk.esp_ptr.record_sig))
                    .or_default()
                    .push(sk);
            } else {
                string_index.insert(
                    (
                        sk.esp_ptr.form_id,
                        sk.esp_ptr.record_sig,
                        sk.esp_ptr.field_sig,
                    ),
                    sk,
                );
            }
        }
    }

    // 创建 ESP 文件的可变副本以进行重建
    let mut esp_file_mut = esp_file.clone();

    // 遍历树中的所有记录，更新可翻译字段
    let mut records_modified = 0u32;

    fn update_records_in_grup(
        grup: &mut xt_core::esp::record_tree::EspGrup,
        string_index: &HashMap<(u32, [u8; 4], [u8; 4]), &SkyString>,
        vmad_index: &HashMap<(u32, [u8; 4]), Vec<&SkyString>>,
        codepage: &xt_core::strings::CodepageConfig,
    ) -> Result<u32, String> {
        let mut modified = 0u32;

        for record in &mut grup.records {
            if record.header.name == *b"TES4" {
                continue;
            }

            for field in &mut record.fields {
                if field.is_size_xxxx {
                    continue;
                }

                // 处理 VMAD 字段：逐个替换每个 VMAD 字符串
                if field.header.name == *b"VMAD" {
                    let vmad_key = (record.form_id, record.header.name);
                    if let Some(vmad_strings) = vmad_index.get(&vmad_key) {
                        for sk in vmad_strings.iter() {
                            let offset = (-sk.esp_ptr.str_id) as usize;
                            if let Ok(new_buf) = xt_core::vmad::write_vmad_string(
                                &field.buffer,
                                offset,
                                &sk.translation,
                            ) {
                                field.buffer = new_buf;
                                field.header.dsize = field.buffer.len() as u16;
                                modified += 1;
                            }
                        }
                    }
                    continue;
                }

                // 处理普通可翻译字段
                let key = (record.form_id, record.header.name, field.header.name);
                if let Some(sk) = string_index.get(&key) {
                    field.update_buffer(&sk.translation, codepage);
                    modified += 1;
                }
            }

            if modified > 0 {
                record
                    .rebuild_data()
                    .map_err(|e| format!("Failed to rebuild record: {}", e))?;
            }
        }

        for child in &mut grup.children {
            modified += update_records_in_grup(child, string_index, vmad_index, codepage)?;
        }

        Ok(modified)
    }

    // 获取游戏的代码页配置
    let codepage_config = {
        let codepage_table = state.codepage_table.lock().map_err(|e| e.to_string());
        match codepage_table {
            Ok(ref table) if table.is_some() => table.as_ref().unwrap().get_or_utf8("default"),
            _ => xt_core::strings::CodepageConfig::default(),
        }
    };

    for grup in &mut esp_file_mut.top_level_grups {
        records_modified +=
            update_records_in_grup(grup, &string_index, &vmad_index, &codepage_config)?;
    }

    // 重建所有记录以重新计算大小
    esp_file_mut
        .rebuild_all()
        .map_err(|e| format!("Failed to rebuild ESP: {}", e))?;

    // 保存到文件
    esp_file_mut
        .save_to_file(&request.path, request.create_backup)
        .map_err(|e| format!("Failed to save ESP file: {}", e))?;

    let bytes_written = std::fs::metadata(&request.path)
        .map(|m| m.len())
        .unwrap_or(0);

    Ok(xt_shared::dto::SaveEspResponse {
        bytes_written,
        records_modified,
    })
}

/// 最终导出生成 ESP：应用 SST -> 重构 -> 序列化 -> 导出 Strings 文件。
#[tauri::command]
pub async fn finalize_esp(
    state: tauri::State<'_, Arc<AppState>>,
    request: xt_shared::dto::FinalizeEspRequest,
) -> Result<xt_shared::dto::FinalizeEspResponse, String> {
    let esp_file_lock = state.esp_file.lock().map_err(|e| e.to_string())?;
    let esp_file = esp_file_lock
        .as_ref()
        .ok_or_else(|| "No ESP file loaded or ESP mode not enabled".to_string())?;

    let strings = state.strings.lock().map_err(|e| e.to_string())?;
    let _sst_old_data = state.sst_old_data.lock().map_err(|e| e.to_string())?;

    // 构建索引：(form_id, record_sig, field_sig) → &SkyString，O(1) 查找
    let mut string_index: HashMap<(u32, [u8; 4], [u8; 4]), &SkyString> = HashMap::new();
    for sk in strings.iter() {
        string_index.insert(
            (
                sk.esp_ptr.form_id,
                sk.esp_ptr.record_sig,
                sk.esp_ptr.field_sig,
            ),
            sk,
        );
    }

    // 创建 ESP 文件的可变副本以进行重建
    let mut esp_file_mut = esp_file.clone();

    let mut records_modified = 0u32;

    fn apply_translations_to_records(
        grup: &mut xt_core::esp::record_tree::EspGrup,
        string_index: &HashMap<(u32, [u8; 4], [u8; 4]), &SkyString>,
        codepage: &xt_core::strings::CodepageConfig,
    ) -> Result<u32, String> {
        let mut modified = 0u32;

        for record in &mut grup.records {
            if record.header.name == *b"TES4" {
                continue;
            }

            for field in &mut record.fields {
                if field.is_size_xxxx {
                    continue;
                }

                let key = (record.form_id, record.header.name, field.header.name);
                if let Some(sk) = string_index.get(&key) {
                    let text = if !sk.translation.is_empty() {
                        sk.translation.clone()
                    } else {
                        sk.source.clone()
                    };

                    if !text.is_empty() && text != field.buffer_to_string(codepage) {
                        field.update_buffer(&text, codepage);
                        modified += 1;
                    }
                }
            }

            if modified > 0 {
                record
                    .rebuild_data()
                    .map_err(|e| format!("Failed to rebuild record: {}", e))?;
            }
        }

        for child in &mut grup.children {
            modified += apply_translations_to_records(child, string_index, codepage)?;
        }

        Ok(modified)
    }

    // 获取语言的代码页配置
    let codepage_config = {
        let codepage_table = state.codepage_table.lock().map_err(|e| e.to_string());
        match codepage_table {
            Ok(ref table) if table.is_some() => {
                table.as_ref().unwrap().get_or_utf8(&request.language)
            }
            _ => xt_core::strings::CodepageConfig::default(),
        }
    };

    for grup in &mut esp_file_mut.top_level_grups {
        records_modified += apply_translations_to_records(grup, &string_index, &codepage_config)?;
    }

    // 重建所有记录
    esp_file_mut
        .rebuild_all()
        .map_err(|e| format!("Failed to rebuild ESP: {}", e))?;

    // 保存 ESP 文件
    esp_file_mut
        .save_to_file(&request.esp_path, request.create_backup)
        .map_err(|e| format!("Failed to save ESP file: {}", e))?;

    // 导出 Strings 文件
    let strings_files = export_strings_files(
        &strings,
        &request.strings_dir,
        &request.base_name,
        &request.language,
        &codepage_config,
    )?;

    Ok(xt_shared::dto::FinalizeEspResponse {
        esp_path: request.esp_path,
        strings_files,
        records_modified,
    })
}

/// 使用标准的二进制格式（与 Bethesda 游戏兼容），从已翻译的字符串中导出 .STRINGS/.DLSTRINGS/.ILSTRINGS 文件。
fn export_strings_files(
    strings: &[xt_core::types::sky_string::SkyString],
    strings_dir: &str,
    base_name: &str,
    language: &str,
    codepage: &xt_core::strings::CodepageConfig,
) -> Result<Vec<String>, String> {
    export_strings_files_inner(strings, strings_dir, base_name, language, codepage, |sk| {
        (!sk.translation.is_empty(), sk.translation.clone())
    })
}

/// 导出非本地化 ESP 的 Strings 文件（使用源文本作为回退）。
fn export_strings_files_for_delocalize(
    strings: &[xt_core::types::sky_string::SkyString],
    strings_dir: &str,
    base_name: &str,
    language: &str,
    codepage: &xt_core::strings::CodepageConfig,
) -> Result<Vec<String>, String> {
    export_strings_files_inner(strings, strings_dir, base_name, language, codepage, |sk| {
        let text = if !sk.translation.is_empty() {
            sk.translation.clone()
        } else {
            sk.source.clone()
        };
        (!text.is_empty(), text)
    })
}

/// 导出二进制格式 Strings 文件的通用实现。
fn export_strings_files_inner(
    strings: &[xt_core::types::sky_string::SkyString],
    strings_dir: &str,
    base_name: &str,
    language: &str,
    codepage: &xt_core::strings::CodepageConfig,
    pick_text: impl Fn(&xt_core::types::sky_string::SkyString) -> (bool, String),
) -> Result<Vec<String>, String> {
    use std::collections::HashMap;
    use std::fs;
    use std::path::Path;

    // 按 list_index 分组字符串
    let mut grouped: HashMap<u8, Vec<(u32, String)>> = HashMap::new();
    for sk in strings {
        let (should_include, text) = pick_text(sk);
        if should_include && sk.esp_ptr.str_id >= 0 {
            grouped
                .entry(sk.list_index)
                .or_default()
                .push((sk.esp_ptr.str_id as u32, text));
        }
    }

    let mut exported_files = Vec::new();
    let strings_path = Path::new(strings_dir);

    fs::create_dir_all(strings_path)
        .map_err(|e| format!("Failed to create strings directory: {}", e))?;

    for (list_index, mut entries) in grouped {
        // 按字符串 ID 排序以确保输出稳定
        entries.sort_by_key(|(id, _)| *id);

        let ext = match list_index {
            0 => "STRINGS",
            1 => "DLSTRINGS",
            2 => "ILSTRINGS",
            _ => "STRINGS",
        };

        let filename = format!("{}_{}.{}", base_name, language, ext);
        let filepath = strings_path.join(&filename);

        let format = xt_core::strings::StringsFile::detect_format(&filepath);
        let sfile = xt_core::strings::StringsFile::from_entries(entries, codepage.clone());
        sfile
            .save_with_format(&filepath, format)
            .map_err(|e| format!("Failed to write strings file {}: {}", filename, e))?;

        exported_files.push(filepath.to_string_lossy().to_string());
    }

    Ok(exported_files)
}

/// 去本地化 ESP：将本地化 ESP 转换为非本地化格式。
#[tauri::command]
pub async fn delocalize_esp(
    state: tauri::State<'_, Arc<AppState>>,
    request: xt_shared::dto::DelocalizeEspRequest,
) -> Result<xt_shared::dto::DelocalizeEspResponse, String> {
    let esp_file_lock = state.esp_file.lock().map_err(|e| e.to_string())?;
    let esp_file = esp_file_lock
        .as_ref()
        .ok_or_else(|| "No ESP file loaded or ESP mode not enabled".to_string())?;

    let strings = state.strings.lock().map_err(|e| e.to_string())?;

    // 构建索引：(form_id, record_sig, field_sig) → &SkyString，O(1) 查找
    let mut string_index: HashMap<(u32, [u8; 4], [u8; 4]), &SkyString> = HashMap::new();
    for sk in strings.iter() {
        string_index.insert(
            (
                sk.esp_ptr.form_id,
                sk.esp_ptr.record_sig,
                sk.esp_ptr.field_sig,
            ),
            sk,
        );
    }

    // 创建 ESP 文件的可变副本
    let mut esp_file_mut = esp_file.clone();

    let mut new_string_count = 0u32;

    fn delocalize_records_in_grup(
        grup: &mut xt_core::esp::record_tree::EspGrup,
        string_index: &HashMap<(u32, [u8; 4], [u8; 4]), &SkyString>,
        codepage: &xt_core::strings::CodepageConfig,
    ) -> Result<u32, String> {
        let mut new_strings = 0u32;

        for record in &mut grup.records {
            if record.header.name == *b"TES4" {
                continue;
            }

            for field in &mut record.fields {
                if field.is_size_xxxx {
                    continue;
                }

                let key = (record.form_id, record.header.name, field.header.name);
                if let Some(sk) = string_index.get(&key) {
                    let text = if !sk.translation.is_empty() {
                        sk.translation.clone()
                    } else {
                        sk.source.clone()
                    };

                    if !text.is_empty() {
                        field.update_buffer(&text, codepage);
                        new_strings += 1;
                    }
                }
            }

            if new_strings > 0 {
                record
                    .rebuild_data()
                    .map_err(|e| format!("Failed to rebuild record: {}", e))?;
            }
        }

        for child in &mut grup.children {
            new_strings += delocalize_records_in_grup(child, string_index, codepage)?;
        }

        Ok(new_strings)
    }

    // 获取语言的代码页配置
    let codepage_config = {
        let codepage_table = state.codepage_table.lock().map_err(|e| e.to_string());
        match codepage_table {
            Ok(ref table) if table.is_some() => {
                table.as_ref().unwrap().get_or_utf8(&request.language)
            }
            _ => xt_core::strings::CodepageConfig::default(),
        }
    };

    for grup in &mut esp_file_mut.top_level_grups {
        new_string_count += delocalize_records_in_grup(grup, &string_index, &codepage_config)?;
    }

    // 重建所有记录
    esp_file_mut
        .rebuild_all()
        .map_err(|e| format!("Failed to rebuild ESP: {}", e))?;

    // 保存 ESP 文件
    esp_file_mut
        .save_to_file(&request.esp_path, request.create_backup)
        .map_err(|e| format!("Failed to save delocalized ESP file: {}", e))?;

    // 导出 Strings 文件
    let strings_files = export_strings_files_for_delocalize(
        &strings,
        &request.strings_dir,
        &request.base_name,
        &request.language,
        &codepage_config,
    )?;

    Ok(xt_shared::dto::DelocalizeEspResponse {
        new_string_count,
        strings_files_paths: strings_files,
    })
}

/// 返回 API 翻译器配置信息（服务商、模型、限制等）。
#[tauri::command]
pub async fn get_api_config(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<xt_shared::dto::ApiConfigResponse, String> {
    use xt_shared::dto::ApiProviderInfo;
    let mut providers = Vec::new();
    for (name, cfg) in &state.api_config.providers {
        providers.push(ApiProviderInfo {
            name: name.clone(),
            label: cfg.label.clone(),
            enabled: cfg.enabled,
            models: cfg.models.clone(),
            default_query: cfg.default_query.clone(),
            char_limit: cfg.char_limit,
            array_limit: cfg.array_limit,
        });
    }
    providers.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(xt_shared::dto::ApiConfigResponse { providers })
}

// ── Finalize Command ────────────────────────────────────────────────

/// Finalize 翻译流程：一次性完成 Strings 文件保存、SST 字典保存、XML 导出。
#[tauri::command]
pub async fn finalize(
    window: tauri::Window,
    state: tauri::State<'_, Arc<AppState>>,
    request: FinalizeRequest,
) -> Result<FinalizeResponse, String> {
    // 提取所有需要的数据（锁的scope要尽量小）
    let (strings_data, file_info_data, old_data) = {
        let strings = state.strings.lock().map_err(|e| e.to_string())?;
        let file_info = state.file_info.lock().map_err(|e| e.to_string())?;
        let old_data = state.sst_old_data.lock().map_err(|e| e.to_string())?;

        let source_lang = file_info
            .as_ref()
            .map(|fi| fi.language.clone())
            .unwrap_or_else(|| "english".to_string());
        let strings_dir = file_info
            .as_ref()
            .and_then(|fi| fi.strings_dir.clone())
            .unwrap_or_default();
        let esp_path = file_info
            .as_ref()
            .map(|fi| fi.esp_path.clone())
            .unwrap_or_default();

        let total_strings = strings.len() as u32;
        let translated_count = strings
            .iter()
            .filter(|sk| sk.params.is_translated())
            .count() as u32;

        let mut translated_map: std::collections::HashMap<(u8, i32), String> =
            std::collections::HashMap::new();
        for sk in strings.iter() {
            if !sk.translation.is_empty() {
                translated_map.insert((sk.list_index, sk.esp_ptr.str_id), sk.translation.clone());
            }
        }

        (
            (
                strings.clone(),
                total_strings,
                translated_count,
                translated_map,
            ),
            (source_lang, strings_dir, esp_path),
            old_data.clone(),
        )
    }; // 锁在这里释放

    let (strings_clone, total_strings, translated_count, translated_map) = strings_data;
    let (source_lang, strings_dir, esp_path) = file_info_data;

    // 加载源文件时获取代码页表以正确编码
    let codepage_table = state.codepage_table.lock().map_err(|e| e.to_string())?;
    let codepage_table_ref: Option<&CodepageTable> = codepage_table.as_ref();

    let output_dir = std::path::Path::new(&request.strings_output_dir);
    let base_name = &request.base_name;
    let target_lang = &request.target_lang;

    emit_xml_progress(&window, "preparing", 0, 3, "Preparing finalize...");

    // 1. 保存 Strings 文件
    let mut strings_count_val = 0u32;
    let mut dlstrings_count_val = 0u32;
    let mut ilstrings_count_val = 0u32;

    for (list_index, ext, count_ref) in [
        (0u8, "STRINGS", &mut strings_count_val),
        (1u8, "DLSTRINGS", &mut dlstrings_count_val),
        (2u8, "ILSTRINGS", &mut ilstrings_count_val),
    ] {
        let source_path = std::path::Path::new(&strings_dir).join(format!(
            "{}_{}.{}",
            base_name,
            source_lang,
            ext.to_lowercase()
        ));

        let mut strings_file = if source_path.exists() {
            if let Some(ref table) = codepage_table_ref {
                xt_core::strings::StringsFile::load_with_codepage_table(&source_path, table)
            } else {
                xt_core::strings::StringsFile::load_with_format(
                    &source_path,
                    xt_core::strings::StringsFile::detect_format(&source_path),
                )
            }
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

        let target_path = output_dir.join(format!(
            "{}_{}.{}",
            base_name,
            target_lang,
            ext.to_lowercase()
        ));

        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create output dir: {}", e))?;
        }

        let format = xt_core::strings::StringsFile::detect_format(&target_path);
        strings_file.format = format;

        strings_file
            .save_with_format(&target_path, format)
            .map_err(|e| format!("Failed to write {}: {}", ext, e))?;

        *count_ref = strings_file.strings.len() as u32;
    }

    let strings_path = output_dir
        .join(format!("{}_{}.strings", base_name, target_lang))
        .to_str()
        .unwrap_or("")
        .to_string();
    let dlstrings_path = output_dir
        .join(format!("{}_{}.dlstrings", base_name, target_lang))
        .to_str()
        .unwrap_or("")
        .to_string();
    let ilstrings_path = output_dir
        .join(format!("{}_{}.ilstrings", base_name, target_lang))
        .to_str()
        .unwrap_or("")
        .to_string();

    emit_xml_progress(&window, "strings_done", 1, 3, "Strings files saved");

    // 2. 保存 SST 字典
    let mut sst_saved_path = String::new();
    if let Some(ref sst_path) = request.sst_path {
        let mut entries = strings_clone.clone();
        append_old_data_entries(&mut entries, &old_data);
        let dict = xt_core::sst::v8::SstDictionary::from_entries(entries);
        dict.save_to_file(sst_path)
            .map_err(|e| format!("Failed to save SST: {}", e))?;
        sst_saved_path = sst_path.clone();
        emit_xml_progress(&window, "sst_done", 2, 3, "SST dictionary saved");
    }

    // 3. 导出 XML
    let mut xml_saved_path = String::new();
    if let Some(ref xml_path) = request.xml_path {
        if !esp_path.is_empty() {
            let addon = std::path::Path::new(&esp_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown")
                .to_string();

            let entries = xt_core::xml::sky_strings_to_xml_entries(&strings_clone);

            let params = xt_core::xml::XmlExportParams {
                addon,
                source_lang,
                dest_lang: request.target_lang.clone(),
                version: 2,
            };

            xt_core::xml::write_xml_file(std::path::Path::new(xml_path), &params, &entries)
                .map_err(|e| format!("Failed to write XML: {}", e))?;
            xml_saved_path = xml_path.clone();

            emit_xml_progress(&window, "xml_done", 3, 3, "XML exported");
        }
    }

    // 全部完成后清除脏标记
    *state.is_dirty.lock().map_err(|e| e.to_string())? = false;

    emit_xml_progress(&window, "done", 3, 3, "Finalize complete");

    Ok(FinalizeResponse {
        strings_path,
        dlstrings_path,
        ilstrings_path,
        sst_path: sst_saved_path,
        xml_path: xml_saved_path,
        translated_count,
        total_count: total_strings,
    })
}

/// 检查是否有未应用的翻译缓存
#[tauri::command]
pub fn check_pending_cache(
    state: tauri::State<'_, Arc<AppState>>,
    esp_hash: String,
) -> Result<CheckPendingCacheResponse, String> {
    let base_dir = cache_dir()
        .parent()
        .ok_or("无法确定缓存父目录")?
        .to_path_buf();
    let cache = TranslationCache::new(base_dir);

    let file_info = state.file_info.lock().map_err(|e| e.to_string())?;
    let esp_name = file_info
        .as_ref()
        .map(|f| {
            std::path::Path::new(&f.esp_path)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());

    let strings = state.strings.lock().map_err(|e| e.to_string())?;
    let current: Vec<(i32, Option<&str>)> = strings
        .iter()
        .filter(|s| s.esp_ptr.str_id >= 0)
        .map(|s| (s.esp_ptr.str_id, Some(s.translation.as_str())))
        .collect();

    let result = cache
        .detect_pending(&esp_hash, &esp_name, &current)
        .map_err(|e| e.to_string())?;

    Ok(CheckPendingCacheResponse {
        recovery: result.map(|r| RecoveryInfo {
            esp_name: r.esp_name,
            pending_count: r.pending_count,
            cache_file_path: r.cache_file_path,
        }),
    })
}

/// 应用翻译缓存恢复
#[tauri::command]
pub fn apply_translation_cache(
    state: tauri::State<'_, Arc<AppState>>,
    esp_hash: String,
) -> Result<ApplyCacheResponse, String> {
    let base_dir = cache_dir()
        .parent()
        .ok_or("无法确定缓存父目录")?
        .to_path_buf();
    let cache = TranslationCache::new(base_dir);

    let translations = cache
        .read_translations(&esp_hash)
        .map_err(|e| e.to_string())?;

    let mut strings = state.strings.lock().map_err(|e| e.to_string())?;
    let mut applied = 0u32;

    for (str_id, translated) in &translations {
        if let Some(s) = strings.iter_mut().find(|s| s.esp_ptr.str_id == *str_id) {
            s.translation = translated.clone();
            applied += 1;
        }
    }

    cache.discard_cache(&esp_hash).map_err(|e| e.to_string())?;

    *state.is_dirty.lock().map_err(|e| e.to_string())? = true;

    Ok(ApplyCacheResponse {
        applied_count: applied,
    })
}

/// 丢弃翻译缓存（不恢复）
#[tauri::command]
pub fn discard_translation_cache(esp_hash: String) -> Result<(), String> {
    let base_dir = cache_dir()
        .parent()
        .ok_or("无法确定缓存父目录")?
        .to_path_buf();
    let cache = TranslationCache::new(base_dir);
    cache.discard_cache(&esp_hash).map_err(|e| e.to_string())
}

/// 启动字符串级批量翻译（操作已加载的字符串）
#[tauri::command]
pub async fn start_string_batch_translate(
    state: tauri::State<'_, Arc<AppState>>,
    window: tauri::Window,
    ids: Vec<u32>,
    concurrency: u8,
) -> Result<String, String> {
    let concurrency = concurrency.clamp(1, 10);

    let provider_type = *state.current_provider.lock().map_err(|e| e.to_string())?;
    let openai_key = state
        .openai_api_key
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    let deepl_key = state
        .deepl_api_key
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    let baidu_app_id = state
        .baidu_app_id
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    let baidu_key = state.baidu_key.lock().map_err(|e| e.to_string())?.clone();
    let youdao_app_key = state
        .youdao_app_key
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    let youdao_secret_key = state
        .youdao_secret_key
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    let azure_key = state.azure_key.lock().map_err(|e| e.to_string())?.clone();
    let api_config = state.api_config.clone();

    match provider_type {
        ProviderType::OpenAI if openai_key.is_none() => {
            return Err("Please configure OpenAI API Key first".to_string());
        }
        ProviderType::DeepL if deepl_key.is_none() => {
            return Err("Please configure DeepL API Key first".to_string());
        }
        ProviderType::Baidu if baidu_app_id.is_none() || baidu_key.is_none() => {
            return Err("Please configure Baidu AppId/Key first".to_string());
        }
        ProviderType::Youdao if youdao_app_key.is_none() || youdao_secret_key.is_none() => {
            return Err("Please configure Youdao AppKey/SecretKey first".to_string());
        }
        ProviderType::Azure if azure_key.is_none() => {
            return Err("Please configure Azure subscription key first".to_string());
        }
        ProviderType::Google => {} // keyless
        _ => {}
    }

    // 读取选中的字符串
    let items: Vec<xt_core::batch_queue::BatchItem> = {
        let strings = state.strings.lock().map_err(|e| e.to_string())?;
        let id_set: std::collections::HashSet<u32> = ids.iter().copied().collect();
        strings
            .iter()
            .filter(|s| id_set.contains(&(s.esp_ptr.str_id as u32)))
            .map(|s| xt_core::batch_queue::BatchItem {
                str_id: s.esp_ptr.str_id as u32,
                source_text: s.source.clone(),
            })
            .collect()
    };

    if items.is_empty() {
        return Err("没有找到可翻译的字符串".to_string());
    }

    let total = items.len() as u32;
    let queue = Arc::new(xt_core::batch_queue::BatchQueue::new(concurrency, total));
    let queue_clone = queue.clone();

    {
        let mut bq = state.batch_queue.lock().map_err(|e| e.to_string())?;
        *bq = Some(queue.clone());
    }

    tokio::spawn(async move {
        let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency as usize));
        let mut handles = Vec::new();

        for item in items {
            if queue_clone.is_cancelled() {
                break;
            }

            let permit = match semaphore.clone().acquire_owned().await {
                Ok(p) => p,
                Err(_) => break,
            };

            let queue = queue_clone.clone();
            let provider_type = provider_type;
            let openai_key = openai_key.clone();
            let deepl_key = deepl_key.clone();
            let baidu_app_id = baidu_app_id.clone();
            let baidu_key = baidu_key.clone();
            let youdao_app_key = youdao_app_key.clone();
            let youdao_secret_key = youdao_secret_key.clone();
            let azure_key = azure_key.clone();
            let api_config = api_config.clone();
            let window = window.clone();

            let handle = tokio::spawn(async move {
                let _permit = permit;

                let result = translate_single_with_retry(
                    &item.source_text,
                    provider_type,
                    &openai_key,
                    &deepl_key,
                    &baidu_app_id,
                    &baidu_key,
                    &youdao_app_key,
                    &youdao_secret_key,
                    &azure_key,
                    &api_config,
                )
                .await;

                let mut batch_result = xt_core::batch_queue::BatchResult {
                    str_id: item.str_id,
                    translated: String::new(),
                    error: None,
                };

                match &result {
                    Ok(text) => {
                        batch_result.translated = text.clone();
                    }
                    Err(e) => {
                        batch_result.error = Some(e.clone());
                    }
                }

                let progress = queue.mark_done();

                let _ = window.emit(
                    "batch-string-progress",
                    serde_json::json!({
                        "str_id": batch_result.str_id,
                        "translated": batch_result.translated,
                        "error": batch_result.error,
                        "completed": progress.completed,
                        "total": progress.total,
                    }),
                );

                batch_result
            });

            handles.push(handle);
        }

        let mut summary = xt_core::batch_queue::BatchSummary {
            total,
            succeeded: 0,
            failed: 0,
            errors: Vec::new(),
        };

        for handle in handles {
            match handle.await {
                Ok(result) => {
                    if result.error.is_none() {
                        summary.succeeded += 1;
                    } else {
                        summary.failed += 1;
                        summary.errors.push(xt_core::batch_queue::BatchErrorEntry {
                            str_id: result.str_id,
                            source: String::new(),
                            error: result.error.unwrap_or_default(),
                        });
                    }
                }
                Err(_) => {
                    summary.failed += 1;
                }
            }
        }

        let _ = window.emit(
            "batch-string-complete",
            serde_json::json!({
                "total": summary.total,
                "succeeded": summary.succeeded,
                "failed": summary.failed,
                "errors": summary.errors,
            }),
        );
    });

    Ok("started".to_string())
}

async fn translate_single_with_retry(
    source: &str,
    provider_type: ProviderType,
    openai_key: &Option<String>,
    deepl_key: &Option<String>,
    baidu_app_id: &Option<String>,
    baidu_key: &Option<String>,
    youdao_app_key: &Option<String>,
    youdao_secret_key: &Option<String>,
    azure_key: &Option<String>,
    _api_config: &ApiTranslatorConfig,
) -> Result<String, String> {
    let mut delay = 1u64;

    for attempt in 0..3 {
        let result = match provider_type {
            ProviderType::OpenAI => {
                let key = openai_key.as_ref().ok_or("No OpenAI API key")?;
                let provider = OpenAIProvider::new(key.clone());
                provider
                    .translate(source, "", "", None)
                    .await
                    .map_err(|e| e.to_string())
            }
            ProviderType::DeepL => {
                let key = deepl_key.as_ref().ok_or("No DeepL API key")?;
                let provider = DeepLProvider::new(key.clone());
                provider
                    .translate(source, "", "", None)
                    .await
                    .map_err(|e| e.to_string())
            }
            ProviderType::Baidu => {
                let app_id = baidu_app_id.as_ref().ok_or("No Baidu AppId")?;
                let key = baidu_key.as_ref().ok_or("No Baidu Key")?;
                let provider = BaiduProvider::new(app_id.clone(), key.clone());
                provider
                    .translate(source, "", "", None)
                    .await
                    .map_err(|e| e.to_string())
            }
            ProviderType::Youdao => {
                let app_key = youdao_app_key.as_ref().ok_or("No Youdao AppKey")?;
                let secret_key = youdao_secret_key.as_ref().ok_or("No Youdao SecretKey")?;
                let provider = YoudaoProvider::new(app_key.clone(), secret_key.clone());
                provider
                    .translate(source, "", "", None)
                    .await
                    .map_err(|e| e.to_string())
            }
            ProviderType::Azure => {
                let key = azure_key.as_ref().ok_or("No Azure subscription key")?;
                let provider = AzureProvider::new(key.clone());
                provider
                    .translate(source, "", "", None)
                    .await
                    .map_err(|e| e.to_string())
            }
            ProviderType::Google => {
                let provider = GoogleProvider::new();
                provider
                    .translate(source, "", "", None)
                    .await
                    .map_err(|e| e.to_string())
            }
        };

        match result {
            Ok(text) if !text.is_empty() => return Ok(text),
            Ok(_) => return Ok(String::new()),
            Err(e) => {
                let is_retriable = e.contains("timeout")
                    || e.contains("429")
                    || e.contains("503")
                    || e.contains("502");

                if !is_retriable || attempt == 2 {
                    return Err(e);
                }

                tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                delay *= 2;
            }
        }
    }

    Err("max retries exceeded".to_string())
}

/// 取消字符串级批量翻译
#[tauri::command]
pub fn cancel_string_batch_translate(state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    let mut bq = state.batch_queue.lock().map_err(|e| e.to_string())?;
    if let Some(ref queue) = *bq {
        queue.cancel();
    }
    *bq = None;
    Ok(())
}

/// 写入文本文件（用于导出报告等）
#[tauri::command]
pub fn write_text_file(path: String, content: String) -> Result<(), String> {
    use std::io::Write;
    if let Some(parent) = std::path::Path::new(&path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create parent directory: {}", e))?;
        }
    }
    let mut file =
        std::fs::File::create(&path).map_err(|e| format!("Failed to create file: {}", e))?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("Failed to write file: {}", e))?;
    Ok(())
}

/// Read a UTF-8 text file for editor-style tools such as Command Processor.
#[tauri::command]
pub fn read_text_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| format!("Failed to read text file: {e}"))
}
