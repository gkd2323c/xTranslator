# 今日工作计划 — 2026-04-27

## 1. 清理编译器 warnings ✅

> 目标：0 warnings across project (Rust + TS)

| 修复 | 文件 |
|------|------|
| 移除 6 个 unused imports (SeekFrom, BsaFileRecord, Read, Seek, anyhow, Serialize, async_trait) | bsa/header.rs, bsa/mod.rs, deepl.rs |
| 移除 1 个 unused variable (updated_ids → _updated_ids) | xml/mod.rs |
| 移除 1 个 unused variable (folder → _folder) | bsa/mod.rs |
| Suppress 3 个 dead code methods (parse_top_level, parse_record, parse_record_fields) | esp/parser.rs |
| Suppress 1 个 dead field (detected_source_language) | deepl.rs |
| Suppress batch.rs warnings (BatchJobState fields, is_idle) | batch.rs |
| 移除 test 中 unused `use std::env` | deepl.rs (test) |

## 2. Auto-backup ✅

> 目标：每 5 分钟自动备份 SST 到 backups/ 目录

| 步骤 | 文件 |
|------|------|
| DTO: AutoBackupRequest, AutoBackupResponse | crates/xt-shared/src/dto.rs |
| Backend command: auto_backup_sst (build SST → save → rotate) | src-tauri/src/commands.rs |
| Register command + add to handle! | src-tauri/src/main.rs |
| Frontend types + invoke wrapper | ui/src/api/strings.ts |
| 5-min timer in App.tsx (useEffect + setInterval) | ui/src/App.tsx |

## 3. Undo/Redo ✅

> 目标：撤销/重做编辑操作，Ctrl+Z/Y

| 步骤 | 文件 |
|------|------|
| UndoEntry type + MAX_UNDO_STACK = 100 | ui/src/stores/appStore.ts |
| undoStack / redoStack in state | ui/src/stores/appStore.ts |
| updateItemTranslation records undo before mutation | ui/src/stores/appStore.ts |
| undo() / redo() actions: swap stacks, invoke update_translation, apply locally | ui/src/stores/appStore.ts |
| Ctrl+Z / Ctrl+Y / Ctrl+Shift+Z shortcuts | ui/src/App.tsx |
| Clear stacks on reset | ui/src/stores/appStore.ts |

## 统计

- 修改文件：13 个
- 新增 DTO: 2 (AutoBackupRequest, AutoBackupResponse)
- 新增 IPC 命令: 1 (auto_backup_sst)
- Rust tests: 77/77 passing
- Warnings: 0 (Rust) / 0 (TypeScript)
