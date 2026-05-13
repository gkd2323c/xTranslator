# xTranslator 注释快速参考

## 核心概念速查

### 数据流路径

```
ESP 文件
  ↓ (load_esp)
EspParser → SkyString[] → AppState.strings
  ↓ (sky_string_to_dto)
SkyStringDTO[] → 前端 allItems
  ↓ (applyFilterAndSort)
items[] → react-window 虚拟滚动
```

### 关键 ID 系统

| ID 类型 | 范围 | 用途 | 持久化 |
|--------|------|------|--------|
| `SkyString.id` | u32 | 前端定位（排序/过滤后稳定） | ✗ 运行时 |
| `esp_ptr.str_id` | i32 | Strings 文件中的字符串索引 | ✓ SST/XML |
| `esp_ptr.form_id` | u32 | ESP 中的对象 ID | ✓ ESP |
| `selectedId` | u32 | 前端选中的字符串 | ✗ 会话 |

### 三层架构

```
前端 (React + Zustand)
  ↕ IPC (Tauri)
后端 (Rust + Tokio)
  ↕ 文件 I/O
游戏文件 (ESP/SST/XML)
```

---

## 文件导航

### 后端关键文件

| 文件 | 职责 | 关键函数 |
|------|------|---------|
| `src-tauri/src/commands.rs` | IPC 命令入口 | `load_esp()`, `update_translation()` |
| `crates/xt-core/src/esp/parser.rs` | ESP 二进制解析 | `EspParser::parse()` |
| `crates/xt-core/src/types/sky_string.rs` | 字符串数据结构 | `SkyString` |
| `crates/xt-core/src/matching.rs` | T1-T4 匹配算法 | `apply_dictionary_entries_with_policy()` |
| `crates/xt-core/src/sst/v8.rs` | SST 字典格式 | `SstDictionary::load()` |
| `crates/xt-core/src/cache.rs` | 内容寻址缓存 | `SqliteCache` |

### 前端关键文件

| 文件 | 职责 | 关键组件 |
|------|------|---------|
| `ui/src/stores/appStore.ts` | 全局状态管理 | `useAppStore()` |
| `ui/src/api/strings.ts` | IPC 包装函数 | `loadEsp()`, `updateTranslation()` |
| `ui/src/components/StringTable.tsx` | 虚拟滚动表格 | react-window v2 |
| `ui/src/components/EditorDialog.tsx` | 编辑对话框 | 单字符串编辑 |

### 共享文件

| 文件 | 职责 |
|------|------|
| `crates/xt-shared/src/dto.rs` | IPC 数据结构 |

---

## 常见操作流程

### 加载 ESP 文件

```rust
// 后端
load_esp(esp_path, strings_dir, language, game)
  → EspParser::parse() // 解析 ESP 二进制
  → load_strings_files() // 加载 .STRINGS 文件
  → build_esp_file() // 构建记录树（用于回写）
  → SqliteCache::store() // 缓存结果
  → LoadEspResponse { total, cached, esp_hash, ... }

// 前端
setAllItems(response.items)
setEspStats(response)
applyFilterAndSort() // 初始化显示集
```

### 更新单个翻译

```rust
// 前端
updateTranslation(id, newText)

// 后端
update_translation(id, newText)
  → 查找 AppState.strings[id]
  → 更新 translation 字段
  → 设置 is_dirty = true
  → 返回更新后的 SkyStringDTO

// 前端
updateItemTranslation(id, newText) // 本地更新，零 IPC
setIsDirty(true)
```

### SST 字典加载

```rust
// 后端
load_sst(sst_path)
  → SstDictionary::load() // 解析 SST 文件
  → apply_dictionary_entries_with_policy() // T1-T4 匹配
    → T1: (str_id, record_sig, field_sig) 精确匹配
    → T2: (edid_hash, record_sig, field_sig) EDID 匹配
    → T3: (normalized_hash, record_sig, field_sig) 规范化匹配
    → T4: word_hashes Jaccard >= 0.5 词汇匹配
  → 返回 LoadSstResponse { matched, tier_exact, tier_edid, ... }
```

### 批量翻译

```rust
// 前端
startStringBatchTranslate(selectedIds, provider, targetLang)

// 后端
start_string_batch_translate()
  → 创建 BatchQueue
  → 对每个 ID 调用 translate_single_with_retry()
  → 发送进度事件 "batch-progress"
  → 返回 BatchStatus

// 前端
监听 "batch-progress" 事件
更新 batchProgress, batchErrors
```

---

## 设计模式

### 1. 虚拟滚动 + 完整数据集

```typescript
// 后端返回完整数据集
allItems: SkyStringDTO[] // 10K+ 条

// 前端维护两个集合
allItems // 完整集（用于统计）
items    // 过滤+排序后的显示集（用于虚拟滚动）

// react-window 只渲染可见行
<List rowCount={items.length} rowHeight={32} />
```

### 2. 稳定 ID 定位

```typescript
// ✗ 错误：使用数组索引
updateTranslation(index, newText) // 排序后索引变化

// ✓ 正确：使用稳定 ID
updateTranslation(id, newText) // ID 不变
```

### 3. 分层匹配

```rust
// 优先级递减，置信度递减
if let Some(match) = tier1_exact_match() {
    apply(match) // 非常高置信度
} else if let Some(match) = tier2_edid_match() {
    apply(match) // 高置信度
} else if let Some(match) = tier3_normalized_match() {
    apply(match) // 高置信度
} else if let Some(matches) = tier4_vocab_matches() {
    if matches.len() == 1 {
        apply(matches[0]) // 中等置信度
    } else {
        // 歧义，不自动应用
    }
}
```

### 4. 缓存策略

```rust
// 内容寻址缓存（SHA-256）
let hash = hash_file(esp_path); // 计算文件哈希
if let Some(cached) = cache.lookup(&hash) {
    return cached; // 命中，直接返回
}
// 未命中，完整解析
let result = parse_esp(esp_path);
cache.store(&hash, &result); // 存储结果
```

---

## 常见问题

### Q: 为什么前端使用 `selectedId` 而非数组索引？

**A:** 因为用户可能应用过滤或排序，数组索引会变化。使用稳定的 `id` 字段确保选择不会因 UI 操作而丢失。

### Q: `list_index` 的三个值是什么？

**A:** 
- `0` = `.STRINGS` 文件（通常字符串）
- `1` = `.DLSTRINGS` 文件（对话字符串）
- `2` = `.ILSTRINGS` 文件（信息字符串）

### Q: 为什么 ESP 加载使用阻塞线程池？

**A:** ESP 解析是 CPU 密集型操作（二进制解析、zlib 解压），会阻塞异步运行时。使用 `tokio::task::spawn_blocking()` 避免卡住其他任务。

### Q: T1-T4 匹配中，为什么 T4 有歧义检查？

**A:** T4 是词汇重叠匹配，置信度最低。多个候选项意味着无法确定哪个是正确的，所以不自动应用，让用户手动选择。

### Q: 缓存何时失效？

**A:** 当 ESP 文件内容变化时（SHA-256 哈希不同）。修改时间或文件大小变化会触发重新计算哈希。

---

## 性能指标

| 操作 | 耗时 | 备注 |
|------|------|------|
| ESP 解析（首次） | 2-5s | 取决于文件大小和压缩记录数 |
| ESP 加载（缓存命中） | <100ms | 直接从 SQLite 读取 |
| SST 加载 | 1-2s | 包括 T1-T4 匹配 |
| 单字符串翻译 | 1-3s | 取决于 API 提供方 |
| 批量翻译（100 条） | 30-60s | 并发数可配置 |
| 虚拟滚动渲染 | <16ms | 仅渲染可见行 |

---

## 调试技巧

### 查看 ESP 解析进度

```typescript
// 前端监听进度事件
window.addEventListener('esp-load-progress', (event) => {
  console.log(event.detail); // { stage, current, total, percentage, message }
});
```

### 检查缓存状态

```rust
// 后端
let cached = response.cached; // true = 缓存命中
let esp_hash = response.esp_hash; // 文件哈希
```

### 查看匹配统计

```typescript
// 前端
const stats = store.sstStats;
console.log(`T1: ${stats.tier_exact}, T2: ${stats.tier_edid}, T3: ${stats.tier_normalized}, T4: ${stats.tier_vocab}`);
```

---

## 相关文档

- `AGENTS.md` - 项目架构总览
- `ARCHITECTURE.md` - 详细架构文档
- `docs/development_roadmap.md` - 开发路线图
- `.kiro/COMMENT_IMPROVEMENTS.md` - 注释补充记录
