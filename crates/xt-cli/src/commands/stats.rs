use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use xt_core::esp::parser::EspParser;
use xt_core::types::game_id::GameId;

/// 加载 ESP/ESM 并输出 golden snapshot 所需的全部统计信息。
pub fn dump_stats(input: &str, game: Option<GameId>) -> Result<()> {
    let path = Path::new(input);
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    println!("Loading: {}", input);
    let start = std::time::Instant::now();

    let mut parser = if let Some(g) = game {
        EspParser::with_game(Path::new("Data"), g)?
    } else {
        EspParser::new()
    };
    parser.enable_esp_mode();

    // 尝试加载同目录下的 Strings 子目录
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

    println!("\n# Skyrim SE Golden Snapshot");
    println!("\n## 环境");
    println!("| 项目 | 值 |");
    println!("|------|-----|");
    println!("| 文件 | {} |", input);
    println!("| 大小 | {} bytes |", std::fs::metadata(path)?.len());
    println!("| 解析时间 | {:.2}s |", elapsed.as_secs_f64());
    println!(
        "| 游戏 | {} |",
        game.map(|g| format!("{:?}", g))
            .unwrap_or_else(|| "Auto".to_string())
    );

    // ── 字符串统计 ───────────────────────────────────────────
    let total_strings = parser.strings.len();
    let unique_str_ids: HashSet<i32> = parser.strings.iter().map(|s| s.esp_ptr.str_id).collect();

    let mut rec_counts: HashMap<String, usize> = HashMap::new();
    let mut field_counts: HashMap<String, usize> = HashMap::new();
    let mut record_field_counts: HashMap<String, usize> = HashMap::new();

    for sk in &parser.strings {
        let rec = String::from_utf8_lossy(&sk.esp_ptr.record_sig).to_string();
        let fld = String::from_utf8_lossy(&sk.esp_ptr.field_sig).to_string();
        let rf = format!("{}:{}", rec, fld);
        *rec_counts.entry(rec).or_insert(0) += 1;
        *field_counts.entry(fld).or_insert(0) += 1;
        *record_field_counts.entry(rf).or_insert(0) += 1;
    }

    println!("\n## 字符串统计");
    println!("| 指标 | 值 |");
    println!("|------|-----|");
    println!("| 总字符串数 | {} |", total_strings);
    println!("| 唯一 str_id 数 | {} |", unique_str_ids.len());
    println!("| 不同 record_sig 数 | {} |", rec_counts.len());
    println!("| 不同 field_sig 数 | {} |", field_counts.len());
    println!(
        "| 不同 record:field 组合数 | {} |",
        record_field_counts.len()
    );

    // record_sig 分布（全部，按数量降序）
    println!("\n### record_sig 分布");
    println!("| record_sig | 字符串数 | 占比 |");
    println!("|-----------|---------|------|");
    let mut rec_vec: Vec<_> = rec_counts.iter().collect();
    rec_vec.sort_by(|a, b| b.1.cmp(a.1));
    for (rec, count) in &rec_vec {
        let pct = 100.0 * **count as f64 / total_strings as f64;
        println!("| {} | {} | {:.2}% |", rec, count, pct);
    }

    // field_sig 分布（全部，按数量降序）
    println!("\n### field_sig 分布");
    println!("| field_sig | 字符串数 | 占比 |");
    println!("|----------|---------|------|");
    let mut fld_vec: Vec<_> = field_counts.iter().collect();
    fld_vec.sort_by(|a, b| b.1.cmp(a.1));
    for (fld, count) in &fld_vec {
        let pct = 100.0 * **count as f64 / total_strings as f64;
        println!("| {} | {} | {:.2}% |", fld, count, pct);
    }

    // record:field 组合分布（前30）
    println!("\n### record:field 组合分布（前30）");
    println!("| 组合 | 字符串数 | 占比 |");
    println!("|------|---------|------|");
    let mut rf_vec: Vec<_> = record_field_counts.iter().collect();
    rf_vec.sort_by(|a, b| b.1.cmp(a.1));
    for (rf, count) in rf_vec.iter().take(30) {
        let pct = 100.0 * **count as f64 / total_strings as f64;
        println!("| {} | {} | {:.2}% |", rf, count, pct);
    }

    // ── GRUP 树统计 ───────────────────────────────────────────
    let top_grups = parser.record_tree.len();
    let mut sub_grups = 0usize;
    let mut total_records = 0usize;
    let mut cell_strings = 0usize;
    let mut wrld_strings = 0usize;
    let mut refr_strings = 0usize;

    fn count_grup(
        grup: &xt_core::esp::record_tree::EspGrup,
        sub_count: &mut usize,
        rec_count: &mut usize,
    ) {
        *sub_count += grup.children.len();
        *rec_count += grup.records.len();
        for child in &grup.children {
            count_grup(child, sub_count, rec_count);
        }
    }

    for grup in &parser.record_tree {
        count_grup(grup, &mut sub_grups, &mut total_records);
    }

    // CELL/WRLD/REFR 字符串统计
    for sk in &parser.strings {
        let rec = String::from_utf8_lossy(&sk.esp_ptr.record_sig);
        match rec.as_ref() {
            "CELL" => cell_strings += 1,
            "WRLD" => wrld_strings += 1,
            "REFR" => refr_strings += 1,
            _ => {}
        }
    }

    println!("\n## GRUP 树结构");
    println!("| 指标 | 值 |");
    println!("|------|-----|");
    println!("| 顶层 GRUP 数 | {} |", top_grups);
    println!("| 子 GRUP 数 | {} |", sub_grups);
    println!("| GRUP 中记录总数 | {} |", total_records);
    println!("| CELL strings | {} |", cell_strings);
    println!("| WRLD strings | {} |", wrld_strings);
    println!("| REFR strings | {} |", refr_strings);

    // ── VMAD 统计 ───────────────────────────────────────────
    // VMAD 字符串通过负 str_id 标记（见 SPEC V33）
    let vmad_strings: Vec<_> = parser
        .strings
        .iter()
        .filter(|s| (s.esp_ptr.str_id) < 0)
        .collect();
    let vmad_count = vmad_strings.len();
    let mut vmad_by_record: HashMap<String, usize> = HashMap::new();
    for sk in &vmad_strings {
        let rec = String::from_utf8_lossy(&sk.esp_ptr.record_sig).to_string();
        *vmad_by_record.entry(rec).or_insert(0) += 1;
    }

    println!("\n## VMAD 字符串统计");
    println!("| 指标 | 值 |");
    println!("|------|-----|");
    println!("| VMAD 字符串总数 | {} |", vmad_count);
    println!("| 含 VMAD 的记录类型数 | {} |", vmad_by_record.len());
    if vmad_count > 0 {
        println!("\n### VMAD 按 record_sig 分布");
        println!("| record_sig | 数量 |");
        println!("|-----------|------|");
        let mut vmad_vec: Vec<_> = vmad_by_record.iter().collect();
        vmad_vec.sort_by(|a, b| b.1.cmp(a.1));
        for (rec, count) in &vmad_vec {
            println!("| {} | {} |", rec, count);
        }
    }

    // ── 压缩记录统计 ───────────────────────────────────────────
    println!("\n## 压缩记录");
    println!("| 指标 | 值 |");
    println!("|------|-----|");
    println!("| compressed_records | {} |", parser.compressed_records);

    // ── 游戏定义覆盖 ───────────────────────────────────────────
    let defined_sigs: HashSet<String> = parser
        .record_defs
        .iter()
        .map(|d| String::from_utf8_lossy(&d.record_sig).to_string())
        .collect();
    let actual_sigs: HashSet<String> = rec_counts.keys().cloned().collect();
    let uncovered: Vec<String> = actual_sigs.difference(&defined_sigs).cloned().collect();

    println!("\n## 游戏定义覆盖");
    println!("| 统计项 | 值 |");
    println!("|--------|-----|");
    println!("| 已定义 record_sig 数 | {} |", defined_sigs.len());
    println!("| 实际出现的 record_sig 数 | {} |", actual_sigs.len());
    println!(
        "| 未覆盖的 record_sig | {} |",
        if uncovered.is_empty() {
            "无".to_string()
        } else {
            uncovered.join(", ")
        }
    );

    println!("\n---");
    println!("*Generated by xt-cli stats*");

    Ok(())
}
