//! Smoke test: load ESP -> edit translation -> save Strings -> verify roundtrip.
//!
//! Requires Skyrim SE installed at the standard path.
//! Run: cargo test -p xt-core --test smoke_test -- --ignored

use std::path::PathBuf;
use xt_core::esp::parser::EspParser;
use xt_core::sst::v8::SstDictionary;
use xt_core::strings::StringsFile;
use xt_core::types::params::SkyStringParams;

const SKYRIM_ESM: &str = r"D:\SteamLibrary\steamapps\common\Skyrim Special Edition\Data\Skyrim.esm";
const DATA_DIR: &str = r"D:\SteamLibrary\steamapps\common\Skyrim Special Edition\Data";

fn skyrim_data_available() -> bool {
    PathBuf::from(SKYRIM_ESM).exists()
}

/// 1. Parse ESP -> verify string count
#[test]
#[ignore = "requires Skyrim SE data"]
fn smoke_parse_esp() {
    assert!(skyrim_data_available(), "Skyrim.esm not found");

    let mut parser = EspParser::new();
    parser.load_strings_files(DATA_DIR, "skyrim");
    let mut file = std::fs::File::open(SKYRIM_ESM).unwrap();
    parser.parse(&mut file).unwrap();

    assert!(
        parser.strings.len() > 70000,
        "Too few strings: {}",
        parser.strings.len()
    );
    assert!(
        !parser.strings[0].source.is_empty(),
        "First string source empty"
    );
}

/// 2. Parse -> edit a string -> save Strings -> reload -> verify
#[test]
#[ignore = "requires Skyrim SE data"]
fn smoke_edit_save_reload() {
    assert!(skyrim_data_available(), "Skyrim.esm not found");

    let mut parser = EspParser::new();
    parser.load_strings_files(DATA_DIR, "skyrim");
    let mut file = std::fs::File::open(SKYRIM_ESM).unwrap();
    parser.parse(&mut file).unwrap();

    let test_text = "SMOKE_TEST_ROUNDTRIP_42";
    parser.strings[0].translation = test_text.to_string();
    parser.strings[0]
        .params
        .set(SkyStringParams::TRANSLATED, true);

    // Save translations to temp Strings file
    let tmp = std::env::temp_dir().join("xt_smoke_strings");
    let _ = std::fs::create_dir_all(&tmp);
    let output = tmp.join("skyrim_english_chinese.strings");

    let mut sf = StringsFile::new();
    sf.format = xt_core::strings::StringsFormat::NullTerminated;
    for s in &parser.strings {
        if s.params.is_translated() && !s.translation.is_empty() {
            sf.strings
                .insert(s.esp_ptr.str_id as u32, s.translation.clone());
        }
    }
    sf.save(&output).unwrap();

    // Reload and verify
    let reloaded =
        StringsFile::load_with_format(&output, xt_core::strings::StringsFormat::NullTerminated)
            .unwrap();
    let first_id = parser.strings[0].esp_ptr.str_id as u32;
    assert_eq!(
        reloaded.get(first_id).unwrap(),
        test_text,
        "Roundtrip failed"
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

/// 3. SST roundtrip: parse -> build SST -> save -> reload -> verify
#[test]
#[ignore = "requires Skyrim SE data"]
fn smoke_sst_roundtrip() {
    assert!(skyrim_data_available(), "Skyrim.esm not found");

    let mut parser = EspParser::new();
    parser.load_strings_files(DATA_DIR, "skyrim");
    let mut file = std::fs::File::open(SKYRIM_ESM).unwrap();
    parser.parse(&mut file).unwrap();

    let test_text = "SMOKE_SST_TEST_99";
    parser.strings[0].translation = test_text.to_string();
    parser.strings[0]
        .params
        .set(SkyStringParams::TRANSLATED, true);

    let sst = SstDictionary::from_entries(parser.strings.clone());
    let tmp = std::env::temp_dir().join("smoke_roundtrip.sst");
    sst.save_to_file(tmp.to_str().unwrap()).unwrap();

    let reloaded = SstDictionary::load_from_file(&tmp).unwrap();
    let first_id = parser.strings[0].esp_ptr.str_id;
    let found = reloaded
        .entries
        .iter()
        .find(|e| e.esp_ptr.str_id == first_id);
    assert!(found.is_some(), "SST roundtrip: entry not found");
    assert!(
        found.unwrap().translation.contains(test_text),
        "SST roundtrip: mismatch"
    );

    let _ = std::fs::remove_file(&tmp);
}
