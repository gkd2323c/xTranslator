# Fallout 4 Little-Endian PEX fixtures

These files are hexadecimal encodings of real Fallout 4-compatible PEX files
captured from the local `Data/Scripts` environment. Their headers retain the
compiler provenance:

- source path prefix: `E:\github\f4se\scripts\build_src`
- compiler user: `ianpatt`
- compiler host: `KURTHNAGA`

They are therefore treated as F4SE compiler output, not described as Bethesda
source-distribution assets.

- `Armor.pex.hex`: 293 decoded bytes, PEX 3.9, `GameID = 2`, SHA-256
  `2bc34ab0d58f701e8684fc911742257e0768bd3e63b1eb8bdb2e043e7b67346b`; a
  minimal object-body fixture for Little-Endian parsing and no-op roundtrip.
- `Form.pex.hex`: 3760 decoded bytes, PEX 3.9, `GameID = 2`, SHA-256
  `3ac9cd7ecb22d377800ca316413eb1d8f4def3ff3721a14b4c6fa61500f9f568`; it
  includes a real `Return` opcode in the `kSlotMask30` property getter.

`pex_real_fixture.rs` verifies Little-Endian header parsing, object
decompilation, opcode/value decoding for `Form.pex`, and byte-for-byte
no-op `parse -> compile` roundtrips.
