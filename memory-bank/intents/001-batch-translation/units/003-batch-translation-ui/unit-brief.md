---
unit: 003-batch-translation-ui
intent: 001-batch-translation
phase: inception
status: complete
created: 2026-05-01T12:00:00Z
updated: 2026-05-01T12:00:00Z
unit_type: frontend
default_bolt_type: simple-construction-bolt
---

# Unit Brief: Batch Translation UI

## Purpose

批量翻译的前端界面。提供翻译触发控件、进度显示、并发滑块、取消按钮、错误汇总和崩溃恢复提示。

## Scope

### In Scope
- 工具栏增加"批量翻译"按钮 + 并发滑块（1-10，默认 3）
- 点击触发 → 调用 `start_batch` IPC command
- 监听 Tauri events 实时更新进度（"12/50 已完成"）
- 取消按钮 → 调用 `cancel_batch` IPC command
- 翻译完成/取消后显示汇总弹窗（成功 N 条，失败 M 条 + 错误详情）
- 启动时调用 `check_pending_cache` IPC → 如有未应用翻译，弹窗提示恢复
- 单条手动翻译按钮保持不变，可与批量翻译同时操作
- 表格中实时高亮翻译完成的条目

### Out of Scope
- 翻译队列逻辑（translation-queue 负责）
- 缓存文件 I/O（translation-cache 负责）
- Provider/API Key 配置（现有设置页）
- 翻译记忆/词典 UI

---

## Assigned Requirements

| FR | Requirement | Priority |
|----|-------------|----------|
| FR-2 | 非阻塞 UI | Must |
| FR-4 | 翻译进度与取消 | Should |

---

## Key Components

| Component | Description |
|-----------|-------------|
| BatchTranslateBar | 工具栏：并发滑块 + 批量翻译按钮 + 取消按钮 |
| ProgressIndicator | 进度条/计数器，显示 "12/50 已完成" |
| TranslationSummaryModal | 翻译完成后的汇总弹窗（成功/失败/错误） |
| RecoveryPromptModal | 启动时检测到未应用翻译的恢复提示弹窗 |

## Zustand Store Extensions

```
batchState: idle | running | completed | cancelled
batchProgress: { completed: number, total: number }
batchErrors: Array<{ id: number, source: string, error: string }>
batchConcurrency: number (default 3)
```

## Dependencies

### Depends On
| Unit | Reason |
|------|--------|
| 001-translation-queue | IPC commands: start_batch, cancel_batch, translate_single |
| 002-translation-cache | IPC command: check_pending_cache |

---

## Technical Context

### Suggested Technology
- React functional components
- Zustand for batch state management
- Tauri `invoke()` for IPC, `listen()` for events
- react-hot-toast for notifications
- lucide-react for icons (Play, Square, RotateCcw)

### Integration Points
| Integration | Type | Protocol |
|-------------|------|----------|
| 001-translation-queue | IPC | Tauri command + events |
| 002-translation-cache | IPC | Tauri command |
| appStore (existing) | Internal | Zustand |

---

## Story Summary

| Metric | Count |
|--------|-------|
| Total Stories | 4 |
| Must Have | 1 |
| Should Have | 3 |
| Could Have | 0 |

### Stories

| Story ID | Title | Priority | Status |
|----------|-------|----------|--------|
| 001-batch-control-bar | Batch control bar | must | ✅ GENERATED |
| 002-live-progress-display | Live progress display | should | ✅ GENERATED |
| 003-cancel-translation | Cancel translation | should | ✅ GENERATED |
| 004-recovery-prompt | Recovery prompt | should | ✅ GENERATED |

---

## Success Criteria

### Functional
- [ ] 选中 10 条 → 设置并发 2 → 点击批量翻译 → 进度实时显示 → 10 条全部翻译完成
- [ ] 翻译过程中可以滚动表格、编辑其他条目
- [ ] 点击取消 → 进度停止 → 已完成的不丢失

### Non-Functional
- [ ] 翻译期间 UI 交互延迟 < 100ms
- [ ] 进度更新不触发不必要的重渲染（Zustand selector 优化）
