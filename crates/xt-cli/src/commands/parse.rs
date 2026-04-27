use anyhow::Result;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;
use xt_core::esp::parser::EspParser;
use xt_core::types::params::SkyStringParams;
use xt_core::xml::{sky_strings_to_xml_entries, write_xml_export, XmlExportParams};

pub fn parse_esp(input: &str, output: Option<&str>) -> Result<()> {
    let path = Path::new(input);
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    println!("Parsing: {}", input);
    let start = std::time::Instant::now();

    let mut parser = EspParser::new();

    // 尝试加载同目录下的 Strings 子目录（用于把 str_id 反查为可读文本）。
    if let Some(parent) = path.parent() {
        let strings_dir = parent.join("Strings");
        if strings_dir.exists() {
            // 以 ESP 文件名（去扩展名）作为 strings 基名。
            let base_name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Skyrim");

            parser.load_strings_files(&strings_dir, base_name);

            let loaded = parser.strings_files.loaded_count();
            if loaded > 0 {
                println!(
                    "  Loaded {}/3 strings files from: {}",
                    loaded,
                    strings_dir.display()
                );
            }
        }
    }

    parser.parse(&mut reader)?;

    let elapsed = start.elapsed();

    println!("\n========== Parse Result ==========");
    println!("Time: {:.2}s", elapsed.as_secs_f64());
    println!("Total strings extracted: {}", parser.strings.len());

    // 统计记录类型与字段分布，便于快速判断解析覆盖情况。
    let mut rec_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut field_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for sk in &parser.strings {
        let rec = String::from_utf8_lossy(&sk.esp_ptr.record_sig).to_string();
        let fld = String::from_utf8_lossy(&sk.esp_ptr.field_sig).to_string();
        *rec_counts.entry(rec.clone()).or_insert(0) += 1;
        *field_counts.entry(format!("{}:{}", rec, fld)).or_insert(0) += 1;
    }

    println!("\nRecord type distribution (top 20):");
    let mut rec_vec: Vec<_> = rec_counts.iter().collect();
    rec_vec.sort_by(|a, b| b.1.cmp(a.1));
    for (rec, count) in rec_vec.iter().take(20) {
        println!("  {}: {}", rec, count);
    }

    println!("\nField type distribution (top 20):");
    let mut fld_vec: Vec<_> = field_counts.iter().collect();
    fld_vec.sort_by(|a, b| b.1.cmp(a.1));
    for (fld, count) in fld_vec.iter().take(20) {
        println!("  {}: {}", fld, count);
    }

    // 可选导出 TSV（便于脚本后处理）。
    if let Some(out) = output {
        let mut lines = Vec::new();
        for sk in &parser.strings {
            lines.push(format!(
                "{}\t{}\t{}\t{}\t{}",
                sk.esp_ptr.str_id,
                String::from_utf8_lossy(&sk.esp_ptr.record_sig),
                String::from_utf8_lossy(&sk.esp_ptr.field_sig),
                sk.source.replace('\t', " ").replace('\n', "\\n"),
                sk.translation.replace('\t', " ").replace('\n', "\\n")
            ));
        }
        std::fs::write(out, lines.join("\n"))?;
        println!("\nExported {} strings to {}", parser.strings.len(), out);
    }

    // 打印前 20 条样本，快速确认文本是否正常解码。
    println!("\nSample strings (first 20):");
    for sk in parser.strings.iter().take(20) {
        println!(
            "  [{}] {}:{} | {} | {} chars",
            sk.esp_ptr.str_id,
            String::from_utf8_lossy(&sk.esp_ptr.record_sig),
            String::from_utf8_lossy(&sk.esp_ptr.field_sig),
            sk.source.chars().take(80).collect::<String>(),
            sk.source.len()
        );
    }

    println!("===================================\n");

    Ok(())
}

/// 解析 ESP 并应用 SST 字典（命令行验证流程）。
pub fn apply_sst(esp_path: &str, sst_path: &str, output: Option<&str>) -> Result<()> {
    let path = Path::new(esp_path);
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    println!("Parsing: {} with SST: {}", esp_path, sst_path);
    let start = std::time::Instant::now();

    let mut parser = EspParser::new();

    // 加载 strings 侧文件，避免输出中只有 <ID:xxxx> 占位文本。
    if let Some(parent) = path.parent() {
        let strings_dir = parent.join("Strings");
        if strings_dir.exists() {
            let base_name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Skyrim");
            parser.load_strings_files(&strings_dir, base_name);
            let loaded = parser.strings_files.loaded_count();
            if loaded > 0 {
                println!("  Loaded {}/3 strings files", loaded);
            }
        }
    }

    parser.parse(&mut reader)?;
    let elapsed = start.elapsed();

    println!(
        "\nParsing done in {:.2}s, {} strings",
        elapsed.as_secs_f64(),
        parser.strings.len()
    );

    // 应用 SST：按三元组匹配（str_id + record_sig + field_sig）。
    let sst_file = File::open(sst_path)?;
    let mut sst_reader = BufReader::new(sst_file);
    let dict = xt_core::sst::v8::SstDictionary::read_from(&mut sst_reader)?;

    println!("SST loaded: {} entries", dict.entries.len());

    let mut applied = 0;
    for sst_entry in &dict.entries {
        for string_entry in parser.strings.iter_mut() {
            if string_entry.esp_ptr.str_id == sst_entry.esp_ptr.str_id
                && string_entry.esp_ptr.record_sig == sst_entry.esp_ptr.record_sig
                && string_entry.esp_ptr.field_sig == sst_entry.esp_ptr.field_sig
            {
                if !sst_entry.translation.is_empty() {
                    string_entry.translation = sst_entry.translation.clone();
                    string_entry.params.set(SkyStringParams::TRANSLATED, true);
                    string_entry
                        .params
                        .set(SkyStringParams::INCOMPLETE_TRANS, false);
                    applied += 1;
                }
            }
        }
    }

    println!("Applied {} translations", applied);

    // 统计翻译状态分布。
    let total = parser.strings.len();
    let translated = parser
        .strings
        .iter()
        .filter(|s| s.params.is_translated())
        .count();
    let incomplete = parser
        .strings
        .iter()
        .filter(|s| s.params.is_incomplete())
        .count();

    println!("\nTranslation status:");
    println!("  Total: {}", total);
    println!(
        "  Translated: {} ({:.1}%)",
        translated,
        100.0 * translated as f64 / total as f64
    );
    println!(
        "  Incomplete: {} ({:.1}%)",
        incomplete,
        100.0 * incomplete as f64 / total as f64
    );
    println!("  Untranslated: {}", total - translated - incomplete);

    // 可选导出带状态的 TSV。
    if let Some(out) = output {
        let mut lines = Vec::new();
        for sk in &parser.strings {
            let status = if sk.params.is_translated() {
                "TRANSLATED"
            } else if sk.params.is_incomplete() {
                "INCOMPLETE"
            } else {
                "PENDING"
            };
            lines.push(format!(
                "{}\t{}\t{}\t{}\t{}\t{}",
                status,
                sk.esp_ptr.str_id,
                String::from_utf8_lossy(&sk.esp_ptr.record_sig),
                String::from_utf8_lossy(&sk.esp_ptr.field_sig),
                sk.source.replace('\t', " ").replace('\n', "\\n"),
                sk.translation.replace('\t', " ").replace('\n', "\\n")
            ));
        }
        std::fs::write(out, lines.join("\n"))?;
        println!("\nExported {} strings to {}", parser.strings.len(), out);
    }

    // 打印前 20 条样本（包含状态符号）。
    println!("\nSample strings (first 20):");
    for sk in parser.strings.iter().take(20) {
        let status = if sk.params.is_translated() {
            "✓"
        } else if sk.params.is_incomplete() {
            "○"
        } else {
            "·"
        };
        println!(
            "  [{}] {}:{} | {} → {}",
            status,
            String::from_utf8_lossy(&sk.esp_ptr.record_sig),
            String::from_utf8_lossy(&sk.esp_ptr.field_sig),
            sk.source.chars().take(40).collect::<String>(),
            sk.translation.chars().take(40).collect::<String>()
        );
    }

    println!("===================================\n");

    Ok(())
}

/// 解析 ESP、应用 SST 字典，并导出为 Delphi 兼容的 XML 格式。
pub fn apply_and_export_xml(esp_path: &str, sst_path: &str, xml_output: &str) -> Result<()> {
    let path = Path::new(esp_path);
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    println!("Parsing: {} with SST: {}", esp_path, sst_path);
    let start = std::time::Instant::now();

    let mut parser = EspParser::new();

    // 加载 strings 侧文件
    if let Some(parent) = path.parent() {
        let strings_dir = parent.join("Strings");
        if strings_dir.exists() {
            let base_name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Skyrim");
            parser.load_strings_files(&strings_dir, base_name);
            let loaded = parser.strings_files.loaded_count();
            if loaded > 0 {
                println!("  Loaded {}/3 strings files", loaded);
            }
        }
    }

    parser.parse(&mut reader)?;
    let elapsed = start.elapsed();
    println!(
        "Parsing done in {:.2}s, {} strings",
        elapsed.as_secs_f64(),
        parser.strings.len()
    );

    // 应用 SST
    let sst_file = File::open(sst_path)?;
    let mut sst_reader = BufReader::new(sst_file);
    let dict = xt_core::sst::v8::SstDictionary::read_from(&mut sst_reader)?;
    println!("SST loaded: {} entries", dict.entries.len());

    let mut applied = 0;
    for sst_entry in &dict.entries {
        for string_entry in parser.strings.iter_mut() {
            if string_entry.esp_ptr.str_id == sst_entry.esp_ptr.str_id
                && string_entry.esp_ptr.record_sig == sst_entry.esp_ptr.record_sig
                && string_entry.esp_ptr.field_sig == sst_entry.esp_ptr.field_sig
            {
                if !sst_entry.translation.is_empty() {
                    string_entry.translation = sst_entry.translation.clone();
                    string_entry.params.set(SkyStringParams::TRANSLATED, true);
                    string_entry
                        .params
                        .set(SkyStringParams::INCOMPLETE_TRANS, false);
                    applied += 1;
                }
            }
        }
    }
    println!("Applied {} translations", applied);

    // 导出 XML（与 Delphi 兼容的格式）
    let entries = sky_strings_to_xml_entries(&parser.strings);
    println!("XML entries to export: {} (translated only)", entries.len());

    let params = XmlExportParams {
        addon: path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Skyrim".to_string()),
        source_lang: "english".to_string(),
        dest_lang: "chinese".to_string(),
        version: 2,
    };

    let xml_file = File::create(xml_output)?;
    let mut xml_writer = BufWriter::new(xml_file);
    write_xml_export(&mut xml_writer, &params, &entries)?;

    println!("XML exported to: {}", xml_output);
    println!("  Total entries: {}", entries.len());
    println!("  Applied translations: {}", applied);

    Ok(())
}
