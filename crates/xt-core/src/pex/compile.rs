//! PEX 二进制编译器 — 将翻译后的字符串写回 PEX 文件
//!
//! 此模块重建具有更新字符串表的 PEX 文件，保留
//! 所有原始结构，同时将字符串文本替换为翻译。
//!
//! 策略：解析会保留所有原始字节（调试信息、用户标志、对象体）。
//! 编译仅在原地修改字符串表条目，保持索引稳定，
//! 以便对象体中的操作码引用保持有效。这与
//! Delphi xTranslator 使用的方法相同。

use super::types::{PexScript, PexStringEntry, PexTranslatableString};
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
///
/// 关键保证：索引绝不会改变，只会改变每个索引对应的文本。
/// 这确保了对象体中的所有操作码引用保持有效。
pub fn build_string_table(
    original: &[PexStringEntry],
    translations: &[PexTranslatableString],
) -> (Vec<PexStringEntry>, HashMap<String, u16>, usize) {
    // 克隆表以便我们可以原地修改
    let mut table: Vec<PexStringEntry> = original.to_vec();

    // 构建原始文本到索引的映射
    let mut text_to_index: HashMap<String, u16> = HashMap::new();
    for entry in &table {
        text_to_index.insert(entry.text.clone(), entry.index);
    }

    // 原地应用翻译（索引绝不改变）
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

    // 重建映射以供调用者参考
    let mut new_map = HashMap::new();
    for entry in &table {
        new_map.insert(entry.text.clone(), entry.index);
    }

    (table, new_map, updated_count)
}

/// 写入带有更新字符串的 PEX 文件
///
/// 除了字符串表文本之外，保留所有原始二进制数据：
/// - Magic, Header: 逐字保留
/// - 字符串表：现有索引处更新的文本
/// - 调试信息：逐字保留自原始文件
/// - 用户标志：逐字保留自原始文件
/// - 对象体：逐字保留自原始文件（索引不变）
pub fn compile_pex(
    original_script: &PexScript,
    translations: &[PexTranslatableString],
    output_path: &str,
) -> io::Result<CompileResult> {
    let mut warnings = Vec::new();

    // 重建已更新的字符串表（保留索引）
    let (new_string_table, _, updated_count) =
        build_string_table(&original_script.string_table, translations);

    // 如果翻译引用了表中未找到的字符串，则发出警告
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

    // Magic
    buffer.write_all(&0xFA57C0DEu32.to_le_bytes())?;

    // Header
    buffer.write_all(&[
        original_script.header.major_version,
        original_script.header.minor_version,
    ])?;
    buffer.write_all(&original_script.header.game_id.to_le_bytes())?;
    buffer.write_all(&original_script.header.compile_time.to_le_bytes())?;

    // 字符串表（更新的文本，相同的索引）
    buffer.write_all(&(new_string_table.len() as u16).to_le_bytes())?;
    for entry in &new_string_table {
        let text_bytes = entry.text.as_bytes();
        buffer.write_all(&(text_bytes.len() as u16).to_le_bytes())?;
        buffer.write_all(text_bytes)?;
    }

    // 调试信息 — 逐字保留自原始文件
    buffer.write_all(&original_script.debug_info_raw)?;

    // 用户标志 — 逐字保留自原始文件
    buffer.write_all(&original_script.user_flags_raw)?;

    // 对象体 — 逐字保留自原始文件（相同的数量，相同的大小）
    buffer.write_all(&(original_script.object_bodies_raw.len() as u16).to_le_bytes())?;
    for body in original_script.object_bodies_raw.iter() {
        // 对象名称索引（从原始对象的偏移量 0 处读取）
        if body.len() >= 2 {
            let name_idx = u16::from_le_bytes([body[0], body[1]]);
            buffer.write_all(&name_idx.to_le_bytes())?;
        } else {
            buffer.write_all(&0u16.to_le_bytes())?;
        }
        // 对象体大小
        buffer.write_all(&(body.len() as u32).to_le_bytes())?;
        // 对象体数据逐字保留
        buffer.write_all(body)?;
    }

    // 写入文件
    let data = buffer.into_inner();
    std::fs::write(output_path, &data)?;

    Ok(CompileResult {
        path: output_path.to_string(),
        updated_count,
        warnings,
    })
}

/// 便捷函数：编译单个 PEX 文件
///
/// 打开文件，解析，应用翻译并写入结果。
pub fn compile_pex_file(
    input_path: &str,
    output_path: &str,
    translations: &[PexTranslatableString],
) -> io::Result<CompileResult> {
    let mut file = std::fs::File::open(input_path)?;
    let script = super::parser::parse_pex(&mut file)?;
    compile_pex(&script, translations, output_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn build_test_pex_bytes(
        strings: &[(&str, u16)],
        object_count: u16,
        body_data: &[u8],
    ) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&0xFA57C0DEu32.to_le_bytes());
        data.push(3); // major
        data.push(10); // minor
        data.extend_from_slice(&1u16.to_le_bytes()); // game_id
        data.extend_from_slice(&0u64.to_le_bytes()); // compile_time

        // 字符串表
        data.extend_from_slice(&(strings.len() as u16).to_le_bytes());
        for (text, _) in strings {
            let bs = text.as_bytes();
            data.extend_from_slice(&(bs.len() as u16).to_le_bytes());
            data.extend_from_slice(bs);
        }

        // 调试信息（空）
        data.extend_from_slice(&0u64.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());

        // 用户标志（空）
        data.extend_from_slice(&0u16.to_le_bytes());

        // 对象
        data.extend_from_slice(&object_count.to_le_bytes());
        if object_count > 0 {
            // 最小的有效空对象体需要 14 字节：
            // parent(2) + doc_idx(2) + uf_count(2) + auto_state(2)
            // + var_count(2) + guard_count(2) + pg_count(2) + state_count(2)
            let min_body = [0u8; 16];
            let body = if body_data.len() >= 16 {
                body_data
            } else {
                &min_body[..]
            };
            data.extend_from_slice(&0u16.to_le_bytes()); // name_idx
            data.extend_from_slice(&(body.len() as u32).to_le_bytes());
            data.extend_from_slice(body);
        }

        data
    }

    #[test]
    fn test_build_string_table_updates_translations() {
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
                text: "Test".to_string(),
            },
        ];

        let translations = vec![PexTranslatableString {
            object_name: "MyScript".to_string(),
            state_name: String::new(),
            function_name: String::new(),
            string_type: "DebugString".to_string(),
            source_text: "Hello".to_string(),
            translation: "你好".to_string(),
        }];

        let (updated, _, count) = build_string_table(&original, &translations);

        assert_eq!(count, 1);
        assert_eq!(updated[0].text, "你好");
        assert_eq!(updated[1].text, "World");
        assert_eq!(updated[2].text, "Test");
        assert_eq!(updated[0].index, 0); // index preserved
    }

    #[test]
    fn test_build_string_table_preserves_indices() {
        let original = vec![
            PexStringEntry {
                index: 0,
                text: "A".to_string(),
            },
            PexStringEntry {
                index: 5,
                text: "B".to_string(),
            },
        ];

        let (updated, _, _) = build_string_table(&original, &[]);

        assert_eq!(updated[0].index, 0);
        assert_eq!(updated[1].index, 5);
    }

    /// 往返测试：解析 → 编译 → 重新解析，验证字符串表不变
    #[test]
    fn test_compile_preserves_binary_structure() {
        let body = [0u8; 16]; // 最小有效空对象体（最少 14 字节）
        let original_bytes = build_test_pex_bytes(
            &[
                ("TestObject", 0),
                ("English text", 1),
                ("Another string", 2),
            ],
            1,
            &body,
        );

        // 解析
        let mut cur = Cursor::new(&original_bytes[..]);
        let script = super::super::parser::parse_pex(&mut cur).unwrap();
        assert_eq!(script.string_table.len(), 3);
        assert_eq!(script.string_table[1].text, "English text");
        assert_eq!(script.object_bodies_raw.len(), 1);
        assert_eq!(script.object_bodies_raw[0].len(), 16);

        // 应用翻译
        let translations = vec![PexTranslatableString {
            object_name: "TestObject".to_string(),
            state_name: String::new(),
            function_name: String::new(),
            string_type: "DebugString".to_string(),
            source_text: "English text".to_string(),
            translation: "英文文本".to_string(),
        }];

        // 编译到临时文件
        let tmp_path = std::env::temp_dir().join("xt_pex_roundtrip_test.pex");
        compile_pex(&script, &translations, tmp_path.to_str().unwrap()).unwrap();

        let mut reparse_cur = Cursor::new(std::fs::read(&tmp_path).unwrap());
        let reparsed = super::super::parser::parse_pex(&mut reparse_cur).unwrap();

        // 验证：字符串表文本已更新，索引未改变，对象体已保留
        assert_eq!(reparsed.string_table.len(), 3);
        assert_eq!(reparsed.string_table[0].text, "TestObject");
        assert_eq!(reparsed.string_table[1].text, "英文文本");
        assert_eq!(reparsed.string_table[1].index, 1);
        assert_eq!(reparsed.string_table[2].text, "Another string");
        assert_eq!(reparsed.object_bodies_raw.len(), 1);
        assert_eq!(reparsed.object_bodies_raw[0].len(), 16);
        assert_eq!(reparsed.object_bodies_raw[0], &[0u8; 16]);

        let _ = std::fs::remove_file(&tmp_path);
    }
}
