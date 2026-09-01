# Delphi xTranslator 功能等价开发计划

> 更新日期：2026-09-01  
> 对比基线：`legacy/original-delphi/`（xTranslator 1.6.0） vs 当前 Rust/Tauri 重写版  
> 目标：记录**经过源码核对的真实功能差距**，并作为后续 Delphi parity 工作的执行入口。

## 1. 文档定位

当前仓库已有 `feature_comparison.md`、`development_roadmap.md`、`delphi_rust_fix_plan.md` 等历史对比文档，但其中一部分结论已经被后续实现推翻，也有一些新实现被旧文档低估。

本文件不按“模块是否存在”判断完成度，而按以下标准判断是否与 Delphi 原版等价：

1. **用户工作流是否能完整走通**，而不只是底层函数存在。
2. **行为选项是否保留**，尤其是 SST/XML 应用、搜索、归档和批处理行为。
3. **格式语义是否一致**，包括 PEX opcode、SST 状态语义、XML 元数据等。
4. **多游戏上下文是否真正贯穿主流程**，不能仅以 `GameId` 枚举或 `Data/<Game>` 目录存在作为“已支持”。
5. UI 可以现代化，但不能因为 UI 重构而丢失原版能力。

本轮核对主要参考：

- `legacy/original-delphi/TESVT_main.pas`
- `legacy/original-delphi/TESVT_commandProcessor.pas`
- `legacy/original-delphi/TESVT_AdvSearch.pas`
- `legacy/original-delphi/TESVT_ApplySSTOpts.*`
- `legacy/original-delphi/TESVT_XMLExportOpts.*`
- `legacy/original-delphi/TESVT_bsa.pas`
- `legacy/original-delphi/TESVT_scriptPex.pas`
- `legacy/original-delphi/TESVT_DefUIGen.*`
- 当前 `crates/xt-core/`、`src-tauri/`、`ui/` 实现

---

## 2. 当前结论

Rust/Tauri 重写已经完成了绝大多数**核心翻译引擎**：ESP/ESM 解析与写回、STRINGS 三格式、SST、XML、BSA/BA2 读取、PEX 字符串写回、FUZ、MCM、启发式匹配、翻译 API、TCSC、缓存、撤销/重做等均已有实际实现。

剩余差距主要集中在四类：

- **主工作流没有真正接通的能力**：多游戏上下文、Localized/Hybrid 加载策略。
- **原版高级工作流缩水**：XML Export options；Apply SST、Advanced Search、BatchProcessor 已恢复（DP-03/DP-04/DP-05）。
- **格式级兼容差距**：PEX opcode、XML EDID 信息、归档注入。
- **低频辅助工具未移植**：DEFUI Component Generator、Codepage 手动覆盖、部分旧式工具窗。

因此，当前版本更准确的描述是：

> 核心翻译引擎完成度很高，但还不是 Delphi xTranslator 1.6.0 的完整功能等价重写。

---

## 3. 优先级总表

| ID    | 优先级 | 差距                              | 当前状态                       | Delphi 参考                                                             | 当前实现入口                                                                   |
| ----- | ------ | --------------------------------- | ------------------------------ | ----------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| DP-01 | P0     | 多游戏上下文贯穿主流程            | ✅ **已完成**                   | `TESVT_main.pas` 游戏状态                                               | `game_detect.rs`, `appStore.ts`, `GroupedMenuBar.tsx`, `commands.rs::load_esp` |
| DP-02 | P0     | PEX opcode / 反编译语义           | ⏳ **真实 LE fixture 待补**     | `TESVT_scriptPex.pas`                                                   | `crates/xt-core/src/pex/decompile.rs`                                          |
| DP-03 | P0     | Apply SST 高级选项                | ⏳ **主体完成，VMAD/L3 待闭环** | `TESVT_ApplySSTOpts.*`                                                  | `matching.rs`, `commands.rs::load_sst`                                         |
| DP-04 | P1     | Advanced Search                   | ✅ **已完成**                   | `TESVT_AdvSearch.*`                                                     | `ui/src/components/AdvSearchDialog.tsx`, `appStore.ts`                        |
| DP-05 | P1     | BatchProcessor 命令脚本           | ✅ **已完成（L3 交叉验证待补）** | `TESVT_commandProcessor.*`, `TESVT_main.pas::batchCommands/runCommands` | `xt-core::command_processor`、`src-tauri::command_processor`、`CommandProcessorDialog.tsx` |
| DP-06 | P1     | BSA/BA2 注入                      | **未实现**                     | `TESVT_bsa.pas::InjectData`                                             | `crates/xt-core/src/bsa`, `ba2` 目前主要读取/提取                              |
| DP-07 | P1     | Localized / Hybrid 加载策略       | **部分实现**                   | `TESVT_delocOpts.*`, MainLoader                                         | `commands.rs::load_esp`                                                        |
| DP-08 | P1     | XML Export 选项                   | **缩水**                       | `TESVT_XMLExportOpts.*`                                                 | `XmlExportRequest`, `commands.rs::export_xml`                                  |
| DP-09 | P2     | XML EDID 元数据完整性             | **部分缺失**                   | `TESVT_XMLFunc.pas`                                                     | `SkyString`, `xml/mod.rs`                                                      |
| DP-10 | P2     | DEFUI Component Generator         | **未实现**                     | `TESVT_DefUIGen.*`, `doComponentGenerator`                              | 无对应实现                                                                     |
| DP-11 | P2     | Codepage 手动选择/覆盖            | **底层有，工作流缺**           | `TESVT_Codepage.*`, `TESVT_ChooseCP.*`                                  | `strings/codepage.rs`                                                          |
| DP-12 | P3     | Yandex / freeApi provider         | **未实现**                     | `TESVT_TranslatorApi.pas`                                               | `translation_api/`                                                             |
| DP-13 | P3     | MS Word 拼写检查后端              | **未实现**                     | `TESVT_SpellCheck.pas`                                                  | 当前仅 Hunspell                                                                |
| DP-14 | P3     | AddId / OldDialogStyle 等低频工具 | **未实现或未等价**             | 对应 Delphi 窗体                                                        | 当前 UI 无直接等价入口                                                         |

---

## 4. P0：先修影响正确性的差距

### DP-01 多游戏上下文贯穿主流程

#### 状态

✅ **2026-09-01 完成。** 主编辑器、批处理和辅助工具现在共享同一套游戏上下文规则，不再把语言或文件路径当作游戏类型来源。

#### 目标

建立单一的 `currentGame` 状态，并让它贯穿：

- ESP/ESM 加载
- record definitions
- codepage
- vocabulary
- data configs
- PEX
- Header Processor / Wizard
- 归档与 Strings 自动发现
- 导出/Finalize

#### 实现要求

1. 在 Zustand store 中加入明确的 `currentGame`。
2. 游戏来源允许：自动检测 + 用户显式覆盖。
3. 自动检测失败时不能静默假定 Skyrim SE；应明确显示当前回退状态或要求选择。
4. `loadEsp`、`loadVocabulary`、`loadDataConfigs`、PEX 相关 IPC 全部使用同一游戏上下文。
5. 移除 `language === "english" ? "SkyrimSE" : ...` 一类把“语言”当“游戏”的逻辑。

#### 验收

- 同一套主 UI 分别加载 Skyrim SE、Fallout 4、Fallout 76、Starfield 测试文件时，日志能显示实际使用的 `GameId` 和对应 Data 目录。
- 不允许 Fallout/Starfield 文件走到 SkyrimSE record_defs/codepage。
- 增加前端和后端测试覆盖游戏上下文传递。

#### 完成内容

- `crates/xt-core/src/esp/game_detect.rs` 按 Delphi `getGameByFormVersion` 移植 TES4 Form Version 游戏检测；未知值返回 `None`，不伪装成 Skyrim SE。
- `GameId::as_str()` / `GameId::from_alias()` 成为统一游戏标识入口，后端不再散落重复 alias match。
- `load_esp` 采用 `显式工作区 > TES4 自动检测 > fallback`，并通过 `LoadEspResponse.game_id / detected_game_id / game_source` 把结果返回前端；缓存命中路径保持相同行为。
- Zustand 增加 `currentGame / detectedGame / gameSource / gameSelectionMode`。Auto 模式把判断权交给 ESP；Manual 模式才向后端传用户选择。
- 工具栏增加 Game Workspace 下拉框；切换工作区时若已有 ESP，会重新加载当前文件，保证 record definitions、codepage 和前端状态同步。
- `config.json` 持久化 `last_game` 与 `game_selection_mode`。Auto 模式即使保留历史 `last_game`，启动后也不会把它当成强制覆盖。
- `Vocabulary`、`Data Configs`、`PEX`、`Header Wizard/Processor` 均使用全局 `currentGame`；删除了 `language === "english" ? "SkyrimSE" : ...` 类型逻辑。
- `BatchPanel` 不再通过文件路径猜游戏；Auto 模式下每个 ESP 由后端读取 TES4 Form Version，Manual 模式使用显式工作区。
- 自动检测失败时 `currentGame` 保持不可信（`null`），阻止 PEX/Header 等后续工具误用 fallback；UI 会要求用户显式选择游戏。
- 显式工作区与 ESP 检测结果不一致时给出警告，保留用户选择，不偷偷切换，行为与 Delphi 的 workspace mismatch 思路一致。

#### 验证结果

- `cargo test -p xt-core --lib` → **310 passed / 0 failed**。
- `cargo test -p xt-core --lib game_detect` → **8 passed / 0 failed**。
- `cargo check -p xtranslator-tauri` → **通过**。
- `npx tsc --noEmit` → **通过**。
- `npm run test` → **25 passed / 0 failed**，其中 `gameContext.test.ts` 新增 6 项 Auto/Manual/持久化语义测试。
- Playwright 全套在正确的 `VITE_E2E=true` 环境下为 **58 passed / 6 failed**；6 项失败均是现有布局重构后的旧选择器/旧 tab 断言（Dialogs、ESP Tree、Quests、旧 MenuBar、ESP Compare、FUZ），与 DP-01 无关，另行修复。
- `git diff --check` → **通过**。

真实 Fallout 4 / Fallout 76 / Starfield 安装文件的跨游戏 smoke test 仍属于发布 QA；它不再是 DP-01 的实现缺口。

---

### DP-02 PEX opcode 与原版对齐 (⏳ 待完成 - 等待真实 Little-Endian fixture)

#### 状态

⏳ **待完成（主体与写回层已修复，真实 Little-Endian fixture 待补）。** 已对齐真实 Bethesda PEX 头部规范、大小端、GameID 建模、Object Body 字段顺序与字节级无损写回；仓库现已加入 Bethesda PapyrusCompiler 实际产出的 Skyrim SE Big-Endian fixture，并通过真实 parse/decompile/byte-for-byte roundtrip。FO4/FO76/Starfield Little-Endian 仍缺真实编译器产物，L3 交叉验证尚未完全闭环。

#### 目标

按**实际 PEX 版本 / 游戏**建立准确 opcode 表，而不是用一个统一枚举猜测所有游戏。

#### 实现要求

1. 逐条核对 Delphi `drawInstruction`、实际 PEX 格式资料和现有 fixtures。
2. 将 opcode 定义按 Skyrim / Fallout 4+ / Starfield 差异建模。
3. 未知 opcode 必须保留原始值并安全显示，不能静默映射成 `Nop`。
4. 反编译只负责展示；字符串写回路径继续保持二进制结构保留原则。

#### 完成内容

- **真实 Bethesda PEX 格式全链路支持（关键根因修复）**：
  - 彻底抛弃早期简化 PEX 头，全面对齐 Champollion 与 Delphi 原版 `TESVT_scriptPex.pas` 规范：支持 `Magic (0xFA57C0DE BE / 0xDEC057FA LE)` -> `Major` -> `Minor` -> `GameID` -> `CompilationTime` -> `SourceFileName (.psc)` -> `UserName` -> `ComputerName` -> `StringTable` -> `HasDebugInfo (u8)`。
  - 统一重构 `parser.rs`、`decompile.rs` 与 `compile.rs`，引入基于 `PexEndian` 的自适应 `PexReader` 读取器，自动根据前 4 字节 Magic 识别 Skyrim Big-Endian（`[0xFA, 0x57, 0xC0, 0xDE]`）与 FO4/Starfield Little-Endian（`[0xDE, 0xC0, 0x57, 0xFA]`）。
- **写回边界修正（P0 roundtrip 修复）**：
  - 修正 `parser.rs` 的 `header_raw` / `data_raw` 截取边界：游标建立在完整 `raw_bytes` 上，位置即绝对偏移，不再丢失 magic 的 +4；
  - `header_raw` 现在**包含 stringTableCount (u16)**，与 Delphi `headerPexBuffer` 在读 tableCount 后截取的行为一致；
  - `compile_pex_bytes` 恢复 `header_raw → string entries → data_raw` 的完整写回，恢复**无翻译时 parse→compile byte-for-byte 相同**的大小端双 roundtrip 测试（`test_roundtrip_big_endian_byte_for_byte` / `test_roundtrip_little_endian_byte_for_byte`）。
- **Object Body 字段顺序对齐 Delphi `checkObjectData` / `checkVariables` / `checkFunction` / `checkProperty`（P0 结构修复）**：
  - Object: `parentClass(u16)` → `docString(u16)` → `[LE] uConst(u8)` → `userFlags(u32)` → `autoStateName(u16)` → `[LE] Structs` → `Variables` → `[game_id==4] Guards` → `Properties` → `States`；
  - 纠正 Object `userFlags` 为单个 `u32`（不再读 `u16 count + entries`）；
  - 补 FO4+ 的 `uConst(u8)` 与 LE Struct 段；
  - Variable 改为 `name → type → uFlags(u32) → VarData → [LE] group(u8) → [struct] docType`（去掉虚构的 doc_idx / user_flags(u32)）；
  - Guard 仅在 `game_id == 4` 读取，且每项只是一个字符串表 ID（去掉虚构的 `sc: u32 + user_flags[]`）；
  - Properties 直接读 `count`，Property 结构为 `name → type → doc → uFlags(u32) → flag(u8)`，并按 flag 位读取 `AutoVar` / `ReadHandler` / `WriteHandler`（handler 为无名的函数体）；
  - Function 改为 `name → returnType → doc → uFlags(u32) → flags(u8) → params → locals → instructions`（去掉虚构的 user_flags 段）。
- **纠正 GameID 游戏体系建模**：
  - `GameID: 1` => Skyrim / Skyrim SE / Skyrim VR（Big-Endian，支持 `0x00..=0x23`）；
  - `GameID: 2` => Fallout 4（Little-Endian，支持 `0x00..=0x2E`）；
  - `GameID: 3` => Fallout 76（Little-Endian，支持 `0x00..=0x2E`）；
  - `GameID: 4` => Starfield（Little-Endian，支持 `0x00..=0x32`，涵盖 Guard 与 GetAllMatchingStruct）。
- **变长参数完整覆盖（P0 修复）**：修复了 `is_extended_proc()`，将 `extendedproc` 完整扩展为 `0x17 (Callmethod)`, `0x18 (Callparent)`, `0x19 (Callstatic)`, `0x30 (GuardLock)`, `0x31 (GuardUnlock)`, `0x32 (GuardTryLock)`。严格按照 Delphi 字节流规则提取 `extraArg` 并消费变长操作数，避免了 Starfield Guard 指令导致的后续操作码字节流错位。
- **反编译语义纠正**：纠正了 `ArrayGetElement (0x20)` 的格式化字符串，从误写的 `array[index] = val` 恢复为 Delphi 原版 `dest = array[index]`。
- **Delphi 格式细节严格对齐**：
  - `PexValue::Float` 输出严格使用 `%.4f`（例如 `1.0000`）；
  - `includeNewArray` 严格对齐 Delphi：仅当类型字符串中包含 `]` 时插入维度，否则原样返回；
  - `::NoneVar` 作为方法调用返回值时自动抑制赋值前缀。
- **未知操作码保护**：引入 `Opcode::Unknown(u8)`，保留原始字节码并安全发射 `unknown OpCode: XX`。
- **二进制测试覆盖**：
  - Skyrim Big-Endian PEX 反编译测试（`test_decompile_real_skyrim_big_endian_pex`）；
  - Starfield Little-Endian PEX 反编译测试（`test_decompile_real_starfield_little_endian_pex`），验证 uConst/Struct/Guard 段；
  - 大小端双 byte-for-byte roundtrip 测试（`test_roundtrip_big_endian_byte_for_byte` / `test_roundtrip_little_endian_byte_for_byte`）。
  - 新增 `crates/xt-core/tests/fixtures/pex/skyrim_se/XtPexFixture.pex.hex`：由 Skyrim Special Edition 自带 Bethesda `PapyrusCompiler.exe` 对项目自有 `XtPexFixture.psc` 实际编译产生，仅对 Header 中 UserName / ComputerName 做等长脱敏；`pex_real_fixture.rs` 验证真实编译产物能够 parse、decompile，并在无修改写回时 byte-for-byte 完全一致。
  - **剩余边界**：FO4 / FO76 / Starfield Little-Endian 路径仍只有按真实规范构造的字节测试，尚缺对应游戏编译器实际产出的 fixture。

#### 验证结果

- `cargo test -p xt-core --lib` → **307 passed / 0 failed**。
- `cargo test --workspace` → **全部通过**。
- `cargo check -p xtranslator-tauri` → **通过**。
- `npx tsc --noEmit` → **通过**。
- `git diff --check` → **通过**。

#### 待办（真实文件交叉验证）

- 放入至少一个真实游戏 / Papyrus 编译器产出的 **Skyrim `.pex`** 与 **FO4/Starfield `.pex`** fixture，跑通 `decompile` 与 `parse→compile→parse` roundtrip，并将这两项测试命名为真实 fixture 测试，取代现场拼接。

---

### DP-03 恢复 Apply SST 高级选项 (⏳ 主体完成，VMAD/L3 待闭环)

#### 原版能力

`TESVT_ApplySSTOpts.dfm` / `TESVT_ApplySSTOpts.pas` 暴露了：

**覆盖范围 (5 种)：**

- All (全部未锁定项)
- NoTrans Exclusive (仅未翻译项)
- NoTransAndPartials（保留 Delphi 原名；实际比较器排除 `translated` / `validated` / `incompleteTrans`，即严格未翻译项）
- Partial Only (仅部分翻译项)
- Selection (仅选中项，按稳定 `u32 id` 集合)

**匹配模式 (4 种)：**

- FORMID only (`sanitizeFormID(FormID)` + EDID hash + FIELD + index)
- FORMID + strict string control（上述键 + 原文精确 hash/source + index）
- FORMID + string control（上述 FormID/EDID/FIELD 键 + 原文精确 hash/source，放宽 index）
- String only（忽略 FormID，仅按原文精确 hash/source；重复源文再按 REC/FIELD 消歧，不走规范化/T4 模糊匹配）

**附加控制 (3 个)：**

- Apply Tag Only (仅打标签不修改译文)
- Reset StringState（在匹配前重置覆盖范围内的目标行；即使最终未命中也保持重置；不会把 SST 的 incomplete 强制升级为 translated）
- Restrict to Filter (仅限当前过滤可见的 `u32 id` 集合)

#### 落地成果

1. **Rust 核心引擎 (`crates/xt-core/src/matching.rs`)**：
   - 实现了 `SstOverwriteScope`、`SstMatchMode`、`SstApplyOptions` 及对应的稳定 `u32 id` 范围判定。
   - SST V4 FormID 模式已改为使用真实 `form_id` + Delphi `sanitizeFormID` 规则，不再把 `str_id` 三元组误当 FormID。
   - `StringOnly` 已改回 Delphi 的精确源文路径，不再借用 T3 规范化 / T4 Jaccard 模糊匹配。
   - `reset_state` 已按 Delphi 改为候选预重置语义；未命中候选也会被重置，且 SST incomplete/locked 状态仍按来源保留。
   - SST 正常应用与 `tag_only` 均同步 `colab_id`；`restrict_to_filter=true` 缺少 `filtered_ids` 时 fail-closed。
   - matching 专项测试现为 35 项全绿；新增覆盖 StringOnly 未命中 reset、同语言不自动应用，以及 Tag Only 现代契约；原有 XML/通用 4-Tier 路径保持独立。
2. **IPC DTO (`crates/xt-shared/src/dto.rs`)**：
   - 定义 `SstOverwriteScopeDto`, `SstMatchModeDto`, `SstApplyOptionsDto`。
3. **Tauri 后端命令 (`src-tauri/src/commands.rs`)**：
   - `load_sst` 升级支持可选 `options: Option<SstApplyOptionsDto>` 参数。
4. **前端交互 (`ui/src/components/ApplySstDialog.tsx` & `ui/src/api/strings.ts`)**：
   - 新建 `ApplySstDialog`（支持 5 种覆盖范围单选、4 种匹配模式单选、3 种附加复选框、回车直接应用与 Esc 关闭）。
   - 在 `MenuBar.tsx` 和 `GroupedMenuBar.tsx` 中全面集成，自动采集当前选中的 `selectedIds` 与过滤后的 `items` ID 列表传递给后端。
   - 增加前端 Vitest 单元测试 `ApplySstDialog.test.tsx`。

#### 当前验收状态

- `cargo test -p xt-core matching::tests --lib` → **36 passed / 0 failed**。
- `cargo test --workspace` → **通过**；其中 `xt-core` **322 passed / 0 failed**，其余 workspace 测试与 doc-tests 全绿，release-only 测试按测试声明保持 ignored。
- `cargo check -p xtranslator-tauri` → **通过**。
- `npx vitest run` → **27 passed / 0 failed**（4 个测试文件）。
- `npx tsc --noEmit` → **通过**。
- `git diff --check` → **通过**（仅报告仓库现有 LF→CRLF 提示，无 whitespace error）。
- `cargo fmt --all -- --check` → **未通过**，但输出包含 DP-02/CLI 等本轮范围外的大量既有未格式化改动；本轮未为追求全局格式绿灯而改写这些无关文件。
- **VMAD 进展**：已完成 `getfProcCompareOptVMADString` 的防污染保护机制，StringOnly 模式下 5 档 Scope 矩阵已全量单测覆盖（All / NoTransExclusive / NoTransAndPartial 屏蔽；PartialOnly / Selection 放行）；VMAD 其余专用 FormID/comparator Apply 路径仍待闭环。
- **尚未完成项（L3）**：按本文件第 9 节定义，SST apply 需要 Delphi / 真实游戏交叉验证；完成前不得宣称 100% Delphi parity。

---

## 5. P1：恢复原版高级工作流

### DP-04 Advanced Search

#### 状态

✅ **2026-09-01 完成。** 独立 Advanced Search 面板已实现，简单搜索框保留作为快速搜索；两者互斥（Advanced Search 激活时接管文本过滤）。Keyword 维度因依赖 ESP keyword 字典数据管道（与 DP-09 同源）标记为占位，不在 UI 上假装生效。

#### 原版能力

`TESVT_AdvSearch` 支持独立条件：

- Source
- Translated
- EDID/FormID
- REC
- FIELD
- Keyword

其中多个文本条件可以**独立切换 Regex**，并可保存/删除搜索 preset。

#### 当前差距

当前 `appStore.applyFilterAndSort()` 的文本过滤主要是把一个输入同时匹配：

- source
- translation
- record_sig

这不能替代原版 Advanced Search。

#### 目标

新增独立 Advanced Search 面板，简单搜索框继续保留作为快速搜索。

#### 实现内容

- **Rust 数据管道**：`SkyString` 新增 `edid: Option<String>` 字段，ESP 解析时从记录 EDID 字段填充（普通字段与 VMAD 字符串均覆盖），`SkyStringDTO` 暴露 `edid` 供前端搜索与展示。
- **过滤核心（`appStore.ts`）**：新增 `AdvSearchState`（六维度 + 每字段独立 Regex + Source/Translated 比较模式），`applyAdvancedFilter()` 按 Delphi `launchSearchTimer` Advanced 分支语义实现：
  - Source → 源文本子串/正则；
  - Translated → 译文子串/正则；
  - EDID/FormID → `$`/`0x` 十六进制 FormID 精确匹配（归一化去前缀），否则 EDID 文本子串/正则；
  - REC / FIELD → 记录/字段签名精确匹配（大小写不敏感）；REC 框支持 `REC:FIELD` 联合语法；
  - 比较模式 `(.*)||(.*)` / `(.*)=(.*)` / `(.*)!=(.*)` 对应 any / eq / neq。
  - 匹配谓词预编译 + 一次遍历，避免大数据集逐行 `new RegExp`。
- **`applyFilterAndSort`** 增加可选 `advSearch` 参数：非空时接管文本过滤；所有 store 重过滤路径（setAllItems / setFilter / setSort / replaceAll / undo-redo / 增量更新等）均传递当前 `advSearch`，保证激活期间条件持续生效。
- **UI（`AdvSearchDialog.tsx`）**：六输入框（Source / Translated / EDID·FormID / REC : FIELD / Keyword 占位），Source/Translated/EDID/Keyword 各自独立 Regex 切换，比较模式单选，preset 保存 / 载入 / 删除（localStorage 持久化，`xtranslator-advsearch-presets`），Enter 应用并关闭，Esc 关闭。
- **入口**：GroupedMenuBar → Search 菜单新增 "Advanced Search" 项。

#### 验收

- ✅ 每个搜索维度彼此独立（AND 组合）。
- ✅ Regex 开关按字段保存（`useRegex.source/translated/edid/keyword`）。
- ✅ 支持 REC:FIELD 联合条件（单框 `REC:FIELD` 语法 + 双框 REC/FIELD）。
- ✅ 支持保存、载入、删除 presets（localStorage 持久化）。
- ✅ 大数据集保持客户端可接受性能（预编译谓词 + 一次遍历）。
- ⚠️ Keyword 维度：依赖 ESP keyword 字典数据管道（`getKwd`/`findKeyWord` 的 Rust 等价物），当前标记为占位不可用；与 DP-09（EDID 元数据）同源，数据管道建立后可激活。

#### 验证结果

- `cargo test --workspace` → **全部通过**；xt-core **322 passed / 0 failed**。
- `cargo check -p xtranslator-tauri` → **通过**。
- `npx tsc --noEmit` → **通过**。
- `npx vitest run` → **57 passed / 0 failed**（5 个测试文件；新增 `advSearch.test.ts` 30 项：维度独立、Regex 分字段、EDID/FormID 匹配、比较模式、preset 状态、集成过滤）。
- `git diff --check` → **通过**（仅 LF→CRLF 提示，无 whitespace error）。
- **L3 注记**：搜索行为按 Delphi 源码逐条对齐（`TESVT_main.pas` 6195-6420 行 Advanced 分支），但未在 Delphi 原版上做自动化交叉验证；EDID 文本来源（解析期提取）与 Delphi `getEdidNameexport` 的差异已在实现中注明。

---

### DP-05 BatchProcessor 命令脚本

#### 状态

✅ **2026-09-01 完成（除 L3 交叉验证）。** 已直接对照仓库内 Delphi `TESVT_commandProcessor.dfm`、`TESVT_main.pas::batchCommands/runCommands` 落地完整白名单 parser，并把 processor 执行接入现有 ESP/SST/XML/Finalize 能力。全部 11 个原版命令均已有真实执行路径；剩余仅为原版示例/真实游戏文件的端到端交叉验证（L3）。

#### 原版能力

原版 BatchProcessor 不是普通“多文件翻译队列”，而是一个轻量脚本解释器。

脚本包含：

- `StartRule` / `EndRule`
- `global_vocabfolder`
- `global_importfolder`
- `global_exportfolder`
- `langsource`
- `langdest`
- `usedatadir`
- `exportsubfolder`

已确认的命令包括：

- `LoadFile`
- `CloseFile`
- `CloseAll`
- `Finalize`
- `GenerateDictionaries`
- `ApplySst`
- `ImportSst`
- `ImportXml`
- `LoadMasters`
- `SaveDictionary`
- `ApiTranslation`

#### 当前差距

现有 `BatchPanel` / `BatchExecutor` 是现代化多文件 translate/export 队列，功能有价值，但**不是原版 Command Processor 的替代品**。

#### 目标

保留现有 BatchPanel，同时新增独立的 command script parser + executor。

#### 架构要求

1. parser 只解析脚本文本为 AST/command list，不直接执行。
2. executor 只调用已有 IPC/core 能力，不重新实现翻译逻辑。
3. 每个命令有结构化日志、错误和停止策略。
4. 默认禁止脚本执行任意 shell 命令；只允许白名单内的 xTranslator command。
5. 支持加载/保存 `.txt` processor 文件和临时草稿恢复。

#### 已完成

- 新增 `crates/xt-core/src/command_processor.rs`，parser 只把 processor 文本解析成 `CommandProcessorScript → CommandRule → ProcessorCommand`，不执行任何应用操作。
- 全量识别 Delphi 当前 `runCommands()` 白名单：`LoadFile`、`CloseFile`、`CloseAll`、`Finalize`、`GenerateDictionaries`、`ApplySst`、`ImportSst`、`ImportXml`、`LoadMasters`、`SaveDictionary`、`ApiTranslation`。
- 对齐原版 rule/global 配置：`global_vocabfolder`、`global_importfolder`、`global_exportfolder`、`langsource`、`langdest`、`usedatadir`、`exportsubfolder`；`UseDataDir` 缺失或非法时按 Delphi `strtobooldef(..., true)` 回退为 `true`。
- `ApplySst` / `ImportSst` / `ImportXml` 按 Delphi `:<compareOption>:<applyMode>:<path>` 语法解析；使用三段 split，Windows `C:\...` 路径中的冒号不会被破坏。
- `ApiTranslation:<apiId>:<noTransOnly>` 已结构化建模。
- 未知 `Command=` 不再像 Delphi `runCommands()` 那样静默跳过，而是 fail-closed，报告源码行号；因此 processor 无法扩展成任意 shell 执行入口。
- 建立 `CommandProcessorHost` + `execute_command_processor()`：核心 executor 负责 rule/command 顺序、Stop/Continue 错误策略与结构化失败位置；具体 ESP/SST/XML/API 操作由应用层 host 实现，避免复制已有翻译逻辑。
- executor 错误报告包含 `rule_number`、`command_number`、源码 `line`、命令名和错误消息，为后续 UI 日志直接提供稳定数据。
- 新增 `src-tauri/src/command_processor.rs` 实现真实 Tauri host，并注册 `run_command_processor` IPC；实时发射 `command-processor-progress`，前端可以稳定显示 rule / command / line 级执行日志。
- `LoadFile` 已复用 `load_esp`；`UseDataDir=true` 严格按 Delphi 的 `Game_EspFolder + extractFileName(...)` 语义要求调用方提供游戏 `Data` 目录，不做路径名猜测。
- `ApplySst` / `ImportSst` 已复用 DP-03 的高级 SST 引擎。经 `TESVT_FastSearch.pas::getfProcCompareOpt/getProcSortCompare` 核对，processor 第一参数 `0..4` 精确映射五档 overwrite scope，第二参数 `0..3` 精确映射四档 match mode。
- `Finalize` 已按 TES4 localized 标志分流：localized 插件只导出目标语言 Strings；非 localized 插件走 ESP 字段写回，避免把字符串 ID 型字段错误改成内联文本。
- `CloseFile` / `CloseAll` 已清理 Rust 当前单活动 loader 的完整后端状态；由于 Rust 重写目前只有一个活动文件，两者在应用层效果等价。
- `SaveDictionary` 已接通 SST 保存；存在 `Global_VocabFolder` 时按 Delphi `${addon}_${LangSource}_${LangDest}.sst` 命名。Rust 尚无 Delphi `SSTUserFolder` 全局设置，因此未提供该 fallback 时会明确报错而不是猜目录。
- `ImportXml` 已可执行现有 Rust XML 导入，但当前 XML 管线仍使用通用 T1-T4 matcher；processor 的 `0/1/2` EDID/Strict/Relax comparator 语义依赖 DP-09 XML EDID/FormID 元数据闭环。执行报告会显式返回 warning，不宣称已经等价。
- `Global_ImportFolder` 已接入 import 路径解析；`Global_ExportFolder` 在 Rust 中作为显式输出基目录使用。需注明：Delphi 当前 `runCommands()` 虽解析该字段，但实际 `Finalize` 路径没有消费它，因此这是兼容脚本字段的现代化补全，不作为逐字节 parity 依据。
- 新增独立 `CommandProcessorDialog`：支持 `.txt` 打开/保存、localStorage 草稿恢复、Data 目录选择、Stop/Continue 策略、实时执行日志、失败/警告报告；现有现代 `BatchPanel` 保持不变。
- processor 执行结果会返回最后的活动文件上下文；前端同步 `espPath` / `stringsDir` / `currentGame`，脚本 `CloseFile/CloseAll` 后也会清除旧上下文，避免后端与 UI 状态漂移。
- `read_text_file` / 既有 `write_text_file` 作为 processor 编辑器文件 IO；processor 语言本身仍然只有固定 xTranslator 命令白名单，不存在 shell escape。
- **`ApiTranslation`**：按 Delphi `aApiBaseName` 数字 ID 映射到 Rust provider（0→azure、2→baidu、3→youdao、5→google、6→deepl、7→openai；1=Yandex、4=freeApi 明确报"未实现"，不猜映射）。候选集按 Delphi `StartApiTranslationArray(false,...)` 的 `compareOptNoTransAndPartialsExLocked` 语义排除 VMAD/locked/translated/incomplete/validated；等源字符串复用同一次 API 结果；成功后重置状态为 incomplete。`noTransOnly=1` 的 NoTranslation 预标记请求会显式返回 warning（Rust 无 lRulesNoTransListIn/Out 规则集）。
- **`GenerateDictionaries`**：读取 `Data/<Game>/vocabulary.txt` 的 STRINGS= 列表，逐个加载插件、按目标语言 Strings 回填翻译、按 Delphi `${addon}_${LangSource}_${LangDest}.sst` 命名生成 SST 到 `Global_VocabFolder`。执行前对当前 AppState 做快照、结束后恢复，避免污染用户正在编辑的工作区。
- **`LoadMasters`**：解析当前插件 TES4 声明的 masters，把继承的 FormID（本插件内无 EDID 的字符串）按 master slot 解析回 EDID 并回填 `sk.edid` / `edid_hash`。非 Starfield 按 Delphi 高字节规则，Starfield 按 FE/FD 与 normal/medium/light 分桶重建 owner slot（对齐 Delphi `buildInheritedData` 的 `getPluginType` 语义）。
- `ImportXml` 已可执行现有 Rust XML 导入，但当前 XML 管线仍使用通用 T1-T4 matcher；processor 的 `0/1/2` EDID/Strict/Relax comparator 语义依赖 DP-09 XML EDID/FormID 元数据闭环。执行报告会显式返回 warning，不宣称已经等价。

#### 当前验证

- `cargo test -p xt-core command_processor::tests --lib` → **9 passed / 0 failed**。
- `cargo test -p xt-core --lib` → **335 passed / 0 failed**。
- `cargo test -p xtranslator-tauri` → **7 passed / 0 failed**（含 command_processor 单元测试）。
- `cargo test --workspace` → **全部通过**。
- `cargo check -p xtranslator-tauri` → **通过**。
- `npx tsc --noEmit` → **通过**。
- `npx vitest run` → **59 passed / 0 failed**（6 个测试文件；新增 `CommandProcessorDialog.test.tsx` 2 项：默认脚本渲染与草稿持久化）。
- `npm run build` → **通过**。
- `git diff --check` → **通过**（仅仓库行尾转换提示，无 whitespace error）。

#### 下一步（L3）

1. 用原版 Delphi processor 示例和真实 Skyrim SE 文件跑 `LoadFile → ApplySst/ImportXml → Finalize → CloseFile` 端到端交叉验证，并锁成 fixture/E2E。
2. 随 DP-09 补齐 processor `ImportXml` 的 EDID/Strict/Relax comparator 语义及 fake `[FormID]` EDID hash 规则。

#### 验收

- 用原版示例 processor 文件跑通等价工作流。
- 命令 parser 有独立单元测试。
- 某一步失败时能明确指出 rule、command 和目标文件。

---

### DP-06 BSA/BA2 注入

#### 原版能力

Delphi `TESVT_bsa.pas` 从 2016 年起支持对已有 BSA/BA2 中的文件进行 replacement injection：

- BSA zlib / SSE LZ4
- BA2 GNRL zlib
- 重建 offset / packed size / size
- 临时文件写出后替换原文件

#### 当前差距

Rust 当前已有浏览、查找、提取和 GNRL 读取，但未实现 archive injection。

#### 目标

只实现“替换归档内已存在文件”，不顺手扩大成通用 BSA/BA2 authoring 工具。

#### 安全要求

- 永远写入临时归档。
- 完成完整结构校验后再原子替换。
- 替换前创建备份。
- 保留原 entry 的压缩策略，除非格式要求改变。
- DX10 texture archive 不因为 parity 工作被强行纳入；Delphi 主路径同样主要针对 GNRL。

#### 验收

- BSA v0x68/v0x69 replacement roundtrip。
- BA2 GNRL replacement roundtrip。
- 注入后能被 Rust 自己重新打开并读取替换后的文件。
- 有真实 Bethesda 工具/xEdit 的交叉验证样本时再加 L3 验证。

---

### DP-07 Localized / Hybrid 加载策略

#### 原版能力

原版针对 localized 插件允许用户选择：

- 常规加载
- 优先从 archive 读取 Strings
- 手动选择 Strings

#### 当前差距

`load_esp` 主要查找磁盘 `Strings` 目录；BSA/BA2 浏览能力和 ESP 主加载流程没有形成统一资源解析器。

#### 目标

把 Strings 来源抽象为明确策略：

```text
DiskPreferred
ArchivePreferred
Manual
```

并在加载结果中记录**每个 STRINGS 文件实际来自哪里**，方便排错。

#### 验收

- 同一 localized ESP 可分别从 loose Strings 和 archive Strings 加载。
- 用户手动选择可覆盖自动发现。
- UI 明确显示 Strings 来源。

---

### DP-08 XML Export 选项

#### 原版能力

原版 XML Export 支持：

- All
- Translated
- Selection
- Diff
- Split collaboration
- 可选 FUZ 数据

#### 当前差距

当前 `XmlExportRequest` 只有：

- `path`
- `dest_lang`

`export_xml` 直接通过 `sky_strings_to_xml_entries()` 收集当前可导出条目，行为基本固定为“已有译文”。

#### 目标

扩展请求 DTO，恢复导出范围和附加信息控制。

#### 验收

- 四种 export scope 有测试。
- Selection 使用稳定 IDs。
- Diff 定义必须在 SPEC 中明确，不凭 UI 名称猜实现。
- Split collaboration 与 `colab_id` 行为一致。

---

## 6. P2：补全信息和低频但有价值的能力

### DP-09 XML EDID 元数据

当前 XML parser/writer 已能读写 `<EDID>`，但 `SkyString` 不保存 EDID 文本，`sky_strings_to_xml_entries()` 因此无法稳定导出它。

建议不要只为 XML 临时查询记录树。更合理的是在解析期把需要跨工作流使用的 `edid` 作为运行时元数据保留在 `SkyString` 或单独 metadata table 中。

验收：Delphi 有 EDID 的 XML 条目，新版在等价 export scope 下也能输出同样 EDID。

---

### DP-10 DEFUI Component Generator

原版针对 MISC 等记录提供组件名生成器，可组合：

- base string
- component names
- weight
- quantity indicator
- keyword ignore list
- regex cleanup
- `%BASE%` / `%WEIGHT%` / `%COMPOS%` 模板

这是完整工具，不应误归类为 Header Processor。

实现时应尽量复用已有 record tree、关键字查询和 undo 系统。

---

### DP-11 Codepage 手动选择/覆盖

当前底层 `CodepageTable` 已支持主要 Windows codepage，自动 fallback 也已实现；缺的是原版 `ChooseCP` / `Codepage` 的用户覆盖工作流。

目标不是重做编码系统，而是允许：

- 查看当前推断 codepage
- 临时覆盖当前文件 codepage
- 重新加载/预览
- 保存为该会话/配置的显式选择

这对老 MOD 和错误标记编码文件仍有实际价值。

---

## 7. P3：兼容性尾项

### DP-12 翻译 Provider

Delphi provider 列表为：

1. Microsoft
2. Yandex
3. Baidu
4. Youdao
5. freeApi
6. Google
7. DeepL
8. OpenAI

当前 Rust 有 Azure/Microsoft、Baidu、Youdao、Google、DeepL、OpenAI；缺 Yandex 和原版 freeApi。

这两项优先级低于核心工作流。若服务本身已经废弃或 API 已不可用，应在文档中明确“有意不兼容”，而不是为了数字 8/8 保留死代码。

### DP-13 Microsoft Word 拼写后端

原版可在 Windows 通过 OLE 调 Word，也可使用 Hunspell。当前 Rust Hunspell 实现已经比较完整。

MS Word backend 属于 Windows 特有兼容功能，应作为可选 adapter，而不能污染跨平台 spell core。

### DP-14 其他低频工具

需要逐项决定是：

- 功能已被新 UI 取代；
- 功能应补；
- 或有意移除。

重点包括：

- `AddIdToStrings`
- `OldDialogStyle`
- 独立 Regex 工具窗
- 一些旧式 ESP structure / search dialogs

对于这些项目，**不要求像素级复制窗体**，只要求明确功能去向。

---

## 8. 本轮确认已经过时的旧“缺口”

以下旧文档结论不能再继续作为 TODO：

### BSA 特殊扩展名 hash

旧 `delphi_rust_fix_plan.md` 曾记录 `.nif/.kf/.dds/.wav` 特殊 hash 位操作未实现；当前 `bsa/directory.rs` 已有对应特殊扩展名逻辑和测试。

### 翻译 API retry

旧文档记录 retry 缺失；当前 `src-tauri/src/commands.rs` 已有 `translate_single_with_retry()`，最多三次尝试并区分可重试错误。

### SkyString internal flags

旧文档称 Delphi 的内部诊断标记只实现少数；当前 `SkyStringInternalParams` 已覆盖到 40+ 位，包括 translation retry、soft lock、CRLF array、string ID changed 等。

### SpellCheck 独立 UI

当前已有 `SpellCheckSettingsDialog.tsx` 和编辑器内拼写检查反馈，不能再标成“无独立 UI”。

### Editor modes / inline editing

当前已存在 Modal / Inline / Sidebar 三种编辑器模式。旧文档关于“只有弹窗编辑”的描述已经失效。

后续更新 parity 状态时，应优先核对当前源码，不能机械继承旧 roadmap。

---

## 9. 验证策略

每个 parity 项完成后至少经过三层验证：

### L1：单元测试

针对纯逻辑：

- parser
- matcher policy
- opcode mapping
- command processor parser
- archive layout calculations

### L2：固定 fixture

仓库内保存可重复的小型测试样本，验证输入/输出字节或结构化结果。

### L3：Delphi / 真实游戏交叉验证

适用于：

- SST apply 行为
- XML export
- PEX decompile
- BSA/BA2 injection
- 多游戏 ESP 加载

不能做 L3 时必须标记 `UNVERIFIED AGAINST DELPHI`，不能只凭 Rust roundtrip 宣称“100% Delphi compatible”。

---

## 10. 推荐实施顺序

DP-01、DP-04、DP-05 已完成，后续严格按下面顺序推进：

1. **DP-02 PEX opcode** — 修正已经发现的格式级风险。
2. **DP-03 Apply SST options** — 恢复核心翻译工作流的用户控制能力。
3. ~~**DP-04 Advanced Search**~~ — ✅ 已完成（2026-09-01）：独立面板、六维度、按字段 Regex、REC:FIELD 联合、preset 持久化；Keyword 维度待数据管道。
4. ~~**DP-05 BatchProcessor**~~ — ✅ 已完成（2026-09-01，L3 交叉验证待补）：完整白名单 parser + 全部 11 命令执行路径（含 GenerateDictionaries / LoadMasters / ApiTranslation）+ 独立 UI；ImportXml comparator 语义随 DP-09 闭环。
5. **DP-07 Localized/Hybrid loading** — 打通 archive 与 ESP 主流程。
6. **DP-06 BSA/BA2 injection** — 在资源加载稳定后实现写回。
7. **DP-08 / DP-09 XML export + EDID**。
8. **DP-10 / DP-11 辅助工具**。
9. **P3 兼容尾项**。

不要按 Delphi `.pas` 文件数量机械推进；优先完成用户实际工作流。

---

## 11. 完成定义

当满足以下条件时，才能把项目描述为“Delphi xTranslator 的完整现代重写”：

- 主要支持游戏在主 UI 中使用正确的游戏配置，不依赖 SkyrimSE fallback。
- ESP/SST/XML/PEX/Archive 核心格式在对应真实样本上完成交叉验证。
- Apply SST、Advanced Search、BatchProcessor、Localized loading 这些原版高级工作流有等价入口。
- 原版仍有价值的辅助工具要么实现，要么明确记录为有意移除及原因。
- `feature_comparison.md`、README 和本文件的状态保持同步，不再出现“代码已实现但文档说缺失”或“文档宣称完成但主流程没接通”的情况。
