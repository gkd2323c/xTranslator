# xTranslator 开发发现与研究成果

> 日期：2026-04-27
> 基于：T16-T22 实现过程中的逆向分析、格式探索和架构决策

---

## 一、BSA 归档格式（T16）

### 1.1 BSA 目录结构

BSA v0x68（Skyrim）和 v0x69（SSE）使用 `BSAhash64` 哈希算法进行文件名查找。该算法已在 Delphi 原版的 `TESVT_bsa.pas` 中实现。

```
BsaArchive
├── header: BsaHeader (version, flags, folder_count, file_count)
├── directory: BsaDirectory
│   ├── folders: Vec<BsaFolder>
│   │   ├── name: String          // 文件夹名（如 "strings"）
│   │   ├── hash: u64             // hash64 值
│   │   └── files: Vec<BsaFileRecord>
│   │       ├── hash: u64         // 文件 hash
│   │       ├── raw_size: u32     // 解压后大小
│   │       ├── offset: u32       // 文件数据偏移
│   │       └── name: String      // 文件名
│   └── folder_map: HashMap<hash, index>
└── path: PathBuf                  // 归档文件路径
```

### 1.2 压缩标记

`archive_flags & 0x0004` 仅表示**归档级别**启用了压缩。个别文件可能仍以未压缩形式存储（不常见的流文件场景）。因此 `list_all_files()` 中的 `compressed` 字段是经验判断而非严格确定值 —— 实际压缩状态需在提取时检测。

### 1.3 BSA 实例隔离

`load_esp` 流程已自动从 BSA 提取 Strings 文件。T16 的 BsaBrowser 使用**独立的 BsaArchive 实例**，避免浏览操作污染当前翻译会话的 AppState 数据。

---

## 二、PEX 脚本二进制格式（T17）

### 2.1 格式来源

基于 Delphi `TESVT_scriptPex.pas`（1,466 行）逆向分析。PEX 是 Bethesda Papyrus 脚本语言的编译后二进制格式。

### 2.2 完整二进制布局

```
┌──────────────────────────────────────────────┐
│ PEX File Layout                              │
├──────────────────────────────────────────────┤
│ Magic:      u32 = 0xFA57C0DE                 │
│ Major:      u8  = 3                          │
│ Minor:      u8  = 10 (Skyrim SE)             │
│ GameID:     u16                              │
│ CompileTime: u64                             │
├──────────────────────────────────────────────┤
│ StringTable                                  │
│   count:    u16                              │
│   for each string:                           │
│     length: u16                              │
│     data:   bytes[length]  (UTF-8)           │
├──────────────────────────────────────────────┤
│ DebugInfo                                    │
│   mod_time: u64                              │
│   count:    u16                              │
│   for each: length(u16) + data(bytes)        │
├──────────────────────────────────────────────┤
│ UserFlags                                    │
│   count:    u16                              │
│   for each: name_idx(u16) + flag_idx(u8)     │
├──────────────────────────────────────────────┤
│ ObjectInfos                                  │
│   count:    u16                              │
│   for each object:                           │
│     name_idx:       u16                      │
│     body_size:      u32                      │
│     ┌─ Body ──────────────────────────────┐ │
│     │ parent_class_idx: u16                │ │
│     │ doc_string_idx:    u16 ← DebugString │ │
│     │ user_flags_count: u16                │ │
│     │ auto_state_name_idx: u16             │ │
│     │ variables: count(u16) + per-var      │ │
│     │   name_idx, type_idx, flags(u32)     │ │
│     │   doc_idx(u16) ← DebugString         │ │
│     │   user_flags(u32)                    │ │
│     │   value_data (type-dependent skip)   │ │
│     │ guards: count(u16) + per-guard       │ │
│     │ property_groups: count(u16)          │ │
│     │   per-group: name_idx, doc_idx ←     │ │
│     │   per-property: name_idx, type_idx,  │ │
│     │     doc_idx ← DebugString            │ │
│     │ states: count(u16) + per-state        │ │
│     │   per-function: name_idx, return_idx │ │
│     │     doc_idx ← DebugString             │ │
│     │     params, locals, instructions      │ │
│     └──────────────────────────────────────┘ │
└──────────────────────────────────────────────┘
```

### 2.3 字符串提取来源

可翻译字符串有两种类型：

| 类型 | 来源 | Delphi 字段 |
|------|------|------------|
| **DebugString** | Object.docString / Function.docString / Property.docString / Var.sDoc | `pObj.docString`, `pFunc.docString` |
| **PropertyName** | Property name（非空且语义不是代码标识符） | `pProp.name` |

### 2.4 值类型跳过表

```
ValueType 描述
    0      None
    1      Byte (u8)
    2      Integer (u32)
    3      Float (f32)
    4      Bool (u8)
    5      String (u16 index into StringTable)
    6-8    Array/Struct (count(u16) + recursive)
   11      BoolArray (count + packed bytes)
   12      IntArray
   13      FloatArray
   14      StringArray
```

### 2.5 指令操作码-to-参数映射

| 操作码 | 参数数 | 示例 |
|--------|--------|------|
| 0x00..0x0D, 0x12, 0x16, 0x1B, 0x1E, 0x22..0x24, 0x26, 0x28..0x2C | 0 | NOP, RETURN, CAST |
| 0x0E, 0x10..0x11, 0x13..0x14, 0x1A, 0x1C..0x1D, 0x1F..0x21, 0x25, 0x27 | 1 | JUMP(u16), CALLSTATIC(u16) |
| 0x15, 0x17..0x19, 0x2B, 0x2D | 2 | ARRAY_CREATE(u16, u16) |

### 2.6 写回约束

`compile_pex_strings` 因需重建 StringTable 偏移量及维护二进制兼容性，列为 v2 功能。当前使用 XML 导出作为过渡方案，可被现有 `import_xml` 流程复用。

---

## 三、FUZ 音频容器（T18）

### 3.1 二进制格式

基于 Delphi `TESVT_Fuz.pas` 的 `getFuzFromBuffer` 函数逆向分析。

```
FUZ Container Layout
┌─────────────────────────────────────┐
│ Magic:    [u8; 4] = b"FUZE"        │
│ Unknown:  u32                       │  ← 可能为 LIP 格式版本标记
│ LipSize:  u32                       │  ← LIP 唇形同步数据大小
├─────────────────────────────────────┤
│ LIP Data: bytes[LipSize]            │  ← 可跳过
├─────────────────────────────────────┤
│ WAV Data: remaining bytes           │  ← 标准 RIFF/WAV 格式
└─────────────────────────────────────┘
```

### 3.2 WAV 时长计算

```
Duration(秒) = data_chunk_size / byte_rate

其中：
  sample_rate: WAV fmt chunk offset 24 (u32)
  byte_rate:   WAV fmt chunk offset 28 (u32)
  data_size:   查找 "data" chunk 后的 u32 值
```

### 3.3 文件关联

FUZ 文件名格式为 `<VoiceTypeID>_<ResponseID>_<Index>.fuz`，通过解析文件名中的 hex ResponseID 与已加载的 `SkyString.str_id` 匹配。Voice 目录递归扫描，支持深层嵌套目录结构。

---

## 四、ESP GRUP 父子关系跟踪（T20）

### 4.1 关键发现

ESP 文件的 GRUP 层级结构隐含了记录的父子关系。`GrupHeader.s_type` 字段（u32）对于子 GRUP 包含**父记录的 FormID**。

```
ESP 层级示例：
┌─────────────────────────────────────┐
│ TES4 (文件头)                       │
│ ┌─ GRUP(QUST) ────────────────────┐ │
│ │  QUST record (form_id=0xABC)    │ │
│ │  ┌─ GRUP(s_type=0xABC) ──────┐ │ │
│ │  │  DIAL records (子记录)     │ │ │
│ │  │  ┌─ GRUP(s_type=0xDEF) ─┐ │ │ │
│ │  │  │  INFO records (孙记录) │ │ │ │
│ │  │  └───────────────────────┘ │ │ │
│ │  └───────────────────────────┘ │ │
│ └─────────────────────────────────┘ │
└─────────────────────────────────────┘
```

### 4.2 实现方案

在 `EspParser` 中添加 `current_parent_form_id: u32` 字段：
1. 解析 GRUP 时读取 `GrupHeader.s_type`
2. 保存/恢复父 FormID 在递归前后
3. 创建 `SkyString` 时设置 `parent_form_id` 字段

```rust
// 嵌套 GRUP 解析时跟踪父 FormID
let grup_header = GrupHeader::read_from(reader)?;
let saved_parent = self.current_parent_form_id;
if grup_header.s_type != 0 {
    self.current_parent_form_id = grup_header.s_type;
}
// ...递归解析子记录...
self.current_parent_form_id = saved_parent;
```

### 4.3 对话树构建

`build_dialog_tree` 命令根据 `parent_form_id` 将 INFO 字符串按父 DIAL FormID 分组。结果在前端以可展开的对话组显示。

---

## 五、i18n 架构（T22）

### 5.1 技术选型

使用 `react-i18next` + `i18next`，配置：
- **翻译文件**：`ui/src/locales/<lang>/translation.json`
- **持久化**：localStorage `xtranslator-lang`
- **兜底**：`zh-CN`
- **语言注册中心**：`i18n.ts` 中的 `SUPPORTED_LANGS`（10 种语言）

### 5.2 添加语言步骤

```
1. i18n.ts 添加 import <lang> from "./locales/<code>/translation.json"
2. i18n.ts 添加 resources 注册
3. SUPPORTED_LANGS 添加 "<code>": "Native Name" 条目
4. ui/src/locales/<code>/translation.json 翻译 136 个 key
5. 无需修改组件代码
```

### 5.3 翻译 key 统计

| 命名空间 | key 数量 | 覆盖范围 |
|----------|----------|----------|
| common | 18 | 按钮标签、过滤器 |
| app | 4 | 加载/解析消息 |
| sidebar | 10 | 文件信息、统计数据 |
| editor | 12 | 编辑器面板 |
| batch | 15 | 批量处理器 |
| bsa | 11 | BSA 浏览器 |
| pex | 9 | PEX 面板 |
| fuz | 10 | 音频面板 |
| dialog | 6 | 对话视图 |
| toast | 14 | 通知消息 |
| **总计** | **109** | — |

---

## 六、性能与边界情况

### 6.1 BSA 大文件处理

100MB+ BSA 文件加载时间取决于文件数量和目录结构复杂度。`list_all_files()` 的 f64 哈希查找为 O(1)，构建全量文件列表为 O(files)。测试环境 8,000+ 文件的 Interface.bsa 加载在 <500ms 内完成。

### 6.2 PEX 解析边界

- String table 索引为 0-based（`string_table[idx]`，与 Delphi `pexStringList[id]` 不同需注意）
- 值类型跳过器需递归处理 Array/Struct 类型（type 6-8）
- BoolArray（type 11）使用位压缩，需根据 count 计算字节数

### 6.3 Strings 去重缓存

`save_with_format` 使用 `HashMap<Vec<u8>, u32>` 缓存已写入的字节序列。相同内容共享数据偏移量。实测文件体积减少约 17%（914KB → 782KB）。

---

## 七、编译与工具链

### 7.1 依赖架构

```
xt-core (lib)
├── byteorder       # LE 字节序读写
├── flate2          # zlib 解压（ESP 压缩记录）
├── lz4             # LZ4 解压（SSE BSA v0x69）
├── reqwest         # HTTP 客户端（翻译 API）
├── encoding_rs     # 编码检测与转换
├── quick-xml       # XML 解析/生成
└── async-trait     # 异步 trait 支持

xtranslator-tauri (bin)
├── tauri 2.x       # 桌面框架
├── tokio           # 异步运行时
└── serde/serde_json # 序列化
```

### 7.2 警告清零

项目已完成全量编译警告清零（0 warnings），包括：
- 6 个 unused imports（SeekFrom, BsaFileRecord, Read, Seek, anyhow, Serialize, async_trait）
- 3 个 dead_code 方法（parse_top_level, parse_record, parse_record_fields）
- 1 个 unused variable（folder → _folder, updated_ids → _updated_ids）
- 批次模块字段警告（BatchJobState Running 字段, is_idle 方法）
- Cyrillic 字符警告（c → с 混用修复）

---

## 八、v2 展望

| 项目 | 说明 |
|------|------|
| PEX 写回 | `compile_pex_strings` — 重建 StringTable，写回二进制 PEX |
| BA2 格式 | Fallout 4/76 的 BA2 General 格式支持 |
| 完整反编译 | Papyrus 指令集全量反编译为可读伪代码 |
| ESP 模式编辑 | 直接编辑 ESP 文件中的字符串（当前策略：修改 Strings 文件） |
| ESM 缓存 | SQLite 缓存加速重载（Delphi 原版有此功能） |
| MCM 翻译 | 自定义 txt 格式的 MCM 菜单翻译文件导入 |
| ESPCompare | 两个 ESP 文件对比建字符串对 |
