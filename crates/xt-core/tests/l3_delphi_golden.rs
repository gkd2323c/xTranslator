//! L3 交叉验证测试：Rust 重写版与 Delphi xTranslator 1.6.0 金样本的字节级一致性与内容对照。
//!
//! 这些测试**不依赖 238MB Skyrim.esm 实时解析**（debug 模式下过慢），
//! 而是直接验证 Rust 能否正确读取 Delphi 预先导出的参考文件：
//!   - `.STRINGS` / `.DLSTRINGS` / `.ILSTRINGS` 二进制格式解析
//!   - `.SST` 字典读取
//!   - `.XML` 导出解析与字段对齐
//!
//! 金样本来源：`tests/fixtures/delphi_golden/`（Delphi 1.6.0 导出 Skyrim.esm）
//! 验证矩阵详情见 `docs/l3_verification_matrix.md`

use std::path::PathBuf;

use xt_core::sst::v8::SstDictionary;
use xt_core::strings::StringsFile;
use xt_core::strings::StringsFormat;
use xt_core::xml::parse_xml_file;

fn golden_dir() -> PathBuf {
    // 仓库根 / tests/fixtures/delphi_golden
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests")
        .join("fixtures")
        .join("delphi_golden")
}

#[test]
fn l3_strings_binaries_byte_compatible_with_delphi() {
    let dir = golden_dir();

    // Delphi 金样本条目数（来自 docs/l3_verification_matrix.md 实测）
    let cases = [
        (
            "Skyrim_chinese.STRINGS",
            StringsFormat::NullTerminated,
            30294usize,
        ),
        (
            "Skyrim_chinese.DLSTRINGS",
            StringsFormat::LengthPrefixed,
            2669,
        ),
        (
            "Skyrim_chinese.ILSTRINGS",
            StringsFormat::LengthPrefixed,
            34427,
        ),
    ];

    for (name, fmt, expected) in cases {
        let path = dir.join(name);
        assert!(path.exists(), "golden file missing: {}", path.display());
        let bytes = std::fs::read(&path).expect("read golden strings");
        let sf =
            StringsFile::load_from_bytes(&bytes, fmt, xt_core::strings::CodepageConfig::utf8())
                .unwrap_or_else(|e| panic!("Rust failed to parse Delphi {}: {}", name, e));
        assert_eq!(
            sf.strings.len(),
            expected,
            "Rust parsed {} entry count mismatch vs Delphi golden",
            name
        );
    }
}

#[test]
fn l3_strings_str_id_aligned_with_xml_dest() {
    // 金样本 .STRINGS 存的是译文（Dest），其 str_id 与 Delphi XML 的 sID 一一对应。
    // 验证：.STRINGS[1] = "鼠道地下室" 对应 XML sID=1 的 <Dest>。
    let dir = golden_dir();
    let strings_bytes = std::fs::read(dir.join("Skyrim_chinese.STRINGS")).unwrap();
    let sf = StringsFile::load_from_bytes(
        &strings_bytes,
        StringsFormat::NullTerminated,
        xt_core::strings::CodepageConfig::utf8(),
    )
    .unwrap();

    // str_id=1 应存在且为 "鼠道地下室"
    let s1 = sf
        .strings
        .get(&1)
        .expect("str_id 1 missing in golden .STRINGS");
    assert_eq!(
        s1, "鼠道地下室",
        "golden .STRINGS[1] should match XML sID=1 Dest"
    );

    // 与 XML 交叉核对
    let xml_path = dir.join("Skyrim_english_chinese.xml");
    let (_params, entries) = parse_xml_file(&xml_path).expect("parse golden XML");
    let s1_xml = entries
        .iter()
        .find(|e| e.str_id == 1)
        .expect("sID=1 missing in golden XML");
    assert_eq!(
        s1_xml.translation, "鼠道地下室",
        "XML sID=1 Dest should equal .STRINGS[1]"
    );
    assert_eq!(
        s1_xml.source, "The Ratway Vaults",
        "XML sID=1 Source should be English original"
    );
}

#[test]
fn l3_sst_reads_delphi_dictionary() {
    let dir = golden_dir();
    let path = dir.join("Skyrim_english_chinese.sst");
    assert!(path.exists(), "golden SST missing");
    let file = std::fs::File::open(&path).expect("open golden SST");
    let mut reader = std::io::BufReader::new(file);
    let sst =
        SstDictionary::read_from(&mut reader).expect("Rust should read Delphi SST golden file");
    // Delphi 金样本含 67390 条（见 docs/l3_verification_matrix.md）
    assert_eq!(
        sst.entries.len(),
        67390,
        "Rust SST entry count should match Delphi golden"
    );
}

#[test]
fn l3_xml_export_parse_and_field_alignment() {
    let dir = golden_dir();
    let path = dir.join("Skyrim_english_chinese.xml");
    let (_params, entries) = parse_xml_file(&path).expect("parse golden XML");
    assert_eq!(
        entries.len(),
        67390,
        "Rust XML parser should read all Delphi golden entries"
    );

    // 验证首条字段对齐
    let first = entries.first().expect("at least one entry");
    assert_eq!(first.str_id, 1);
    assert_eq!(first.edid.as_deref(), Some("RiftenRatway02"));
    assert_eq!(String::from_utf8_lossy(&first.record_sig), "CELL");
    assert_eq!(String::from_utf8_lossy(&first.field_sig), "FULL");
    assert_eq!(first.source, "The Ratway Vaults");
    assert_eq!(first.translation, "鼠道地下室");
}
