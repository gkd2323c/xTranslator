# xTranslator Validation Procedure

> 目标：为每次 parser 变更提供可重复的验证流程。
> 创建日期：2026-05-20
> 适用范围：ESP 解析器、SST 读写、XML 导入导出

---

## Overview

本流程分三级：

| 级别 | 覆盖 | 运行时间 | 触发条件 |
|------|------|---------|---------|
| **L1 — 单元测试** | 所有 parser 模块的单测 + 集成测试 | ~1s | 每次提交 |
| **L2 — 可重复验证** | 特定样本的固定流程 + golden file diff | ~5min | 每次 parser 变更 |
| **L3 — 全量交叉验证** | 真实游戏数据的 Delphi 对照 | ~1-2 天 | 发布前 / Delphi 环境可用时 |

---

## L1 — 单元测试（自动）

```bash
# 核心库全部测试
cargo test -p xt-core --lib

# 特定关注模块
cargo test -p xt-core --lib esp::parser
cargo test -p xt-core --lib sst
cargo test -p xt-core --lib xml
cargo test -p xt-core --lib vmad
cargo test -p xt-core --lib config
cargo test -p xt-core --lib spell

# Tauri 后端编译
cargo build -p xtranslator-tauri

# 前端类型检查
cd ui && npx tsc --noEmit
```

**验收条件**：以上全部通过。

---

## L2 — 可重复验证

本级别使用 `tests/fixtures/` 下的固定样本，不依赖真实游戏数据。

### 2.1 SST 读写 roundtrip

```bash
cargo test -p xt-core --lib sst::v8::tests
```

验证项：
- SST v8 空文件读写 ✅
- SST v8 Unicode roundtrip ✅
- SST v8 10 条 roundtrip ✅
- SST merge: 新条目添加 ✅
- SST merge: 更新已有（空译文填充） ✅
- SST merge: 冲突不覆盖 ✅
- SST merge: 冲突覆盖 ✅

### 2.2 XML roundtrip

```bash
cargo test -p xt-core --lib xml::tests
```

验证项：
- XML 解析基础 ✅
- XML 写入 + 解析 roundtrip ✅
- XML escape ✅
- XML → SkyStrings + 回写 ✅

### 2.3 ESP 解析 roundtrip

```bash
cargo test -p xt-core --lib esp::record_tree::tests
```

验证项：
- 基础字段解析 ✅
- 压缩字段 decompress/recompress ✅
- XXXX 扩展字段 ✅
- 重建输出与输入一致 ✅
- 嵌套 GRUP 大小重算 ✅
- rebuild_with_translation ✅
- serialization roundtrip ✅

### 2.4 Nested GRUP 验证（已通过）

验证结果（基于 Skyrim.esm 真实数据，2026-05-12）：

| 指标 | 值 |
|------|-----|
| 顶层 GRUP | 118 |
| 子 GRUP | 50,376 |
| CELL strings | 583 ✅ |
| WRLD strings | 36 ✅ |
| REFR strings（仅 CELL 子 GRUP） | 405 ✅ |

验证记录：`docs/development_roadmap.md` P4.2，commit `564859d`.

### 2.5 Delphi golden 文件快照

`tests/fixtures/delphi_golden/` 存放 Delphi 1.6.0 生成的参考文件：

```
tests/fixtures/delphi_golden/
├── Skyrim_english_chinese.xml       # Delphi XML 导出（67,390 条已翻译）
├── Skyrim_english_chinese.sst       # Delphi SST（v8 格式）
├── Skyrim_chinese.STRINGS           # Delphi strings（null-terminated）
├── Skyrim_chinese.DLSTRINGS         # Delphi strings（长度前缀）
└── Skyrim_chinese.ILSTRINGS         # Delphi strings（索引）
```

**用途**：
- XML 文件可用于 Rust parser 字段数量对照（详见 `docs/cross_validation_report.md`）
- SST 文件可用于双向兼容测试
- Strings 文件可用于 CP1252/UTF-8 编码一致性验证

**限制**：Delphi 导出为"仅已翻译条目"，非全量。需全量导出才能完成逐字符串 diff。

---

## L3 — 全量交叉验证

### 3.1 阻塞条件

当前 L3 验证依赖以下外部条件：

| 条件 | 状态 | 替代方案 |
|------|------|---------|
| Delphi xTranslator 1.6.0 运行环境 | ⛔ 不可用 | 使用已有 golden 文件做有限对照 |
| 真实 Skyrim.esm（249MB） | ✅ 本地可用 | 通过环境变量 `XTRANSLATOR_TEST_SKYRIM_ESM` 引用 |
| 全量 Delphi XML 导出（含空译文） | ⛔ 不可用 | 锁存当前 parser 输出为 golden file，后续变更与 golden diff |

### 3.2 替代验证路径

#### 路径 A：当前 parser 行为快照

当 Delphi 不可用时，锁定当前 parser 输出为 golden reference：

```bash
# 需要真实 ESM 文件（配置 XTRANSLATOR_TEST_SKYRIM_ESM 环境变量）
# e2e 测试定义在独立的 xtranslator-tests package 中
cargo test --release -p xtranslator-tests --test e2e_comprehensive
```

输出存档项：
- 字符串总数与分布
- GRUP 树结构统计
- 所有 `record_defs` 覆盖情况
- VMAD 提取字符串列表

每次 parser 变更后比较输出差异。仅 `record_defs.txt` 中的字段变化应产生 diff，解析逻辑的 passive regression 不应产生 diff。

#### 路径 B：社区导出样本

从 xEdit 社区获取已知 MOD 的：
- 原始 ESP
- Delphi xTranslator 的 XML 导出
- 手工验证过的翻译数据

使用 `xt-cli` 的 `diff` 命令逐字符串对比：

```bash
cargo run -p xt-cli -- diff sample.esm delphi_export.xml
```

### 3.3 验证清单

```markdown
- [ ] L1: cargo test -p xt-core --lib 全部通过
- [ ] L1: cargo build -p xtranslator-tauri
- [ ] L1: cd ui && npx tsc --noEmit
- [ ] L2: SST roundtrip 测试通过
- [ ] L2: XML roundtrip 测试通过
- [ ] L2: ESP 解析 roundtrip 测试通过
- [ ] L2: merge 测试（新增/更新/冲突）通过
- [ ] L3: （如数据可用）e2e 测试通过
- [ ] L3: （如 Delphi 可用）逐字符串 diff 一致 ≤0.1% 差异
- [ ] 文档：`docs/feature_comparison.md` 无过时状态
- [ ] 文档：`docs/cross_validation_report.md` 后续步骤已更新
```

---

## FAQ

**Q: 为什么不用真实游戏数据做自动化测试？**
A: 真实 ESM 文件（249MB+）受版权保护，不能提交到仓库。CI 环境也无法访问游戏数据。

**Q: 每次 parser 变更都要跑 L3 吗？**
A: 不需要。L3 只在以下情况触发：
- 新游戏格式支持（如 Starfield、ESX 新版本）
- 影响字段定义宽度（record_defs.txt 修改）
- 发布前的最终验证

**Q: 没有 Delphi 怎么做完全验证？**
A: 当前准确率基于 DIAL 等 14 个类型的精确匹配（见 `docs/cross_validation_report.md`）。没有 Delphi 环境的情况下，替代方案是用当前 Rust parser 输出作为新的 golden baseline，后续变更与之 diff。

---

## 版本历史

| 日期 | 版本 | 修改 |
|------|------|------|
| 2026-05-20 | v1 | 初始版本，从现有文档提炼验证流程 |
