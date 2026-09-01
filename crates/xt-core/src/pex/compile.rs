//! PEX 二进制编译器 — 将翻译后的字符串写回 PEX 文件
//!
//! 策略：严格对齐 Delphi xTranslator 的 dosavePex 机制：
//! - `header_raw` 逐字节保留 Magic、Header、SourceFileName、UserName、ComputerName；
//! - 字符串表写入更新后的 UTF-8 字符串，严格按照对应的大小端写入长度前缀；
//! - `data_raw` 逐字节保留 DebugInfo、UserFlags 与 ObjectBodies，保证字节级无损。

use super::types::{PexEndian, PexScript, PexStringEntry, PexTranslatableString};
use byteorder::{BigEndian, LittleEndian, WriteBytesExt};
use std::collections::HashMap;
use std::io::{self, Cursor, Write};

/// 编译结果
#[derive(Debug)]
pub struct CompileResult {
    /// 编译后文件的路径
    pub path: String,
    /// 更新的字符串数量
    pub updated_count: usize,
    /// 遇到的警告
    pub warnings: Vec<String>,
}

/// 重建已更新的字符串表并保留原始索引
pub fn build_string_table(
    original: &[PexStringEntry],
    translations: &[PexTranslatableString],
) -> (Vec<PexStringEntry>, HashMap<String, u16>, usize) {
    let mut table: Vec<PexStringEntry> = original.to_vec();

    let mut text_to_index: HashMap<String, u16> = HashMap::new();
    for entry in &table {
        text_to_index.insert(entry.text.clone(), entry.index);
    }

    let mut updated_count = 0;
    for trans in translations {
        if !trans.source_text.is_empty() && !trans.translation.is_empty() {
            if let Some(&original_index) = text_to_index.get(&trans.source_text) {
                if let Some(entry) = table.iter_mut().find(|e| e.index == original_index) {
                    entry.text = trans.translation.clone();
                    updated_count += 1;
                }
            }
        }
    }

    let mut new_map = HashMap::new();
    for entry in &table {
        new_map.insert(entry.text.clone(), entry.index);
    }

    (table, new_map, updated_count)
}

/// 写入带有更新字符串的 PEX 字节流
pub fn compile_pex_bytes(
    original_script: &PexScript,
    translations: &[PexTranslatableString],
) -> io::Result<(Vec<u8>, usize, Vec<String>)> {
    let mut warnings = Vec::new();

    let (new_string_table, _, updated_count) =
        build_string_table(&original_script.string_table, translations);

    let mut found_indices = HashMap::new();
    for entry in &original_script.string_table {
        found_indices.insert(entry.text.clone(), entry.index);
    }
    for trans in translations {
        if !trans.source_text.is_empty()
            && !trans.translation.is_empty()
            && !found_indices.contains_key(&trans.source_text)
        {
            warnings.push(format!(
                "Translation source '{}' not found in string table (object: {}, function: {})",
                trans.source_text, trans.object_name, trans.function_name
            ));
        }
    }

    let mut buffer = Cursor::new(Vec::new());

    // 1. 写入 Header Raw
    buffer.write_all(&original_script.header_raw)?;

    // 2. 写入 String Table
    let endian = original_script.header.endian;
    for entry in &new_string_table {
        let text_bytes = entry.text.as_bytes();
        let len = text_bytes.len() as u16;
        match endian {
            PexEndian::LittleEndian => buffer.write_u16::<LittleEndian>(len)?,
            PexEndian::BigEndian => buffer.write_u16::<BigEndian>(len)?,
        }
        buffer.write_all(text_bytes)?;
    }

    // 3. 写入 Data Raw（DebugInfo + UserFlags + Objects）
    buffer.write_all(&original_script.data_raw)?;

    Ok((buffer.into_inner(), updated_count, warnings))
}

pub fn compile_pex(
    original_script: &PexScript,
    translations: &[PexTranslatableString],
    output_path: &str,
) -> io::Result<CompileResult> {
    let (bytes, updated_count, warnings) = compile_pex_bytes(original_script, translations)?;
    std::fs::write(output_path, bytes)?;

    Ok(CompileResult {
        path: output_path.to_string(),
        updated_count,
        warnings,
    })
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_build_string_table_preserves_indices() {
        let original = vec![
            PexStringEntry {
                index: 0,
                text: "Hello".to_string(),
            },
            PexStringEntry {
                index: 1,
                text: "World".to_string(),
            },
            PexStringEntry {
                index: 2,
                text: "Unchanged".to_string(),
            },
        ];

        let translations = vec![PexTranslatableString {
            object_name: "TestObj".to_string(),
            state_name: String::new(),
            function_name: "TestFunc".to_string(),
            string_type: "StringLiteral".to_string(),
            source_text: "Hello".to_string(),
            translation: "你好".to_string(),
        }];

        let (new_table, _, updated) = build_string_table(&original, &translations);
        assert_eq!(updated, 1);
        assert_eq!(new_table[0].index, 0);
        assert_eq!(new_table[0].text, "你好");
        assert_eq!(new_table[1].index, 1);
        assert_eq!(new_table[1].text, "World");
        assert_eq!(new_table[2].index, 2);
        assert_eq!(new_table[2].text, "Unchanged");
    }
}
