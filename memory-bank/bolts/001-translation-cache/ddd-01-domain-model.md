---
stage: model
bolt: 001-translation-cache
created: 2026-05-01T12:00:00Z
---

# Static Model: translation-cache

## Entities

### TranslationRecord
**Properties**: str_id (u32), source_text (String), translated_text (String), timestamp (DateTime)
**Business Rules**:
- str_id 必须在当前 ESP 中存在（写入时校验）
- translated_text 不能为空（空翻译不写入缓存）
- timestamp 为写入时刻的 UTC 时间，单调递增
- 两条 TranslationRecord 的等价性基于 str_id 相等（同一字符串只保留最新翻译）

### JournalCacheFile
**Properties**: esp_hash (String, SHA-256), file_path (PathBuf), records (Vec<TranslationRecord>), is_applied (bool)
**Business Rules**:
- 文件名格式：`{esp_hash}_translation_cache.journal`
- 文件位置：`%LOCALAPPDATA%/xTranslator/translation_cache/`
- 每个 ESP 最多 1 个 journal 文件（由 esp_hash 唯一确定）
- is_applied 为 false 表示文件中有未应用到 ESP 的记录
- 文件以 append-only 模式写入，不支持修改或删除单条记录

## Value Objects

### EspHash
**Properties**: hash (String, 64 hex chars)
**Constraints**: 必须是有效的 SHA-256 十六进制字符串，长度固定 64
**Equality**: 按 hash 值比较，大小写不敏感

### RecoveryResult
**Properties**: esp_name (String), pending_count (u32), cache_file_path (PathBuf)
**Constraints**: pending_count > 0（如果为 0 则返回 None，不创建 RecoveryResult）
**Equality**: 按 esp_hash + file_path 比较

## Aggregates

### TranslationJournal
**Aggregate Root**: JournalCacheFile
**Members**: TranslationRecord (集合)
**Invariants**:
- 聚合内所有 TranslationRecord 的 str_id 必须唯一（通过最新记录覆盖旧记录确保）
- JournalCacheFile.records 为空时 is_applied 必须为 true
- 文件存在但 records 为空 → 无效状态 → 删除文件
- 写入操作必须原子性：先写盘 flush 再更新内存 records

## Domain Events

### TranslationCached
**Trigger**: 单条翻译完成后调用 `append_translation()`
**Payload**: { str_id, source_text, translated_text, esp_hash }
**Consumers**: 无（纯持久化操作）

### PendingCacheDetected
**Trigger**: ESP 加载时调用 `detect_pending()`，扫描到未应用的 journal 文件
**Payload**: { esp_name, pending_count, cache_file_path }
**Consumers**: 前端 RecoveryPromptModal（提示用户恢复）

### CacheRecovered
**Trigger**: 用户确认恢复，调用 `apply_and_clear()` 完成
**Payload**: { esp_hash, applied_count }
**Consumers**: ESP 字符串列表刷新，前端进度通知

### CacheDiscarded
**Trigger**: 用户拒绝恢复，或显式调用 `discard_cache()`
**Payload**: { esp_hash, cache_file_path }
**Consumers**: 无

## Domain Services

### TranslationCacheService
**Operations**:
- `append_translation(str_id, source, translated, esp_hash)` → 追加单条翻译到 journal
- `detect_pending(esp_hash)` → 扫描并返回未应用的翻译记录
- `apply_and_clear(esp_hash)` → 将缓存中的翻译应用到 ESP 字符串并删除 journal
- `discard_cache(esp_hash)` → 删除 journal 文件（不恢复）
**Dependencies**:
- 文件系统（`std::fs`）
- ESP 字符串列表（通过 `AppState.strings` 访问）

## Repository Interfaces

### JournalFileRepository
**Entity**: JournalCacheFile
**Methods**:
- `open_or_create(esp_hash: &str) -> Result<JournalCacheFile>` — 打开已有 journal 或创建新文件
- `append_record(file: &mut JournalCacheFile, record: TranslationRecord) -> Result<()>` — 追加一条记录
- `read_all(file: &JournalCacheFile) -> Result<Vec<TranslationRecord>>` — 读取所有记录
- `delete(file: &JournalCacheFile) -> Result<()>` — 删除 journal 文件

## Ubiquitous Language

| 术语 | 定义 |
|------|------|
| Journal | append-only 翻译缓存文件，每条翻译结果追加到文件末尾 |
| Journal Record | journal 中的一行 JSON，包含 str_id、源文本、翻译结果、时间戳 |
| Pending Cache | journal 文件中有记录但尚未应用到 ESP 的状态 |
| Recovery | 将 pending cache 中的翻译应用到 ESP 字符串并删除 journal |
| esp_hash | ESP 文件的 SHA-256 哈希值，用于唯一标识和关联缓存文件 |
| Applied | journal 中的所有记录已写入 ESP，文件可安全删除 |
