use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::Emitter;
use xt_core::cache::EsmCache;
use xt_core::esp::parser::{EspParser, StringsFiles};
use xt_core::matching::{apply_dictionary_entries_with_policy, ApplyPolicy, DictionaryApplyEntry};
use xt_core::pex::types::PexTranslatableString;
use xt_core::sst::v8::SstDictionary;
use xt_core::strings::CodepageTable;
use xt_core::translation_api::{DeepLProvider, OpenAIProvider, ProviderType, TranslationProvider};
use xt_core::types::game_id::GameId;
use xt_core::types::params::SkyStringParams;
use xt_core::types::sky_string::SkyString;
use xt_core::xml::{
    import_xml_to_sky_strings, parse_xml_file, sky_strings_to_xml_entries, write_xml_file,
    XmlExportParams,
};
use xt_shared::dto::{
    AutoBackupRequest, AutoBackupResponse, BatchConfig, BatchEntry, BatchStatus, BsaFileEntryDto,
    BsaFileListDto, DialogInfoDto, DialogTreeDto, EspComparePairDto, EspCompareResultDto,
    EspLoadProgress, FuzMapping, FuzScanResponse, HeuristicMatchDTO, HeuristicSearchRequest,
    LoadEspResponse, LoadSstResponse, McmEntryDto, McmFileDto, McmSaveRequest, NpcDialogDto,
    PexScriptDto, PexTranslatableDto, QueryRequest, QueryResponse, SaveStringsRequest,
    SaveStringsResponse, SkyStringDTO, TranslateRequest, XmlExportRequest, XmlImportResponse,
    XmlProgress,
};

use crate::batch::BatchExecutor;

/// 已加载的 ESP 文件信息
pub struct EspFileInfo {
    pub esp_path: String,
    pub strings_dir: Option<String>,
    pub language: String,
}

/// 应用状态：持有所有加载的文件数据
pub struct AppState {
    pub strings: Mutex<Vec<SkyString>>,
    pub sst_old_data: Mutex<Vec<SkyString>>,
    pub file_info: Mutex<Option<EspFileInfo>>,
    /// OpenAI 兼容 API Key（内存存储，不持久化）
    pub openai_api_key: Mutex<Option<String>>,
    /// DeepL API Key（内存存储，不持久化）
    pub deepl_api_key: Mutex<Option<String>>,
    /// 当前选中的翻译提供方
    pub current_provider: Mutex<ProviderType>,
    /// 是否有未保存的翻译修改
    pub is_dirty: Mutex<bool>,
}

impl AppState {
    pub fn new() -> Self {
        // 尝试从环境变量读取 API Key
        let openai_env_key = std::env::var("XT_TRANSLATE_API_KEY").ok();
        let deepl_env_key = std::env::var("XT_DEEPL_API_KEY").ok();

        // 默认为 OpenAI，或根据有哪个 key 自动选择
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
            current_provider: Mutex::new(default_provider),
            is_dirty: Mutex::new(false),
        }
    }
}

/// 将 SkyString 状态转为前端字符串
///
/// 约定：
/// - translated：已翻译
/// - incomplete：未完成翻译
/// - locked：不可编辑/锁定
///
/// 兜底策略：未知状态统一映射为 `locked`，避免前端出现未定义分支。
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

fn config_dir() -> std::path::PathBuf {
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

fn status_string(sk: &SkyString) -> String {
    if sk.params.is_translated() {
        "translated"
    } else if sk.params.is_incomplete() {
        "incomplete"
    } else if sk.params.is_locked() {
        "locked"
    } else {
        "locked"
    }
    .to_string()
}

/// 将 SkyString 转为 DTO
///
/// 说明：
/// - `form_id` 以十六进制字符串返回，便于前端直接展示。
/// - `list_index` 来自 ESP 解析或 SST 加载，标识 STRINGS/DLSTRINGS/ILSTRINGS 归属。
fn sky_string_to_dto(sk: &SkyString) -> SkyStringDTO {
    SkyStringDTO {
        id: sk.id,
        source: sk.source.clone(),
        translation: sk.translation.clone(),
        record_sig: String::from_utf8_lossy(&sk.esp_ptr.record_sig).to_string(),
        field_sig: String::from_utf8_lossy(&sk.esp_ptr.field_sig).to_string(),
        form_id: format!("0x{:08X}", sk.esp_ptr.form_id),
        status: status_string(sk),
        list_index: sk.list_index,
        str_id: sk.esp_ptr.str_id,
    }
}

fn append_old_data_entries(entries: &mut Vec<SkyString>, old_data: &[SkyString]) {
    for old in old_data {
        let mut sk = old.clone();
        sk.params.set(SkyStringParams::OLD_DATA, true);
        entries.push(sk);
    }
}

/// 加载 ESP/ESM 文件并构建内存中的字符串列表。
///
/// 行为要点：
/// - 解析会覆盖当前 `AppState.strings`（相当于重新打开文件）。
/// - 若提供 `strings_dir`，会尝试加载对应语言的 STRINGS 文件。
/// - 返回值中的统计信息用于前端侧边栏和加载反馈。
/// - 先检查本地缓存（基于 ESP 文件 SHA-256 哈希），命中则直接返回。
#[tauri::command]
pub async fn load_esp(
    window: tauri::Window,
    state: tauri::State<'_, Arc<AppState>>,
    esp_path: String,
    strings_dir: Option<String>,
    language: Option<String>,
    game: Option<String>,
) -> Result<LoadEspResponse, String> {
    let esp_path_clone = esp_path.clone();
    let strings_dir_clone = strings_dir.clone();
    let language_clone = language.clone();
    let game_clone = game.clone();

    let c_dir = cache_dir();

    // ESP 解析是 CPU 密集型任务，放到阻塞线程池里执行，避免卡住异步运行时。
    let result = tokio::task::spawn_blocking(
        move || -> Result<(Vec<SkyString>, LoadEspResponse), String> {
            let start = std::time::Instant::now();

            // ── 缓存检查阶段 ──
            let cache = EsmCache::new(c_dir, 50);
            let esp_path_ref = std::path::Path::new(&esp_path_clone);

            if let Some(cached) = cache.lookup(esp_path_ref) {
                let _ = window.emit(
                    "esp-load-progress",
                    EspLoadProgress {
                        stage: "cached".to_string(),
                        current: 100,
                        total: 100,
                        percentage: 100,
                        message: format!("Loaded from cache ({} strings)", cached.strings.len()),
                    },
                );

                let total = cached.strings.len() as u32;
                let record_counts = EsmCache::compute_record_counts(&cached.strings);

                return Ok((
                    cached.strings,
                    LoadEspResponse {
                        total,
                        compressed_records: cached.compressed_records,
                        strings_loaded: cached.strings_loaded,
                        parse_time_ms: 0,
                        record_counts,
                        cached: true,
                    },
                ));
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

            // 兼容前端传入的游戏别名；无法识别时默认回退到天际特别版。
            let game_id = game_clone
                .as_deref()
                .and_then(|g| match g.to_lowercase().as_str() {
                    "skyrim" => Some(GameId::Skyrim),
                    "skyrimse" | "skyrim se" => Some(GameId::SkyrimSE),
                    "fallout4" | "fo4" => Some(GameId::Fallout4),
                    "falloutnv" | "fonv" => Some(GameId::FalloutNV),
                    "fallout76" | "fo76" => Some(GameId::Fallout76),
                    "starfield" | "sf" => Some(GameId::Starfield),
                    _ => None,
                })
                .unwrap_or(GameId::SkyrimSE);

            // 优先加载对应游戏的 record_defs；失败时回退到内置默认定义。
            let data_dir = std::path::Path::new("Data");
            let mut parser =
                EspParser::with_game(data_dir, game_id).unwrap_or_else(|_| EspParser::new());

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

            let mut strings_loaded = 0u8;

            if let Some(ref dir) = strings_dir_clone {
                let dir_path = std::path::Path::new(dir);
                if let Some(ref table) = codepage_table {
                    parser.strings_files =
                        StringsFiles::load_from_dir_with_language(dir_path, base_name, lang, table);
                } else {
                    parser.load_strings_files(dir_path, base_name);
                }
                strings_loaded = parser.strings_files.loaded_count() as u8;
            }

            if strings_loaded == 0 {
                let esp_dir = std::path::Path::new(&esp_path_clone)
                    .parent()
                    .unwrap_or_else(|| std::path::Path::new("."));
                if let Some(ref table) = codepage_table {
                    parser.strings_files =
                        StringsFiles::load_from_dir_with_language(esp_dir, base_name, lang, table);
                } else {
                    parser.load_strings_files(esp_dir, base_name);
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

            // 存储解析结果到缓存（静默失败，不影响主流程）
            let cache_payload = xt_core::cache::CachePayload {
                version: 1,
                strings: parser.strings.clone(),
                compressed_records,
                strings_loaded,
            };
            let _ = cache.store(esp_path_ref, &cache_payload);

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
                },
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
        esp_path,
        strings_dir,
        language: language.unwrap_or_else(|| "english".to_string()),
    });

    *state.is_dirty.lock().map_err(|e| e.to_string())? = false;

    Ok(result.1)
}

/// 加载 SST 字典并合并到当前内存字符串。
///
/// 使用共享字典匹配引擎，按 exact / EDID / normalized / vocab 顺序应用。
/// 该命令仅更新匹配成功的条目，不会新增行。
#[tauri::command]
pub async fn load_sst(
    state: tauri::State<'_, Arc<AppState>>,
    sst_path: String,
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
    let result =
        apply_dictionary_entries_with_policy(&mut strings, &apply_entries, ApplyPolicy::sst_load());

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

/// 按内部 `id` 更新单条翻译文本。
///
/// 注意：这里使用内部行 ID，而不是 `str_id`（两者语义不同）。
#[tauri::command]
pub async fn update_translation(
    state: tauri::State<'_, Arc<AppState>>,
    id: u32,
    translation: String,
) -> Result<(), String> {
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

    Ok(())
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
#[tauri::command]
pub async fn heuristic_search(
    state: tauri::State<'_, Arc<AppState>>,
    request: HeuristicSearchRequest,
) -> Result<Vec<HeuristicMatchDTO>, String> {
    let data = state.strings.lock().map_err(|e| e.to_string())?;

    // 候选集仅来自“已翻译”条目；未翻译条目没有可用目标文本。
    let candidates: Vec<(String, String)> = data
        .iter()
        .filter(|sk| sk.params.is_translated() && !sk.source.is_empty())
        .map(|sk| (sk.source.clone(), sk.translation.clone()))
        .collect();

    if candidates.is_empty() {
        return Ok(Vec::new());
    }

    let min_sim = request.min_similarity.unwrap_or(0.5);
    let max_res = request.max_results.unwrap_or(5);

    let matches = xt_core::heuristic::find_similar_translations(
        &request.source,
        &candidates,
        min_sim,
        max_res,
    );

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
    };

    // 保持默认语言兜底，避免前端漏传参数导致请求失败。
    let source_lang = request.source_lang.unwrap_or_else(|| "EN".to_string());
    let target_lang = request.target_lang.unwrap_or_else(|| "ZH".to_string());
    let text = request.text;

    // 异步执行翻译
    let result = match provider_type {
        ProviderType::OpenAI => {
            let provider = OpenAIProvider::from_key(api_key);
            provider
                .translate(&text, &source_lang, &target_lang)
                .await
                .map_err(|e| e.to_string())?
        }
        ProviderType::DeepL => {
            let provider = DeepLProvider::new(api_key);
            provider
                .translate(&text, &source_lang, &target_lang)
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
#[tauri::command]
pub async fn get_translation_providers(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(String, Vec<String>, bool, bool), String> {
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

    Ok((
        current.to_string(),
        ProviderType::all()
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
        openai_set,
        deepl_set,
    ))
}

/// 将当前已翻译内容导出为 Delphi 兼容 XML。
///
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
        if !sk.translation.is_empty() {
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
            xt_core::strings::StringsFile::load_with_format(
                &source_path,
                xt_core::strings::StringsFile::detect_format(&source_path),
            )
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
    // Generate timestamp from UNIX epoch seconds
    let epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let backup_name = format!("{}_{}.sst", stem, epoch);
    let backup_path = backup_dir.join(&backup_name);

    // Build SST from current strings
    let strings = state.strings.lock().map_err(|e| e.to_string())?;
    let old_data = state.sst_old_data.lock().map_err(|e| e.to_string())?;
    let mut entries = strings.clone();
    append_old_data_entries(&mut entries, &old_data);
    let sst = xt_core::sst::v8::SstDictionary::from_entries(entries);
    sst.save_to_file(backup_path.to_str().ok_or("Invalid backup path")?)
        .map_err(|e| format!("Failed to save backup: {}", e))?;

    // Rotate: keep max_backups newest files
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

    // Sort by modified time descending (newest first)
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

    // Delete excess old backups
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

    // Determine output filename from the file path
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
                        eprintln!("Failed to write {}: {}", entry.path, e);
                    } else {
                        extracted.push(output_path.to_str().unwrap_or("").to_string());
                    }
                }
                Err(e) => {
                    eprintln!("Failed to extract {}: {}", entry.path, e);
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
                    eprintln!("Failed to extract {}: {}", entry.path, e);
                }
            }
        }
    }

    Ok(extracted)
}

// ── PEX Commands ────────────────────────────────────────────────────

/// Parse a PEX file and extract translatable strings
#[tauri::command]
pub async fn parse_pex_strings(pex_path: String) -> Result<PexScriptDto, String> {
    let mut file =
        std::fs::File::open(&pex_path).map_err(|e| format!("Failed to open PEX: {}", e))?;

    let script = xt_core::pex::parser::parse_pex(&mut file)
        .map_err(|e| format!("Failed to parse PEX: {}", e))?;

    let script_name = std::path::Path::new(&pex_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let translatable: Vec<PexTranslatableDto> = script
        .translatable
        .iter()
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

/// Compile a PEX file with updated translations
///
/// Takes the original PEX script and a list of translated strings,
/// writes a new PEX file with the updated string table.
#[tauri::command]
pub async fn compile_pex(
    pex_path: String,
    output_path: String,
    translations: Vec<PexTranslatableDto>,
) -> Result<String, String> {
    use std::fs::File;
    use xt_core::pex::compile::compile_pex;

    // Parse original PEX
    let mut file = File::open(&pex_path).map_err(|e| format!("Failed to open PEX: {}", e))?;
    let script = xt_core::pex::parser::parse_pex(&mut file)
        .map_err(|e| format!("Failed to parse PEX: {}", e))?;

    // Convert DTOs to PexTranslatableString
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

    // Compile with actual translations
    let result = compile_pex(&script, &pex_translations, &output_path)
        .map_err(|e| format!("Failed to compile PEX: {}", e))?;

    Ok(result.path)
}

// ── ESP Compare Commands ───────────────────────────────────────────

use xt_core::esp::compare::{self, EspComparison};

/// Convert internal comparison result to DTO
fn comparison_to_dto(comp: EspComparison) -> EspCompareResultDto {
    let sig_to_str = |sig: &[u8; 4]| String::from_utf8_lossy(sig).to_string();

    let old_by_id: HashMap<u32, &SkyString> = comp.old_strings.iter().map(|s| (s.id, s)).collect();
    let new_by_id: HashMap<u32, &SkyString> = comp.new_strings.iter().map(|s| (s.id, s)).collect();

    let to_pair = |new_id: u32, old_id: u32| -> EspComparePairDto {
        let new_s = new_by_id.get(&new_id).copied();
        let old_s = old_by_id.get(&old_id).copied();
        let record_sig = new_s
            .or(old_s)
            .map(|s| sig_to_str(&s.record_sig))
            .unwrap_or_default();
        let field_sig = new_s
            .or(old_s)
            .map(|s| sig_to_str(&s.esp_ptr.field_sig))
            .unwrap_or_default();

        EspComparePairDto {
            new_id,
            old_id,
            source: new_s.map(|s| s.source.clone()).unwrap_or_default(),
            record_sig,
            field_sig,
            old_source: old_s.map(|s| s.source.clone()).unwrap_or_default(),
            new_source: new_s.map(|s| s.source.clone()).unwrap_or_default(),
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

/// Compare two ESP/ESM files and return string pair mappings
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

/// Convert internal McmFile to DTO
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

/// Load and parse an MCM translation file
#[tauri::command]
pub async fn load_mcm_file(mcm_path: String) -> Result<McmFileDto, String> {
    let file = mcm::parse_mcm_file(&mcm_path)
        .map_err(|e| format!("Failed to parse MCM file: {}", e))?;
    Ok(mcm_file_to_dto(&file))
}

/// Save an MCM file with updated translations
#[tauri::command]
pub async fn save_mcm_file(request: McmSaveRequest) -> Result<(), String> {
    // We need the original McmFile to preserve encoding and normalized_lines.
    // Load it, apply translations from request, then save.
    let mut file = mcm::parse_mcm_file(&request.path)
        .map_err(|e| format!("Failed to open MCM file for save: {}", e))?;

    for dto_entry in &request.entries {
        if let Some(entry) = file.entries.iter_mut().find(|e| e.line_index as u32 == dto_entry.line_index) {
            entry.translation = dto_entry.translation.clone();
        }
    }

    mcm::save_mcm_file(&request.path, &file)
        .map_err(|e| format!("Failed to save MCM file: {}", e))?;
    Ok(())
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
            let dur = xt_core::fuz::FuzFile::parse(
                &mut std::fs::File::open(fuz_path).map_err(|e| format!("Failed: {}", e))?,
            )
            .map(|f| f.duration_secs)
            .unwrap_or(0.0);

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

// ── Dialog Tree Commands ─────────────────────────────────────────────

#[tauri::command]
pub async fn build_dialog_tree(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<DialogTreeDto, String> {
    let strings = state.strings.lock().map_err(|e| e.to_string())?;

    // Group INFO strings by their parent DIAL FormID
    let mut npc_groups: std::collections::HashMap<String, Vec<DialogInfoDto>> =
        std::collections::HashMap::new();

    for s in strings.iter() {
        let record_sig = String::from_utf8_lossy(&s.record_sig);

        // Focus on INFO records (dialog responses) and NPC_ records (for name association)
        if record_sig == "INFO" && !s.source.is_empty() {
            let parent_form_id = s.parent_form_id;

            // Build dialog entry
            let entry = DialogInfoDto {
                id: s.id,
                form_id: parent_form_id,
                source: s.source.clone(),
                translation: s.translation.clone(),
                dialog_text: s.source.clone(),
            };

            // Group by parent DIAL form_id as string key
            let key = format!("DIAL_{:08X}", parent_form_id);
            npc_groups.entry(key).or_default().push(entry);
        }
    }

    // Also associate NPC_ record strings (names) with their dialogues
    let mut npc_names: std::collections::HashMap<u32, String> = std::collections::HashMap::new();
    for s in strings.iter() {
        let sig = String::from_utf8_lossy(&s.record_sig);
        if sig == "NPC_" {
            npc_names.insert(s.esp_ptr.form_id, s.source.clone());
        }
    }

    // Build response
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

// ── Config Commands ─────────────────────────────────────────────────

use xt_shared::dto::AppConfigDto;

fn config_to_dto(cfg: &xt_core::config::AppConfig) -> AppConfigDto {
    AppConfigDto {
        openai_api_key: cfg.openai_api_key.clone(),
        deepl_api_key: cfg.deepl_api_key.clone(),
        current_provider: cfg.current_provider.clone(),
        theme: cfg.theme.clone(),
        language: cfg.language.clone(),
    }
}

fn dto_to_config(dto: &AppConfigDto) -> xt_core::config::AppConfig {
    xt_core::config::AppConfig {
        openai_api_key: dto.openai_api_key.clone(),
        deepl_api_key: dto.deepl_api_key.clone(),
        current_provider: dto.current_provider.clone(),
        theme: dto.theme.clone(),
        language: dto.language.clone(),
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
    existing.save(&dir)
        .map_err(|e| format!("Failed to save config: {}", e))
}
