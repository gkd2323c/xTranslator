# xTranslator 代码注释补充记录

## 概述

已对 xTranslator 项目的关键路径补充了详细的中文注释，涵盖前后端核心模块。

## 补充范围

### 1. IPC 数据传输层 (`crates/xt-shared/src/dto.rs`)

**补充内容：**
- `QueryRequest` / `QueryResponse` - 虚拟滚动分页机制
- `SkyStringDTO` - 前端展示的字符串 DTO 结构
- `LoadEspResponse` / `EspLoadProgress` - ESP 加载流程和进度事件
- 各 DTO 字段的用途、约束和设计意图

**关键说明：**
- 强调了 `id` 的稳定性（用于前端定位）
- 解释了 `list_index` 的三值含义（.STRINGS / .DLSTRINGS / .ILSTRINGS）
- 说明了缓存机制（SHA-256 哈希）

---

### 2. Tauri 后端命令层 (`src-tauri/src/commands.rs`)

**补充内容：**
- `AppState` 结构 - 全局应用状态容器
- `EspFileInfo` - 已加载文件信息
- 辅助函数：`status_string()`, `sky_string_to_dto()`, `append_old_data_entries()`
- `load_esp()` 命令 - 核心 ESP 加载流程

**关键说明：**
- 解释了 Mutex 的使用（快速操作，不需要并发读）
- API Key 的内存存储策略（不持久化）
- ESP 模式与 Strings 文件模式的区别
- 缓存检查流程（mtime+size 快速路径 → SHA-256 完整路径）
- 阻塞线程池的使用（CPU 密集型任务）

---

### 3. 前端状态管理 (`ui/src/stores/appStore.ts`)

**补充内容：**
- `Theme` 类型 - 主题系统
- `ActivePanel` 类型 - 工具面板互斥管理
- `BottomTabId` 类型 - 底部标签页
- `AppState` 接口 - 全局前端状态

**关键说明：**
- 面板系统的单选互斥设计（替代 9 个布尔标志）
- `allItems` vs `items` 的区别（完整集 vs 显示集）
- 侧边栏统计基于 `allItems` 的设计
- 选择操作使用 `selectedId` 而非数组索引的原因

---

### 4. 核心库 - ESP 解析 (`crates/xt-core/src/esp/parser.rs`)

**补充内容：**
- `decompress_bethesda_record()` - Bethesda 压缩格式解析
- `TranslatableField` - 可翻译字段定义

**关键说明：**
- Bethesda 压缩格式的结构（4 字节大小头 + zlib 数据）
- 合理性检查（100MB 上限）
- record_defs 的作用（定义哪些字段可翻译）

---

### 5. 核心库 - 字符串数据结构 (`crates/xt-core/src/types/sky_string.rs`)

**补充内容：**
- `SkyString` 结构 - 核心字符串数据结构

**关键说明：**
- 与 Delphi 原版的对应关系
- 各字段的用途和生命周期
- 运行时 ID vs 持久化匹配的区别
- 启发式搜索相关字段（word_hashes, ld_result, ld_found）

---

### 6. 核心库 - 匹配算法 (`crates/xt-core/src/matching.rs`)

**补充内容：**
- T1-T4 分层匹配算法说明
- `DictionarySourceFormat` - 字典来源
- `DictionaryApplyEntry` - 统一字典条目表示

**关键说明：**
- 四层匹配的置信度递减
- Jaccard 相似度的计算和阈值
- 歧义匹配的处理策略

---

## 注释风格

所有补充的注释遵循以下原则：

1. **中文编写** - 与项目现有风格一致
2. **结构化** - 使用标题、列表、代码块等组织信息
3. **实用性** - 解释"是什么"和"为什么"，而非"怎么做"
4. **跨层级** - 说明组件间的交互和数据流
5. **约束说明** - 强调设计约束和使用限制

---

## 后续补充建议

### 高优先级
- [ ] `crates/xt-core/src/sst/v8.rs` - SST 字典格式
- [ ] `crates/xt-core/src/xml/mod.rs` - XML 导入/导出
- [ ] `ui/src/api/strings.ts` - 前端 IPC 包装函数
- [ ] `src-tauri/src/batch.rs` - 批处理状态机

### 中优先级
- [ ] `crates/xt-core/src/esp/record_tree.rs` - ESP 记录树（用于回写）
- [ ] `crates/xt-core/src/cache.rs` - 缓存机制
- [ ] `ui/src/components/StringTable.tsx` - 虚拟滚动表格
- [ ] `ui/src/components/EditorDialog.tsx` - 编辑对话框

### 低优先级
- [ ] 翻译 API 提供方实现
- [ ] 工具面板组件
- [ ] 底部标签页组件

---

## 验证方法

已补充的注释可通过以下方式验证：

```bash
# 检查 DTO 文件
cargo doc -p xt-shared --no-deps --open

# 检查后端命令
cargo doc -p xtranslator-tauri --no-deps --open

# 检查核心库
cargo doc -p xt-core --no-deps --open

# 前端注释检查（IDE 悬停提示）
cd ui && npx tsc --noEmit
```

---

## 更新日期

- **2026-05-13** - 初始补充（关键路径）
