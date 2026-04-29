//! 中文繁简转换 (Traditional Chinese ↔ Simplified Chinese)
//!
//! 基于 OpenCC 项目的 STCharacters.txt + TSCharacters.txt 合并构建，
//! 包含 3960 对一致性验证过的双向单字符映射。通过 `include_str!` 编译期嵌入。
//! 与 Delphi `doConvertTCSC` 功能对应，但字典覆盖更完整且保证双向一致性。

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

/// 从嵌入的 OpenCC 字典解析字符对。
///
/// 文件格式（与 Delphi `Charset_SCTC.txt` 兼容）：
/// - Line 0: `#SC:` 标题行
/// - Line 1: 简体中文序列
/// - Line 2: `#TC:` 标题行
/// - Line 3: 繁体中文序列
/// - 两个序列长度相同，位置一一对应
fn load_pairs() -> Vec<(char, char)> {
    let data = include_str!("../../../Misc/opencc_dict.txt");
    let lines: Vec<&str> = data.lines().collect();
    if lines.len() < 4 {
        return Vec::new();
    }
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
        assert!(pairs.len() > 3900, "Expected >3900 pairs, got {}", pairs.len());
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
