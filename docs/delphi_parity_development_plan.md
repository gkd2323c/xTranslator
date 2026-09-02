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

- **主工作流没有真正接通的能力**：多游戏上下文、Localized/Hybrid 加载策略均已接通（DP-01/DP-07）。
- **原版高级工作流缩水**：XML Export options 已恢复（DP-08）；Apply SST、Advanced Search、BatchProcessor 已恢复（DP-03/DP-04/DP-05）。
- **格式级兼容差距**：PEX opcode（FO76/Starfield fixture 待补）、XML EDID 信息已闭环（DP-09）；归档注入已实现（DP-06）。
- **低频辅助工具未移植**：DEFUI Component Generator、Codepage 手动覆盖、部分旧式工具窗。

因此，当前版本更准确的描述是：

> 核心翻译引擎完成度很高，但还不是 Delphi xTranslator 1.6.0 的完整功能等价重写。

---

## 3. 优先级总表

| ID    | 优先级 | 差距                              | 当前状态                       | Delphi 参考                                                             | 当前实现入口                                                                   |
| ----- | ------ | --------------------------------- | ------------------------------ | ----------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| DP-01 | P0     | 多游戏上下文贯穿主流程            | ✅ **已完成**                   | `TESVT_main.pas` 游戏状态                                               | `game_detect.rs`, `appStore.ts`, `GroupedMenuBar.tsx`, `commands.rs::load_esp` |
| DP-02 | P0     | PEX opcode / 反编译语义           | ✅ **Skyrim + FO4 fixture；FO76/Starfield L3 待补** | `TESVT_scriptPex.pas`                                                   | `crates/xt-core/src/pex/decompile.rs`                                          |
| DP-03 | P0     | Apply SST 高级选项                | ✅ **核心完成；L3 交叉验证待补** | `TESVT_ApplySSTOpts.*`                                                  | `matching.rs`, `commands.rs::load_sst`                                         |
| DP-04 | P1     | Advanced Search                   | ✅ **已完成**                   | `TESVT_AdvSearch.*`                                                     | `ui/src/components/AdvSearchDialog.tsx`, `appStore.ts`                        |
| DP-05 | P1     | BatchProcessor 命令脚本           | ✅ **已完成（L3 交叉验证待补）** | `TESVT_commandProcessor.*`, `TESVT_main.pas::batchCommands/runCommands` | `xt-core::command_processor`、`src-tauri::command_processor`、`CommandProcessorDialog.tsx` |
| DP-06 | P1     | BSA/BA2 注入                      | ✅ **已完成（真实归档交叉验证待补）** | `TESVT_bsa.pas::InjectData`                                             | `archive_inject.rs`, `bsa/injection.rs`, `ba2/injection.rs`                 |
| DP-07 | P1     | Localized / Hybrid 加载策略       | ✅ **已完成（真实游戏交叉验证待补）** | `TESVT_delocOpts.*`, MainLoader                                         | `StringsLoadStrategy`, `commands.rs::load_esp`                                |
| DP-08 | P1     | XML Export 选项                   | ✅ **已完成（L3 交叉验证待补）** | `TESVT_XMLExportOpts.*`                                                 | `xml/mod.rs::collect_xml_export_entries`, `XmlExportDialog.tsx`, `commands.rs::export_xml` |
| DP-09 | P2     | XML EDID 元数据完整性             | ✅ **已完成（L3 交叉验证待补）** | `TESVT_XMLFunc.pas`                                                     | `SkyString.edid`, `xml/mod.rs`                                                  |
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

### DP-02 PEX opcode 与原版对齐 (✅ 核心与真实 BE/LE fixture 已完成)

#### 状态

✅ **2026-09-02 完成核心验收闭环（FO76/Starfield 真实 fixture 仍待补）。** 已对齐真实 Bethesda PEX 头部规范、大小端、GameID 建模、Object Body 字段顺序与字节级无损写回；仓库现已加入 Bethesda PapyrusCompiler 实际产出的 Skyrim SE Big-Endian fixture，以及本机 Fallout 4/F4SE 构建环境中的 Little-Endian `Armor.pex` 与 `Form.pex` fixture，并通过真实 parse/decompile/byte-for-byte roundtrip。FO4 的 Little-Endian 路径已有真实产物与实际 `Return` opcode/value 断言覆盖；FO76/Starfield 尚缺对应编译器产物，因此保留游戏级 L3 验证边界。

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
  - 新增 `crates/xt-core/tests/fixtures/pex/fallout4/Armor.pex.hex` 与 `Form.pex.hex`：来自本机 Fallout 4/F4SE 构建环境中的真实 `Data/Scripts` PEX；Header 保留 `E:\github\f4se\scripts\build_src`、`ianpatt`、`KURTHNAGA` provenance，不将其描述为 Bethesda 原版发行资产。`pex_real_fixture.rs` 覆盖 Little-Endian Header、对象反编译、`Form.kSlotMask30` getter 的真实 `Return(1)` 指令解析、`parse→compile→parse` 与 byte-for-byte roundtrip。
  - **剩余边界**：FO76 / Starfield Little-Endian 路径尚缺对应游戏编译器实际产出的 fixture；现有构造测试仍保留，用于覆盖 Guard/Struct 等 Starfield 特有布局。

#### 验证结果

- `cargo test -p xt-core --test pex_real_fixture` → **3 passed / 0 failed**（Skyrim SE Big-Endian 与 Fallout 4 Little-Endian 真实 fixture，其中 Form fixture 含实际 opcode/value 断言）。
- `cargo test -p xt-core --lib` → **当前 346 passed / 0 failed**。
- `cargo test --workspace` → **全部通过**。
- `cargo check -p xtranslator-tauri` → **通过**。
- `npx tsc --noEmit` → **通过**。
- `git diff --check` → **通过**。

#### 待办（剩余真实文件交叉验证）

- 若取得 FO76 或 Starfield 的真实 Papyrus 编译器产物，再补对应 fixture，重点验证游戏专属 opcode / Struct / Guard 布局；这不再阻塞 DP-02 的 Skyrim + FO4 基线验收。

---

### DP-03 恢复 Apply SST 高级选项 (✅ 核心完成，L3 交叉验证待补)

#### 原版能力

`TESVT_ApplySSTOpts.dfm` / `TESVT_ApplySSTOpts.pas` 暴露了：

**覆盖范围 (5 种)：**

- All（普通字符串为全部未锁定项；VMAD 由专用 comparator 决定，locked VMAD 仍可参与）
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
   - `reset_state` 已按 Delphi 改为候选预重置语义；未命中候选也会被重置，且 SST incomplete/locked 状态仍按来源保留；已有 `nTrans` 标记时也会在下一次匹配前重置，VMAD reset 使用源文 + `lockedTrans` 状态。
   - SST 正常应用与 `tag_only` 均同步 `colab_id`；`restrict_to_filter=true` 缺少 `filtered_ids` 时 fail-closed。
   - VMAD EDID 条目已固定到 V4Strict 路径：三个 FormID 档位均对 VMAD 强制原文与 index 精确校验，普通字符串仍分别使用 V4Edid/V4Strict/V4Relax；VMAD 专用 comparator 不再被通用 `lockedTrans` 预过滤截断。VMAD 译文变更按 Delphi 标为 `validated`；同语言下即使译文未变而源文仍不同，也保持 `validated`，其余完全相同项才标为 `translated`。
   - 拖放 SST 与菜单加载统一先进入 `ApplySstDialog`；`load_sst` 的 `options=None` 明确定义为默认高级 SST 选项，不再回退到通用 T1-T4 matcher。
   - `same_language` 已从 UI 的源/目标语言状态（`language === targetLang`）写入 SST DTO，再映射到 `ApplyPolicy`；BatchProcessor 则从 `LangSource/LangDest` 规则推导；目标语言列表包含 English，因此同语言路径具备真实入口。
   - matching 专项测试现为 43 项全绿；新增覆盖 VMAD 三档 FormID 正向路由、锁定目标资格、译文状态、同语言状态、`nTrans` 命中/未命中 reset、StringOnly 未命中 reset，以及 Tag Only 现代契约；原有 XML/通用 4-Tier 路径保持独立。
2. **IPC DTO (`crates/xt-shared/src/dto.rs`)**：
   - 定义 `SstOverwriteScopeDto`, `SstMatchModeDto`, `SstApplyOptionsDto`。
3. **Tauri 后端命令 (`src-tauri/src/commands.rs`)**：
   - `load_sst` 升级支持可选 `options: Option<SstApplyOptionsDto>` 参数。
4. **前端交互 (`ui/src/components/ApplySstDialog.tsx` & `ui/src/api/strings.ts`)**：
   - 新建 `ApplySstDialog`（支持 5 种覆盖范围单选、4 种匹配模式单选、3 种附加复选框、回车直接应用与 Esc 关闭）。
   - 在 `MenuBar.tsx` 和 `GroupedMenuBar.tsx` 中全面集成，自动采集当前选中的 `selectedIds` 与过滤后的 `items` ID 列表传递给后端。
   - 增加前端 Vitest 单元测试 `ApplySstDialog.test.tsx`。

#### 当前验收状态

- `cargo test -p xt-core matching::tests --lib` → **43 passed / 0 failed**。
- `cargo test --workspace` → **全部通过**；xt-core 单元测试 **346 passed / 0 failed**，集成测试与 doc-tests 也通过，release-only 测试按测试声明保持 ignored。
- `cargo check -p xtranslator-tauri` → **通过**。
- `npx vitest run` → **59 passed / 0 failed**（6 个测试文件）。
- `npx tsc --noEmit` → **通过**。
- `npm run build` → **通过**（Vite 仅提示既有的大 chunk warning）。
- `git diff --check` → **通过**（仅报告仓库现有 LF→CRLF 提示，无 whitespace error）。
- `cargo fmt --all -- --check` → **未通过**，输出包含本轮范围外及既有 DP-02/DP-03/CLI 等未格式化改动；本轮未为追求全局格式绿灯而改写这些无关文件。
- **VMAD 进展**：已完成 `getfProcCompareOptVMADString` 的防污染保护机制，StringOnly 模式下 5 档 Scope 矩阵已全量单测覆盖（All / NoTransExclusive / NoTransAndPartial 屏蔽；PartialOnly / Selection 放行）；三个 FormID 档位均已路由到 VMAD 专用 V4Strict 约束，locked VMAD 资格、VMAD+nTrans 未命中 reset，以及译文状态映射均有回归测试覆盖。
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

#### 状态

✅ **2026-09-01 完成（真实归档交叉验证待补）。** replacement injection 已实现：BSA（zlib / SSE LZ4）与 BA2 GNRL（zlib），含安全替换流程（临时文件 → 重开校验 → 备份 → 原子替换）。

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

#### 实现内容

- **`crates/xt-core/src/bsa/injection.rs`**：`inject_bsa()` 按 Delphi `InjectData` 语义逐文件夹逐文件处理。命中替换映射 → 用新数据（保留原压缩策略：SSE 写 `[uSize][LZ4]`，Skyrim 写 `[uSize][zlib]`；未压缩原样写）；否则原样复制整个数据块（含 flag 9 名字前缀）。重写文件表条目的 raw_size（含压缩标志位，对齐 Delphi `SetFileCompressedFlag`）与 offset。文件表位置计算考虑 folder name 交错布局（每个 folder 的 name 块在 file records 之前）。
- **`crates/xt-core/src/ba2/injection.rs`**：`inject_ba2()` 按 Delphi `TwbBA2File.InjectData` 复制 header + 重建数据区（GNRL zlib 压缩或原样）→ 复制文件表（name + 36 字节 record 交错）→ 重写每个 entry 的 offset/packedSize/size → 复制 string table → 更新 file_table_offset。DX10 纹理归档在 open 时即拒绝。
- **`crates/xt-core/src/archive_inject.rs`**：`inject_archive()` 统一安全流程：open 校验（含替换目标必须存在于归档，fail-closed）→ 写同目录临时文件 → 重新打开临时文件验证可读 → 备份原文件（可选）→ 原子替换（Windows 先删后改，POSIX rename）。
- **IPC**：`inject_archive` 命令 + `InjectArchiveRequest/Response` DTO（replacements 用 Base64 传输）；`src-tauri` 增加 base64 依赖。
- **前端**：`BsaBrowser` 预览面板新增 "Replace…" 按钮：选择新内容文件 → 确认（提示会创建备份）→ 调用注入 → 成功提示含备份路径。
- **顺带修复**：BA2 `contains_file`/`extract_file` 比较文件路径时用了分割后的文件名而非完整路径，导致按路径查找永远失败（真实 BA2 的 `files[i].name` 存完整路径）。已修复为完整路径比较。

#### 安全要求（已落实）

- ✅ 永远写入临时归档（同目录，保证 rename 原子性）。
- ✅ 完成完整结构校验后再原子替换（重开验证）。
- ✅ 替换前创建备份（`create_backup` 默认 true）。
- ✅ 保留原 entry 的压缩策略（BSA 按版本 zlib/LZ4；BA2 zlib）。
- ✅ DX10 texture archive 不纳入（open 时拒绝）。

#### 验收

- ✅ BSA v0x69 replacement roundtrip（构造 fixture：替换后可重开读取替换与未替换文件）。
- ✅ BA2 GNRL replacement roundtrip（构造 fixture：替换后可重开读取）。
- ✅ 注入后能被 Rust 自己重新打开并读取替换后的文件。
- ⚠️ L3：真实 Skyrim Interface.bsa / FO4 归档的注入交叉验证待真实样本。

#### 验证结果

- `cargo test -p xt-core --test injection_roundtrip` → **3 passed / 0 failed**（BSA roundtrip、BA2 roundtrip、missing 报告）。
- `cargo test --workspace` → **全部通过**（xt-core 339 + 注入 3）。
- `cargo check -p xtranslator-tauri` → **通过**。
- `npx tsc --noEmit` → **通过**。
- `npx vitest run` → **59 passed / 0 failed**。
- `npm run build` → **通过**。
- `git diff --check` → **通过**（仅仓库行尾转换提示）。

---

### DP-07 Localized / Hybrid 加载策略

#### 状态

✅ **2026-09-01 完成（真实游戏交叉验证待补）。** Strings 来源已抽象为显式策略并贯穿加载主流程，每个 STRINGS 文件的实际来源可追溯并展示在 UI。

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

#### 实现内容

- **`StringsLoadStrategy` 枚举**（`crates/xt-core/src/esp/parser.rs`）：`DiskPreferred`（Delphi locOpts=0）/ `ArchivePreferred`（locOpts=1）/ `Manual`（locOpts=2），带字符串解析（"disk"/"archive"/"manual" 及数字别名）。
- **`StringsSource` 来源追踪**：`StringsFiles` 新增 `sources: [StringsSource; 3]`（Disk / Archive / Missing），对齐 Delphi `loadAddonStrings` 的 `bfile[j]` 0/1/2 逐文件判定。
- **`load_from_dir_with_strategy`**：统一加载入口。DiskPreferred 磁盘优先、缺失回退 archive（与既有 BSA/BA2 回退一致）；ArchivePreferred 先扫 archive 提取、缺失回退磁盘；Manual 返回空集由调用方手动填充。archive 遍历保持单次打开、Interface/Misc 归档优先、跳过 >512MB 纹理包。
- **`load_esp` 新增 `strings_strategy` 参数**：Manual 时跳过自动发现；来源数组写入 `LoadEspResponse.strings_sources`（"disk"/"archive"/"missing"；缓存命中为 "cache"）。
- **config 持久化**：`AppConfig` / `AppConfigDto` 新增 `strings_strategy`，启动时恢复。
- **前端**：工具栏新增 Strings source 下拉（Disk / Archive / Manual，persist 到 config）；Manual 策略加载时弹目录选择（对齐 Delphi ChooseStrings）；加载日志输出 `Strings sources: STRINGS:disk, DLSTRINGS:archive, ...`；SidePanel 文件信息区显示每个 Strings 文件的来源缩写。

#### 验收

- ✅ 同一 localized ESP 可分别从 loose Strings（DiskPreferred）和 archive Strings（ArchivePreferred）加载。
- ✅ 用户手动选择可覆盖自动发现（Manual 弹目录选择）。
- ✅ UI 明确显示 Strings 来源（SidePanel + 日志）。
- ⚠️ L3：需在真实 Skyrim SE / Fallout 4 安装上分别用两种策略加载同一插件对比来源标记。

#### 验证结果

- `cargo test --workspace` → **全部通过**；xt-core **339 passed / 0 failed**（新增 4 项策略/来源测试：策略解析、sources 默认值、磁盘来源标记、Manual 空集）。
- `cargo check -p xtranslator-tauri` → **通过**。
- `npx tsc --noEmit` → **通过**。
- `npx vitest run` → **59 passed / 0 failed**。
- `npm run build` → **通过**。
- `git diff --check` → **通过**（仅仓库行尾转换提示）。

---

### DP-08 XML Export 选项

✅ **2026-09-02 完成（L3 交叉验证待补）。**

#### 原版能力

原版 XML Export 支持：

- All
- Translated
- Selection
- Diff
- Split collaboration
- 可选 FUZ 数据

#### 当前差距（已完成前）

当前 `XmlExportRequest` 只有：

- `path`
- `dest_lang`

`export_xml` 直接通过 `sky_strings_to_xml_entries()` 收集当前可导出条目，行为基本固定为“已有译文”。

#### 目标（已完成前）

扩展请求 DTO，恢复导出范围和附加信息控制。

#### 实现内容

- **核心（`crates/xt-core/src/xml/mod.rs`）**：新增 `XmlExportScope`（Everything / TranslatedAndValidated / Selection / SourceDestDiff，对应 Delphi `TFormXmlOpt.RadioGroup1` 4 档）与 `XmlExportOptions`；新增 `collect_xml_export_entries()`，分两步对齐 Delphi `XMLExportbase`：
  1. `prepareSSTXML` 候选集：排除空串（`source` 与 `translation` 皆空）、`lockedStatus`（`pexNoTrans` 或 locked VMAD）、内部删除/警告标记（`isDeleted/lowwarning/warning/bigwarning/nTrans`）、无导出状态位（`sparams ∩ [translated,lockedTrans,incompleteTrans,validated] = ∅`）；lockedTrans 条目按 Delphi 归一化（清掉 translated/incomplete/validated）；
  2. 候选集上应用 scope comparator（`compareOptEverything` / `compareOptTranslatedAndValidated` / `compareOptSelection` / `compareSourceDestDiffandColab`）。
- **DTO（`xt-shared` + TS）**：`XmlExportRequest` 新增 `scope: Option<XmlExportScopeDto>`、`selected_ids: Option<Vec<u32>>`、`export_fuz: bool`；后端 `export_xml` 有 scope 时走 `collect_xml_export_entries`，缺省保持旧“已有译文”快速路径（批处理/finalize/CLI 不受影响）。
- **UI**：新增 `XmlExportDialog`（All/Translated/Selection/Diff 单选，对齐 RadioGroup1），`MenuBar` 与 `GroupedMenuBar` 的 Export XML 菜单项先弹选项对话框、确认后携带 scope 导出；Selection 档自动把 `appStore.selectedIds`（稳定 u32 id）作为 `selected_ids`。
- **FUZ 开关注记**：Delphi `chk_exportFuzData` 依赖 FUZ 元数据管道，当前 Rust 尚未接通，按仓库“不在 UI 假装生效”原则**不渲染无效开关**；`export_fuz` 字段保留在 DTO 以备后续接通。

#### 验收（完成情况）

- ✅ 四种 export scope 有测试（`xml::tests`：Everything / TranslatedAndValidated / Selection / SourceDestDiff + 排除条件 + lockedTrans 归一化 + EDID 保留，共 8 项新增）。
- ✅ Selection 使用稳定 IDs（`selected_ids: Option<HashSet<u32>>`，前端由 `selectedIds` Set 映射）。
- ✅ Diff 定义为 `colab_id != 0 || hash != hash_trans`（对照 Delphi `compareSourceDestDiffandColab` 源码，非 UI 名称猜测）。
- ⚠️ Split collaboration：Diff scope 的 colab 语义已包含；“按 colab 拆分为多文件导出”为 Delphi 在 colab 模式下的附加文件拆分行为，Rust 当前导出单文件，未实现多文件拆分（低频，标记 L3 尾项）。
- ⚠️ L3：需在真实游戏安装上与 Delphi 导出结果交叉验证条目范围。

#### 验证结果

- `cargo test -p xt-core --lib` → **354 passed / 0 failed**（新增 8 项 XML export scope 测试）。
- `cargo test --workspace` → **全部通过**。
- `cargo check -p xtranslator-tauri` → **通过**。
- `npx tsc --noEmit` → **通过**。
- `NODE_ENV=test npx vitest run` → **59 passed / 0 failed**（注意：不带 `NODE_ENV=test` 时 React 走 production build 导致 act 报错，属环境问题）。
- `npm run build` → **通过**（仅既有大 chunk 提示）。
- `git diff --check` → **通过**（仅仓库 LF→CRLF 提示）。

---

## 6. P2：补全信息和低频但有价值的能力

### DP-09 XML EDID 元数据

✅ **2026-09-02 完成（与 DP-08 一并闭环）。**

#### 背景（已完成前）

当前 XML parser/writer 已能读写 `<EDID>`，但 `SkyString` 不保存 EDID 文本，`sky_strings_to_xml_entries()` 因此无法稳定导出它。

建议不要只为 XML 临时查询记录树。更合理的是在解析期把需要跨工作流使用的 `edid` 作为运行时元数据保留在 `SkyString` 或单独 metadata table 中。

#### 完成内容

- `SkyString.edid: Option<String>` 字段（前序会话已加入）在 ESP 解析期从记录 EDID 字段填充：普通可翻译字段路径（`esp/parser.rs` 1218/1452）与 VMAD 字符串路径（1532）均已赋值。
- `sky_strings_to_xml_entries()` 与 `collect_xml_export_entries()` 均输出 `sk.edid`（不再硬编码 `None`）。
- 注记：Delphi `getEdidNameExport` 对 VMAD 字符串输出 `rec_edid + "_" + propName`；当前 `SkyString` 只保留裸 EDID（prop 名只进了 `edid_hash`），故 XML 导出的 VMAD 条目 EDID 为记录 EDID，未拼接 prop 名——如需完整对齐需在 SkyString 上补存 prop 名，标记为已知差异（L3）。

#### 验收

- ✅ 解析期填充：普通与 VMAD 字符串均有 `edid`（parser.rs 三处赋值）。
- ✅ 导出：`collect_xml_export_entries`/`sky_strings_to_xml_entries` 输出 `sk.edid`；`test_xml_export_edid_preserved_through_collect` 覆盖。
- ⚠️ L3：Delphi 有 EDID 的 XML 条目，新版在等价 export scope 下也能输出同样 EDID——需真实样本交叉验证（VMAD `_prop` 拼接差异见注记）。

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

DP-01、DP-04、DP-05、DP-06、DP-07 已完成，后续严格按下面顺序推进：

1. ~~**DP-02 PEX opcode**~~ — ✅ 2026-09-02：Skyrim SE 与 Fallout 4 真实 fixture 已覆盖；FO76/Starfield fixture 作为发布 QA 尾项。
2. ~~**DP-03 Apply SST options**~~ — ✅ 2026-09-02：ApplySstDialog 已覆盖菜单与拖放入口，默认调用统一高级 matcher；VMAD 与 same-language 语义已接通，剩余仅 L3 交叉验证。
3. ~~**DP-04 Advanced Search**~~ — ✅ 已完成（2026-09-01）：独立面板、六维度、按字段 Regex、REC:FIELD 联合、preset 持久化；Keyword 维度待数据管道。
4. ~~**DP-05 BatchProcessor**~~ — ✅ 已完成（2026-09-01，L3 交叉验证待补）：完整白名单 parser + 全部 11 命令执行路径（含 GenerateDictionaries / LoadMasters / ApiTranslation）+ 独立 UI；ImportXml comparator 语义随 DP-09 闭环。
5. ~~**DP-06 BSA/BA2 注入**~~ — ✅ 已完成（2026-09-01，真实归档交叉验证待补）：BSA zlib/LZ4 + BA2 GNRL zlib replacement injection，安全替换流程（临时文件→校验→备份→原子替换）。
6. ~~**DP-07 Localized/Hybrid loading**~~ — ✅ 已完成（2026-09-01，真实游戏交叉验证待补）：DiskPreferred / ArchivePreferred / Manual 三策略 + 逐文件来源追踪 + UI 展示。
7. ~~**DP-08 / DP-09 XML export + EDID**~~ — ✅ 2026-09-02：XML Export Options 对话框（All/Translated/Selection/Diff）已接入两条菜单栏；collect_xml_export_entries 对齐 prepareSSTXML 候选集 + 4 comparator；SkyString.edid 解析期填充并随导出输出；8 项 scope 单测 + 59 vitest + 354 xt-core 全绿；Split-colab 多文件拆分与 VMAD `_prop` EDID 拼接标记为已知差异（L3）。
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
