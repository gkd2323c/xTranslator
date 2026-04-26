//! 字符串规范化模块
//!
//! 为启发式搜索和字典匹配提供源字符串的规范化版本。
//! normalization 的目标是将同一语义的不同字符串形式映射到相同表示，
//! 例如："Hello, World!" → "hello world"。
//!
//! 当前实现包括：
//! - Unicode 大小写折叠（to_lowercase）
//! - 非字母数字字符替换为单个空格
//! - 连续空白字符压缩
//! - 首尾空白去除

/// 将字符串规范化为用于搜索/匹配的标准形式。
///
/// 规范化步骤：
/// 1. Unicode 大小写折叠转为小写
/// 2. 将非字母数字字符（包括标点、符号）替换为单个空格
/// 3. 压缩连续空格为一个空格
/// 4. 去除首尾空格
///
/// # 示例
/// ```
/// use xt_core::normalization::normalize;
/// let s = "  Hello,  World!  ";
/// let norm = normalize(s);
/// assert_eq!(norm, "hello world");
/// ```
pub fn normalize(s: &str) -> String {
    let mut result = String::new();
    let mut last_was_space = false;

    for c in s.chars() {
        if c.is_alphanumeric() {
            // 字母数字：直接追加（小写化）
            let lower = if c.is_ascii() {
                // ASCII 快速路径
                let mut c = c as u8;
                if c >= b'A' && c <= b'Z' {
                    c += b'a' - b'A';
                }
                c as char
            } else {
                // Unicode 字符使用 to_lowercase（可能产生多个字符，如德语 ß → ss）
                // 但大多数情况是 1 对 1
                for ch in c.to_lowercase() {
                    result.push(ch);
                }
                last_was_space = false;
                continue;
            };
            result.push(lower);
            last_was_space = false;
        } else if !last_was_space {
            // 非字母数字且上一个不是空格 → 添加单个空格
            result.push(' ');
            last_was_space = true;
        }
        // 如果已经上一个字符是空格，跳过（压缩）
    }

    // Trim 两端空格
    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_basic() {
        assert_eq!(normalize("Hello"), "hello");
        assert_eq!(normalize("HELLO"), "hello");
        assert_eq!(normalize("Hello, World!"), "hello world");
        assert_eq!(normalize("  spaced  out  "), "spaced out");
        assert_eq!(normalize("multiple   spaces"), "multiple spaces");
    }

    #[test]
    fn test_normalize_non_ascii() {
        // 中文等非 ASCII 字母数字应保留原形（不参与大小写转换）
        assert_eq!(normalize("你好世界"), "你好世界");
        assert_eq!(normalize("Привет"), "привет"); // 俄文会转小写
        assert_eq!(normalize("こんにちは"), "こんにちは"); // 日文平假名unchanged
    }

    #[test]
    fn test_normalize_punctuation() {
        assert_eq!(normalize("Hello-World"), "hello world");
        assert_eq!(normalize("Hello_world"), "hello world");
        assert_eq!(normalize("Hello.World"), "hello world");
        assert_eq!(normalize("Hello's"), "hello s"); // 撇号被替换为空格
    }

    #[test]
    fn test_normalize_empty_and_spaces() {
        assert_eq!(normalize(""), "");
        assert_eq!(normalize("   "), "");
        assert_eq!(normalize("  \t\n  "), "");
    }
}
