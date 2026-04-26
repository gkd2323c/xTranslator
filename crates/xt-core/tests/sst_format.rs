use std::io::{Cursor, Read};
use xt_core::sst::encoding::read_delphi_string;
use xt_core::sst::v8::{SstDictionary, SST_V8_MAGIC};
use xt_core::types::esp_pointer::EspPointer;
use xt_core::types::params::SkyStringParams;
use xt_core::types::sky_string::SkyString;

/// 验证 SST v8 文件的字节级结构
///
/// 这个测试确保 Rust 生成的 SST 与 Delphi 格式完全一致
#[test]
fn test_sst_byte_level_structure() {
    let mut dict = SstDictionary::new();
    dict.master_list = vec!["Skyrim.esm".to_string()];
    dict.colab_labels = vec![(1, "TeamA".to_string())];

    let mut sk = SkyString::new(0, "Hello".to_string(), "Bonjour".to_string(), *b"INFO", *b"DESC");
    sk.esp_ptr = EspPointer {
        str_id: 42,
        form_id: 0xDEADBEEF,
        record_sig: *b"INFO",
        field_sig: *b"NAM1",
        index: 1,
        index_max: 3,
        edid_hash: 0x12345678,
    };
    sk.colab_id = 2;
    sk.params.set(SkyStringParams::TRANSLATED, true);
    dict.entries.push(sk);

    // Write
    let mut buf = Vec::new();
    dict.write_to(&mut buf).unwrap();

    // Verify byte-level structure
    let mut cursor = Cursor::new(&buf);

    // 1. Magic
    let mut magic = [0u8; 4];
    cursor.read_exact(&mut magic).unwrap();
    assert_eq!(u32::from_le_bytes(magic), SST_V8_MAGIC);

    // 2. v4 flag
    let mut flag = [0u8; 1];
    cursor.read_exact(&mut flag).unwrap();
    assert_eq!(flag[0], 0);

    // 3. Master count
    let mut master_count = [0u8; 4];
    cursor.read_exact(&mut master_count).unwrap();
    assert_eq!(i32::from_le_bytes(master_count), 1);

    // 4. Master string size
    let mut master_size = [0u8; 4];
    cursor.read_exact(&mut master_size).unwrap();
    let master_size = i32::from_le_bytes(master_size);
    // "Skyrim.esm" = 10 chars * 2 bytes = 20 bytes
    assert_eq!(master_size, 20);

    // 5. Master string bytes
    let mut master_bytes = vec![0u8; master_size as usize];
    cursor.read_exact(&mut master_bytes).unwrap();
    // UTF-16LE: 'S' = 0x53, 0x00, 'k' = 0x6B, 0x00, ...
    assert_eq!(&master_bytes[0..2], &[0x53, 0x00]); // 'S'
    assert_eq!(&master_bytes[2..4], &[0x6B, 0x00]); // 'k'

    // 6. Colab count
    let mut colab_count = [0u8; 4];
    cursor.read_exact(&mut colab_count).unwrap();
    assert_eq!(i32::from_le_bytes(colab_count), 1);

    // 7. Colab ID
    let mut colab_id = [0u8; 4];
    cursor.read_exact(&mut colab_id).unwrap();
    assert_eq!(i32::from_le_bytes(colab_id), 1);

    // 8. Colab label size
    let mut colab_size = [0u8; 4];
    cursor.read_exact(&mut colab_size).unwrap();
    let colab_size = i32::from_le_bytes(colab_size);
    // "TeamA" = 5 chars * 2 = 10 bytes
    assert_eq!(colab_size, 10);

    // 9. Colab label bytes
    let mut colab_bytes = vec![0u8; colab_size as usize];
    cursor.read_exact(&mut colab_bytes).unwrap();
    assert_eq!(&colab_bytes[0..2], &[0x54, 0x00]); // 'T'

    // 10. Entry listIndex
    let mut list_index = [0u8; 1];
    cursor.read_exact(&mut list_index).unwrap();
    assert_eq!(list_index[0], 0);

    // 11. EspPointerLite (24 bytes)
    let mut esp_bytes = [0u8; 24];
    cursor.read_exact(&mut esp_bytes).unwrap();
    assert_eq!(i32::from_le_bytes([esp_bytes[0], esp_bytes[1], esp_bytes[2], esp_bytes[3]]), 42);
    assert_eq!(u32::from_le_bytes([esp_bytes[4], esp_bytes[5], esp_bytes[6], esp_bytes[7]]), 0xDEADBEEF);

    // 12. colabId
    let mut colab_id_byte = [0u8; 1];
    cursor.read_exact(&mut colab_id_byte).unwrap();
    assert_eq!(colab_id_byte[0], 2);

    // 13. sparams (without validated)
    let mut sparams = [0u8; 1];
    cursor.read_exact(&mut sparams).unwrap();
    assert_eq!(sparams[0], SkyStringParams::TRANSLATED); // validated removed

    // 14. Source string
    let source = read_delphi_string(&mut cursor).unwrap();
    assert_eq!(source, "Hello");

    // 15. Translation string
    let trans = read_delphi_string(&mut cursor).unwrap();
    assert_eq!(trans, "Bonjour");

    // Should be at EOF
    let mut eof_check = [0u8; 1];
    assert!(cursor.read_exact(&mut eof_check).is_err());
}

/// 验证多种字符类型的 SST 兼容性
#[test]
fn test_sst_various_characters() {
    let long_a = "A".repeat(1000);
    let long_b = "B".repeat(1000);
    let test_cases: Vec<(&str, &str, &str)> = vec![
        ("ASCII", "Hello", "World"),
        ("Chinese", "你好世界", "Hello World"),
        ("Japanese", "こんにちは", "Konnichiwa"),
        ("Russian", "Привет", "Privet"),
        ("Arabic", "مرحبا", "Marhaba"),
        ("Emoji", "Hello 👋", "World 🌍"),
        ("Mixed", "铁剑⚔️魔法", "Iron Sword Magic"),
        ("Empty", "Source", ""),
        ("Long", &long_a, &long_b),
    ];

    for (name, source, translation) in test_cases {
        let mut dict = SstDictionary::new();
        let sk = SkyString::new(0, source.to_string(), translation.to_string(), *b"INFO", *b"NAME");
        dict.entries.push(sk);

        let mut buf = Vec::new();
        dict.write_to(&mut buf).unwrap();

        let dict2 = SstDictionary::read_from(&mut buf.as_slice()).unwrap();
        assert_eq!(
            dict2.entries[0].source, source,
            "Source mismatch for '{}'", name
        );
        assert_eq!(
            dict2.entries[0].translation, translation,
            "Translation mismatch for '{}'", name
        );
    }
}

/// 验证大文件 SST 性能
#[test]
fn test_sst_large_file() {
    let mut dict = SstDictionary::new();
    for i in 0..10_000 {
        let sk = SkyString::new(
            i,
            format!("Source string number {}", i),
            format!("Translation number {}", i),
            *b"INFO",
            *b"DESC",
        );
        dict.entries.push(sk);
    }

    let mut buf = Vec::new();
    dict.write_to(&mut buf).unwrap();

    let dict2 = SstDictionary::read_from(&mut buf.as_slice()).unwrap();
    assert_eq!(dict2.entries.len(), 10_000);

    // 抽查
    assert_eq!(dict2.entries[0].source, "Source string number 0");
    assert_eq!(dict2.entries[9999].source, "Source string number 9999");
}
