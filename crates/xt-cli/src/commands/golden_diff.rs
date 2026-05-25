//! Delphi vs Rust 交叉验证工具
//!
//! 对比 Rust ESP 解析与导出输出同 Delphi 黄金参考文件的差异。
//! 为每种数据格式生成结构化的 diff 报告。

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use xt_core::cache::hash_file;
use xt_core::esp::parser::EspParser;
use xt_core::sqlite_cache::SqliteCache;
use xt_core::strings::{StringsFile, StringsFormat};
use xt_core::xml::parse_xml_file;

/// 运行完整的黄金对比测试：解析 ESP 并同 Delphi 参考文件进行对比。
pub fn run_golden_diff(delphi_dir: &str, esp_path: &str) -> Result<()> {
    let delphi_dir = Path::new(delphi_dir);
    let esp_path = Path::new(esp_path);

    if !delphi_dir.exists() {
        anyhow::bail!(
            "Delphi golden directory not found: {}",
            delphi_dir.display()
        );
    }
    if !esp_path.exists() {
        anyhow::bail!("ESP file not found: {}", esp_path.display());
    }

    println!("=== xTranslator Golden Diff ===");
    println!("Rust ESP:    {}", esp_path.display());
    println!("Delphi dir:  {}", delphi_dir.display());
    println!();

    // 步骤 1: 使用 Rust 解析 ESP（使用缓存）
    println!("[1/5] Parsing ESP with Rust (cached)...");
    let esp_strings = parse_esp_cached(esp_path)?;
    println!("  => {} strings extracted from ESP", esp_strings.len());

    // 步骤 2: 加载 Delphi XML 与 SST 统计数据
    println!("[2/5] Reading Delphi golden files...");

    // 步骤 3: 对比 XML 导出
    println!("[3/5] Comparing XML exports...");
    let xml_result = compare_xml(&esp_strings, delphi_dir)?;

    // 步骤 4: 对比 SST 文件（如果存在）
    println!("[4/5] Comparing SST files...");
    let sst_result = compare_sst(delphi_dir)?;

    // Step 5: Summary report
    println!("[5/5] Generating summary...");
    print_summary(&xml_result, &sst_result);

    Ok(())
}

// ── ESP parsing ────────────────────────────────────────────────────────

/// 使用 SQLite 缓存解析 ESP（与 Tauri 应用所使用的缓存相同）。
fn parse_esp_cached(esp_path: &Path) -> Result<Vec<xt_core::types::sky_string::SkyString>> {
    let cache_dir = if cfg!(windows) {
        std::env::var("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_default()
            .join("xTranslator")
            .join("cache")
    } else {
        PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()))
            .join(".cache")
            .join("xTranslator")
    };
    let _ = fs::create_dir_all(&cache_dir);
    let cache = SqliteCache::new(cache_dir);

    // 首先尝试 SQLite 缓存
    let hash = hash_file(esp_path)?;
    if let Some(payload) = cache.lookup(&hash) {
        println!("  (using SQLite cache: {} strings)", payload.strings.len());
        return Ok(payload.strings);
    }

    // 缓存未命中 - 从头解析
    println!("  (cache miss, parsing... this may take a while)");
    let file = fs::File::open(esp_path)?;
    let mut reader = BufReader::new(file);
    let mut parser = EspParser::new();

    if let Some(parent) = esp_path.parent() {
        let strings_dir = parent.join("Strings");
        if strings_dir.exists() {
            let base_name = esp_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Skyrim");
            parser.load_strings_files(&strings_dir, base_name);
        }
    }

    parser.parse(&mut reader)?;

    Ok(parser.strings)
}

// ── XML comparison ─────────────────────────────────────────────────────

#[derive(Debug, Default)]
struct XmlDiffResult {
    rust_count: usize,
    delphi_count: usize,
    matched: usize,
    only_in_rust: Vec<String>,
    only_in_delphi: Vec<String>,
    source_mismatches: Vec<SourceMismatch>,
}

#[derive(Debug)]
struct SourceMismatch {
    key: String,
    rust_source: String,
    delphi_source: String,
}

fn compare_xml(
    esp_strings: &[xt_core::types::sky_string::SkyString],
    delphi_dir: &Path,
) -> Result<XmlDiffResult> {
    let mut result = XmlDiffResult::default();

    let xml_path = find_file_with_ext(delphi_dir, "xml").with_context(|| {
        format!(
            "No XML golden file found in {}. Expected skyrim_se_export.xml",
            delphi_dir.display()
        )
    })?;

    let (_, delphi_entries) = parse_xml_file(&xml_path)?;
    result.delphi_count = delphi_entries.len();
    println!("  Delphi XML: {} entries", result.delphi_count);

    // 构建 Rust 查找表: "strId:recordSIG:fieldSIG" -> SkyString
    let mut rust_map: HashMap<String, &xt_core::types::sky_string::SkyString> = HashMap::new();
    for sk in esp_strings {
        let key = format!(
            "{:06X}:{}:{}",
            sk.esp_ptr.str_id,
            String::from_utf8_lossy(&sk.esp_ptr.record_sig),
            String::from_utf8_lossy(&sk.esp_ptr.field_sig)
        );
        rust_map.insert(key, sk);
    }
    result.rust_count = rust_map.len();

    let mut delphi_keys: HashMap<String, &xt_core::xml::XmlStringEntry> = HashMap::new();
    for e in &delphi_entries {
        let key = format!(
            "{:06X}:{}:{}",
            e.str_id,
            String::from_utf8_lossy(&e.record_sig),
            String::from_utf8_lossy(&e.field_sig)
        );
        delphi_keys.insert(key, e);
    }

    for (key, de) in &delphi_keys {
        if let Some(rk) = rust_map.get(key) {
            result.matched += 1;

            if rk.source != de.source {
                result.source_mismatches.push(SourceMismatch {
                    key: key.clone(),
                    rust_source: rk.source.clone(),
                    delphi_source: de.source.clone(),
                });
            }
        } else {
            result.only_in_delphi.push(key.clone());
        }
    }

    for key in rust_map.keys() {
        if !delphi_keys.contains_key(key) {
            result.only_in_rust.push(key.clone());
        }
    }

    Ok(result)
}

// ── SST comparison ─────────────────────────────────────────────────────

#[derive(Debug, Default)]
struct SstDiffResult {
    status: String,
    rust_entry_count: usize,
}

fn compare_sst(delphi_dir: &Path) -> Result<SstDiffResult> {
    let mut result = SstDiffResult::default();

    if let Some(sst_path) = find_file_with_ext(delphi_dir, "sst") {
        let file = fs::File::open(&sst_path)?;
        let mut reader = BufReader::new(file);
        match xt_core::sst::v8::SstDictionary::read_from(&mut reader) {
            Ok(sst) => {
                result.rust_entry_count = sst.entries.len();
                result.status = format!(
                    "Rust can read {} entries from Delphi SST",
                    result.rust_entry_count
                );
            }
            Err(e) => {
                result.status = format!("WARNING: Rust failed to parse Delphi SST: {}", e);
            }
        }
    } else {
        result.status = "No Delphi SST golden file found (skip)".to_string();
    }

    Ok(result)
}

// ── Strings files comparison ───────────────────────────────────────────

#[derive(Debug, Default)]
#[allow(dead_code)]
struct StringsDiffResult {
    status: String,
    formats_compared: Vec<String>,
    total_matched: usize,
    total_mismatched: usize,
}

#[allow(dead_code)]
fn compare_strings_files(
    esp_strings: &[xt_core::types::sky_string::SkyString],
    delphi_dir: &Path,
) -> Result<StringsDiffResult> {
    let mut result = StringsDiffResult::default();

    let formats = [
        ("strings", StringsFormat::NullTerminated, 0u8),
        ("dlstrings", StringsFormat::LengthPrefixed, 1u8),
        ("ilstrings", StringsFormat::LengthPrefixed, 2u8),
    ];

    for (ext, format, list_index) in &formats {
        let delphi_file = find_file_with_ext(delphi_dir, ext);

        if let Some(ref_path) = delphi_file {
            match StringsFile::load_with_format(&ref_path, *format) {
                Ok(delphi_strs) => {
                    result.formats_compared.push(ext.to_string());

                    // 从 Delphi 构建映射表: strId -> source
                    let delphi_map: HashMap<u32, String> = delphi_strs
                        .strings
                        .iter()
                        .map(|(&id, src)| (id, src.clone()))
                        .collect();

                    let mut matched = 0usize;
                    let mut mismatched = 0usize;

                    for sk in esp_strings.iter().filter(|s| s.list_index == *list_index) {
                        let sid = sk.esp_ptr.str_id.max(0) as u32;
                        if let Some(delphi_source) = delphi_map.get(&sid) {
                            if &sk.source == delphi_source {
                                matched += 1;
                            } else {
                                mismatched += 1;
                            }
                        }
                    }

                    result.total_matched += matched;
                    result.total_mismatched += mismatched;
                }
                Err(e) => {
                    result.status = format!("WARNING: Failed to parse Delphi .{} file: {}", ext, e);
                }
            }
        }
    }

    if result.formats_compared.is_empty() {
        result.status = "No Delphi Strings golden files found (skip)".to_string();
    } else {
        result.status = format!(
            "Compared {}: {} matched, {} mismatched",
            result.formats_compared.join(", "),
            result.total_matched,
            result.total_mismatched
        );
    }

    Ok(result)
}

// ── Summary ────────────────────────────────────────────────────────────

fn print_summary(xml: &XmlDiffResult, sst: &SstDiffResult) {
    println!();
    println!("═══════════════════════════════════════════════");
    println!("  xTranslator Cross-Validation Summary");
    println!("═══════════════════════════════════════════════");
    println!();

    // XML section
    println!("── XML Export ──");
    println!(
        "  Entry counts:    Rust={}, Delphi={}",
        xml.rust_count, xml.delphi_count
    );
    let xml_match_pct = if xml.delphi_count > 0 {
        100.0 * xml.matched as f64 / xml.delphi_count as f64
    } else {
        0.0
    };
    println!("  Matched:         {} ({:.1}%)", xml.matched, xml_match_pct);
    println!("  Only in Rust:    {}", xml.only_in_rust.len());
    println!("  Only in Delphi:  {}", xml.only_in_delphi.len());
    println!("  Source diff:     {}", xml.source_mismatches.len());

    if !xml.source_mismatches.is_empty() {
        println!();
        println!("  ── Top source mismatches (max 10) ──");
        for m in xml.source_mismatches.iter().take(10) {
            println!("    Key: {}", m.key);
            println!("      Rust:   {}", truncate(&m.rust_source, 60));
            println!("      Delphi: {}", truncate(&m.delphi_source, 60));
        }
        if xml.source_mismatches.len() > 10 {
            println!("    ... and {} more", xml.source_mismatches.len() - 10);
        }
    }

    if !xml.only_in_delphi.is_empty() {
        println!();
        println!("  ── Entries only in Delphi (max 10) ──");
        for key in xml.only_in_delphi.iter().take(10) {
            println!("    {}", key);
        }
        if xml.only_in_delphi.len() > 10 {
            println!("    ... and {} more", xml.only_in_delphi.len() - 10);
        }
    }

    if !xml.only_in_rust.is_empty() {
        println!();
        println!("  ── Entries only in Rust (max 10) ──");
        for key in xml.only_in_rust.iter().take(10) {
            println!("    {}", key);
        }
        if xml.only_in_rust.len() > 10 {
            println!("    ... and {} more", xml.only_in_rust.len() - 10);
        }
    }

    println!();
    println!("── SST ──");
    println!("  {}", sst.status);
    println!();

    // Verdict
    println!("── Verdict ──");
    let total_issues =
        xml.source_mismatches.len() + xml.only_in_delphi.len() + xml.only_in_rust.len();
    if xml.delphi_count == 0 {
        println!("  WARN: No Delphi golden files found.");
        println!("  Run Delphi xTranslator 1.6.0 and export reference files:");
        println!("  See tests/fixtures/delphi_golden/README.md for instructions.");
    } else if xml_match_pct >= 99.0 && total_issues == 0 {
        println!("  PASS: >= 99% match with zero issues. Compatible!");
    } else if xml_match_pct >= 95.0 {
        println!(
            "  WARN: {:.1}% match, {} issues found. Review diffs above.",
            xml_match_pct, total_issues
        );
    } else {
        println!(
            "  FAIL: {:.1}% match, {} issues found. Significant mismatch.",
            xml_match_pct, total_issues
        );
    }
    println!("═══════════════════════════════════════════════");
}

// ── Helpers ────────────────────────────────────────────────────────────

fn find_file_with_ext(dir: &Path, ext: &str) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path
            .extension()
            .and_then(|s| s.to_str())
            .map(|e| e.eq_ignore_ascii_case(ext))
            .unwrap_or(false)
        {
            return Some(path);
        }
    }
    None
}

fn truncate(s: &str, max_len: usize) -> String {
    let chars: Vec<char> = s.chars().take(max_len).collect();
    let mut result: String = chars.into_iter().collect();
    if s.chars().count() > max_len {
        result.push_str("...");
    }
    result
}
