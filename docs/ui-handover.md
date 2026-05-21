# xTranslator UI 打磨交接文档

> 编写日期：2026-05-20
> 最后更新：2026-05
> 涉及：Phase 1~5 UI 打磨及后续补强，前端从 ~55% 提升至 ~90%
> 核心文件：EditorPanel, StringTable, StatusBar, ContextMenu, DialogView, BsaBrowser, BatchPanel, PexPanel, SidePanel, LogPanel, VocabularyPanel
> 新增 Phase 4：SpellCheckSettingsDialog, MergeSstDialog

---

## 一、总览

三轮 UI 打磨（Phase 1~3）覆盖了 15 个组件，累计 ~2,400 行新增 / ~350 行删除。

| Phase | 范围 | 新增行数 | 前端完成度 |
|-------|------|----------|-----------|
| **Phase 1** | EditorPanel + StringTable 核心增强 | ~500 | 55%→65% |
| **Phase 2** | StatusBar + ContextMenu + 底部面板 + DialogView | ~1,000 | 65%→72% |
| **Phase 3** | BSA + BatchPanel + PEX 面板增强 | ~900 | 72%→75% |
| **Phase 4** | SpellCheckSettingsDialog + MergeSstDialog + 配置持久化 | ~500 | 75%→82% |
| **Phase 5** | McmPanel + EspComparePanel + FuzPanel 三个缺口面板增强 | ~600 | 82%→90% |
| **合计** | 20 个组件 | ~3,500 | 55%→90% |

---

## 二、组件完成度一览

### 2.1 核心编辑区

| 组件 | 文件 | 完成度 | Phase | 关键功能 |
|------|------|--------|-------|----------|
| **StringTable** | `ui/src/components/StringTable.tsx` | ~80% | 1B | 虚拟滚动(76K+)、多选(Ctrl/Shift+Click)、键盘导航(↑↓/PgUp/PgDn/Home/End)、右键多选感知 |
| **EditorPanel** | `ui/src/components/EditorPanel.tsx` | ~78% | 1A | 弹窗编辑器、元数据行(FormID/Rec/Field/Type/Size)、语法高亮(XML/$/{})、Tab 插空格、原文复制 |
| **ContextMenu** | `ui/src/components/StringTable.tsx` (内联) | ~85% | 2A | 多选感知批量操作(Copy Sources/Copy Translations/Translate)、单项编辑/复制/过滤 |
| **StatusBar** | `ui/src/components/StatusBar.tsx` | ~85% | 2A | 选择位置 N/M、多选计数、可视化进度条+百分比、过滤提示 |

### 2.2 工具面板

| 组件 | 文件 | 完成度 | Phase | 关键功能 |
|------|------|--------|-------|----------|
| **BSA Browser** | `ui/src/components/BsaBrowser.tsx` | ~70% | 3A | 元数据预览面板、文件名搜索+高亮、文件夹展开/折叠、提取 |
| **BatchPanel** | `ui/src/components/BatchPanel.tsx` | ~80% | 3B | 多文件批处理、实时进度、逐文件错误详情、统计卡片+重试失败 |
| **PEX Panel** | `ui/src/components/PexPanel.tsx` | ~65% | 3C | 字符串搜索+表格化、类型过滤、伪代码视图(行号+复制)、反编译 |
| **DialogView** | `ui/src/components/DialogView.tsx` | ~78% | 2C | NPC→DIAL→INFO 树、展开/折叠全部、搜索过滤+高亮、翻译状态色标 |

### 2.3 底部面板

| 组件 | 文件 | 完成度 | Phase | 关键功能 |
|------|------|--------|-------|----------|
| **Home (SidePanel)** | `ui/src/components/SidePanel.tsx` | ~95% | 2B | 状态概览四卡片、大号进度条+百分比、文件/统计/ESP 头部信息 |
| **Vocabulary** | `ui/src/components/bottom/VocabularyPanel.tsx` | ~70% | 2B | 加载词汇统计、可搜索词汇预览(200条)、source↔translation 对照 |
| **Log** | `ui/src/components/bottom/LogPanel.tsx` | ~80% | 2B | 日志级别着色(INFO/WARN/ERROR)、搜索过滤、自动滚动、复制/清空 |

### 2.4 Phase 4 新增组件

| 组件 | 文件 | 完成度 | 关键功能 |
|------|------|--------|----------|
| **SpellCheckSettingsDialog** | `ui/src/components/SpellCheckSettingsDialog.tsx` | ~95% | 字典选择/扫描、加载/卸载、启用/禁用切换、配置持久化（dictionary/active/loaded 自动恢复）、MenuBar 启动恢复 |
| **MergeSstDialog** | `ui/src/components/MergeSstDialog.tsx` | ~95% | 来源 SST 文件选择（Tauri dialog）、overwrite 策略切换、合并执行、统计结果表格（added/updated/overwritten/skipped） |

### 2.5 Phase 5 新增 — 三个缺口面板增强

| 组件 | 文件 | 完成度 | 新增功能 |
|------|------|--------|----------|
| **McmPanel** | `ui/src/components/McmPanel.tsx` | ~92% | 翻译状态徽章（🟢已翻译/🟠部分翻译/⚪未翻译）、统计明细（三类计数）、批量操作（Copy sources/Clear all/Reverse）、比较结果对话框（diff 列表 + 统计卡片） |
| **EspComparePanel** | `ui/src/components/EspComparePanel.tsx` | ~80% | 差异报告导出（新增 `write_text_file` Rust 命令）、字符级 diff 高亮（红色删除线/绿色新增）、排序（ID/Field）、摘要统计条 |
| **FuzPanel** | `ui/src/components/FuzPanel.tsx` | ~85% | 播放进度条（`requestAnimationFrame` 实时更新）、排序（ID/Duration/Status）、统计可视化（内嵌进度条）、行可读性改进（文件名/播放高亮） |

### 2.6 低优先级打磨

| 组件 | 文件 | 完成度 | 备注 |
|------|------|--------|------|
| 主窗口布局 | `ui/src/App.tsx` + `App.css` | ~80% | 三阶段布局复刻完成，EDID/ID/LD 列宽可调；仍缺列拖放排序 |

---

## 三、新增 Store API

### 3.1 多选系统 (`appStore.ts`)

```typescript
selectedIds: Set<number>       // 多选集合
toggleSelectId: (id) => void   // 切换单项选择
clearSelection: () => void     // 清空多选
```

**使用方式**：`selectedIds` 配合 `ctrlKey`/`shiftKey`/`metaKey` 事件修饰键。右键菜单通过 `selectedIds.has(item.id)` 判断是否展示批量操作。

### 3.2 日志系统 (`appStore.ts`)

```typescript
interface LogEntry {
  id: number;
  timestamp: Date;
  level: "info" | "warn" | "error";
  message: string;
  source?: string;
}

logs: LogEntry[]          // 最多 500 条（逆序，最新在前）
addLog: (level, msg, source?) => void
clearLogs: () => void
```

**使用方式**：`useAppStore((s) => s.addLog)("info", "File loaded", "esp")`

---

## 四、关键 UI 模式与约定

### 4.1 语法高亮函数

```typescript
function highlightTags(text: string): string
```

位于 `EditorPanel.tsx`。使用正则 `/(<\/?[A-Za-z][^>]*>)|(\$\w+(?:\.\w+)*)|(\{[^}]+\})/g` 分三种类型着色：
- 捕获组 1 → `.tag-highlight`（青色，XML 标签）
- 捕获组 2 → `.tag-variable`（紫色，$变量）
- 捕获组 3 → `.tag-placeholder`（橙色，{占位符}）

**添加新类型**：在正则中新增捕获组，在 `while` 循环中新增 `if (match[N])` 分支，在 CSS 中新增对应类。

### 4.2 搜索高亮函数

使用 `<mark>` 标签包裹匹配部分，CSS 统一为：
```css
mark {
  background: rgba(212, 160, 23, 0.2);
  color: var(--color-accent-amber);
  border-radius: 2px;
}
```

实现模式：`text.split(new RegExp((${escapedQuery}), "gi")).map(...)`，用于 DialogView、BsaBrowser、PexPanel。

### 4.3 多选事件处理

```typescript
// StringTable.tsx — handleSelect
if (e.ctrlKey || e.metaKey) {
  toggleSelectId(id);                           // 切换
} else if (e.shiftKey && lastClickedRef.current !== null) {
  // 范围选择：从 lastClickedRef.current 到 currentIndex
} else {
  clearSelection();
  setSelectedById(id);                          // 单选
}
```

### 4.4 键盘导航映射

| 按键 | StringTable | EditorPanel(textarea) |
|------|-------------|----------------------|
| ↑/↓ | 上/下行 | — |
| Shift+↑/↓ | 扩展选择 | — |
| PageUp/Down | ±20 行 | — |
| Home/End | 首/尾行 | — |
| Enter | 打开编辑器 | — |
| Tab | — | 插入 2 空格 |
| Ctrl+Enter | — | 保存并关闭 |

---

## 五、CSS 变量与主题

所有 UI 颜色使用 CSS 变量，4 种主题（Obsidian/Slate/Light/Auto）。

### 新增 CSS 类体系

| 前缀 | 用途 | 示例 |
|------|------|------|
| `.editor-meta-*` | 编辑器元数据行 | `.editor-meta-tag`, `.editor-meta-label` |
| `.editor-size-bar-*` | 字段大小进度条 | `.editor-size-bar-bg`, `.editor-size-bar-fill` |
| `.tag-variable`/`.tag-placeholder` | 语法高亮 | $变量(紫)/{占位符}(橙) |
| `.row-selected-multi` | 多选行高亮 | 青色左侧边框 |
| `.stats-card*` | 统计卡片 | `.stats-card-translated` 颜色变体 |
| `.log-level-*` | 日志级别 | `.log-level-warn`(黄)/`.log-level-error`(红) |
| `.dialog-group-count[data-status]` | 对话状态徽标 | `done`(绿)/`partial`(黄)/`none`(灰) |
| `.batch-stat-*` | 批处理统计卡片 | `.batch-stat-ok`(绿)/`.batch-stat-fail`(红) |
| `.bsa-preview-*` | BSA 预览面板 | `.bsa-preview-toolbar`, `.bsa-preview-content` |
| `.pex-pseudocode-*` | PEX 伪代码视图 | `.pex-pseudocode-line-num` 行号 |

---

## 六、验证清单

每次修改后端后验证：

```bash
# TypeScript 编译（前端错误）
cd ui && npx tsc --noEmit

# 前端构建
cd ui && npm run build

# Rust 后端（核心库单元测试）
cargo test -p xt-core --lib
```

新增语法高亮类型后需额外验证：
- `highlightTags` 正则是否匹配 new 类型
- CSS class 是否在 App.css 中存在
- 转义逻辑是否安全（`escapeHtml` 前置）

---

## 七、未解决问题与后续建议

### 7.1 需后端支持的功能

| 功能 | 当前状态 | 所需后端 IPC | 建议优先级 |
|------|----------|-------------|-----------|
| PEX 内联编辑写回 | 只读浏览 | `write_pex_strings` | P1 — 使 PEX 面板可交互 |
| BSA 文件内容预览 | 元数据预览 | `preview_bsa_file`(直接返回内容) | P2 — 现有提取+临时文件方案太绕 |
| StringTable 列拖放排序 | 未实现 | 纯前端（react-window 不原生支持） | P3 — 需改为 AG Grid 或手写实现 |

### 7.2 纯前端可继续打磨

| 功能 | 当前状态 | 预估行数 | 说明 |
|------|----------|----------|------|
| FUZ 面板 LIP 预览 | 状态可见，暂无预览 | ~100 | 唇形数据预览、时间轴或波形展示 |
| MCM 面板进一步增强 | 对照编辑已完成 | ~80 | 批量校验、快捷键、差异视图等深层交互 |
| 键盘快捷键统一管理 | 散落在组件中 | ~80 | 考虑用 `react-hotkeys-hook` 集中管理 |
| 底部 Tab 拖拽排序 | 未实现 | ~60 | 每个 Tab 可拖拽重排 |

### 7.3 已知边缘情况

- **超长字符串**: EditorPanel 源文本设定了 `max-height: 300px` + 滚动，但超过 ~5KB 的字符串可能导致编辑器渲染卡顿（textarea 本身无问题）
- **大词汇量 Vocabulary**: 当前限制预览 200 条，完整词汇表可能有数万条，需分页或虚拟滚动
- **日志数量**: `LogEntry[]` 限制 500 条上限，量产环境日志可能需增加 buffer
- **多选性能**: `selectedIds` 使用 `Set<number>`，一次 Shift 范围选择最多 toggle 全部 items(~76K)，理论上单次操作为 O(n)，建议分片成不超过 1000 条/批
- **BSA 压缩文件**: 提取大文件(>100MB)时无进度反馈，涉及 BA2 GNRL 的 block compression 需要额外注意

---

## 八、快速参考

```bash
# 日常开发
cd ui && npm run dev

# 类型检查
cd ui && npx tsc --noEmit

# 构建
cd ui && npm run build

# 源端测试
cargo test -p xt-core --lib

# 完整 UI 相关文件索引
# 核心: EditorPanel.tsx, StringTable.tsx, App.tsx
# Store: appStore.ts
# CSS: App.css (~3880 行)
# 底部面板: bottom/LogPanel.tsx, bottom/VocabularyPanel.tsx
# 工具面板: BsaBrowser.tsx, BatchPanel.tsx, PexPanel.tsx, DialogView.tsx
```

### 文件大小速查

| 文件 | 行数 | 说明 |
|------|------|------|
| `App.css` | ~3,880 | 所有 UI 样式 |
| `EditorPanel.tsx` | ~880 | 编辑器弹窗 |
| `StringTable.tsx` | ~720 | 字符串表格 |
| `BatchPanel.tsx` | ~665 | 批量处理 |
| `appStore.ts` | ~1,140 | 全局状态 |
| `BsaBrowser.tsx` | ~310 | BSA 浏览器 |
| `PexPanel.tsx` | ~330 | PEX 面板 |
| `DialogView.tsx` | ~220 | 对话视图 |
| `SidePanel.tsx` | ~230 | Home 面板 |
| `StatusBar.tsx` | ~120 | 状态栏 |
| `LogPanel.tsx` | ~115 | 日志面板 |
| `VocabularyPanel.tsx` | ~120 | 词汇面板 |
