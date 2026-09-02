use std::io::Cursor;

use xt_core::pex::compile::compile_pex_bytes;
use xt_core::pex::decompile::{decompile_pex, emit_pseudocode, Opcode, PexValue};
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
    let original = decode_hex_fixture(include_str!("fixtures/pex/skyrim_se/XtPexFixture.pex.hex"));
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
    assert_eq!(
        rebuilt, original,
        "no-op compile must be byte-for-byte identical"
    );
}

#[test]
fn f4se_compiler_fallout4_fixture_parses_decompiles_and_roundtrips() {
    let original = decode_hex_fixture(include_str!("fixtures/pex/fallout4/Armor.pex.hex"));
    assert_eq!(original.len(), 293);

    let mut cursor = Cursor::new(&original);
    let parsed = parse_pex(&mut cursor).expect("parse Fallout 4 Papyrus PEX");
    assert_eq!(parsed.header.endian, PexEndian::LittleEndian);
    assert_eq!(parsed.header.game_id, 2);
    assert_eq!(parsed.header.major_version, 3);
    assert_eq!(parsed.header.minor_version, 9);
    assert_eq!(
        parsed.header.source_file_name,
        r"E:\github\f4se\scripts\build_src\Armor.psc"
    );
    assert_eq!(parsed.header.user_name, "ianpatt");
    assert_eq!(parsed.header.computer_name, "KURTHNAGA");

    let decompiled = decompile_pex(&original).expect("decompile Fallout 4 Papyrus PEX");
    assert_eq!(decompiled.header.endian, PexEndian::LittleEndian);
    assert_eq!(decompiled.game_id, 2);
    assert_eq!(decompiled.objects.len(), 1);
    assert_eq!(decompiled.objects[0].name, "Armor");
    assert!(!emit_pseudocode(&decompiled).is_empty());

    let (rebuilt, updated, warnings) =
        compile_pex_bytes(&parsed, &[]).expect("recompile Fallout 4 Papyrus PEX");
    assert_eq!(updated, 0);
    assert!(warnings.is_empty());
    assert_eq!(
        rebuilt, original,
        "no-op compile must be byte-for-byte identical"
    );

    let reparsed = parse_pex(&mut Cursor::new(&rebuilt)).expect("reparse rebuilt Fallout 4 PEX");
    assert_eq!(reparsed.header, parsed.header);
    assert_eq!(reparsed.string_table, parsed.string_table);
    assert_eq!(reparsed.translatable.len(), parsed.translatable.len());
}

#[test]
fn f4se_compiler_fallout4_opcode_fixture_parses_and_roundtrips() {
    let original = decode_hex_fixture(include_str!("fixtures/pex/fallout4/Form.pex.hex"));
    assert_eq!(original.len(), 3760);

    let mut cursor = Cursor::new(&original);
    let parsed = parse_pex(&mut cursor).expect("parse Fallout 4 Form PEX");
    assert_eq!(parsed.header.endian, PexEndian::LittleEndian);
    assert_eq!(parsed.header.game_id, 2);
    assert_eq!(parsed.header.major_version, 3);
    assert_eq!(parsed.header.minor_version, 9);
    assert_eq!(
        parsed.header.source_file_name,
        r"E:\github\f4se\scripts\build_src\Form.psc"
    );
    assert_eq!(parsed.header.user_name, "ianpatt");
    assert_eq!(parsed.header.computer_name, "KURTHNAGA");

    let decompiled = decompile_pex(&original).expect("decompile Fallout 4 Form PEX");
    assert_eq!(decompiled.objects.len(), 1);
    assert_eq!(decompiled.objects[0].name, "Form");
    let slot_mask = decompiled.objects[0]
        .properties
        .iter()
        .find(|property| property.name == "kSlotMask30")
        .expect("Form.kSlotMask30 property");
    let read_handler = slot_mask
        .read_handler
        .as_ref()
        .expect("Form.kSlotMask30 read handler");
    assert_eq!(read_handler.instructions.len(), 1);
    assert_eq!(read_handler.instructions[0].opcode, Opcode::Return);
    assert_eq!(
        read_handler.instructions[0].args,
        vec![PexValue::Integer(1)]
    );
    let (rebuilt, updated, warnings) =
        compile_pex_bytes(&parsed, &[]).expect("recompile Fallout 4 Form PEX");
    assert_eq!(updated, 0);
    assert!(warnings.is_empty());
    assert_eq!(
        rebuilt, original,
        "no-op compile must be byte-for-byte identical"
    );
}
