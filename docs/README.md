# xTranslator Documentation

This directory is organized by reader need. Start with the repository entry points below, then move into architecture notes, format references, or historical plans only when you need them.

## Start Here

| Document | Use it for |
|----------|------------|
| [`../README.md`](../README.md) | Product overview, supported workflows, quick start, build/test commands |
| [`../AGENTS.md`](../AGENTS.md) | AI development guide: workspace, architecture, gotchas, conventions |
| [`../SPEC.md`](../SPEC.md) | Canonical goals, constraints, interfaces, invariants, and task completion |
| [`../ARCHITECTURE.md`](../ARCHITECTURE.md) | Data flow, module responsibilities, IPC patterns, and implementation rules |
| [`../LAYOUT_REDESIGN_PLAN.md`](../LAYOUT_REDESIGN_PLAN.md) | UI layout redesign (3 phases completed) and completion assessment |
| [`ui_reproduction_plan.md`](ui_reproduction_plan.md) | 原版 Delphi 界面复刻方案：截图对照、当前实现对比、后续复刻顺序 |
| [`release_qa.md`](release_qa.md) | Reusable release verification checklist and real-data smoke test plan |

## Reading Order

1. Read `README.md` if you are new to the project.
2. Read `AGENTS.md` before making code changes.
3. Read `SPEC.md` before making behavior changes.
4. Read `ARCHITECTURE.md` before touching IPC, state flow, or parser boundaries.
5. Use the docs below as focused references, not as a second source of truth.

## Current Planning And Roadmap

| Document | Use it for |
|----------|------------|
| [`feature_comparison.md`](feature_comparison.md) | Gap analysis against Delphi xTranslator and next-priority candidates |
| [`ui_reproduction_plan.md`](ui_reproduction_plan.md) | Active Delphi UI recreation plan based on current screenshots, form mapping, and implementation gaps |
| [`development_roadmap.md`](development_roadmap.md) | Comprehensive Delphi parity gaps and development roadmap with priorities and effort estimates |
| [`delphi_analysis.md`](delphi_analysis.md) | Delphi source findings mapped to Rust implementation areas |
| [`delphi_rust_fix_plan.md`](delphi_rust_fix_plan.md) | Delphi → Rust inconsistency remediation plan |
| [`cross_validation_report.md`](cross_validation_report.md) | Cross-validation between Delphi and Rust implementations |
| [`../legacy/original-delphi/README.md`](../legacy/original-delphi/README.md) | Original Delphi project archive layout |

## Architecture Notes

| Document | Use it for |
|----------|------------|
| [`i18n_architecture.md`](i18n_architecture.md) | UI localization architecture and language coverage |
| [`esp_grup_tracking.md`](esp_grup_tracking.md) | ESP GRUP parent tracking and dialog tree context |

## File Format References

| Document | Use it for |
|----------|------------|
| [`esp_format.md`](esp_format.md) | ESP/ESM binary layout, compressed records, GRUP sizing, translatable field extraction |
| [`strings_format.md`](strings_format.md) | `.STRINGS`, `.DLSTRINGS`, `.ILSTRINGS`, codepage behavior, write-back details |
| [`sst_v8_format.md`](sst_v8_format.md) | SST v8 binary format and Delphi-compatible params |
| [`bsa_format.md`](bsa_format.md) | BSA v0x68/v0x69 and BA2 General structure, archive lookup, compression |
| [`pex_format.md`](pex_format.md) | PEX binary layout and translatable string extraction notes |
| [`fuz_format.md`](fuz_format.md) | FUZ container structure and dialogue/audio association notes |

## Archive

Completed implementation plans and historical documents live in [`archive/`](archive/). They are kept for historical context, but current work should use the docs above first.

| Document | Historical context |
|----------|--------------------|
| [`archive/execution_plan_v1.md`](archive/execution_plan_v1.md) | v1 execution plan after completion |
| [`archive/phase1_5_execution_plan.md`](archive/phase1_5_execution_plan.md) | Tauri UI foundation plan |
| [`archive/bsa_implementation_plan.md`](archive/bsa_implementation_plan.md) | Completed BSA support implementation plan |
| [`archive/api_compat_plan.md`](archive/api_compat_plan.md) | Earlier API feature parity plan |
| [`archive/p3_plan.md`](archive/p3_plan.md) | Earlier phase-based UX/output plan |
| [`archive/next_actions_v1.1.md`](archive/next_actions_v1.1.md) | Earlier v1.1 action list, superseded by development_roadmap.md |

## Maintenance Rules

### 文档角色与优先级

| 文档 | 角色 | 受众 | 更新触发条件 |
|------|------|------|------------|
| `AGENTS.md` | AI 开发主指南 | AI 编码助手 | 新增模块/命令/模式时 |
| `SPEC.md` | 规范真相源（caveman 编码） | 开发者、AI | 任何行为变更 |
| `ARCHITECTURE.md` | 架构设计文档 | 开发者 | IPC/数据流/模块边界变更 |
| `README.md` / `README_zh-CN.md` | 产品入口 | 用户、新开发者 | 功能增删、状态变化 |
| `LAYOUT_REDESIGN_PLAN.md` | 布局复刻记录 | 前端开发者 | UI 布局变更 |
| `RELEASE.md` | 发布流程 + 当前版本 | 发布者 | 每次提交后更新元数据 |
| `docs/feature_comparison.md` | Delphi vs Rust 差距分析 | 开发者、AI | 功能覆盖度变化 |
| `docs/development_roadmap.md` | 开发路线图 | 开发者 | 任务完成/新增 |
| `docs/release_qa.md` | 发布 QA 清单 | QA | 每次发布前 |
| `docs/ui-handover.md` | UI 打磨交接文档 | 前端开发者、AI | UI Phase 完成时 |
| `docs/archive/` | 历史计划归档 | 历史参考 | 计划完成后移入 |

### 每次功能变更后的必做检查

提交重大功能后，必须检查以下数字是否同步（用 `cargo test -p xt-core --lib` 和 `npx tsc --noEmit` 获取最新值）：

| 检查项 | 分布位置 |
|--------|---------|
| 测试数量 | `ARCHITECTURE.md`·`RELEASE.md`·`LAYOUT_REDESIGN_PLAN.md` |
| SPEC 任务完成数 | `README.md`·`README_zh-CN.md`·`RELEASE.md` |
| 翻译 API 提供商数 | `README.md`(2处)·`README_zh-CN.md`(2处)·`ARCHITECTURE.md`·`docs/development_roadmap.md`(2处) |
| 分块大小 (25K) | `README.md`(2处)·`README_zh-CN.md`(2处)·`ARCHITECTURE.md` |
| 最新提交哈希 | `RELEASE.md` |

### 文档清理规则

1. **误放文件立即删除** — 不属于本项目的文档（如其他产品的设计稿）不应存在于仓库中。
2. **被取代的文件应删除或归档** — 新版本就绪后，旧版本移入 `docs/archive/` 或直接删除（如果是一次性日计划/QA记录）。
3. **AI 代理指南合并** — `AGENTS.md` 是主指南，`CLAUDE.md` 仅作存根引用。不要在两者间分裂信息。
4. **中英文 README 同步** — `README.md` 和 `README_zh-CN.md` 的事实数据（数字、功能列表、状态）必须一致。
5. **外部工具产物不入库** — `superpowers/`、`.waylog/` 等第三方工具生成的目录不应提交。
6. **死引用立即修正** — 删除文件后，运行 `rg <deleted_file_name> --include '*.md'` 检查所有残留引用。

### 定期审计命令

```bash
# 检查是否有引用已删除文件的过时链接
rg "toolchain_and_roadmap|IMPLEMENTATION_SUMMARY|DESIGN\.md" --include "*.md" docs/ *.md

# 检查数字一致性
rg "293 tests|290 tests|181 tests|247 tests" *.md docs/
rg "45.*task|100.*task" *.md docs/
rg "4/8|6/8.*provider|翻译 API.*OpenAI.*DeepL.*Baidu.*Youdao.*Azure.*Google" *.md docs/
rg "10K items|25K items" *.md docs/
```
