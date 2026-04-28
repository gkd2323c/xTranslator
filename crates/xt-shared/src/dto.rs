use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 前后端共享 DTO（Tauri IPC 载荷）。
///
/// 设计约束：
/// - 字段尽量稳定，避免频繁破坏前后端兼容。
/// - 新增字段优先使用 `#[serde(default)]`，降低旧版本调用方升级成本。

/// 视口分页查询请求
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryRequest {
    /// 文件 ID（阶段 0 固定为 "test"）
    pub file_id: String,
    /// 视口起始偏移
    pub offset: u32,
    /// 视口大小（通常 50-100）
    pub limit: u32,
    /// 搜索过滤词（在 source/translation 中搜索）
    #[serde(default)]
    pub filter: Option<String>,
    /// 排序字段
    #[serde(default)]
    pub sort_field: Option<String>,
    /// 排序方向："asc" 或 "desc"
    #[serde(default)]
    pub sort_dir: Option<String>,
    /// 状态筛选: "translated" / "incomplete" / "locked" / None(全部)
    #[serde(default)]
    pub status_filter: Option<String>,
}

/// 视口分页查询响应
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryResponse {
    /// 总记录数
    pub total: u32,
    /// 筛选后总数
    pub filtered: u32,
    /// 当前视口数据
    pub items: Vec<SkyStringDTO>,
    /// 当前偏移
    pub offset: u32,
    /// 响应耗时（毫秒），用于性能观测与回归对比
    pub elapsed_ms: u64,
}

/// 前端展示的 SkyString 简化 DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkyStringDTO {
    /// 内部稳定 ID（用于更新定位），不是 ESP 的 str_id
    pub id: u32,
    pub source: String,
    pub translation: String,
    pub record_sig: String,
    pub field_sig: String,
    pub form_id: String,
    pub status: String,
    /// Strings 文件类型索引: 0=.STRINGS, 1=.DLSTRINGS, 2=.ILSTRINGS
    #[serde(default)]
    pub list_index: u8,
    /// Strings 文件中的字符串 ID（用于 SST/XML 精确匹配）
    #[serde(default)]
    pub str_id: i32,
}

/// 加载 ESP 文件响应
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoadEspResponse {
    /// 解析出的总字符串数
    pub total: u32,
    /// 压缩记录数（当前可能为占位统计）
    pub compressed_records: u32,
    /// 成功加载的 Strings 文件数 (0-3)
    pub strings_loaded: u8,
    /// 解析耗时（毫秒）；缓存命中时为 0
    pub parse_time_ms: u64,
    /// 各记录类型数量统计
    pub record_counts: HashMap<String, usize>,
    /// 是否从缓存加载（而非完整解析）
    #[serde(default)]
    pub cached: bool,
}

/// ESP 文件加载进度事件
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EspLoadProgress {
    /// 加载阶段："reading_defs", "loading_strings", "parsing", "finalizing"
    pub stage: String,
    /// 当前进度值
    pub current: u64,
    /// 总进度值
    pub total: u64,
    /// 百分比 (0-100)
    pub percentage: u8,
    /// 用户可读的消息
    pub message: String,
}

/// 加载 SST 字典响应
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoadSstResponse {
    /// 匹配成功的条目数
    pub matched: u32,
    /// 未匹配的 SST 条目数
    pub unmatched: u32,
    /// 被更新的 SkyString 内部 ID 列表（用于前端增量刷新）
    #[serde(default)]
    pub updated_ids: Vec<u32>,
    /// Tier 1 精确三元组匹配数
    #[serde(default)]
    pub tier_exact: u32,
    /// Tier 2 EDID 哈希匹配数
    #[serde(default)]
    pub tier_edid: u32,
    /// Tier 3 规范化文本匹配数
    #[serde(default)]
    pub tier_normalized: u32,
    /// Tier 4 词汇重叠匹配数
    #[serde(default)]
    pub tier_vocab: u32,
    /// 歧义但未自动应用的条目数
    #[serde(default)]
    pub ambiguous: u32,
    /// 因 pending 状态跳过文本应用的条目数
    #[serde(default)]
    pub pending_skipped: u32,
    /// 保留为 oldData 的 SST 条目数
    #[serde(default)]
    pub old_data_preserved: u32,
    /// 因 index/indexMax 可疑而标记 warning 的条目数
    #[serde(default)]
    pub warning: u32,
    /// 因 index/indexMax 不一致而标记 bigWarning 的条目数
    #[serde(default)]
    pub big_warning: u32,
}

/// 启发式搜索请求
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeuristicSearchRequest {
    /// 待搜索的源字符串
    pub source: String,
    /// 最小相似度阈值（0.0 ~ 1.0，默认 0.5）
    #[serde(default)]
    pub min_similarity: Option<f32>,
    /// 最大返回结果数（默认 5）
    #[serde(default)]
    pub max_results: Option<usize>,
}

/// 启发式匹配结果 DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeuristicMatchDTO {
    /// 候选源字符串
    pub source: String,
    /// 候选翻译
    pub translation: String,
    /// 归一化相似度 0.0~1.0
    pub similarity: f32,
    /// 编辑距离
    pub levenshtein: usize,
    /// 最长公共子串长度
    pub lcs_len: usize,
}

/// 翻译请求
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TranslateRequest {
    /// 待翻译文本
    pub text: String,
    /// 源语言（默认 "english"，由后端兜底）
    #[serde(default)]
    pub source_lang: Option<String>,
    /// 目标语言（默认 "chinese"，由后端兜底）
    #[serde(default)]
    pub target_lang: Option<String>,
    /// 翻译提供方（"openai" 或 "deepl"，默认 "openai"）
    #[serde(default)]
    pub provider: Option<String>,
}

/// XML 导出请求
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct XmlExportRequest {
    /// 导出文件路径
    pub path: String,
    /// 目标语言（如 "chinese"）
    pub dest_lang: String,
}

/// XML 导入响应
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct XmlImportResponse {
    /// 匹配成功的总条目数
    pub matched: u32,
    /// 未匹配的 XML 条目数
    pub unmatched: u32,
    /// XML 中的总条目数
    pub total: u32,
    /// 被更新的 SkyString 内部 ID 列表（用于前端增量刷新）
    #[serde(default)]
    pub updated_ids: Vec<u32>,
    /// Tier 1 精确三元组匹配数
    #[serde(default)]
    pub tier_exact: u32,
    /// Tier 2 EDID 哈希匹配数
    #[serde(default)]
    pub tier_edid: u32,
    /// Tier 3 词汇重叠匹配数
    #[serde(default)]
    pub tier_vocab: u32,
    /// Tier 4 规范化文本匹配数
    #[serde(default)]
    pub tier_normalized: u32,
    /// 歧义但未自动应用的条目数
    #[serde(default)]
    pub ambiguous: u32,
    /// 因 pending 状态跳过文本应用的条目数
    #[serde(default)]
    pub pending_skipped: u32,
    /// 保留为 oldData 的 SST 条目数（XML 导入通常为 0）
    #[serde(default)]
    pub old_data_preserved: u32,
    /// 因 index/indexMax 可疑而标记 warning 的条目数
    #[serde(default)]
    pub warning: u32,
    /// 因 index/indexMax 不一致而标记 bigWarning 的条目数
    #[serde(default)]
    pub big_warning: u32,
}

/// 保存 Strings 文件请求
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SaveStringsRequest {
    /// 输出目录路径
    pub output_dir: String,
    /// 目标语言（如 "chinese"）
    pub target_lang: String,
    /// ESP 基础文件名（如 "Skyrim"）
    pub base_name: String,
}

/// XML 导入/导出进度事件
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct XmlProgress {
    /// 操作阶段："parsing", "merging", "writing"
    pub stage: String,
    /// 当前进度值
    pub current: u64,
    /// 总进度值
    pub total: u64,
    /// 百分比 (0-100)
    pub percentage: u8,
    /// 用户可读的消息
    pub message: String,
}

// ── Batch Processor DTOs ──────────────────────────────────────────

/// 批处理中单文件的条目信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchEntry {
    /// ESP 文件路径
    pub esp_path: String,
    /// Strings 目录，默认 ESP 所在目录
    #[serde(default)]
    pub strings_dir: Option<String>,
    /// 源语言（如 "english"），默认 "english"
    #[serde(default)]
    pub language: Option<String>,
    /// 游戏类型（如 "SkyrimSE"），自动探测
    #[serde(default)]
    pub game: Option<String>,
    /// 可选 SST 字典路径，用于预合并现有翻译
    #[serde(default)]
    pub sst_path: Option<String>,
}

/// 批处理配置
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchConfig {
    /// 要处理的文件列表
    pub entries: Vec<BatchEntry>,
    /// 翻译提供方 "openai" | "deepl"
    #[serde(default)]
    pub provider: Option<String>,
    /// 目标语言（如 "chinese"）
    pub target_lang: Option<String>,
    /// 是否跳过已有翻译的字符串
    #[serde(default)]
    pub skip_translated: Option<bool>,
}

/// 批处理状态（给前端轮询或一次性返回）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchStatus {
    /// 当前 Job ID
    pub job_id: String,
    /// 任务类型 "translate" | "export"
    pub job_type: String,
    pub total_files: u32,
    pub completed_files: u32,
    pub failed_files: u32,
    /// 当前正在处理的文件名
    #[serde(default)]
    pub current_file: Option<String>,
    /// 当前文件的进度 0.0~1.0
    pub current_file_progress: f32,
    pub total_strings: u32,
    pub translated_strings: u32,
    pub is_running: bool,
    pub is_cancelled: bool,
    pub is_completed: bool,
    pub is_failed: bool,
    #[serde(default)]
    pub errors: Vec<String>,
    /// 已消耗的毫秒数
    pub elapsed_ms: u64,
}

/// 批处理进度事件载荷（实时下发）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchProgress {
    pub job_id: String,
    pub file_path: String,
    /// 当前阶段："parsing", "translating", "saving"
    pub stage: String,
    pub current_file: u32,
    pub total_files: u32,
    pub strings_translated: u32,
    pub total_strings: u32,
    #[serde(default)]
    pub message: String,
}

/// 单文件完成事件
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchFileComplete {
    pub job_id: String,
    pub file_path: String,
    pub translated: u32,
    pub skipped: u32,
    pub errors: u32,
    pub duration_ms: u64,
}

/// 批次完成事件
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchComplete {
    pub job_id: String,
    pub total_files: u32,
    pub success: u32,
    pub failed: u32,
    pub total_translated: u32,
    pub total_errors: u32,
    pub duration_ms: u64,
    pub is_cancelled: bool,
    #[serde(default)]
    pub errors: Vec<BatchFileError>,
}

/// 批处理中出错的文件信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchFileError {
    pub file_path: String,
    pub message: String,
}

/// 保存 Strings 文件响应
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SaveStringsResponse {
    /// 写入的 STRINGS 条目数
    pub strings_count: u32,
    /// 写入的 DLSTRINGS 条目数
    pub dlstrings_count: u32,
    /// 写入的 ILSTRINGS 条目数
    pub ilstrings_count: u32,
    /// 被翻译覆盖的条目总数
    pub translated_count: u32,
}

/// 自动备份请求
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AutoBackupRequest {
    /// SST 文件路径（用于推导备份目录和基础文件名）
    pub sst_path: String,
    /// 最大保留备份数（默认 10）
    pub max_backups: Option<u32>,
}

/// 自动备份响应
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AutoBackupResponse {
    /// 备份文件路径（为空表示无需备份）
    pub backup_path: Option<String>,
    /// 备份目录中的总备份数
    pub total_backups: u32,
}

// ── BSA Browser DTOs ───────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BsaFileEntryDto {
    pub path: String,
    pub size: u64,
    pub compressed: bool,
    pub folder: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BsaFileListDto {
    pub archive_name: String,
    pub version: u32,
    pub total_files: u32,
    pub folders: Vec<String>,
    pub files: Vec<BsaFileEntryDto>,
}

// ── PEX DTOs ────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PexTranslatableDto {
    pub object_name: String,
    pub state_name: String,
    pub function_name: String,
    pub string_type: String,
    pub source_text: String,
    /// 翻译后的文本（为空表示未翻译）
    #[serde(default)]
    pub translation: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PexScriptDto {
    pub script_name: String,
    pub game_id: u16,
    pub major_version: u8,
    pub minor_version: u8,
    pub string_count: u32,
    pub translatable: Vec<PexTranslatableDto>,
}

// ── FUZ DTOs ────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FuzMapping {
    pub response_id: u32,
    pub dialog_text: String,
    pub fuz_file: String,
    pub duration_secs: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FuzScanResponse {
    pub fuz_mappings: Vec<FuzMapping>,
    pub total_fuz_files: u32,
}

// ── Dialog Tree DTOs ─────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DialogInfoDto {
    pub id: u32,
    pub form_id: u32,
    pub source: String,
    pub translation: String,
    pub dialog_text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NpcDialogDto {
    pub npc_edid: String,
    pub dialogues: Vec<DialogInfoDto>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DialogTreeDto {
    pub npcs: Vec<NpcDialogDto>,
}

// ── ESP Compare DTOs ─────────────────────────────────────────────────

/// A single string pair in the comparison result
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EspComparePairDto {
    pub new_id: u32,
    pub old_id: u32,
    pub source: String,
    pub record_sig: String,
    pub field_sig: String,
    pub old_source: String,
    pub new_source: String,
}

/// Summary of the comparison result
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EspCompareResultDto {
    pub identical_count: usize,
    pub added_count: usize,
    pub removed_count: usize,
    pub modified_count: usize,
    /// New IDs that are identical (same key + same text)
    pub identical: Vec<EspComparePairDto>,
    /// New IDs present in new ESP but not in old
    pub added: Vec<EspComparePairDto>,
    /// Old IDs present in old ESP but not in new
    pub removed: Vec<EspComparePairDto>,
    /// New IDs with same key but different text
    pub modified: Vec<EspComparePairDto>,
}

// ── MCM Types ────────────────────────────────────────────────────────

/// 单个 MCM 条目
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McmEntryDto {
    pub id: String,
    pub source: String,
    pub translation: String,
    pub line_index: u32,
    pub byte_offset: u32,
}

/// 已解析的 MCM 文件摘要
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McmFileDto {
    pub path: String,
    pub entry_count: u32,
    pub encoding: String,
    pub entries: Vec<McmEntryDto>,
}

/// MCM 保存请求
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McmSaveRequest {
    pub path: String,
    pub entries: Vec<McmEntryDto>,
}
