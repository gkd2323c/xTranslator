---
stage: plan
bolt: 003-batch-translation-ui
created: 2026-05-01T12:00:00Z
---

# Implementation Plan: batch-translation-ui

## Objective

为 xTranslator 前端添加批量翻译控制栏、实时进度、取消功能和崩溃恢复提示。

## Deliverables

4 个 React 组件 + Zustand store 扩展:

1. **BatchTranslateBar** — 工具栏控件（按钮 + 并发滑块 + 取消）
2. **ProgressIndicator** — 进度显示 "12/50 已完成"
3. **TranslationSummaryModal** — 完成汇总弹窗（成功/失败/错误）
4. **RecoveryPromptModal** — 启动时恢复提示弹窗

## Dependencies

- `start_string_batch_translate` IPC (002-translation-queue bolt 提供)
- `cancel_string_batch_translate` IPC (002-translation-queue bolt 提供)
- `check_pending_cache` IPC (001-translation-cache bolt 提供)
- `apply_translation_cache` IPC (001-translation-cache bolt 提供)
- Tauri events: `batch-string-progress`, `batch-string-complete`
- Zustand `appStore` (已有): `selectedIds`, `updateItemTranslation`
- react-hot-toast, lucide-react (已有)

## Technical Approach

### 状态管理 (Zustand)

添加 batch state:
```ts
batchState: 'idle' | 'running' | 'cancelling' | 'completed'
batchProgress: { completed: number; total: number }
batchErrors: { strId: number; error: string }[]
batchConcurrency: number // default 3
```

### 组件位置

- `ui/src/components/BatchTranslateBar.tsx` — 新文件
- `ui/src/components/TranslationSummaryModal.tsx` — 新文件
- `ui/src/components/RecoveryPromptModal.tsx` — 新文件
- 现有 SidePanel/Toolbar — 集成 BatchTranslateBar

### IPC 调用

- `invoke('start_string_batch_translate', { ids, concurrency })` — 启动
- `invoke('cancel_string_batch_translate')` — 取消
- `invoke('check_pending_cache', { espHash })` — 启动时检查
- `listen('batch-string-progress', callback)` — 事件监听

## Acceptance Criteria

- [ ] 选中字符串后，"批量翻译"按钮可点击
- [ ] 并发滑块可调 (1-10)，默认 3
- [ ] 翻译运行时显示 "N/Total 已完成"
- [ ] 表格实时更新翻译文本
- [ ] 取消按钮停止新请求，已完成保留
- [ ] 完成后弹窗显示成功/失败汇总
- [ ] 启动时若有未应用缓存，弹出恢复提示
- [ ] 翻译期间 UI 可交互（滚动/筛选/排序）
