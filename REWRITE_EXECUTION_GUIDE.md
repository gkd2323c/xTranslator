# xTranslator Rust 重写 - 详细执行指南

## 项目概览

**当前状态**：Phase 2 基本完成（Record Types 过滤 + Zustand 选择器优化 + 虚拟滚动稳定运行，60 个 lib 测试通过）

**目标**：保持 100% SST 字典兼容性，实现跨平台，性能超越原版

**文档版本**：v3.3（Record Types 过滤 + Zustand 选择器优化，Tauri 应用可正常使用）

**变更记录**：
- v3.3: Record Types 点击过滤 + Zustand 选择器模式（全部组件）+ \u25CF Unicode 修复 + `dev.ps1` 一键启动脚本
- v3.2: 虚拟滚动改造 — `get_strings_chunk` 分块加载 + `react-window` 虚拟滚动 + `selectedId` 机制，SidePanel 全局统计
- v3.1: Phase 2 核心功能完成 — 翻译 API 集成（OpenAIProvider），XML 导出/导入闭环
- v3.0: Phase 2 启动 — 启发式搜索算法（Levenshtein + LCS + LCP），GMST:DATA 过滤，端到端验证 76,385 条
- v2.9: Tauri UI 完成 — MenuBar/SidePanel/StringTable/EditorPanel，Zustand 状态管理，Toast 通知
- v2.8: Phase 1.5 执行方案细化，Tauri UI 从 CLI 升级为桌面应用
- v2.7: 完整 record_defs 加载（*/?/-proc 标记，GameId 映射，EspParser::with_game()）
- v2.6: SST 字典保存实现（save_to_file/load_from_file, CLI sst save/roundtrip），清理死代码
- v2.5: Codepage 回退解码实现（CodepageConfig/CodepageTable，932/936/949/950/1250-1257），清理死代码
- v2.4: EDID 字段提取实现，按记录跟踪 Editor ID，用于 XML diff 匹配
- v2.3: StringsFile::save() 实现，CLI strings load/save/modify 命令，9 个新测试
- v2.2: XML 解析器实现，diff-xml/diff 命令，apply SST 命令，文档更新
- v2.1: 修复 Bethesda 压缩记录解压 Bug，字符串从 68,937 → 71,937
- v2.0: Phase 0 关闭，Phase 1 规划完成，Strings 格式修复
- v1.3: Gate 0 验收报告
- v1.1: 初始执行指南

---

## Phase 0 成果总结

### 已完成任务

| 任务 | 状态 | 关键成果 |
|------|------|---------|
| 0.1 项目脚手架 | ✅ | Tauri 2.x + React + Cargo Workspace，零 warning |
| 0.2 基础类型 | ✅ | EspPointer/SkyString/SkyStringParams/GameId，FNV-1a hash |
| 0.3 SST v8 读写 | ✅ | 完整 roundtrip 测试，UTF-16LE 编码 |
| 0.4 ESP 解析 PoC | ✅ | Skyrim.esm 249MB 解析成功，71,937 条字符串（含压缩记录） |
| 0.5 Tauri 性能原型 | ✅ | 筛选 35-40ms，分页 < 1ms，内存 ~25MB |
| 0.3.1 SST 格式文档 | ✅ | `docs/sst_v8_format.md` |
| 0.4.1 ESP 格式文档 | ✅ | `docs/esp_format.md` |
| Strings 文件集成 | ✅ | .STRINGS/.DLSTRINGS/.ILSTRINGS 三格式解析，71,937 条字符串 |
| 压缩记录解压 | ✅ | 44,153 条 zlib 压缩记录正确解压，NPC_/CELL 等类型可见 |
| SST apply 命令 | ✅ | CLI `apply` 命令加载 SST 字典匹配到 ESP 解析结果 |
| XML 解析器 | ✅ | Delphi xTranslator XML 导出格式解析，支持 diff 对比 |
| Diff 对比命令 | ✅ | `diff-xml` 两个 XML 对比 + `diff` ESP vs XML 交叉验证 |
| EDID 字段提取 | ✅ | 按记录跟踪 Editor ID，用于 XML diff 匹配和界面显示 |
| Strings 写入 | ✅ | `StringsFile::save()` 支持两种格式，CLI strings 命令 |

### 关键技术发现

| 发现 | 内容 | 影响 |
|------|------|------|
| FNV-1a 哈希 | Delphi `StringHash` 对 UTF-16 低字节哈希 | ✅ 已复刻验证 |
| EspPointer 大小 | `rEspPointerLite` = 24 字节 | ✅ 已修正 |
| SST 字符串编码 | Delphi UnicodeString → UTF-16LE | ✅ 已实现 |
| Tauri IPC 性能 | 后端 35-40ms + JSON < 1ms | ✅ 满足需求 |
| **Strings 格式差异** | .STRINGS=null终止，.DLSTRINGS/.ILSTRINGS=4字节长度前缀 | ✅ 已修复 |
| **ESP dsize 含义** | Record 的 dsize **不包含** RecordHeaderData(16B)；GRUP 的 dsize **包含**自身的 GenericHeader(8B)+GrupHeader(16B) | ✅ 已修正 |
| **Strings codepage** | Delphi 用 codepage 系统处理编码（UTF-8 优先 + Windows codepage fallback） | ✅ 已实现 CodepageTable + 932/936/949/950/1250-1257 |
| **INFO:NAM1** | 对话条目的 NAM1 字段存储的是 strings_id (4字节)，不是内联文本 | ✅ 已实现 |
| **Bethesda 压缩记录** | 格式为 `[4字节 decompressedSize LE] + [zlib数据]`，44,153 条压缩记录需要解压 | ✅ 已修复（之前全部跳过） |
| **压缩记录数据分布** | NAVM:15,966 / LAND:15,563 / CELL:7,506 / NPC_:5,118 | ✅ 解压后 NPC_:2,419+CELL:583 新增 |

### 解析数据统计（Skyrim.esm）

```
Time: 3.82s
Total strings: 71,937
  INFO:NAM1   34,427  (对话条目)
  DIAL:FULL    5,170  (对话主题)
  ARMO:DESC    2,752  (护甲描述)
  ARMO:FULL    2,623  (护甲名称)
  WEAP:DESC    2,484  (武器描述)
  WEAP:FULL    2,451  (武器名称)
  NPC_:FULL    2,159  (NPC 名称) ← 新增！压缩记录解压后可见
  INFO:RNAM    1,441  (对话回应文本)
  QUST:FULL    1,286  (任务名称)
  ...
Groups: 118, Records: 819,311
Strings files: 30,294 (.STRINGS) + 1,877 (.DLSTRINGS) + 33,062 (.ILSTRINGS)
Compressed records: 44,153 (NAVM/LAND/CELL/NPC_ etc.) ← 全部正确解压
```

### 待验证项

| 项目 | 状态 | 说明 |
|------|------|------|
| Delphi 读取 Rust SST | ⚠️ | 需要 Delphi 环境测试 |
| ESP 99%+ 一致率 | ⚠️ | 需要 Delphi 工具导出做 diff；当前 71,937 条，包含 NPC 和 CELL |
| Codepage 回退机制 | ✅ 已实现 | 932/936/949/950/1250-1257 全部支持 |
| ~~压缩 Record 解压~~ | ✅ 已修复 | Bethesda 格式：4字节大小 + zlib，44,153 条正确解压 |
| 嵌套 GRUP 解析 | ⚠️ | 当前跳过嵌套 GRUP（CELL/PGRD 等） |

---

## 第一部分：当前代码结构

### 1.1 实际目录结构

```
xTranslator/
├── Cargo.toml                          # Workspace 根配置
├── crates/
│   ├── xt-core/                        # 核心业务逻辑
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  # 模块导出
│   │       ├── esp/                    # ESP/ESM 解析
│   │       │   ├── mod.rs
│   │       │   ├── header.rs           # GenericHeader/RecordHeaderData/GrupHeader/FieldHeader
│   │       │   └── parser.rs           # EspParser + StringsFiles 集成 + 压缩记录解压
│   │       ├── strings/                # STRINGS 文件解析
│   │       │   ├── mod.rs              # StringsFile（支持 null-terminator 和 length-prefix 两种格式 + codepage 编码）
│   │       │   └── codepage.rs         # Codepage 编码系统（CodepageConfig/CodepageTable，932/936/949/950/1250-1257）
│   │       ├── sst/                    # SST 字典格式
│   │       │   ├── mod.rs
│   │       │   ├── v8.rs              # SST v8 Reader/Writer
│   │       │   └── encoding.rs       # Delphi UTF-16LE 字符串编解码
│   │       ├── xml/                    # XML 导出格式解析（新增）
│   │       │   └── mod.rs              # Delphi xTranslator XML 解析 + diff
│   │       ├── testing/               # 虚拟数据 + 查询引擎（性能测试用）
│   │       │   ├── mod.rs
│   │       │   ├── generator.rs       # 10 万条虚拟数据生成
│   │       │   └── query.rs           # 筛选/排序/分页查询引擎
│   │       └── types/                 # 核心数据类型
│   │           ├── mod.rs
│   │           ├── sky_string.rs      # SkyString（源/翻译字符串+哈希+参数）
│   │           ├── esp_pointer.rs      # EspPointer（24字节 FNV-1a）
│   │           ├── params.rs           # SkyStringParams + SkyStringInternalParams 标志位
│   │           └── game_id.rs         # GameId 枚举
│   ├── xt-shared/                      # IPC DTO
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── dto.rs                 # QueryRequest/QueryResponse/SkyStringDTO
│   │       └── query.rs
│   └── xt-cli/                         # CLI 测试工具
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs                # CLI 命令路由
│           └── commands/
│               ├── mod.rs
│               ├── sst.rs             # sst generate/read/export/roundtrip
│               ├── parse.rs           # parse + apply 命令（ESP 解析 + SST 应用）
│               └── diff.rs            # diff + diff-xml 命令（交叉验证）
├── src-tauri/                          # Tauri 2.x 后端
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs
│   │   └── commands.rs               # query_strings_command, get_stats
│   ├── tauri.conf.json
│   └── capabilities/default.json
├── ui/                                 # React 前端
│   ├── package.json
│   ├── vite.config.ts
│   └── src/
│       ├── main.tsx
│       ├── App.tsx
│       ├── App.css
│       └── api/strings.ts
├── tests/
│   ├── fixtures/                    # 测试数据
│   │   ├── test_v8.sst
│   │   └── README.md
│   ├── benchmark_ipc.rs             # IPC 性能基准
│   └── sst_format.rs                 # SST 字节级验证
├── docs/
│   ├── delphi_analysis.md            # Delphi 代码分析
│   ├── sst_v8_format.md              # SST v8 二进制格式规范
│   └── esp_format.md                 # ESP/ESM 文件格式分析
└── Data/                              # 游戏数据（原版工具数据）
    └── SkyrimSE/_recorddefs.txt       # 可翻译字段定义
```

### 1.2 关键类型映射（已实现）

| Delphi | Rust | 文件 | 状态 |
|--------|------|------|------|
| `tSkyStr` | `SkyString` | `types/sky_string.rs` | ✅ |
| `rEspPointer` / `rEspPointerLite` | `EspPointer` | `types/esp_pointer.rs` | ✅ |
| `sStrParams` / `sStrInternalParams` | `SkyStringParams` / `SkyStringInternalParams` | `types/params.rs` | ✅ |
| `StringHash()` | `string_hash()` | `types/esp_pointer.rs` | ✅ |
| `parseStringsEx()` | `StringsFile::load_with_format()` | `strings/mod.rs` | ✅ |
| `parseRecordDefs()` | `parse_record_defs()` | `esp/parser.rs` | ✅ |

---

## 第二部分：Strings 文件详解

### 2.1 格式差异（关键发现）

| 格式 | listIndex | 读取方式 | 大小示例 |
|------|-----------|---------|----------|
| `.STRINGS` | 0 | null 终止字符串 | 30,294 条 / ~540KB |
| `.DLSTRINGS` | 1 | 4 字节 LE 长度前缀 + (length-1) 字节内容 | 1,877 条 / ~2.2MB |
| `.ILSTRINGS` | 2 | 4 字节 LE 长度前缀 + (length-1) 字节内容 | 33,062 条 / ~2.2MB |

### 2.2 文件结构（三种格式共享）

```
Header:
  count: u32       // 条目数量
  data_size: u32   // 数据区大小（字节）

Directory (count * 8 bytes):
  entry[i]: { id: u32, offset: u32 }  // offset 是数据区内的偏移

Data Section:
  .STRINGS:    null-terminated UTF-8 strings
  .DLSTRINGS:  [4-byte length][length-1 bytes][null terminator]
  .ILSTRINGS:  [4-byte length][length-1 bytes][null terminator]
```

### 2.3 Codepage 系统（Delphi 原版）

Delphi 原版使用 `codepage.txt` 配置文件指定每种语言的编码：

```
# Data/SkyrimSE/codepage.txt
english=utf8,1252     # UTF-8 优先，1252 降级
chinese=utf8          # 仅 UTF-8
japanese=utf8,932     # UTF-8 优先，Shift-JIS 降级
```

**读取逻辑**（`TESVT_StringsFunc.pas` → `parseStringsEx`）：
1. 根据文件名匹配 codepage
2. `.STRINGS` (listIndex=0): 逐字节读 null 终止
3. `.DLSTRINGS/ILSTRINGS` (listIndex>0): 读 4 字节长度前缀 + (size-1) 字节内容
4. 解码：先尝试 UTF-8，失败则 fallback 到 codepage 指定的 Windows 编码

**写入逻辑**（`saveStringFile`）：
1. 按 `strId` 排序
2. 去重（相同 `hash_trans + trans` 共享数据偏移）
3. 使用 `codepage.f` 函数指针编码：UTF-8 用 `utf8encode()`，其他用 `AnsiString(codepage)`
4. `.DLSTRINGS/ILSTRINGS` 写入 4 字节长度前缀

### 2.4 当前 Rust 实现状态

- ✅ `.STRINGS` null 终止读取
- ✅ `.DLSTRINGS/ILSTRINGS` 长度前缀读取
- ✅ 按文件扩展名自动检测格式
- ✅ UTF-8 解码 + Windows codepage fallback（932/936/949/950/1250-1257）
- ✅ Codepage 配置解析（CodepageTable）
- ✅ 从文件名自动推断语言并应用对应编码
- ✅ 写入时使用 codepage 编码

---

## 第三部分：ESP 解析器详解

### 3.1 解析架构

```
EspParser
├── record_defs: Vec<TranslatableField>   // 从 _recorddefs.txt 加载
├── strings: Vec<SkyString>               // 提取的字符串结果
├── strings_files: StringsFiles           // .STRINGS/.DLSTRINGS/.ILSTRINGS
│
├── parse(reader)                          // 主解析入口
│   ├── 读取 TES4 header + RecordHeaderData
│   ├── 读取 TES4 字段（独立处理）
│   └── 循环解析 GRUP / Record
│
├── parse_top_level_debug()               // 顶层 GRUP/Record 解析
│
└── parse_record_debug()                   // Record 解析
    ├── 读取 GenericHeader(8) + RecordHeaderData(16)
    ├── 处理压缩记录（当前跳过）
    └── parse_record_fields_direct()
        ├── 解析 FieldHeader(6) + data
        ├── XXXX 字段 → 下一字段使用 32 位大小
        └── 匹配 TranslatableField → 从 StringsFiles 查找文本
```

### 3.2 ESP/ESM 关键格式细节

| 结构体 | 大小 | 说明 |
|--------|------|------|
| `GenericHeader` | 8B | `name[4] + dsize:u32` |
| `RecordHeaderData` | 16B | `flags:u32 + form_id:u32 + version:u32 + f_version:u16 + v_info:u16` |
| `GrupHeader` | 16B | `s_ident[4] + s_type:u32 + s_tstamp:u16 + param1:u16 + param2:u16 + param3:u16` |
| `FieldHeader` | 6B | `name[4] + dsize:u16` |

**dsize 含义**（最关键的发现）：
- **Record**: `dsize` = 字段数据大小（**不含** RecordHeaderData 的 16 字节）
- **GRUP**: `dsize` = 整个 GRUP 块的大小（**含** GenericHeader 8B + GrupHeader 16B）

### 3.3 字符串提取逻辑

ESP 记录中的可翻译字段（如 `FULL`、`DESC`、`NAM1`）存储的不是文本本身，而是一个 **4 字节的 string_id**，实际文本需要从 Strings 文件中查找：

```
ESP 记录: ARMO → FULL 字段 → [4字节 string_id]
                                          ↓
Strings 文件: skyrim_english.STRINGS → string_id → "铁甲"
```

`list_index` 决定查找哪个 Strings 文件：
- 0 → `.STRINGS`
- 1 → `.DLSTRINGS`
- 2 → `.ILSTRINGS`

---

## 第四部分：Phase 1 执行计划

### 1.1 Phase 1 目标

实现 **MVP 最小可用产品**：用户能加载 ESP+Strings，查看翻译列表，编辑翻译，保存 SST 字典。

### 1.2 Phase 1 任务分解

#### Task 1.1：ESP 解析器增强（1 周）

**当前状态**：大部分已完成

| 编号 | 任务 | 优先级 | 状态 | 说明 |
|------|------|--------|------|------|
| 1.1.1 | 压缩记录解压 | P0 | ✅ 已完成 | `[4B size LE] + [zlib]` 格式，44,153 条正确解压 |
| 1.1.2 | 嵌套 GRUP 递归解析 | P1 | ✅ 已完成 | 代码已支持递归，需验证 CELL 内字符串 |
| 1.1.3 | EDID 字段提取 | P1 | ✅ 已完成 | 按记录跟踪 EDID，用于 XML diff 匹配和界面显示 |
| 1.1.4 | 完整 record_defs 加载 | P1 | ✅ 已完成 | 支持 */?/-proc 标记，GameId 映射，load_game_record_defs() |
| 1.1.5 | Codepage 回退解码 | P1 | ✅ 已完成 | CodepageTable/CodepageConfig，932/936/949/950/1250-1257 全部支持 |
| 1.1.6 | 与 Delphi 工具 diff 对比 | P0 | ✅ 已完成 | XML 解析器 + diff 命令已实现，待真实数据验证 |

#### Task 1.2：Strings 写入功能（3-5 天）

**当前状态**：✅ 已完成

| 编号 | 任务 | 优先级 | 状态 | 说明 |
|------|------|--------|------|------|
| 1.2.1 | StringsFile::save() | P0 | ✅ 已完成 | 支持 Null-terminated 和 Length-prefixed 两种格式 |
| 1.2.2 | 保存时的 codepage 编码 | P1 | ✅ 已完成 | StringsFile::save() 使用 codepage 编码写入 |
| 1.2.3 | 与 Delphi 保存的文件做字节级 diff | P0 | ⏳ 待验证 | 需 Delphi 环境生成对照文件 |

**CLI 命令**：
```bash
xt-cli strings load <file>              # 加载并显示
xt-cli strings save <source> <dest>     # 保存（自动检测格式）
xt-cli strings modify <file> <id> <text> # 修改条目
```

**写入逻辑**（来自 Delphi `saveStringFile`）：
1. 按 `strId` 排序
2. 去重：相同 `(hash_trans + trans)` 共享数据偏移（优化，暂未实现）
3. 使用 `codepage.f` 函数指针编码字符串（当前仅 UTF-8）
4. `.DLSTRINGS/ILSTRINGS` 写入 4 字节长度前缀

#### Task 1.3：ESP 写入功能（1 周）

| 编号 | 任务 | 优先级 | 说明 |
|------|------|--------|------|
| 1.3.1 | ESP 文件写入框架 | P0 | 复制原始文件 + 修改指定字段 |
| 1.3.2 | Strings ID 回写 | P0 | 翻译后的文本 → 写回 Strings 文件 |
| 1.3.3 | 增量写入（避免全量重写） | P2 | 只修改变化的字段 |

**关键设计**：ESP 写入需要 **原样保留所有未被修改的数据**，仅更新翻译字符串对应的 field data。这需要：
1. 记住每个字段在文件中的精确偏移和大小
2. 新字符串可能比原字符串大 → 需要调整后续数据偏移
3. 或采用 Strings 文件方式：不修改 ESP，只修改 .STRINGS/.DLSTRINGS/.ILSTRINGS

**Delphi 原版策略**：翻译结果写入 .STRINGS 文件，ESP 本身不修改（除非 delocalize 模式）

#### Task 1.4：SST 字典完整集成（1 周）

| 编号 | 任务 | 优先级 | 状态 | 说明 |
|------|------|--------|------|------|
| 1.4.1 | ESP 解析结果 → SkyString 列表 | P0 | ✅ 已完成 | Strings 查找填充 source 文本 |
| 1.4.2 | SST 字典应用（apply） | P0 | ✅ 已完成 | CLI `apply` 命令，按 strId+record+field 匹配 |
| 1.4.3 | SST 字典保存 | P0 | ✅ 已完成 | SstDictionary::save_to_file(), CLI sst save/roundtrip 命令 |
| 1.4.4 | Delphi 交叉验证 | P0 | ⚠️ 部分 | diff 命令已实现，需 Delphi 环境测试 |

**apply 命令使用方式**：
```bash
# 解析 ESP 并应用 SST 翻译
xt-cli apply "Skyrim.esm" "translation.sst" result.txt

# 输出包含翻译状态：
#   Total: 71937
#   Translated: 5 (匹配 SST 中的条目数)
#   Incomplete: 71932 (待翻译)
```

#### Task 1.5：Tauri UI 基础框架（2 周）

| 编号 | 任务 | 优先级 | 说明 |
|------|------|--------|------|
| 1.5.1 | 主窗口布局（菜单栏 + 左侧文件树 + 右侧编辑区） | P0 | 参考 Delphi 版界面 |
| 1.5.2 | 字符串列表（虚拟表格，TanStack Table） | P0 | 显示 source/translation/status/record |
| 1.5.3 | Tauri IPC 命令完善 | P0 | `load_esp`, `load_sst`, `query_strings`, `save_sst` |
| 1.5.4 | 字符串编辑器 | P1 | 行内编辑 + 保存 |
| 1.5.5 | 筛选/搜索栏 | P1 | 按状态/记录类型/关键词搜索 |

---

## 新增模块：XML 解析器与 Diff 对比

### XML 解析器 (`crates/xt-core/src/xml/mod.rs`)

Delphi xTranslator 的 XML 导出格式是跨版本验证的核心工具。XML 格式如下：

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<SSTXMLRessources>
  <Params>
    <Addon>Skyrim</Addon>
    <Source>english</Source>
    <Dest>chinese</Dest>
    <Version>2</Version>
  </Params>
  <Content>
    <String List="0" sID="000001">
      <EDID>TestEDID</EDID>
      <REC id="9" idMax="9">LCTN:FULL</REC>
      <Source>Hello</Source>
      <Dest>你好</Dest>
    </String>
  </Content>
</SSTXMLRessources>
```

**关键属性**：
- `sID`: 十六进制字符串 ID（如 `000001` = 1）
- `List`: listIndex（0/1/2）
- `REC id`/`idMax`: 字段索引（对应 `EspPointer.index`/`index_max`）
- `REC` 文本：`RECORD:FIELD` 格式（如 `LCTN:FULL`）

**数据结构**：
- `XmlExportParams`: addon, source_lang, dest_lang, version
- `XmlStringEntry`: list_index, str_id, edid, record_sig, field_sig, index, index_max, source, translation
- `to_sky_string()`: 转换为 `SkyString` 方便与 ESP 解析结果对比

### Diff 对比命令 (`crates/xt-cli/src/commands/diff.rs`)

| 命令 | 用途 | 说明 |
|------|------|------|
| `xt-cli diff <esp> <xml>` | ESP 与 XML 交叉验证 | 按 `str_id:record:field` JOIN，计算匹配率、缺失、源文本不匹配 |
| `xt-cli diff-xml <xml1> <xml2>` | 两个 XML 对比 | 检查翻译差异 |

**diff 对比的关键字段**：
```rust
// JOIN key: "{str_id:06X}:{record_sig}:{field_sig}"
// 例如: "000738:CLAS:FULL"
```

**对比维度**：
1. **匹配率** = Rust 中找到对应 XML 条目的比例
2. **缺失数** = XML 中有但 Rust 中没有的条目数
3. **源文本不匹配** = 同一个 key 在 Rust 和 XML 中 source 文本不同

### EDID 字段提取 (`crates/xt-core/src/esp/parser.rs`)

EDID（Editor ID）是记录的人类可读标识符，用于界面显示和 XML diff 匹配。

**实现方式**：
- 在 `parse_record_fields_direct` 中识别 `EDID` 字段签名
- 提取 null-terminated 字符串作为当前记录的 EDID
- 存储在 `EspPointer.edid` 字段中（从 `edid_hash` 改为 `edid: Option<String>`）
- 用于与 XML 导出中的 `<EDID>` 元素匹配

**Delphi 参考**（`TESVT_espDefinition.pas`）：
```pascal
// EDID 字段提取
if (tField(r.fList[z]).header.name = headerEDID) and (length(tField(r.fList[z]).buffer) > 0) then
  SetString(edidNameString, PAnsiChar(@nameField.buffer[0]), length(nameField.buffer) - 1)

// 无 EDID 时使用 FormID 作为后备
result := format('[%.8x]', [self.headerData.formID])
```

**关键设计**：
1. 每个记录解析时先扫描 EDID 字段
2. 后续字符串条目继承当前记录的 EDID
3. 无 EDID 的记录使用 `[FormID]` 格式作为后备

### 1.3 Phase 1 执行顺序

```
Week 1:                          ✅ 已完成
  ├── 1.1.1 压缩记录解压 ✅
  ├── 1.1.6 diff 对比 ✅
  └── 1.1.5 Codepage 回退 ⏳

Week 2:                          ✅ 已完成
  ├── 1.1.2 嵌套 GRUP ✅
  ├── XML 解析器 ✅
  └── SST apply 命令 ✅

Week 3:                          ✅ 已完成
  ├── 1.2 Strings 写入 ✅
  ├── CLI strings 命令 ✅
  └── 1.1.3 EDID 提取 ✅

Week 4 ✅:
  ├── 1.1.5 Codepage 回退 ✅
  ├── 1.4.3 SST 保存 ✅
  └── 1.1.4 完整 record_defs 加载 ✅

Week 5-6（当前）:
  ├── 1.5.1 IPC 命令扩展（load_esp, load_sst, save_sst, update_translation）✅
  ├── 1.5.2 菜单栏 + 文件加载对话框 ✅
  ├── 1.5.3 左侧统计面板 ✅
  ├── 1.5.4 虚拟表格（TanStack Table）✅
  ├── 1.5.5 字符串编辑器（底部编辑区）✅
  ├── 1.5.6 筛选/搜索栏 ⬅️ Day 5-6
  └── 1.5.7 错误处理 + 快捷键 + 性能验收
```

**详细方案**：`docs/phase1_5_execution_plan.md`

### Week 5 任务分解

#### Day 1-2：IPC 命令扩展（后端）✅ 已完成

| 任务 | 说明 | 验收标准 |
|------|------|---------|
| `load_esp` | 加载 ESP+Strings，存入 AppState | ✅ 前端调用成功，返回真实数据 |
| `load_sst` | 加载 SST 字典并匹配 | ✅ 匹配后 translation 填充，status 更新 |
| `save_sst` | 保存当前编辑状态为 SST | ✅ 保存后文件可被 xt-cli 读取 |
| `update_translation` | 单条翻译更新 | ✅ 更新后 query 返回新数据 |

**关键实现**：
- `load_esp` 使用 `tokio::task::spawn_blocking` 避免 UI 阻塞
- `update_translation` 使用 Vec index 做 O(1) 直接访问
- `load_sst` 复用 strId+recordSig+fieldSig 三元组匹配
- `query_strings_command` 内联实现状态筛选+文本筛选+排序+分页

#### Day 3-4：主窗口布局（前端）✅ 已完成

| 任务 | 说明 | 验收标准 |
|------|------|---------|
| MenuBar | 文件菜单 + 对话框 | ✅ 支持 Load ESP / Load SST / Save SST |
| SidePanel | 统计面板 | ✅ 显示文件信息、字符串统计、状态分布 |
| StringTable | 表格 + 分页 + 排序 | ✅ 分页加载，状态筛选，列排序 |
| EditorPanel | 底部编辑区 | ✅ 选中行可编辑 translation，Ctrl+Enter 保存 |

**新增文件**：
- `ui/src/stores/appStore.ts` — Zustand 状态管理
- `ui/src/components/MenuBar.tsx` — 文件菜单 + Tauri dialog
- `ui/src/components/SidePanel.tsx` — 统计面板（文件/统计/SST/Record Types）
- `ui/src/components/StringTable.tsx` — 表格 + 筛选 + 分页 + 排序
- `ui/src/components/EditorPanel.tsx` — 底部编辑区 + 保存
- `ui/src/App.tsx` — 重构为三栏布局
- `ui/src/App.css` — 全新暗色主题样式

**技术栈**：
- Zustand（状态管理）
- react-hot-toast（通知）
- lucide-react（图标）
- @tauri-apps/plugin-dialog（文件对话框）

#### Day 5-6：筛选完善 + 性能验收 ✅ 已完成

| 任务 | 说明 | 验收标准 |
|------|------|---------|
| 文本筛选 | 实时搜索 source/translation | ✅ Debounce 150ms |
| 状态筛选 | All/Incomplete/Translated/Locked | ✅ 单选切换 |
| 列排序 | ID/Record/Source | ✅ 点击表头排序 |
| 错误处理 | Toast 通知 | ✅ react-hot-toast |
| 快捷键 | Escape 关闭编辑 | ✅ App.tsx 全局监听 |
| Bugfix | update_translation 索引错位 | ✅ 改为 id 查找 |

---

## 第八部分：Phase 1.5 完成总结

### 完成内容

#### 后端（src-tauri/）

| 命令 | 功能 | 状态 |
|------|------|------|
| `load_esp` | 加载 ESP+Strings，spawn_blocking 解析 | ✅ |
| `load_sst` | 加载 SST 并匹配翻译 | ✅ |
| `save_sst` | 保存当前状态为 SST | ✅ |
| `update_translation` | 按 id 更新单条翻译 | ✅ |
| `query_strings_command` | 状态筛选+文本筛选+排序+分页 | ✅ |
| `get_stats` | 统计信息 | ✅ |

#### 前端（ui/）

| 组件 | 功能 | 状态 |
|------|------|------|
| `MenuBar` | Load ESP/SST/Save SST + Tauri dialog | ✅ |
| `SidePanel` | 文件信息、统计、Record 类型分布 | ✅ |
| `StringTable` | 表格、分页(100条)、排序、筛选 | ✅ |
| `EditorPanel` | 底部编辑区、Ctrl+Enter 保存 | ✅ |
| `AppStore` | Zustand 状态管理 | ✅ |

#### 新增依赖

| 包 | 用途 |
|----|------|
| `zustand` | 状态管理 |
| `react-hot-toast` | Toast 通知 |
| `lucide-react` | 图标 |
| `@tauri-apps/plugin-dialog` | 文件对话框 |

### 构建状态

- ✅ `cargo test -p xt-core` — 47 个测试通过
- ✅ `npx tsc --noEmit` — TypeScript 零错误
- ✅ `cargo build -p xtranslator-tauri` — 编译成功（1 个 dead_code warning）

### 下一步（Phase 2 可选）

| 功能 | 优先级 | 说明 |
|------|--------|------|
| Record/Field 筛选下拉框 | P1 | 当前只有文本和状态筛选 |
| 虚拟滚动（react-window） | P1 | 当前是简单表格，大数据量可能卡顿 |
| 最近文件列表 | P2 | MenuBar 下拉菜单 |
| 主题切换 | P2 | 亮/暗主题 |
| 多语言 UI | P3 | i18n |

---

### 1.4 Phase 1 验收标准

| 标准 | 指标 |
|------|------|
| ESP 解析覆盖 | Skyrim.esm 非压缩记录 100% 解析，压缩记录 100% 解压 |
| 字符串提取完整率 | 与 Delphi 工具对比 ≥ 99%（按条目数和内容） |
| SST 双向兼容 | Rust 写的 SST 被 Delphi 正确读取，反之亦然 |
| Strings 保存兼容 | Rust 保存的 .STRINGS/.DLSTRINGS/.ILSTRINGS 被游戏正确加载 |
| UI 基础 | 能加载 ESP+Strings，显示字符串列表，编辑翻译，保存 SST |

---

## 第五部分：技术债务清单

### 当前已知问题

| 问题 | 优先级 | 说明 |
|------|--------|------|
| ~~压缩记录跳过~~ | ~~P0~~ | ✅ 已修复：4字节大小前缀 + zlib 格式 |
| ~~GMST:DATA 一刀切过滤~~ | ~~P1~~ | ✅ 已修复：按 EDID 前缀过滤（`s` 开头保留，其余跳过），Rust 与 Delphi SST 导出 100% 一致 |
| 嵌套 GRUP 跳过 | P1 | CELL/WRLD 内的子 GRUP 被整体跳过，部分 CELL 内字符串可能丢失 |
| `<ID:0>` 占位符 | P1 | string_id=0 不在 Strings 文件中，需确认是否属于 null 引用 |
| Codepage fallback | ✅ 已实现 | 932/936/949/950/1250-1257 全部支持，CodepageTable 自动推断 |
| 死代码清理 | P3 | `parse_top_level`, `parse_record`, `parse_record_fields` 未使用 |
| `lib.rs` 中的 `add()` 函数 | ✅ 已清理 | 已在 Phase 1 Week 4 删除 |

### 与原版功能差距

> 详细对比见 [`docs/feature_comparison.md`](docs/feature_comparison.md)

| 差距层级 | 主要缺失功能 | 预估总工作量 |
|---------|------------|------------|
| **P0 - MVP 必需** | Tauri UI 框架、字符串编辑+保存流程、字典应用增强 | ~4 周 |
| **P1 - 核心功能** | 启发式搜索、翻译 API、XML 导出、正则搜索 | ~3-4 周 |
| **P2 - 功能完善** | BSA/BA2 归档、PEX 脚本、FUZ 音频、MCM、ESPCompare、ESM 缓存、撤销 | ~6-8 周 |
| **P3 - 体验优化** | 批量处理器、Header 处理器、主题、UI 多语言、自动备份 | ~3-4 周 |

**Delphi 代码待分析**（影响 P1/P2 实现）：
- TESVT_HeuristicSearch.pas - 启发式搜索算法与阈值参数
- TESVT_TranslateFunc.pas - 翻译匹配流程
- TESVT_MainLoader.pas - 统一文件加载器
- TESVT_TranslatorApi.pas - 在线翻译 API

### 关键技术决策

| 决策项 | 选择 | 理由 |
|--------|------|------|
| ESP 修改策略 | 修改 .STRINGS 文件，不修改 ESP | 原版策略，避免损坏 ESP |
| SST 编码 | UTF-16LE（Delphi 兼容） | 原版使用 UnicodeString |
| Strings 编码 | UTF-8 + codepage fallback | 原版 codepage 系统精确复刻 |
| 并发模型 | tokio async + rayon parallel | IO 用 tokio，CPU 密集用 rayon |
| 前端框架 | TanStack Table + Zustand | 虚拟滚动 + 轻量状态管理 |

---

## 第六部分：Delphi 原版关键逻辑参考

### 6.1 Strings 文件读写（TESVT_StringsFunc.pas）

**读取**（`parseStringsEx`）：
```
1. 读 count + data_size 头部
2. 读 count 个 (id, offset) 目录条目
3. 对每个条目：
   - if listIndex > 0  (.DLSTRINGS/.ILSTRINGS):
       seek → read 4字节 length → read (length-1) 字节内容
   - else (.STRINGS):
       seek → 逐字节读直到 #0
4. 用 codepage 解码 rawbytestring → string
```

**写入**（`saveStringFile`）：
```
1. 按 hash_trans+trans 排序去重
2. 按 strId 排序写回
3. 去重条目共享数据偏移
4. 使用 codepage.f 函数指针编码
5. .DLSTRINGS/.ILSTRINGS 写入 4 字节长度前缀
```

### 6.2 ESP 字段字符串提取（TESVT_espDefinition.pas）

**关键流程**：
1. 读取 `_recorddefs.txt` 获取可翻译字段定义
2. 解析 ESP 记录，匹配 `(record_sig, field_sig, listIndex)`
3. 从字段数据中读取 4 字节 `string_id`
4. 用 `string_id + listIndex` 查找对应的 Strings 文件

### 6.3 Codepage 系统（TESVT_fstreamSave.pas → `getcodepage`）

```
1. 读取 Data/<game>/codepage.txt
2. 匹配语言名 → (主编码, 降级编码)
3. 如 "english=utf8,1252" → 主编码=65001(UTF-8), 降级=1252
4. 读取时先尝试 UTF-8，失败则用降级编码
5. 写入时使用主编码
```

---

### 6.4 Bethesda 压缩记录格式（Bug 修复记录）

**发现问题**：ESP 解析器提取字符串只有 68,935 条，缺少所有压缩记录中的字符串。

**扫描结果**：Skyrim.esm 中有 44,153 条压缩记录：
- NAVM: 15,966 条（导航网格）
- LAND: 15,563 条（地形）
- CELL: 7,506 条（单元格）
- NPC_: 5,118 条（NPC）

**Bethesda 压缩格式**（从 Delphi 源码 + 实际数据确认）：
```
偏移 0-3: decompressedSize (u32 LE) ← 解压后大小
偏移 4+:  zlib 压缩数据           ← 0x78 DA/9C/01 标准 zlib
```

实际数据示例：
- NAVM: `3c 47 00 00 78 da 95 7b...` (decompressedSize=0x473c, zlib default)
- CELL: `d1 00 00 00 78 da 73 75...` (decompressedSize=0xd1=209)
- NPC_: `50 01 00 00 78 da 73 75...` (decompressedSize=0x150=336)

**Delphi 原版**（`TESVT_espDefinition.pas:1719-1768`）：
```pascal
// 1. 读取前4字节 = decompressedSize
getBufferData(b, @decompressedSize, startpos, sizeOf(cardinal), length(b));
// 2. 从偏移4开始解压
DecompressToUserBuf(@b[4], header.dsize - sizeOf(cardinal), @destBuffer[0], decompressedSize);
// 3. 压缩级别检查（zlib header 的2字节）
move(b[4], compressionlvl, 2);  // 0x0178=level1, 0xDA78=level9
```

**原 Rust Bug**：`decompress_bethesda_record()` 三路分支全部错误：
1. `data[0] == 0x10` 检查 12 字节 header → 不匹配（data[0] 是大小低字节如 0x3C）
2. `data[0] == 0x78` 检查裸 zlib → 不匹配（data[0] 是大小低字节，data[4] 才是 0x78）
3. else 分支 → 返回原始压缩数据不做解压，字段解析静默失败

**修复**：改为 `[4字节大小 LE] + [zlib数据]` 格式，与 Delphi 一致。

**修复影响**：
| 指标 | 修复前 | 修复后 |
|------|--------|--------|
| 总字符串 | 68,935 | 71,937 (+3,002) |
| NPC_ 记录 | 0 | 2,419 |
| NPC_:FULL | 0 | 2,159 |
| CELL 记录 | 0 | 583 |
| 解压警告 | 0 | 0 |
| 解析时间 | 2.08s | 3.82s（含 zlib 解压）|

---

## 第九部分：Phase 2 执行计划

### Phase 2 目标

实现**翻译工作流核心功能**：启发式搜索、翻译 API 集成、XML 导出写入。

### 已完成（Phase 2 Day 1）

| 任务 | 状态 | 说明 |
|------|------|------|
| GMST:DATA 过滤 | ✅ | 按 EDID 前缀智能过滤：EDID 以 `s` 开头的字符串型保留，其余数值型跳过 |
| 启发式搜索算法 | ✅ | Levenshtein + LCS + LCP，xt-core heuristic 模块 |
| Tauri IPC 集成 | ✅ | `heuristic_search` 命令，从已翻译字符串中搜索相似项 |
| 前端 UI | ✅ | EditorPanel "Similar" 按钮，显示候选翻译列表 |

### 启发式搜索设计

**算法**：
- `levenshtein_distance(s, t)` — 编辑距离（两行 DP，O(n*m) 空间优化）
- `normalized_similarity(s, t)` — 归一化相似度 0.0~1.0
- `longest_common_substring_len(s, t)` — 最长公共子串
- `longest_common_prefix_len(s, t)` — 最长公共前缀

**使用场景**：
1. 用户选中一个未翻译字符串
2. 点击 "Similar" 按钮
3. 后端在所有已翻译字符串中搜索相似项
4. 返回 Top-5 候选（按相似度排序）
5. 用户点击候选，自动填充翻译框

**API**：
```typescript
// 前端调用
const matches = await heuristicSearch({
  source: "Retrieve the axe",
  min_similarity: 0.4,
  max_results: 5,
});
// 返回: [{ source: "Retrieve the sword", translation: "取回剑", similarity: 0.82, ... }]
```

### Phase 2 已完成任务

| 任务 | 状态 | 说明 |
|------|------|------|
| GMST:DATA 过滤 | ✅ | 按 EDID 前缀智能过滤：EDID 以 `s` 开头的字符串型保留，其余数值型跳过 |
| 启发式搜索算法 | ✅ | Levenshtein + LCS + LCP，xt-core heuristic 模块，6 个单元测试 |
| Tauri IPC 集成 | ✅ | `heuristic_search` 命令，EditorPanel "Similar" 按钮 |
| 翻译 API 集成 | ✅ | OpenAIProvider，API Key 管理（环境变量 + 弹窗设置），EditorPanel "Translate" 按钮 |
| XML 导出写入 | ✅ | `write_xml_export` — Delphi 兼容格式，实体转义，只导出有翻译的条目 |
| XML 导入匹配 | ✅ | `import_xml_to_sky_strings` — 三元组匹配，更新翻译+状态，4 个单元测试 |
| Tauri IPC | ✅ | `export_xml` / `import_xml` 命令已注册 |
| 前端 UI | ✅ | MenuBar 新增 Export/Import XML 按钮，导入后自动刷新表格 |
| **虚拟滚动改造** | ✅ | `get_strings_chunk` 分块加载（10K/批）+ `react-window` FixedSizeList，客户端筛选/排序零延迟 |
| **selectedId 机制** | ✅ | `selectedId` 替代 `selectedIndex`，虚拟滚动卸载行后状态不丢失 |
| **全局统计** | ✅ | SidePanel 基于 `allItems` 计算 translated/incomplete/locked，useMemo 缓存 |
| **Record Types 过滤** | ✅ | SidePanel 点击 Record Type 过滤表格内容（如只显示 INFO），再次点击取消 |
| **Zustand 选择器优化** | ✅ | 全部组件改用 `useAppStore((s) => s.xxx)`，减少无关重渲染 |
| **Unicode 修复** | ✅ | SidePanel `\u25CF` 改为 `{"\u25CF"}` JSX 表达式，正确显示 `●` |
| **dev.ps1 脚本** | ✅ | 一键启动 Vite + Tauri，自动杀旧进程、等待端口、清理 |

### Phase 2 剩余任务

| 任务 | 优先级 | 预估工作量 |
|------|--------|-----------|
| IPC Payload 分块加载 | ✅ | 已实施 `get_strings_chunk`（10K/批 ~2MB）+ `get_strings_count`，8 批拉取 76K 条 |
| DeepL 翻译 Provider | P1 | 2-3 天 |
| 批量处理器 | P2 | 1 周 |
| NPC 地图 | P2 | 3-5 天 |
| UI 多语言 | P2 | 1 周 |
| 主题系统 | P3 | 3-5 天 |

---

## 第十部分：端到端验证报告

### 验证环境
- **OS**: Windows 10/11
- **游戏**: Skyrim Special Edition
- **ESP**: Skyrim.esm (249MB)
- **Strings**: skyrim_english.STRINGS + .DLSTRINGS + .ILSTRINGS

### 验证结果

| 测试项 | 实测 | 目标 | 状态 |
|--------|------|------|------|
| 字符串数量 | 76,385 | >70,000 | ✅ |
| ESP 解析时间 | 4,630ms | <10,000ms | ✅ |
| 查询 (筛选+排序+分页) | 46ms | <500ms | ✅ |
| 内存占用 | ~18MB | <500MB | ✅ |
| SST 保存 | 6.5MB | 合理 | ✅ |
| SST 重新加载 | 76,385 条 | 数量一致 | ✅ |

### Record 类型分布（Top 10）

| 类型 | 数量 | 说明 |
|------|------|------|
| INFO | 35,868 | 对话条目（含 NAM1/RNAM） |
| ARMO | 5,375 | 护甲名称/描述 |
| DIAL | 5,170 | 对话主题 |
| WEAP | 4,935 | 武器名称/描述 |
| QUST | 3,509 | 任务名称 |
| BOOK | 2,461 | 书籍内容 |
| NPC_ | 2,419 | NPC 名称（压缩记录解压后） |
| MGEF | 1,897 | 魔法效果 |
| ACTI | 1,638 | 激活对象 |
| SPEL | 1,625 | 法术 |

---

## 第十一部分：今日执行计划（2025-04-24）

### 最高优先级：IPC Payload 风险验证与解决 ✅

**风险**：`get_all_strings` 传输 76K DTO 约 15-20MB JSON，可能超出 WebView2 `postMessage` 限制（1-4MB on Windows）。

**验证结果**：E2E 测试测算 74,801 条 → **15.94 MB JSON**，远超 4MB 限制。

**分块加载方案已实施**：
- 后端新增 `get_strings_chunk(offset, limit)` + `get_strings_count()` 命令
- 前端 `loadAllStrings()` 改为循环调用，每批 10K 条（~2MB JSON，安全范围内）
- 8 批调用合并为完整 `allItems`，保持虚拟滚动逻辑不变
- 三级 Fallback：分块 → 单发 → 分页查询

### 次要任务：Review 问题修复 ✅

| # | 问题 | 文件 | 状态 |
|---|------|------|------|
| 1 | `updated_ids` 二次遍历 | `commands.rs` + `xml/mod.rs` | ✅ `import_xml_to_sky_strings` 返回 `(matched, unmatched, updated_ids)` |
| 2 | Fallback 数据不一致 | `appStore.ts` | ✅ 降级到 `queryStrings` 时设置 `allItems: []` |

### Review 修复（基于用户 review）

| # | 问题 | 修复 | 状态 |
|---|------|------|------|
| P7/P7.5 | 前后端状态不一致：空翻译时前端 "incomplete" vs 后端 "locked" | 后端 `update_translation` 清空翻译时设置 `INCOMPLETE_TRANS=true` | ✅ |
| P6 | `chunks.flat()` 多一次全量数组拷贝（峰值内存 +15MB） | 改为 `allItems.push(...chunk)` 逐批追加 | ✅ |
| P8 | Load ESP 依赖 StringTable useEffect 隐式触发，连续加载两个 ESP 可能不刷新 | MenuBar `handleLoadEsp` 中显式 `await store.loadAllStrings()` | ✅ |
| — | REWRITE_EXECUTION_GUIDE 描述过时 | 更新为分块加载描述 | ✅ |

### 验证清单

- [x] E2E 测试：74,801 条 → 15.94 MB JSON（确认超限）
- [x] 分块加载后端命令 `get_strings_chunk` + `get_strings_count`
- [x] 分块加载前端 `loadAllStrings` 循环调用
- [x] 前后端状态一致性修复
- [x] 内存优化（逐批 push）
- [x] `cargo test -p xt-core --lib` 全部通过（60 tests）
- [x] `cargo test -p xt-core --test e2e_real_data e2e_ipc_payload_size` 通过
- [x] `npx tsc --noEmit` 零错误
- [x] `cargo build -p xtranslator-tauri` 编译成功
- [x] 真实 Tauri 运行验证（74,801 条，虚拟滚动流畅，筛选/排序零延迟）
- [x] Record Types 点击过滤功能验证
- [x] Zustand 选择器模式全部组件优化
- [x] \u25CF Unicode 显示修复
- [x] `dev.ps1` 一键启动脚本测试

---

**文档版本**：v3.3
**最后更新**：2025-04-24
**更新内容**：
- Record Types 点击过滤 + Zustand 选择器优化（全部组件）
- 虚拟滚动稳定运行：分块加载 + react-window + selectedId 机制
- Phase 2 核心功能完成：启发式搜索 + 翻译 API（OpenAI）+ XML 导出/导入闭环
- 60 个 lib 测试 + E2E 测试全部通过
- 端到端验证报告：76,385 条，4.6s 解析，客户端筛选 <10ms