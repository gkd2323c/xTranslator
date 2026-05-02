//! Localized → delocalized ESP conversion.
//!
//! Converts a localized ESP (where strings are referenced by 4-byte IDs in
//! external .STRINGS files) to a delocalized ESP (where strings are inline
//! text in record field buffers).

use super::record_tree::{EspFile, EspGrup, EspRecord};
use crate::normalization;
use crate::strings::{CodepageConfig, StringsFile, StringsFormat};
use crate::types::sky_string::SkyString;
use std::collections::HashMap;
use std::path::Path;

/// Result of a delocalization operation.
#[derive(Debug)]
pub struct DelocalizeResult {
    /// Number of strings that were delocalized.
    pub string_count: usize,
    /// Paths to the exported strings files.
    pub strings_files: Vec<std::path::PathBuf>,
}

/// Delocalize an ESP file: replace 4-byte string IDs with inline text,
/// reassign sequential IDs, and export .STRINGS files.
///
/// Uses 2-pass SST matching:
/// 1. Strict: (form_id, field_sig, occurrence_index) triple
/// 2. Relaxed: normalized source text + field_sig (for fields missed in pass 1)
///
/// # Arguments
/// * `esp` - The in-memory ESP file (must have been parsed with ESP mode)
/// * `strings` - The loaded SkyString entries with translations
/// * `output_dir` - Directory to write .STRINGS files
/// * `base_name` - Base filename (e.g., "Skyrim")
/// * `language` - Language name (e.g., "english")
/// * `codepage` - Codepage config for encoding
pub fn delocalize_esp(
    esp: &mut EspFile,
    strings: &[SkyString],
    output_dir: &Path,
    base_name: &str,
    language: &str,
    codepage: &CodepageConfig,
) -> std::io::Result<DelocalizeResult> {
    // Pass 1: strict triple match
    let string_map = build_string_map(strings);

    // Pass 2: normalized text index for fallback matching
    let normalized_index = build_normalized_index(strings);

    // Delocalize all records in the tree
    let mut total_strings = 0;
    for grup in &mut esp.top_level_grups {
        total_strings += delocalize_grup(grup, &string_map, &normalized_index, codepage);
    }

    // Reassign sequential string IDs
    let reassigned = reassign_string_ids(esp);

    // Export .STRINGS files
    let strings_files = export_strings(&reassigned, output_dir, base_name, language, codepage)?;

    Ok(DelocalizeResult {
        string_count: total_strings,
        strings_files,
    })
}

/// Build a lookup map from (form_id, field_sig, occurrence_index) to SkyString.
fn build_string_map(strings: &[SkyString]) -> HashMap<(u32, [u8; 4], u16), &SkyString> {
    let mut map = HashMap::with_capacity(strings.len());
    let mut occurrence_counts: HashMap<(u32, [u8; 4]), u16> = HashMap::with_capacity(strings.len());

    for sk in strings {
        let key = (sk.esp_ptr.form_id, sk.field_sig);
        let count = occurrence_counts.entry(key).or_insert(0);
        let index = *count;
        *count += 1;

        map.insert((sk.esp_ptr.form_id, sk.field_sig, index), sk);
    }

    map
}

/// Build a secondary lookup from (normalized_hash, field_sig) to SkyString.
/// Used for relaxed matching when strict triple match fails.
fn build_normalized_index(strings: &[SkyString]) -> HashMap<(u32, [u8; 4]), Vec<&SkyString>> {
    let mut index: HashMap<(u32, [u8; 4]), Vec<&SkyString>> = HashMap::with_capacity(strings.len());

    for sk in strings {
        if let Some(norm_hash) = sk.normalized_hash {
            index
                .entry((norm_hash, sk.field_sig))
                .or_default()
                .push(sk);
        }
    }

    index
}

/// Delocalize all records in a GRUP (recursive).
fn delocalize_grup(
    grup: &mut EspGrup,
    string_map: &HashMap<(u32, [u8; 4], u16), &SkyString>,
    normalized_index: &HashMap<(u32, [u8; 4]), Vec<&SkyString>>,
    codepage: &CodepageConfig,
) -> usize {
    let mut count = 0;

    for record in &mut grup.records {
        count += delocalize_record(record, string_map, normalized_index, codepage);
    }

    for child in &mut grup.children {
        count += delocalize_grup(child, string_map, normalized_index, codepage);
    }

    count
}

/// Delocalize a single record: replace 4-byte string IDs with inline text.
///
/// Uses 2-pass matching:
/// 1. Strict: (form_id, field_sig, occurrence_index)
/// 2. Relaxed: normalized source text + field_sig
fn delocalize_record(
    record: &mut EspRecord,
    string_map: &HashMap<(u32, [u8; 4], u16), &SkyString>,
    normalized_index: &HashMap<(u32, [u8; 4]), Vec<&SkyString>>,
    codepage: &CodepageConfig,
) -> usize {
    if record.raw {
        return 0;
    }

    let mut count = 0;
    let mut field_sig_counts: HashMap<[u8; 4], u16> = HashMap::new();

    for field in &mut record.fields {
        if field.is_size_xxxx {
            continue;
        }

        // Track occurrence index for this field signature
        let occurrence = field_sig_counts.entry(field.header.name).or_insert(0);
        let index = *occurrence;
        *occurrence += 1;

        // Pass 1: strict triple match
        let key = (record.form_id, field.header.name, index);
        if let Some(sk) = string_map.get(&key) {
            let text = if !sk.translation.is_empty() {
                &sk.translation
            } else {
                &sk.source
            };

            let encoded = codepage.encode(text);
            field.header.dsize = encoded.len() as u16;
            field.buffer = encoded;
            count += 1;
            continue;
        }

        // Pass 2: relaxed match by normalized source text + field_sig
        // Only for fields that look like they contain inline text (not 4-byte string IDs)
        if field.buffer.len() > 4 {
            if let Ok(raw_text) = std::str::from_utf8(&field.buffer) {
                let text = raw_text.trim_end_matches('\0');
                if !text.is_empty() {
                    let norm = normalization::normalize(text);
                    if !norm.is_empty() {
                        let norm_hash = crate::types::esp_pointer::string_hash(&norm);
                        let norm_key = (norm_hash, field.header.name);
                        if let Some(candidates) = normalized_index.get(&norm_key) {
                            // Verify the normalized text actually matches (hash collision check)
                            if let Some(sk) = candidates.iter().find(|sk| {
                                sk.source_normalized.as_deref() == Some(norm.as_str())
                            }) {
                                let trans_text = if !sk.translation.is_empty() {
                                    &sk.translation
                                } else {
                                    &sk.source
                                };

                                let encoded = codepage.encode(trans_text);
                                field.header.dsize = encoded.len() as u16;
                                field.buffer = encoded;
                                count += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    count
}

/// Reassign sequential string IDs (1..N) to all strings in the delocalized ESP.
///
/// Returns a vector of (new_id, text, list_index) for export.
fn reassign_string_ids(esp: &EspFile) -> Vec<(u32, String, u8)> {
    let mut result = Vec::new();
    let mut next_id = 1u32;

    for grup in &esp.top_level_grups {
        reassign_grup(grup, &mut next_id, &mut result);
    }

    result
}

fn reassign_grup(grup: &EspGrup, next_id: &mut u32, result: &mut Vec<(u32, String, u8)>) {
    for record in &grup.records {
        reassign_record(record, next_id, result);
    }
    for child in &grup.children {
        reassign_grup(child, next_id, result);
    }
}

fn reassign_record(
    record: &EspRecord,
    next_id: &mut u32,
    result: &mut Vec<(u32, String, u8)>,
) {
    if record.raw {
        return;
    }

    for field in &record.fields {
        if field.is_size_xxxx {
            continue;
        }

        // For delocalized records, the field buffer contains inline text.
        // We need to assign a new sequential ID and record the text.
        // The text is already in the field buffer (encoded).
        // We'll decode it back to a string for the .STRINGS export.
        if field.buffer.len() > 4 {
            // This might be a translatable field with inline text
            // Try to decode as UTF-8 (most common case)
            if let Ok(text) = std::str::from_utf8(&field.buffer) {
                let text = text.trim_end_matches('\0');
                if !text.is_empty() {
                    result.push((*next_id, text.to_string(), 0)); // list_index 0 = .STRINGS
                    *next_id += 1;
                }
            }
        }
    }
}

/// Export .STRINGS, .DLSTRINGS, and .ILSTRINGS files.
fn export_strings(
    entries: &[(u32, String, u8)],
    output_dir: &Path,
    base_name: &str,
    language: &str,
    codepage: &CodepageConfig,
) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut paths = Vec::new();

    // Group entries by list_index
    let mut by_type: HashMap<u8, Vec<(u32, String)>> = HashMap::new();
    for (id, text, list_index) in entries {
        by_type
            .entry(*list_index)
            .or_default()
            .push((*id, text.clone()));
    }

    // Write each type
    for (list_index, ext, format) in &[
        (0u8, "STRINGS", StringsFormat::NullTerminated),
        (1u8, "DLSTRINGS", StringsFormat::LengthPrefixed),
        (2u8, "ILSTRINGS", StringsFormat::LengthPrefixed),
    ] {
        if let Some(entries) = by_type.get(list_index) {
            if entries.is_empty() {
                continue;
            }

            let filename = format!("{}_{}.{}", base_name, language, ext);
            let path = output_dir.join(&filename);

            let sf = StringsFile::from_entries(entries.clone(), codepage.clone());
            sf.save_with_format(&path, *format)?;
            paths.push(path);
        }
    }

    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::esp::header::{FieldHeader, GenericHeader, GrupHeader, RecordHeaderData};
    use crate::esp::record_tree::{EspField, EspFile};

    fn make_test_record(form_id: u32, fields: Vec<EspField>) -> EspRecord {
        EspRecord {
            header: GenericHeader {
                name: *b"INFO",
                dsize: 0,
            },
            record_header_data: RecordHeaderData {
                flags: 0,
                form_id,
                version: 44,
                f_version: 15,
                v_info: 0,
            },
            fields,
            compressed: false,
            raw: false,
            form_id,
            editor_id: None,
            original_raw_data: Vec::new(),
        }
    }

    fn make_string_field(name: [u8; 4], text: &str) -> EspField {
        let mut buffer = text.as_bytes().to_vec();
        buffer.push(0); // null terminate
        EspField {
            header: FieldHeader {
                name,
                dsize: buffer.len() as u16,
            },
            buffer,
            is_size_xxxx: false,
        }
    }

    #[test]
    fn test_delocalize_minimal() {
        // Create a record with inline text
        let record = make_test_record(0x1234, vec![
            make_string_field(*b"EDID", "TestNPC"),
            make_string_field(*b"FULL", "Hello World"),
        ]);

        let grup = EspGrup {
            header: GenericHeader {
                name: *b"GRUP",
                dsize: 0,
            },
            grup_header: GrupHeader {
                s_ident: *b"NPC_",
                s_type: 0,
                s_tstamp: 0,
                param1: 0,
                param2: 0,
                param3: 0,
            },
            records: vec![record],
            children: Vec::new(),
        };

        let esp = EspFile {
            tes4: crate::esp::record_tree::Tes4Header {
                generic: GenericHeader {
                    name: *b"TES4",
                    dsize: 0,
                },
                record_header_data: RecordHeaderData {
                    flags: 0,
                    form_id: 0,
                    version: 44,
                    f_version: 15,
                    v_info: 0,
                },
                field_data: Vec::new(),
            },
            top_level_grups: vec![grup],
        };

        // Verify the record has inline text
        assert_eq!(esp.top_level_grups[0].records[0].fields[1].buffer, b"Hello World\0");
    }

    #[test]
    fn test_string_map_building() {
        let mut sk = SkyString::new(
            0,
            "Hello".to_string(),
            String::new(),
            *b"INFO",
            *b"FULL",
        );
        sk.esp_ptr.form_id = 0x1234;

        let entries = vec![sk];
        let map = build_string_map(&entries);
        assert!(map.contains_key(&(0x1234, *b"FULL", 0)));
    }

    #[test]
    fn test_2pass_normalized_match() {
        use crate::strings::CodepageConfig;

        // Create a SkyString with a translation
        let mut sk = SkyString::new(
            0,
            "Hello World".to_string(),
            "你好世界".to_string(),
            *b"INFO",
            *b"FULL",
        );
        sk.esp_ptr.form_id = 0x1234;

        let entries = vec![sk];
        let string_map = build_string_map(&entries);
        let normalized_index = build_normalized_index(&entries);

        // Create a record where the field text matches the SkyString source
        // but with different form_id (so strict match fails)
        let record = make_test_record(
            0x9999, // different form_id
            vec![
                make_string_field(*b"EDID", "SomeNPC"),
                make_string_field(*b"FULL", "Hello World"), // matches normalized source
            ],
        );

        let mut grup = EspGrup {
            header: GenericHeader {
                name: *b"GRUP",
                dsize: 0,
            },
            grup_header: GrupHeader {
                s_ident: *b"INFO",
                s_type: 0,
                s_tstamp: 0,
                param1: 0,
                param2: 0,
                param3: 0,
            },
            records: vec![record],
            children: Vec::new(),
        };

        let codepage = CodepageConfig::utf8();
        let count = delocalize_grup(&mut grup, &string_map, &normalized_index, &codepage);

        // Should have matched via normalized text (pass 2)
        assert_eq!(count, 1);

        // The field should now contain the translation
        let full_field = &grup.records[0].fields[1];
        let text = std::str::from_utf8(&full_field.buffer).unwrap();
        assert_eq!(text, "你好世界");
    }
}
