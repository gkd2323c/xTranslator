---
unit: 002-translation-cache
intent: 001-batch-translation
phase: inception
status: complete
created: 2026-05-01T12:00:00Z
updated: 2026-05-01T12:00:00Z
---

# Unit Brief: Translation Cache

## Purpose

独立翻译缓存文件管理。提供 append-only journal 格式的读写接口，以及应用启动时的崩溃恢复检测和恢复逻辑。

## Scope

### In Scope
- Append-only journal 文件格式设计：`(str_id, source_text, translated_text, timestamp)`
- 每条翻译结果立即追加写入（不等待批次完成）
- 启动时扫描缓存目录，检测未应用的翻译记录
- 恢复检测结果通过 IPC 传递给前端 → 弹出恢复提示
- 用户确认后：读取缓存 → 应用到 ESP 字符串 → 清除缓存文件
- 用户拒绝后：保留缓存文件，可稍后手动恢复
- 缓存文件存储在 `%LOCALAPPDATA%/xTranslator/translation_cache/`

### Out of Scope
- 翻译执行逻辑（translation-queue 负责）
- 恢复 UI 弹窗（batch-translation-ui 负责）
- ESP 字符串解析/操作（xt-core 负责）
- ESP 主缓存（EsmCache 独立管理）

---

## Assigned Requirements

| FR | Requirement | Priority |
|----|-------------|----------|
| FR-3 | 独立翻译缓存文件 | Must |
| FR-7 | 崩溃恢复 | Should |

---

## Domain Concepts

### Key Entities
| Entity | Description | Attributes |
|--------|-------------|------------|
| TranslationRecord | 单条翻译缓存记录 | str_id, source_hash, translated_text, timestamp |
| CacheFile | 翻译缓存文件（per ESP） | esp_hash, records: Vec<TranslationRecord>, is_applied: bool |
| RecoveryResult | 恢复检测结果 | esp_name, pending_count, cache_file_path |

### Key Operations
| Operation | Description | Inputs | Outputs |
|-----------|-------------|--------|---------|
| append_translation | 追加一条翻译记录到 journal | str_id, source, translated | () |
| detect_pending | 启动时扫描未应用的缓存 | esp_hash | Option<RecoveryResult> |
| apply_and_clear | 恢复缓存到 ESP 并清除文件 | esp_hash | Vec<TranslationRecord> |
| discard_cache | 删除缓存文件 | esp_hash | () |

---

## Dependencies

### Depended By
| Unit | Reason |
|------|--------|
| 001-translation-queue | 每条翻译完成后调用 append_translation |
| 003-batch-translation-ui | 启动时获取恢复提示数据 |

---

## Technical Context

### Suggested Technology
- Rust `serde_json` 或自定义二进制格式（每行一条 JSON record）
- 文件名格式：`{esp_sha256}_translation_cache.journal`
- 使用 `fs::OpenOptions` append 模式确保崩溃安全
- 启动时 `AppState::new()` 中调用 `detect_pending()`

### Data Storage
| Data | Type | Volume | Retention |
|------|------|--------|-----------|
| Journal records | Append-only file | 每条约 200 bytes，1000 条约 200KB | 应用后自动清除 |

### Integration Points
| Integration | Type | Protocol |
|-------------|------|----------|
| 001-translation-queue | Internal | Rust function call |
| 003-batch-translation-ui | IPC | Tauri command |

---

## Story Summary

| Metric | Count |
|--------|-------|
| Total Stories | 3 |
| Must Have | 1 |
| Should Have | 2 |
| Could Have | 0 |

### Stories

| Story ID | Title | Priority | Status |
|----------|-------|----------|--------|
| 001-journal-file-io | Journal file I/O | must | ✅ COMPLETED |
| 002-detect-pending-cache | Detect pending cache | should | ✅ COMPLETED |
| 003-apply-recovery | Apply recovery | should | ✅ COMPLETED |

---

## Success Criteria

### Functional
- [ ] 翻译到第 50 条时 kill 进程 → 重启后 journal 中有 50 条记录
- [ ] 恢复后 ESP 中 50 条翻译全部生效
- [ ] 拒绝恢复后缓存文件保留

### Non-Functional
- [ ] 写入操作不阻塞翻译流程（fs write < 1ms）
- [ ] 文件损坏时优雅降级（跳过损坏行，不丢失其余记录）
