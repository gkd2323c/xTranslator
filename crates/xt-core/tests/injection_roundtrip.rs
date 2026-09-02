//! BSA / BA2 injection roundtrip 测试（DP-06）
//!
//! 构造最小归档 → 注入替换 → 重新打开 → 验证替换后的内容可读取。

use std::collections::HashMap;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

// ────────────────────────────────────────────────────────────
// 最小 BSA 构造器（SSE v0x69 布局）
// ────────────────────────────────────────────────────────────

struct MiniBsa {
    bytes: Vec<u8>,
    /// (folder, file, offset_in_archive) 供测试断言
    data_offsets: Vec<(String, String, u32)>,
}

/// 构造一个最小 BSA（SSE 0x69）：1 个文件夹、2 个文件、未压缩。
/// 布局：header(40) + folder record(24) + file records(2*16) + names + 数据
fn build_mini_bsa(folder: &str, files: &[(&str, &[u8])]) -> MiniBsa {
    let header_size = 36u32; // BSA\0 + 8 个 u32（SSE 版本 header 共 36 字节）
    let folder_count = 1u32;
    let file_count = files.len() as u32;

    // folder record: hash(8) + file_count(4) + unk(4) + offset(8) = 24
    let folder_record_size = 24u32;
    // file records: 每项 16 字节（hash + raw_size + offset）
    let file_records_size = file_count * 16;

    // name 块：folder name（length 前缀，含 null terminator）+ file names（纯 null 结尾，无长度前缀）
    // 解析器：folder name 用 read_u8+read_exact；file names 用逐字节读到 null
    let mut folder_names: Vec<u8> = Vec::new();
    let folder_name_bytes = folder.as_bytes();
    folder_names.push(folder_name_bytes.len() as u8 + 1); // 含 null terminator
    folder_names.extend_from_slice(folder_name_bytes);
    folder_names.push(0);
    let mut file_names: Vec<u8> = Vec::new();
    for (name, _) in files {
        let name_bytes = name.as_bytes();
        file_names.extend_from_slice(name_bytes);
        file_names.push(0);
    }
    let total_name_len = (folder_names.len() + file_names.len()) as u32;

    // BSA 目录区布局（解析器期望）：
    //   header | folder records | [folder name][file records] | file names | data
    // folder.offset = folder name 绝对位置 + total_file_name_length（file names 总长）
    let folder_names_pos = header_size + folder_record_size;
    let data_start = folder_names_pos + total_name_len + file_records_size;
    let folder_offset = folder_names_pos + file_names.len() as u32;

    let mut buf = Vec::new();
    buf.extend_from_slice(b"BSA\0");
    buf.extend_from_slice(&0x69u32.to_le_bytes()); // version SSE
    buf.extend_from_slice(&header_size.to_le_bytes());
    // archive flags: COMPRESSFILES 未置位（未压缩），PREFIXFULLFILENAMES 未置位
    buf.extend_from_slice(&0u32.to_le_bytes());
    buf.extend_from_slice(&folder_count.to_le_bytes());
    buf.extend_from_slice(&file_count.to_le_bytes());
    buf.extend_from_slice(&0u32.to_le_bytes()); // total folder name length
    buf.extend_from_slice(&(file_names.len() as u32).to_le_bytes()); // total file name length
    buf.extend_from_slice(&0u32.to_le_bytes()); // file flags

    // folder record
    // folder hash: bsa_hash64(folder, "")
    let folder_hash = xt_core::bsa::directory::bsa_hash64(folder, "");
    buf.extend_from_slice(&folder_hash.to_le_bytes());
    buf.extend_from_slice(&file_count.to_le_bytes());
    buf.extend_from_slice(&0u32.to_le_bytes()); // unk
    buf.extend_from_slice(&(folder_offset as u64).to_le_bytes());

    // folder name + file records
    buf.extend_from_slice(&folder_names);
    let mut data_offsets: Vec<(String, String, u32)> = Vec::new();
    let mut running = data_start;
    for (name, data) in files {
        let (base, ext) = split_name_ext(name);
        let file_hash = xt_core::bsa::directory::bsa_hash64(base, &format!(".{ext}"));
        buf.extend_from_slice(&file_hash.to_le_bytes());
        buf.extend_from_slice(&(data.len() as u32).to_le_bytes());
        buf.extend_from_slice(&running.to_le_bytes());
        data_offsets.push((folder.to_string(), name.to_string(), running));
        running += data.len() as u32;
    }

    // file names
    buf.extend_from_slice(&file_names);

    // 数据
    for (name, data) in files {
        let _ = name;
        buf.extend_from_slice(data);
    }

    MiniBsa {
        bytes: buf,
        data_offsets,
    }
}

fn split_name_ext(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(i) => (&name[..i], &name[i + 1..]),
        None => (name, ""),
    }
}

/// 从 BSA 解析目录（复用生产代码）
fn parse_bsa(bytes: &[u8]) -> xt_core::bsa::BsaArchive {
    let path = write_temp(bytes, "roundtrip");
    xt_core::bsa::BsaArchive::open(&path).expect("reopened BSA should parse")
}

fn write_temp(bytes: &[u8], tag: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "xtranslator-inject-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::write(&path, bytes).unwrap();
    path
}

#[test]
fn bsa_injection_roundtrip_replaces_file_and_reopens() {
    let mini = build_mini_bsa(
        "strings",
        &[
            ("skyrim_english.strings", b"Hello World"),
            ("skyrim_chinese.strings", b"\xe4\xbd\xa0\xe5\xa5\xbd"), // 你好
        ],
    );
    let path = write_temp(&mini.bytes, "bsa-src");
    let bsa = xt_core::bsa::BsaArchive::open(&path).unwrap();

    // 提取原文确认基线
    let original = bsa
        .extract_file("strings/skyrim_english.strings")
        .expect("extract original");
    assert_eq!(original, b"Hello World");

    // 注入替换
    let mut replacements = HashMap::new();
    replacements.insert(
        "strings/skyrim_english.strings".to_string(),
        b"Replaced Content!".to_vec(),
    );
    let mut out = Vec::new();
    let summary = bsa
        .inject_file(&mut Cursor::new(&mut out), &replacements)
        .expect("inject should succeed");
    assert_eq!(summary.injected, 1);
    assert!(summary.not_found.is_empty());

    // 写盘并重新打开验证
    let out_path = write_temp(&out, "bsa-out");
    let reopened = xt_core::bsa::BsaArchive::open(&out_path).expect("reopened BSA");
    let replaced = reopened
        .extract_file("strings/skyrim_english.strings")
        .expect("extract replaced");
    assert_eq!(replaced, b"Replaced Content!");
    // 未注入的文件保持原样
    let untouched = reopened
        .extract_file("strings/skyrim_chinese.strings")
        .expect("extract untouched");
    assert_eq!(untouched, b"\xe4\xbd\xa0\xe5\xa5\xbd");

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn bsa_injection_reports_missing_files() {
    let mini = build_mini_bsa("strings", &[("skyrim_english.strings", b"Hello")]);
    let path = write_temp(&mini.bytes, "bsa-miss");
    let bsa = xt_core::bsa::BsaArchive::open(&path).unwrap();

    let mut replacements = HashMap::new();
    replacements.insert("strings/nonexistent.file".to_string(), b"x".to_vec());
    let mut out = Vec::new();
    let summary = bsa
        .inject_file(&mut Cursor::new(&mut out), &replacements)
        .expect("inject should succeed even with missing target");
    assert_eq!(summary.injected, 0);
    assert_eq!(summary.not_found.len(), 1);
    assert_eq!(summary.not_found[0], "strings/nonexistent.file");

    let _ = std::fs::remove_file(&path);
}

// ────────────────────────────────────────────────────────────
// 最小 BA2 GNRL 构造器
// ────────────────────────────────────────────────────────────

struct MiniBa2 {
    bytes: Vec<u8>,
}

/// 构造最小 BA2 GNRL（FO4 0x01）：2 个文件，未压缩。
/// 布局：header(24) + 数据区 + 文件表（name + 36 字节 record）+ string table(空)
fn build_mini_ba2(files: &[(&str, &[u8])]) -> MiniBa2 {
    let mut buf = Vec::new();
    buf.extend_from_slice(b"BTDX");
    buf.extend_from_slice(&0x01u32.to_le_bytes()); // version FO4
    buf.extend_from_slice(b"GNRL");
    buf.extend_from_slice(&(files.len() as u32).to_le_bytes());
    // file_table_offset 暂填 0，随后修正
    let table_offset_pos = buf.len();
    buf.extend_from_slice(&0i64.to_le_bytes());

    // 数据区（zlib 压缩存储，符合 GNRL 压缩约定）
    let mut data_offsets: Vec<u32> = Vec::new();
    let mut packed_sizes: Vec<u32> = Vec::new();
    for (_, data) in files {
        data_offsets.push(buf.len() as u32);
        let mut encoder =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(data).unwrap();
        let compressed = encoder.finish().unwrap();
        packed_sizes.push(compressed.len() as u32);
        buf.extend_from_slice(&compressed);
    }

    // 文件表（name + 36 字节 record 交错）
    let table_offset = buf.len() as u64;
    let mut data_offsets_iter = data_offsets.iter();
    let mut packed_iter = packed_sizes.iter();
    for (name, data) in files {
        // name：长度前缀（字符数）+ UTF-16 LE + 终止 null
        let chars: Vec<u16> = name.encode_utf16().collect();
        buf.push(chars.len() as u8);
        for c in chars {
            buf.extend_from_slice(&c.to_le_bytes());
        }
        buf.extend_from_slice(&0u16.to_le_bytes());

        // 36 字节 record
        let offset = *data_offsets_iter.next().unwrap();
        let packed = *packed_iter.next().unwrap();
        buf.extend_from_slice(&0u32.to_le_bytes()); // name_hash
        buf.extend_from_slice(&0u32.to_le_bytes()); // ext_hash
        buf.extend_from_slice(&0u32.to_le_bytes()); // dir_hash
        buf.extend_from_slice(&0u32.to_le_bytes()); // unk_0c
        buf.extend_from_slice(&(offset as i64).to_le_bytes()); // offset
        buf.extend_from_slice(&packed.to_le_bytes()); // packed_size
        buf.extend_from_slice(&(data.len() as u32).to_le_bytes()); // size
        buf.extend_from_slice(&0u32.to_le_bytes()); // flag
    }

    // 修正 file_table_offset
    let bytes = buf.len();
    let mut final_buf = buf;
    final_buf[table_offset_pos..table_offset_pos + 8]
        .copy_from_slice(&(table_offset as i64).to_le_bytes());
    let _ = bytes;

    MiniBa2 { bytes: final_buf }
}

#[test]
fn ba2_injection_roundtrip_replaces_file_and_reopens() {
    let mini = build_mini_ba2(&[
        ("strings\\skyrim_english.strings", b"Hello World"),
        (
            "strings\\skyrim_chinese.strings",
            b"\xe4\xbd\xa0\xe5\xa5\xbd",
        ),
    ]);
    let path = write_temp(&mini.bytes, "ba2-src");
    let ba2 = xt_core::ba2::Ba2Archive::open(&path).unwrap();

    let original = ba2
        .extract_file("strings/skyrim_english.strings")
        .expect("extract original");
    assert_eq!(original, b"Hello World");

    let mut replacements = HashMap::new();
    replacements.insert(
        "strings/skyrim_english.strings".to_string(),
        b"BA2 Replaced!".to_vec(),
    );
    let mut out = Vec::new();
    let summary = ba2
        .inject_file(&mut Cursor::new(&mut out), &replacements)
        .expect("inject should succeed");
    assert_eq!(summary.injected, 1);

    let out_path = write_temp(&out, "ba2-out");
    let reopened = xt_core::ba2::Ba2Archive::open(&out_path).expect("reopened BA2");
    let replaced = reopened
        .extract_file("strings/skyrim_english.strings")
        .expect("extract replaced");
    assert_eq!(replaced, b"BA2 Replaced!");
    let untouched = reopened
        .extract_file("strings/skyrim_chinese.strings")
        .expect("extract untouched");
    assert_eq!(untouched, b"\xe4\xbd\xa0\xe5\xa5\xbd");

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&out_path);
}
