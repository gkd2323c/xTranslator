use anyhow::Result;
use std::path::Path;
use xt_core::xml::{parse_xml_file, XmlStringEntry};

/// Compare Rust ESP parsing output with Delphi xTranslator XML export
pub fn diff_esp_with_xml(esp_path: &str, xml_path: &str) -> Result<()> {
    println!("=== Rust ESP vs Delphi XML Diff ===\n");

    // Parse ESP
    let esp_strings = parse_esp_strings(esp_path)?;
    println!("Rust ESP strings: {}", esp_strings.len());

    // Parse XML
    let (_, xml_entries) = parse_xml_file(Path::new(xml_path))?;
    println!("Delphi XML entries: {}", xml_entries.len());

    // Build lookup map for Rust strings
    let mut rust_map: std::collections::HashMap<String, &xt_core::types::sky_string::SkyString> =
        std::collections::HashMap::new();
    for sk in &esp_strings {
        let key = format!(
            "{:06X}:{}:{}",
            sk.esp_ptr.str_id,
            String::from_utf8_lossy(&sk.esp_ptr.record_sig),
            String::from_utf8_lossy(&sk.esp_ptr.field_sig)
        );
        rust_map.insert(key, sk);
    }

    // Compare
    let mut matched = 0;
    let mut missing_in_rust = Vec::new();
    let mut source_mismatch = Vec::new();

    for xml_entry in &xml_entries {
        let key = format!(
            "{:06X}:{}:{}",
            xml_entry.str_id,
            String::from_utf8_lossy(&xml_entry.record_sig),
            String::from_utf8_lossy(&xml_entry.field_sig)
        );

        if let Some(rust_sk) = rust_map.get(&key) {
            matched += 1;
            // Check source string match (case sensitive)
            if rust_sk.source != xml_entry.source {
                source_mismatch.push(DiffEntry {
                    str_id: xml_entry.str_id,
                    record_sig: xml_entry.record_sig,
                    field_sig: xml_entry.field_sig,
                    rust_source: rust_sk.source.clone(),
                    xml_source: xml_entry.source.clone(),
                });
            }
        } else {
            missing_in_rust.push(xml_entry);
        }
    }

    // Report
    println!("\n--- Match Results ---");
    println!("Matched: {}", matched);
    println!("Missing in Rust: {}", missing_in_rust.len());
    println!("Source mismatches: {}", source_mismatch.len());

    if !missing_in_rust.is_empty() {
        println!("\n--- Missing in Rust (first 20) ---");
        for entry in missing_in_rust.iter().take(20) {
            println!(
                "  [{:06X}] {}:{} | {}",
                entry.str_id,
                String::from_utf8_lossy(&entry.record_sig),
                String::from_utf8_lossy(&entry.field_sig),
                entry.source.chars().take(50).collect::<String>()
            );
        }
    }

    if !source_mismatch.is_empty() {
        println!("\n--- Source Mismatches (first 20) ---");
        for entry in source_mismatch.iter().take(20) {
            println!(
                "  [{:06X}] {}:{}",
                entry.str_id,
                String::from_utf8_lossy(&entry.record_sig),
                String::from_utf8_lossy(&entry.field_sig)
            );
            println!(
                "    Rust:   {}",
                entry.rust_source.chars().take(60).collect::<String>()
            );
            println!(
                "    XML:    {}",
                entry.xml_source.chars().take(60).collect::<String>()
            );
        }
    }

    // Calculate match rate
    if !xml_entries.is_empty() {
        let match_rate = 100.0 * matched as f64 / xml_entries.len() as f64;
        println!("\nMatch rate: {:.2}%", match_rate);
    }

    println!("\n===========================");
    Ok(())
}

#[derive(Debug)]
struct DiffEntry {
    str_id: i32,
    record_sig: [u8; 4],
    field_sig: [u8; 4],
    rust_source: String,
    xml_source: String,
}

/// Parse ESP and return strings (simplified for diff)
fn parse_esp_strings(esp_path: &str) -> Result<Vec<xt_core::types::sky_string::SkyString>> {
    use std::fs::File;
    use std::io::BufReader;
    use xt_core::esp::parser::EspParser;

    let path = Path::new(esp_path);
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let mut parser = EspParser::new();

    // Try to load Strings files
    if let Some(parent) = path.parent() {
        let strings_dir = parent.join("Strings");
        if strings_dir.exists() {
            let base_name = path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Skyrim");
            parser.load_strings_files(&strings_dir, base_name);
        }
    }

    parser.parse(&mut reader)?;

    Ok(parser.strings)
}

/// Compare two XML exports (Delphi vs Rust if we implement XML export)
pub fn diff_xml_with_xml(xml1_path: &str, xml2_path: &str) -> Result<()> {
    println!("=== XML vs XML Diff ===\n");

    let (_, entries1) = parse_xml_file(Path::new(xml1_path))?;
    let (_, entries2) = parse_xml_file(Path::new(xml2_path))?;

    println!("XML1 entries: {}", entries1.len());
    println!("XML2 entries: {}", entries2.len());

    // Build key maps
    let mut map1: std::collections::HashMap<String, &XmlStringEntry> = std::collections::HashMap::new();
    let mut map2: std::collections::HashMap<String, &XmlStringEntry> = std::collections::HashMap::new();

    for e in &entries1 {
        let key = format!(
            "{:06X}:{}:{}",
            e.str_id,
            String::from_utf8_lossy(&e.record_sig),
            String::from_utf8_lossy(&e.field_sig)
        );
        map1.insert(key, e);
    }

    for e in &entries2 {
        let key = format!(
            "{:06X}:{}:{}",
            e.str_id,
            String::from_utf8_lossy(&e.record_sig),
            String::from_utf8_lossy(&e.field_sig)
        );
        map2.insert(key, e);
    }

    let mut missing_in_2 = 0;
    let mut translation_diffs = Vec::new();

    for (key, e1) in &map1 {
        if let Some(e2) = map2.get(key) {
            if e1.translation != e2.translation {
                translation_diffs.push(TransDiff {
                    key: key.clone(),
                    source: e1.source.clone(),
                    trans1: e1.translation.clone(),
                    trans2: e2.translation.clone(),
                });
            }
        } else {
            missing_in_2 += 1;
        }
    }

    println!("\n--- Results ---");
    println!("Missing in XML2: {}", missing_in_2);
    println!("Translation differences: {}", translation_diffs.len());

    if !translation_diffs.is_empty() {
        println!("\n--- Translation Diff (first 20) ---");
        for diff in translation_diffs.iter().take(20) {
            println!("  {}", diff.key);
            println!("    Source: {}", diff.source.chars().take(50).collect::<String>());
            println!("    XML1:   {}", diff.trans1.chars().take(50).collect::<String>());
            println!("    XML2:   {}", diff.trans2.chars().take(50).collect::<String>());
        }
    }

    Ok(())
}

#[derive(Debug)]
struct TransDiff {
    key: String,
    source: String,
    trans1: String,
    trans2: String,
}
