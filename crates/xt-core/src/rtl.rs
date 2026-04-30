//! RTL (Right-to-Left) text processing for Arabic/Hebrew translations.
//!
//! Ported from Delphi `TESVT_TranslateFunc.pas`:
//! - `IsArabicLetter` → `is_arabic_char`
//! - `MirrorSymbol` → `mirror_symbol`
//! - `splitBlock` → `split_blocks`
//! - `ReverseRTLStringEx` → `reverse_rtl`

/// Check if a Unicode code point is in the Arabic script range.
///
/// Covers: Arabic (0600-06FF), Arabic Supplement (0750-077F),
/// Arabic Presentation Forms-A (FB50-FDFF), Arabic Presentation Forms-B (FE70-FEFF).
pub fn is_arabic_char(ch: char) -> bool {
    let cp = ch as u32;
    (0x0600..=0x06FF).contains(&cp)
        || (0x0750..=0x077F).contains(&cp)
        || (0xFB50..=0xFDFF).contains(&cp)
        || (0xFE70..=0xFEFF).contains(&cp)
}

/// Mirror bracket-like symbols for RTL display.
///
/// Note: `<` and `>` are intentionally NOT mirrored (Bethesda tags).
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

/// Block type classification for RTL segmentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockType {
    Arabic,
    Other,
}

fn classify_char(ch: char, prev: Option<char>, _next: Option<char>) -> BlockType {
    if ch.is_whitespace() {
        // Whitespace after Arabic text is grouped with the Arabic block,
        // so the trailing space moves correctly when blocks are reversed.
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

/// Split text into contiguous blocks of Arabic vs non-Arabic characters.
fn split_blocks(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return Vec::new();
    }

    let mut blocks = Vec::new();
    let mut current = String::new();
    let mut current_type = classify_char(
        chars[0],
        None,
        chars.get(1).copied(),
    );
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

/// Reverse RTL text for proper display.
///
/// Algorithm (matching Delphi `ReverseRTLStringEx`):
/// 1. Split text into Arabic/non-Arabic blocks
/// 2. Iterate blocks in reverse order
/// 3. Arabic blocks: reverse character order
/// 4. Non-Arabic blocks: mirror bracket symbols
///
/// Returns `None` if no Arabic characters were found (pass-through).
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
            // Reverse Arabic block character by character
            for ch in block.chars().rev() {
                result.push(ch);
            }
        } else {
            // Mirror symbols in non-Arabic blocks
            for ch in block.chars() {
                result.push(mirror_symbol(ch));
            }
        }
    }

    if has_arabic { Some(result) } else { None }
}

/// Process a multi-line RTL string.
///
/// Each line is processed independently through `reverse_rtl`.
/// Returns `None` if no Arabic characters were found in any line.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_arabic_char() {
        assert!(is_arabic_char('\u{0627}')); // Arabic Alef
        assert!(is_arabic_char('\u{0644}')); // Arabic Lam
        assert!(!is_arabic_char('A'));
        assert!(!is_arabic_char('中'));
    }

    #[test]
    fn test_mirror_symbol() {
        assert_eq!(mirror_symbol('('), ')');
        assert_eq!(mirror_symbol(')'), '(');
        assert_eq!(mirror_symbol('{'), '}');
        assert_eq!(mirror_symbol('['), ']');
        assert_eq!(mirror_symbol('A'), 'A'); // no change
        // < and > are NOT mirrored (Bethesda tags)
        assert_eq!(mirror_symbol('<'), '<');
        assert_eq!(mirror_symbol('>'), '>');
    }

    #[test]
    fn test_split_blocks_mixed() {
        let blocks = split_blocks("Hello مرحبا World");
        assert!(blocks.len() >= 3);
        // Should have: "Hello " , "مرحبا" , " World"
    }

    #[test]
    fn test_reverse_rtl_arabic_only() {
        let text = "مرحبا"; // "Hello" in Arabic
        let result = reverse_rtl(text);
        assert!(result.is_some());
        // Should reverse the characters
        let reversed = result.unwrap();
        assert_ne!(reversed, text);
        // Reversing twice should give back the original
        let double = reverse_rtl(&reversed).unwrap();
        assert_eq!(double, text);
    }

    #[test]
    fn test_reverse_rtl_no_arabic() {
        let text = "Hello World";
        let result = reverse_rtl(text);
        assert!(result.is_none()); // No Arabic, pass through
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
        // Brackets should be mirrored and Arabic reversed
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
}
