# 下一阶段开发计划

> 基于当前状态（SPEC 核心功能 100%、UI ~78%、代码审查 12/12 项闭环）
> 编制日期：2026-05 | 最后更新：2026-05

---

## 当前项目状态

```
核心引擎 (ESP/SST/XML/Strings)   ■■■■■■■■■■ 100%
翻译 API (6 providers)           ■■■■■■■■■■ 100%
Header Processor                 ■■■■■■■■■■ 100%
UI 组件                          ■■■■■■■□□□  ~78% (渐进式改进)
E2E 测试覆盖                     ■■■■■■■■■□  ~85% (67 tests, 16 模块)
跨游戏验证                       □□□□□□□□□□   0% (需游戏数据)
代码审查                         ■■■■■■■■■■ 100% (12/12 项闭环)

## 已完成 Phase

| Phase | 说明 | 状态 |
|-------|------|------|
| 1 VMAD 片段处理 | PERK/PACK/SCEN/INFO/QUST 片段跳过 + 写回保留 | ✅ |
| 2 启发式搜索增强 | Delphi 评分集成（词级哈希/LCS/LCP/代理惩罚） | ✅ |
| 5 E2E 测试增强 | 5/5 场景（23 tests）：搜索/编辑/ESP/批量/拼写 | ✅ |
| — 代码审查修复 | 两轮共 12 项发现全部闭环 | ✅ |
```

---

## Phase 1 — VMAD 片段处理（预计 2-3 天）

**目标：** 完成 P4.4 剩余工作——PERK/PACK/SCEN/INFO/QUST 片段中嵌套脚本的解析和写回。

> **状态：✅ 已完成**

**变更：**
- `vmad.rs`: 新增 `has_fragments()` / `skip_fragment_data()`，`decode_vmad_fast`、`VmadDecoder::decode()`、`write_back_rebuild` 全部增加片段处理
- 7 个测试用例添加片段数据（fragmentVersion=0, fragmentCount=0）
- 验证：`cargo test` 293 passed，`cargo check` 零警告

---

## Phase 2 — 启发式搜索增强（预计 3-4 天）

**目标：** Delphi 的 `TESVT_FastSearch.pas`（708 行）包含 30+ 比较器，当前 Rust `matching.rs` 仅覆盖核心 4 层匹配（T1-T4）和 Levenshtein/LCS/LCP。补齐 Delphi 原版的额外搜索策略。

| 任务 | 文件 | 说明 | 预估 |
|------|------|------|------|
| 2.1 Delphi 比较器审计 | `docs/delphi_analysis.md` | 列出 FastSearch.pas 中全部比较器，标记 Rust 已覆盖/未覆盖 | 0.5 天 |
| 2.2 二分搜索增强 | `crates/xt-core/src/matching.rs` | 实现 Delphi 的 `TStringList` 二分搜索风格的快速候选人筛选 | 1 天 |
| 2.3 多级评分排序 | `crates/xt-core/src/heuristic/delphi_scoring.rs` | Delphi 原版使用了加权多维度评分系统，当前实现可能未完全对齐 | 1 天 |
| 2.4 单元测试覆盖 | `crates/xt-core/src/matching.rs` tests | 为新增比较器添加验证测试 | 0.5 天 |

**验收标准：** 在 Skyrim.esm 数据集上，启发式搜索的 Top-5 推荐结果与 Delphi 原版一致。

---

## Phase 3 — 前端剩余组件打磨（预计 3-5 天）

**目标：** 将 UI 完成度从 78% 提升至 90%+，对标 Delphi 原版功能完整度。

| 组件 | 当前完成度 | 待补功能 | 预估 |
|------|-----------|----------|------|
| McmPanel | ~60% | MCM 配置编辑器增强、翻译状态同步、比较结果可视化改进 | 1.5 天 |
| EspComparePanel | ~60% | 差异列表交互增强、过滤/排序、导出差异报告 | 1 天 |
| DataConfigsPanel | ~70% | 配置项搜索/分类、编辑保存、多游戏配置切换 | 0.5 天 |
| 全局 UX | — | 主题微调、动画过渡、加载状态统一、键盘快捷键更完整 | 1 天 |
| 无障碍 | — | aria-label、焦点管理、屏幕阅读器支持 | 0.5 天 |

**验收标准：** 每个面板的操作路径可用键盘完整操作，无未处理的边界状态 UI。

---

## Phase 4 — 跨游戏验证（预计 3-5 天，需各游戏数据）

**目标：** 验证 ESP 解析器在 Skyrim LE、Fallout 4、Fallout NV、Fallout 76、Starfield 上的正确性。

| 游戏 | 验证项 | 依赖 | 预估 |
|------|--------|------|------|
| Skyrim LE | 字符串提取数量 vs Delphi、BSA v0x68 | Skyrim.esm | 0.5 天 |
| Fallout 4 | BA2 支持验证、Strings 文件加载 | Fallout4.esm | 1 天 |
| Fallout NV | 旧格式兼容（TES4 → Oblivion 风格） | FalloutNV.esm | 1 天 |
| Fallout 76 | 在线数据格式的特殊性 | SeventySix.esm | 1 天 |
| Starfield | 新版记录头格式 | Starfield.esm | 1.5 天 |

**验收标准：** 每个游戏的主要对话记录（DIAL/INFO）的字符串提取数量与 Delphi 原版 ±5% 以内。

---

## Phase 5 — E2E 测试增强（预计 2-3 天）

**目标：** 从当前 1 个 Playwright spec 扩展到覆盖核心用户工作流。

| 场景 | 说明 | 预估 |
|------|------|------|
| ESP 加载 → 查看字符串 | 模拟文件选择 → 验证表格渲染正确 | 0.5 天 |
| 搜索/过滤 | 输入搜索词 → 验证高亮和过滤结果 | 0.5 天 |
| 编辑保存 | 修改翻译 → 保存 SST → 重新加载验证 | 0.5 天 |
| 批量翻译 | 配置 API → 启动批处理 → 验证进度事件 | 0.5 天 |
| 拼写检查 | 加载词典 → 检查文本 → 查看建议 | 0.5 天 |

> **状态：✅ 已完成（5/5 场景）**

**变更：**
- `tauri-event.ts` — 增强 mock，新增 `__dispatch`/`__resetListeners`/全局暴露，支持测试驱动事件派发
- `tauri-core.ts` — 新增 `spell_check_*`、`start_string_batch_translate`、`get_batch_status` mock 响应
- `app.spec.ts` — 新增 `@batch` 模块（3 test：工具栏可见/选择启用按钮/事件驱动进度）和 `@spell` 模块（2 test：设置按钮存在/mock 数据合约）

**当前 E2E 覆盖：** 6 smoke + 1 theme + 3 search + 3 edit + 2 esp + 3 batch + 2 spell = **23 tests**

**验收标准：** 关键用户路径的 Playwright E2E 测试稳定通过。

---

## 优先级建议

| 优先级 | Phase | 投入 | 收益 | 推荐理由 |
|--------|-------|------|------|----------|
| 🥇 | Phase 1 VMAD 片段 | 2-3 天 | ESP 写回完整 | P4.4 尾部工作，范围明确 |
| 🥇 | Phase 2 搜索增强 | 3-4 天 | 翻译效率提升 | 直接影响核心翻译体验 |
| 🥈 | Phase 3 前端打磨 | 3-5 天 | UI 完成度 78% → 90%+ | 用户感知改善最大 |
| 🥉 | Phase 4 跨游戏验证 | 3-5 天 | 质量保证 | 需游戏数据，依赖外部条件 |
| 🥉 | Phase 5 E2E 测试 | 2-3 天 | 回归防护 | 短期不影响功能交付 |

> **优先推进 Phase 1 + Phase 2**：两者都有明确的范围、可验证的交付标准，且直接提升核心翻译功能的完整度。
