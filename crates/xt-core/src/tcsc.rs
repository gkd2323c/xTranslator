//! 中文繁简转换 (Traditional Chinese ↔ Simplified Chinese)
//!
//! 默认使用 OpenCC 合并词典 (3960 对)，回退到 Delphi 原版 Charset_SCTC.txt (2552 对)。
//! 两者均通过 `include_str!` 编译期嵌入，零运行时 I/O。
//!
//! 回退逻辑：若 OpenCC 词典解析结果为空（文件不存在或格式损坏），自动使用 Delphi 词典。

use std::collections::HashMap;
use std::sync::OnceLock;

fn tc2sc() -> &'static HashMap<char, char> {
    static MAP: OnceLock<HashMap<char, char>> = OnceLock::new();
    MAP.get_or_init(|| {
        let pairs = load_pairs();
        let mut m = HashMap::with_capacity(pairs.len());
        for (tc, sc) in pairs {
            m.insert(tc, sc);
        }
        m
    })
}

fn sc2tc() -> &'static HashMap<char, char> {
    static MAP: OnceLock<HashMap<char, char>> = OnceLock::new();
    MAP.get_or_init(|| {
        let pairs = load_pairs();
        let mut m = HashMap::with_capacity(pairs.len());
        for (tc, sc) in pairs {
            m.entry(sc).or_insert(tc);
        }
        m
    })
}

/// 解析位置对齐格式的字典文件
fn parse_positional(data: &str) -> Vec<(char, char)> {
    let lines: Vec<&str> = data.lines().collect();
    if lines.len() < 4 {
        return Vec::new();
    }
    // Line 0: `#SC:` header, Line 1: SC chars
    // Line 2: `#TC:` header, Line 3: TC chars
    let sc: Vec<char> = lines[1].chars().collect();
    let tc: Vec<char> = lines[3].chars().collect();
    let len = sc.len().min(tc.len());
    // Return (Traditional, Simplified) pairs
    tc[..len]
        .iter()
        .copied()
        .zip(sc[..len].iter().copied())
        .collect()
}

/// 加载字符对：优先 OpenCC，回退 Delphi 原版
fn load_pairs() -> Vec<(char, char)> {
    // Primary: OpenCC merged dictionary (3960 pairs, bidirectional-verified)
    let opencc = include_str!("../../../Misc/opencc_dict.txt");
    let pairs = parse_positional(opencc);
    if !pairs.is_empty() {
        return pairs;
    }
    // Fallback: Delphi original Charset_SCTC.txt (2552 pairs)
    let delphi = include_str!("../../../Misc/Charset_SCTC.txt");
    parse_positional(delphi)
}

/// 繁体 → 简体
pub fn to_simplified(text: &str) -> String {
    let map = tc2sc();
    text.chars().map(|c| map.get(&c).copied().unwrap_or(c)).collect()
}

/// 简体 → 繁体
pub fn to_traditional(text: &str) -> String {
    let map = sc2tc();
    text.chars().map(|c| map.get(&c).copied().unwrap_or(c)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_pairs_coverage() {
        let pairs = load_pairs();
        assert!(pairs.len() > 2500, "Expected >2500 pairs, got {}", pairs.len());
    }

    #[test]
    fn test_to_simplified() {
        assert_eq!(to_simplified("\u{9580}"), "\u{95E8}");   // 門→门
        assert_eq!(to_simplified("\u{570B}"), "\u{56FD}");   // 國→国
        assert_eq!(to_simplified("\u{5B78}\u{7FD2}"), "\u{5B66}\u{4E60}"); // 學習→学习
    }

    #[test]
    fn test_to_traditional() {
        assert_eq!(to_traditional("\u{95E8}"), "\u{9580}");   // 门→門
        assert_eq!(to_traditional("\u{56FD}"), "\u{570B}");   // 国→國
        assert_eq!(to_traditional("\u{5B66}\u{4E60}"), "\u{5B78}\u{7FD2}"); // 学习→學習
    }

    #[test]
    fn test_roundtrip_sc() {
        let inputs = ["\u{5B66}\u{4E60}", "\u{4E2D}\u{56FD}", "\u{6B63}\u{5728}"];
        for input in &inputs {
            assert_eq!(to_simplified(&to_traditional(input)), *input);
        }
    }

    #[test]
    fn test_ascii_unchanged() {
        assert_eq!(to_simplified("Hello World 123"), "Hello World 123");
        assert_eq!(to_traditional("Hello World 123"), "Hello World 123");
    }

    #[test]
    fn test_known_chars() {
        assert_eq!(to_simplified("\u{842C}"), "\u{4E07}"); // 萬→万
        assert_eq!(to_simplified("\u{9AD4}"), "\u{4F53}"); // 體→体
        assert_eq!(to_traditional("\u{4E07}"), "\u{842C}"); // 万→萬
        assert_eq!(to_traditional("\u{4F53}"), "\u{9AD4}"); // 体→體
    }
}
