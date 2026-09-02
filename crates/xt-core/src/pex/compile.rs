//! PEX 二进制编译器 — 将翻译后的字符串写回 PEX 文件
//!
//! 策略：严格对齐 Delphi xTranslator 的 dosavePex 机制：
//! - `header_raw` 逐字节保留 Magic、Header、SourceFileName、UserName、ComputerName 以及 **stringTableCount**；
//! - 字符串表写入更新后的 UTF-8 字符串，严格按照对应的大小端写入长度前缀；
//! - `data_raw` 逐字节保留 DebugInfo、UserFlags 与 ObjectBodies，保证字节级无损。
//!
//! 该策略保证了"无翻译时 parse→compile 后 byte-for-byte 相同"的 roundtrip 性质。

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
///
/// 布局: [header_raw(含 stringTableCount)] [string entries...] [data_raw]
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

    // 1. 写入 Header Raw（含 magic + 头部字段 + source/user/computer + stringTableCount）
    buffer.write_all(&original_script.header_raw)?;

    // 2. 写入 String Table entries（不含 count，count 已在 header_raw 内）
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

    // 3. 写入 Data Raw（hasDebugInfo + DebugInfo + UserFlags + Objects）
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
    use crate::pex::parser::{parse_pex, PEX_MAGIC_BIG};
    use byteorder::{BigEndian, WriteBytesExt};

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

    /// 构造一个最小的真实 Skyrim PEX（Big-Endian，包含完整 header + stringTableCount + 一个空对象）
    fn build_minimal_skyrim_pex() -> Vec<u8> {
        let mut data = Vec::new();

        data.write_u32::<BigEndian>(PEX_MAGIC_BIG).unwrap();
        data.push(3); // major
        data.push(9); // minor
        data.write_u16::<BigEndian>(1).unwrap(); // game_id = 1 (Skyrim)
        data.write_u64::<BigEndian>(12345678).unwrap();

        for s in &["Source/Test.psc", "user", "machine"] {
            data.write_u16::<BigEndian>(s.len() as u16).unwrap();
            data.extend_from_slice(s.as_bytes());
        }

        // string table count = 4
        let strings = ["", "TestScript", "TestFunc", "None"];
        data.write_u16::<BigEndian>(strings.len() as u16).unwrap();
        for s in strings {
            data.write_u16::<BigEndian>(s.len() as u16).unwrap();
            data.extend_from_slice(s.as_bytes());
        }

        // has_debug_info = 0
        data.push(0);
        // user_flags = 0
        data.write_u16::<BigEndian>(0).unwrap();
        // objects = 1
        data.write_u16::<BigEndian>(1).unwrap();
        // object name = 1 ("TestScript")
        data.write_u16::<BigEndian>(1).unwrap();
        // object size = (parent:2 + doc:2 + uFlags:4 + autoState:2 + varCount:2 + propCount:2 + stateCount:2) = 16 + 4 (size自身) = 20
        let body: Vec<u8> = {
            let mut b = Vec::new();
            b.write_u16::<BigEndian>(0).unwrap(); // parentClass
            b.write_u16::<BigEndian>(0).unwrap(); // docString
            b.write_u32::<BigEndian>(0).unwrap(); // userFlags
            b.write_u16::<BigEndian>(0).unwrap(); // autoStateName
            b.write_u16::<BigEndian>(0).unwrap(); // variables count
            b.write_u16::<BigEndian>(0).unwrap(); // properties count
            b.write_u16::<BigEndian>(0).unwrap(); // states count
            b
        };
        data.write_u32::<BigEndian>((body.len() as u32) + 4)
            .unwrap(); // size = body + 4 (size字段自身)
        data.extend_from_slice(&body);

        data
    }

    #[test]
    fn test_roundtrip_big_endian_byte_for_byte() {
        let original = build_minimal_skyrim_pex();

        let mut cur = Cursor::new(original.clone());
        let script = parse_pex(&mut cur).unwrap();

        // 无翻译 roundtrip
        let (recompiled, updated, _) = compile_pex_bytes(&script, &[]).unwrap();
        assert_eq!(updated, 0, "无翻译时不应更新任何字符串");
        assert_eq!(
            recompiled, original,
            "Big-Endian roundtrip 必须 byte-for-byte 相同"
        );
    }

    /// 构造一个最小的真实 Starfield PEX（Little-Endian，含完整 Object Body 结构）
    fn build_minimal_starfield_pex() -> Vec<u8> {
        use byteorder::{LittleEndian, WriteBytesExt};
        let mut data = Vec::new();
        data.extend_from_slice(&[0xDE, 0xC0, 0x57, 0xFA]); // Magic LE
        data.push(3);
        data.push(9);
        data.write_u16::<LittleEndian>(4).unwrap(); // GameID = 4 (Starfield)
        data.write_u64::<LittleEndian>(0).unwrap();
        for s in &["Source/StarTest.psc", "StarUser", "StarPC"] {
            data.write_u16::<LittleEndian>(s.len() as u16).unwrap();
            data.extend_from_slice(s.as_bytes());
        }
        let strings = &["", "StarScript", "DoIt", "None", "myGuard"];
        data.write_u16::<LittleEndian>(strings.len() as u16)
            .unwrap();
        for s in strings {
            data.write_u16::<LittleEndian>(s.len() as u16).unwrap();
            data.extend_from_slice(s.as_bytes());
        }
        data.push(0); // has_debug_info = 0
        data.write_u16::<LittleEndian>(0).unwrap(); // user_flags
        data.write_u16::<LittleEndian>(1).unwrap(); // objects count
        data.write_u16::<LittleEndian>(1).unwrap(); // name = "StarScript"

        let mut body = Vec::new();
        body.write_u16::<LittleEndian>(0).unwrap(); // parentClass
        body.write_u16::<LittleEndian>(0).unwrap(); // docString
        body.push(0); // uConst (LE)
        body.write_u32::<LittleEndian>(0).unwrap(); // userFlags
        body.write_u16::<LittleEndian>(0).unwrap(); // autoStateName
        body.write_u16::<LittleEndian>(0).unwrap(); // structs count = 0
        body.write_u16::<LittleEndian>(0).unwrap(); // variables count = 0
        body.write_u16::<LittleEndian>(1).unwrap(); // guards count = 1 (Starfield)
        body.write_u16::<LittleEndian>(4).unwrap(); // "myGuard"
        body.write_u16::<LittleEndian>(0).unwrap(); // properties count = 0
        body.write_u16::<LittleEndian>(1).unwrap(); // states count = 1
        body.write_u16::<LittleEndian>(0).unwrap(); // state name = ""
        body.write_u16::<LittleEndian>(1).unwrap(); // functions count = 1
        body.write_u16::<LittleEndian>(2).unwrap(); // name = "DoIt"
        body.write_u16::<LittleEndian>(3).unwrap(); // return_type = "None"
        body.write_u16::<LittleEndian>(0).unwrap(); // doc = ""
        body.write_u32::<LittleEndian>(0).unwrap(); // uFlags
        body.push(0); // flags
        body.write_u16::<LittleEndian>(0).unwrap(); // params = 0
        body.write_u16::<LittleEndian>(0).unwrap(); // locals = 0
        body.write_u16::<LittleEndian>(1).unwrap(); // instructions = 1
        body.push(0x1A); // Return
        body.push(0); // None

        data.write_u32::<LittleEndian>((body.len() as u32) + 4)
            .unwrap(); // size = body + 4
        data.extend_from_slice(&body);
        data
    }

    #[test]
    fn test_roundtrip_little_endian_byte_for_byte() {
        let original = build_minimal_starfield_pex();

        let mut cur = Cursor::new(original.clone());
        let script = parse_pex(&mut cur).unwrap();

        let (recompiled, updated, _) = compile_pex_bytes(&script, &[]).unwrap();
        assert_eq!(updated, 0, "无翻译时不应更新任何字符串");
        assert_eq!(
            recompiled, original,
            "Little-Endian roundtrip 必须 byte-for-byte 相同"
        );
    }
}
