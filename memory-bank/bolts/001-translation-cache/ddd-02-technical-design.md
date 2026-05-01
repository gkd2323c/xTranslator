---
stage: design
bolt: 001-translation-cache
created: 2026-05-01T12:00:00Z
---

# Technical Design: translation-cache

## Architecture Pattern

**Module Pattern** (not full Hexagonal — this is a single-module utility within xt-core)

The translation cache is a self-contained Rust module within `xt-core`, exposing a `TranslationCache` struct. No separate application/infrastructure layers needed — the module directly uses `std::fs` for file I/O and communicates with other modules via direct function calls.

Rationale: This is not a distributed service. It's an embedded utility in a desktop app. Full DDD layering would be over-engineered.

## Layer Structure

```text
┌─────────────────────────────────────────────┐
│  Tauri IPC Layer (commands.rs)              │
│  check_pending_cache, apply_translation_cache │
├─────────────────────────────────────────────┤
│  TranslationCache (xt-core)                  │
│  ┌─────────────┐  ┌────────────────────┐    │
│  │ CacheService │  │ JournalRepository  │    │
│  │ (business    │  │ (file I/O)         │    │
│  │  logic)      │  │                    │    │
│  └─────────────┘  └────────────────────┘    │
│                      │                      │
│               ┌──────┴──────┐               │
│               │  File System │               │
│               │  (%LOCALAPPDATA%)            │
│               └─────────────┘               │
└─────────────────────────────────────────────┘
```

**Responsibilities**:
- **Tauri IPC Layer**: 薄封装层，将 IPC command 参数转换为 TranslationCache 方法调用。不包含业务逻辑。
- **CacheService**: 协调 cache 操作的核心逻辑 — append、detect、apply、discard。调用 JournalRepository 进行 I/O。
- **JournalRepository**: 封装文件系统操作 — 打开/创建 journal 文件、追加行、读取全部行、删除文件。处理 JSONL 序列化/反序列化。

## API Design (Tauri IPC Commands)

### `check_pending_cache`
- **Direction**: 前端 → 后端 (invoke)
- **Method**: `#[tauri::command]`
- **Request**: `{ esp_hash: String }`
- **Response**: `Option<RecoveryResult>` — `{ esp_name: String, pending_count: u32, cache_file_path: String }` or null
- **Timing**: ESP 加载后、UI 初始化前调用

### `apply_translation_cache`
- **Direction**: 前端 → 后端 (invoke)
- **Method**: `#[tauri::command]`
- **Request**: `{ esp_hash: String }`
- **Response**: `{ applied_count: u32 }`
- **Side effect**: 更新 `AppState.strings` 中对应 str_id 的 translation 字段，删除 journal 文件，触发前端数据刷新

### `discard_translation_cache`
- **Direction**: 前端 → 后端 (invoke)
- **Method**: `#[tauri::command]`
- **Request**: `{ esp_hash: String }`
- **Response**: `()` (void)
- **Side effect**: 仅删除 journal 文件，不修改任何 ESP 数据

## Data Model

### Journal File Format (JSONL)

```
{"str_id":12345,"source":"Iron Sword","translated":"铁剑","timestamp":"2026-05-01T12:00:00Z"}
{"str_id":12346,"source":"Steel Armor","translated":"钢甲","timestamp":"2026-05-01T12:00:01Z"}
```

**文件路径**: `{cache_dir}/xTranslator/translation_cache/{esp_hash}.journal`

### In-Memory Data Structures

```rust
struct TranslationCache {
    base_dir: PathBuf,  // %LOCALAPPDATA%/xTranslator/translation_cache
}

struct TranslationRecord {
    str_id: u32,
    source_text: String,
    translated_text: String,
    timestamp: DateTime<Utc>,
}

struct RecoveryResult {
    esp_name: String,
    pending_count: u32,
    cache_file_path: String,
}
```

## Security Design

| Concern | Approach |
|---------|----------|
| 路径遍历 | esp_hash 经过 SHA-256 验证（64 hex），不会包含路径分隔符 |
| 大文件 | 单行约 200 bytes，1000 条约 200KB — 无需限制（合理范围内） |
| 竞态条件 | 批量翻译写入通过 `Mutex<TranslationCache>` 序列化，确保原子追加 |
| 损坏恢复 | JSONL 逐行解析，损坏行跳过不丢失其余数据 |

## NFR Implementation

| Requirement | Design Approach |
|-------------|-----------------|
| 写入不阻塞翻译 | `append_translation` 使用 `fs::write` (同步) — 单行写入 < 1ms，不显著影响吞吐 |
| 中断恢复 | append + flush 确保每条记录完整落地；JSONL 格式保证部分写入不破坏已有数据 |
| 检测性能 | `detect_pending` 读取 journal 文件全部行（最多数千行），O(n) 扫描 < 100ms |
| 恢复性能 | `apply_and_clear` 逐条更新 ESP 字符串 + 删除文件，O(n) 操作 |

## Integration Points

| Integration | Type | Protocol |
|-------------|------|----------|
| 002-translation-queue | Internal | Rust fn: `TranslationCache::append()` |
| 003-batch-translation-ui | IPC | Tauri commands: check, apply, discard |
| xt-core::AppState | Internal | 读取/写入 `AppState.strings` (恢复时) |
| std::fs | System | 文件 I/O (读写 journal) |
