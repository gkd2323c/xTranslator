use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 前后端共享 DTO（Tauri IPC 载荷）。
///
/// 设计约束：
/// - 字段尽量稳定，避免频繁破坏前后端兼容。
/// - 新增字段优先使用 `#[serde(default)]`，降低旧版本调用方升级成本。
/// - 所有 DTO 必须同时在 Rust 和 TypeScript 中定义（ui/src/api/strings.ts）。

/// 视口分页查询请求
///
/// 用于前端虚拟滚动（react-window）的分页加载。
/// 前端维护完整数据集 `allItems`，通过此请求获取当前视口的数据。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryRequest {
    /// 文件 ID（阶段 0 固定为 "test"）
    pub file_id: String,
    /// 视口起始偏移（0-based）
    pub offset: u32,
    /// 视口大小（通常 50-100，取决于行高和窗口大小）
    pub limit: u32,
    /// 搜索过滤词（在 source/translation 中搜索，支持模糊匹配）
    #[serde(default)]
    pub filter: Option<String>,
    /// 排序字段（如 "source", "translation", "form_id"）
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
///
/// 返回当前视口的数据片段及统计信息。
/// 前端使用 `total` 和 `filtered` 来计算虚拟滚动的总行数。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueryResponse {
    /// 总记录数（未应用任何过滤）
    pub total: u32,
    /// 筛选后总数（应用了 filter/status_filter 后的结果数）
    pub filtered: u32,
    /// 当前视口数据（最多 limit 条）
    pub items: Vec<SkyStringDTO>,
    /// 当前偏移（回显请求中的 offset，用于验证）
    pub offset: u32,
    /// 响应耗时（毫秒），用于性能观测与回归对比
    pub elapsed_ms: u64,
}

/// 前端展示的 SkyString 简化 DTO
///
/// 这是 SkyString 的精简版本，用于 IPC 传输。
/// 完整的 SkyString 包含更多内部字段（如 params、ld_found 等），
/// 这里只保留前端需要的字段以减少 JSON 体积。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkyStringDTO {
    /// 内部稳定 ID（用于更新定位），不是 ESP 的 str_id
    /// 前端使用此 ID 来定位要更新的字符串，避免因排序/过滤导致的索引错位
    pub id: u32,
    /// 源文本（通常为英文）
    pub source: String,
    /// 翻译文本（目标语言）
    pub translation: String,
    /// 记录类型签名（如 "DIAL", "INFO", "BOOK"），用于分类和过滤
    pub record_sig: String,
    /// 字段签名（如 "FULL", "DESC"），标识记录内的具体字段
    pub field_sig: String,
    /// FormID（十六进制字符串，如 "0x00012345"），用于定位 ESP 中的对象
    pub form_id: String,
    /// 翻译状态："translated" / "incomplete" / "locked"
    pub status: String,
    /// Strings 文件类型索引: 0=.STRINGS, 1=.DLSTRINGS, 2=.ILSTRINGS
    /// 用于 SST/XML 导出时确定字符串的目标文件
    #[serde(default)]
    pub list_index: u8,
    /// Strings 文件中的字符串 ID（用于 SST/XML 精确匹配）
    /// 这是 Bethesda 格式中的字符串索引，不同于内部 ID
    #[serde(default)]
    pub str_id: i32,
    /// 是否为 VMAD 脚本字符串（负 str_id 编码偏移量）
    /// VMAD 是 Skyrim 脚本虚拟机的字符串存储方式
    #[serde(default)]
    pub is_vmad: bool,
    /// 启发式搜索匹配数量（0-255，对应 Delphi LD 列）
    /// 表示与词汇库中的相似条目数，用于翻译建议
    #[serde(default)]
    pub ld: u8,
}

/// 加载 ESP 文件响应
///
/// 包含 ESP 解析的统计信息和缓存状态。
/// 前端使用这些信息来更新侧边栏统计和加载状态指示。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoadEspResponse {
    /// 解析出的总字符串数（包括所有记录类型）
    pub total: u32,
    /// 压缩记录数（使用 zlib 压缩的 ESP 记录数）
    pub compressed_records: u32,
    /// 成功加载的 Strings 文件数 (0-3)
    /// 0 = 未加载任何 Strings 文件
    /// 1-3 = 分别加载了 .STRINGS / .DLSTRINGS / .ILSTRINGS
    pub strings_loaded: u8,
    /// 解析耗时（毫秒）；缓存命中时为 0
    pub parse_time_ms: u64,
    /// 各记录类型数量统计（如 {"DIAL": 1000, "INFO": 5000, "BOOK": 200}）
    pub record_counts: HashMap<String, usize>,
    /// 是否从缓存加载（而非完整解析）
    /// 缓存命中时为 true，此时 parse_time_ms 为 0
    #[serde(default)]
    pub cached: bool,
    /// ESP 文件 SHA-256 哈希（用于翻译缓存关联）
    /// 用于内容寻址缓存，确保文件内容变化时缓存失效
    #[serde(default)]
    pub esp_hash: String,
}

/// ESP 文件加载进度事件
///
/// 通过 Tauri 事件系统实时发送给前端，用于显示加载进度条。
/// 前端监听 "esp-load-progress" 事件并更新 UI。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EspLoadProgress {
    /// 加载阶段："reading_defs", "loading_strings", "parsing", "finalizing"
    /// 用于前端显示当前处理阶段的描述
    pub stage: String,
    /// 当前进度值（字节数或条目数，取决于阶段）
    pub current: u64,
    /// 总进度值（文件大小或总条目数）
    pub total: u64,
    /// 百分比 (0-100)，便于前端直接用于进度条
    pub percentage: u8,
    /// 用户可读的消息（如 "Parsing... 45.2%"）
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecompilePexResponse {
    /// Script name (from first object)
    pub script_name: String,
    /// Number of objects decompiled
    pub object_count: u32,
    /// Number of functions decompiled
    pub function_count: u32,
    /// Number of instructions decoded
    pub instruction_count: u32,
    /// Generated pseudocode
    pub pseudocode: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EspHeaderInfoDto {
    /// HEDR version
    pub version: f32,
    /// Number of records in the file
    pub num_records: u32,
    /// Next available FormID
    pub next_object_id: u32,
    /// Author name (CNAM)
    pub author: String,
    /// File description (SNAM)
    pub description: String,
    /// Master file names (MAST)
    pub masters: Vec<String>,
    /// Number of overridden FormIDs
    pub overridden_count: u32,
    /// Whether the file has the ESM flag
    pub is_master: bool,
    /// Whether the file is localized
    pub is_localized: bool,
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

/// MCM Compare 覆盖策略（对应 Delphi RadioGroup1）
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McmComparePolicy {
    /// 覆盖所有匹配项
    All,
    /// 仅覆盖未翻译的项
    NoTrans,
    /// 覆盖未翻译 + 部分翻译
    NoTransAndPartial,
    /// 仅覆盖部分翻译
    PartialOnly,
}

/// MCM Compare 请求
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McmCompareRequest {
    /// 当前 McmPanel 中的条目
    pub entries: Vec<McmEntryDto>,
    /// 参考 MCM 文件路径
    pub reference_path: String,
    /// 覆盖策略
    pub policy: McmComparePolicy,
}

/// MCM Compare 结果
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McmCompareResult {
    /// 匹配到的条目数
    pub matched: u32,
    /// 未匹配的条目数
    pub unmatched: u32,
    /// 更新后的条目（带新译文）
    pub updated_entries: Vec<McmEntryDto>,
}

// ── Data Config DTOs ────────────────────────────────────────────────

/// CTDA 函数信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CtdaFuncDto {
    pub id: u32,
    pub name: String,
    pub params: String,
}

/// 字段大小信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FieldSizeInfoDto {
    pub max_size: u32,
    pub can_wrap: bool,
}

/// Data 配置文件摘要
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataConfigsDto {
    pub ctda_funcs: Vec<CtdaFuncDto>,
    pub field_size_ref: HashMap<String, FieldSizeInfoDto>,
    pub dial_sub_type: HashMap<String, String>,
    pub emote_definition: HashMap<String, String>,
}

// ── API Config DTOs ──────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiProviderInfo {
    pub name: String,
    pub label: String,
    pub enabled: bool,
    pub models: Vec<String>,
    pub default_query: Option<String>,
    pub char_limit: u32,
    pub array_limit: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiConfigResponse {
    pub providers: Vec<ApiProviderInfo>,
}

// ── Finalize DTOs ─────────────────────────────────────────────────────

/// Finalize workflow request — orchestrates save_strings + save_sst + export_xml.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinalizeRequest {
    /// Output directory for .STRINGS/.DLSTRINGS/.ILSTRINGS files.
    pub strings_output_dir: String,
    /// Target language for strings files.
    pub target_lang: String,
    /// ESP base name (e.g. "Skyrim").
    pub base_name: String,
    /// Optional SST output path. If None, SST is skipped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sst_path: Option<String>,
    /// Optional XML output path. If None, XML export is skipped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xml_path: Option<String>,
}

/// Finalize workflow response — paths of all generated output files.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinalizeResponse {
    /// .STRINGS file path (empty if skipped/failed).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub strings_path: String,
    /// .DLSTRINGS file path (empty if skipped/failed or no entries).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub dlstrings_path: String,
    /// .ILSTRINGS file path (empty if skipped/failed or no entries).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ilstrings_path: String,
    /// SST dictionary path (empty if skipped).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sst_path: String,
    /// XML export path (empty if skipped).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub xml_path: String,
    /// Number of translated strings written.
    pub translated_count: u32,
    /// Total strings in the file.
    pub total_count: u32,
}

// ── TCSC Types ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TcscDirection {
    ToSimplified,
    ToTraditional,
}

// ── Config DTOs ─────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AppConfigDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub openai_api_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deepl_api_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baidu_app_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baidu_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub youdao_app_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub youdao_secret_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub azure_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_server: Option<String>,
    #[serde(default)]
    pub proxy_port: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_password: Option<String>,
    /// ESP mode: when true, save operations write back to the ESP file directly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub esp_mode: Option<bool>,
}

// ── ESP Write-back DTOs ──────────────────────────────────────────────

/// Request for saving ESP directly (delocalized ESP write-back).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SaveEspRequest {
    /// Path to save the ESP file.
    pub path: String,
    /// Whether to create a backup before writing.
    #[serde(default = "default_true")]
    pub create_backup: bool,
}

fn default_true() -> bool {
    true
}

/// Response from saving ESP.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SaveEspResponse {
    /// Total bytes written.
    pub bytes_written: u64,
    /// Number of records that were modified.
    pub records_modified: u32,
}

/// Request for finalizing ESP (apply SST → rebuild → serialize → export Strings).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinalizeEspRequest {
    /// Path to save the ESP file.
    pub esp_path: String,
    /// Directory to export .STRINGS files.
    pub strings_dir: String,
    /// Base name for strings files (e.g., "Skyrim").
    pub base_name: String,
    /// Language (e.g., "english").
    pub language: String,
    /// Whether to create a backup before writing.
    #[serde(default = "default_true")]
    pub create_backup: bool,
}

/// Response from finalizing ESP.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinalizeEspResponse {
    /// Path to the saved ESP file.
    pub esp_path: String,
    /// Paths to exported strings files.
    pub strings_files: Vec<String>,
    /// Number of records modified.
    pub records_modified: u32,
}

/// Request for delocalizing ESP (localized → delocalized conversion).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DelocalizeEspRequest {
    /// Path to save the delocalized ESP file.
    pub esp_path: String,
    /// Directory to export .STRINGS files.
    pub strings_dir: String,
    /// Base name for strings files.
    pub base_name: String,
    /// Language.
    pub language: String,
    /// Whether to create a backup before writing.
    #[serde(default = "default_true")]
    pub create_backup: bool,
}

/// Response from delocalizing ESP.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DelocalizeEspResponse {
    /// Number of strings delocalized.
    pub new_string_count: u32,
    /// Paths to exported strings files.
    pub strings_files_paths: Vec<String>,
}

/// Response for checking pending translation cache.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckPendingCacheResponse {
    /// null if no pending cache, or the recovery details.
    pub recovery: Option<RecoveryInfo>,
}

/// Recovery info for pending translation cache.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecoveryInfo {
    pub esp_name: String,
    pub pending_count: u32,
    pub cache_file_path: String,
}

/// Spell check fault word with byte positions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpellFaultDto {
    pub word: String,
    pub start_byte: usize,
    pub end_byte: usize,
}

/// Spell check analysis result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpellCheckResultDto {
    pub faults: Vec<SpellFaultDto>,
    pub total_words: usize,
    pub fault_ratio_locked: bool,
    pub active: bool,
}

/// Spell check configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpellCheckConfigDto {
    pub available_dictionaries: Vec<String>,
    pub current_dictionary: Option<String>,
    pub active: bool,
    pub loaded: bool,
}

/// SST merge statistics.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MergeStatsDto {
    pub added: usize,
    pub updated: usize,
    pub overwritten: usize,
    pub conflicts_skipped: usize,
}

/// Response for applying translation cache recovery.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApplyCacheResponse {
    pub applied_count: u32,
}
