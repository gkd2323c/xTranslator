# Skyrim SE PapyrusCompiler fixture

`XtPexFixture.psc` is project-owned test source. `XtPexFixture.pex.hex` is the
byte-for-byte PEX payload produced by the Bethesda Papyrus compiler shipped with
Skyrim Special Edition (`Papyrus Compiler/PapyrusCompiler.exe`, PEX version 3.2,
GameID 1), encoded as hexadecimal text so it can live in the repository without
binary patch tooling.

Only the compiler-generated `UserName` and `ComputerName` header strings were
redacted in-place before encoding, preserving their original byte lengths and
therefore every structural offset:

- UserName: `fixture0` (8 bytes)
- ComputerName: `fixturehost0000` (15 bytes)

The fixture is 731 bytes after hex decoding. SHA-256 of the decoded, redacted
fixture: `8c33d5d753c10e0b6929c11ef90eb486b8b422257acb0c90c22835b0de392500`.

Compiler invocation used for generation:

```text
PapyrusCompiler.exe XtPexFixture -i=<fixture-source-dir>;<Skyrim Data/Source/Scripts> -o=<output-dir> -f=<Skyrim Data/Source/Scripts/TESV_Papyrus_Flags.flg>
```
