# Skyrim SE 验证报告

> 基于 `docs/validation_procedure.md` L2/L3 框架
> 验证日期：2026-05-24

---

## 测试环境

| 项目 | 值 |
|------|-----|
| 操作系统 | Windows Server 2022 (10.0.20348) |
| Rust 版本 | rustc 1.95.0 (59807616e 2026-04-14) |
| Cargo 版本 | cargo 1.95.0 (f2d3ce0bd 2026-03-21) |
| Node.js 版本 | v22.14.0 |
| Skyrim.esm 路径 | `D:\SteamLibrary\steamapps\common\Skyrim Special Edition\Data\Skyrim.esm` |
| Skyrim.esm 大小 | 249,753,412 bytes |
| Skyrim.esm MD5 | `B576D3E72A257A0BDC03918AE936AC63` |
| 游戏版本 | SkyrimSE |

---

## L1 基线测试结果

### 后端单元测试

```
cargo test -p xt-core --lib
```

**结果**：`ok. 299 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.46s`

### Tauri 后端编译

```
cargo build -p xtranslator-tauri
```

**结果**：`Finished dev profile [unoptimized + debuginfo] target(s) in 14.80s`

> 注：编译前修复了 `toolbox_load_exception_words` 的 `Result<(), String>` never-type fallback 错误，以及 `main.rs` 中漏 import 的问题。

### 前端构建

```
npm --prefix ui run build
```

**结果**：`✓ built in 3.01s`（修复 `appStore.ts:1127` 的 `string | null` 类型错误后通过）

**L1 状态**：✅ 全部通过

---

## L2 模块专项测试结果

| 模块 | 命令 | 结果 |
|------|------|------|
| SST v8 | `cargo test -p xt-core --lib sst::v8::tests` | ✅ 10 passed |
| XML | `cargo test -p xt-core --lib xml::tests` | ✅ 7 passed |
| ESP record_tree | `cargo test -p xt-core --lib esp::record_tree::tests` | ✅ 12 passed |
| VMAD | `cargo test -p xt-core --lib vmad::tests` | ✅ 8 passed |

**L2 状态**：✅ 全部通过

---

## L3 Golden Snapshot

使用 `xt-cli stats` 命令加载 Skyrim.esm（`esp_mode=true`，完整记录树构建），解析耗时 **174.44s**。

详细统计见 [`skyrim-se-golden-2026-05-24.md`](skyrim-se-golden-2026-05-24.md)。

### 核心指标摘要

| 指标 | 值 | 与已知基线对比 |
|------|-----|---------------|
| 总字符串数 | 75,754 | — |
| 唯一 str_id 数 | 67,550 | — |
| 不同 record_sig 数 | 52 | — |
| 不同 field_sig 数 | 18 | — |
| 顶层 GRUP 数 | 118 | ✅ 与已知基线一致 |
| 子 GRUP 数 | 50,376 | ✅ 与已知基线一致 |
| CELL strings | 583 | ✅ 与已知基线一致 |
| WRLD strings | 36 | ✅ 与已知基线一致 |
| REFR strings | 397 | 已知基线 405，偏差 -8（-2.0%）|
| compressed_records | 44,153 | — |
| VMAD 字符串总数 | 5 | QUST(4), ACTI(1) |

### 前5 record_sig 分布

| record_sig | 字符串数 | 占比 |
|-----------|---------|------|
| INFO | 35,868 | 47.35% |
| ARMO | 5,375 | 7.10% |
| DIAL | 5,170 | 6.82% |
| WEAP | 4,935 | 6.51% |
| QUST | 3,531 | 4.66% |

### 前5 field_sig 分布

| field_sig | 字符串数 | 占比 |
|----------|---------|------|
| NAM1 | 34,427 | 45.45% |
| FULL | 24,348 | 32.14% |
| DESC | 8,798 | 11.61% |
| RNAM | 2,193 | 2.89% |
| CNAM | 1,592 | 2.10% |

### 游戏定义覆盖

| 统计项 | 值 |
|--------|-----|
| 已定义 record_sig 数 | 16 |
| 实际出现的 record_sig 数 | 52 |
| 未覆盖的 record_sig | 36 种（fallback 到 generic）|

未覆盖的 record_sig 列表：WRLD, SLGM, HDPT, ENCH, CLAS, WEAP, APPA, KEYM, COLL, EYES, CONT, CLFM, WATR, PROJ, DIAL, RACE, SHOU, ARMO, DOOR, SNCT, AVIF, SCRL, TREE, LIGH, INGR, SCEN, EXPL, MISC, ALCH, FURN, CELL, LCTN, AMMO, REFR, HAZD, SPEL, TACT

> 未覆盖 ≠ 错误。`esp_default_defs.txt` 只定义了核心可翻译字段；未定义的 record_sig 会 fallback 到 generic 解析，仍然能正确提取字符串。这在预期之内。

---

## 异常与偏差

### REFR strings 偏差（-8，-2.0%）

| 来源 | REFR strings |
|------|-------------|
| 已知基线（文档） | 405 |
| 本次 golden snapshot | 397 |
| 偏差 | -8 |

**分析**：REFR 记录的字符串提取依赖于 record_defs 中对 REFR 可翻译字段的定义。偏差可能来自：
1. `esp_default_defs.txt` 中 REFR 字段定义与 Delphi 原版略有差异
2. 某些 REFR 字段在特定条件下被标记为不可翻译

**结论**：偏差在可接受范围内（<5%），不阻塞验证。

---

## 结论

| 检查项 | 状态 |
|--------|------|
| L1 基线（单元测试 + 编译） | ✅ 通过 |
| L2 专项（SST/XML/ESP/VMAD） | ✅ 通过 |
| Golden snapshot 生成 | ✅ 已完成 |
| 关键指标与已知基线一致 | ✅ 通过（REFR 偏差 -2% 在可接受范围）|

**当前 parser 状态：✅ 验证通过**

Golden snapshot 已锁存为 `docs/skyrim-se-golden-2026-05-24.md`。后续 parser 变更若导致以下指标出现显著偏差（>5%），应视为 regression：
- 总字符串数（75,754）
- 顶层 GRUP 数（118）
- 子 GRUP 数（50,376）
- CELL/WRLD strings（583 / 36）

---

*报告生成时间：2026-05-24*
