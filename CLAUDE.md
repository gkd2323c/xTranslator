# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

**主指南：参见 `AGENTS.md`** — 包含工作区结构、构建命令、架构模式、IPC 规范、前端组件树、惯例和限制。

## 快速参考

### 构建 & 测试
```bash
cargo build --workspace              # 编译全部
cargo test -p xt-core --lib          # 核心单元测试（359 tests, 0 warnings）
cd ui && npx tsc --noEmit           # 前端类型检查
cd ui && npm run test               # 前端 Vitest
```

### 关键原则
- **Update by ID, not index** — `update_translation(id, text)` 使用 `u32 id`
- **DTO 同步** — `crates/xt-shared/src/dto.rs` ↔ `ui/src/api/strings.ts`
- **zustand** — 使用 `useAppStore((s) => s.field)`，不要解构整个 store
- **react-window v2** — `rowComponent`/`rowCount`/`rowProps`，不是 v1 的 `children`/`itemCount`

### 文件定位
| 需求 | 文件 |
|------|------|
| 修改字符串表格 | `ui/src/components/StringTable.tsx` |
| 修改编辑器 | `ui/src/components/EditorPanel.tsx` (导出 `EditorDialog`) |
| 添加工具弹窗 | `ui/src/App.tsx` (Modal 包裹) + `ui/src/stores/appStore.ts` (activePanel) |
| 添加日志功能 | `ui/src/stores/appStore.ts` (LogEntry / addLog / clearLogs) + `ui/src/components/bottom/LogPanel.tsx` |
| 后端核心逻辑 | `crates/xt-core/src/` 对应模块 |

### 版本要求
- Rust 1.70+ (edition 2021) · Tauri 2.x · Node.js 18+
- Windows (primary), macOS, Linux
