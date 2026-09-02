//! Codepage 编码系统
//! 对应 Delphi TESVT_fstreamSave.pas 中的 getcodepage / rawStringtoString 逻辑
//!
//! 配置文件格式 (Data/<game>/codepage.txt):
//! ```text
//! # 格式: language=primary_codepage[,fallback_codepage]
//! english=utf8,1252
//! chinese=utf8
//! japanese=utf8,932
//! ```
//!
//! 规则:
//! - 主编码: 读取和保存时使用
//! - 降级编码: 仅当 UTF-8 解码失败时使用（仅读取时）
//! - utf8 = codepage 65001

use encoding_rs::{Encoding, BIG5, EUC_KR, GBK, SHIFT_JIS};
use std::collections::HashMap;
use std::fmt;
use std::path::Path;

/// Codepage 编号对应的编码定义
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodepageId {
    Utf8,   // 65001
    Cp932,  // Shift-JIS (Japanese)
    Cp936,  // GBK (Chinese Simplified)
    Cp949,  // EUC-KR (Korean)
    Cp950,  // Big5 (Chinese Traditional)
    Cp1250, // Central European
    Cp1251, // Cyrillic
    Cp1252, // Western
    Cp1253, // Greek
    Cp1254, // Turkish
    Cp1256, // Arabic
    Cp1257, // Baltic (Estonian etc.)
}

impl CodepageId {
    /// 从 codepage 数字解析
    pub fn from_number(n: u32) -> Option<Self> {
        match n {
            65001 | 0 => Some(CodepageId::Utf8),
            932 => Some(CodepageId::Cp932),
            936 => Some(CodepageId::Cp936),
            949 => Some(CodepageId::Cp949),
            950 => Some(CodepageId::Cp950),
            1250 => Some(CodepageId::Cp1250),
            1251 => Some(CodepageId::Cp1251),
            1252 => Some(CodepageId::Cp1252),
            1253 => Some(CodepageId::Cp1253),
            1254 => Some(CodepageId::Cp1254),
            1256 => Some(CodepageId::Cp1256),
            1257 => Some(CodepageId::Cp1257),
            _ => None,
        }
    }

    /// 从字符串解析 (配置文件中的值，如 "utf8" 或 "1252")
    pub fn from_str_value(s: &str) -> Option<Self> {
        let s_lower = s.to_lowercase();
        if s_lower == "utf8" || s_lower == "utf-8" {
            return Some(CodepageId::Utf8);
        }
        // 尝试解析数字
        if let Ok(n) = s_lower.parse::<u32>() {
            return Self::from_number(n);
        }
        None
    }

    /// 获取对应的 encoding_rs 编码器
    pub fn encoding(self) -> &'static Encoding {
        match self {
            CodepageId::Utf8 => encoding_rs::UTF_8,
            CodepageId::Cp932 => SHIFT_JIS,
            CodepageId::Cp936 => GBK,
            CodepageId::Cp949 => EUC_KR,
            CodepageId::Cp950 => BIG5,
            CodepageId::Cp1250 => encoding_rs::WINDOWS_1250,
            CodepageId::Cp1251 => encoding_rs::WINDOWS_1251,
            CodepageId::Cp1252 => encoding_rs::WINDOWS_1252,
            CodepageId::Cp1253 => encoding_rs::WINDOWS_1253,
            CodepageId::Cp1254 => encoding_rs::WINDOWS_1254,
            CodepageId::Cp1256 => encoding_rs::WINDOWS_1256,
            CodepageId::Cp1257 => encoding_rs::WINDOWS_1257,
        }
    }

    /// 是否是 UTF-8 编码
    pub fn is_utf8(self) -> bool {
        self == CodepageId::Utf8
    }
}

impl fmt::Display for CodepageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CodepageId::Utf8 => write!(f, "utf8"),
            other => write!(f, "{}", other.number()),
        }
    }
}

/// 辅助：获取 codepage 数字
impl CodepageId {
    pub fn number(self) -> u32 {
        match self {
            CodepageId::Utf8 => 65001,
            CodepageId::Cp932 => 932,
            CodepageId::Cp936 => 936,
            CodepageId::Cp949 => 949,
            CodepageId::Cp950 => 950,
            CodepageId::Cp1250 => 1250,
            CodepageId::Cp1251 => 1251,
            CodepageId::Cp1252 => 1252,
            CodepageId::Cp1253 => 1253,
            CodepageId::Cp1254 => 1254,
            CodepageId::Cp1256 => 1256,
            CodepageId::Cp1257 => 1257,
        }
    }
}

/// 编码配置（主编码 + 可选降级编码）
#[derive(Clone, Debug)]
pub struct CodepageConfig {
    /// 主编码（读取和保存）
    pub primary: CodepageId,
    /// 降级编码（仅读取，UTF-8 失败时使用）
    pub fallback: Option<CodepageId>,
}

impl CodepageConfig {
    /// UTF-8 无降级的默认配置
    pub fn utf8() -> Self {
        Self {
            primary: CodepageId::Utf8,
            fallback: None,
        }
    }

    /// 从名字创建单一编码配置（如 "utf8", "936", "1252" 等）
    pub fn from_name(name: &str) -> Option<Self> {
        let id = CodepageId::from_str_value(name)?;
        Some(Self {
            primary: id,
            fallback: None,
        })
    }

    /// UTF-8 + 降级编码
    pub fn utf8_with_fallback(fallback: CodepageId) -> Self {
        Self {
            primary: CodepageId::Utf8,
            fallback: Some(fallback),
        }
    }

    /// 从配置行解析 (如 "english=utf8,1252")
    fn parse_line(line: &str) -> Option<(String, Self)> {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }

        // 仅按第一个 '=' 分割，避免右侧出现额外 '=' 时误拆分。
        let parts: Vec<&str> = line.splitn(2, '=').collect();
        if parts.len() != 2 {
            return None;
        }

        let lang = parts[0].trim().to_lowercase();
        // 右侧允许 "主编码,降级编码" 两段；缺省第二段表示无降级编码。
        let encodings: Vec<&str> = parts[1].trim().split(',').collect();

        let primary = CodepageId::from_str_value(encodings[0].trim())?;
        let fallback = encodings
            .get(1)
            .and_then(|s| CodepageId::from_str_value(s.trim()));

        Some((lang, CodepageConfig { primary, fallback }))
    }

    /// 解码字节到字符串
    ///
    /// 对应 Delphi rawStringtoString:
    /// 1. 如果主编码是 UTF-8: 先尝试 UTF-8，失败则用 fallback
    /// 2. 如果主编码是其他: 直接用主编码解码
    pub fn decode(&self, bytes: &[u8]) -> String {
        if self.primary.is_utf8() {
            // 先尝试 UTF-8
            match String::from_utf8(bytes.to_vec()) {
                Ok(s) => return s,
                Err(_) => {
                    // UTF-8 失败：按配置尝试降级编码（与 Delphi 行为一致）。
                    if let Some(fb) = self.fallback {
                        return decode_with_codepage(bytes, fb);
                    }
                    // 无降级编码时做保底解码，保证不 panic 且尽量保留信息。
                    return bytes.iter().map(|&b| b as char).collect();
                }
            }
        }
        // 主编码不是 UTF-8：直接按主编码解码，不走 UTF-8 探测。
        decode_with_codepage(bytes, self.primary)
    }

    /// 将字符串编码为字节（用于写入）
    ///
    /// 对应 Delphi codepage.f 写入函数
    pub fn encode(&self, s: &str) -> Vec<u8> {
        if self.primary.is_utf8() {
            return s.as_bytes().to_vec();
        }
        encode_with_codepage(s, self.primary)
    }
}

/// 支持的手动覆盖代码页列表（对齐 Delphi supportedCodepage）
pub const SUPPORTED_CODEPAGES: &[&str] = &[
    "utf8", "utf16", "1250", "1251", "1252", "1253", "1254", "1256", "932", "936", "950",
];

/// Codepage 配置表（按语言名索引）
#[derive(Clone, Debug, Default)]
pub struct CodepageTable {
    /// 语言名 -> 编码配置
    entries: HashMap<String, CodepageConfig>,
}

impl CodepageTable {
    /// 创建空表
    pub fn new() -> Self {
        Self::default()
    }

    /// 从 codepage.txt 内容解析
    pub fn parse(content: &str) -> Self {
        let mut entries = HashMap::new();
        for line in content.lines() {
            if let Some((lang, config)) = CodepageConfig::parse_line(line) {
                // 同语言重复定义时，后出现的配置覆盖先前配置。
                entries.insert(lang, config);
            }
        }
        Self { entries }
    }

    /// 从文件加载
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())?;
        Ok(Self::parse(&content))
    }

    /// 根据语言名查询编码配置
    pub fn get(&self, language: &str) -> Option<&CodepageConfig> {
        self.entries.get(&language.to_lowercase())
    }

    /// 注册或覆盖语言的 codepage 配置
    pub fn register(&mut self, language: &str, config: CodepageConfig) {
        self.entries.insert(language.to_lowercase(), config);
    }

    /// 使用特定的强制代码页覆盖某个语言条目（如 "utf8", "936", "1252" 等）
    pub fn set_override(&mut self, language: &str, codepage: &str) {
        if let Some(cfg) = CodepageConfig::from_name(codepage) {
            self.entries.insert(language.to_lowercase(), cfg);
        }
    }

    /// 根据语言名查询，找不到则返回 UTF-8 默认
    pub fn get_or_utf8(&self, language: &str) -> CodepageConfig {
        // 查不到配置时回退到 UTF-8，避免上层出现 None 分支扩散。
        self.get(language).cloned().unwrap_or_default()
    }

    /// 从文件名推断语言并查询编码配置
    ///
    /// 文件名格式: `<plugin>_<language>.<ext>`
    /// 如 `skyrim_english.STRINGS` → language = "english"
    /// 如 `skyrim_japanese.DLSTRINGS` → language = "japanese"
    pub fn get_for_filename(&self, filename: &str) -> CodepageConfig {
        let lang = extract_language_from_filename(filename);
        // 语言提取失败时 lang 为空字符串，会走 UTF-8 默认配置。
        self.get_or_utf8(&lang)
    }

    /// 条目数量
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for CodepageConfig {
    fn default() -> Self {
        Self::utf8()
    }
}

/// 从 Strings 文件名提取语言名
///
/// 如 `skyrim_english.STRINGS` → "english"
/// 如 `skyrim_japanese.DLSTRINGS` → "japanese"
fn extract_language_from_filename(filename: &str) -> String {
    let path = Path::new(filename);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

    // 使用“最后一个下划线”规则：
    // 可兼容带下划线的插件名，如 my_mod_english.STRINGS。
    if let Some(pos) = stem.rfind('_') {
        stem[pos + 1..].to_lowercase()
    } else {
        String::new()
    }
}

/// 使用指定 codepage 解码字节
fn decode_with_codepage(bytes: &[u8], cp: CodepageId) -> String {
    let enc = cp.encoding();
    // encoding_rs 会对非法序列做替换，不抛异常。
    let (cow, _encoding_used, _had_errors) = enc.decode(bytes);
    cow.into_owned()
}

/// 使用指定 codepage 编码字符串
fn encode_with_codepage(s: &str, cp: CodepageId) -> Vec<u8> {
    let enc = cp.encoding();
    // 无法表示的字符会按 encoding_rs 规则替换，避免编码阶段失败。
    let (cow, _encoding_used, _had_errors) = enc.encode(s);
    cow.into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codepage_id_from_number() {
        assert_eq!(CodepageId::from_number(65001), Some(CodepageId::Utf8));
        assert_eq!(CodepageId::from_number(932), Some(CodepageId::Cp932));
        assert_eq!(CodepageId::from_number(936), Some(CodepageId::Cp936));
        assert_eq!(CodepageId::from_number(949), Some(CodepageId::Cp949));
        assert_eq!(CodepageId::from_number(950), Some(CodepageId::Cp950));
        assert_eq!(CodepageId::from_number(1250), Some(CodepageId::Cp1250));
        assert_eq!(CodepageId::from_number(1251), Some(CodepageId::Cp1251));
        assert_eq!(CodepageId::from_number(1252), Some(CodepageId::Cp1252));
        assert_eq!(CodepageId::from_number(1253), Some(CodepageId::Cp1253));
        assert_eq!(CodepageId::from_number(1254), Some(CodepageId::Cp1254));
        assert_eq!(CodepageId::from_number(1256), Some(CodepageId::Cp1256));
        assert_eq!(CodepageId::from_number(1257), Some(CodepageId::Cp1257));
        assert_eq!(CodepageId::from_number(999), None);
    }

    #[test]
    fn test_codepage_id_from_str() {
        assert_eq!(CodepageId::from_str_value("utf8"), Some(CodepageId::Utf8));
        assert_eq!(CodepageId::from_str_value("UTF8"), Some(CodepageId::Utf8));
        assert_eq!(CodepageId::from_str_value("1252"), Some(CodepageId::Cp1252));
        assert_eq!(CodepageId::from_str_value("932"), Some(CodepageId::Cp932));
    }

    #[test]
    fn test_parse_skyrimse_codepage() {
        let content = r#"#codepage correspondance
english=utf8,1252
japanese=utf8,932
chinese=utf8
russian=utf8,1251
"#;
        let table = CodepageTable::parse(content);
        assert_eq!(table.len(), 4);

        let en = table.get("english").unwrap();
        assert_eq!(en.primary, CodepageId::Utf8);
        assert_eq!(en.fallback, Some(CodepageId::Cp1252));

        let ja = table.get("japanese").unwrap();
        assert_eq!(ja.primary, CodepageId::Utf8);
        assert_eq!(ja.fallback, Some(CodepageId::Cp932));

        let cn = table.get("chinese").unwrap();
        assert_eq!(cn.primary, CodepageId::Utf8);
        assert_eq!(cn.fallback, None);

        let ru = table.get("russian").unwrap();
        assert_eq!(ru.primary, CodepageId::Utf8);
        assert_eq!(ru.fallback, Some(CodepageId::Cp1251));
    }

    #[test]
    fn test_decode_utf8_success() {
        let config = CodepageConfig::utf8_with_fallback(CodepageId::Cp1252);
        let bytes = "Hello World".as_bytes();
        assert_eq!(config.decode(bytes), "Hello World");
    }

    #[test]
    fn test_decode_utf8_fallback_to_cp1252() {
        let config = CodepageConfig::utf8_with_fallback(CodepageId::Cp1252);
        // 0x80 在 Windows-1252 中是 €，但不是合法 UTF-8
        let bytes: Vec<u8> = vec![0x80];
        let result = config.decode(&bytes);
        assert_eq!(result, "€");
    }

    #[test]
    fn test_decode_utf8_fallback_to_shift_jis() {
        let config = CodepageConfig::utf8_with_fallback(CodepageId::Cp932);
        // Shift-JIS 编码的 "あ" = 0x82 0xA0
        let bytes: Vec<u8> = vec![0x82, 0xA0];
        let result = config.decode(&bytes);
        assert_eq!(result, "あ");
    }

    #[test]
    fn test_decode_utf8_fallback_to_gbk() {
        let config = CodepageConfig::utf8_with_fallback(CodepageId::Cp936);
        // GBK 编码的 "中文" = 0xD6 0xD0 0xCE 0xC4
        let bytes: Vec<u8> = vec![0xD6, 0xD0, 0xCE, 0xC4];
        let result = config.decode(&bytes);
        assert_eq!(result, "中文");
    }

    #[test]
    fn test_decode_no_fallback_byte_by_byte() {
        let config = CodepageConfig::utf8();
        // 0x80 不是合法 UTF-8，无 fallback 则逐字节
        let bytes: Vec<u8> = vec![0x80];
        let result = config.decode(&bytes);
        assert_eq!(result, "\u{0080}");
    }

    #[test]
    fn test_decode_non_utf8_primary() {
        // Skyrim (原版) english=1252，主编码不是 UTF-8
        let config = CodepageConfig {
            primary: CodepageId::Cp1252,
            fallback: None,
        };
        let bytes: Vec<u8> = vec![0x80]; // € in CP1252
        let result = config.decode(&bytes);
        assert_eq!(result, "€");
    }

    #[test]
    fn test_extract_language_from_filename() {
        assert_eq!(
            extract_language_from_filename("skyrim_english.STRINGS"),
            "english"
        );
        assert_eq!(
            extract_language_from_filename("skyrim_japanese.DLSTRINGS"),
            "japanese"
        );
        assert_eq!(
            extract_language_from_filename("C:\\Data\\skyrim_chinese.ILSTRINGS"),
            "chinese"
        );
        assert_eq!(extract_language_from_filename("nolang.STRINGS"), "");
    }

    #[test]
    fn test_get_for_filename() {
        let content = "english=utf8,1252\njapanese=utf8,932\n";
        let table = CodepageTable::parse(content);

        let en_config = table.get_for_filename("skyrim_english.STRINGS");
        assert_eq!(en_config.primary, CodepageId::Utf8);
        assert_eq!(en_config.fallback, Some(CodepageId::Cp1252));

        let ja_config = table.get_for_filename("skyrim_japanese.DLSTRINGS");
        assert_eq!(ja_config.primary, CodepageId::Utf8);
        assert_eq!(ja_config.fallback, Some(CodepageId::Cp932));

        let unknown = table.get_for_filename("skyrim_klingon.STRINGS");
        assert_eq!(unknown.primary, CodepageId::Utf8);
        assert_eq!(unknown.fallback, None);
    }

    #[test]
    fn test_encode_utf8() {
        let config = CodepageConfig::utf8();
        let result = config.encode("Hello");
        assert_eq!(result, b"Hello");
    }

    #[test]
    fn test_encode_decode_roundtrip_cp1252() {
        let config = CodepageConfig {
            primary: CodepageId::Cp1252,
            fallback: None,
        };
        // CP1252 特有字符
        let original = "café €";
        let encoded = config.encode(original);
        let decoded = config.decode(&encoded);
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_encode_decode_roundtrip_shift_jis() {
        let config = CodepageConfig {
            primary: CodepageId::Cp932,
            fallback: None,
        };
        let original = "こんにちは";
        let encoded = config.encode(original);
        let decoded = config.decode(&encoded);
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_parse_fallout4_codepage() {
        let content = r#"#codepage correspondance
en=1252
fr=utf8
cn=utf8
ja=utf8
"#;
        let table = CodepageTable::parse(content);
        let en = table.get("en").unwrap();
        assert_eq!(en.primary, CodepageId::Cp1252);
        assert_eq!(en.fallback, None);

        let fr = table.get("fr").unwrap();
        assert_eq!(fr.primary, CodepageId::Utf8);
    }
}
