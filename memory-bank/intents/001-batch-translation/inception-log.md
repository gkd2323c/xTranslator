---
intent: 001-batch-translation
created: 2026-05-01T12:00:00Z
completed: 2026-05-01T12:00:00Z
status: complete
---

# Inception Log: 批量翻译

## Overview

**Intent**: 为 xTranslator 添加批量 AI 翻译能力，异步非阻塞执行，实时缓存防丢失
**Type**: green-field
**Created**: 2026-05-01T12:00:00Z

## Artifacts Created

| Artifact | Status | File |
|----------|--------|------|
| Requirements | ✅ | requirements.md |
| System Context | ✅ | system-context.md |
| Units | ✅ | units/001-translation-queue/unit-brief.md, units/002-translation-cache/unit-brief.md, units/003-batch-translation-ui/unit-brief.md |
| Stories | ✅ | 11 stories across 3 units |
| Bolt Plan | ✅ | memory-bank/bolts/001-translation-cache/bolt.md, memory-bank/bolts/002-translation-queue/bolt.md, memory-bank/bolts/003-batch-translation-ui/bolt.md |

## Summary

| Metric | Count |
|--------|-------|
| Functional Requirements | 8 |
| Non-Functional Requirements | 2 |
| Units | 3 |
| Stories | 11 |
| Bolts Planned | 3 |

## Units Breakdown

| Unit | Stories | Bolts | Priority |
|------|---------|-------|----------|
| 001-translation-queue | 4 | 1 | Must |
| 002-translation-cache | 3 | 1 | Must |
| 003-batch-translation-ui | 4 | 1 | Must |

## Decision Log

| Date | Decision | Rationale | Approved |
|------|----------|-----------|----------|
| 2026-05-01 | 独立翻译缓存文件（非 ESP cache） | 隔离翻译进度与 ESP 解析缓存 | Yes |
| 2026-05-01 | append-only journal 格式 | 逐条实时写入，崩溃安全 | Yes |
| 2026-05-01 | 用户可调并发 1-10，默认 3 | 平衡速度与 API 限流 | Yes |
| 2026-05-01 | 失败跳过不阻塞 + 重试 3 次 | 保证翻译流程连续 | Yes |
| 2026-05-01 | 3 个 units: cache → queue → ui | 依赖顺序，cache 是基础 | Yes |
| 2026-05-01 | 每个 unit 1 个 bolt | 故事内聚，复杂度可在一个 bolt 完成 | Yes |

## Ready for Construction

**Checklist**:
- [x] All requirements documented
- [x] System context defined
- [x] Units decomposed
- [x] Stories created for all units
- [x] Bolts planned
- [x] Human review complete

## Next Steps

1. Start Construction Phase
2. Start with Unit: 002-translation-cache
3. Execute: `/specsmd-construction-agent --unit="002-translation-cache" --bolt-id="001-translation-cache"`

## Summary

| Metric | Count |
|--------|-------|
| Functional Requirements | 6 |
| Non-Functional Requirements | 2 |
| Units | 0 |
| Stories | 0 |
| Bolts Planned | 0 |

## Decision Log

| Date | Decision | Rationale | Approved |
|------|----------|-----------|----------|

## Scope Changes

| Date | Change | Reason | Impact |
|------|--------|--------|--------|
