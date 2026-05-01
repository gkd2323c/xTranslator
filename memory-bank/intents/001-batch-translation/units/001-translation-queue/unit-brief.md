---
unit: 001-translation-queue
intent: 001-batch-translation
phase: inception
status: complete
created: 2026-05-01T12:00:00Z
updated: 2026-05-01T12:00:00Z
---

# Unit Brief: Translation Queue

## Purpose

批量翻译核心引擎。管理翻译队列、并发控制、API 调用调度和重试逻辑。通过 Tauri IPC command 暴露给前端。

## Scope

### In Scope
- 接收选中字符串列表，建立翻译队列
- 并发控制（用户可调 1-10，默认 3）
- 调用现有 Provider 接口（OpenAI/DeepL）逐条翻译
- 错误重试（最多 3 次，指数退避 1s/2s/4s）
- 每条翻译完成后通过事件通知前端更新 UI
- 每条翻译完成后调用 translation-cache 写入缓存
- 取消信号处理（停止新请求，已完成保留）
- 保持现有单条翻译功能不变，与批量队列共存

### Out of Scope
- 缓存文件格式和 I/O（translation-cache 负责）
- 崩溃恢复 UI 逻辑（translation-cache 负责检测，UI 负责提示）
- 前端进度显示和滑块控件（batch-translation-ui 负责）
- 翻译记忆/词典匹配

---

## Assigned Requirements

| FR | Requirement | Priority |
|----|-------------|----------|
| FR-1 | 批量翻译 | Must |
| FR-5 | 并发控制 | Should |
| FR-6 | 错误处理与重试 | Must |

---

## Domain Concepts

### Key Entities
| Entity | Description | Attributes |
|--------|-------------|------------|
| TranslationJob | 单条翻译任务 | str_id, source_text, status (pending/in_progress/done/failed), result, retries |
| TranslationQueue | 翻译队列管理器 | jobs: Vec<TranslationJob>, concurrency: u32, provider: ProviderType, tx (channel sender) |
| BatchResult | 批量翻译结果汇总 | total, succeeded, failed, errors: Vec<(id, error)> |

### Key Operations
| Operation | Description | Inputs | Outputs |
|-----------|-------------|--------|---------|
| start_batch | 接收选中 ID 列表，建立队列并启动 | Vec<u32> string_ids, u32 concurrency | starts async processing |
| cancel_batch | 取消批量翻译 | - | stops new requests, returns BatchResult |
| get_progress | 获取当前进度 | - | (completed, total) |
| translate_single | 现有单条翻译（保持不变） | u32 string_id | translation result |

---

## Dependencies

### Depends On
| Unit | Reason |
|------|--------|
| 002-translation-cache | 每条翻译完成后写入缓存 |

### External Dependencies
| System | Purpose | Risk |
|--------|---------|------|
| OpenAI API | LLM 翻译服务 | High: 不可用或限流 |
| DeepL API | 专业翻译服务 | High: 不可用或限流 |
| xt-core::translation_api | 现有 Provider trait | Low (已存在) |

---

## Technical Context

### Suggested Technology
- Rust `tokio::sync::Semaphore` 控制并发
- Rust `tokio::sync::mpsc` channel 用于取消信号
- Tauri `emit` 事件通知前端每条翻译完成
- 使用现有 `translation_api::translate_string()` 方法

### Integration Points
| Integration | Type | Protocol |
|-------------|------|----------|
| 002-translation-cache | Internal | Rust function call |
| 003-batch-translation-ui | IPC | Tauri command + events |
| OpenAI/DeepL | External | HTTPS REST |

---

## Success Criteria

### Functional
- [ ] 选中 N 条 → 批量翻译 → N 条全部获得翻译结果（或标记为失败）
- [ ] 并发数 3 时，同时最多 3 个 API 请求
- [ ] 取消后不再发起新请求，已完成的结果保留

### Non-Functional
- [ ] 单条失败不阻塞队列
- [ ] 重试 3 次后标记失败并继续
- [ ] 现有单条翻译功能不受影响

---

## Story Summary

| Metric | Count |
|--------|-------|
| Total Stories | 4 |
| Must Have | 3 |
| Should Have | 1 |
| Could Have | 0 |

### Stories

| Story ID | Title | Priority | Status |
|----------|-------|----------|--------|
| 001-create-translation-queue | Create translation queue | must | ✅ GENERATED |
| 002-call-api-translate | Call API translate | must | ✅ GENERATED |
| 003-error-handling-retry | Error handling and retry | must | ✅ GENERATED |
| 004-cancel-and-progress | Cancel and progress | should | ✅ GENERATED |

---

## Bolt Suggestions

| Bolt | Type | Objective |
|------|------|-----------|
| bolt-translation-queue-1 | DDD | 翻译队列引擎 + 并发控制 |
