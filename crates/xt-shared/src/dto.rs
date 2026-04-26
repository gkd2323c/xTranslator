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
    /// 解析耗时（毫秒）
    pub parse_time_ms: u64,
    /// 各记录类型数量统计
    pub record_counts: HashMap<String, usize>,
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
    /// 匹配成功的条目数
    pub matched: u32,
    /// 未匹配的 XML 条目数
    pub unmatched: u32,
    /// XML 中的总条目数
    pub total: u32,
    /// 被更新的 SkyString 内部 ID 列表（用于前端增量刷新）
    #[serde(default)]
    pub updated_ids: Vec<u32>,
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
