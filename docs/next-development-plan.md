# 下一阶段开发计划

> 专注 Skyrim SE，跨游戏验证搁置
> 编制日期：2026-05 | 最后更新：2026-05-24

---

## 当前项目状态

```
核心引擎 (ESP/SST/XML/Strings)   ■■■■■■■■■■ 100%
翻译 API (6 providers)           ■■■■■■■■■■ 100%
Header Processor                 ■■■■■■■■■■ 100%
启发式搜索                       ■■■■■■■■■■ 100% (Delphi 评分集成)
VMAD 片段处理                    ■■■■■■■■■■ 100% (含 PERK/PACK/SCEN/INFO/QUST)
拼写检查                         ■■■■■■■■■■ 100% (含配置持久化/自动恢复)
SST 合并                         ■■■■■■■■■■ 100% (前后端全链路)
UI 组件                          ■■■■■■■■■□  ~90% (MCM/EspCompare/FUZ 面板增强完成)
E2E 测试覆盖                     ■■■■■■■■□□  ~78% (6 面板测试: 45/58 通过, 15 待修)
代码审查                         ■■■■■■■■■■ 100% (12/12 项闭环)
验证硬化 (Skyrim SE)             ■■■■■■■■■■ 100% (golden snapshot + 回归脚本)
跨游戏验证 (搁置)                □□□□□□□□□□   0%
```

## 已完成 Phase

| Phase | 说明 | 状态 |
|-------|------|------|
| **VMAD 片段处理** | PERK/PACK/SCEN/INFO/QUST 片段跳过 + 写回保留 | ✅ |
| **启发式搜索增强** | Delphi 评分集成（词级哈希/LCS/LCP/代理惩罚） | ✅ |
| **E2E 测试增强** | 5/5 场景（23 tests）：搜索/编辑/ESP/批量/拼写 | ✅ |
| **代码审查修复** | 两轮共 12 项发现全部闭环 | ✅ |
| **拼写检查收尾** | 配置持久化（dictionary/loaded/active 自动恢复）+ 编辑器集成 | ✅ |
| **验证硬化** | Skyrim SE golden snapshot + 回归脚本 + L1/L2/L3 验证报告 | ✅ |
| **SST Merge 前端** | MergeSstDialog（文件选择/overwrite 策略/统计结果/自动刷新） | ✅ |
| **UI Phase D** | RTL 预览 + HTML 导出 + 协作系统 | ✅ |
| **McmPanel 增强** | 翻译状态徽章、统计明细、批量操作、比较结果对话框 | ✅ |
| **EspComparePanel 增强** | 差异报告导出、diff 高亮、排序、摘要统计条 | ✅ |
| **FuzPanel 改进** | 播放进度条、排序、统计可视化、行可读性改进 | ✅ |
| **E2E 面板测试** | panels.spec.ts 覆盖 MCM/ESP Compare/FUZ/BSA/PEX (5 tests) | ✅ |

---

## 待推进方向

### 方向 1 — UI 打磨 ✅ 已完成

三个缺口面板已全部增强：

| 面板 | 原完成度 | 现完成度 | 新增功能 |
|------|---------|---------|----------|
| **McmPanel** | ~80% | ~92% | 翻译状态徽章、统计明细、批量操作、比较结果对话框 |
| **EspComparePanel** | ~60% | ~80% | 差异报告导出（新 `write_text_file` 命令）、diff 高亮、排序、摘要统计条 |
| **FuzPanel** | ~70% | ~85% | 播放进度条（`requestAnimationFrame` 实时更新）、排序、统计可视化、行可读性改进 |

剩余可打磨（低优先级）：全局 UX（主题微调、键盘快捷键补全）

---

### 方向 2 — 验证硬化（Skyrim SE 范围） ✅ 已完成

- 对真实 Skyrim SE ESP/ESM 产出可重复验证记录：加载 → 导出 → diff → 结论 ✅
- golden snapshot 已锁存：`docs/skyrim-se-golden-2026-05-24.md`（75,754 strings, 118 top GRUPs）✅
- 验证报告已产出：`docs/skyrim-se-validation-report.md`（L1/L2/L3 全部通过）✅
- 回归检查脚本已提交：`scripts/validate_skyrim_se.ps1` ✅
- Delphi 交叉验证 ⏸ 仍阻塞（需 Delphi 1.6.0 运行环境）
- 已有 `docs/validation_procedure.md`（L1/L2/L3 框架）
- 详细实施计划见 [`docs/skyrim-se-validation-plan.md`](docs/skyrim-se-validation-plan.md)

---

### 方向 3 — P5 低优先级

| 项目 | 说明 | 预估 |
|------|------|------|
| SST 旧版兼容 (v1-v7) | 读取旧版 SST 字典格式 | 3-5 天 |
| 命令脚本编辑器 | `commandProcessor.pas` — SynEdit 风格编辑 | 2-3 天 |

---

## 优先级建议

| 优先级 | 方向 | 投入 | 收益 | 理由 |
|--------|------|------|------|------|
| 🥇 | ~~验证硬化（Skyrim SE）~~ | ~~1-2 天~~ | ~~质量闭环~~ | ✅ 已完成：golden snapshot + 回归脚本 + 验证报告 |
| 🥈 | P5 低优先级 | 按需 | 兼容性/扩展 | 命令脚本编辑器（SST 旧版读取已兼容 v1-v7） |

> **当前推荐优先推进验证硬化**：UI 打磨已完成，剩余工作聚焦质量闭环。
