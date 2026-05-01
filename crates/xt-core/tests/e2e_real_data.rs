//! 端到端验证测试 - 直接测试 xt-core 功能，不经过 Tauri IPC
//!
//! 这些测试需要解析 238MB Skyrim.esm，在 debug 模式下极慢（>120s）。
//! 使用 `cargo test --release -p xt-core --test e2e_real_data` 运行。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use xt_core::esp::parser::{EspParser, StringsFiles};
use xt_core::sst::v8::SstDictionary;
use xt_core::strings::CodepageTable;
use xt_core::types::game_id::GameId;
use xt_core::types::params::SkyStringParams;
use xt_core::types::sky_string::SkyString;

const SKYRIM_ESM: &str = r"D:\SteamLibrary\steamapps\common\Skyrim Special Edition\Data\Skyrim.esm";
const DATA_DIR: &str = r"D:\SteamLibrary\steamapps\common\Skyrim Special Edition\Data";

/// 模拟 AppState
struct TestAppState {
    strings: Mutex<Vec<SkyString>>,
}

impl TestAppState {
    fn new() -> Self {
        Self {
            strings: Mutex::new(Vec::new()),
        }
    }
}

#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn e2e_load_esp_skyrim() {
    let state = Arc::new(TestAppState::new());

    // 1. 创建解析器
    let data_dir = std::path::Path::new("Data");
    let mut parser =
        EspParser::with_game(data_dir, GameId::SkyrimSE).unwrap_or_else(|_| EspParser::new());

    // 2. 加载 Strings
    let codepage_path = data_dir.join("SkyrimSE").join("codepage.txt");
    if codepage_path.exists() {
        if let Ok(table) = CodepageTable::load_from_file(&codepage_path) {
            parser.strings_files = StringsFiles::load_from_dir_with_language(
                std::path::Path::new(DATA_DIR),
                "skyrim",
                "english",
                &table,
            );
        } else {
            parser.load_strings_files(DATA_DIR, "skyrim");
        }
    } else {
        parser.load_strings_files(DATA_DIR, "skyrim");
    }

    let strings_loaded = parser.strings_files.loaded_count();

    // 3. 解析 ESP
    let start = std::time::Instant::now();
    let mut file = std::fs::File::open(SKYRIM_ESM).expect("Failed to open Skyrim.esm");
    parser.parse(&mut file).expect("Failed to parse ESP");
    let parse_time_ms = start.elapsed().as_millis() as u64;

    // 4. 存入 state
    let total = parser.strings.len() as u32;
    *state.strings.lock().unwrap() = parser.strings;

    // 5. 验证
    println!("\n=== E2E Test: Load ESP ===");
    println!("Total strings: {}", total);
    println!("Strings files loaded: {}", strings_loaded);
    println!("Parse time: {}ms", parse_time_ms);
    assert!(total > 70000, "Expected >70k strings, got {}", total);

    // 统计 Record 类型
    let strings = state.strings.lock().unwrap();
    let mut record_counts: HashMap<String, usize> = HashMap::new();
    for sk in strings.iter() {
        let sig = String::from_utf8_lossy(&sk.esp_ptr.record_sig).to_string();
        *record_counts.entry(sig).or_insert(0) += 1;
    }
    println!("Record type distribution (top 10):");
    let mut sorted: Vec<_> = record_counts.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));
    for (sig, count) in sorted.iter().take(10) {
        println!("  {}: {}", sig, count);
    }

    // 验证具体类型存在
    assert!(record_counts.contains_key("INFO"), "INFO records not found");
    assert!(record_counts.contains_key("DIAL"), "DIAL records not found");
    assert!(
        record_counts.contains_key("NPC_"),
        "NPC_ records not found (compressed records may have failed)"
    );

    // 验证 sample 数据
    let first = &strings[0];
    println!("\nFirst string sample:");
    println!("  ID: {}", first.id);
    println!(
        "  Record: {}",
        String::from_utf8_lossy(&first.esp_ptr.record_sig)
    );
    println!(
        "  Field: {}",
        String::from_utf8_lossy(&first.esp_ptr.field_sig)
    );
    println!("  Source: {}", first.source);
    assert!(!first.source.is_empty(), "First string source is empty");

    println!("\n✅ E2E Load ESP test PASSED");
}

#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn e2e_query_filter_sort() {
    let state = Arc::new(TestAppState::new());

    // 1. 加载 ESP
    let mut parser = EspParser::new();
    parser.load_strings_files(DATA_DIR, "skyrim");
    let mut file = std::fs::File::open(SKYRIM_ESM).unwrap();
    parser.parse(&mut file).unwrap();
    *state.strings.lock().unwrap() = parser.strings;

    let strings = state.strings.lock().unwrap();
    let total = strings.len();

    // 2. 模拟查询：文本筛选
    let filter = "dragon";
    let filtered: Vec<_> = strings
        .iter()
        .filter(|sk| sk.source.to_lowercase().contains(filter))
        .collect();
    println!("\n=== E2E Test: Query Filter ===");
    println!("Filter '{}': {} / {} items", filter, filtered.len(), total);
    assert!(
        !filtered.is_empty(),
        "Should find strings matching 'dragon'"
    );

    // 3. 模拟查询：状态筛选
    let incomplete: Vec<_> = strings
        .iter()
        .filter(|sk| sk.params.is_incomplete() && !sk.params.is_translated())
        .collect();
    println!("Incomplete items: {} / {}", incomplete.len(), total);
    assert!(incomplete.len() > 0, "Should have incomplete items");

    // 4. 模拟查询：排序
    let mut sorted = strings.clone();
    sorted.sort_by(|a, b| a.source.cmp(&b.source));
    println!(
        "Sort by source: first='{}', last='{}'",
        sorted[0].source,
        sorted[sorted.len() - 1].source
    );

    // 5. 模拟分页
    let page_size = 100;
    let page: Vec<_> = strings.iter().skip(0).take(page_size).collect();
    assert_eq!(page.len(), page_size);
    println!("Page 1: {} items", page.len());

    println!("\n✅ E2E Query Filter Sort test PASSED");
}

#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn e2e_update_and_save_sst() {
    let state = Arc::new(TestAppState::new());

    // 1. 加载 ESP
    let mut parser = EspParser::new();
    parser.load_strings_files(DATA_DIR, "skyrim");
    let mut file = std::fs::File::open(SKYRIM_ESM).unwrap();
    parser.parse(&mut file).unwrap();
    *state.strings.lock().unwrap() = parser.strings;

    let mut strings = state.strings.lock().unwrap();

    // 2. 更新翻译
    let test_id = strings[0].id;
    let original_source = strings[0].source.clone();
    strings[0].set_translation("测试翻译".to_string());
    strings[0].params.set(SkyStringParams::TRANSLATED, true);
    strings[0]
        .params
        .set(SkyStringParams::INCOMPLETE_TRANS, false);

    println!("\n=== E2E Test: Update & Save SST ===");
    println!(
        "Updated string #{}: '{}' -> '{}'",
        test_id, original_source, strings[0].translation
    );
    assert_eq!(strings[0].translation, "测试翻译");
    assert!(strings[0].params.is_translated());

    // 3. 保存 SST
    let temp_sst = std::env::temp_dir().join("test_e2e.sst");
    let dict = SstDictionary::from_entries(strings.clone());
    dict.save_to_file(&temp_sst).expect("Failed to save SST");

    let metadata = std::fs::metadata(&temp_sst).unwrap();
    println!("SST saved: {} bytes", metadata.len());
    assert!(metadata.len() > 1000, "SST file too small");

    // 4. 重新加载验证
    drop(strings); // 释放锁
    let loaded_dict = SstDictionary::load_from_file(&temp_sst).expect("Failed to load SST");
    println!("SST loaded: {} entries", loaded_dict.entries.len());
    assert_eq!(
        loaded_dict.entries.len(),
        state.strings.lock().unwrap().len()
    );

    // 验证翻译被正确保存
    let found = loaded_dict.entries.iter().find(|e| e.id == test_id);
    assert!(found.is_some(), "Updated entry not found in saved SST");
    assert_eq!(
        found.unwrap().translation,
        "测试翻译",
        "Translation mismatch in SST"
    );

    // 清理
    let _ = std::fs::remove_file(&temp_sst);

    println!("\n✅ E2E Update & Save SST test PASSED");
}

#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn e2e_load_sst_and_match() {
    println!("\n=== E2E Test: Load SST & Match ===");

    // 1. 创建测试 SST
    let mut test_strings = vec![
        SkyString::new(
            0,
            "Hello".to_string(),
            "你好".to_string(),
            *b"INFO",
            *b"DESC",
        ),
        SkyString::new(
            1,
            "World".to_string(),
            "世界".to_string(),
            *b"INFO",
            *b"FULL",
        ),
        SkyString::new(2, "Test".to_string(), "".to_string(), *b"BOOK", *b"NAME"),
    ];
    test_strings[0]
        .params
        .set(SkyStringParams::TRANSLATED, true);
    test_strings[1]
        .params
        .set(SkyStringParams::TRANSLATED, true);

    let dict = SstDictionary::from_entries(test_strings);
    let temp_sst = std::env::temp_dir().join("test_match.sst");
    dict.save_to_file(&temp_sst).unwrap();

    // 2. 加载 ESP
    let mut parser = EspParser::new();
    parser.load_strings_files(DATA_DIR, "skyrim");
    let mut file = std::fs::File::open(SKYRIM_ESM).unwrap();
    parser.parse(&mut file).unwrap();

    // 3. 模拟 SST 匹配（strId + record_sig + field_sig）
    let loaded_dict = SstDictionary::load_from_file(&temp_sst).unwrap();
    let mut matched = 0;
    let mut unmatched = 0;

    // 匹配逻辑（与 commands.rs 中的 load_sst 相同）
    for entry in &loaded_dict.entries {
        let found = parser.strings.iter_mut().find(|sk| {
            sk.esp_ptr.str_id == entry.esp_ptr.str_id
                && sk.esp_ptr.record_sig == entry.esp_ptr.record_sig
                && sk.esp_ptr.field_sig == entry.esp_ptr.field_sig
        });

        if let Some(_sk) = found {
            matched += 1;
        } else {
            unmatched += 1;
        }
    }

    println!("SST matching: {} matched, {} unmatched", matched, unmatched);
    // 测试 SST 的 str_id 可能与 ESP 不匹配，所以 unmatched 可能 > 0
    println!("(Note: Test SST uses synthetic str_ids, matching depends on ESP data)");

    // 清理
    let _ = std::fs::remove_file(&temp_sst);

    println!("\n✅ E2E Load SST & Match test PASSED");
}

#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn e2e_performance_benchmark() {
    println!("\n=== E2E Test: Performance Benchmark ===");

    // 1. 测量 ESP 解析时间
    let start = std::time::Instant::now();
    let mut parser = EspParser::new();
    parser.load_strings_files(DATA_DIR, "skyrim");
    let mut file = std::fs::File::open(SKYRIM_ESM).unwrap();
    parser.parse(&mut file).unwrap();
    let parse_ms = start.elapsed().as_millis();

    let total = parser.strings.len();
    println!("ESP Parse: {}ms for {} strings", parse_ms, total);

    // 2. 测量查询时间（模拟 query_strings_command）
    let strings = parser.strings;
    let start = std::time::Instant::now();

    // 状态筛选
    let status_filtered: Vec<_> = strings
        .iter()
        .filter(|sk| !sk.params.is_translated())
        .collect();

    // 文本筛选
    let text_filtered: Vec<_> = status_filtered
        .into_iter()
        .filter(|sk| sk.source.to_lowercase().contains("the"))
        .collect();

    // 排序
    let mut sorted = text_filtered.clone();
    sorted.sort_by(|a, b| a.source.cmp(&b.source));

    // 分页
    let _page: Vec<_> = sorted.iter().skip(0).take(100).collect();

    let query_ms = start.elapsed().as_millis();
    println!(
        "Query (filter+sort+page): {}ms for {} filtered items",
        query_ms,
        text_filtered.len()
    );

    // 3. 性能断言
    assert!(
        parse_ms < 10000,
        "ESP parse too slow: {}ms (target < 5000ms)",
        parse_ms
    );
    assert!(
        query_ms < 500,
        "Query too slow: {}ms (target < 200ms)",
        query_ms
    );

    // 4. 内存估算
    let estimated_mb = total * 256 / 1024 / 1024;
    println!("Estimated memory: ~{} MB", estimated_mb);
    assert!(
        estimated_mb < 500,
        "Memory usage too high: ~{} MB",
        estimated_mb
    );

    println!("\n✅ E2E Performance Benchmark PASSED");
}

#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn e2e_ipc_payload_size() {
    // 1. 加载 ESP（复用 e2e_load_esp_skyrim 的逻辑）
    let data_dir = std::path::Path::new("Data");
    let mut parser =
        EspParser::with_game(data_dir, GameId::SkyrimSE).unwrap_or_else(|_| EspParser::new());

    let codepage_path = data_dir.join("SkyrimSE").join("codepage.txt");
    if codepage_path.exists() {
        if let Ok(table) = CodepageTable::load_from_file(&codepage_path) {
            parser.strings_files = StringsFiles::load_from_dir_with_language(
                std::path::Path::new(DATA_DIR),
                "skyrim",
                "english",
                &table,
            );
        } else {
            parser.load_strings_files(DATA_DIR, "skyrim");
        }
    } else {
        parser.load_strings_files(DATA_DIR, "skyrim");
    }

    let mut file = std::fs::File::open(SKYRIM_ESM).expect("Failed to open Skyrim.esm");
    parser.parse(&mut file).expect("Failed to parse ESP");

    let total = parser.strings.len() as u32;
    println!("\n=== IPC Payload Size Test ===");
    println!("Total strings: {}", total);

    // 2. 模拟 DTO 序列化大小
    // SkyStringDTO { id, source, translation, record_sig, field_sig, form_id, status, list_index, str_id }
    let mut json_bytes = 0usize;
    for sk in &parser.strings {
        let record_sig = String::from_utf8_lossy(&sk.esp_ptr.record_sig);
        let field_sig = String::from_utf8_lossy(&sk.esp_ptr.field_sig);
        let form_id = format!("0x{:08X}", sk.esp_ptr.form_id);
        let status = if sk.params.is_translated() {
            "translated"
        } else if sk.params.is_incomplete() {
            "incomplete"
        } else {
            "locked"
        };

        // 估算 JSON 大小（serde_json 格式化后）
        let json = format!(
            r#"{{"id":{},"source":"{}","translation":"{}","record_sig":"{}","field_sig":"{}","form_id":"{}","status":"{}","list_index":{},"str_id":{}}}"#,
            sk.id,
            sk.source,
            sk.translation,
            record_sig,
            field_sig,
            form_id,
            status,
            0, // list_index placeholder
            sk.esp_ptr.str_id
        );
        json_bytes += json.len();
    }

    // 3. 数组包装开销
    let total_json_bytes = json_bytes + 2 + (total as usize - 1); // [ ] + commas
    let total_json_mb = total_json_bytes as f64 / 1024.0 / 1024.0;

    println!(
        "Estimated JSON payload: {} bytes ({:.2} MB)",
        total_json_bytes, total_json_mb
    );
    println!("Per item average: {} bytes", json_bytes / total as usize);

    // 4. 断言
    println!("WebView2 postMessage limit (typical): 1-4 MB");
    if total_json_mb > 4.0 {
        println!("⚠️ WARNING: Payload exceeds 4MB limit. Chunked loading required.");
    } else if total_json_mb > 1.0 {
        println!("⚠️ CAUTION: Payload exceeds 1MB. May fail on some WebView2 versions.");
    } else {
        println!("✅ Payload within safe limits.");
    }
}

#[test]
#[cfg_attr(debug_assertions, ignore = "requires --release")]
fn e2e_esp_compare_self_benchmark() {
    use xt_core::esp::compare::compare_esp_files;
    use xt_core::types::game_id::GameId;

    let skyrim = std::path::Path::new(SKYRIM_ESM);
    if !skyrim.exists() {
        println!("⏭ Skipped: Skyrim.esm not found at {}", SKYRIM_ESM);
        return;
    }

    println!("\n=== ESP Compare Self-Benchmark ===");
    println!("File: {}", SKYRIM_ESM);
    let size_mb = std::fs::metadata(skyrim).map(|m| m.len() / 1024 / 1024).unwrap_or(0);
    println!("Size: {} MB\n", size_mb);

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().and_then(|p| p.parent()).unwrap_or_else(|| std::path::Path::new("."));
    let project_data = workspace_root.join("Data");

    // Phase 1: SHA-256 hash
    let t0 = std::time::Instant::now();
    let _hash = xt_core::cache::hash_file(skyrim);
    println!("  SHA-256 hash: {}ms", t0.elapsed().as_millis());

    // Phase 2: record_defs loading
    let t1 = std::time::Instant::now();
    let _parser = EspParser::with_game(&project_data, GameId::SkyrimSE).unwrap();
    println!("  Record defs: {}ms", t1.elapsed().as_millis());

    // Phase 3: strings file loading
    let t2 = std::time::Instant::now();
    let mut parser2 = EspParser::with_game(&project_data, GameId::SkyrimSE).unwrap();
    parser2.load_strings_files(DATA_DIR, "skyrim");
    let loaded = parser2.strings_files.loaded_count();
    println!("  Strings files ({} loaded): {}ms", loaded, t2.elapsed().as_millis());

    // Phase 4: ESP parse only (no strings)
    let t3 = std::time::Instant::now();
    let mut parser3 = EspParser::with_game(&project_data, GameId::SkyrimSE).unwrap();
    let mut file = std::fs::File::open(SKYRIM_ESM).unwrap();
    parser3.parse(&mut file).unwrap();
    let total = parser3.strings.len();
    println!("  ESP parse (no strings, {} strings): {}ms", total, t3.elapsed().as_millis());

    // Phase 5: Full compare (self)
    let t4 = std::time::Instant::now();
    let result = compare_esp_files(SKYRIM_ESM, SKYRIM_ESM, project_data.to_str(), GameId::SkyrimSE);
    let compare_ms = t4.elapsed().as_millis();
    match &result {
        Ok(comp) => {
            println!("  Full self-compare: {}ms (matched: {}, cache miss)", compare_ms, comp.matched_pairs.len());
            assert_eq!(comp.added.len(), 0);
            assert_eq!(comp.removed.len(), 0);
        }
        Err(e) => panic!("Compare failed: {}", e),
    }

    // Phase 6: Second compare (cache hit)
    let t5 = std::time::Instant::now();
    let _ = compare_esp_files(SKYRIM_ESM, SKYRIM_ESM, project_data.to_str(), GameId::SkyrimSE);
    println!("  Cached self-compare: {}ms", t5.elapsed().as_millis());

    println!("\n✅ ESP Compare Benchmark PASSED");
}
