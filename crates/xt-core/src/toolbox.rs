//! 工具箱 — 7 种文本转换工具
//!
//! 对应 Delphi TESVT_StringsStatus.pas 的 applytoolBox + setStr_* 系列函数。

use std::collections::HashSet;
use std::sync::{LazyLock, RwLock};

/// 用于 TitleCase 的全局例外词集合。
/// 从 config.json 的 `word_exception_list` 字段加载。
/// 不区分大小写的匹配（规范化为小写进行查找）。
static EXCEPTION_WORDS: LazyLock<RwLock<HashSet<String>>> =
    LazyLock::new(|| RwLock::new(HashSet::new()));

/// 从换行符分隔的字符串中加载例外词。
pub fn load_exception_words(words: &str) {
    let mut set = EXCEPTION_WORDS.write().unwrap();
    set.clear();
    for line in words.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            set.insert(trimmed.to_lowercase());
        }
    }
}

/// 检查单词是否在例外列表中（不区分大小写）。
pub fn is_exception_word(word: &str) -> bool {
    EXCEPTION_WORDS.read().unwrap().contains(&word.to_lowercase())
}

/// 获取所有已排序的例外词向量。
pub fn get_exception_words() -> Vec<String> {
    let set = EXCEPTION_WORDS.read().unwrap();
    let mut words: Vec<String> = set.iter().cloned().collect();
    words.sort();
    words
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolType {
    UppercaseAll,
    LowercaseAll,
    UppercaseFirstWord,
    TitleCase,
    FixAlias,
    AddHeader,
    TrimString,
}

impl ToolType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "uppercase_all" => Some(ToolType::UppercaseAll),
            "lowercase_all" => Some(ToolType::LowercaseAll),
            "uppercase_first" => Some(ToolType::UppercaseFirstWord),
            "title_case" => Some(ToolType::TitleCase),
            "fix_alias" => Some(ToolType::FixAlias),
            "add_header" => Some(ToolType::AddHeader),
            "trim" => Some(ToolType::TrimString),
            _ => None,
        }
    }

    pub fn all_names() -> Vec<&'static str> {
        vec![
            "uppercase_all",
            "lowercase_all",
            "uppercase_first",
            "title_case",
            "fix_alias",
            "add_header",
            "trim",
        ]
    }
}

/// 对文本字符串应用工具箱转换。
///
/// `source` — 原始源文本（FixAlias 提取标签所需）
/// `header_text` — 要添加的前缀（仅用于 AddHeader）
pub fn apply_tool(tool: ToolType, text: &str, source: &str, header_text: Option<&str>) -> String {
    match tool {
        ToolType::UppercaseAll => uppercase_all(text),
        ToolType::LowercaseAll => lowercase_all(text),
        ToolType::UppercaseFirstWord => uppercase_first_word(text),
        ToolType::TitleCase => title_case(text),
        ToolType::FixAlias => fix_alias(text, source),
        ToolType::AddHeader => add_header(text, header_text.unwrap_or("")),
        ToolType::TrimString => text.trim().to_string(),
    }
}

// ── Individual tools ──────────────────────────────────────────────

fn uppercase_all(text: &str) -> String {
    split_and_transform(text, |word, _, _| word.to_uppercase())
}

fn lowercase_all(text: &str) -> String {
    split_and_transform(text, |word, _, _| word.to_lowercase())
}

fn uppercase_first_word(text: &str) -> String {
    split_and_transform(text, |word, word_index, _| {
        if word_index == 0 {
            first_char_upper_rest_lower(word)
        } else {
            word.to_lowercase()
        }
    })
}

fn title_case(text: &str) -> String {
    split_and_transform(text, |word, _, _| {
        if is_exception_word(word) {
            word.to_lowercase()
        } else {
            first_char_upper_rest_lower(word)
        }
    })
}

/// 修复 `<Alias=...>` 及类似标签：将源文本中的标签模式复制到翻译中。
///
/// 从源文本中提取所有 `<...>` 序列，然后替换翻译中对应的
/// `<...>` 序列。如果标签数量不一致，则返回未修改的文本。
fn fix_alias(translation: &str, source: &str) -> String {
    let tag_re = regex::Regex::new(r"<[^>]+>").unwrap();
    let source_tags: Vec<&str> = tag_re.find_iter(source).map(|m| m.as_str()).collect();
    let trans_tags: Vec<(usize, usize)> = tag_re
        .find_iter(translation)
        .map(|m| (m.start(), m.end()))
        .collect();

    if source_tags.is_empty() || source_tags.len() != trans_tags.len() {
        return translation.to_string();
    }

    let mut result = String::with_capacity(translation.len());
    let mut last_end = 0;
    for (i, (start, end)) in trans_tags.iter().enumerate() {
        result.push_str(&translation[last_end..*start]);
        result.push_str(source_tags[i]);
        last_end = *end;
    }
    result.push_str(&translation[last_end..]);
    result
}

/// 为文本添加头部前缀。如果头部为空，则返回原文本。
fn add_header(text: &str, header: &str) -> String {
    if header.is_empty() {
        text.to_string()
    } else {
        format!("{} {}", header, text)
    }
}

// ── Helpers ───────────────────────────────────────────────────────

fn first_char_upper_rest_lower(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => {
            let mut result = String::with_capacity(s.len());
            result.extend(first.to_uppercase());
            result.extend(chars.flat_map(|c| c.to_lowercase()));
            result
        }
    }
}

/// 将文本分割为单词（以空格和标点符号分隔），
/// 应用转换函数，然后重新组合。
///
/// `<...>` 标签内的内容保持不变。
/// 转换函数接收：单词、其从 0 开始的索引以及当前是否在 `<>` 标签内。
fn split_and_transform<F>(text: &str, transform: F) -> String
where
    F: Fn(&str, usize, bool) -> String,
{
    let mut result = String::with_capacity(text.len());
    let mut word_count = 0usize;
    let mut buf = String::new();
    let mut in_tag = false;

    for ch in text.chars() {
        match ch {
            '<' => {
                // 在进入标签之前刷新任何挂起的单词
                if !buf.is_empty() && !in_tag {
                    result.push_str(&transform(&buf, word_count, false));
                    word_count += 1;
                    buf.clear();
                }
                in_tag = true;
                buf.push('<');
            }
            '>' => {
                buf.push('>');
                if in_tag {
                    // 标签内容原样通过
                    result.push_str(&buf);
                    buf.clear();
                    in_tag = false;
                }
            }
            _ if in_tag => {
                buf.push(ch);
            }
            _ if is_delimiter(ch) => {
                // 刷新当前单词，然后推送分隔符
                if !buf.is_empty() {
                    result.push_str(&transform(&buf, word_count, false));
                    word_count += 1;
                    buf.clear();
                }
                result.push(ch);
            }
            _ => {
                buf.push(ch);
            }
        }
    }

    // 刷新最后一个单词
    if !buf.is_empty() {
        if in_tag {
            result.push_str(&buf);
        } else {
            result.push_str(&transform(&buf, word_count, false));
        }
    }

    result
}

fn is_delimiter(c: char) -> bool {
    c.is_whitespace()
        || c == ','
        || c == '.'
        || c == '!'
        || c == '?'
        || c == ':'
        || c == ';'
        || c == '-'
        || c == '\''
        || c == '"'
        || c == '('
        || c == ')'
        || c == '['
        || c == ']'
        || c == '{'
        || c == '}'
        || c == '/'
        || c == '\\'
        || c == '\n'
        || c == '\r'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uppercase_all() {
        assert_eq!(
            apply_tool(ToolType::UppercaseAll, "hello world", "", None),
            "HELLO WORLD"
        );
        assert_eq!(
            apply_tool(ToolType::UppercaseAll, "Hello <Alias=test> World", "", None),
            "HELLO <Alias=test> WORLD"
        );
    }

    #[test]
    fn test_lowercase_all() {
        assert_eq!(
            apply_tool(ToolType::LowercaseAll, "HELLO WORLD", "", None),
            "hello world"
        );
        assert_eq!(
            apply_tool(ToolType::LowercaseAll, "HELLO <Alias=Test> WORLD", "", None),
            "hello <Alias=Test> world"
        );
    }

    #[test]
    fn test_uppercase_first_word() {
        assert_eq!(
            apply_tool(ToolType::UppercaseFirstWord, "hello world test", "", None),
            "Hello world test"
        );
    }

    #[test]
    fn test_title_case() {
        assert_eq!(
            apply_tool(ToolType::TitleCase, "hello world", "", None),
            "Hello World"
        );
    }

    #[test]
    fn test_title_case_with_exception_words() {
        load_exception_words("is\na\nthe\n");
        assert_eq!(
            apply_tool(ToolType::TitleCase, "it is a good dog", "", None),
            "It is a Good Dog"
        );
        load_exception_words(""); // Clear
    }

    #[test]
    fn test_exception_words_case_insensitive() {
        load_exception_words("IS\n");
        assert!(is_exception_word("IS"));
        assert!(is_exception_word("is"));
        assert!(is_exception_word("Is"));
        load_exception_words("");
    }

    #[test]
    fn test_get_exception_words_sorted() {
        load_exception_words("zebra\napple\nmango");
        let words = get_exception_words();
        assert_eq!(words, vec!["apple", "mango", "zebra"]);
        load_exception_words("");
    }

    #[test]
    fn test_trim() {
        assert_eq!(
            apply_tool(ToolType::TrimString, "  hello world  ", "", None),
            "hello world"
        );
    }

    #[test]
    fn test_fix_alias() {
        let source = "Eat <Alias=Apple>";
        let trans = "Eat <Alias=香蕉>";
        let fixed = apply_tool(ToolType::FixAlias, trans, source, None);
        assert_eq!(fixed, "Eat <Alias=Apple>");
    }

    #[test]
    fn test_fix_alias_no_tags() {
        let source = "Hello";
        let trans = "Hello";
        assert_eq!(apply_tool(ToolType::FixAlias, trans, source, None), "Hello");
    }

    #[test]
    fn test_fix_alias_count_mismatch() {
        // 如果标签数量不一致，应保持不变
        let source = "X <A> Y <B>";
        let trans = "X <C>";
        assert_eq!(apply_tool(ToolType::FixAlias, trans, source, None), "X <C>");
    }

    #[test]
    fn test_add_header() {
        assert_eq!(
            apply_tool(ToolType::AddHeader, "hello", "", Some("Title: ")),
            "Title:  hello"
        );
    }

    #[test]
    fn test_add_header_empty() {
        assert_eq!(
            apply_tool(ToolType::AddHeader, "hello", "", Some("")),
            "hello"
        );
    }

    #[test]
    fn test_tool_type_from_str() {
        assert_eq!(
            ToolType::from_str("uppercase_all"),
            Some(ToolType::UppercaseAll)
        );
        assert_eq!(ToolType::from_str("nonexistent"), None);
    }
}
