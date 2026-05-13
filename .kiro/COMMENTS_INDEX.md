# xTranslator 代码注释索引

## 📋 文档导航

### 快速开始
- **[快速参考](./COMMENT_QUICK_REFERENCE.md)** - 核心概念、常见操作、设计模式
- **[补充记录](./COMMENT_IMPROVEMENTS.md)** - 详细的补充范围和后续建议

---

## 🎯 按模块查找

### IPC 数据传输层
**文件:** `crates/xt-shared/src/dto.rs`

**关键类型:**
- `QueryRequest` / `QueryResponse` - 虚拟滚动分页
- `SkyStringDTO` - 前端字符串 DTO
- `LoadEspResponse` - ESP 加载响应
- `EspLoadProgress` - 加载进度事件
- `LoadSstResponse` - SST 加载响应
- `BatchStatus` / `BatchProgress` - 批处理状态

**核心概念:**
- 字符串 ID 的稳定性
- Strings 文件类型索引（0/1/2）
- 缓存机制（SHA-256 哈希）

---

### 后端命令层
**文件:** `src-tauri/src/commands.rs`

**关键结构:**
- `AppState` - 全局应用状态
- `EspFileInfo` - 已加载文件信息

**关键函数:**
- `load_esp()` - ESP 文件加载（核心命令）
- `update_translation()` - 单字符串更新
- `batch_update_translations()` - 批量更新
- `load_sst()` - SST 字典加载
- `export_xml()` / `import_xml()` - XML 导入导出

**设计要点:**
- Mutex 的使用（快速操作）
- API Key 内存存储
- 阻塞线程池（CPU 密集型）
- 缓存检查流程

---

### 前端状态管理
**文件:** `ui/src/stores/appStore.ts`

**关键类型:**
- `Theme` - 主题系统
- `ActivePanel` - 工具面板（单选互斥）
- `BottomTabId` - 底部标签页
- `AppState` - 全局前端状态

**核心概念:**
- `allItems` vs `items` 的区别
- 面板系统的单选设计
- 选择操作使用 `selectedId`
- 侧边栏统计基于 `allItems`

---

### ESP 解析
**文件:** `crates/xt-core/src/esp/parser.rs`

**关键函数:**
- `decompress_bethesda_record()` - 压缩记录解压
- `EspParser::parse()` - ESP 二进制解析

**关键类型:**
- `TranslatableField` - 可翻译字段定义

**设计要点:**
- Bethesda 压缩格式（4 字节大小头 + zlib）
- record_defs 的作用
- 合理性检查（100MB 上限）

---

### 字符串数据结构
**文件:** `crates/xt-core/src/types/sky_string.rs`

**关键类型:**
- `SkyString` - 核心字符串数据结构

**字段分类:**
- 基本字段：`id`, `source`, `translation`
- 位置字段：`record_sig`, `field_sig`, `esp_ptr`
- 哈希字段：`hash`, `hash_trans`, `word_hashes`
- 状态字段：`params`, `internal_params`
- 搜索字段：`ld_result`, `ld_found`, `min_word`

**设计要点:**
- 与 Delphi 原版的对应关系
- 运行时 ID vs 持久化匹配
- 启发式搜索相关字段

---

### 匹配算法
**文件:** `crates/xt-core/src/matching.rs`

**关键函数:**
- `apply_dictionary_entries_with_policy()` - T1-T4 匹配

**关键类型:**
- `DictionarySourceFormat` - 字典来源（XML/SST）
- `DictionaryApplyEntry` - 统一字典条目

**T1-T4 分层匹配:**
- **T1** (str_id, record_sig, field_sig) - 精确匹配，非常高置信度
- **T2** (edid_hash, record_sig, field_sig) - EDID 匹配，高置信度
- **T3** (normalized_hash, record_sig, field_sig) - 规范化匹配，高置信度
- **T4** word_hashes Jaccard >= 0.5 - 词汇匹配，中等置信度

**设计要点:**
- 歧义匹配不自动应用
- Jaccard 相似度计算
- 置信度递减

---

### SST 字典格式
**文件:** `crates/xt-core/src/sst/v8.rs`

**关键类型:**
- `SstDictionary` - SST 字典结构
- `CachePayload` - 缓存载荷

**关键函数:**
- `load_from_file()` - 从文件读取 SST
- `save_to_file()` - 保存 SST 到文件
- `read_from()` - 解析 SST v8 格式

**文件格式（v8）:**
1. 魔数 (4 bytes): 0x39555353
2. v4 占位符 (1 byte)
3. Master List (v8+): 游戏主文件列表
4. Colab Label List (v7+): 协作标签
5. 字符串条目: 循环到 EOF

**设计要点:**
- 内容寻址缓存（SHA-256 哈希）
- 向后兼容 v6-v8 格式
- 版本管理和失效检查

---

### XML 导入/导出
**文件:** `crates/xt-core/src/xml/mod.rs`

**关键类型:**
- `XmlExportParams` - 导出参数
- `XmlStringEntry` - XML 字符串条目

**关键函数:**
- `parse_xml_file()` - 解析 XML 文件
- `write_xml_file()` - 写入 XML 文件
- `import_xml_to_sky_strings()` - 导入 XML 到字符串
- `sky_strings_to_xml_entries()` - 导出字符串到 XML

**XML 格式:**
```xml
<SSTXMLRessources>
  <Params>
    <Addon>插件名</Addon>
    <Source>源语言</Source>
    <Dest>目标语言</Dest>
    <Version>格式版本</Version>
  </Params>
  <Content>
    <String List="0" sID="000001">
      <EDID>Editor ID</EDID>
      <REC id="0" idMax="0">RECORD:FIELD</REC>
      <Source>源文本</Source>
      <Dest>翻译文本</Dest>
    </String>
  </Content>
</SSTXMLRessources>
```

**设计要点:**
- 通用交换格式
- T1-T4 匹配应用
- 与 Delphi 兼容

---

### 前端 IPC 包装
**文件:** `ui/src/api/strings.ts`

**关键接口:**
- `QueryRequest` / `QueryResponse` - 虚拟滚动分页
- `LoadEspResponse` - ESP 加载响应
- `LoadSstResponse` - SST 加载响应
- `HeuristicSearchRequest` / `HeuristicMatchDTO` - 启发式搜索
- `TranslateRequest` - 翻译请求

**关键函数:**
- `loadEsp()` - 加载 ESP 文件
- `loadSst()` - 加载 SST 字典
- `updateTranslation()` - 更新单个翻译
- `batchUpdateTranslations()` - 批量更新
- `heuristicSearch()` - 启发式搜索
- `translateString()` - 翻译单个字符串

**设计要点:**
- Tauri IPC 包装
- 类型安全（TypeScript）
- 与后端 DTO 同步

---

### 批处理状态机
**文件:** `src-tauri/src/batch.rs`

**关键类型:**
- `BatchExecutor` - 批处理器
- `BatchJobState` - 批处理状态

**状态机:**
- Idle → Running → Done → Idle

**关键函数:**
- `get_status()` - 获取当前状态
- `start_batch_job()` - 启动批处理
- `cancel_batch_job()` - 取消批处理

**设计要点:**
- 独立于 AppState
- 原子取消标志
- 实时进度事件

---

### ESP 记录树
**文件:** `crates/xt-core/src/esp/record_tree.rs`

**关键类型:**
- `EspField` - ESP 字段
- `EspRecord` - ESP 记录
- `EspGrup` - ESP 组
- `EspFile` - ESP 文件树

**关键函数:**
- `parse_fields()` - 解析字段
- `update_buffer()` - 更新字段缓冲区
- `write_to()` - 序列化字段

**设计要点:**
- XXXX 大小前缀处理
- 代码页编码/解码
- 用于 ESP 回写

---

### 缓存机制
**文件:** `crates/xt-core/src/cache.rs`

**关键类型:**
- `EsmCache` - 缓存管理器
- `CachePayload` - 缓存载荷

**关键函数:**
- `lookup()` - 查找缓存
- `store()` - 存储缓存
- `prune()` - 清理旧缓存

**缓存策略:**
- 内容寻址（SHA-256 哈希）
- LRU 清理（基于访问时间）
- 版本管理

**性能收益:**
- Skyrim.esm：2-5s → <100ms
- Update.esm：0.5-1s → <50ms

---

## 🔍 按概念查找

### 数据流
1. **ESP 加载流程** → `load_esp()` 命令
2. **SST 加载流程** → `load_sst()` 命令 + T1-T4 匹配
3. **翻译更新流程** → `update_translation()` 命令
4. **批量翻译流程** → `start_string_batch_translate()` 命令

### 缓存机制
- **内容寻址缓存** → `crates/xt-core/src/cache.rs`
- **缓存检查** → `load_esp()` 中的快速路径
- **缓存失效** → SHA-256 哈希变化

### 虚拟滚动
- **完整数据集** → `AppState.allItems`
- **显示集** → `AppState.items`（过滤+排序）
- **前端组件** → `ui/src/components/StringTable.tsx`

### 状态管理
- **全局状态** → `AppState` (Zustand store)
- **面板系统** → `activePanel`（单选互斥）
- **选择系统** → `selectedId`（稳定 ID）

### 翻译提供方
- **OpenAI** - 官方 API + 兼容服务
- **DeepL** - 专业翻译 API
- **百度翻译** - 中文优化
- **有道翻译** - 中文优化
- **Azure** - 企业级服务

---

## 📚 相关文档

### 项目文档
- `AGENTS.md` - 项目架构总览
- `ARCHITECTURE.md` - 详细架构文档
- `README.md` - 项目说明
- `RELEASE.md` - 发布说明

### 开发指南
- `docs/development_roadmap.md` - 开发路线图
- `docs/feature_comparison.md` - 功能对比（vs Delphi）
- `docs/README.md` - 文档维护规则

### 注释文档
- `.kiro/COMMENT_IMPROVEMENTS.md` - 补充范围详情
- `.kiro/COMMENT_QUICK_REFERENCE.md` - 快速参考

---

## 🚀 常见任务

### 我想了解 ESP 加载流程
1. 阅读 `load_esp()` 命令注释 → `src-tauri/src/commands.rs`
2. 查看 ESP 解析器 → `crates/xt-core/src/esp/parser.rs`
3. 参考快速参考中的"加载 ESP 文件"流程

### 我想了解翻译匹配
1. 阅读 T1-T4 匹配说明 → `crates/xt-core/src/matching.rs`
2. 查看 SST 加载流程 → `load_sst()` 命令
3. 参考快速参考中的"SST 字典加载"流程

### 我想了解前端状态管理
1. 阅读 `AppState` 接口 → `ui/src/stores/appStore.ts`
2. 查看虚拟滚动设计 → 快速参考中的"虚拟滚动 + 完整数据集"
3. 理解面板系统 → `ActivePanel` 类型

### 我想了解数据结构
1. 查看 `SkyString` → `crates/xt-core/src/types/sky_string.rs`
2. 查看 `SkyStringDTO` → `crates/xt-shared/src/dto.rs`
3. 理解 ID 系统 → 快速参考中的"关键 ID 系统"

---

## 📊 统计信息

### 补充范围
- **文件数** 12 个（第一批 6 个 + 第二批 6 个）
- **新增注释行数** ~600 行
- **覆盖的关键路径** 100%

### 文件清单

#### 第一批（关键路径）
| 文件 | 行数 | 注释行数 | 覆盖率 |
|------|------|---------|--------|
| `crates/xt-shared/src/dto.rs` | 900+ | 150+ | 高 |
| `src-tauri/src/commands.rs` | 4000+ | 80+ | 中 |
| `ui/src/stores/appStore.ts` | 500+ | 70+ | 高 |
| `crates/xt-core/src/esp/parser.rs` | 1000+ | 50+ | 中 |
| `crates/xt-core/src/types/sky_string.rs` | 200+ | 60+ | 高 |
| `crates/xt-core/src/matching.rs` | 500+ | 40+ | 中 |

#### 第二批（高优先级）
| 文件 | 行数 | 注释行数 | 覆盖率 |
|------|------|---------|--------|
| `crates/xt-core/src/sst/v8.rs` | 300+ | 80+ | 高 |
| `crates/xt-core/src/xml/mod.rs` | 400+ | 70+ | 高 |
| `ui/src/api/strings.ts` | 600+ | 100+ | 高 |
| `src-tauri/src/batch.rs` | 500+ | 60+ | 中 |
| `crates/xt-core/src/esp/record_tree.rs` | 300+ | 70+ | 高 |
| `crates/xt-core/src/cache.rs` | 400+ | 80+ | 高 |

---

## 🔄 后续计划

### 高优先级（已完成 ✓）
- [x] `crates/xt-core/src/sst/v8.rs` - SST 字典格式
- [x] `crates/xt-core/src/xml/mod.rs` - XML 导入/导出
- [x] `ui/src/api/strings.ts` - 前端 IPC 包装
- [x] `src-tauri/src/batch.rs` - 批处理状态机
- [x] `crates/xt-core/src/esp/record_tree.rs` - ESP 记录树
- [x] `crates/xt-core/src/cache.rs` - 缓存机制

### 中优先级（建议后续补充）
- [ ] `ui/src/components/StringTable.tsx` - 虚拟滚动表格
- [ ] `ui/src/components/EditorDialog.tsx` - 编辑对话框
- [ ] `crates/xt-core/src/translation_api/` - 翻译 API 提供方
- [ ] `crates/xt-core/src/heuristic/mod.rs` - 启发式搜索算法

### 低优先级（可选补充）
- [ ] 工具面板组件（BatchPanel, BsaBrowser 等）
- [ ] 底部标签页组件（VocabularyPanel, HeuristicPanel 等）
- [ ] 配置管理模块
- [ ] 日志系统

---

## 💡 使用建议

1. **新开发者** → 从快速参考开始，然后查看相关源文件
2. **代码审查** → 参考补充记录中的设计要点
3. **问题排查** → 使用索引快速定位相关代码
4. **性能优化** → 参考快速参考中的性能指标
5. **功能扩展** → 查看后续计划中的高优先级文件

---

**最后更新:** 2026-05-13（第二批补充完成）
