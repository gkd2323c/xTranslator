//! 词汇表加载器 —— 从游戏 Strings 文件构建源语言→翻译配对。
//!
//! 解析 `vocabulary.txt` 以获取 Strings 文件基本名称的列表，
//! 然后加载源语言和目标语言文件，通过 str_id 匹配它们，
//! 并生成用于启发式搜索和自动翻译建议的 `(source, translation)` 对。
//!
//! 这镜像了 Delphi xTranslator 的 "vocabulary" 功能：
//! `vocabulary.txt` 文件列出了 `STRINGS=Name` 条目，工具会加载
//! 源语言和目标语言的 `Name_<lang>.strings` + `Name_<lang>.dlstrings` + `Name_<lang>.ilstrings`
//! 来构建翻译语料库。

use std::path::Path;

use crate::strings::{CodepageTable, StringsFile};

/// 解析 `vocabulary.txt` 文件并返回 STRINGS 基本名称的列表。
///
/// 格式：在去除注释（以 `#` 开头的行）和空格后，以 `STRINGS=` 开头的行（区分大小写）。
pub fn parse_vocabulary_file(path: &Path) -> Result<Vec<String>, std::io::Error> {
    let content = std::fs::read_to_string(path)?;
    Ok(parse_vocabulary_content(&content))
}

/// 从字符串解析词汇表内容。
fn parse_vocabulary_content(content: &str) -> Vec<String> {
    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with('#') || trimmed.is_empty() {
                return None;
            }
            trimmed
                .strip_prefix("STRINGS=")
                .map(|s| s.trim().to_string())
        })
        .collect()
}

/// 词汇表语料库：从游戏 Strings 文件中提取的源语言→翻译对。
#[derive(Debug, Clone, Default)]
pub struct Vocabulary {
    /// 源语言→翻译对，为了去重而以 (base_name, str_id) 作为键。
    pairs: Vec<(String, String)>,
}

impl Vocabulary {
    /// 通过加载源和目标 Strings 文件来构建词汇表。
    ///
    /// - `names`：来自 vocabulary.txt 的基本名称列表（例如 "Skyrim"、"Update"）
    /// - `strings_dir`：包含 Strings 文件的目录
    /// - `source_lang`：源语言（例如 "english"）
    /// - `target_lang`：目标语言（例如 "chinese"）
    /// - `codepage`：可选的用于解码的 codepage 表
    pub fn load(
        names: &[String],
        strings_dir: &Path,
        source_lang: &str,
        target_lang: &str,
        codepage: Option<&CodepageTable>,
    ) -> Self {
        let mut vocab = Self::default();
        for name in names {
            vocab.add_base_name(name, strings_dir, source_lang, target_lang, codepage);
        }
        vocab
    }

    /// 添加一个基本名称的源→目标对。
    fn add_base_name(
        &mut self,
        base_name: &str,
        strings_dir: &Path,
        source_lang: &str,
        target_lang: &str,
        codepage: Option<&CodepageTable>,
    ) {
        for ext in &["strings", "dlstrings", "ilstrings"] {
            let source_path = strings_dir.join(format!("{}_{}.{}", base_name, source_lang, ext));
            let target_path = strings_dir.join(format!("{}_{}.{}", base_name, target_lang, ext));

            if !source_path.exists() || !target_path.exists() {
                continue;
            }

            let source_file = match Self::load_strings_file(&source_path, codepage) {
                Ok(f) => f,
                Err(_) => continue,
            };
            let target_file = match Self::load_strings_file(&target_path, codepage) {
                Ok(f) => f,
                Err(_) => continue,
            };

            // 通过 str_id 匹配：源文本 → 目标文本
            for (&id, source_text) in &source_file.strings {
                if source_text.is_empty() {
                    continue;
                }
                if let Some(target_text) = target_file.strings.get(&id) {
                    if !target_text.is_empty() {
                        self.pairs.push((source_text.clone(), target_text.clone()));
                    }
                }
            }
        }
    }

    fn load_strings_file(
        path: &Path,
        codepage: Option<&CodepageTable>,
    ) -> Result<StringsFile, anyhow::Error> {
        let result = match codepage {
            Some(table) => StringsFile::load_with_codepage_table(path, table),
            None => StringsFile::load(path),
        };
        result.map_err(|e| anyhow::anyhow!("Failed to load {}: {}", path.display(), e))
    }

    /// 以切片形式返回源语言→翻译对。
    pub fn pairs(&self) -> &[(String, String)] {
        &self.pairs
    }

    /// 返回配对的数量。
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    /// 词汇表是否为空。
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_vocabulary_content() {
        let content = "\
#strings 对列表
#注释行
STRINGS=Update
STRINGS=Dawnguard
STRINGS=Skyrim

STRINGS=hearthfires
";
        let names = parse_vocabulary_content(content);
        assert_eq!(names, vec!["Update", "Dawnguard", "Skyrim", "hearthfires"]);
    }

    #[test]
    fn test_parse_vocabulary_ignores_comments() {
        let content = "\
# 这是一个注释
STRINGS=Skyrim
# 另一个注释
STRINGS=Update
";
        let names = parse_vocabulary_content(content);
        assert_eq!(names, vec!["Skyrim", "Update"]);
    }

    #[test]
    fn test_parse_vocabulary_empty() {
        let content = "\
# 仅注释
# 没有有用的内容
";
        let names = parse_vocabulary_content(content);
        assert!(names.is_empty());
    }

    #[test]
    fn test_vocabulary_default_empty() {
        let vocab = Vocabulary::default();
        assert!(vocab.is_empty());
        assert_eq!(vocab.len(), 0);
    }
}
