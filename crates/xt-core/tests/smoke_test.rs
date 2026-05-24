//! Comprehensive smoke tests: load ESP -> edit translation -> save Strings -> verify roundtrip.
//!
//! Enhanced with additional validation, error handling, and edge case testing.
//!
//! Requires Skyrim SE installed at standard path.
//! Run: cargo test --release -p xt-core --test smoke_test

use std::path::PathBuf;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use xt_core::esp::parser::EspParser;
use xt_core::matching::StringMatcher;
use xt_core::sst::v8::SstDictionary;
use xt_core::strings::StringsFile;
use xt_core::types::params::SkyStringParams;
use xt_core::xml;

const SKYRIM_ESM: &str = r"D:\SteamLibrary\steamapps\common\Skyrim Special Edition\Data\Skyrim.esm";
const DATA_DIR: &str = r"D:\SteamLibrary\steamapps\common\Skyrim Special Edition\Data";

fn skyrim_data_available() -> bool {
    PathBuf::from(SKYRIM_ESM).exists()
}

fn create_test_parser() -> EspParser {
    let mut parser = EspParser::new();
    parser.load_strings_files(DATA_DIR, "skyrim");
    parser
}

/// 1. Parse ESP -> verify string count
#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn smoke_parse_esp() {
    assert!(skyrim_data_available(), "Skyrim.esm not found");

    let mut parser = create_test_parser();
    let mut parser = EspParser::new();
    parser.load_strings_files(DATA_DIR, "skyrim");
    let mut file = std::fs::File::open(SKYRIM_ESM).unwrap();
    parser.parse(&mut file).unwrap();

    assert!(
        parser.strings.len() > 70000,
        "Too few strings: {}",
        parser.strings.len()
    );
    assert!(
        !parser.strings[0].source.is_empty(),
        "First string source empty"
    );
}

/// 2. Parse -> edit a string -> save Strings -> reload -> verify
#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn smoke_edit_save_reload() {
    assert!(skyrim_data_available(), "Skyrim.esm not found");

    let mut parser = EspParser::new();
    parser.load_strings_files(DATA_DIR, "skyrim");
    let mut file = std::fs::File::open(SKYRIM_ESM).unwrap();
    parser.parse(&mut file).unwrap();

    let test_text = "SMOKE_TEST_ROUNDTRIP_42";
    parser.strings[0].translation = test_text.to_string();
    parser.strings[0]
        .params
        .set(SkyStringParams::TRANSLATED, true);

    // Save translations to temp Strings file
    let tmp = std::env::temp_dir().join("xt_smoke_strings");
    let _ = std::fs::create_dir_all(&tmp);
    let output = tmp.join("skyrim_english_chinese.strings");

    let mut sf = StringsFile::new();
    sf.format = xt_core::strings::StringsFormat::NullTerminated;
    for s in &parser.strings {
        if s.params.is_translated() && !s.translation.is_empty() {
            sf.strings
                .insert(s.esp_ptr.str_id as u32, s.translation.clone());
        }
    }
    sf.save(&output).unwrap();

    // Reload and verify
    let reloaded =
        StringsFile::load_with_format(&output, xt_core::strings::StringsFormat::NullTerminated)
            .unwrap();
    let first_id = parser.strings[0].esp_ptr.str_id as u32;
    assert_eq!(
        reloaded.get(first_id).unwrap(),
        test_text,
        "Roundtrip failed"
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

/// 3. SST roundtrip: parse -> build SST -> save -> reload -> verify
#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn smoke_sst_roundtrip() {
    assert!(skyrim_data_available(), "Skyrim.esm not found");

    let mut parser = create_test_parser();
    let mut file = std::fs::File::open(SKYRIM_ESM).unwrap();
    parser.parse(&mut file).unwrap();

    let test_text = "SMOKE_SST_TEST_99";
    parser.strings[0].translation = test_text.to_string();
    parser.strings[0]
        .params
        .set(SkyStringParams::TRANSLATED, true);

    let sst = SstDictionary::from_entries(parser.strings.clone());
    let tmp = std::env::temp_dir().join("smoke_roundtrip.sst");
    sst.save_to_file(tmp.to_str().unwrap()).unwrap();

    let reloaded = SstDictionary::load_from_file(&tmp).unwrap();
    let first_id = parser.strings[0].esp_ptr.str_id;
    let found = reloaded
        .entries
        .iter()
        .find(|e| e.esp_ptr.str_id == first_id);
    assert!(found.is_some(), "SST roundtrip: entry not found");
    assert!(
        found.unwrap().translation.contains(test_text),
        "SST roundtrip: mismatch"
    );

    let _ = std::fs::remove_file(&tmp);
}

/// 4. XML import/export roundtrip
#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn smoke_xml_roundtrip() {
    assert!(skyrim_data_available(), "Skyrim.esm not found");

    let mut parser = create_test_parser();
    let mut file = std::fs::File::open(SKYRIM_ESM).unwrap();
    parser.parse(&mut file).unwrap();

    // Add test translations
    let test_translations = vec![
        ("XML_TEST_1", "XML测试一"),
        ("XML_TEST_2", "XML测试二"),
        ("XML_TEST_3", "XML测试三"),
    ];

    for (i, (source, translation)) in test_translations.iter().enumerate() {
        if i < parser.strings.len() {
            parser.strings[i].source = source.to_string();
            parser.strings[i].translation = translation.to_string();
            parser.strings[i]
                .params
                .set(SkyStringParams::TRANSLATED, true);
        }
    }

    // Export to XML
    let tmp_dir = TempDir::new().unwrap();
    let xml_path = tmp_dir.path().join("smoke_test.xml");

    let exported_count = xml::write_xml_export(&parser.strings, &xml_path, "skyrim").unwrap();
    assert!(
        exported_count >= 3,
        "Too few entries exported: {}",
        exported_count
    );

    // Import back
    let mut import_strings = parser.strings.clone();
    for s in &mut import_strings {
        s.translation.clear();
        s.params.set(SkyStringParams::TRANSLATED, false);
    }

    let import_result = xml::parse_xml_file(&xml_path).unwrap();
    let matcher = StringMatcher::new();
    let matched = matcher.apply_xml_translations(&mut import_strings, &import_result);

    assert!(matched >= 3, "Too few matches: {}", matched);

    // Verify specific translations
    for (i, (_, translation)) in test_translations.iter().enumerate() {
        if i < import_strings.len() {
            assert!(
                import_strings[i].translation.contains(translation),
                "XML roundtrip failed for index {}",
                i
            );
        }
    }

    println!("✅ XML roundtrip completed successfully");
}

/// 5. Performance validation
#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn smoke_performance_validation() {
    assert!(skyrim_data_available(), "Skyrim.esm not found");

    let start = Instant::now();
    let mut parser = create_test_parser();
    let mut file = std::fs::File::open(SKYRIM_ESM).unwrap();
    parser.parse(&mut file).unwrap();
    let parse_time = start.elapsed();

    let string_count = parser.strings.len();
    let strings_per_second = string_count as f64 / parse_time.as_secs_f64();

    println!("📊 Performance metrics:");
    println!("  Strings parsed: {}", string_count);
    println!("  Parse time: {:?}", parse_time);
    println!("  Strings/sec: {:.0}", strings_per_second);

    // Performance assertions
    assert!(
        parse_time < Duration::from_secs(30),
        "Parsing too slow: {:?}",
        parse_time
    );
    assert!(
        strings_per_second > 2000.0,
        "Parsing rate too low: {:.0} strings/sec",
        strings_per_second
    );
    assert!(
        string_count > 70000,
        "Too few strings loaded: {}",
        string_count
    );
}

/// 6. Error handling validation
#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn smoke_error_handling() {
    // Test invalid file path
    let result = std::fs::File::open("nonexistent_file.esp");
    assert!(result.is_err(), "Should fail on nonexistent file");

    // Test empty SST
    let empty_sst = SstDictionary::new();
    let tmp = std::env::temp_dir().join("empty_test.sst");

    assert!(
        empty_sst.save_to_file(tmp.to_str().unwrap()).is_ok(),
        "Should save empty SST"
    );

    let reloaded = SstDictionary::load_from_file(&tmp).unwrap();
    assert!(reloaded.entries.is_empty(), "Reloaded SST should be empty");

    // Test malformed XML
    let malformed_xml = r#"<?xml version="1.0"?><broken>"#;
    let xml_path = std::env::temp_dir().join("malformed.xml");
    std::fs::write(&xml_path, malformed_xml).unwrap();

    let parse_result = xml::parse_xml_file(&xml_path);
    assert!(parse_result.is_err(), "Should fail on malformed XML");

    // Cleanup
    let _ = std::fs::remove_file(&tmp);
    let _ = std::fs::remove_file(&xml_path);

    println!("✅ Error handling validation passed");
}

/// 7. Data integrity validation
#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn smoke_data_integrity() {
    assert!(skyrim_data_available(), "Skyrim.esm not found");

    let mut parser = create_test_parser();
    let mut file = std::fs::File::open(SKYRIM_ESM).unwrap();
    parser.parse(&mut file).unwrap();

    // Validate string structure
    for (i, s) in parser.strings.iter().take(100).enumerate() {
        assert!(!s.source.is_empty(), "String {} has empty source", i);
        assert!(s.esp_ptr.str_id != 0, "String {} has invalid ID", i);
        assert!(
            !s.record_sig.is_empty(),
            "String {} has empty record sig",
            i
        );
        assert!(!s.field_sig.is_empty(), "String {} has empty field sig", i);
    }

    // Validate record type distribution
    let mut record_counts = std::collections::HashMap::new();
    for s in &parser.strings {
        *record_counts.entry(&s.record_sig).or_insert(0) += 1;
    }

    assert!(
        record_counts.len() > 5,
        "Too few record types: {:?}",
        record_counts.keys()
    );
    println!("📋 Record types found: {:?}", record_counts);

    // Validate compressed records
    let compressed_count = parser
        .strings
        .iter()
        .filter(|s| s.esp_ptr.compressed)
        .count();
    let total_count = parser.strings.len();
    let compression_ratio = compressed_count as f64 / total_count as f64;

    println!(
        "📊 Compression ratio: {:.2}% ({}/{})",
        compression_ratio * 100.0,
        compressed_count,
        total_count
    );
    assert!(compression_ratio > 0.1, "Too few compressed records");

    println!("✅ Data integrity validation passed");
}

/// 8. Multi-format validation
#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn smoke_multi_format_validation() {
    assert!(skyrim_data_available(), "Skyrim.esm not found");

    let mut parser = create_test_parser();
    let mut file = std::fs::File::open(SKYRIM_ESM).unwrap();
    parser.parse(&mut file).unwrap();

    // Add test translations
    for i in 0..10.min(parser.strings.len()) {
        parser.strings[i].translation = format!("多格式测试_{}", i);
        parser.strings[i]
            .params
            .set(SkyStringParams::TRANSLATED, true);
    }

    let tmp_dir = TempDir::new().unwrap();

    // Test Null-terminated format
    let null_path = tmp_dir.path().join("test_null.strings");
    let mut sf_null = StringsFile::new();
    sf_null.format = xt_core::strings::StringsFormat::NullTerminated;
    for s in &parser.strings {
        if s.params.is_translated() && !s.translation.is_empty() {
            sf_null
                .strings
                .insert(s.esp_ptr.str_id as u32, s.translation.clone());
        }
    }
    sf_null.save(&null_path).unwrap();

    let reloaded_null =
        StringsFile::load_with_format(&null_path, xt_core::strings::StringsFormat::NullTerminated)
            .unwrap();

    // Test Length-prefixed format
    let length_path = tmp_dir.path().join("test_length.strings");
    let mut sf_length = StringsFile::new();
    sf_length.format = xt_core::strings::StringsFormat::LengthPrefixed;
    for s in &parser.strings {
        if s.params.is_translated() && !s.translation.is_empty() {
            sf_length
                .strings
                .insert(s.esp_ptr.str_id as u32, s.translation.clone());
        }
    }
    sf_length.save(&length_path).unwrap();

    let reloaded_length = StringsFile::load_with_format(
        &length_path,
        xt_core::strings::StringsFormat::LengthPrefixed,
    )
    .unwrap();

    // Verify both formats have same content
    assert_eq!(
        reloaded_null.strings.len(),
        reloaded_length.strings.len(),
        "Format mismatch in string count"
    );

    for (id, translation) in &reloaded_null.strings {
        assert_eq!(
            reloaded_length.strings.get(id),
            Some(translation),
            "Translation mismatch for ID {} between formats",
            id
        );
    }

    println!("✅ Multi-format validation passed");
}
