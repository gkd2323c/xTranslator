# Delphi 代码分析文档

本文档记录从 Delphi 源代码中提取的关键信息，用于指导 Rust 重写。

原始 Delphi 工程已整理到 `legacy/original-delphi/`。下文的 `TESVT_*.pas`、`*.dfm`、`xTranslator.dproj` 等文件名均以该目录为参考根。

## 核心文件清单

| 文件 | 内容 | 分析状态 | 关键发现 |
|------|------|---------|---------|
| TESVT_typedef.pas | 核心类型定义 (tSkyStr, rEspPointer) | ✅ 已完成 | rEspPointerLite=24B, rId={id:integer,offset:integer}, stringHash=FNV-1a on UTF-16 low bytes |
| TESVT_espDefinition.pas | ESP/ESM 格式定义与解析 | ✅ 已完成 | 压缩记录格式: [4B size LE]+[zlib], dsize 含义差异, getCompressedFlag |
| TESVT_SSTFunc.pas | SST 字典格式读写 | ✅ 已完成 | SST v8 魔数=$39535553, UTF-16LE 编码；`prepareSSTFile`/`loadVocabUserCache` 还定义 pending、oldData、colab、EDID list 行为 |
| TESVT_fstreamSave.pas | 编码处理与 Strings 读写 | ✅ 已完成 | Codepage 系统, rawStringtoString, saveStringFile 写入逻辑 |
| TESVT_StringsFunc.pas | STRINGS 文件读写 | ✅ 已完成 | .STRINGS=null终止, .DLSTRINGS/.ILSTRINGS=4字节长度前缀, 详见 strings_format.md |
| TESVT_HeuristicSearch.pas | 启发式搜索算法 | ⏳ 待分析 | Levenshtein/LCS 算法, 关键阈值参数 |
| TESVT_TranslateFunc.pas | 翻译匹配函数 | ✅ 已分析 | `findStrMatchEx` / `findEdidMatchEx`：pending 跳过、locked/incomplete 继承、同语言 validated、indexMax warning、pexNoTrans 跳过 |
| TESVT_MainLoader.pas | 统一文件加载器 | ✅ 已分析 | `doApplySst`、XML/SST 应用、oldData 保留、PEX 提取/保存、ESP compare、缓存流程 |
| TESVT_TranslatorApi.pas | 在线翻译 API 实现 | ✅ 已分析 | 原版 8 类 provider；Rust 当前保留 OpenAI-compatible + DeepL 更适合短期维护 |
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
- [x] 翻译匹配流程 (TESVT_TranslateFunc.pas)
- [x] 文件加载流程 (TESVT_MainLoader.pas)
- [x] SST 字典应用逻辑 (id 匹配, EDID, normalized, vocab, 状态语义)
- [x] 翻译保存流程 (checkForNullTrans, hash 去重)
- [x] BSA v0x68/v0x69 归档读取与浏览
- [ ] BA2 归档读取
- [x] PEX 脚本解析（字符串提取）
- [x] FUZ 音频匹配

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
- **Rust**: ✅ 已实现 `StringsFile::save()`，支持 null-终止和长度前缀两种格式 + codepage 编码 + 去重（hash_trans+trans 共享偏移）。

### Codepage 系统
- **Delphi**: `TESVT_fstreamSave.pas:189-364` (`getcodepage`)
- **配置**: `Data/<game>/codepage.txt`
- **Rust**: ✅ 已实现 `CodepageConfig`/`CodepageTable`，支持 932/936/949/950/1250-1257，详见 `strings/codepage.rs`

### 字符串哈希
- **Delphi**: `TESVT_Utils.pas` (`StringHash`)
- **算法**: FNV-1a, 对 UTF-16LE 低字节序列哈希
- **Rust**: `crates/xt-core/src/types/esp_pointer.rs::string_hash()`

### SST 保存过滤与旧数据
- **Delphi**: `TESVT_SSTFunc.pas:147-201` (`prepareSSTFile`)
- **关键逻辑**: 仅保存安全状态；collab 空状态转 pending；跳过 lockedStatus、warning、nTrans、deleted；`unusedInSST` 在保存时恢复为 `oldData`。
- **Rust**: `crates/xt-core/src/matching.rs` 通过 `ApplyPolicy::sst_load()` 保留未匹配/歧义 SST 项，`src-tauri/src/commands.rs::save_sst` 将 oldData 项带回 SST 输出。

### SST 加载与 EDID 列表
- **Delphi**: `TESVT_SSTFunc.pas:447-598` (`loadVocabUserCache`)
- **关键逻辑**: 支持 SST v1-v8；v8 master list、v7 colab labels、v2+ ESP pointer、v4+ indexMax/EDID hash、v6+ colabId；同时构造 base vocab list 和 EDID list。
- **Rust**: `crates/xt-core/src/sst/v8.rs` 聚焦 v8 读写；`crates/xt-core/src/matching.rs` 使用统一 `DictionaryApplyEntry` 进入 exact / EDID / normalized / vocab tier。

### 字典应用语义
- **Delphi**: `TESVT_MainLoader.pas:2071-2158` (`doApplySst` / `keepOldData`), `TESVT_TranslateFunc.pas:777-948` (`findStrMatchEx` / `findEdidMatchEx`)
- **关键逻辑**: 同语言应用倾向 `validated`，不同语言应用倾向 `translated`；`pending` 不覆盖译文；`lockedTrans`/`incompleteTrans` 继承状态；EDID/indexMax 不确定时标记 warning/bigWarning；可 tagOnly 更新 colab；可改写 string ID；未应用 SST 项保留为 oldData。
- **Rust**: `crates/xt-core/src/matching.rs` 将这些行为显式建模为 `ApplyPolicy`，避免把状态语义混入匹配 tier，并有 pending、oldData、tagOnly、stringID、warning/bigWarning 回归测试覆盖。

### XML 导入导出
- **Delphi**: `TESVT_XMLFunc.pas:96-155` 写出 `SSTXMLRessources`、`Params`、`EDID`、`REC`、`Source`、`Dest`；`TESVT_XMLFunc.pas:241-412` 导入时构造 vocab/EDID list 并调用同一套匹配函数。
- **Rust**: `crates/xt-core/src/xml/mod.rs` 使用 Delphi 兼容 XML 结构，导入进入共享 matcher；后续需要继续改进 EDID 可导出性。

### PEX 与 noTrans
- **Delphi**: `TESVT_MainLoader.pas:1653-1676` 通过 `NoTransPexCheck` 和 PEX `auth/warn` 标记 `pexNoTrans` / `pexWarn`；`TESVT_MainLoader.pas:2027-2045` 可写回 PEX。
- **Rust**: `crates/xt-core/src/pex` 当前范围是 PEX 字符串提取和展示，不做二进制写回。

### 翻译 API
- **Delphi**: `TESVT_TranslatorApi.pas` 支持 Microsoft、Yandex、Baidu、Youdao、Google、DeepL、OpenAI 等 provider，并有偏好、请求数、字符数、array translation、CRLF clear/restore。
- **Rust**: `crates/xt-core/src/translation_api` 当前实现 OpenAI-compatible 与 DeepL；短期建议优先完善 UI/配置体验，而非追齐所有原版 provider。
