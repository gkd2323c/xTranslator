---
stage: model
bolt: 002-translation-queue
created: 2026-05-01T12:00:00Z
---

# Static Model: translation-queue

## Entities

### TranslationJob
**Properties**: str_id (u32), source_text (String), status (JobStatus), result (Option<String>), retries (u8)
**Business Rules**:
- 初始状态为 `pending`
- retries 最大 3，超过后状态变为 `failed`
- result 仅在 status 为 `done` 时有值
- str_id 在当前队列中唯一（同一字符串不重复入队）

### BatchJob (Aggregate Root)
**Properties**: id (String), jobs (Vec<TranslationJob>), concurrency (u8), provider (ProviderType), state (BatchState)
**Business Rules**:
- concurrency 范围 [1, 10]，默认 3
- 同时进行的 API 调用数 ≤ concurrency
- state 流转: idle → running → completed | cancelled
- running 状态下不接受新的 jobs
- cancelled 后已完成 jobs 的结果保留

## Value Objects

### JobStatus
**Variants**: pending | in_progress | done | failed
**Constraints**: 状态只能向前推进，不可回退

### BatchState
**Variants**: idle | running | completed | cancelled
**Constraints**: idle → running → completed | cancelled

### BatchProgress
**Properties**: completed (u32), total (u32)
**Constraints**: completed ≤ total

### BatchError
**Properties**: str_id (u32), source_text (String), error_message (String), retries_exhausted (bool)
**Constraints**: retries_exhausted 为 true 表示已重试 3 次

## Aggregates

### BatchJob (Aggregate Root)
**Members**: TranslationJob (集合)
**Invariants**:
- 聚合内 TranslationJob 的 str_id 唯一
- state 为 running 时，最多 concurrency 个 job 同时处于 in_progress
- state 为 completed 时，所有 non-failed jobs 的状态为 done
- state 为 cancelled 时，pending jobs 停止调度，in_progress jobs 完成

## Domain Events

### BatchStarted
**Trigger**: 用户点击"批量翻译" → `start_batch()` 调用
**Payload**: { batch_id, total_jobs, concurrency }
**Consumers**: 前端进度显示初始化

### JobCompleted
**Trigger**: 单条翻译 API 返回成功
**Payload**: { str_id, translated_text, batch_progress }
**Consumers**: 前端表格更新、TranslationCache::append_translation()

### JobFailed
**Trigger**: 单条翻译 API 失败（含重试耗尽）
**Payload**: { str_id, error_message, retries_exhausted }
**Consumers**: 前端错误记录

### BatchProgressUpdated
**Trigger**: 每个 job 完成时
**Payload**: { completed, total }
**Consumers**: 前端进度条更新

### BatchCompleted
**Trigger**: 所有 jobs 完成（done + failed）
**Payload**: { total, succeeded, failed, errors: [BatchError] }
**Consumers**: 前端汇总弹窗

### BatchCancelled
**Trigger**: 用户点击取消 → 所有 in_progress jobs 完成
**Payload**: { completed, total, errors }
**Consumers**: 前端状态更新

## Domain Services

### BatchTranslationService
**Operations**:
- `start_batch(string_ids, concurrency, provider)` → 创建 BatchJob，启动并发翻译
- `cancel_batch()` → 发送取消信号，停止调度新 job
- `get_progress()` → 返回当前 BatchProgress
**Dependencies**:
- `translation_api::translate_string()` (现有 Provider trait)
- `TranslationCache::append_translation()` (001-translation-cache)
- `AppState.strings` (更新翻译结果)

## Repository Interfaces

### JobRepository (In-Memory)
**Entity**: TranslationJob
**Methods**:
- `enqueue(job)` — 加入队列
- `dequeue() -> Option<Job>` — 取出下一个 pending job
- `mark_done(id, result)` — 标记完成
- `mark_failed(id, error)` — 标记失败

## Ubiquitous Language

| 术语 | 定义 |
|------|------|
| 并发数 (Concurrency) | 同时发送的 API 请求数上限 |
| 信号量 (Semaphore) | 控制并发的 `tokio::sync::Semaphore` |
| 取消令牌 (Cancel Token) | `tokio::sync::watch` channel 用于广播取消信号 |
| 重试退避 (Backoff) | 指数退避策略：1s → 2s → 4s |
| Batch ID | 批量翻译的唯一标识 |
