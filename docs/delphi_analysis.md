# Delphi 代码分析文档

本文档记录从 Delphi 源代码中提取的关键信息，用于指导 Rust 重写。

## 核心文件清单

| 文件 | 内容 | 分析状态 | 关键发现 |
|------|------|---------|---------|
| TESVT_typedef.pas | 核心类型定义 (tSkyStr, rEspPointer) | ✅ 已完成 | rEspPointerLite=24B, rId={id:integer,offset:integer}, stringHash=FNV-1a on UTF-16 low bytes |
| TESVT_espDefinition.pas | ESP/ESM 格式定义与解析 | ✅ 已完成 | 压缩记录格式: [4B size LE]+[zlib], dsize 含义差异, getCompressedFlag |
| TESVT_SSTFunc.pas | SST 字典格式读写 | ✅ 已完成 | SST v8 魔数=$39535553, UTF-16LE 编码, 详见 sst_v8_format.md |
| TESVT_fstreamSave.pas | 编码处理与 Strings 读写 | ✅ 已完成 | Codepage 系统, rawStringtoString, saveStringFile 写入逻辑 |
| TESVT_StringsFunc.pas | STRINGS 文件读写 | ✅ 已完成 | .STRINGS=null终止, .DLSTRINGS/.ILSTRINGS=4字节长度前缀, 详见 strings_format.md |
| TESVT_HeuristicSearch.pas | 启发式搜索算法 | ⏳ 待分析 | Levenshtein/LCS 算法, 关键阈值参数 |
| TESVT_TranslateFunc.pas | 翻译匹配函数 | ⏳ 待分析 | 翻译流程, ID 匹配, 词汇匹配 |
| TESVT_MainLoader.pas | 统一文件加载器 | ⏳ 待分析 | 加载流程, Strings+SST+ESP 联合加载 |
| TESVT_TranslatorApi.pas | 在线翻译 API 实现 | ⏳ 待分析 | 多 API 支持 (DeepL/Google/OpenAI) |
| TESVT_Const.pas | 常量定义 | ✅ 已分析 | strList=['.strings','.dlstrings','.ilstrings'], stringHash 实现 |
| TESVT_Utils.pas | 工具函数 | ✅ 已分析 | StringHash()=FNV-1a on UTF-16LE low bytes |

## 已完成的分析任务

- [x] tSkyStr 字段完整映射 → `types/sky_string.rs`
- [x] rEspPointer 结构验证 → `types/esp_pointer.rs`, 24 字节确认
- [x] rEspPointerLite 结构验证 → `types/esp_pointer.rs`, 24 字节确认 (无 pointer 字段)
- [x] stringHash 算法提取 → FNV-1a on UTF-16LE low bytes, `types/esp_pointer.rs::string_hash()`
- [x] SST v8 魔数与格式确认 → 详见 `docs/sst_v8_format.md`
- [x] STRINGS 文件格式 → 详见 `docs/strings_format.md`
- [x] ESP 压缩记录格式 → 详见 `docs/esp_format.md`
- [x] Codepage 编码系统 → 详见 `docs/strings_format.md`

## 待完成的分析任务

- [ ] 启发式搜索阈值参数 (TESVT_HeuristicSearch.pas)
- [ ] 翻译匹配流程 (TESVT_TranslateFunc.pas)
- [ ] 文件加载流程 (TESVT_MainLoader.pas)
- [ ] SST 字典应用逻辑 (id 匹配, 词汇匹配)
- [ ] 翻译保存流程 (checkForNullTrans, hash 去重)
- [ ] BSA/BA2 归档读取
- [ ] PEX 脚本解析
- [ ] FUZ 音频匹配

## 关键代码位置索引

### 压缩记录解压
- **Delphi**: `TESVT_espDefinition.pas:1719-1768` (`trecord.getrawdata`)
- **Delphi 关键行**: `DecompressToUserBuf(@b[4], header.dsize - sizeOf(cardinal), @destBuffer[0], decompressedSize)`
- **Rust**: `crates/xt-core/src/esp/parser.rs` (`decompress_bethesda_record`)

### Strings 文件读取
- **Delphi**: `TESVT_StringsFunc.pas:228-311` (`parseStringsEx`)
- **关键逻辑**: `readExtraInt = listIndex > 0` 决定读取模式
- **Rust**: `crates/xt-core/src/strings/mod.rs` (`StringsFile::load_with_format`)

### Strings 文件写入
- **Delphi**: `TESVT_StringsFunc.pas:326-430` (`saveStringFile`)
- **关键逻辑**: 按 hash_trans+trans 去重, 按 strId 排序, codepage.f 函数指针编码
- **Rust**: ✅ 已实现 `StringsFile::save()`，支持 null-终止和长度前缀两种格式 + codepage 编码。去重优化（hash_trans+trans 共享偏移）未实现。

### Codepage 系统
- **Delphi**: `TESVT_fstreamSave.pas:189-364` (`getcodepage`)
- **配置**: `Data/<game>/codepage.txt`
- **Rust**: ✅ 已实现 `CodepageConfig`/`CodepageTable`，支持 932/936/949/950/1250-1257，详见 `strings/codepage.rs`

### 字符串哈希
- **Delphi**: `TESVT_Utils.pas` (`StringHash`)
- **算法**: FNV-1a, 对 UTF-16LE 低字节序列哈希
- **Rust**: `crates/xt-core/src/types/esp_pointer.rs::string_hash()`