# Skyrim SE 验证硬化实施计划

> 基于 `docs/validation_procedure.md` L2/L3 框架，专注 Skyrim SE
> 跨游戏验证已搁置，本计划不涉及 LE/Fallout/Starfield

---

## 目标

对 Rust xTranslator 的 ESP 解析器在 Skyrim SE 数据上建立可重复验证流程，锁存当前行为为 golden reference，确保后续 parser 变更不引入 regression。

---

## 验证范围

| 模块 | 验证项 | 覆盖 |
|------|--------|------|
| ESP 解析 | 字符串提取总数 vs 已知基线 | 全量 |
| ESP 解析 | record_sig / field_sig 覆盖 | 全量 |
| SST 读写 | roundtrip 一致 | ✅ 已有测试 |
| XML 导入导出 | roundtrip 一致 | ✅ 已有测试 |
| VMAD 字段 | 脚本字符串提取 + 写回 | ✅ 已有测试 |
| 嵌套 GRUP | 树结构统计 | ✅ 已通过 |
| ESP 写回 | rebuild_with_translation roundtrip | ✅ 已有测试 |

---

## 实施步骤

### Step 1: 确认测试数据

需要一份可访问的 Skyrim SE `Skyrim.esm`（~249MB），通过环境变量引用：

```bash
# 设置 ESM 路径
set XTRANSLATOR_TEST_SKYRIM_ESM=D:\Games\SkyrimSE\Data\Skyrim.esm
```

验证可用性：

```bash
cargo test --release -p xtranslator-tests --test e2e_comprehensive 2>&1 | tail -20
```

如果 `e2e_comprehensive` 不存在或路径不同，改为手动加载验证。

### Step 2: 运行 L1 基线

```bash
cargo test -p xt-core --lib 2>&1 | tail -5
# 预期: 293 passed, 0 failed

cargo build -p xtranslator-tauri 2>&1 | tail -3
# 预期: Finished

npm --prefix ui run build 2>&1 | tail -5
# 预期: ✓ built
```

### Step 3: 运行 L2 测试

```bash
cargo test -p xt-core --lib sst::v8::tests 2>&1 | tail -5
cargo test -p xt-core --lib xml::tests 2>&1 | tail -5
cargo test -p xt-core --lib esp::record_tree::tests 2>&1 | tail -5
cargo test -p xt-core --lib vmad::tests 2>&1 | tail -5
```

全部通过后，L2 验证完成。

### Step 4: 锁存当前 parser 输出为 golden snapshot

运行一次完整的 parser 输出抓取，将以下信息写入 `docs/skyrim-se-golden-<date>.md`：

**4.1 字符串总数**

| 类别 | 数量 |
|------|------|
| 总字符串数 | ? |
| 唯一 str_id 数 | ? |
| 不同 record_sig 数 | ? |
| 不同 field_sig 数 | ? |

**4.2 record_sig 分布**

按条数降序列出所有出现的 record_sig：

| record_sig | 字符串数 | 占比 |
|------------|---------|------|
| INFO | ? | ?% |
| DIAL | ? | ?% |
| ... | ... | ... |

**4.3 field_sig 分布**

按条数降序列出所有出现的 field_sig：

| field_sig | 字符串数 | 占比 |
|-----------|---------|------|
| DNAM | ? | ?% |
| ... | ... | ... |

**4.4 GRUP 树结构**

| 指标 | 值 |
|------|-----|
| 顶层 GRUP 数 | 118 (已知) |
| 子 GRUP 数 | 50,376 (已知) |
| CELL strings | 583 (已知) |
| WRLD strings | 36 (已知) |
| REFR strings | 405 (已知) |

**4.5 VMAD 字符串统计**

| 指标 | 值 |
|------|-----|
| 含 VMAD 的记录数 | ? |
| VMAD 字符串总数 | ? |
| 涉及的 record_sig | INFO, PERK, PACK, ... |

**4.6 游戏定义覆盖**

检查 `Data/` 目录下 record_defs 对 Skyrim SE 的覆盖情况：

| 统计项 | 值 |
|--------|-----|
| 已定义 record_sig 数 | ? |
| 实际出现的 record_sig 数 | ? |
| 未覆盖的 record_sig | ? (fallback 到 generic) |

**输出工具**：可通过 `xt-cli` 或新增 Tauri 命令导出统计。如果两者都不方便，前端加载 Skyrim.esm 后从 `get_stats` 命令获取数据，手工记录。

### Step 5: 产出验证报告

创建 `docs/skyrim-se-validation-report.md`，包含：

- 测试环境（OS、Rust 版本、Skyrim.esm MD5/大小）
- L1/L2 测试结果截图
- Step 4 全部统计
- 结论：当前 parser 状态 ✅ / ⚠️

### Step 6: 建立回归检查脚本

在 `tests/` 下或 `scripts/` 下创建 `validate_skyrim_se.sh` / `validate_skyrim_se.ps1`，内容：

```powershell
# validate_skyrim_se.ps1
# 运行所有 L1 测试
cargo test -p xt-core --lib
if ($LASTEXITCODE -ne 0) { exit 1 }

# 运行 L2 测试
cargo test -p xt-core --lib sst::v8::tests
cargo test -p xt-core --lib xml::tests
cargo test -p xt-core --lib esp::record_tree::tests

# 编译检查
cargo build -p xtranslator-tauri
npm --prefix ui run build

Write-Output "All validation checks passed"
```

---

## 验收标准

- [ ] Step 1-3: L1/L2 全部测试通过
- [ ] Step 4: Golden snapshot 文档已创建，包含所有统计项
- [ ] Step 5: 验证报告已产出
- [ ] Step 6: 回归检查脚本已提交
- [ ] golden snapshot 文档中所有字段已填写（非 `?`）

---

## 不在此计划内

| 项目 | 原因 |
|------|------|
| Delphi 交叉验证 | 阻塞 — 需 Delphi 1.6.0 运行环境 |
| Skyrim LE / Fallout / Starfield | 用户决策：跨游戏验证搁置 |
| SST 旧版兼容 | 属于 P5 独立方向 |
| 命令脚本编辑器 | 属于 P5 独立方向 |

---

## 依赖

- 可访问的 Skyrim SE `Skyrim.esm` 文件
- Rust 工具链（cargo）
- Node.js + npm（前端构建验证）
