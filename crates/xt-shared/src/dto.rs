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
    /// Editor ID（EDID 字段文本，如 "Whiterun"），ESP 解析时提取；
    /// 非 ESP 来源（SST/XML 直接加载）可能为空。用于 Advanced Search / XML 导出。
    #[serde(default)]
    pub edid: Option<String>,
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
    /// Game actually used to parse the file (canonical `GameId` string such as "SkyrimSE").
    /// Resolution order: explicit request > TES4 Form Version detection > fallback.
    #[serde(default)]
    pub game_id: String,
    /// Game detected from TES4 Form Version, or `None` for unknown/non-standard versions.
    /// A mismatch should warn the frontend without silently switching workspaces.
    #[serde(default)]
    pub detected_game_id: Option<String>,
    /// Source of `game_id`: "requested" | "detected" | "fallback".
    /// "fallback" is not trusted and requires explicit user selection downstream.
    #[serde(default)]
    pub game_source: String,
    /// 每个 Strings 文件的实际来源（0=.STRINGS, 1=.DLSTRINGS, 2=.ILSTRINGS）。
    /// 值为 "disk" | "archive" | "missing"（DP-07，对齐 Delphi bfile[j] 判定）。
    #[serde(default)]
    pub strings_sources: Vec<String>,
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
    /// 导出范围（Delphi `TFormXmlOpt.RadioGroup1` 的 ItemIndex）。
    /// 缺省为 `everything`，保持与旧版“已有译文快速导出”一致。
    #[serde(default)]
    pub scope: Option<XmlExportScopeDto>,
    /// Selection 范围使用的稳定 `u32` 字符串 ID 集合。
    #[serde(default)]
    pub selected_ids: Option<Vec<u32>>,
    /// 是否导出对话条目的 FUZ 数据（对齐 Delphi `chk_exportFuzData`；
    /// FUZ 元数据尚未接通时保留开关语义）。
    #[serde(default)]
    pub export_fuz: bool,
}

/// XML 导出范围（对齐 Delphi `TFormXmlOpt.RadioGroup1` 的 4 档）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum XmlExportScopeDto {
    /// 候选集内全部条目（Delphi `compareOptEverything`）
    Everything,
    /// 仅已翻译或已验证（Delphi `compareOptTranslatedAndValidated`）
    TranslatedAndValidated,
    /// 仅当前选中项（Delphi `compareOptSelection`）
    Selection,
    /// 源文 != 译文，或带协作 ID（Delphi `compareSourceDestDiffandColab`）
    SourceDestDiff,
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
    /// 待处理的文件总数
    pub total_files: u32,
    /// 已成功处理的文件数
    pub completed_files: u32,
    /// 处理失败的文件数
    pub failed_files: u32,
    /// 当前正在处理的文件名
    #[serde(default)]
    pub current_file: Option<String>,
    /// 当前文件的进度 0.0~1.0
    pub current_file_progress: f32,
    /// 全部文件中的字符串总数
    pub total_strings: u32,
    /// 已翻译完成的字符串数
    pub translated_strings: u32,
    /// 任务是否正在运行
    pub is_running: bool,
    /// 任务是否已被用户取消
    pub is_cancelled: bool,
    /// 任务是否全部完成
    pub is_completed: bool,
    /// 任务是否因错误而中止
    pub is_failed: bool,
    #[serde(default)]
    /// 收集到的错误信息列表
    pub errors: Vec<String>,
    /// 已消耗的毫秒数
    pub elapsed_ms: u64,
}

/// 批处理进度事件载荷（实时下发）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchProgress {
    /// 当前处理任务的 Job ID
    pub job_id: String,
    /// 当前文件的完整路径
    pub file_path: String,
    /// 当前阶段："parsing", "translating", "saving"
    pub stage: String,
    /// 当前处理的文件序号（1-based）
    pub current_file: u32,
    /// 待处理的文件总数
    pub total_files: u32,
    /// 当前文件已翻译的字符串数
    pub strings_translated: u32,
    /// 当前文件的字符串总数
    pub total_strings: u32,
    #[serde(default)]
    pub message: String,
}

/// 单文件完成事件
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchFileComplete {
    /// 任务 ID
    pub job_id: String,
    /// 已完成的文件路径
    pub file_path: String,
    /// 成功翻译的字符串数
    pub translated: u32,
    /// 跳过的字符串数（已翻译或无需翻译）
    pub skipped: u32,
    /// 遇到的错误数
    pub errors: u32,
    /// 处理该文件耗时（毫秒）
    pub duration_ms: u64,
}

/// 批次完成事件
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchComplete {
    /// 任务 ID
    pub job_id: String,
    /// 处理的文件总数
    pub total_files: u32,
    /// 成功处理的文件数
    pub success: u32,
    /// 失败的文件数
    pub failed: u32,
    /// 所有文件已翻译的字符串总数
    pub total_translated: u32,
    /// 所有文件遇到的错误总数
    pub total_errors: u32,
    /// 整个批次耗时（毫秒）
    pub duration_ms: u64,
    /// 是否是被用户取消而结束
    pub is_cancelled: bool,
    #[serde(default)]
    pub errors: Vec<BatchFileError>,
}

/// 批处理中出错的文件信息
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BatchFileError {
    /// 出错的文件路径
    pub file_path: String,
    /// 错误描述
    pub message: String,
}

// ── Delphi Command Processor DTOs ─────────────────────────────────

/// Error handling policy for the Delphi-compatible command processor.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandProcessorErrorPolicyDto {
    /// Stop at the first rule/command failure.
    #[default]
    Stop,
    /// Record the failure and continue with later commands/rules.
    Continue,
}

/// Request to parse and execute one Delphi BatchProcessor script.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommandProcessorRunRequest {
    /// Processor source text.
    pub script: String,
    /// Bethesda game Data directory used by `UseDataDir=true` rules.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_dir: Option<String>,
    /// Optional explicit game workspace passed through to `load_esp`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game: Option<String>,
    /// Stop/continue behavior after command failures.
    #[serde(default)]
    pub error_policy: CommandProcessorErrorPolicyDto,
}

/// One structured command processor failure.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommandProcessorFailureDto {
    pub rule_number: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_number: Option<usize>,
    pub line: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    pub message: String,
}

/// File context left open after command processor execution.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommandProcessorActiveFileDto {
    pub esp_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strings_dir: Option<String>,
    pub stats: LoadEspResponse,
}

/// Final report returned by `run_command_processor`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommandProcessorRunResponse {
    pub rules_started: usize,
    pub rules_completed: usize,
    pub commands_succeeded: usize,
    pub failures: Vec<CommandProcessorFailureDto>,
    #[serde(default)]
    pub warnings: Vec<String>,
    /// True when LoadFile/CloseFile/CloseAll changed the backend active-file context.
    #[serde(default)]
    pub file_context_changed: bool,
    /// Active file remaining open after the script, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_file: Option<CommandProcessorActiveFileDto>,
    pub stopped_early: bool,
}

/// Real-time command processor event payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommandProcessorProgressDto {
    /// `rule_start`, `command_start`, `command_done`, `rule_done`, or `message`.
    pub stage: String,
    pub rule_number: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command_number: Option<usize>,
    pub line: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
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

/// BSA 文件浏览器单个文件条目 DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BsaFileEntryDto {
    /// 归档内相对路径
    pub path: String,
    /// 文件大小（字节）
    pub size: u64,
    /// 是否已压缩
    pub compressed: bool,
    /// 所属文件夹名
    pub folder: String,
}

/// BSA 归档文件列表 DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BsaFileListDto {
    /// BSA 文件名
    pub archive_name: String,
    /// BSA 格式版本
    pub version: u32,
    /// 归档中文件总数
    pub total_files: u32,
    /// 包含的文件夹路径列表
    pub folders: Vec<String>,
    /// 所有文件条目
    pub files: Vec<BsaFileEntryDto>,
}

// ── Archive Injection DTOs（DP-06）──────────────────────────────

/// 归档注入请求
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InjectArchiveRequest {
    /// 归档文件路径（.bsa 或 .ba2）
    pub archive_path: String,
    /// 替换映射：小写 `folder/filename` → 新数据（Base64 编码）
    pub replacements: std::collections::HashMap<String, String>,
    /// 替换前是否备份原文件（默认 true）
    #[serde(default = "default_true")]
    pub create_backup: bool,
}

/// 归档注入响应
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InjectArchiveResponse {
    /// 注入的文件数
    pub injected: usize,
    /// 未在归档中找到的请求路径
    pub not_found: Vec<String>,
    /// 备份文件路径（如创建）
    pub backup_path: Option<String>,
    /// 最终归档字节数
    pub output_size: u64,
}

// ── PEX DTOs ────────────────────────────────────────────────────────

/// PEX 脚本中可翻译的字符串条目 DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PexTranslatableDto {
    /// 脚本对象名（即 PEX 文件名）
    pub object_name: String,
    /// 状态名（空字符串表示默认状态）
    pub state_name: String,
    /// 包含该字符串的函数名
    pub function_name: String,
    /// 字符串类型："string"、"none"等
    pub string_type: String,
    /// 源语言文本
    pub source_text: String,
    /// 翻译后的文本（为空表示未翻译）
    #[serde(default)]
    pub translation: String,
}

/// PEX 脚本文件概要 DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PexScriptDto {
    /// 脚本名称（对象名）
    pub script_name: String,
    /// 游戏 ID（PEX 文件头部编码）
    pub game_id: u16,
    /// 主版本号
    pub major_version: u8,
    /// 次版本号
    pub minor_version: u8,
    /// 脚本中的字符串总数（包括不可翻译的）
    pub string_count: u32,
    /// 可翻译条目列表
    pub translatable: Vec<PexTranslatableDto>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecompilePexResponse {
    /// 脚本名称（来自第一个对象）
    pub script_name: String,
    /// 反编译的对象数量
    pub object_count: u32,
    /// 反编译的函数数量
    pub function_count: u32,
    /// 解码的指令数量
    pub instruction_count: u32,
    /// 生成的伪代码
    pub pseudocode: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EspHeaderInfoDto {
    /// HEDR 版本号
    pub version: f32,
    /// 文件中的记录数
    pub num_records: u32,
    /// 下一个可用的 FormID
    pub next_object_id: u32,
    /// 作者名称 (CNAM)
    pub author: String,
    /// 文件描述 (SNAM)
    pub description: String,
    /// 母版文件名列表 (MAST)
    pub masters: Vec<String>,
    /// 被覆盖的 FormID 数量
    pub overridden_count: u32,
    /// 文件是否具有 ESM 标志
    pub is_master: bool,
    /// 文件是否已被本地化
    pub is_localized: bool,
}

/// LIP 关键帧 — FUZ 语音文件的口型同步数据点
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LipKeyframeDto {
    /// 以秒为单位的时间偏移
    pub time: f32,
    /// 口型同步形状索引 (0-15)：0=静音，1=A，2=E，3=I，4=O，5=U，6=F，7=V，8=静音
    pub shape: u8,
}

/// 来自 FUZ 文件的 LIP 数据
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LipDataDto {
    pub version: u32,
    pub keyframes: Vec<LipKeyframeDto>,
}

/// 获取特定 FUZ 文件的 LIP 关键帧数据的响应
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FuzLipDataResponse {
    pub lip_data: Option<LipDataDto>,
    pub duration_secs: f32,
    pub sample_rate: u32,
    pub channels: u16,
}

// ── FUZ DTOs ────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FuzMapping {
    pub response_id: u32,
    pub dialog_text: String,
    pub fuz_file: String,
    pub duration_secs: f32,
    pub has_lip: bool,
    /// FUZ 文件是否解析成功。
    /// 当为 false 时，has_lip 和 duration_secs 将不可信。
    pub parse_ok: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FuzScanResponse {
    pub fuz_mappings: Vec<FuzMapping>,
    pub total_fuz_files: u32,
}

// ── Dialog Tree DTOs ─────────────────────────────────────────────────

/// 对话信息条目 DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DialogInfoDto {
    /// SkyString 唯一 ID
    pub id: u32,
    /// 对话的 FormID
    pub form_id: u32,
    /// 源语言文本
    pub source: String,
    /// 翻译文本
    pub translation: String,
    /// 对话原文（可能与 source 不同，如来自 INFO:NAM1 vs INFO:RNAM）
    pub dialog_text: String,
}

/// 单个 NPC 的对话树 DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NpcDialogDto {
    /// NPC 的 Editor ID
    pub npc_edid: String,
    /// NPC 的对话条目列表
    pub dialogues: Vec<DialogInfoDto>,
}

/// 全局对话树 DTO
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DialogTreeDto {
    /// 所有 NPC 的对话树
    pub npcs: Vec<NpcDialogDto>,
}

// ── ESP Compare DTOs ─────────────────────────────────────────────────

/// 对比结果中的单个字符串对
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

/// 对比结果的摘要
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EspCompareResultDto {
    pub identical_count: usize,
    pub added_count: usize,
    pub removed_count: usize,
    pub modified_count: usize,
    /// 相同的新 ID（键和文本均相同）
    pub identical: Vec<EspComparePairDto>,
    /// 仅在旧 ESP 中不存在，但存在于新 ESP 中的新 ID
    pub added: Vec<EspComparePairDto>,
    /// 仅在旧 ESP 中存在，但不存在于新 ESP 中的旧 ID
    pub removed: Vec<EspComparePairDto>,
    /// 键相同但文本不同（被修改）的新 ID
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

/// 导出最终生成工作流请求，协调调用 save_strings + save_sst + export_xml。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinalizeRequest {
    /// .STRINGS/.DLSTRINGS/.ILSTRINGS 文件的输出目录。
    pub strings_output_dir: String,
    /// 字符串文件的目标语言。
    pub target_lang: String,
    /// ESP 基准名称（例如 "Skyrim"）。
    pub base_name: String,
    /// 可选的 SST 输出路径。如果为 None 则跳过生成 SST。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sst_path: Option<String>,
    /// 可选的 XML 导出路径。如果为 None 则跳过导出 XML。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xml_path: Option<String>,
}

/// 最终导出工作流响应，包含所有生成输出文件的路径。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinalizeResponse {
    /// .STRINGS 文件路径（如果跳过/失败则为空）。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub strings_path: String,
    /// .DLSTRINGS 文件路径（如果跳过/失败或无条目则为空）。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub dlstrings_path: String,
    /// .ILSTRINGS 文件路径（如果跳过/失败或无条目则为空）。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ilstrings_path: String,
    /// SST 字典路径（如果跳过则为空）。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sst_path: String,
    /// XML 导出路径（如果跳过则为空）。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub xml_path: String,
    /// 写入的已翻译字符串数量。
    pub translated_count: u32,
    /// 文件中的总字符串数量。
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
    pub last_game: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_selection_mode: Option<String>,
    /// Strings 加载策略（DP-07）: "disk" | "archive" | "manual"。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strings_strategy: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_server: Option<String>,
    #[serde(default)]
    pub proxy_port: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_password: Option<String>,
    /// ESP 模式：为 true 时，保存操作将直接写回 ESP 文件。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub esp_mode: Option<bool>,
    /// 上次使用的拼写检查词典名称。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spellcheck_dictionary: Option<String>,
    /// 上次保存时拼写检查是否处于活动状态。
    /// false = 已加载但未激活（被关闭），或未加载（见 spellcheck_loaded）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spellcheck_active: Option<bool>,
    /// 上次保存时是否已加载 Hunspell 词典。
    /// true = 启动时自动恢复加载；false = 除非用户手动加载，否则不加载。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spellcheck_loaded: Option<bool>,
    /// 工具箱 TitleCase 转换的例外词列表（换行符分隔的字符串）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub word_exception_list: Option<String>,
}

// ── ESP Write-back DTOs ──────────────────────────────────────────────

/// 直接保存 ESP 的请求（去本地化 ESP 回写）。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SaveEspRequest {
    /// 保存 ESP 文件的路径。
    pub path: String,
    /// 是否在写入前创建备份。
    #[serde(default = "default_true")]
    pub create_backup: bool,
}

fn default_true() -> bool {
    true
}

/// 保存 ESP 的响应。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SaveEspResponse {
    /// 写入的总字节数。
    pub bytes_written: u64,
    /// 修改的记录数量。
    pub records_modified: u32,
}

/// 导出最终生成 ESP 的请求（应用 SST → 重构 → 序列化 → 导出 Strings）。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinalizeEspRequest {
    /// 保存 ESP 文件的路径。
    pub esp_path: String,
    /// 导出 .STRINGS 文件的目录。
    pub strings_dir: String,
    /// 字符串文件的基准名称（例如 "Skyrim"）。
    pub base_name: String,
    /// 语言（例如 "english"）。
    pub language: String,
    /// 是否在写入前创建备份。
    #[serde(default = "default_true")]
    pub create_backup: bool,
}

/// 导出最终生成 ESP 的响应。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinalizeEspResponse {
    /// 保存的 ESP 文件路径。
    pub esp_path: String,
    /// 导出的字符串文件路径。
    pub strings_files: Vec<String>,
    /// 修改的记录数量。
    pub records_modified: u32,
}

/// 去本地化 ESP 的请求（本地化 → 去本地化转换）。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DelocalizeEspRequest {
    /// 保存去本地化 ESP 文件的路径。
    pub esp_path: String,
    /// 导出 .STRINGS 文件的目录。
    pub strings_dir: String,
    /// 字符串文件的基准名称。
    pub base_name: String,
    /// 语言。
    pub language: String,
    /// 是否在写入前创建备份。
    #[serde(default = "default_true")]
    pub create_backup: bool,
}

/// 去本地化 ESP 的响应。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DelocalizeEspResponse {
    /// 被去本地化的字符串数量。
    pub new_string_count: u32,
    /// 导出的字符串文件路径。
    pub strings_files_paths: Vec<String>,
}

/// 检查未决翻译缓存的响应。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckPendingCacheResponse {
    /// 如果没有未决缓存则为 null，否则返回恢复的详细信息。
    pub recovery: Option<RecoveryInfo>,
}

/// 未决翻译缓存的恢复信息。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecoveryInfo {
    /// ESP 文件名称
    pub esp_name: String,
    /// 待恢复的未决翻译数量
    pub pending_count: u32,
    /// 缓存文件路径
    pub cache_file_path: String,
}

/// 拼写错误单词及其字节位置。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpellFaultDto {
    /// 拼写错误的单词
    pub word: String,
    /// 单词在原文中的起始字节偏移
    pub start_byte: usize,
    /// 单词在原文中的结束字节偏移（不含）
    pub end_byte: usize,
}

/// 拼写检查分析结果。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpellCheckResultDto {
    /// 汉语检测到的错误列表
    pub faults: Vec<SpellFaultDto>,
    /// 检测到的单词总数
    pub total_words: usize,
    /// 错误比率锁定标志（true 表示该居所已达锃误阈值）
    pub fault_ratio_locked: bool,
    /// 拼写检查功能是否已激活
    pub active: bool,
}

/// 拼写检查配置。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpellCheckConfigDto {
    /// 当前可用的词典文件名列表
    pub available_dictionaries: Vec<String>,
    /// 当前选中的词典
    pub current_dictionary: Option<String>,
    /// 拼写检查是否已激活
    pub active: bool,
    /// 词典是否已载入内存
    pub loaded: bool,
}

/// SST 合并统计信息。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MergeStatsDto {
    /// 新增的条目数
    pub added: usize,
    /// 更新的条目数
    pub updated: usize,
    /// 被强制覆盖的条目数
    pub overwritten: usize,
    /// 因冲突而跳过的条目数
    pub conflicts_skipped: usize,
}

/// 应用翻译缓存恢复的响应。
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApplyCacheResponse {
    /// 成功应用的翻译条目数
    pub applied_count: u32,
}

// ── SST 高级应用选项 (DP-03 Delphi 对齐) ──────────────────────────

/// SST 覆盖范围（对齐 Delphi TESVT_ApplySSTOpts RadioGroup1）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SstOverwriteScopeDto {
    /// 全部项（未锁定）
    All,
    /// 仅未翻译项（排除已翻译/已验证）
    NoTransExclusive,
    /// 严格未翻译项（保留 Delphi 原名；排除已翻译/已验证/部分翻译）
    NoTransAndPartial,
    /// 仅部分翻译项
    PartialOnly,
    /// 仅选中的项
    Selection,
}

impl Default for SstOverwriteScopeDto {
    fn default() -> Self {
        // SST import 使用 Mode=1；Delphi iCompareEspOpt[1] 初始值为 0 (= All)。
        Self::All
    }
}

/// SST 匹配模式（对齐 Delphi TESVT_ApplySSTOpts RadioGroup2）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SstMatchModeDto {
    /// FormID + EDID hash + field + index
    FormIdOnly,
    /// FormID + EDID hash + 严格源文本 + field + index
    FormIdStrictString,
    /// FormID + EDID hash + field + 严格源文本，忽略 index
    FormIdRelaxedString,
    /// 仅源文本精确匹配（忽略 FormID）
    StringOnly,
}

impl Default for SstMatchModeDto {
    fn default() -> Self {
        Self::FormIdStrictString
    }
}

/// 应用 SST 时的完整高级选项
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SstApplyOptionsDto {
    /// 覆盖范围
    #[serde(default)]
    pub overwrite_scope: SstOverwriteScopeDto,
    /// 匹配模式
    #[serde(default)]
    pub match_mode: SstMatchModeDto,
    /// 仅打标记，不修改翻译文本
    #[serde(default)]
    pub tag_only: bool,
    /// 源语言与目标语言相同，对齐 Delphi TESVTSameLanguage
    #[serde(default)]
    pub same_language: bool,
    /// 匹配前重置覆盖范围内的目标字符串状态（未命中项也会被重置）
    #[serde(default)]
    pub reset_state: bool,
    /// 仅限制在当前过滤结果中应用
    #[serde(default)]
    pub restrict_to_filter: bool,
    /// 选中的条目 ID 列表（当 overwrite_scope 为 Selection 时生效）
    #[serde(default)]
    pub selected_ids: Option<Vec<u32>>,
    /// 当前过滤可见的条目 ID 列表（当 restrict_to_filter 为 true 时生效）
    #[serde(default)]
    pub filtered_ids: Option<Vec<u32>>,
}

// ── DEF_UI / Component Generator DTOs (DP-10) ──────────────────────────

/// Shared scope enum for DefUI / AddId style batch tools.
/// Canonical values are the string literals below; UI and core must use them verbatim.
pub mod def_ui_scope {
    pub const ALL: &str = "all";
    pub const ONLY_UNTRANSLATED: &str = "only_untranslated";
    pub const ONLY_SELECTED: &str = "only_selected";
}

/// Single request DTO for `apply_def_ui_generator`.
/// Single-object contract so frontend/backend parameter shapes cannot drift again.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefUiApplyRequestDto {
    /// Explicit game context (frontend `currentGame`). Optional: when omitted,
    /// backend falls back to TES4 form-version detection of the loaded ESP.
    #[serde(default)]
    pub game: Option<String>,
    pub options: DefUiOptionsDto,
    /// Scope: one of `def_ui_scope` constants ("all" | "only_untranslated" | "only_selected")
    pub scope: String,
    /// Selected row IDs; required when scope == only_selected
    #[serde(default)]
    pub selected_ids: Vec<u32>,
    /// Preview only (do not mutate AppState)
    #[serde(default)]
    pub preview_only: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefUiOptionsDto {
    pub use_source_for_string: bool,
    pub use_source_for_components: bool,
    pub clean_base: bool,
    pub clean_compo: bool,
    pub add_quantity: bool,
    pub use_first_char: bool,
    pub do_auto_header: bool,
    pub regex_clean_base: String,
    pub regex_clean_compo: String,
    pub template: String,
    pub template_with_weight: Option<String>,
    pub component_separator: String,
    pub quantity_indicator1: String,
    pub quantity_indicator2: String,
    pub ignore_list: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefUiApplyResultDto {
    pub modified_count: u32,
    pub total_misc_records: u32,
    pub details: Vec<DefUiItemPreviewDto>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefUiItemPreviewDto {
    pub string_id: u32,
    pub form_id: u32,
    pub edid: String,
    pub original: String,
    pub generated: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodepageInfoDto {
    pub current_codepage: String,
    pub supported_codepages: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodepageOverrideRequestDto {
    pub codepage: String,
    pub save_as_default: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddIdRequestDto {
    pub offset_value: i64,
    pub apply_to_form_id: bool,
    pub scope: String, // "all" | "only_untranslated" | "only_selected"
    pub selected_ids: Option<Vec<u32>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddIdResultDto {
    pub modified_count: u32,
    pub total_processed: u32,
}

/// AddIdToStrings 请求 DTO（Delphi `addIdToStringEx` 的 Rust 等价）
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddIdToStringsRequestDto {
    /// 作用范围："everything" | "no_trans_valid" | "selection"
    pub scope: String,
    pub selected_ids: Option<Vec<u32>>,
    /// 添加 String ID 前缀 `[%.5x]`
    pub add_string_id: bool,
    /// 添加 FormID 前缀 `[%.8x]`
    pub add_form_id: bool,
    /// 添加记录/字段引用 `[REC:FIELD]`
    pub add_record_ref: bool,
    /// 添加 DIAL master 引用 `[@%.8x]`（仅 INFO 记录）
    pub add_dial_ref: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddIdToStringsResultDto {
    pub modified_count: u32,
    pub total_processed: u32,
}

