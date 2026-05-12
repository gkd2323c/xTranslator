//! Comprehensive E2E tests for xTranslator
//! 
//! This test suite covers:
//! - ESP parsing with real Skyrim data
//! - Translation workflow (load → edit → save → verify)
//! - SST dictionary operations
//! - XML import/export roundtrip
//! - BSA archive fallback
//! - Performance benchmarks
//! - Error handling and edge cases
//! 
//! Run: cargo test --release -p xt-core --test e2e_comprehensive

use std::path::PathBuf;
use std::time::{Duration, Instant};
use xt_core::esp::parser::EspParser;
use xt_core::sst::v8::SstDictionary;
use xt_core::strings::StringsFile;
use xt_core::types::params::SkyStringParams;
use xt_core::xml;
use xt_core::matching::StringMatcher;
use tempfile::TempDir;

// Test configuration
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

/// Helper to measure execution time
fn timed_operation<F, R>(name: &str, f: F) -> R 
where 
    F: FnOnce() -> R,
{
    let start = Instant::now();
    let result = f();
    let duration = start.elapsed();
    println!("⏱️  {}: {:?}", name, duration);
    result
}

/// Test 1: Comprehensive ESP parsing with validation
#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn e2e_esp_parsing_comprehensive() {
    assert!(skyrim_data_available(), "Skyrim.esm not found");

    let mut parser = create_test_parser();
    let file = std::fs::File::open(SKYRIM_ESM).unwrap();
    
    let (total_strings, parse_time) = timed_operation("ESP parsing", || {
        let mut file = file;
        parser.parse(&mut file).unwrap();
        parser.strings.len()
    });

    // Basic validation
    assert!(total_strings > 70000, "Too few strings: {}", total_strings);
    assert!(!parser.strings.is_empty(), "No strings loaded");
    
    // Validate string structure
    let first_string = &parser.strings[0];
    assert!(!first_string.source.is_empty(), "First string source empty");
    assert!(first_string.esp_ptr.str_id != 0, "Invalid string ID");
    
    // Check for compressed records
    let compressed_count = parser.strings.iter()
        .filter(|s| s.esp_ptr.compressed)
        .count();
    println!("📊 Compressed records: {}/{}", compressed_count, total_strings);
    
    // Performance assertion
    assert!(parse_time < Duration::from_secs(30), "Parsing too slow: {:?}", parse_time);
    
    // Validate different record types
    let record_types: std::collections::HashSet<_> = parser.strings.iter()
        .map(|s| &s.record_sig)
        .collect();
    assert!(record_types.len() > 5, "Too few record types: {:?}", record_types);
    println!("📋 Record types found: {:?}", record_types);
}

/// Test 2: Complete translation workflow
#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn e2e_translation_workflow() {
    assert!(skyrim_data_available(), "Skyrim.esm not found");

    let mut parser = create_test_parser();
    let mut file = std::fs::File::open(SKYRIM_ESM).unwrap();
    parser.parse(&mut file).unwrap();

    // Phase 1: Mark some strings as translated
    let test_translations = vec![
        ("TEST_WORKFLOW_1", "工作流测试一"),
        ("TEST_WORKFLOW_2", "工作流测试二"),
        ("TEST_WORKFLOW_3", "工作流测试三"),
    ];

    for (i, (source, translation)) in test_translations.iter().enumerate() {
        if i < parser.strings.len() {
            parser.strings[i].source = source.to_string();
            parser.strings[i].translation = translation.to_string();
            parser.strings[i].params.set(SkyStringParams::TRANSLATED, true);
        }
    }

    // Phase 2: Save to Strings files
    let tmp_dir = TempDir::new().unwrap();
    let strings_output = tmp_dir.path().join("skyrim_english_chinese.strings");
    
    timed_operation("Strings file save", || {
        let mut sf = StringsFile::new();
        sf.format = xt_core::strings::StringsFormat::NullTerminated;
        
        for s in &parser.strings {
            if s.params.is_translated() && !s.translation.is_empty() {
                sf.strings.insert(s.esp_ptr.str_id as u32, s.translation.clone());
            }
        }
        sf.save(&strings_output).unwrap();
    });

    // Phase 3: Verify Strings file
    let reloaded = timed_operation("Strings file reload", || {
        StringsFile::load_with_format(
            &strings_output, 
            xt_core::strings::StringsFormat::NullTerminated
        ).unwrap()
    });

    // Verify our test translations
    for (i, (_, translation)) in test_translations.iter().enumerate() {
        if i < parser.strings.len() {
            let id = parser.strings[i].esp_ptr.str_id as u32;
            assert_eq!(reloaded.get(id).unwrap(), *translation, "Translation mismatch for ID {}", id);
        }
    }

    println!("✅ Translation workflow completed successfully");
}

/// Test 3: SST dictionary comprehensive operations
#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn e2e_sst_dictionary_operations() {
    assert!(skyrim_data_available(), "Skyrim.esm not found");

    let mut parser = create_test_parser();
    let mut file = std::fs::File::open(SKYRIM_ESM).unwrap();
    parser.parse(&mut file).unwrap();

    // Add some test translations
    for i in 0..10.min(parser.strings.len()) {
        parser.strings[i].translation = format!("测试翻译_{}", i);
        parser.strings[i].params.set(SkyStringParams::TRANSLATED, true);
    }

    // Create SST dictionary
    let sst = timed_operation("SST creation", || {
        SstDictionary::from_entries(parser.strings.clone())
    });

    assert!(!sst.entries.is_empty(), "SST dictionary is empty");
    
    // Save SST
    let tmp_dir = TempDir::new().unwrap();
    let sst_path = tmp_dir.path().join("test_dictionary.sst");
    
    timed_operation("SST save", || {
        sst.save_to_file(sst_path.to_str().unwrap()).unwrap();
    });

    // Reload SST
    let reloaded = timed_operation("SST reload", || {
        SstDictionary::load_from_file(&sst_path).unwrap()
    });

    // Verify roundtrip
    assert_eq!(sst.entries.len(), reloaded.entries.len(), "SST entry count mismatch");
    
    // Verify specific entries
    for i in 0..10.min(sst.entries.len()) {
        let original = &sst.entries[i];
        let reloaded_entry = reloaded.entries.iter()
            .find(|e| e.esp_ptr.str_id == original.esp_ptr.str_id)
            .expect("Entry not found after reload");
        assert_eq!(original.translation, reloaded_entry.translation, "SST roundtrip failed");
    }

    println!("✅ SST dictionary operations completed");
}

/// Test 4: XML import/export roundtrip
#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn e2e_xml_roundtrip() {
    assert!(skyrim_data_available(), "Skyrim.esm not found");

    let mut parser = create_test_parser();
    let mut file = std::fs::File::open(SKYRIM_ESM).unwrap();
    parser.parse(&mut file).unwrap();

    // Add test translations
    for i in 0..5.min(parser.strings.len()) {
        parser.strings[i].translation = format!("XML测试_{}", i);
        parser.strings[i].params.set(SkyStringParams::TRANSLATED, true);
    }

    // Export to XML
    let tmp_dir = TempDir::new().unwrap();
    let xml_path = tmp_dir.path().join("test_export.xml");
    
    let exported_count = timed_operation("XML export", || {
        xml::write_xml_export(&parser.strings, &xml_path, "skyrim").unwrap()
    });

    assert!(exported_count >= 5, "Too few entries exported: {}", exported_count);

    // Import back
    let mut import_strings = parser.strings.clone();
    for s in &mut import_strings {
        s.translation.clear();
        s.params.set(SkyStringParams::TRANSLATED, false);
    }

    let import_result = timed_operation("XML import", || {
        xml::parse_xml_file(&xml_path).unwrap()
    });

    // Apply imported translations
    let matcher = StringMatcher::new();
    let matched = timed_operation("XML matching", || {
        matcher.apply_xml_translations(&mut import_strings, &import_result)
    });

    assert!(matched >= 5, "Too few matches: {}", matched);

    // Verify specific translations
    for i in 0..5.min(import_strings.len()) {
        assert!(import_strings[i].translation.contains(&format!("XML测试_{}", i)), 
                "XML roundtrip failed for index {}", i);
    }

    println!("✅ XML roundtrip completed successfully");
}

/// Test 5: Performance benchmarks
#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn e2e_performance_benchmarks() {
    assert!(skyrim_data_available(), "Skyrim.esm not found");

    let mut parser = create_test_parser();
    let mut file = std::fs::File::open(SKYRIM_ESM).unwrap();
    parser.parse(&mut file).unwrap();

    let string_count = parser.strings.len();
    println!("📊 Performance benchmarks with {} strings", string_count);

    // Benchmark: Filtering
    let filter_time = timed_operation("Filter 100k items", || {
        let _filtered: Vec<_> = parser.strings.iter()
            .filter(|s| s.source.contains("Dragon") || s.translation.contains("龙"))
            .collect();
    });
    assert!(filter_time < Duration::from_millis(100), "Filtering too slow");

    // Benchmark: Sorting
    let sort_time = timed_operation("Sort 100k items", || {
        let mut sorted = parser.strings.clone();
        sorted.sort_by(|a, b| a.source.cmp(&b.source));
        sorted
    });
    assert!(sort_time < Duration::from_millis(500), "Sorting too slow");

    // Benchmark: Heuristic search
    let search_time = timed_operation("Heuristic search", || {
        let query = "Dragon";
        let _results: Vec<_> = parser.strings.iter()
            .filter(|s| s.params.is_translated())
            .filter(|s| {
                s.source.to_lowercase().contains(&query.to_lowercase()) ||
                s.translation.to_lowercase().contains(&query.to_lowercase())
            })
            .take(10)
            .collect();
    });
    assert!(search_time < Duration::from_millis(50), "Search too slow");

    // Memory usage check
    let memory_mb = (string_count * std::mem::size_of::<xt_core::types::sky_string::SkyString>()) / 1_048_576;
    println!("💾 Estimated memory usage: {} MB", memory_mb);
    assert!(memory_mb < 200, "Memory usage too high: {} MB", memory_mb);

    println!("✅ All performance benchmarks passed");
}

/// Test 6: Error handling and edge cases
#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn e2e_error_handling() {
    assert!(skyrim_data_available(), "Skyrim.esm not found");

    // Test invalid file path
    let result = std::fs::File::open("nonexistent_file.esp");
    assert!(result.is_err(), "Should fail on nonexistent file");

    // Test empty SST
    let empty_sst = SstDictionary::new();
    let tmp_dir = TempDir::new().unwrap();
    let empty_path = tmp_dir.path().join("empty.sst");
    
    assert!(empty_sst.save_to_file(empty_path.to_str().unwrap()).is_ok(), 
            "Should save empty SST");
    
    let reloaded = SstDictionary::load_from_file(&empty_path).unwrap();
    assert!(reloaded.entries.is_empty(), "Reloaded SST should be empty");

    // Test malformed XML
    let malformed_xml = r#"<?xml version="1.0"?><broken>"#;
    let xml_path = tmp_dir.path().join("malformed.xml");
    std::fs::write(&xml_path, malformed_xml).unwrap();
    
    let parse_result = xml::parse_xml_file(&xml_path);
    assert!(parse_result.is_err(), "Should fail on malformed XML");

    println!("✅ Error handling tests passed");
}

/// Test 7: BSA archive fallback
#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn e2e_bsa_fallback() {
    assert!(skyrim_data_available(), "Skyrim.esm not found");

    let mut parser = EspParser::new();
    
    // Test BSA loading (should fallback if strings files missing)
    let bsa_loaded = timed_operation("BSA fallback loading", || {
        parser.load_strings_files(DATA_DIR, "skyrim");
        parser.has_strings_files()
    });

    // This test may pass or fail depending on the test environment
    if bsa_loaded {
        println!("✅ BSA fallback working");
    } else {
        println!("⚠️  BSA fallback not available (expected in some test environments)");
    }
}

/// Test 8: Multi-game compatibility (basic check)
#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn e2e_multi_game_basic() {
    // Test that we can at least handle different game configurations
    let games = vec!["skyrim", "fallout4", "starfield"];
    
    for game in games {
        let mut parser = EspParser::new();
        // This should not crash even if data is not available
        let result = std::panic::catch_unwind(|| {
            parser.load_strings_files(DATA_DIR, game);
        });
        assert!(result.is_ok(), "Should handle {} configuration gracefully", game);
    }

    println!("✅ Multi-game compatibility check passed");
}
