//! 中文繁简转换 (Traditional Chinese ↔ Simplified Chinese)
//!
//! 基于 Delphi 原版 `Charset_SCTC.txt` 字典，包含 2552 对字符映射。
//! 字典在编译期通过 `include_str!` 嵌入，零运行时 I/O。
//! 与 Delphi `doConvertTCSC` / `buildTCSCList` 逻辑一致。

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

/// 从嵌入的 Charset_SCTC.txt 解析字符对。
///
/// 文件格式（与 Delphi 原版一致）：
/// - Line 0: `#SC:` 标题行（忽略）
/// - Line 1: 简体中文序列
/// - Line 2: `#TC:` 标题行（忽略）
/// - Line 3: 繁体中文序列
/// - 两个序列长度相同，位置一一对应
fn load_pairs() -> Vec<(char, char)> {
    let data = include_str!("../../../Misc/Charset_SCTC.txt");
    let lines: Vec<&str> = data.lines().collect();
    if lines.len() < 4 {
        return Vec::new();
    }
    // Line 0: #SC: header, Line 1: Simplified chars, Line 2: #TC: header, Line 3: Traditional chars
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
        assert!(pairs.len() > 2500, "Expected >2500 pairs, got {}", pairs.len());
    }

    #[test]
    fn test_to_simplified() {
        assert_eq!(to_simplified("門"), "门");
        assert_eq!(to_simplified("國"), "国");
        assert_eq!(to_simplified("學習"), "学习");
    }

    #[test]
    fn test_to_traditional() {
        assert_eq!(to_traditional("门"), "門");
        assert_eq!(to_traditional("国"), "國");
        assert_eq!(to_traditional("学习"), "學習");
    }

    #[test]
    fn test_roundtrip_sc() {
        let inputs = ["学习", "中国", "见面", "时间", "争议", "图书馆"];
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
        // Delphi 原版验证：这些繁体→简体映射必须正确
        assert_eq!(to_simplified("萬"), "万");
        assert_eq!(to_simplified("體"), "体");
        assert_eq!(to_simplified("點"), "点");
        assert_eq!(to_traditional("万"), "萬");
        assert_eq!(to_traditional("体"), "體");
    }
}
