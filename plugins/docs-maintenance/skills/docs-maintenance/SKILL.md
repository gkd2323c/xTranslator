---
name: docs-maintenance
description: xTranslator 文档维护 — 每次功能提交后自动检查并修正所有文档中的过时数字、死引用和事实错误。
---

# xTranslator 文档维护技能

每次做完功能变更后，使用此技能确保所有项目文档的一致性。完整规范见 `docs/README.md#maintenance-rules`。

## 触发条件

- 提交了功能变更
- 新增/删除了翻译 API 提供商
- 新增/删除了 IPC 命令或模块
- 测试数量发生变化
- 删除了文档文件
- 用户要求 "更新文档" 或 "检查文档"

## 工作流

### Step 1: 获取当前事实数据

```powershell
# 测试数量
cargo test -p xt-core --lib 2>&1 | select-string "test result:" 

# TypeScript 编译状态
cd ui; npx tsc --noEmit 2>&1

# 最新提交
git log --oneline -1
```

### Step 2: 检查数字一致性

搜索每个关键数字在所有文档中的出现位置，确保一致：

| 数字 | 命令 | 预期值 |
|------|------|--------|
| 测试数 | `rg "test.*\d{3}|283 tests" *.md docs/` | `cargo test -p xt-core --lib` 输出 |
| SPEC 任务数 | `rg "\d+.*task|100.*task|45.*task" *.md docs/` | SPEC.md 中 `[x]` 数量 |
| API 提供商数 | `rg "4/8|6/8.*provider|翻译 API.*provider" *.md docs/` | `translation_api/` 下 `*Provider` impl 数量 |
| 批量大小 | `rg "10K items|25K items" *.md docs/` | 25K |
| 主题列表 | `rg "Dark/Light/Gray|Obsidian" *.md` | Obsidian, Slate, Light, Auto |
| 最新提交 | `rg "最后提交|最新提交|latest commit" RELEASE.md` | `git log --oneline -1` |

### Step 3: 检查死引用

```powershell
# 搜索最近删除的文件的残留引用（替换 <filename> 为实际文件名）
rg "<filename>" --include "*.md" *.md docs/
```

### Step 4: 修复发现的差异

按照 `docs/README.md` 中的文档角色矩阵确定需要更新哪些文件。顺序：
1. 先更新真相源（SPEC.md、AGENTS.md）
2. 再更新分发文档（README.md、ARCHITECTURE.md)
3. 最后更新辅助文档（RELEASE.md、LAYOUT_REDESIGN_PLAN.md））

### Step 5: 中英文同步

如果修改了 README.md 中的事实数据，同步更新 README_zh-CN.md。

### Step 6: 提交

```powershell
git add <changed docs> 
git commit -m "docs: sync documentation after <简述变更>"
```

## 文档真相源映射

| 事实 | 真相源 | 同步目标 |
|------|--------|---------|
| 测试数量 | `cargo test -p xt-core --lib` | `ARCHITECTURE.md`·`RELEASE.md`·`LAYOUT_REDESIGN_PLAN.md` |
| SPEC 任务数 | `SPEC.md` 中 `[x]` 计数 | `README.md`(×2)·`RELEASE.md` |
| API 提供商数 | `translation_api/` 模块 | `README.md`(×2)·`ARCHITECTURE.md`·`development_roadmap.md`(×2) |
| 批量大小 | `commands.rs` 中 `CHUNK_SIZE` | `README.md`(×2)·`ARCHITECTURE.md` |
| 后端模块数 | `xt-core/src/lib.rs` pub mod 声明 | `ARCHITECTURE.md` |
| IPC 命令数 | `commands.rs` 中 `#[tauri::command]` 计数 | `ARCHITECTURE.md` |
| 主题列表 | `appStore.ts` Theme 类型 | `README.md`(×2·`feature_comparison.md` |
| 前端组件数 | `ui/src/components/` 目录 | `LAYOUT_REDESIGN_PLAN.md` |
