# xTranslator 执行计划书

> 日期：2026-04-27
> 基于：SPEC.md §T、PLAN.md、feature_comparison.md
> 当前版本：MVP 核心完成（77 测试通过，TypeScript 无错误，构建干净）

---

## 一、当前状态

### 1.1 已完成功能（21/26 任务）

| 编号 | 功能 | 对应模块 |
|------|------|---------|
| T1 | ESP/ESM 解析（zlib 解压、GRUP 嵌套） | xt-core/esp/parser |
| T2 | Strings 文件读写（null-终止 + 长度前缀） | xt-core/strings |
| T3 | SST v8 字典读写（UTF-16LE Delphi 兼容） | xt-core/sst/v8 |
| T4 | XML 导入/导出（实体转义） | xt-core/xml |
| T5 | 启发式搜索（Levenshtein + LCS + LCP） | xt-core/heuristic |
| T6 | OpenAI 翻译 API | xt-core/translation_api |
| T7 | Tauri IPC 命令 + AppState | src-tauri/commands.rs |
| T8 | React 前端 + react-window 虚拟滚动 | ui/ |
| T9 | BSA 归档 Strings 提取（v0x68/v0x69） | xt-core/bsa |
| T10 | Codepage 回退（932/936/949/950/1250-1257） | xt-core/strings |
| T11 | Record Type 筛选（SidePanel 点击过滤） | ui/components/SidePanel |
| T12 | 按 ID 更新（非索引） | src-tauri/commands.rs |
| T13 | 全量加载 + 客户端筛选/排序 | ui/stores/appStore |
| T14 | XML 进度事件 | src-tauri/commands.rs |
| T15 | DeepL 翻译 API | xt-core/translation_api/deepl |
| T19 | 批量处理器 | src-tauri/batch.rs |
| T21 | 主题系统（暗/亮/灰） | ui/App.css + appStore |
| T23 | 正则搜索/替换（捕获组） | ui/components |
| T24 | Strings 写入去重（~17% 缩减） | xt-core/strings |
| T25 | 自动备份（5 分钟定时，保留 10 份） | src-tauri |
| T26 | 撤销/重做（Ctrl+Z/Y，最大 100） | src-tauri + appStore |

### 1.2 剩余任务（5/26）

| 编号 | 功能 | 优先级 | 预估工作量 |
|------|------|--------|-----------|
| T16 | BSA/BA2 完整档案浏览器 | P1 | 6-7 天 |
| T17 | PEX 脚本字符串提取（仅提取可翻译字符串，不做完整反编译） | P2 | 8-10 天 |
| T18 | FUZ 音频映射 | P2 | 4-5 天 |
| T20 | NPC 地图 / 对话视图 | P2 | 4-5 天 |
| T22 | UI 多语言 i18n | P3 | 4-5 天 |

> **总预估**：~32 天 / ~7 周（含 buffer）。T17 工作量最大且风险最高。

### 1.3 技术指标

| 指标 | 数值 |
|------|------|
| Rust 测试 | 77 通过，0 失败 |
| TypeScript 类型检查 | 0 错误 |
| Cargo build | 0 警告 |
| 项目代码量 | ~7,000 行（Rust + TypeScript） |
| 旧版 Delphi 代码 | ~67,000 行（参考用） |

---

## 二、执行路线图

```
Week 1:     T0  冒烟测试 + 代码清理（今天开始，0.5 天）
Week 1-2:   T16 BSA/BA2 档案浏览器
Week 2-4:   T17 PEX 脚本字符串提取
Week 5:     T18 FUZ 音频映射
Week 6:     T20 NPC 地图 / 对话视图
Week 7:     T22 UI 多语言 i18n
```

### 二.A、T0：冒烟测试 + 代码清理 [前置任务]

在进入 T16 之前，先花半天建立质量基线：

| 步骤 | 内容 | 预估 |
|------|------|------|
| T0.1 | 编写冒烟测试脚本：加载 ESP → 编辑翻译 → 保存 Strings → SST roundtrip 验证 | 2h |
| T0.2 | 确认 `cargo test -p xt-core --lib` 和 `npx tsc --noEmit` 全绿 | 已验证 ✓ |
| T0.3 | 清理 `docs/feature_comparison.md`：同步已完成项（T19/T21/T23/T24/T25/T26 标注为完成，与 SPEC.md 对齐） | 0.5h |
| T0.4 | 确认 `cargo build -p xtranslator-tauri --release` 无警告 | 已验证 ✓ |

> 冒烟测试脚本放在 `tests/smoke_test.rs`，作为 CI 快速回归。

---

## 三、T16：BSA/BA2 完整档案浏览器 [P1]

### 3.1 现状分析

**已有能力**：
- `xt-core/bsa/` 模块完整实现 BSA v0x68（Skyrim）和 v0x69（SSE）解析
- 已支持 `BSAhash64` 文件名哈希查找
- 已支持 zlib（v0x68）和 LZ4（v0x69）解压
- Strings 加载时已自动从 BSA 提取：`load_esp` → 扫描 `.bsa` → `strings/` 文件夹提取

**缺失部分**：
- 无文件列表/目录树暴露给前端
- 无法浏览 BSA 内部结构
- 无法按需提取任意文件
- 无 BA2（Fallout 4/76）格式支持

### 3.2 技术方案

#### 3.2.1 后端：新增 IPC 命令

```rust
// 1. 获取 BSA 文件列表
#[tauri::command]
async fn list_bsa_files(bsa_path: String)
    -> Result<BsaFileList, String>

// 2. 提取单个文件
#[tauri::command]
async fn extract_bsa_file(bsa_path: String, file_path: String, output_dir: String)
    -> Result<String, String>

// 3. 批量提取
#[tauri::command]
async fn extract_bsa_folder(bsa_path: String, folder: String, output_dir: String)
    -> Result<Vec<String>, String>
```

#### 3.2.2 新增 DTO（xt-shared）

```rust
#[derive(Serialize, Deserialize, Clone)]
pub struct BsaFileEntry {
    pub path: String,        // e.g. "strings/skyrim_english.strings"
    pub size: u64,           // compressed size in archive
    pub size_decompressed: u64,
    pub compressed: bool,
    pub folder: String,      // parent folder name
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BsaFileList {
    pub archive_name: String,
    pub version: u32,        // 0x68 or 0x69
    pub total_files: u32,
    pub folders: Vec<String>,
    pub files: Vec<BsaFileEntry>,
}
```

#### 3.2.3 前端：React 组件

```
BsaBrowser.tsx
├── 左侧：文件夹树（可按文件夹筛选）
├── 右侧：文件列表（名称、大小、压缩状态）
├── 顶部：地址栏 + 打开 BSA 按钮
└── 底部：提取按钮（单文件 / 选中 / 全部）
```

#### 3.2.4 核心逻辑改造

在 `xt-core/bsa/mod.rs` 中新增：
- `BsaArchive::list_files() -> Vec<(String, u64, bool)>` — 遍历文件记录表
- `BsaArchive::extract_file(name: &str) -> Vec<u8>` — 提取单个文件
- 保持现有 `extract_strings_folder()` 不变（向后兼容）

**⚠️ BSA 实例隔离**：T16 的 BsaBrowser 使用**独立** `BsaArchive` 实例打开 `.bsa` 文件。不与 `AppState.strings` 的加载路径共用同一 BSA 实例——避免浏览操作副作用影响已加载的字符串数据。用户手动提取的文件输出到用户指定目录，不自动注入当前翻译会话。

#### 3.2.5 BA2 格式评估

- Fallout 4 使用 BA2（General 格式）
- 如时间允许，在 xt-core 中新建 `ba2` 模块，复用 BSA 的文件列表/提取模式
- 否则标记为 TODO，仅支持 Skyrim/SSE 的 BSA 格式

### 3.3 分步执行

| 步骤 | 内容 | 预估 |
|------|------|------|
| 3.3.1 | 分析 `xt-core/bsa/` 现有代码，设计 API 接口 | 0.5 天 |
| 3.3.2 | 实现 `BsaArchive::list_files()` + `extract_file()` | 1 天 |
| 3.3.3 | 添加 DTO + IPC 命令（list_bsa_files, extract_bsa_file, extract_bsa_folder） | 1 天 |
| 3.3.4 | 创建 BsaBrowser React 组件（文件夹树 + 文件列表）— **所有 UI 文本使用 `t('key')`** | 2 天 |
| 3.3.5 | 集成到 MenuBar + App.tsx，添加快捷入口 | 0.5 天 |
| 3.3.6 | 端到端测试（Skyrim + SSE 真实 .bsa 文件） | 1 天 |
| 3.3.7 | 补充 BSA 模块单元测试（list_files / extract_file） | 0.5 天 |

> **i18n 预埋**：步骤 3.3.4 创建组件时直接使用 `t('key')`，在 `ui/src/locales/zh-CN/translation.json` 同步添加新 key。这样 T22 时无需重新改造 BsaBrowser。

---

## 四、T17：PEX 脚本字符串提取 [P2]

> **范围限定**：仅实现 PEX 解析 + 可翻译字符串提取 + UI 展示。**不追求完整反编译（伪代码生成）**，写回 PEX 留作 v2。

### 4.1 现状分析

- Rust 端：**零实现**
- Delphi 参考：`TESVT_scriptPex.pas`（47KB，约 1,200 行）
- PEX 是 Papyrus 编译后的二进制格式，包含：Header、StringTable、DebugInfo、UserFlags、ObjectInfos

### 4.2 技术方案

#### 4.2.1 PEX 二进制格式

```
PEX Header (9 bytes):
  magic: u32   = 0xFA57C0DE
  major: u8    = 3
  minor: u8    = 10 (Skyrim SE)
  game_id: u16

String Table:
  count: u16
  strings: [length: u16, data: bytes] × N

Debug Info:
  mod_time: u64
  string_count: u16
  strings: [length: u16, data: bytes] × N

User Flags:
  count: u16
  flags: [name_idx: u16, flag_idx: u8] × N

Object Infos:
  count: u16
  objects: [name_idx: u16, size: u32, ...] × N
```

#### 4.2.2 新模块结构

```
crates/xt-core/src/pex/
├── mod.rs          # 模块入口
├── parser.rs       # PEX 二进制解析器
├── decompiler.rs   # 伪代码生成（可翻译字符串提取）
└── types.rs        # PexHeader, PexObject, PexFunction 等类型
```

#### 4.2.3 IPC 命令

```rust
// 解析 PEX，提取可翻译字符串
#[tauri::command]
async fn parse_pex_strings(pex_path: String)
    -> Result<PexStringsResponse, String>

// PexStringsResponse {
//   script_name: String,
//   objects: Vec<PexObjectInfo>,               // 脚本对象列表
//   translatable_strings: Vec<PexTranslatable>, // 可翻译字符串
// }

// PexTranslatable {
//   object_name: String,    // 所属脚本对象
//   string_type: String,    // "DebugString" | "PropertyName" | "StringLiteral"
//   original: String,       // 原文
//   translation: String,    // 译文（空字符串为未翻译）
//   line_hint: u32,         // DebugInfo 行号提示
// }
```

**写回暂缓**：`compile_pex_strings(pex_path, translations) -> Result<(), String>` 留作 v2 功能。当前迭代仅提取 + 展示，不修改二进制 PEX。原因是 PEX 字符串写回需要重建 StringTable 偏移量，需额外 2-3 天验证。

#### 4.2.4 前端：PexPanel 组件

- 左侧：脚本对象树（Script → State → Function）
- 右侧：可翻译字符串列表（原文 + 译文对照）
- 底部：字符串编辑器（复用 EditorPanel 模式）
- 导出：支持将翻译结果导出为 XML（可被现有 import_xml 流程使用），作为不写回 PEX 的过渡方案

### 4.3 分步执行

| 步骤 | 内容 | 预估 |
|------|------|------|
| 4.3.1 | 逆向分析 Delphi `TESVT_scriptPex.pas`，梳理 PEX 格式细节 | 1.5 天 |
| 4.3.2 | 实现 PEX Header + StringTable 解析 | 1 天 |
| 4.3.3 | 实现 ObjectInfo / StateInfo / FunctionInfo 解析 | 2 天 |
| 4.3.4 | 实现可翻译字符串提取（DebugString、PropertyName、StringLiteral） | 1 天 |
| 4.3.5 | 添加 IPC 命令 + DTO | 0.5 天 |
| 4.3.6 | 创建 PexPanel 前端组件 — **所有 UI 文本使用 `t('key')`** | 1.5 天 |
| 4.3.7 | 真实 PEX 文件测试（Skyrim SE scripts/）+ 单元测试 | 1.5 天 |

> **i18n 预埋**：同 T16，PexPanel 所有文本直接走 `t()`，无需 T22 时二次改造。

---

## 五、T18：FUZ 音频映射 [P2]

### 5.1 现状分析

- Rust 端：**零实现**
- FUZ 格式 = WAV 音频数据 + LIP 唇形同步数据，用于 NPC 对话
- 核心价值：将翻译字符串映射到对应音频，帮助译者在对话上下文中理解文本

### 5.2 技术方案

```
crates/xt-core/src/fuz/
├── mod.rs       # FUZ 解析器
└── types.rs     # FuzHeader, FuzFile
```

#### FUZ 格式

```
FuzHeader (12 bytes):
  magic: [u8; 4]  = b"FUZE"
  lip_size: u32    # LIP data size
  wav_size: u32    # WAV data size (remaining data = wav_size bytes of WAV)
```

#### IPC 命令

```rust
#[tauri::command]
async fn load_fuz_mapping(esp_dir: String)
    -> Result<Vec<FuzMapping>, String>

// FuzMapping {
//   response_id: u32,       // RESP form ID
//   dialog_text: String,     // 对话文本
//   fuz_file: String,        // FUZ 文件名
//   wav_duration_secs: f32,  // 音频时长
//   has_translation: bool,
// }
```

### 5.3 分步执行

| 步骤 | 内容 | 预估 |
|------|------|------|
| 5.3.1 | 分析 Delphi 原版 FUZ 处理逻辑 | 0.5 天 |
| 5.3.2 | 实现 FUZ 解析器 + WAV duration 计算 | 1 天 |
| 5.3.3 | 实现 FUZ 文件扫描 + 与 RESP/INFO 字符串关联 | 1 天 |
| 5.3.4 | 添加 IPC 命令 + 音频播放（Tauri 原生或 Web Audio） | 1 天 |
| 5.3.5 | 前端：StringTable 增加音频播放列 — **新增 UI 文本使用 `t('key')`** | 0.5 天 |

---

## 六、T20：NPC 地图 / 对话视图 [P2]

### 6.1 目标

将平铺的字符串列表按对话上下文组织：
- **对话视图**：按 DIAL/QUST/SCEN 分组，展示对话树结构（INFO 节点）
- **NPC 视图**：按 NPC_ 记录分组，展示每个 NPC 关联的所有对话

### 6.2 技术方案

#### 6.2.1 后端

```rust
// 从已加载的 SkyString 列表中构建对话树
#[tauri::command]
async fn build_dialog_tree()
    -> Result<DialogTree, String>

// DialogTree {
//   quests: Vec<QuestDialogs>,
//   npcs: Vec<NpcDialogs>,
// }

// QuestDialogs {
//   quest_edid: String,
//   dial_groups: Vec<DialogGroup>,  // DIAL record → Vec<INFO>
// }

// NpcDialogs {
//   npc_edid: String,
//   npc_name: String,
//   dialogues: Vec<DialogStringInfo>,
// }
```

#### 6.2.2 前端

- 新 Tab：`对话` — 替代或补充当前 StringTable
- 左侧：Quest/NPC 列表
- 中间：对话树（嵌套 INFO 节点，缩进展示）
- 右侧：选中条目的编辑器（复用 EditorPanel）

### 6.3 分步执行

| 步骤 | 内容 | 预估 |
|------|------|------|
| 6.3.1 | 实现 `build_dialog_tree` 后端逻辑（QUST→DIAL→INFO 关联） | 1.5 天 |
| 6.3.2 | 创建 DialogView React 组件（Quest 列表 + 对话树）— **使用 `t('key')`** | 1.5 天 |
| 6.3.3 | 创建 NpcView React 组件（NPC 列表 + 关联对话）— **使用 `t('key')`** | 1 天 |
| 6.3.4 | 集成编辑功能 + 与主 StringTable 联动 | 1 天 |

---

## 七、T22：UI 多语言 i18n [P3]

### 7.1 技术选型

- 框架：`react-i18next` + `i18next`
- 配置：JSON 语言文件（`ui/src/locales/{zh-CN,en}/translation.json`）
- 语言切换：Zustand store `language` + localStorage 持久化

### 7.2 改造范围

```
ui/src/
├── locales/
│   ├── zh-CN/
│   │   └── translation.json
│   └── en/
│       └── translation.json
├── i18n.ts              # i18next 初始化
└── components/          # 所有组件中的硬编码中文 → t('key')
    ├── MenuBar.tsx       ~25 个 key
    ├── SidePanel.tsx     ~10 个 key
    ├── StringTable.tsx   ~15 个 key
    ├── EditorPanel.tsx   ~20 个 key
    ├── BatchPanel.tsx    ~30 个 key
    ├── BsaBrowser.tsx    （T16 新增组件，直接双语）
    └── PexPanel.tsx      （T17 新增组件，直接双语）
```

### 7.3 分步执行

| 步骤 | 内容 | 预估 |
|------|------|------|
| 7.3.1 | 安装依赖 + 初始化 i18next 配置 | 0.5 天 |
| 7.3.2 | 提取所有硬编码中文 → 生成 zh-CN.json（含 T16-T20 新增 key） | 1 天 |
| 7.3.3 | 翻译 en.json（英文） | 1 天 |
| 7.3.4 | 现有组件 `<t>` / `useTranslation` 替换 | 1 天 |
| 7.3.5 | 添加语言切换入口（MenuBar 下拉）+ localStorage 持久化 | 0.5 天 |
| 7.3.6 | 完整回归测试（中文/英文切换后全部页面 UI 正常） | 0.5 天 |

> **关键策略**：T16-T20 新增的组件（BsaBrowser、PexPanel、DialogView、NpcView）在创建时**直接使用 `t('key')`** 并同步写到 `locales/zh-CN/translation.json`。T22 时只需要翻译 `en.json` + 现有组件改造，无需碰新组件。由此 T22 的 4.5 天包含了些许 buffer。

---

## 八、风险与依赖

| 风险 | 等级 | 缓解措施 |
|------|------|---------|
| BA2 格式复杂度超出预期 | 中 | T16 限定 BSA 范围，BA2 另开任务 |
| PEX 指令集理解不足 | 高 | 仅实现字符串提取（StringTable + DebugString），**不追求完整反编译**；写回 PEX 留 v2 |
| PEX 格式版本差异（Skyrim vs FO4 vs Starfield） | 中 | 先用 Skyrim SE PEX 验证，其他游戏逐个适配 |
| FUZ 文件不可获取（需真实游戏数据） | 低 | 已有 Skyrim SE 数据路径 |
| 对话树逻辑复杂度（DIAL/INFO 多分支、跨 DIAL 引用） | 中 | 先实现扁平 DIAL→INFO 分组，再迭代对话树；降级方案为纯分组视图 |
| 所有新组件 UI 文本遗漏 i18n | 中 | 每个新组件的步骤清单中**显式标注**"使用 `t('key')`"；T22 验收标准包含新增组件的语言切换

---

## 九、验收标准

### T16 验收标准
- [ ] 可打开任意 Skyrim/SSE `.bsa` 文件，浏览完整目录树
- [ ] 可预览文本文件内容
- [ ] 可提取单个/批量文件到指定目录
- [ ] 大文件（>100MB BSA）加载不阻塞 UI

### T17 验收标准
- [ ] 可解析 Skyrim SE `.pex` 文件
- [ ] 可提取可翻译字符串（DebugString、PropertyName、StringLiteral）并显示在 UI 中
- [ ] 翻译结果可导出为 XML（可被现有 `import_xml` 流程使用）
- [ ] 单元测试覆盖 Header / StringTable / ObjectInfo 三层解析

> **写回 PEX**（`compile_pex_strings`）标记为 v2 功能，不纳入本次验收。

### T18 验收标准
- [ ] 可扫描 Skyrim `Sound/Voice/` 目录
- [ ] 字符串列表中对应对话条目显示音频图标
- [ ] 点击可播放关联音频

### T20 验收标准
- [ ] 对话视图正确展示 QUST→DIAL→INFO 层次结构
- [ ] NPC 视图正确关联 NPC_ → DIAL → INFO
- [ ] 在对话视图中编辑字符串后，主列表同步更新

### T22 验收标准
- [ ] 切换语言后所有 UI 文本即时生效
- [ ] 中文/英文覆盖所有界面元素
- [ ] 新增字符串不会自动翻译为空

---

## 十、里程碑

```
M0 [Week 1 始]  T0 冒烟测试完成
M1 [Week 2 末]  T16 BSA 浏览器完成
M2 [Week 4 末]  T17 PEX 字符串提取完成
M3 [Week 5 末]  T18 FUZ 音频完成
M4 [Week 6 末]  T20 NPC/对话视图完成
M5 [Week 7 末]  T22 i18n 完成 → xTranslator Rust 重写 v1.0
```

> **v2 展望**（不纳入本计划）：PEX 写回、BA2 格式、完整反编译、ESP 模式编辑写入、ESM 缓存、MCM 翻译、ESPCompare。
