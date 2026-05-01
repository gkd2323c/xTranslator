---
intent: 001-batch-translation
phase: inception
status: units-decomposed
updated: 2026-05-01T12:00:00Z
---

# 批量翻译 - Unit Decomposition

## Units Overview

3 个 units，2 个后端 (DDD) + 1 个前端 (Simple):

### Unit 1: translation-queue
**Description**: 批量翻译引擎 — 管理翻译队列、并发控制、API 调用、重试逻辑

**Stories**: ~4
**Deliverables**: Rust `TranslationQueue` struct, `BatchTranslator` service, Tauri IPC commands

**Dependencies**: Depends on `translation-cache` (写入缓存)
**Type**: backend (ddd-construction-bolt)

### Unit 2: translation-cache
**Description**: 独立翻译缓存文件 — append-only journal 写入、崩溃恢复、恢复提示逻辑

**Stories**: ~3
**Deliverables**: Rust `TranslationCache` struct, journal read/write, recovery detection

**Dependencies**: None (foundation)
**Type**: backend (ddd-construction-bolt)

### Unit 3: batch-translation-ui
**Description**: 前端 UI — 批量翻译触发、进度显示、取消按钮、并发滑块、错误汇总弹窗

**Stories**: ~4
**Deliverables**: React components (BatchTranslateBar, ProgressPanel, etc.), Zustand store extensions

**Dependencies**: Depends on `translation-queue`, `translation-cache`
**Type**: frontend (simple-construction-bolt)

## Unit Dependency Graph

```text
[002-translation-cache] ──► [001-translation-queue] ──► [003-batch-translation-ui]
                                   │
                            (uses cache to write results)
```

## Requirement-to-Unit Mapping

| FR | Requirement | Unit |
|----|-------------|------|
| FR-1 | 批量翻译 | 001-translation-queue |
| FR-2 | 非阻塞 UI | 003-batch-translation-ui |
| FR-3 | 独立翻译缓存文件 | 002-translation-cache |
| FR-4 | 翻译进度与取消 | 003-batch-translation-ui |
| FR-5 | 并发控制 | 001-translation-queue |
| FR-6 | 错误处理与重试 | 001-translation-queue |
| FR-7 | 崩溃恢复 | 002-translation-cache |
| FR-8 | 单条手动翻译保持 | (现有功能，不改动) |

## Execution Order

1. **002-translation-cache** (foundation, no dependencies)
2. **001-translation-queue** (depends on cache)
3. **003-batch-translation-ui** (depends on queue + cache)
