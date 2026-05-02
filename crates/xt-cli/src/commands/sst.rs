use anyhow::Result;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use xt_core::sst::v8::SstDictionary;
use xt_core::types::esp_pointer::EspPointer;
use xt_core::types::params::SkyStringParams;
use xt_core::types::sky_string::SkyString;

/// 生成测试 SST 文件
pub fn generate_test_sst(output: &str) -> Result<()> {
    let mut dict = SstDictionary::new();
    dict.master_list = vec!["Skyrim.esm".to_string(), "Update.esm".to_string()];
    dict.colab_labels = vec![
        (1, "TranslatorA".to_string()),
        (2, "TranslatorB".to_string()),
    ];

    // 构造 ASCII 条目
    for i in 0..5 {
        let mut sk = SkyString::new(
            i as u32,
            format!("Iron Sword {}", i),
            format!("Épée de fer {}", i),
            *b"WEAP",
            *b"FULL",
        );
        sk.esp_ptr = EspPointer {
            str_id: i as i32,
            form_id: 0x01000000 + i as u32,
            record_sig: *b"WEAP",
            field_sig: *b"FULL",
            index: 0,
            index_max: 1,
            edid_hash: 0,
        };
        sk.colab_id = 1;
        sk.params.set(SkyStringParams::TRANSLATED, true);
        dict.entries.push(sk);
    }

    // 构造 Unicode 条目（验证跨编码兼容）
    let mut sk_cn = SkyString::new(
        100,
        "铁剑".to_string(),
        "Iron Sword".to_string(),
        *b"WEAP",
        *b"FULL",
    );
    sk_cn.esp_ptr = EspPointer {
        str_id: 100,
        form_id: 0x02000064,
        record_sig: *b"WEAP",
        field_sig: *b"FULL",
        index: 0,
        index_max: 1,
        edid_hash: 0,
    };
    dict.entries.push(sk_cn);

    // 构造空译文条目（验证 incomplete 状态）
    let mut sk_empty = SkyString::new(
        101,
        "Steel Armor".to_string(),
        "".to_string(),
        *b"ARMO",
        *b"FULL",
    );
    sk_empty.esp_ptr = EspPointer {
        str_id: 101,
        form_id: 0x02000065,
        record_sig: *b"ARMO",
        field_sig: *b"FULL",
        index: 0,
        index_max: 1,
        edid_hash: 0,
    };
    sk_empty.params.set(SkyStringParams::INCOMPLETE_TRANS, true);
    dict.entries.push(sk_empty);

    let file = File::create(output)?;
    let mut writer = BufWriter::new(file);
    dict.write_to(&mut writer)?;

    println!("Generated test SST: {}", output);
    println!("  Entries: {}", dict.entries.len());
    println!("  Masters: {:?}", dict.master_list);
    println!("  Colab labels: {:?}", dict.colab_labels);
    println!("\nYou can now open this file with Delphi xTranslator to verify compatibility.");

    Ok(())
}

/// 读取并验证 SST 文件
pub fn read_sst(input: &str) -> Result<()> {
    let file = File::open(input)?;
    let mut reader = BufReader::new(file);

    let dict = SstDictionary::read_from(&mut reader)?;

    println!("SST File: {}", input);
    println!("  Entries: {}", dict.entries.len());
    println!("  Masters: {:?}", dict.master_list);
    println!("  Colab labels: {:?}", dict.colab_labels);
    println!();

    // 仅打印前 20 条，避免大字典刷屏。
    for (i, sk) in dict.entries.iter().take(20).enumerate() {
        println!(
            "  [{}] id={} rec={} fld={} colab={} params={:08b}",
            i,
            sk.esp_ptr.str_id,
            String::from_utf8_lossy(&sk.esp_ptr.record_sig),
            String::from_utf8_lossy(&sk.esp_ptr.field_sig),
            sk.colab_id,
            sk.params.0
        );
        println!("      SRC: {}", sk.source);
        println!("      TRS: {}", sk.translation);
    }

    if dict.entries.len() > 20 {
        println!("  ... and {} more entries", dict.entries.len() - 20);
    }

    Ok(())
}

/// 导出 SST 为文本
pub fn export_sst(input: &str, output: &str) -> Result<()> {
    let file = File::open(input)?;
    let mut reader = BufReader::new(file);
    let dict = SstDictionary::read_from(&mut reader)?;

    // 导出为 TSV，便于 diff / grep / 脚本处理。
    let mut lines = Vec::new();
    for sk in &dict.entries {
        lines.push(format!(
            "{}\t{}\t{}\t{}\t{}",
            sk.esp_ptr.str_id,
            String::from_utf8_lossy(&sk.esp_ptr.record_sig),
            String::from_utf8_lossy(&sk.esp_ptr.field_sig),
            sk.source.replace('\t', " ").replace('\n', "\\n"),
            sk.translation.replace('\t', " ").replace('\n', "\\n")
        ));
    }

    std::fs::write(output, lines.join("\n"))?;
    println!("Exported {} entries to {}", dict.entries.len(), output);

    Ok(())
}

/// 应用 SST 字典到字符串列表
#[allow(dead_code)]
pub fn apply_sst(strings: &mut Vec<SkyString>, sst_path: &str) -> Result<usize> {
    let file = File::open(sst_path)?;
    let mut reader = BufReader::new(file);
    let dict = SstDictionary::read_from(&mut reader)?;

    println!("Loaded SST: {} entries", dict.entries.len());

    let mut applied_count = 0;

    for sst_entry in &dict.entries {
        for string_entry in strings.iter_mut() {
            // 用 EspPointer 三元组精确匹配，避免同 record 下误命中其他字段。
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
                    applied_count += 1;
                }
            }
        }
    }

    println!("Applied translations to {} entries", applied_count);
    Ok(applied_count)
}

/// 从 ESP 解析结果保存 SST 字典
pub fn save_sst(esp_path: &str, output: &str, masters: Option<Vec<String>>) -> Result<()> {
    let mut parser = xt_core::esp::parser::EspParser::new();
    let file = File::open(esp_path)?;
    let mut reader = BufReader::new(file);
    parser.parse(&mut reader)?;

    // 未显式提供 masters 时，默认用当前 ESP 文件名作为主文件列表。
    let master_list = masters.unwrap_or_else(|| {
        vec![std::path::Path::new(esp_path)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default()]
    });

    let dict = SstDictionary::from_entries_with_masters(parser.strings, master_list);
    dict.save_to_file(output)?;

    println!("Saved SST: {}", output);
    println!("  Entries: {}", dict.entries.len());
    println!("  Masters: {:?}", dict.master_list);

    Ok(())
}

/// SST roundtrip 验证（读取 → 写入 → 重新读取 → 对比）
pub fn roundtrip_sst(input: &str, output: &str) -> Result<()> {
    // 1. 读取
    let dict1 = SstDictionary::load_from_file(input)?;
    println!(
        "Read SST: {} entries, {} masters, {} labels",
        dict1.entries.len(),
        dict1.master_list.len(),
        dict1.colab_labels.len()
    );

    // 2. 写入
    dict1.save_to_file(output)?;
    println!("Written to: {}", output);

    // 3. 重新读取
    let dict2 = SstDictionary::load_from_file(output)?;

    // 4. 对比关键字段，验证读写可逆性。
    let mut errors = 0;

    if dict1.master_list != dict2.master_list {
        println!("ERROR: master_list mismatch");
        errors += 1;
    } else {
        println!(
            "✓ master_list matches ({} entries)",
            dict1.master_list.len()
        );
    }

    if dict1.colab_labels.len() != dict2.colab_labels.len() {
        println!(
            "ERROR: colab_labels count mismatch: {} vs {}",
            dict1.colab_labels.len(),
            dict2.colab_labels.len()
        );
        errors += 1;
    } else {
        println!(
            "✓ colab_labels count matches ({})",
            dict1.colab_labels.len()
        );
    }

    if dict1.entries.len() != dict2.entries.len() {
        println!(
            "ERROR: entries count mismatch: {} vs {}",
            dict1.entries.len(),
            dict2.entries.len()
        );
        errors += 1;
    } else {
        let mut entry_errors = 0;
        for (i, (a, b)) in dict1.entries.iter().zip(dict2.entries.iter()).enumerate() {
            if a.source != b.source || a.translation != b.translation || a.esp_ptr != b.esp_ptr {
                if entry_errors < 5 {
                    println!("ERROR: entry {} mismatch", i);
                    println!("  SRC: {:?} vs {:?}", a.source, b.source);
                    println!("  TRS: {:?} vs {:?}", a.translation, b.translation);
                }
                entry_errors += 1;
            }
        }
        if entry_errors > 0 {
            println!(
                "ERROR: {} / {} entries mismatch",
                entry_errors,
                dict1.entries.len()
            );
            errors += 1;
        } else {
            println!("✓ All {} entries match", dict1.entries.len());
        }
    }

    if errors == 0 {
        println!("\n✓ Roundtrip verification PASSED");
    } else {
        println!("\n✗ Roundtrip verification FAILED ({} error(s))", errors);
    }

    Ok(())
}
