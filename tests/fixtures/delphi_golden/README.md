# Delphi Golden Files

This directory stores reference files exported from Delphi xTranslator 1.6.0,
used to cross-validate the Rust implementation.

## Required Files

Generate these from Delphi xTranslator 1.6.0 using the same Skyrim.esm:

| File                     | How to Generate                                                                     |
| ------------------------ | ----------------------------------------------------------------------------------- |
| `skyrim_se_export.xml`   | Load Skyrim.esm -> File -> Export XML                                               |
| `skyrim_se_export.sst`   | Load Skyrim.esm -> File -> Save SST                                                 |
| `skyrim_se_.strings`     | Load Skyrim.esm -> save .strings file                                               |
| `skyrim_se_.dlstrings`   | Load Skyrim.esm -> save .dlstrings file                                             |
| `skyrim_se_.ilstrings`   | Load Skyrim.esm -> save .ilstrings file                                             |
| `heuristic_baseline.txt` | Search "Find the key" -> copy top 5 results with scores |

## How to Generate

1. Install Delphi 12.1 CE (Community Edition)
2. Open `legacy/original-delphi/xTranslator.dpr`
3. Build and run
4. Load `SkyrimSE/Data/Skyrim.esm`
5. Export each file type as listed above
6. Copy files to this directory

## After Files Are Placed

Run the validation script from the project root:

```bash
cargo run -p xt-cli -- golden-diff \
  --delphi-dir tests/fixtures/delphi_golden \
  --esp "D:\SteamLibrary\steamapps\common\Skyrim Special Edition\Data\Skyrim.esm"
```

## Notes

- The ESP file must be identical to the one used in Delphi
- Record counts and string IDs must match exactly
- Acceptable variance: encoding details, whitespace normalization
