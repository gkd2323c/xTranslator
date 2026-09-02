# L3 真实样本交叉验证矩阵 (R-10)

本文件记录 Delphi xTranslator 1.6.0 与 Rust 重写版的 L3 端到端交叉验证结果。
验证工具：`cargo run -p xt-cli -- golden-diff <delphi_golden_dir> <esp_path>`

金样本目录：`tests/fixtures/delphi_golden/`（Delphi 1.6.0 导出的 Skyrim.esm 参考文件）

## 验证日期：2026-09-02

## 最低矩阵结果

| 能力 | Skyrim SE | Fallout 4 | Fallout 76 | Starfield | Delphi 对照 |
|---|---|---|---|---|---|
| ESP load / game detect | ✅ 已有真实 E2E | ⚠️ 样本缺失 | ⚠️ 样本缺失 | ⚠️ 样本缺失 | 不适用/行为对照 |
| PEX | ✅ fixture | ✅ fixture | ⚠️ 样本缺失 | ⚠️ 样本缺失 | 反编译输出抽样 |
| SST apply | ✅ 67390 条可读 | ⚠️ 样本缺失 | ⚠️ 样本缺失 | ⚠️ 样本缺失 | ✅ 必须（已验证） |
| XML export/import | ✅ 67390 条 100% key 匹配 | ⚠️ 样本缺失 | ⚠️ 样本缺失 | ⚠️ 样本缺失 | ✅ 必须（已验证） |
| BSA injection | ✅ 真实 archive 抽样 | - | - | - | ✅ 必须抽样（已验证） |
| BA2 injection | -（Skyrim 无 BA2） | 🔲 待补真实 archive | ⚠️ 样本缺失 | ⚠️ 样本缺失 | 必须抽样 |
| Codepage override | ⚠️ 老 MOD 样本缺失 | ⚠️ 样本缺失 | 视样本 | 视样本 | 行为对照 |

图例：✅ 已验证 / ⚠️ 样本缺失（本机未安装对应游戏或缺少老 MOD）/ 🔲 待补 / - 不适用

## Skyrim SE L3 详细结果

### 1. Strings 二进制格式（Delphi 金样本）
Rust 字节级正确读取 Delphi 生成的三种 strings 文件：

| 格式 | Delphi 条目数 | Rust 解析 | XML Dest 交叉核对 |
|---|---|---|---|
| .STRINGS | 30294 | ✅ 30294 | - |
| .DLSTRINGS | 2669 | ✅ 2669 | - |
| .ILSTRINGS | 34427 | ✅ 34427 | 34427/34427 全中 |
| **合计** | **67390** | **✅ 67390** | - |

验证点：`Skyrim_chinese.STRINGS[1] = "鼠道地下室"` 与 Delphi XML `sID=1` 的 `<Dest>` 完全一致。

### 2. ESP 解析 → XML 导出对照
Rust 解析 Skyrim.esm（75757 条字符串）→ 与 Delphi XML 对照：

- **Key 匹配**：67390/67390（100%，Delphi 所有条目 Rust 都能按 `str_id:record:field` 定位）
- **Source 内容精确匹配**：66052 条（97.98%）
- **仅尾部空白差异**：1338 条（Rust 保留原文 trailing whitespace，Delphi 在导出阶段 trim）
- **真实内容差异**：0 条
- **Rust 独有条目**：178 条（详见第 4 节）

### 3. SST 读取
Rust `SstDictionary::read_from` 可读取 Delphi 生成的 `Skyrim_english_chinese.sst`：**67390 条全部成功解析**。

### 4. Rust 独有 178 条的根因
Rust 多提取的 178 条主要来自 `-proc` 标记的 record definition 被忽略：

- `PERK:EPFD`（大量）：Delphi 标记 `Def_:EPFD=PERK=0-proc2`，EPFD 是否字符串由 EPFT 字段决定（EPFT 指定位字符串/lstring 时才是）；Rust 忽略 proc2，把所有 PERK:EPFD 当字符串提取，包括浮点型的（如 `43A00000`、`3E800000`）。
- `GMST:DATA`：Delphi 标记 `Def_:DATA=GMST=0-proc1`，条件满足时才作为字符串；Rust 忽略。
- `AMMO:DESC` / `COLL:DESC` / `SPEL:DESC` / `WEAP:DESC` 的 FormID=0 条目：Delphi 可能跳过 FormID=0 的记录。

**根因**：`parser.rs::parse_record_defs` 在解析 `_recorddefs.txt` 时显式丢弃 `-procN` 标记（注释 line 688「忽略后续标记」）。Delphi 用 proc 标记实现条件判断：

- **proc1（GMST:DATA）**：仅当该 GMST 的 `EDID` 首字节为小写 `s`（ASCII 115）时，DATA 才作为字符串（list 0）提取（`TESVT_espDefinition.pas:692-700` `tfieldCheckXtraProcGMST`）。
- **proc2（PERK:EPFD）**：仅当记录存在 `EPFT` 字段、其单字节值严格等于 `7`、且 `EPFD` 位于该 `EPFT` 后最多 3 个 fList 位置内时，才作为字符串（list 0）提取（`TESVT_espDefinition.pas:718-729` `tfieldCheckXtraProcPERK`）。

Rust 忽略 proc 标记后，把所有 `Def_:FIELD=RECORD=LIST` 定义无条件当字符串提取，导致 PERK:EPFD 浮点型（如 `43A00000`、`3E800000`）与 GMST:DATA 非字符串型被误纳入。

**状态**：真实 parity 缺陷，已记录在 `docs/delphi_parity_remediation_plan.md`（新整改项 R-12 候选项，待排期实现 proc1/proc2 条件判断）。当前不影响译文质量（这些条目是噪音而非错误），但会造成 Rust 字符串表比 Delphi 多 178 条无关条目。修复需要理解 Delphi `tfieldCheckXtraProcGMST` / `tfieldCheckXtraProcPERK` 的条件逻辑（已在 `legacy/original-delphi/TESVT_espDefinition.pas` 中定位），属于 parser 核心行为修改，需谨慎且单独提交。

### 5. Trailing Whitespace（1338 条）
Rust 忠实保留原文尾部空白，Delphi 在 XML 导出时 trim。属于**有意的格式保留差异**：翻译工具保留原文尾部空白通常更安全（避免改变段落/句子格式）。记录在案，暂不强制对齐 Delphi 的 trim 行为。

## 修复记录（R-10 期间完成）

### FIX-1：Strings 归档查找目录（BSA 回退路径 bug）
**文件**：`crates/xt-core/src/esp/parser.rs`（`StringsFiles::load_from_dir_with_config`）

**问题**：Rust 在 strings 文件所在目录（如 `<Data>/Strings/`）内查找 BSA/BA2 归档，但该目录为空；真实归档（如 `Skyrim - Interface.bsa`）位于其父目录 `<Data>/`。

**对照**：Delphi `TESVT_main.pas:4146` 使用 `GetParentDirectory(folder)` 在 strings 目录的父目录查找 BSA。

**修复**：归档搜索先查 strings 目录，再回退到其父目录（`scan_dirs` 包含 `dir` 与 `dir.parent()`）。

**验证**：修复后，对 Skyrim SE 调用 `load_from_dir("<Data>/Strings", "Skyrim")` 正确从 BSA 加载英文 strings：
- .STRINGS 30294（source=Archive）
- .DLSTRINGS 2669（source=Archive）
- .ILSTRINGS 34427（source=Archive）

此修复使 golden-diff 的 source diff 从 **67390 条（100% 不匹配）** 收敛到 **1338 条（仅尾部空白）**。

### FIX-2：golden_diff 空壳步骤
**文件**：`crates/xt-cli/src/commands/golden_diff.rs`

**问题**：`[2/5] Reading Delphi golden files` 步骤原本是空壳（只有 print，无逻辑），`compare_strings_files` 函数完整实现但被 `#[allow(dead_code)]` 隐藏且从未调用。

**修复**：调用 `compare_strings_files` 并接入 `print_summary`；重写该函数为「直接验证 Rust 字节级读取 Delphi 金样本 strings 文件」（不再错误地与 ESP 解析结果做跨语言文本比对）。

## 已知限制

1. **Fallout 4 / 76 / Starfield 样本缺失**：本机仅安装 Skyrim SE，无法做真实样本交叉验证。这些单元格标记为「样本缺失」，不可伪造为通过。
2. **BA2 真实 archive（Fallout 4）待补**：Skyrim 无 BA2 文件；需要 Fallout 4 安装才能抽样验证。
3. **Delphi 可执行程序缺失**：本环境无 Delphi 12.1 CE 与 xTranslator 1.6.0 二进制，无法在 CI 中实时重新生成金样本。金样本为预先放置的静态参考文件（`tests/fixtures/delphi_golden/`）。
4. **Release 构建前置条件**：`libz-ng-sys` 在普通 shell 下因找不到 VS CMake generator 失败（见整改计划 R-10 注记）。238MB Skyrim.esm 的实时解析需 `--release` 或在加载 VS Developer 环境的 shell 中运行。当前 L3 验证通过 debug 二进制 + SQLite 缓存完成。
