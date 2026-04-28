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

    // BOM 检测
    if buffer[0] == 0xFF && buffer[1] == 0xFE {
        // UTF-16 LE BOM
        Ok((buffer, McmEncoding::Utf16Le))
    } else if buffer[0] == 0xFE && buffer[1] == 0xFF {
        // UTF-16 BE BOM
        Ok((buffer, McmEncoding::Utf16Be))
    } else {
        // 默认 UTF-8（也可能是 ANSI，实际由解析时的编码转换处理）
        Ok((buffer, McmEncoding::Utf8))
    }
}

/// 将字节解码为字符串（根据编码）
fn decode_bytes(bytes: &[u8], encoding: &McmEncoding) -> String {
    match encoding {
        McmEncoding::Utf16Le => {
            // encoding_rs 处理 UTF-16LE 编码（包括 BOM 检测和自动跳过）
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

/// 编码字符串为字节（用于保存）
pub fn encode_to_bytes(text: &str, encoding: &McmEncoding) -> Vec<u8> {
    match encoding {
        McmEncoding::Utf16Le => {
            // encoding_rs 自动加上 BOM
            let (encoded, _, _) = encoding_rs::UTF_16LE.encode(text);
            encoded.into_owned()
        }
        McmEncoding::Utf16Be => {
            let (encoded, _, _) = encoding_rs::UTF_16BE.encode(text);
            encoded.into_owned()
        }
        McmEncoding::Utf8 => text.as_bytes().to_vec(),
        McmEncoding::Ansi(_) => {
            // ANSI 暂用 UTF-8 回退
            text.as_bytes().to_vec()
        }
    }
}

/// 解析 MCM 文件
///
/// `^(\$.+?)\t+(.+)$` — key 为 Group 1，字符串为 Group 2
pub fn parse_mcm_file(path: &str) -> io::Result<McmFile> {
    let path = Path::new(path);

    // 读取文件 + 编码检测
    let (raw_bytes, encoding) = read_file_with_encoding(path)?;

    // 解码为字符串
    let content = decode_bytes(&raw_bytes, &encoding);

    // 按行处理
    let mut entries = Vec::new();
    let mut normalized_lines = Vec::new();
    let mut header_list = Vec::new();
    let mut current_offset = 0usize;

    for (line_index, line) in content.lines().enumerate() {
        normalized_lines.push(line.to_string());
        current_offset += line.len() + 1; // +1 for newline (approximation)

        if let Some((key, value)) = parse_mcm_line(line) {
            header_list.push(key.clone());
            entries.push(McmEntry {
                id: key,
                source: value,
                translation: String::new(),
                line_index,
                byte_offset: current_offset,
            });
        } else {
            // 非 MCM 行（如空行、注释），用空 key 占位
            header_list.push(String::new());
        }
    }

    Ok(McmFile {
        entries,
        normalized_lines,
        header_list,
        encoding,
        path: path.to_string_lossy().to_string(),
    })
}

/// 解析单行 MCM 文件
///
/// 返回 None 表示非 MCM 行（如空行、注释）。
/// 返回 Some((key, value)) 表示 MCM 条目。
fn parse_mcm_line(line: &str) -> Option<(String, String)> {
    let line = line.trim();

    // 跳过空行
    if line.is_empty() {
        return None;
    }

    // 跳过注释行（Delphi 原版逻辑）
    if line.starts_with('#') || line.starts_with("//") {
        return None;
    }

    // 查找 Tab 分隔符
    if let Some(tab_pos) = line.find('\t') {
        let key = line[..tab_pos].trim().to_string();
        let value = line[tab_pos + 1..].trim().to_string();

        // Key 必须是 $ 开头（Delphi 原版要求）
        if key.starts_with('$') && !value.is_empty() {
            return Some((key, value));
        }
    }

    // 备选：多 Tab（如有多个可翻译字段）
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
/// 策略：找到行中 `key<tab>source` 模式，替换 source → translation。
pub fn save_mcm_file(path: &str, file: &McmFile) -> io::Result<()> {
    let mut lines = file.normalized_lines.clone();

    for entry in &file.entries {
        if entry.line_index >= lines.len() {
            continue;
        }

        let line = &mut lines[entry.line_index];

        // 查找 key<tab> 后的原文并替换
        if let Some(tab_pos) = line.find('\t') {
            let key_part = &line[..=tab_pos]; // include tab
            let value_part = &line[tab_pos + 1..];

            // 只替换值部分（如果原文匹配）
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

    // 合并行（保留原始换行符风格）
    let final_text = lines.join("\n");
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
        // 每行格式：key<tab>{{xt=N}}
        // N = entries 中的索引（不是 line_index）
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
        file.path = String::new(); // 抹掉临时路径
        file
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
        assert_eq!(file.header_list.len(), 4); // 2 entries + 2 empty
    }

    #[test]
    fn test_save_and_reload_utf16() {
        let content = "$sGreeting\tHello\n$sFarewell\tGoodbye\n";
        let tmp = std::env::temp_dir().join("test_mcm_save.txt");

        // Write initial content
        let bytes = encode_to_bytes(content, &McmEncoding::Utf16Le);
        std::fs::write(&tmp, &bytes).unwrap();

        // Parse
        let mut file = parse_mcm_file(tmp.to_str().unwrap()).unwrap();

        // Apply translations
        for entry in &mut file.entries {
            if entry.source == "Hello" {
                entry.translation = "你好".to_string();
            }
            if entry.source == "Goodbye" {
                entry.translation = "再见".to_string();
            }
        }

        // Save
        save_mcm_file(tmp.to_str().unwrap(), &file).unwrap();

        // Re-parse and verify
        let reparsed = parse_mcm_file(tmp.to_str().unwrap()).unwrap();
        assert_eq!(reparsed.entries[0].source, "你好");
        assert_eq!(reparsed.entries[1].source, "再见");

        let _ = std::fs::remove_file(&tmp);
    }
}