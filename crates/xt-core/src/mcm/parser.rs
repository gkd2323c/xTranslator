//! MCM 文件解析器
//!
//! 正则：`^(\$.+?)\t+(.+)$`
//! - Group 1 (refID=1)：key（以 $ 开头）
//! - Group 2 (refID=2)：可翻译字符串

use std::io::{self, Read};
use std::path::Path;

use super::types::{McmEncoding, McmEntry, McmFile, XTAG_PREFIX};

/// 检测文件 BOM 并读取内容
fn read_file_with_encoding(path: &Path) -> io::Result<(Vec<u8>, McmEncoding)> {
    let mut file = std::fs::File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    if buffer.len() < 2 {
        return Ok((buffer, McmEncoding::Utf8));
    }

    if buffer[0] == 0xFF && buffer[1] == 0xFE {
        Ok((buffer, McmEncoding::Utf16Le))
    } else if buffer[0] == 0xFE && buffer[1] == 0xFF {
        Ok((buffer, McmEncoding::Utf16Be))
    } else {
        Ok((buffer, McmEncoding::Utf8))
    }
}

/// 将字节解码为字符串（根据编码）
fn decode_bytes(bytes: &[u8], encoding: &McmEncoding) -> String {
    match encoding {
        McmEncoding::Utf16Le => {
            let (decoded, _, _) = encoding_rs::UTF_16LE.decode(bytes);
            decoded.into_owned()
        }
        McmEncoding::Utf16Be => {
            let (decoded, _, _) = encoding_rs::UTF_16BE.decode(bytes);
            decoded.into_owned()
        }
        McmEncoding::Utf8 | McmEncoding::Ansi(_) => {
            String::from_utf8_lossy(bytes).into_owned()
        }
    }
}

/// 计算一行在原始文件中的字节数（无换行符）
fn line_byte_len(line: &str, encoding: &McmEncoding) -> usize {
    match encoding {
        McmEncoding::Utf8 | McmEncoding::Ansi(_) => line.as_bytes().len(),
        McmEncoding::Utf16Le | McmEncoding::Utf16Be => {
            line.encode_utf16().count() * 2
        }
    }
}

/// 原生 UTF-16LE 编码（不含 BOM）
fn encode_utf16le(text: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len() * 2);
    for cu in text.encode_utf16() {
        out.extend_from_slice(&cu.to_le_bytes());
    }
    out
}

/// 原生 UTF-16BE 编码（不含 BOM）
fn encode_utf16be(text: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len() * 2);
    for cu in text.encode_utf16() {
        out.extend_from_slice(&cu.to_be_bytes());
    }
    out
}

/// 编码字符串为字节（用于保存）
pub fn encode_to_bytes(text: &str, encoding: &McmEncoding) -> Vec<u8> {
    match encoding {
        McmEncoding::Utf16Le => {
            let mut bytes = vec![0xFF, 0xFE];
            bytes.extend_from_slice(&encode_utf16le(text));
            bytes
        }
        McmEncoding::Utf16Be => {
            let mut bytes = vec![0xFE, 0xFF];
            bytes.extend_from_slice(&encode_utf16be(text));
            bytes
        }
        McmEncoding::Utf8 => text.as_bytes().to_vec(),
        McmEncoding::Ansi(cp) => {
            log::warn!(
                "MCM file saved with ANSI codepage {} -- falling back to UTF-8. \
                 Some characters may be garbled.",
                cp
            );
            text.as_bytes().to_vec()
        }
    }
}

/// 检测原始文件中的换行符风格："\r\n" 或 "\n"
fn detect_line_ending(raw_bytes: &[u8]) -> String {
    for i in 0..raw_bytes.len().saturating_sub(1) {
        if raw_bytes[i] == b'\r' && i + 1 < raw_bytes.len() && raw_bytes[i + 1] == b'\n' {
            return "\r\n".to_string();
        }
        if raw_bytes[i] == b'\n' {
            return "\n".to_string();
        }
    }
    "\n".to_string()
}

/// 根据编码计算 BOM 字节数
fn bom_len(encoding: &McmEncoding) -> usize {
    match encoding {
        McmEncoding::Utf16Le | McmEncoding::Utf16Be => 2,
        _ => 0,
    }
}

/// 根据编码计算换行符的字节数
fn newline_byte_len(line_ending: &str, encoding: &McmEncoding) -> usize {
    match encoding {
        McmEncoding::Utf8 | McmEncoding::Ansi(_) => line_ending.len(),
        McmEncoding::Utf16Le | McmEncoding::Utf16Be => line_ending.len() * 2,
    }
}

/// 解析 MCM 文件
///
/// `^(\$.+?)\t+(.+)$` — key 为 Group 1，字符串为 Group 2
pub fn parse_mcm_file(path: &str) -> io::Result<McmFile> {
    let path = Path::new(path);

    let (raw_bytes, encoding) = read_file_with_encoding(path)?;

    let line_ending = detect_line_ending(&raw_bytes);
    let nlb = newline_byte_len(&line_ending, &encoding);

    let content = decode_bytes(&raw_bytes, &encoding);
    let content = content.trim_start_matches('\u{FEFF}');

    let boms = bom_len(&encoding);
    let mut entries = Vec::new();
    let mut normalized_lines = Vec::new();
    let mut header_list = Vec::new();
    let mut current_offset = boms;

    for (line_index, line) in content.lines().enumerate() {
        normalized_lines.push(line.to_string());

        if let Some((key, value)) = parse_mcm_line(line) {
            let entry_byte_offset = current_offset;
            header_list.push(key.clone());
            entries.push(McmEntry {
                id: key,
                source: value,
                translation: String::new(),
                line_index,
                byte_offset: entry_byte_offset,
            });
        } else {
            header_list.push(String::new());
        }

        current_offset += line_byte_len(line, &encoding) + nlb;
    }

    Ok(McmFile {
        entries,
        normalized_lines,
        header_list,
        encoding,
        path: path.to_string_lossy().to_string(),
        line_ending,
    })
}

/// 解析单行 MCM 文件
///
/// 返回 None 表示非 MCM 行（如空行、注释）。
/// 返回 Some((key, value)) 表示 MCM 条目。
fn parse_mcm_line(line: &str) -> Option<(String, String)> {
    let line = line.trim();

    if line.is_empty() {
        return None;
    }

    if line.starts_with('#') || line.starts_with("//") {
        return None;
    }

    if let Some(tab_pos) = line.find('\t') {
        let key = line[..tab_pos].trim().to_string();
        let value = line[tab_pos + 1..].trim().to_string();

        if key.starts_with('$') && !value.is_empty() {
            return Some((key, value));
        }
    }

    let parts: Vec<&str> = line.splitn(3, '\t').collect();
    if parts.len() >= 2 {
        let key = parts[0].trim().to_string();
        let value = parts[1].trim().to_string();
        if key.starts_with('$') && !value.is_empty() {
            return Some((key, value));
        }
    }

    None
}

/// 保存 MCM 文件（将翻译填回）
///
/// 将 entries 中的 translation 填入对应的行，然后编码写回。
/// 保留原文件的编码和换行符风格。
pub fn save_mcm_file(path: &str, file: &McmFile) -> io::Result<()> {
    if let McmEncoding::Ansi(cp) = &file.encoding {
        log::warn!(
            "saving MCM file originally in ANSI codepage {} as UTF-8. \
             Re-encoding may cause character loss for non-ASCII text.",
            cp
        );
    }

    let mut lines = file.normalized_lines.clone();

    for entry in &file.entries {
        if entry.line_index >= lines.len() {
            continue;
        }

        let line = &mut lines[entry.line_index];

        if let Some(tab_pos) = line.find('\t') {
            let key_part = &line[..=tab_pos];
            let value_part = &line[tab_pos + 1..];

            if value_part.contains(&entry.source) {
                let new_value = if entry.translation.is_empty() {
                    entry.source.clone()
                } else {
                    entry.translation.clone()
                };
                *line = format!("{}{}", key_part, new_value);
            }
        }
    }

    let final_text = lines.join(&file.line_ending);
    let bytes = encode_to_bytes(&final_text, &file.encoding);
    std::fs::write(path, &bytes)?;

    Ok(())
}

/// 构建归一化文本（用于启发式搜索）
///
/// 将 MCM 条目的原文替换为 {{xt=N}} 占位符，返回归一化后的文本。
#[allow(dead_code)]
pub fn build_normalized_text(entries: &[McmEntry]) -> String {
    let mut lines: Vec<String> = Vec::with_capacity(entries.len() * 2);

    for entry in entries {
        let line = format!("{}\t{}{}}}\t", entry.id, XTAG_PREFIX, entry.line_index);
        lines.push(line);
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rand_simple() -> u32 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u32)
            .unwrap_or(0)
    }

    fn parse_test_string(content: &str, encoding: McmEncoding) -> McmFile {
        let tmp = std::env::temp_dir().join(format!(
            "test_mcm_{:?}_{}_{}.txt",
            encoding,
            std::process::id(),
            rand_simple()
        ));
        let bytes = encode_to_bytes(content, &encoding);
        std::fs::write(&tmp, &bytes).unwrap();

        let mut file = parse_mcm_file(tmp.to_str().unwrap()).unwrap();
        file.path = String::new();
        let _ = std::fs::remove_file(&tmp);
        file
    }

    #[test]
    fn test_encode_utf16le_native() {
        let bytes = encode_utf16le("$sA");
        // $ = U+0024, s = U+0073, A = U+0041
        assert_eq!(bytes, vec![0x24, 0x00, 0x73, 0x00, 0x41, 0x00]);
    }

    #[test]
    fn test_parse_utf16() {
        let content = "$sSetting1\tHello World\n$sSetting2\t你好\n";
        let file = parse_test_string(content, McmEncoding::Utf16Le);

        assert_eq!(file.entries.len(), 2);
        assert_eq!(file.entries[0].id, "$sSetting1");
        assert_eq!(file.entries[0].source, "Hello World");
        assert_eq!(file.entries[1].id, "$sSetting2");
        assert_eq!(file.entries[1].source, "你好");
    }

    #[test]
    fn test_byte_offset_utf16() {
        let content = "$sA\tX\n$sB\tY\n";
        let file = parse_test_string(content, McmEncoding::Utf16Le);
        // BOM = 2 bytes; line 0: "$sA\tX" = 5 chars * 2 = 10 bytes; newline = 2 bytes
        // offset[0] = 2; offset[1] = 2 + 10 + 2 = 14
        assert_eq!(file.entries.len(), 2);
        assert_eq!(file.entries[0].byte_offset, 2);
        assert_eq!(file.entries[1].byte_offset, 14);
    }

    #[test]
    fn test_byte_offset_utf8() {
        let content = "$sA\tX\n$sB\tY\n";
        let file = parse_test_string(content, McmEncoding::Utf8);

        assert_eq!(file.entries.len(), 2);
        assert_eq!(file.entries[0].byte_offset, 0);
        assert_eq!(file.entries[1].byte_offset, 6);
    }

    #[test]
    fn test_line_ending_crlf_preserved() {
        let content = "$sA\tHello\r\n$sB\tWorld\r\n";
        let file = parse_test_string(content, McmEncoding::Utf8);
        assert_eq!(file.line_ending, "\r\n");
        assert_eq!(file.entries.len(), 2);
    }

    #[test]
    fn test_parse_utf8() {
        let content = "$sSetting1\tHello World\n$sSetting2\tBonjour\n";
        let file = parse_test_string(content, McmEncoding::Utf8);

        assert_eq!(file.entries.len(), 2);
        assert_eq!(file.entries[0].source, "Hello World");
    }

    #[test]
    fn test_parse_skips_comments() {
        let content = "# This is a comment\n$sValid\tValue\n// Another comment\n";
        let file = parse_test_string(content, McmEncoding::Utf8);

        assert_eq!(file.entries.len(), 1);
        assert_eq!(file.entries[0].id, "$sValid");
    }

    #[test]
    fn test_parse_empty_lines() {
        let content = "$sFirst\tAlpha\n\n$sSecond\tBeta\n\n";
        let file = parse_test_string(content, McmEncoding::Utf8);

        assert_eq!(file.entries.len(), 2);
        assert_eq!(file.entries[0].id, "$sFirst");
        assert_eq!(file.entries[1].id, "$sSecond");
        assert_eq!(file.header_list.len(), 4);
    }

    #[test]
    fn test_save_and_reload_utf16() {
        let content = "$sGreeting\tHello\n$sFarewell\tGoodbye\n";
        let tmp = std::env::temp_dir().join("test_mcm_save.txt");

        let bytes = encode_to_bytes(content, &McmEncoding::Utf16Le);
        std::fs::write(&tmp, &bytes).unwrap();

        let mut file = parse_mcm_file(tmp.to_str().unwrap()).unwrap();

        for entry in &mut file.entries {
            if entry.source == "Hello" {
                entry.translation = "你好".to_string();
            }
            if entry.source == "Goodbye" {
                entry.translation = "再见".to_string();
            }
        }

        save_mcm_file(tmp.to_str().unwrap(), &file).unwrap();

        let reparsed = parse_mcm_file(tmp.to_str().unwrap()).unwrap();
        assert_eq!(reparsed.entries[0].source, "你好");
        assert_eq!(reparsed.entries[1].source, "再见");

        let _ = std::fs::remove_file(&tmp);
    }
}
