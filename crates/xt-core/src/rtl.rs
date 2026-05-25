//! 针对阿拉伯语/希伯来语翻译的 RTL (从右到左) 文本处理。
//!
//! 移植自 Delphi `TESVT_TranslateFunc.pas`：
//! - `IsArabicLetter` → `is_arabic_char`
//! - `MirrorSymbol` → `mirror_symbol`
//! - `splitBlock` → `split_blocks`
//! - `ReverseRTLStringEx` → `reverse_rtl`

/// 检查 Unicode 码点是否在阿拉伯语字符范围内。
///
/// 覆盖范围：阿拉伯语 (0600-06FF)、阿拉伯语补充 (0750-077F)、
/// 阿拉伯语表达形式-A (FB50-FDFF)、阿拉伯语表达形式-B (FE70-FEFF)。
pub fn is_arabic_char(ch: char) -> bool {
    let cp = ch as u32;
    (0x0600..=0x06FF).contains(&cp)
        || (0x0750..=0x077F).contains(&cp)
        || (0xFB50..=0xFDFF).contains(&cp)
        || (0xFE70..=0xFEFF).contains(&cp)
}

/// 镜像括号类符号以进行 RTL 显示。
///
/// 注意：`<` 和 `>` 有意不进行镜像（Bethesda 标签）。
pub fn mirror_symbol(ch: char) -> char {
    match ch {
        '(' => ')',
        ')' => '(',
        '{' => '}',
        '}' => '{',
        '[' => ']',
        ']' => '[',
        '\u{2039}' => '\u{203A}', // ‹ → ›
        '\u{203A}' => '\u{2039}', // › → ‹
        _ => ch,
    }
}

/// 用于 RTL 分段的块类型分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockType {
    Arabic,
    Other,
}

fn classify_char(ch: char, prev: Option<char>, _next: Option<char>) -> BlockType {
    if ch.is_whitespace() {
        // 阿拉伯语文本后面的空格与阿拉伯语块分在同一组，
        // 这样在反转块时，尾随空格可以正确移动。
        if prev.map_or(false, is_arabic_char) {
            BlockType::Arabic
        } else {
            BlockType::Other
        }
    } else if is_arabic_char(ch) {
        BlockType::Arabic
    } else {
        BlockType::Other
    }
}

/// 将文本拆分为连续的阿拉伯语与非阿拉伯语字符块。
fn split_blocks(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return Vec::new();
    }

    let mut blocks = Vec::new();
    let mut current = String::new();
    let mut current_type = classify_char(chars[0], None, chars.get(1).copied());
    current.push(chars[0]);

    for i in 1..chars.len() {
        let next = chars.get(i + 1).copied();
        let t = classify_char(chars[i], Some(chars[i - 1]), next);
        if t != current_type {
            blocks.push(current);
            current = String::new();
            current_type = t;
        }
        current.push(chars[i]);
    }
    blocks.push(current);
    blocks
}

/// 反转 RTL 文本以进行正确显示。
///
/// 算法（与 Delphi `ReverseRTLStringEx` 匹配）：
/// 1. 将文本拆分为阿拉伯语/非阿拉伯语块
/// 2. 按相反顺序遍历块
/// 3. 阿拉伯语块：反转字符顺序
/// 4. 非阿拉伯语块：镜像括号符号
///
/// 如果未找到阿拉伯语字符，则返回 `None`（直接通过）。
pub fn reverse_rtl(text: &str) -> Option<String> {
    let blocks = split_blocks(text);
    if blocks.is_empty() {
        return None;
    }

    let mut has_arabic = false;
    let mut result = String::new();

    for block in blocks.iter().rev() {
        let first = block.chars().next();
        if first.map_or(false, is_arabic_char) {
            has_arabic = true;
            // 逐个字符反转阿拉伯语块
            for ch in block.chars().rev() {
                result.push(ch);
            }
        } else {
            // 镜像非阿拉伯语块中的符号
            for ch in block.chars() {
                result.push(mirror_symbol(ch));
            }
        }
    }

    if has_arabic {
        Some(result)
    } else {
        None
    }
}

/// 处理多行 RTL 字符串。
///
/// 每行都通过 `reverse_rtl` 进行独立处理。
/// 如果在所有行中都没有找到阿拉伯语字符，则返回 `None`。
pub fn reverse_rtl_multiline(text: &str) -> Option<String> {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut has_arabic = false;
    let mut result_lines = Vec::new();

    for line in &lines {
        if let Some(reversed) = reverse_rtl(line) {
            has_arabic = true;
            result_lines.push(reversed);
        } else {
            result_lines.push(line.to_string());
        }
    }

    if has_arabic {
        Some(result_lines.join("\n"))
    } else {
        None
    }
}

// ── 阿拉伯语整形 (Shape / Deshape) ──────────────────────────────────

/// 阿拉伯语字符在单词中的位置。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArabicPosition {
    Isolated,
    Initial,
    Medial,
    Final,
}

/// 检查阿拉伯语字符是否向左连接（即允许后面的字符附着）。
/// 像 Alef, Dal, Thal, Ra, Zain, Waw 这样的字符不向左连接。
fn connects_left(ch: char) -> bool {
    match ch {
        // 不连接的字符：Alef, Alef Maksura, Dal, Thal, Ra, Zain, Waw, Teh Marbuta (仅 final)
        '\u{0627}' | '\u{0649}' | '\u{062F}' | '\u{0630}' | '\u{0631}' | '\u{0632}'
        | '\u{0648}' | '\u{0629}' => false,
        _ => is_arabic_char(ch),
    }
}

/// 映射：(base_char, position) → 表达形式字符。
/// 如果不需要整形（已经是 isolated 或无映射），则返回 None。
fn shape_char(ch: char, pos: ArabicPosition) -> Option<char> {
    // 映射表列定义: (独立, 词尾, 词首, 词中)
    let shaped = match ch {
        '\u{0621}' => [Some('\u{FE80}'), None, None, None], // Hamza
        '\u{0622}' => [Some('\u{FE81}'), Some('\u{FE82}'), None, None], // Alef Madda
        '\u{0623}' => [Some('\u{FE83}'), Some('\u{FE84}'), None, None], // Alef Hamza Above
        '\u{0624}' => [Some('\u{FE85}'), Some('\u{FE86}'), None, None], // Waw Hamza
        '\u{0625}' => [Some('\u{FE87}'), Some('\u{FE88}'), None, None], // Alef Hamza Below
        '\u{0626}' => [
            Some('\u{FE89}'),
            Some('\u{FE8A}'),
            Some('\u{FE8B}'),
            Some('\u{FE8C}'),
        ], // Ya Hamza
        '\u{0627}' => [Some('\u{FE8D}'), Some('\u{FE8E}'), None, None], // Alef
        '\u{0628}' => [
            Some('\u{FE8F}'),
            Some('\u{FE90}'),
            Some('\u{FE91}'),
            Some('\u{FE92}'),
        ], // Ba
        '\u{0629}' => [Some('\u{FE93}'), Some('\u{FE94}'), None, None], // Teh Marbuta
        '\u{062A}' => [
            Some('\u{FE95}'),
            Some('\u{FE96}'),
            Some('\u{FE97}'),
            Some('\u{FE98}'),
        ], // Ta
        '\u{062B}' => [
            Some('\u{FE99}'),
            Some('\u{FE9A}'),
            Some('\u{FE9B}'),
            Some('\u{FE9C}'),
        ], // Tha
        '\u{062C}' => [
            Some('\u{FE9D}'),
            Some('\u{FE9E}'),
            Some('\u{FE9F}'),
            Some('\u{FEA0}'),
        ], // Jeem
        '\u{062D}' => [
            Some('\u{FEA1}'),
            Some('\u{FEA2}'),
            Some('\u{FEA3}'),
            Some('\u{FEA4}'),
        ], // Ha
        '\u{062E}' => [
            Some('\u{FEA5}'),
            Some('\u{FEA6}'),
            Some('\u{FEA7}'),
            Some('\u{FEA8}'),
        ], // Kha
        '\u{062F}' => [Some('\u{FEA9}'), Some('\u{FEAA}'), None, None], // Dal
        '\u{0630}' => [Some('\u{FEAB}'), Some('\u{FEAC}'), None, None], // Thal
        '\u{0631}' => [Some('\u{FEAD}'), Some('\u{FEAE}'), None, None], // Ra
        '\u{0632}' => [Some('\u{FEAF}'), Some('\u{FEB0}'), None, None], // Zain
        '\u{0633}' => [
            Some('\u{FEB1}'),
            Some('\u{FEB2}'),
            Some('\u{FEB3}'),
            Some('\u{FEB4}'),
        ], // Seen
        '\u{0634}' => [
            Some('\u{FEB5}'),
            Some('\u{FEB6}'),
            Some('\u{FEB7}'),
            Some('\u{FEB8}'),
        ], // Sheen
        '\u{0635}' => [
            Some('\u{FEB9}'),
            Some('\u{FEBA}'),
            Some('\u{FEBB}'),
            Some('\u{FEBC}'),
        ], // Sad
        '\u{0636}' => [
            Some('\u{FEBD}'),
            Some('\u{FEBE}'),
            Some('\u{FEBF}'),
            Some('\u{FEC0}'),
        ], // Dad
        '\u{0637}' => [
            Some('\u{FEC1}'),
            Some('\u{FEC2}'),
            Some('\u{FEC3}'),
            Some('\u{FEC4}'),
        ], // Tah
        '\u{0638}' => [
            Some('\u{FEC5}'),
            Some('\u{FEC6}'),
            Some('\u{FEC7}'),
            Some('\u{FEC8}'),
        ], // Zah
        '\u{0639}' => [
            Some('\u{FEC9}'),
            Some('\u{FECA}'),
            Some('\u{FECB}'),
            Some('\u{FECC}'),
        ], // Ain
        '\u{063A}' => [
            Some('\u{FECD}'),
            Some('\u{FECE}'),
            Some('\u{FECF}'),
            Some('\u{FED0}'),
        ], // Ghain
        '\u{0641}' => [
            Some('\u{FED1}'),
            Some('\u{FED2}'),
            Some('\u{FED3}'),
            Some('\u{FED4}'),
        ], // Fa
        '\u{0642}' => [
            Some('\u{FED5}'),
            Some('\u{FED6}'),
            Some('\u{FED7}'),
            Some('\u{FED8}'),
        ], // Qaf
        '\u{0643}' => [
            Some('\u{FED9}'),
            Some('\u{FEDA}'),
            Some('\u{FEDB}'),
            Some('\u{FEDC}'),
        ], // Kaf
        '\u{0644}' => [
            Some('\u{FEDD}'),
            Some('\u{FEDE}'),
            Some('\u{FEDF}'),
            Some('\u{FEE0}'),
        ], // Lam
        '\u{0645}' => [
            Some('\u{FEE1}'),
            Some('\u{FEE2}'),
            Some('\u{FEE3}'),
            Some('\u{FEE4}'),
        ], // Meem
        '\u{0646}' => [
            Some('\u{FEE5}'),
            Some('\u{FEE6}'),
            Some('\u{FEE7}'),
            Some('\u{FEE8}'),
        ], // Noon
        '\u{0647}' => [
            Some('\u{FEE9}'),
            Some('\u{FEEA}'),
            Some('\u{FEEB}'),
            Some('\u{FEEC}'),
        ], // Ha
        '\u{0648}' => [Some('\u{FEED}'), Some('\u{FEEE}'), None, None], // Waw
        '\u{0649}' => [Some('\u{FEEF}'), Some('\u{FEF0}'), None, None], // Alef Maksura
        '\u{064A}' => [
            Some('\u{FEF1}'),
            Some('\u{FEF2}'),
            Some('\u{FEF3}'),
            Some('\u{FEF4}'),
        ], // Ya
        _ => return None,
    };

    match pos {
        ArabicPosition::Isolated => shaped[0],
        ArabicPosition::Final => shaped[1],
        ArabicPosition::Initial => shaped[2],
        ArabicPosition::Medial => shaped[3],
    }
}

/// 反向查找：查找整形后表达形式的基础字符。
fn deshape_char(ch: char) -> Option<char> {
    // 从所有表达形式构建到基础字符的反向映射
    let base_chars = [
        '\u{0621}', '\u{0622}', '\u{0623}', '\u{0624}', '\u{0625}', '\u{0626}', '\u{0627}',
        '\u{0628}', '\u{0629}', '\u{062A}', '\u{062B}', '\u{062C}', '\u{062D}', '\u{062E}',
        '\u{062F}', '\u{0630}', '\u{0631}', '\u{0632}', '\u{0633}', '\u{0634}', '\u{0635}',
        '\u{0636}', '\u{0637}', '\u{0638}', '\u{0639}', '\u{063A}', '\u{0641}', '\u{0642}',
        '\u{0643}', '\u{0644}', '\u{0645}', '\u{0646}', '\u{0647}', '\u{0648}', '\u{0649}',
        '\u{064A}',
    ];

    for &base in &base_chars {
        for pos in [
            ArabicPosition::Isolated,
            ArabicPosition::Final,
            ArabicPosition::Initial,
            ArabicPosition::Medial,
        ] {
            if let Some(shaped) = shape_char(base, pos) {
                if shaped == ch {
                    return Some(base);
                }
            }
        }
    }
    None
}

/// 整形阿拉伯语文本：将逻辑顺序的阿拉伯语字符转换为表达形式。
///
/// 这会确定每个字符在其单词中的位置 (isolated/initial/medial/final)，
/// 并将其替换为相应的 Unicode 表达形式。
pub fn shape_arabic(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return String::new();
    }

    let mut result = String::with_capacity(text.len());

    for i in 0..chars.len() {
        if !is_arabic_char(chars[i]) {
            result.push(chars[i]);
            continue;
        }

        let prev_connects = i > 0 && {
            let p = chars[i - 1];
            is_arabic_char(p) && connects_left(p)
        };
        let next_connects = i + 1 < chars.len() && is_arabic_char(chars[i + 1]);

        let pos = match (prev_connects, next_connects) {
            (false, false) => ArabicPosition::Isolated,
            (false, true) => ArabicPosition::Initial,
            (true, false) => ArabicPosition::Final,
            (true, true) => ArabicPosition::Medial,
        };

        if let Some(shaped) = shape_char(chars[i], pos) {
            result.push(shaped);
        } else {
            result.push(chars[i]);
        }
    }

    result
}

/// 还原阿拉伯语文本：将表达形式转换回逻辑顺序的基础字符。
///
/// 这会反转由 `shape_arabic` 执行的整形，将表达形式字符 (U+FE70..U+FEFF)
/// 转换回其基础阿拉伯语字符 (U+0621..U+064A)。
pub fn deshape_arabic(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for ch in text.chars() {
        if let Some(base) = deshape_char(ch) {
            result.push(base);
        } else {
            result.push(ch);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_arabic_char() {
        assert!(is_arabic_char('\u{0627}')); // 阿拉伯语 Alef
        assert!(is_arabic_char('\u{0644}')); // 阿拉伯语 Lam
        assert!(!is_arabic_char('A'));
        assert!(!is_arabic_char('中'));
    }

    #[test]
    fn test_mirror_symbol() {
        assert_eq!(mirror_symbol('('), ')');
        assert_eq!(mirror_symbol(')'), '(');
        assert_eq!(mirror_symbol('{'), '}');
        assert_eq!(mirror_symbol('['), ']');
        assert_eq!(mirror_symbol('A'), 'A'); // 无变化
                                             // < 和 > 不进行镜像 (Bethesda 标签)
        assert_eq!(mirror_symbol('<'), '<');
        assert_eq!(mirror_symbol('>'), '>');
    }

    #[test]
    fn test_split_blocks_mixed() {
        let blocks = split_blocks("Hello مرحبا World");
        assert!(blocks.len() >= 3);
        // 应该包含："Hello " , "مرحبا" , " World"
    }

    #[test]
    fn test_reverse_rtl_arabic_only() {
        let text = "مرحبا"; // 阿拉伯语中的 "Hello"
        let result = reverse_rtl(text);
        assert!(result.is_some());
        // 应该反转字符
        let reversed = result.unwrap();
        assert_ne!(reversed, text);
        // 反转两次应该返回原始文本
        let double = reverse_rtl(&reversed).unwrap();
        assert_eq!(double, text);
    }

    #[test]
    fn test_reverse_rtl_no_arabic() {
        let text = "Hello World";
        let result = reverse_rtl(text);
        assert!(result.is_none()); // 无阿拉伯语，直接通过
    }

    #[test]
    fn test_reverse_rtl_mixed() {
        let text = "Say مرحبا to everyone";
        let result = reverse_rtl(text);
        assert!(result.is_some());
    }

    #[test]
    fn test_reverse_rtl_with_brackets() {
        let text = "(مرحبا)";
        let result = reverse_rtl(text).unwrap();
        // 括号应该被镜像，阿拉伯语被反转
        assert!(result.contains('(') || result.contains(')'));
    }

    #[test]
    fn test_reverse_rtl_multiline() {
        let text = "Line 1\nمرحبا\nLine 3";
        let result = reverse_rtl_multiline(text);
        assert!(result.is_some());
        let output = result.unwrap();
        let lines: Vec<&str> = output.split('\n').collect();
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_reverse_rtl_multiline_no_arabic() {
        let text = "Line 1\nLine 2\nLine 3";
        let result = reverse_rtl_multiline(text);
        assert!(result.is_none());
    }

    #[test]
    fn test_shape_isolated() {
        // 单个处于 isolation 的 Alef
        let shaped = shape_arabic("\u{0627}");
        assert_eq!(shaped, "\u{FE8D}");
    }

    #[test]
    fn test_shape_word() {
        // "مرحبا" (Hello) — 一个连接的单词
        let text = "\u{0645}\u{0631}\u{062D}\u{0628}\u{0627}";
        let shaped = shape_arabic(text);
        // Meem 应该是 initial，Ra 是不连接的，所以 Ha 变成 initial，
        // Ba 连接到 Ha，Alef 是 final
        // 只验证它已更改且长度已保留
        assert_eq!(shaped.chars().count(), text.chars().count());
        assert_ne!(shaped, text);
    }

    #[test]
    fn test_deshape_roundtrip() {
        let text = "\u{0645}\u{0631}\u{062D}\u{0628}\u{0627}";
        let shaped = shape_arabic(text);
        let deshaped = deshape_arabic(&shaped);
        assert_eq!(deshaped, text);
    }

    #[test]
    fn test_shape_preserves_non_arabic() {
        let text = "Hello مرحبا World";
        let shaped = shape_arabic(text);
        assert!(shaped.starts_with("Hello "));
        assert!(shaped.ends_with(" World"));
    }

    #[test]
    fn test_deshape_passthrough() {
        // 未整形的文本应保持原样通过
        let text = "Hello World";
        assert_eq!(deshape_arabic(text), text);
    }

    #[test]
    fn test_connects_left() {
        // Alef 不向左连接
        assert!(!connects_left('\u{0627}'));
        // Ba 向左连接
        assert!(connects_left('\u{0628}'));
        // Dal 不向左连接
        assert!(!connects_left('\u{062F}'));
    }
}
