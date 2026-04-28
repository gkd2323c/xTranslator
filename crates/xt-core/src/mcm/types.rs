//! MCM 类型定义

use serde::{Deserialize, Serialize};

/// 占位符前缀（与 Delphi 原版一致）
pub const XTAG_PREFIX: &str = "{{xt=";
pub const XTAG_SUFFIX: &str = "}} ";

/// 从索引生成占位符
pub fn make_xtag(index: usize) -> String {
    format!("{}{}}}{{{}", XTAG_PREFIX, index, XTAG_SUFFIX)
}

/// 从占位符中提取索引（返回 None 表示不是占位符）
pub fn parse_xtag(s: &str) -> Option<usize> {
    let start = s.find(XTAG_PREFIX)?;
    let rest = &s[start + XTAG_PREFIX.len()..];
    let end = rest.find(XTAG_SUFFIX)?;
    rest[..end].parse().ok()
}

/// 单个 MCM 条目
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct McmEntry {
    /// 键，如 "$sMySetting"
    pub id: String,
    /// 原文
    pub source: String,
    /// 译文（可为空）
    #[serde(default)]
    pub translation: String,
    /// 所在行号（0-based）
    pub line_index: usize,
    /// 该行在原始文件中的字节偏移量
    pub byte_offset: usize,
}

/// 已解析的 MCM 文件
#[derive(Clone, Debug)]
pub struct McmFile {
    /// 所有条目（按行顺序）
    pub entries: Vec<McmEntry>,
    /// 归一化后的行（原文被替换为 {{xt=N}} 占位符）
    pub normalized_lines: Vec<String>,
    /// 原始 key 列表（用于保存重建，按行顺序）
    pub header_list: Vec<String>,
    /// 原始文件编码
    pub encoding: McmEncoding,
    /// 文件路径
    pub path: String,
}

/// MCM 文件编码
#[derive(Clone, Debug, Default, PartialEq)]
pub enum McmEncoding {
    /// UTF-16 Little Endian（Delphi tEncoding.unicode，主流格式）
    #[default]
    Utf16Le,
    /// UTF-8
    Utf8,
    /// UTF-16 Big Endian
    Utf16Be,
    /// Windows ANSI（codepage）
    Ansi(u16),
}