//! PEX binary compiler — writes translated strings back to PEX files
//!
//! This module rebuilds PEX files with updated string tables, preserving
//! all original structure while replacing string text with translations.
//!
//! Strategy: parse preserves ALL raw bytes (debug info, user flags, object bodies).
//! Compile only modifies string table entries IN-PLACE, keeping indices stable
//! so opcode references in object bodies remain valid. This is the same approach
//! Delphi xTranslator uses.

use std::collections::HashMap;
use std::io::{self, Write, Cursor};
use super::types::{PexScript, PexTranslatableString, PexStringEntry};

/// Compilation result
#[derive(Debug)]
pub struct CompileResult {
    /// Path to compiled file
    pub path: String,
    /// Number of strings updated
    pub updated_count: usize,
    /// Warnings encountered
    pub warnings: Vec<String>,
}

/// Build updated string table preserving original indices
///
/// Key guarantee: indices are NEVER changed, only the text at each index.
/// This ensures all opcode references in object bodies remain valid.
pub fn build_string_table(
    original: &[PexStringEntry],
    translations: &[PexTranslatableString],
) -> (Vec<PexStringEntry>, HashMap<String, u16>, usize) {
    // Clone table so we can modify in-place
    let mut table: Vec<PexStringEntry> = original.to_vec();

    // Build original text -> index mapping
    let mut text_to_index: HashMap<String, u16> = HashMap::new();
    for entry in &table {
        text_to_index.insert(entry.text.clone(), entry.index);
    }

    // Apply translations in-place (indices never change)
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

    // Rebuild mapping for caller reference
    let mut new_map = HashMap::new();
    for entry in &table {
        new_map.insert(entry.text.clone(), entry.index);
    }

    (table, new_map, updated_count)
}

/// Write PEX file with updated strings
///
/// Preserves ALL original binary data except string table text:
/// - Magic, Header: verbatim
/// - String table: updated text at existing indices
/// - Debug info: verbatim from original
/// - User flags: verbatim from original
/// - Object bodies: verbatim from original (indices unchanged)
pub fn compile_pex(
    original_script: &PexScript,
    translations: &[PexTranslatableString],
    output_path: &str,
) -> io::Result<CompileResult> {
    let mut warnings = Vec::new();

    // Build updated string table (indices preserved)
    let (new_string_table, _, updated_count) =
        build_string_table(&original_script.string_table, translations);

    // Warn if translations reference strings not found in table
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
    buffer.write_all(&[original_script.header.major_version, original_script.header.minor_version])?;
    buffer.write_all(&original_script.header.game_id.to_le_bytes())?;
    buffer.write_all(&original_script.header.compile_time.to_le_bytes())?;

    // String table (updated text, same indices)
    buffer.write_all(&(new_string_table.len() as u16).to_le_bytes())?;
    for entry in &new_string_table {
        let text_bytes = entry.text.as_bytes();
        buffer.write_all(&(text_bytes.len() as u16).to_le_bytes())?;
        buffer.write_all(text_bytes)?;
    }

    // Debug info — verbatim from original
    buffer.write_all(&original_script.debug_info_raw)?;

    // User flags — verbatim from original
    buffer.write_all(&original_script.user_flags_raw)?;

    // Object bodies — verbatim from original (same count, same sizes)
    buffer.write_all(&(original_script.object_bodies_raw.len() as u16).to_le_bytes())?;
    for body in original_script.object_bodies_raw.iter() {
        // Object name index (read from original body at offset 0)
        if body.len() >= 2 {
            let name_idx = u16::from_le_bytes([body[0], body[1]]);
            buffer.write_all(&name_idx.to_le_bytes())?;
        } else {
            buffer.write_all(&0u16.to_le_bytes())?;
        }
        // Body size
        buffer.write_all(&(body.len() as u32).to_le_bytes())?;
        // Body data verbatim
        buffer.write_all(body)?;
    }

    // Write to file
    let data = buffer.into_inner();
    std::fs::write(output_path, &data)?;

    Ok(CompileResult {
        path: output_path.to_string(),
        updated_count,
        warnings,
    })
}

/// Convenience: compile a single PEX file
///
/// Opens the file, parses it, applies translations, and writes result.
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

        // String table
        data.extend_from_slice(&(strings.len() as u16).to_le_bytes());
        for (text, _) in strings {
            let bs = text.as_bytes();
            data.extend_from_slice(&(bs.len() as u16).to_le_bytes());
            data.extend_from_slice(bs);
        }

        // Debug info (empty)
        data.extend_from_slice(&0u64.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());

        // User flags (empty)
        data.extend_from_slice(&0u16.to_le_bytes());

        // Objects
        data.extend_from_slice(&object_count.to_le_bytes());
        if object_count > 0 {
            // Minimum valid empty body needs 14 bytes:
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
            PexStringEntry { index: 0, text: "Hello".to_string() },
            PexStringEntry { index: 1, text: "World".to_string() },
            PexStringEntry { index: 2, text: "Test".to_string() },
        ];

        let translations = vec![
            PexTranslatableString {
                object_name: "MyScript".to_string(),
                state_name: String::new(),
                function_name: String::new(),
                string_type: "DebugString".to_string(),
                source_text: "Hello".to_string(),
                translation: "你好".to_string(),
            },
        ];

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
            PexStringEntry { index: 0, text: "A".to_string() },
            PexStringEntry { index: 5, text: "B".to_string() },
        ];

        let (updated, _, _) = build_string_table(&original, &[]);

        assert_eq!(updated[0].index, 0);
        assert_eq!(updated[1].index, 5);
    }

    /// Roundtrip test: parse → compile → re-parse, verify string table unchanged
    #[test]
    fn test_compile_preserves_binary_structure() {
        let body = [0u8; 16]; // minimal valid empty body (14 bytes min)
        let original_bytes = build_test_pex_bytes(
            &[
                ("TestObject", 0),
                ("English text", 1),
                ("Another string", 2),
            ],
            1,
            &body,
        );

        // Parse
        let mut cur = Cursor::new(&original_bytes[..]);
        let script = super::super::parser::parse_pex(&mut cur).unwrap();
        assert_eq!(script.string_table.len(), 3);
        assert_eq!(script.string_table[1].text, "English text");
        assert_eq!(script.object_bodies_raw.len(), 1);
        assert_eq!(script.object_bodies_raw[0].len(), 16);

        // Apply translation
        let translations = vec![PexTranslatableString {
            object_name: "TestObject".to_string(),
            state_name: String::new(),
            function_name: String::new(),
            string_type: "DebugString".to_string(),
            source_text: "English text".to_string(),
            translation: "英文文本".to_string(),
        }];

        // Compile to temp file
        let tmp_path = std::env::temp_dir().join("xt_pex_roundtrip_test.pex");
        compile_pex(&script, &translations, tmp_path.to_str().unwrap()).unwrap();

        let mut reparse_cur = Cursor::new(std::fs::read(&tmp_path).unwrap());
        let reparsed = super::super::parser::parse_pex(&mut reparse_cur).unwrap();

        // Verify: string table text updated
        assert_eq!(reparsed.string_table[1].text, "英文文本");
        // Verify: indices unchanged
        assert_eq!(reparsed.string_table[1].index, 1);
        assert_eq!(reparsed.string_table[0].text, "TestObject");
        assert_eq!(reparsed.string_table[2].text, "Another string");
        // Verify: object bodies preserved verbatim
        assert_eq!(reparsed.object_bodies_raw.len(), 1);
        assert_eq!(reparsed.object_bodies_raw[0].len(), 16);
        assert_eq!(reparsed.object_bodies_raw[0], &[0u8; 16]);
        // Core invariants: string table updated, indices unchanged, bodies preserved
        assert_eq!(reparsed.string_table.len(), 3);
        assert_eq!(reparsed.string_table[0].text, "TestObject");
        assert_eq!(reparsed.string_table[1].text, "英文文本"); // translated
        assert_eq!(reparsed.string_table[1].index, 1);        // index unchanged
        assert_eq!(reparsed.string_table[2].text, "Another string");
        assert_eq!(reparsed.object_bodies_raw.len(), 1);
        assert_eq!(reparsed.object_bodies_raw[0].len(), 16);  // size unchanged
        assert_eq!(reparsed.object_bodies_raw[0], &[0u8; 16]); // content unchanged

        let _ = std::fs::remove_file(&tmp_path);
    }
}
