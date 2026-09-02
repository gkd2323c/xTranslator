# Delphi Parity 整改计划

> 基于 2026-09-02 对 `docs/delphi_parity_development_plan.md`、当前 Rust/Tauri/React 实现、原 Delphi 1.6.0 源码与现有测试的验收结果制定。
>
> 本文只记录**当前确认存在的整改项**。完成状态以代码、真实调用链和验收测试为准，不以旧计划中的勾选状态为准。

---

## 1. 整改目标

当前项目的核心翻译引擎已经具备较高完成度，Skyrim SE 主解析链、SST matcher、PEX roundtrip、BSA/BA2 基础注入等能力都有实际代码和测试支撑。

本轮整改的目标不是重写核心，而是解决以下三类问题：

1. **文档提前宣告完成**：roadmap 标记为 `✅ 已完成`，但实际主工作流仍有断口。
2. **IPC / 状态链路未闭环**：底层能力存在，但 UI → Tauri → core 之间的参数、游戏上下文、加载策略或缓存语义不一致。
3. **Delphi 功能理解偏差**：实现了一个有用的新工具，但它并不是原版同名功能，不能计入 parity。

整改完成后，`delphi_parity_development_plan.md`、`feature_comparison.md` 与实际代码状态必须重新一致。

---

## 2. 验收基线

2026-09-02 验收时的稳定基线：

- `cargo test --workspace`：通过。
- `xt-core`：359 项单元测试通过。
- PEX 真实 fixture：3 项通过。
- BSA/BA2 injection roundtrip：3 项通过。
- Tauri tests：7 项通过。
- `cd ui && npx tsc --noEmit`：通过。
- `cd ui && npm run test`：59 项通过。
- `cd ui && npm run build`：通过。
- 真实 `Skyrim.esm` debug E2E：75,757 strings、3 个 Strings 文件加载成功，测试通过。
- `cargo fmt --all -- --check`：未通过，当前分支存在既有格式化差异。

这些结果说明整改应采取**外科式修复**，不得借机重构稳定 parser / matcher / serializer。

---

## 3. 优先级总表

| ID | 优先级 | 整改项 | 当前判定 | 主要风险 |
|---|---|---|---|---|
| R-01 | P0 | DEFUI IPC 与游戏上下文 | ❌ 阻塞 | UI 实际调用失败；scope / DTO / game 不一致 |
| R-02 | P0 | Codepage override + Strings reload + cache key | ❌ 阻塞 | 用户选择不生效；缓存可能返回错误语言/编码数据 |
| R-03 | P0 | Manual Strings 加载策略 | ❌ 阻塞 | UI 选择目录后后端不读取 |
| R-04 | P1 | BatchProcessor `ImportXml` comparator | ✅ 已闭环 | （2026-09-02：复用 SST comparator 家族 + 矩阵测试） |
| R-05 | P1 | AddIdToStrings parity | ✅ 已闭环 | 2026-09-02：新增 true AddIdToStrings（三档 scope + 四前缀 + record tree DIAL 解析）；原 FormID offset 工具重命名为 FormIdOffsetDialog，标记为 Rust 新增功能 |
| R-06 | P1 | Advanced Search Keyword 数据链 | ⚠️ 已侦察，数据管道待实现 | Delphi `getKwd`/`findKeyWord` 语义已确认；ESP record tree keyword 提取与 DTO 管道尚未建立 |
| R-07 | P2 | XML Export / EDID 尾项 | ✅ 核心完成 | DP-08/09 已标记 L2/L3 差异；split-colab、FUZ metadata、VMAD _prop EDID 为已知差异 |
| R-08 | P2 | FUZ rename 工具 | ✅ 已确认 | Delphi 源码无 FUZ rename 功能；Rust 当前实现已覆盖全部 Delphi FUZ 能力（扫描、INFO 映射、音频播放、LIP 预览） |
| R-09 | P2 | parity 文档状态重建 | ✅ 已闭环 | R-01~R-05 状态已同步；R-07/08 已确认；DP 编号 01-14 无冲突；development_plan 与 remediation_plan 交叉引用已校准 |
| R-10 | P3 | L3 真实样本交叉验证 | ⚠️ 不完整 | FO76/Starfield、真实 archive、Delphi SST/XML 尚未全覆盖 |
| R-11 | P3 | Rust formatting / warning 清理 | ⚠️ 质量门禁 | `cargo fmt --check` 不通过，存在未使用代码告警 |

---

## 4. P0：先恢复真实主工作流

### R-01 DEFUI IPC、scope 与 GameId 闭环

#### 已确认问题

- `ui/src/api/strings.ts` 调用 `invoke("apply_def_ui_generator", { request })`。
- Tauri 命令实际接收 `options / selected_ids / preview_only` 顶层参数。
- TS `DefUiOptionsDto` 与 `xt-shared::DefUiOptionsDto` 字段集合和命名不同。
- TS 返回类型与 Rust `DefUiApplyResultDto` 不一致。
- UI 使用 `all / only_untranslated / only_selected`，core 当前判断 `all / untranslated / selection`。
- `apply_def_ui_generator` 硬编码 `GameId::Fallout4`。
- `ignore_list` 已进入配置和 IPC，但 generator 没有实际过滤。

#### 整改要求

1. 以 `crates/xt-shared/src/dto.rs` 为共享契约真相源，统一 Rust / TypeScript DTO。
2. Tauri 命令统一采用单个 request DTO，避免前后端参数形状再次漂移。
3. scope 改为共享 enum 或严格一致的字符串枚举，不允许 UI/core 各自定义别名。
4. 从可信 `currentGame` / 当前已加载文件上下文获取 `GameId`，禁止硬编码 Fallout 4。
5. 实现 `ignore_list` 的 EDID / FormID 过滤语义，并补 core 回归测试。
6. 增加 Tauri command 级测试，至少覆盖一次真实序列化参数调用形状，而不仅是纯 core 格式化测试。

#### 验收

- DEFUI Preview 和 Apply 从 UI 实际可调用。
- Fallout 4 / Fallout 76 / Starfield 不会共用错误的硬编码游戏规则。
- `only_untranslated`、`only_selected`、`ignore_list` 均有自动化测试。
- `xt-shared` 与 `ui/src/api/strings.ts` 字段逐项一致。

---

### R-02 Codepage override、加载上下文与缓存语义

#### 已确认问题

`reload_with_codepage()` 先修改 `state.codepage_table`，随后调用 `load_esp()`；但 `load_esp()` 会重新从 `Data/<Game>/codepage.txt` 构造本地 table，因此强制 override 没有参与实际解析。

同时，ESP SQLite cache 以 ESP 内容 SHA-256 为核心身份，缓存的是已经解码后的 `SkyString`。当前缓存命中路径未区分：

- language
- Strings directory
- Strings load strategy
- forced codepage

因此同一 ESP 切换语言、Strings 来源或 codepage 时存在返回旧解码结果的风险。

#### 整改要求

1. 为 `load_esp` 增加明确的加载上下文对象，至少包含：`game`、`language`、`strings_dir`、`strings_strategy`、`forced_codepage`。
2. forced codepage 必须进入实际 `StringsFiles` 解码路径，而不是只写入 `AppState`。
3. `reload_with_codepage` 必须保留当前显式游戏选择和 Strings strategy。
4. 重新定义缓存边界：
   - 推荐：ESP 结构缓存与已解码 Strings 缓存拆分；或
   - 把所有会影响字符串结果的加载上下文纳入 cache identity。
5. 不允许仅靠 ESP SHA 命中后跳过语言/codepage/Strings source 判断。
6. 增加“同一 ESP、不同 codepage / language / strategy 不串缓存”的回归测试。

#### 验收

- 选择 CP936/CP950/1252 后重新加载，实际字符串内容发生符合样本预期的变化。
- Manual / Archive / Disk 三种策略在 codepage reload 后保持原策略。
- cache hit 与 cache miss 在同一加载上下文下结果一致。
- 改变 codepage / language / source 后不会错误复用旧字符串缓存。

---

### R-03 Manual Strings 加载策略

#### 已确认问题

前端 Manual 模式会要求用户选择 Strings 目录，并将目录传给 `loadEsp()`；后端却在 `StringsLoadStrategy::Manual` 分支直接创建空 `StringsFiles`，不读取用户选择目录。

#### 整改要求

1. 明确 Manual 的产品语义：**禁止自动发现，但必须读取用户显式提供的目录**。
2. `strings_dir = Some(path)` + Manual 时，直接从该目录加载三类 Strings。
3. `strings_dir = None` + Manual 时才返回空集或明确提示需要用户选择。
4. 调整现有 `test_strings_manual_strategy_returns_empty`，避免测试锁死错误工作流。
5. 增加 Tauri/UI 路径测试，覆盖“选目录 → load_esp → strings_loaded > 0”。

#### 验收

- Manual 模式选择有效目录后能够加载对应 Strings。
- 不再自动从 ESP 目录或 archive fallback 寻找文件。
- 日志 `strings_sources` 正确标记为 disk/manual-selected 等明确来源。

---

## 5. P1：恢复 Delphi 行为等价

### R-04 BatchProcessor `ImportXml` comparator

当前 processor 的 `ImportXml:<compare_option>:<apply_mode>:<path>` 已解析参数，但执行时仍直接调用普通 XML T1-T4 matcher，参数没有改变行为；代码本身还会发出 parity warning。

整改要求：

- 从 Delphi `batchCommands/runCommands` 与 XML import comparator 路径重新确认 `compare_option / apply_mode` 语义。
- 不要在 `src-tauri::command_processor` 复制 matcher；把策略映射到 core 层共享 apply policy。
- 为每个合法数值模式建立矩阵测试。
- 执行完成后删除“尚未实现”warning，前提是对应测试和 Delphi 证据已闭环。

验收：processor ImportXml 的同一输入在不同 comparator 参数下产生可预测且被测试锁定的差异。

---

### R-05 重新实现 Delphi `AddIdToStrings`

#### 已确认偏差

原 Delphi `TESVT_AddId.*` 的功能是给目标译文增加标识前缀，可选：

- String ID：`[%.5x]`
- FormID：`[%.8x]`
- Record / Field：`[RECR:FIELD]`
- DIAL master ref：`[@%.8x]`

当前 Rust `add_id.rs` / `AddIdDialog.tsx` 实现的是“批量偏移 FormID”，用于重排/迁移。该工具本身可以保留，但**不能继续作为 Delphi AddId parity 项**。

#### 整改要求

1. 将现有 FormID offset 工具改成清晰的新名称，例如 `FormIdOffsetTool`，文档标记为 Rust 新增功能。
2. 新增真正的 `AddIdToStrings`：按稳定 `u32 id` 更新 translation，不修改 ESP FormID。
3. 实现 Delphi 三档 scope：Everything / NoTransValid / Selection。
4. locked / empty 字符串行为与 Delphi `addIdToStringEx` 对齐。
5. INFO → DIAL master reference 必须从真实 record tree 关系解析，不能猜路径或 ID。
6. 添加 Delphi 示例输入/输出 fixture 测试。

#### 验收

- 同一条字符串按四个 checkbox 组合生成与 Delphi 格式一致的前缀。
- 当前 FormID offset 功能保留时，不再出现在 Delphi parity 完成统计中。

---

### R-06 Advanced Search Keyword

当前 Keyword 输入控件被禁用，store 中即使存在 keyword matcher 也会直接使行不匹配。这一维度不能继续计为“六维 Advanced Search 已完成”。

整改要求：

- 先确认 Delphi Keyword 的真实数据来源与匹配对象。
- 从 ESP record tree 提取并通过 DTO/前端数据管道暴露所需 keyword 信息。
- 不把大字典重复塞进每条 IPC DTO；优先建立紧凑索引或客户端映射。
- 完成前 UI 可保留 disabled，但文档必须明确标记为未完成。

验收：Keyword 搜索有真实记录 fixture，普通匹配和 Regex 均通过测试。

---

## 6. P2：补齐已知尾项

### R-07 XML Export / EDID

需要补齐：

- split collaboration 多文件导出；
- `export_fuz` 实际 metadata 输出；
- VMAD XML EDID 的 `_prop` / property name 组合语义；
- 对这些差异增加固定 XML fixture，而不是只测 Rust 自己 write→parse roundtrip。

完成前，DP-08 / DP-09 应标记为“核心完成，存在明确 L2/L3 差异”，不得写“完整完成”。

### R-08 FUZ rename

现有 FUZ 功能包括扫描、INFO 映射、音频播放、LIP 预览，但没有批量重命名代码。

整改前先从 Delphi `TESVT_Fuz.pas` / Browser 调用链确认：

- 重命名规则；
- 冲突处理；
- BSA 内文件与磁盘文件是否同语义；
- 是否要求备份/预览。

若决定不移植 rename，应在 parity 文档中明确标记“有意移除”，而不是继续写“已完成”。

---

## 7. P2：重建 parity 文档可信度

### R-09 统一 ID、状态与真相源

当前 `delphi_parity_development_plan.md` 内存在 DP 编号冲突：总表的 DP-12~14 与第 7 节 DP-12~14 指向不同功能。

整改规则：

1. 给每个 parity 项分配唯一 ID，禁止复用。
2. 状态统一为四类：
   - `✅ Complete`
   - `⚠️ Partial / L3 pending`
   - `❌ Incomplete`
   - `➖ Intentionally omitted`
3. “Complete” 必须满足主调用链已接通；只有 core helper 或 UI 壳不算完成。
4. L3 未做且属于格式/兼容敏感项时，必须保留 `UNVERIFIED AGAINST DELPHI`。
5. 同步更新：
   - `docs/delphi_parity_development_plan.md`
   - `docs/feature_comparison.md`
   - `docs/development_roadmap.md` 中受影响条目
6. 删除过时的测试数字和已经失效的“全绿”总结，避免未来模型再次机械继承。

---

## 8. P3：真实样本与发布 QA

### R-10 L3 验证矩阵

核心实现修复后再做 L3，不要用真实样本测试替代前面的确定性单测。

最低矩阵：

| 能力 | Skyrim SE | Fallout 4 | Fallout 76 | Starfield | Delphi 对照 |
|---|---|---|---|---|---|
| ESP load / game detect | ✅ 已有真实 E2E | 待补 | 待补 | 待补 | 不适用/行为对照 |
| PEX | ✅ fixture | ✅ fixture | 待补 | 待补 | 反编译输出抽样 |
| SST apply | 待补 | 可选 | 可选 | 可选 | 必须 |
| XML export/import | 待补 | 可选 | 可选 | 可选 | 必须 |
| BSA injection | 待补真实 archive | - | - | - | 必须抽样 |
| BA2 injection | - | 待补真实 archive | 可选 | 待补 | 必须抽样 |
| Codepage override | 老 MOD 样本 | 老 MOD 样本 | 视样本 | 视样本 | 行为对照 |

Release E2E 当前在普通 shell 下会因 `libz-ng-sys` 找不到 Visual Studio CMake generator 失败。发布 QA 应使用已加载 VS Developer 环境的 shell，或把该前置条件写进 `docs/release_qa.md`。

---

## 9. P3：代码质量收尾

### R-11 Formatting 与已有 warnings

整改功能全部稳定后再统一处理：

- `cargo fmt --all -- --check`
- 清理本轮改动引入的 unused import / dead code。
- 现有 `sst::v8` 未使用版本常量、旧 cache deprecated tests 等历史 warning 可单独开清理任务，不与 parity 行为修改混在同一提交。
- 不允许为了消 warning 删除旧 SST 读取兼容常量或改变二进制行为。

---

## 10. 推荐执行顺序

严格按依赖推进：

1. **R-01 DEFUI IPC**
2. **R-02 Codepage / cache identity**
3. **R-03 Manual Strings loading**
4. **R-04 BatchProcessor ImportXml**
5. **R-05 AddIdToStrings**
6. **R-06 Keyword search**
7. **R-07 XML 尾项**
8. **R-08 FUZ rename / intentional omission decision**
9. **R-09 文档状态重建**
10. **R-10 L3 矩阵**
11. **R-11 formatting / warnings**

其中 R-02 与 R-03 应作为同一轮加载链整改完成，因为两者共享 `load_esp`、Strings source 和 cache 语义。不要先为其中一个增加临时旁路。

---

## 11. 每项完成定义

任何整改项只有同时满足以下条件才允许改成 `✅ Complete`：

1. 行为修复发生在真实源头，不是 UI workaround。
2. `xt-shared` / TypeScript IPC contract 同步。
3. 新增对应回归测试；parser/matcher/cache/game detection 类修复必须有 core 测试。
4. 涉及 UI 时至少通过 `npx tsc --noEmit` 和相关 Vitest。
5. 涉及 Tauri command 时通过 `cargo build -p xtranslator-tauri`。
6. 涉及二进制格式时保持既有 invariants，不静默改变格式行为。
7. 相关 parity 文档同步更新。
8. 对要求 L3 的项，未完成真实/Delphi 交叉验证前只能标记 `⚠️ Partial`。

建议每轮最低验证命令：

```powershell
cargo test -p xt-core --lib
cargo test --workspace
cargo build -p xtranslator-tauri
cd ui
npx tsc --noEmit
npm run test
npm run build
```

修改完成后再运行：

```powershell
cargo fmt --all -- --check
git diff --check
```

---

## 12. 本轮整改完成后的目标状态

完成 R-01 ~ R-09 后，项目应达到：

> 核心翻译引擎稳定，主 UI 的 DEFUI、Codepage、Manual Strings、Batch XML 等高级工作流真实可用；Delphi parity 文档不再存在“代码没接通但状态全绿”或“实现了另一个工具却计入同名 parity”的情况。

完成 R-10 后，才适合重新评估是否可以使用“Delphi xTranslator 1.6.0 的完整现代重写”这一描述。

