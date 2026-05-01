---
stage: design
bolt: 002-translation-queue
created: 2026-05-01T12:00:00Z
---

# Technical Design: translation-queue

## Architecture Pattern

**Async Actor Model** — 翻译队列作为 Tauri AppState 中的独立 actor，通过 channel 通信。

```text
┌──────────────────────────────────────────────────────────┐
│  Tauri IPC Layer (commands.rs)                           │
│  start_batch_translate, cancel_batch_translate            │
├──────────────────────────────────────────────────────────┤
│  BatchTranslator (AppState managed)                      │
│  ┌──────────────────┐  ┌──────────────────────────┐     │
│  │ BatchQueue       │  │ BatchWorker (tokio task) │     │
│  │ jobs: Vec<Job>   │─▶│ ┌──────────────────┐     │     │
│  │ concurrency: u8  │  │ │ Semaphore(N)     │     │     │
│  │ cancel: watch::  │  │ │ translate_string │     │     │
│  │   Sender<bool>   │  │ │ → emit events    │     │     │
│  └──────────────────┘  │ └──────────────────┘     │     │
│                        └──────────────────────────┘     │
│                              │                          │
│                    ┌─────────┴─────────┐                │
│                    │ TranslationCache   │                │
│                    │ append_translation │                │
│                    └───────────────────┘                │
└──────────────────────────────────────────────────────────┘
```

## Layer Structure

### TranslationBatch (xt-core)
- **BatchConfig**: 并发数、provider、字符串 ID 列表
- **BatchState**: idle | running | completed | cancelled
- **BatchProgress**: completed / total 计数器

### BatchExecutor (src-tauri)
- **Actor pattern**: 通过 `Arc<Mutex<BatchState>>` + `tokio::spawn` 实现
- **Event emission**: `app_handle.emit("batch-translation-progress", ...)`
- Reuses existing `BatchExecutor` infrastructure (src-tauri/src/batch.rs)

## API Design (Tauri IPC)

### `start_batch_translate`
- **Request**: `{ ids: Vec<u32>, concurrency: u8 }`
- **Response**: `{ batch_id: String }`
- **Events emitted**:
  - `batch-translation-progress`: `{ completed, total, str_id, translated }`
  - `batch-translation-error`: `{ str_id, source, error }`
  - `batch-translation-complete`: `{ total, succeeded, failed, errors }`
  - `batch-translation-cancelled`: `{ completed, total }`

### `cancel_batch_translate`
- **Request**: `{}`
- **Response**: `()`

## Concurrency Model

```rust
// 使用 Semaphore 控制并发
let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency as usize));
let cancel_rx = watch::channel(false).1; // 取消信号

for job in jobs {
    let permit = semaphore.clone().acquire_owned().await?;
    
    // 检查取消信号
    if *cancel_rx.borrow() {
        break;
    }
    
    tokio::spawn(async move {
        let result = translate_with_retry(&job).await;
        let _permit = permit; // 释放信号量
        emit_progress(&handle, &result);
    });
}
```

## Retry Strategy

```rust
async fn translate_with_retry(source: &str) -> Result<String> {
    let mut delay = 1;
    for attempt in 0..3 {
        match translate_string(source).await {
            Ok(text) if !text.is_empty() => return Ok(text),
            Ok(_) => return Ok(String::new()), // empty response OK
            Err(e) if is_transient(&e) => {
                if attempt == 2 { return Err(e); }
                tokio::time::sleep(Duration::from_secs(delay)).await;
                delay *= 2; // 1s → 2s → 4s
            }
            Err(e) => return Err(e), // permanent error
        }
    }
    unreachable!()
}

fn is_transient(err: &str) -> bool {
    // timeout, rate limit, server errors
    err.contains("timeout") || err.contains("429") || err.contains("5")
}
```

## Data Model

### BatchJob State Machine

```
  ┌──────┐   start   ┌─────────┐
  │ idle │──────────▶│ running  │
  └──────┘           └────┬─────┘
                          │
              ┌───────────┼───────────┐
              ▼           │           ▼
        ┌───────────┐    │     ┌───────────┐
        │ completed │◄───┘     │ cancelled │
        └───────────┘          └───────────┘
```

## Integration Points

| Integration | Type | Protocol |
|-------------|------|----------|
| TranslationCache | Internal | `cache.append_translation()` after each job |
| OpenAI/DeepL API | External | HTTPS (existing `translation_api`) |
| AppState.strings | Internal | Write translation result after API success |
| Frontend (UI) | IPC | Tauri events (progress, complete, error) |
| Existing single translate | Internal | Coexists — uses same `translate_string` but independent state |

## NFR Implementation

| Requirement | Approach |
|-------------|----------|
| 非阻塞 | `tokio::spawn` 异步任务，不阻塞 Tauri 主线程 |
| UI 响应 < 100ms | 事件通过 Tauri `emit` 异步推送到前端 |
| 重试退避 | 指数退避 (1s/2s/4s)，瞬态错误才重试 |
| 取消安全 | `watch` channel 在每个 job 启动前检查 |

## Security Design

- API Key 从内存读取（`AppState`），不写入日志
- 超时设置: 30s（防止单个请求挂起整个队列）
- 并发上限 10（防止 API 滥用）
