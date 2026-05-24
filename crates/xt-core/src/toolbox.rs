//! 工具箱 — 7 种文本转换工具
//!
//! 对应 Delphi TESVT_StringsStatus.pas 的 applytoolBox + setStr_* 系列函数。

use std::collections::HashSet;
use std::sync::{LazyLock, RwLock};

/// Global exception words set for TitleCase.
/// Loaded from config.json `word_exception_list` field.
/// Case-insensitive matching (normalized to lowercase for lookup).
static EXCEPTION_WORDS: LazyLock<RwLock<HashSet<String>>> =
    LazyLock::new(|| RwLock::new(HashSet::new()));

/// Load exception words from a newline-separated string.
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

/// Check if a word is in the exception list (case-insensitive).
pub fn is_exception_word(word: &str) -> bool {
    EXCEPTION_WORDS.read().unwrap().contains(&word.to_lowercase())
}

/// Get all exception words as a sorted vector.
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

/// Apply a toolbox transformation to a text string.
///
/// `source` — the original source text (needed for FixAlias to extract tags)
/// `header_text` — header to prepend (only for AddHeader)
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

/// Fix `<Alias=...>` and similar tags: copy tag patterns from source into translation.
///
/// Extracts all `<...>` sequences from the source, then replaces corresponding
/// `<...>` sequences in the translation. If tags counts differ, returns unchanged.
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

/// Add a header prefix to text. If header is empty, strips any existing header.
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

/// Split text into words (delimited by whitespace and punctuation),
/// apply a transformation function, and reassemble.
///
/// Content inside `<...>` tags is passed through unchanged.
/// The transform receives: the word, its 0-based index, and whether currently inside `<>` tags.
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
                // Flush any pending word before entering tag
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
                    // Tag content passes through as-is
                    result.push_str(&buf);
                    buf.clear();
                    in_tag = false;
                }
            }
            _ if in_tag => {
                buf.push(ch);
            }
            _ if is_delimiter(ch) => {
                // Flush current word, then push delimiter
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

    // Flush last word
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
        // Should leave unchanged if tag count differs
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
