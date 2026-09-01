use std::io::Cursor;

use xt_core::pex::compile::compile_pex_bytes;
use xt_core::pex::decompile::{decompile_pex, emit_pseudocode};
use xt_core::pex::parser::parse_pex;
use xt_core::pex::types::PexEndian;

fn decode_hex_fixture(text: &str) -> Vec<u8> {
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    assert_eq!(compact.len() % 2, 0, "fixture hex length must be even");
    (0..compact.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&compact[i..i + 2], 16).expect("valid fixture hex"))
        .collect()
}

#[test]
fn bethesda_compiler_skyrim_se_fixture_parses_decompiles_and_roundtrips() {
    let original = decode_hex_fixture(include_str!(
        "fixtures/pex/skyrim_se/XtPexFixture.pex.hex"
    ));
    assert_eq!(original.len(), 731);

    let mut cursor = Cursor::new(&original);
    let parsed = parse_pex(&mut cursor).expect("parse Bethesda PapyrusCompiler PEX");
    assert_eq!(parsed.header.endian, PexEndian::BigEndian);
    assert_eq!(parsed.header.game_id, 1);
    assert_eq!(parsed.header.major_version, 3);
    assert_eq!(parsed.header.minor_version, 2);
    assert_eq!(parsed.header.source_file_name, "XtPexFixture.psc");
    assert_eq!(parsed.header.user_name, "fixture0");
    assert_eq!(parsed.header.computer_name, "fixturehost0000");

    let decompiled = decompile_pex(&original).expect("decompile Bethesda PapyrusCompiler PEX");
    let pseudo = emit_pseudocode(&decompiled);
    assert!(pseudo.contains("Int Function AddTwo(Int a, Int b)"));
    assert!(pseudo.contains("String Function Echo(String value)"));

    let (rebuilt, updated, warnings) =
        compile_pex_bytes(&parsed, &[]).expect("recompile Bethesda PapyrusCompiler PEX");
    assert_eq!(updated, 0);
    assert!(warnings.is_empty());
    assert_eq!(rebuilt, original, "no-op compile must be byte-for-byte identical");
}
