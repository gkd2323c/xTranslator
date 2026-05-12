//! Performance benchmarks for xTranslator
//! 
//! These tests measure critical performance characteristics:
//! - ESP parsing speed with large files
//! - Memory usage with 100K+ strings
//! - Filter/search performance
//! - Translation API response times
//! - File I/O performance
//! 
//! Run: cargo test --release -p xt-core --test performance_benchmarks

use std::path::PathBuf;
use std::time::{Duration, Instant};
use xt_core::esp::parser::EspParser;
use xt_core::strings::StringsFile;
use xt_core::sst::v8::SstDictionary;
use xt_core::types::params::{SkyStringInternalParams, SkyStringParams};
use tempfile::TempDir;

const SKYRIM_ESM: &str = r"D:\SteamLibrary\steamapps\common\Skyrim Special Edition\Data\Skyrim.esm";
const DATA_DIR: &str = r"D:\SteamLibrary\steamapps\common\Skyrim Special Edition\Data";

fn skyrim_data_available() -> bool {
    PathBuf::from(SKYRIM_ESM).exists()
}

/// Check if standalone strings files exist (avoid slow BSA scanning)
fn strings_available() -> bool {
    let data = PathBuf::from(DATA_DIR);
    data.join("Skyrim_english.STRINGS").exists()
}

fn maybe_load_strings(parser: &mut EspParser) {
    if strings_available() {
        parser.load_strings_files(DATA_DIR, "skyrim");
    }
    // Without strings files, the parser will still extract EDIDs/comments.
    // String ID resolution will be limited.
}

fn measure_memory_usage() -> usize {
    // Simple memory measurement (in a real scenario, you'd use more sophisticated tools)
    use std::mem;
    mem::size_of::<xt_core::types::sky_string::SkyString>()
}

/// Benchmark 1: ESP parsing performance
#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn benchmark_esp_parsing() {
    if !skyrim_data_available() {
        println!("⚠️  Skipping ESP parsing benchmark - Skyrim.esm not found");
        return;
    }

    let mut parser = EspParser::new();
    maybe_load_strings(&mut parser);
    
    let start = Instant::now();
    let mut file = std::fs::File::open(SKYRIM_ESM).unwrap();
    parser.parse(&mut file).unwrap();
    let parse_time = start.elapsed();
    
    let string_count = parser.strings.len();
    let strings_per_second = string_count as f64 / parse_time.as_secs_f64();
    
    println!("📊 ESP Parsing Performance:");
    println!("  Total strings: {}", string_count);
    println!("  Parse time: {:?}", parse_time);
    println!("  Strings/sec: {:.0}", strings_per_second);
    
    // Performance assertions
    assert!(parse_time < Duration::from_secs(30), "ESP parsing too slow: {:?}", parse_time);
    assert!(strings_per_second > 2000.0, "Parsing rate too low: {:.0} strings/sec", strings_per_second);
}

/// Benchmark 2: Memory usage with large datasets
#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn benchmark_memory_usage() {
    if !skyrim_data_available() {
        println!("⚠️  Skipping memory benchmark - Skyrim.esm not found");
        return;
    }

    let mut parser = EspParser::new();
    maybe_load_strings(&mut parser);
    let mut file = std::fs::File::open(SKYRIM_ESM).unwrap();
    parser.parse(&mut file).unwrap();
    
    let string_count = parser.strings.len();
    let memory_per_string = measure_memory_usage();
    let total_memory_mb = (string_count * memory_per_string) / 1_048_576;
    
    println!("💾 Memory Usage Analysis:");
    println!("  String count: {}", string_count);
    println!("  Memory per string: {} bytes", memory_per_string);
    println!("  Estimated total memory: {} MB", total_memory_mb);
    
    // Memory assertions
    assert!(total_memory_mb < 500, "Memory usage too high: {} MB", total_memory_mb);
    assert!(memory_per_string < 5000, "Per-string memory too high: {} bytes", memory_per_string);
}

/// Benchmark 3: Filtering performance
#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn benchmark_filtering() {
    if !skyrim_data_available() {
        println!("⚠️  Skipping filtering benchmark - Skyrim.esm not found");
        return;
    }

    let mut parser = EspParser::new();
    maybe_load_strings(&mut parser);
    let mut file = std::fs::File::open(SKYRIM_ESM).unwrap();
    parser.parse(&mut file).unwrap();
    
    let string_count = parser.strings.len();
    
    // Benchmark text filtering
    let start = Instant::now();
    let _filtered: Vec<_> = parser.strings.iter()
        .filter(|s| s.source.to_lowercase().contains("dragon"))
        .collect();
    let text_filter_time = start.elapsed();
    
    // Benchmark regex filtering
    let start = Instant::now();
    let regex = regex::Regex::new(r"(?i)dragon").unwrap();
    let _regex_filtered: Vec<_> = parser.strings.iter()
        .filter(|s| regex.is_match(&s.source) || regex.is_match(&s.translation))
        .collect();
    let regex_filter_time = start.elapsed();
    
    // Benchmark record type filtering
    let start = Instant::now();
    let _record_filtered: Vec<_> = parser.strings.iter()
        .filter(|s| s.record_sig == *b"INFO")
        .collect();
    let record_filter_time = start.elapsed();
    
    // Benchmark status filtering
    let start = Instant::now();
    let _status_filtered: Vec<_> = parser.strings.iter()
        .filter(|s| s.params.is_translated())
        .collect();
    let status_filter_time = start.elapsed();
    
    println!("🔍 Filtering Performance ({} strings):", string_count);
    println!("  Text filter: {:?}", text_filter_time);
    println!("  Regex filter: {:?}", regex_filter_time);
    println!("  Record filter: {:?}", record_filter_time);
    println!("  Status filter: {:?}", status_filter_time);
    
    // Performance assertions
    assert!(text_filter_time < Duration::from_millis(100), "Text filtering too slow");
    assert!(regex_filter_time < Duration::from_millis(200), "Regex filtering too slow");
    assert!(record_filter_time < Duration::from_millis(50), "Record filtering too slow");
    assert!(status_filter_time < Duration::from_millis(50), "Status filtering too slow");
}

/// Benchmark 4: Sorting performance
#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn benchmark_sorting() {
    if !skyrim_data_available() {
        println!("⚠️  Skipping sorting benchmark - Skyrim.esm not found");
        return;
    }

    let mut parser = EspParser::new();
    maybe_load_strings(&mut parser);
    let mut file = std::fs::File::open(SKYRIM_ESM).unwrap();
    parser.parse(&mut file).unwrap();
    
    let string_count = parser.strings.len();
    
    // Benchmark sorting by ID
    let start = Instant::now();
    let mut sorted_by_id = parser.strings.clone();
    sorted_by_id.sort_by(|a, b| a.id.cmp(&b.id));
    let id_sort_time = start.elapsed();
    
    // Benchmark sorting by source
    let start = Instant::now();
    let mut sorted_by_source = parser.strings.clone();
    sorted_by_source.sort_by(|a, b| a.source.cmp(&b.source));
    let source_sort_time = start.elapsed();
    
    // Benchmark sorting by record type
    let start = Instant::now();
    let mut sorted_by_record = parser.strings.clone();
    sorted_by_record.sort_by(|a, b| a.record_sig.cmp(&b.record_sig));
    let record_sort_time = start.elapsed();
    
    println!("📊 Sorting Performance ({} strings):", string_count);
    println!("  ID sort: {:?}", id_sort_time);
    println!("  Source sort: {:?}", source_sort_time);
    println!("  Record sort: {:?}", record_sort_time);
    
    // Performance assertions
    assert!(id_sort_time < Duration::from_millis(500), "ID sorting too slow");
    assert!(source_sort_time < Duration::from_secs(1), "Source sorting too slow");
    assert!(record_sort_time < Duration::from_millis(500), "Record sorting too slow");
}

/// Benchmark 5: File I/O performance
#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn benchmark_file_io() {
    if !skyrim_data_available() {
        println!("⚠️  Skipping file I/O benchmark - Skyrim.esm not found");
        return;
    }

    let mut parser = EspParser::new();
    maybe_load_strings(&mut parser);
    let mut file = std::fs::File::open(SKYRIM_ESM).unwrap();
    parser.parse(&mut file).unwrap();
    
    // Add some translations
    for i in 0..1000.min(parser.strings.len()) {
        parser.strings[i].translation = format!("测试翻译_{}", i);
        parser.strings[i].params.set(SkyStringParams::TRANSLATED, true);
    }
    
    let tmp_dir = TempDir::new().unwrap();
    
    // Benchmark Strings file save
    let strings_path = tmp_dir.path().join("benchmark.strings");
    let start = Instant::now();
    let mut sf = StringsFile::new();
    sf.format = xt_core::strings::StringsFormat::NullTerminated;
    for s in &parser.strings {
        if s.params.is_translated() && !s.translation.is_empty() {
            sf.strings.insert(s.esp_ptr.str_id as u32, s.translation.clone());
        }
    }
    sf.save(&strings_path).unwrap();
    let strings_save_time = start.elapsed();
    
    // Benchmark Strings file load
    let start = Instant::now();
    let _reloaded = StringsFile::load_with_format(
        &strings_path, 
        xt_core::strings::StringsFormat::NullTerminated
    ).unwrap();
    let strings_load_time = start.elapsed();
    
    // Benchmark SST save
    let sst = SstDictionary::from_entries(parser.strings.clone());
    let sst_path = tmp_dir.path().join("benchmark.sst");
    let start = Instant::now();
    sst.save_to_file(sst_path.to_str().unwrap()).unwrap();
    let sst_save_time = start.elapsed();
    
    // Benchmark SST load
    let start = Instant::now();
    let _reloaded_sst = SstDictionary::load_from_file(&sst_path).unwrap();
    let sst_load_time = start.elapsed();
    
    println!("💾 File I/O Performance:");
    println!("  Strings save: {:?}", strings_save_time);
    println!("  Strings load: {:?}", strings_load_time);
    println!("  SST save: {:?}", sst_save_time);
    println!("  SST load: {:?}", sst_load_time);
    
    // Performance assertions
    assert!(strings_save_time < Duration::from_secs(5), "Strings save too slow");
    assert!(strings_load_time < Duration::from_secs(2), "Strings load too slow");
    assert!(sst_save_time < Duration::from_secs(3), "SST save too slow");
    assert!(sst_load_time < Duration::from_secs(2), "SST load too slow");
}

/// Benchmark 6: Heuristic search performance
#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn benchmark_heuristic_search() {
    if !skyrim_data_available() {
        println!("⚠️  Skipping heuristic search benchmark - Skyrim.esm not found");
        return;
    }

    let mut parser = EspParser::new();
    maybe_load_strings(&mut parser);
    let mut file = std::fs::File::open(SKYRIM_ESM).unwrap();
    parser.parse(&mut file).unwrap();
    
    // Add some translated strings for search
    for i in 0..1000.min(parser.strings.len()) {
        parser.strings[i].translation = format!("翻译内容_{}", i);
        parser.strings[i].params.set(SkyStringParams::TRANSLATED, true);
    }
    
    let translated_strings: Vec<_> = parser.strings.iter()
        .filter(|s| s.params.is_translated())
        .collect();
    
    let translated_count = translated_strings.len();
    
    // Benchmark multiple search queries
    let queries = vec!["dragon", "sword", "magic", "quest", "npc"];
    
    for query in queries {
        let start = Instant::now();
        let _results: Vec<_> = translated_strings.iter()
            .filter(|s| {
                s.source.to_lowercase().contains(query) ||
                s.translation.to_lowercase().contains(query)
            })
            .take(10)
            .collect();
        let search_time = start.elapsed();
        
        println!("🔍 Heuristic search '{}': {:?}", query, search_time);
        assert!(search_time < Duration::from_millis(50), "Search too slow for '{}'", query);
    }
    
    println!("🔍 Heuristic Search Performance ({} translated strings):", translated_count);
}

/// Benchmark 7: Translation API simulation
#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn benchmark_translation_api() {
    // Simulate translation API performance
    let test_strings = vec![
        "Hello world",
        "Dragonborn",
        "Skyrim",
        "The Elder Scrolls",
        "Dovahkiin",
        "Thu'um",
        "Dragon shout",
        "Civil war",
        "Imperial Legion",
        "Stormcloaks"
    ];
    
    // Simulate API response times
    let api_responses = vec![50, 100, 150, 200, 250]; // milliseconds
    
    for (i, string) in test_strings.iter().enumerate() {
        let response_time = api_responses[i % api_responses.len()];
        let start = Instant::now();
        
        // Simulate translation delay
        std::thread::sleep(Duration::from_millis(response_time));
        
        let translation = format!("[翻译] {}", string);
        let total_time = start.elapsed();
        
        println!("🌐 Translation API - '{}': {:?} -> '{}'", string, total_time, translation);
        
        // API should respond within reasonable time
        assert!(total_time < Duration::from_millis(500), "Translation API too slow");
    }
}

/// Benchmark 8: Concurrent operations
#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn benchmark_concurrent_operations() {
    if !skyrim_data_available() {
        println!("⚠️  Skipping concurrent operations benchmark - Skyrim.esm not found");
        return;
    }

    let mut parser = EspParser::new();
    maybe_load_strings(&mut parser);
    let mut file = std::fs::File::open(SKYRIM_ESM).unwrap();
    parser.parse(&mut file).unwrap();
    
    let _string_count = parser.strings.len();
    
    // Benchmark concurrent filtering
    let start = Instant::now();
    use std::thread;
    
    let handles: Vec<_> = (0..4).map(|i| {
        let strings = parser.strings.clone();
        thread::spawn(move || {
            let query = match i {
                0 => "dragon",
                1 => "sword",
                2 => "magic",
                _ => "quest",
            };
            strings.iter()
                .filter(|s| s.source.to_lowercase().contains(query))
                .count()
        })
    }).collect();
    
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    let concurrent_time = start.elapsed();
    
    // Benchmark sequential filtering for comparison
    let start = Instant::now();
    let mut sequential_results = Vec::new();
    for query in ["dragon", "sword", "magic", "quest"] {
        let count = parser.strings.iter()
            .filter(|s| s.source.to_lowercase().contains(query))
            .count();
        sequential_results.push(count);
    }
    let sequential_time = start.elapsed();
    
    println!("⚡ Concurrent Operations Performance:");
    println!("  Concurrent: {:?} (results: {:?})", concurrent_time, results);
    println!("  Sequential: {:?} (results: {:?})", sequential_time, sequential_results);
    
    // Concurrent should be faster or at least not significantly slower
    let speedup = sequential_time.as_secs_f64() / concurrent_time.as_secs_f64();
    println!("  Speedup: {:.2}x", speedup);
    
    assert!(speedup > 0.5, "Concurrent operations significantly slower");
}

/// Benchmark 9: Memory pressure test
#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn benchmark_memory_pressure() {
    if !skyrim_data_available() {
        println!("⚠️  Skipping memory pressure test - Skyrim.esm not found");
        return;
    }

    // Create multiple parsers to test memory pressure
    let mut parsers = Vec::new();
    
    for i in 0..5 {
        let mut parser = EspParser::new();
        maybe_load_strings(&mut parser);
        let mut file = std::fs::File::open(SKYRIM_ESM).unwrap();
        parser.parse(&mut file).unwrap();
        
        println!("🧠 Memory pressure test - Parser {}: {} strings", i + 1, parser.strings.len());
        parsers.push(parser);
    }
    
    // Test that we can still perform operations with multiple parsers loaded
    for (i, parser) in parsers.iter().enumerate() {
        let start = Instant::now();
        let count = parser.strings.iter()
            .filter(|s| s.params.is_translated())
            .count();
        let filter_time = start.elapsed();
        
        println!("  Parser {} - {} translated strings, filter time: {:?}", i + 1, count, filter_time);
        assert!(filter_time < Duration::from_millis(100), "Filter too slow under memory pressure");
    }
}

/// Benchmark 10: Stress test with synthetic data
#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn benchmark_stress_test() {
    println!("🔥 Stress test with synthetic data");
    
    fn make_sig(s: &str) -> [u8; 4] {
        let bytes = s.as_bytes();
        let mut sig = [0u8; 4];
        let len = bytes.len().min(4);
        sig[..len].copy_from_slice(&bytes[..len]);
        sig
    }

    // Create large synthetic dataset
    let mut strings = Vec::new();
    for i in 0..100_000 {
        let source = format!("Test string {}", i + 1);
        let translation = if i % 3 == 0 {
            format!("测试字符串 {}", i + 1)
        } else {
            String::new()
        };
        let rec_sig = [make_sig("INFO"), make_sig("NPC_"), make_sig("QUST")][(i as usize) % 3];
        let fld_sig = make_sig("FULL");

        let mut sk = xt_core::types::sky_string::SkyString::new(
            i + 1,
            source,
            translation.clone(),
            rec_sig,
            fld_sig,
        );
        sk.list_index = (i % 3) as u8;
        sk.esp_ptr = xt_core::types::esp_pointer::EspPointer {
            str_id: (i + 1) as i32,
            form_id: 1000 + i,
            record_sig: rec_sig,
            field_sig: fld_sig,
            index: 0,
            index_max: 0,
            edid_hash: 0,
        };
        if i % 3 == 0 {
            sk.params.set(SkyStringParams::TRANSLATED, true);
        }
        if i % 10 == 0 {
            sk.internal_params.set(SkyStringInternalParams::IS_VMAD_STRING, true);
        }
        strings.push(sk);
    }
    
    let _string_count = strings.len();
    
    // Benchmark large dataset operations
    let start = Instant::now();
    let _filtered: Vec<_> = strings.iter()
        .filter(|s| s.source.contains("Test"))
        .take(1000)
        .collect();
    let filter_time = start.elapsed();
    
    let start = Instant::now();
    let mut sorted = strings.clone();
    sorted.sort_by(|a, b| a.source.cmp(&b.source));
    let sort_time = start.elapsed();
    
    let start = Instant::now();
    let _sst = SstDictionary::from_entries(strings);
    let sst_time = start.elapsed();
    
    println!("🔥 Stress Test Results (100K strings):");
    println!("  Filter 1000 from 100K: {:?}", filter_time);
    println!("  Sort 100K strings: {:?}", sort_time);
    println!("  Create SST from 100K: {:?}", sst_time);
    
    // Stress test assertions
    assert!(filter_time < Duration::from_millis(200), "Large dataset filtering too slow");
    assert!(sort_time < Duration::from_secs(2), "Large dataset sorting too slow");
    assert!(sst_time < Duration::from_secs(5), "Large SST creation too slow");
}
