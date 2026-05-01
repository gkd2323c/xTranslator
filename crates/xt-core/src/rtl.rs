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

// ── Arabic Shaping (Shape / Deshape) ──────────────────────────────────

/// Position of an Arabic character in a word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArabicPosition {
    Isolated,
    Initial,
    Medial,
    Final,
}

/// Check if an Arabic character connects to the left (i.e., allows a following character to attach).
/// Characters like Alef, Dal, Thal, Ra, Zain, Waw do NOT connect to the left.
fn connects_left(ch: char) -> bool {
    match ch {
        // Non-connecting: Alef, Alef Maksura, Dal, Thal, Ra, Zain, Waw, Teh Marbuta (final only)
        '\u{0627}' | '\u{0649}' | '\u{062F}' | '\u{0630}' | '\u{0631}' | '\u{0632}'
        | '\u{0648}' | '\u{0629}' => false,
        _ => is_arabic_char(ch),
    }
}

/// Mapping: (base_char, position) → presentation form char.
/// Returns None if no shaping is needed (already isolated or no mapping).
fn shape_char(ch: char, pos: ArabicPosition) -> Option<char> {
    // Table: (isolated, final, initial, medial)
    let shaped = match ch {
        '\u{0621}' => [Some('\u{FE80}'), None, None, None],                         // Hamza
        '\u{0622}' => [Some('\u{FE81}'), Some('\u{FE82}'), None, None],              // Alef Madda
        '\u{0623}' => [Some('\u{FE83}'), Some('\u{FE84}'), None, None],              // Alef Hamza Above
        '\u{0624}' => [Some('\u{FE85}'), Some('\u{FE86}'), None, None],              // Waw Hamza
        '\u{0625}' => [Some('\u{FE87}'), Some('\u{FE88}'), None, None],              // Alef Hamza Below
        '\u{0626}' => [Some('\u{FE89}'), Some('\u{FE8A}'), Some('\u{FE8B}'), Some('\u{FE8C}')], // Ya Hamza
        '\u{0627}' => [Some('\u{FE8D}'), Some('\u{FE8E}'), None, None],              // Alef
        '\u{0628}' => [Some('\u{FE8F}'), Some('\u{FE90}'), Some('\u{FE91}'), Some('\u{FE92}')], // Ba
        '\u{0629}' => [Some('\u{FE93}'), Some('\u{FE94}'), None, None],              // Teh Marbuta
        '\u{062A}' => [Some('\u{FE95}'), Some('\u{FE96}'), Some('\u{FE97}'), Some('\u{FE98}')], // Ta
        '\u{062B}' => [Some('\u{FE99}'), Some('\u{FE9A}'), Some('\u{FE9B}'), Some('\u{FE9C}')], // Tha
        '\u{062C}' => [Some('\u{FE9D}'), Some('\u{FE9E}'), Some('\u{FE9F}'), Some('\u{FEA0}')], // Jeem
        '\u{062D}' => [Some('\u{FEA1}'), Some('\u{FEA2}'), Some('\u{FEA3}'), Some('\u{FEA4}')], // Ha
        '\u{062E}' => [Some('\u{FEA5}'), Some('\u{FEA6}'), Some('\u{FEA7}'), Some('\u{FEA8}')], // Kha
        '\u{062F}' => [Some('\u{FEA9}'), Some('\u{FEAA}'), None, None],              // Dal
        '\u{0630}' => [Some('\u{FEAB}'), Some('\u{FEAC}'), None, None],              // Thal
        '\u{0631}' => [Some('\u{FEAD}'), Some('\u{FEAE}'), None, None],              // Ra
        '\u{0632}' => [Some('\u{FEAF}'), Some('\u{FEB0}'), None, None],              // Zain
        '\u{0633}' => [Some('\u{FEB1}'), Some('\u{FEB2}'), Some('\u{FEB3}'), Some('\u{FEB4}')], // Seen
        '\u{0634}' => [Some('\u{FEB5}'), Some('\u{FEB6}'), Some('\u{FEB7}'), Some('\u{FEB8}')], // Sheen
        '\u{0635}' => [Some('\u{FEB9}'), Some('\u{FEBA}'), Some('\u{FEBB}'), Some('\u{FEBC}')], // Sad
        '\u{0636}' => [Some('\u{FEBD}'), Some('\u{FEBE}'), Some('\u{FEBF}'), Some('\u{FEC0}')], // Dad
        '\u{0637}' => [Some('\u{FEC1}'), Some('\u{FEC2}'), Some('\u{FEC3}'), Some('\u{FEC4}')], // Tah
        '\u{0638}' => [Some('\u{FEC5}'), Some('\u{FEC6}'), Some('\u{FEC7}'), Some('\u{FEC8}')], // Zah
        '\u{0639}' => [Some('\u{FEC9}'), Some('\u{FECA}'), Some('\u{FECB}'), Some('\u{FECC}')], // Ain
        '\u{063A}' => [Some('\u{FECD}'), Some('\u{FECE}'), Some('\u{FECF}'), Some('\u{FED0}')], // Ghain
        '\u{0641}' => [Some('\u{FED1}'), Some('\u{FED2}'), Some('\u{FED3}'), Some('\u{FED4}')], // Fa
        '\u{0642}' => [Some('\u{FED5}'), Some('\u{FED6}'), Some('\u{FED7}'), Some('\u{FED8}')], // Qaf
        '\u{0643}' => [Some('\u{FED9}'), Some('\u{FEDA}'), Some('\u{FEDB}'), Some('\u{FEDC}')], // Kaf
        '\u{0644}' => [Some('\u{FEDD}'), Some('\u{FEDE}'), Some('\u{FEDF}'), Some('\u{FEE0}')], // Lam
        '\u{0645}' => [Some('\u{FEE1}'), Some('\u{FEE2}'), Some('\u{FEE3}'), Some('\u{FEE4}')], // Meem
        '\u{0646}' => [Some('\u{FEE5}'), Some('\u{FEE6}'), Some('\u{FEE7}'), Some('\u{FEE8}')], // Noon
        '\u{0647}' => [Some('\u{FEE9}'), Some('\u{FEEA}'), Some('\u{FEEB}'), Some('\u{FEEC}')], // Ha
        '\u{0648}' => [Some('\u{FEED}'), Some('\u{FEEE}'), None, None],              // Waw
        '\u{0649}' => [Some('\u{FEEF}'), Some('\u{FEF0}'), None, None],              // Alef Maksura
        '\u{064A}' => [Some('\u{FEF1}'), Some('\u{FEF2}'), Some('\u{FEF3}'), Some('\u{FEF4}')], // Ya
        _ => return None,
    };

    match pos {
        ArabicPosition::Isolated => shaped[0],
        ArabicPosition::Final => shaped[1],
        ArabicPosition::Initial => shaped[2],
        ArabicPosition::Medial => shaped[3],
    }
}

/// Reverse lookup: find the base character for a shaped presentation form.
fn deshape_char(ch: char) -> Option<char> {
    // Build reverse map from all presentation forms to base chars
    let base_chars = [
        '\u{0621}', '\u{0622}', '\u{0623}', '\u{0624}', '\u{0625}', '\u{0626}',
        '\u{0627}', '\u{0628}', '\u{0629}', '\u{062A}', '\u{062B}', '\u{062C}',
        '\u{062D}', '\u{062E}', '\u{062F}', '\u{0630}', '\u{0631}', '\u{0632}',
        '\u{0633}', '\u{0634}', '\u{0635}', '\u{0636}', '\u{0637}', '\u{0638}',
        '\u{0639}', '\u{063A}', '\u{0641}', '\u{0642}', '\u{0643}', '\u{0644}',
        '\u{0645}', '\u{0646}', '\u{0647}', '\u{0648}', '\u{0649}', '\u{064A}',
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

/// Shape Arabic text: convert logical-order Arabic characters to presentation forms.
///
/// This determines each character's position in its word (isolated/initial/medial/final)
/// and replaces it with the corresponding Unicode presentation form.
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

/// Deshape Arabic text: convert presentation forms back to logical-order base characters.
///
/// This reverses the shaping done by `shape_arabic`, converting presentation form
/// characters (U+FE70..U+FEFF) back to their base Arabic characters (U+0621..U+064A).
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

    #[test]
    fn test_shape_isolated() {
        // Single Alef in isolation
        let shaped = shape_arabic("\u{0627}");
        assert_eq!(shaped, "\u{FE8D}");
    }

    #[test]
    fn test_shape_word() {
        // "مرحبا" (Hello) — a connected word
        let text = "\u{0645}\u{0631}\u{062D}\u{0628}\u{0627}";
        let shaped = shape_arabic(text);
        // Meem should be initial, Ra is non-connecting so Ha gets initial,
        // Ba connects to Ha, Alef is final
        // Just verify it changed and length is preserved
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
        // Non-shaped text should pass through unchanged
        let text = "Hello World";
        assert_eq!(deshape_arabic(text), text);
    }

    #[test]
    fn test_connects_left() {
        // Alef does NOT connect left
        assert!(!connects_left('\u{0627}'));
        // Ba DOES connect left
        assert!(connects_left('\u{0628}'));
        // Dal does NOT connect left
        assert!(!connects_left('\u{062F}'));
    }
}
