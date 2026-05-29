# UI 深度打磨设计规范

> **项目：** xTranslator  
> **版本：** v2.0 UI Overhaul  
> **日期：** 2026-05-29  
> **状态：** Draft  
> **方案：** 痛点优先，分层推进（方案 A）

---

## 1. 概述

### 1.1 目标

在现有 Tauri 2.x + React + TypeScript 架构上，全面复刻原版 Delphi xTranslator 的信息密度和工作流，同时引入现代 UI 改进（动画过渡、更好的键盘快捷键、响应式布局、HiDPI 适配），实现**超越原版**的用户体验。

### 1.2 范围

覆盖 ui_reproduction_plan.md 的 P0-P3 全部内容，聚焦 4 个核心痛点：

1. **编辑体验** — EditorDialog 内联化，减少切换成本
2. **面板布局** — 引入可停靠分栏系统，工具面板不再遮挡主工作区
3. **顶栏组织** — 分组折叠菜单栏，降低视觉密度
4. **底部面板** — 合并相似 tabs，信息层级更清晰

### 1.3 约束

| 约束 | 说明 |
|------|------|
| C1 | DTO 同步不变：`dto.rs` ↔ `strings.ts` |
| C2 | IPC 命令签名不变，仅新增 `editorMode` 等配置类 IPC |
| C8 | react-window v2 API 保持 `rowComponent`/`rowCount`/`rowProps` |
| zustand | 保持 `useAppStore((s) => s.field)` 选择器模式 |
| 依赖 | 新增：`react-split-pane`（~3KB gzipped）；无其他新依赖 |
| 主题 | 4 个主题（Obsidian/Slate/Light/Auto）全部适配 |
| i18n | 10 语言 locale 全部更新 |

### 1.4 工作量预估

| Phase | 内容 | 预估 |
|-------|------|------|
| Phase 1 | 顶栏重组 | 1-2 天 |
| Phase 2 | 编辑体验 | 2-3 天 |
| Phase 3 | 面板系统 | 3-4 天 |
| Phase 4 | 底部面板 | 2 天 |
| Phase 5 | 视觉统一 | 2-3 天 |
| **总计** | | **10-14 天** |

---

## 2. 整体架构

### 2.1 布局骨架

```
┌─ MenuBar (Phase 1: 分组折叠) ──────────────────────────┐
│ 文件  编辑  搜索  翻译  工具  视图       [TCSC] [主题] [语言] │
├─ Split Pane (Phase 3) ───────────────────────────────────┤
│ ┌─ StringTable ────────┐ │ ┌─ Right Panel (可选) ──────┐│
│ │                      │ │ │ BSA / PEX / FUZ / Compare ││
│ │  react-window v2     │ │ │                           ││
│ │  76K+ 虚拟行         │ │ │                           ││
│ │  右键菜单增强         │ │ │                           ││
│ └──────────────────────┘ │ └───────────────────────────┘│
├─ Bottom Panel (Phase 4: 5 tabs) ────────────────────────┤
│ 概览 │ 词汇 │ 日志 │ 浏览器 │ 头处理                    │
├─ StatusBar (Phase 5: 视觉统一) ─────────────────────────┤
│ 进度条 + 位置 N/M + 多选 + 过滤                          │
└─────────────────────────────────────────────────────────┘
```

### 2.2 组件树变化

```
App
├── GroupedMenuBar          (Phase 1: 替换 MenuBar)
├── EditorDialog            (Phase 2: 重构为 EditorPanel/ 目录)
├── SplitPaneLayout         (Phase 3: 新增)
│   ├── StringTable
│   ├── RightPanelContainer (Phase 3: 新增，管理右侧停靠面板)
│   │   ├── BsaBrowser
│   │   ├── PexPanel
│   │   ├── FuzPanel
│   │   └── EspComparePanel
│   └── BottomPanelContainer (Phase 4: 重组)
│       ├── OverviewTab     (合并 home)
│       ├── VocabularyTab
│       ├── LogTab
│       ├── ExplorerTab     (合并 heuristic + espTree + quests)
│       └── HeaderTab       (合并 headerProc + headerWizard)
├── BatchTranslateBar
├── StatusBar
└── ToolDialogs             (Phase 3: 部分迁移到停靠面板)
    ├── ToolboxDialog
    ├── SettingsDialog
    └── SpellCheckDialog
```

### 2.3 状态管理变化

```typescript
// appStore.ts 新增状态
interface AppState {
  // ... 现有状态 ...

  // Phase 1: 无新增

  // Phase 2: 编辑器模式
  editorMode: 'modal' | 'sidebar' | 'inline'

  // Phase 3: 面板布局
  panelLayout: {
    rightPanels: PanelId[]           // 右侧停靠的面板
    bottomExpanded: PanelId[]        // 底部扩展的面板
    panelSizes: Record<PanelId, number>
    rightPanelVisible: boolean       // 右侧面板整体可见性
    bottomPanelHeight: number        // 底部面板高度
  }

  // Phase 4: 底部面板
  activeBottomTab: string            // 当前活动的底部 tab
  bottomTabs: BottomTabConfig[]      // tab 配置

  // Phase 5: 无新增
}

type PanelId = 'bsa' | 'pex' | 'fuz' | 'compare' | 'mcm' | 'finalize' | 'dataconfigs'
// 注：'header' 不在 PanelId 中，因为 headerProc/headerWizard 始终保留在底部面板（Phase 4）
```

---

## 3. Phase 1：顶栏重组

### 3.1 分组策略

| 分组 | 包含功能 | 图标 |
|------|---------|------|
| 文件 | 打开、保存、导入XML、导出XML、退出 | FolderOpen |
| 编辑 | 撤销、重做、全选、复制、替换 | Edit |
| 搜索 | 文本搜索、正则搜索、状态过滤、Record类型过滤 | Search |
| 翻译 | API 设置、批量翻译、启发式搜索、字典操作、TCSC | Languages |
| 工具 | BSA、PEX、FUZ、Header、MCM、对比、拼写、工具箱、设置 | Wrench |
| 视图 | 底面板切换、侧栏切换、编辑器模式切换 | Layout |

### 3.2 保留为独立按钮

| 按钮 | 原因 |
|------|------|
| TCSC（繁简转换） | 使用频率极高，不放入菜单 |
| 主题切换 | 一键切换，不放入菜单 |
| 语言切换 | 一键切换，不放入菜单 |

### 3.3 交互行为

| 行为 | 说明 |
|------|------|
| 点击分组 | 下拉菜单展示子项，带快捷键提示（右对齐） |
| 悬停分组 | 200ms 延迟展开，防误触 |
| 快捷键 | 所有高频操作保留原有快捷键 |
| 工具分组 | 仅在相关文件加载后启用（如 BSA 需要 .bsa 文件） |
| 分组高亮 | 有活动子功能的分组显示指示器（如工具分组有打开的面板时） |

### 3.4 技术实现

- **新组件：** `ui/src/components/GroupedMenuBar.tsx`
- **替换：** 现有 `MenuBar.tsx`
- **实现方式：** 使用 CSS `position: absolute` 下拉 + 事件委托，无新依赖
- **i18n：** 复用现有 locale key，新增分组标题 key
- **保持不变：** 所有 IPC 调用、store 操作、快捷键绑定

---

## 4. Phase 2：编辑体验

### 4.1 三种编辑模式

| 模式 | 触发方式 | 布局 | 适合场景 |
|------|---------|------|---------|
| **弹窗模式** | 双击行 / Enter | Modal xl（现有） | 快速修改单条 |
| **侧栏模式** | Ctrl+2 或菜单 | 右侧固定 40% 宽度 | 连续编辑多条 |
| **内联模式** | Ctrl+3 或 F2 | 行下方展开 ~120px | 批量快速浏览 |

### 4.2 模式切换

```
编辑器右上角：[弹窗] [侧栏] [内联]  ← 三个图标按钮
快捷键：Ctrl+1 / Ctrl+2 / Ctrl+3
```

- 模式选择持久化到 `AppConfig.editorMode`
- 侧栏模式下 StringTable 宽度自动缩小 40%
- 内联模式下选中行高度从固定 32px 扩展到 ~120px

### 4.3 核心改进（所有模式共享）

| 改进 | 说明 |
|------|------|
| **Tab 链路** | Tab 在源文/译文字段间切换（可配置：字段切换 vs 插入空格） |
| **Ctrl+↑/↓ 增强** | 侧栏模式下自动滚动表格到对应行 |
| **元数据行优化** | FormID、RecordSig、FieldSig、EDID 一行展示，可折叠 |
| **语法高亮增强** | 占位符 `<Alias=...>`、变量 `%s`、XML 标签用不同颜色 |
| **保存反馈** | Ctrl+Enter 保存后显示 toast + 可选自动跳到下一条未翻译项 |
| **Escape 链** | 编辑器 → 取消选中（不再关闭整个弹窗后丢失上下文） |

### 4.4 技术实现

- **重构：** `EditorPanel.tsx` → `EditorPanel/` 目录
  - `EditorCore.tsx` — 共享编辑逻辑（元数据、高亮、快捷键、保存）
  - `EditorModal.tsx` — 弹窗模式（包装现有 Modal xl）
  - `EditorSidebar.tsx` — 侧栏模式（flex 布局，右侧固定）
  - `EditorInline.tsx` — 内联模式（行下方展开）
- **Store 新增：** `editorMode: 'modal' | 'sidebar' | 'inline'`
- **配置新增：** `AppConfig.editorMode`, `AppConfig.tabBehavior: 'switch' | 'space'`
- **保持不变：** `update_translation(id, text)` IPC、`selectedId` 逻辑
- **与 Phase 3 的关系：** 侧栏模式使用独立的右侧面板空间，不与停靠面板冲突——编辑器侧栏在最右侧，停靠面板在编辑器左侧（三栏布局：StringTable | 停靠面板 | 编辑器侧栏）

---

## 5. Phase 3：面板系统

### 5.1 面板挂载位置

| 位置 | 说明 | 适合面板 |
|------|------|---------|
| **右侧分栏** | 可拖拽调整宽度，与 StringTable 并列 | BSA、PEX、FUZ、ESP Compare |
| **底部扩展** | 在 BottomPanel 上方叠加，高度可调 | MCM、Header、Finalize、DataConfigs |
| **浮动窗口** | 保持现有 Modal 行为 | Toolbox、设置、拼写检查 |

### 5.2 面板管理

```typescript
// zustand store
panelLayout: {
  rightPanels: PanelId[]           // 右侧停靠面板列表
  bottomExpanded: PanelId[]        // 底部扩展面板列表
  panelSizes: Record<PanelId, number>  // 面板大小（px 或 %）
  rightPanelVisible: boolean       // 右侧面板整体可见性
  bottomPanelHeight: number        // 底部面板高度（px）
}

// 操作
togglePanel(id: PanelId): void     // 切换面板显示/隐藏
setPanelSize(id: PanelId, size: number): void
toggleRightPanelVisibility(): void
```

### 5.3 交互行为

| 行为 | 说明 |
|------|------|
| 打开面板 | 菜单/快捷键 → 如果已停靠则聚焦，否则添加到默认位置 |
| 关闭面板 | 点击 X / 再次点击菜单项 → 从停靠位置移除 |
| 拖拽调整 | react-split-pane 分割线拖拽，持久化到 AppConfig |
| 响应式 | 窗口宽度 < 1024px 时，右侧面板自动切换为底部扩展 |
| 面板标题 | 每个面板显示标题栏 + 图标 + 关闭按钮 |

### 5.4 技术实现

- **依赖：** `react-split-pane`（~3KB gzipped）
- **新组件：**
  - `DockablePanel.tsx` — 面板容器（标题栏 + 关闭按钮 + 内容区）
  - `RightPanelContainer.tsx` — 右侧面板容器（管理多个停靠面板）
  - `SplitPaneLayout.tsx` — 整体分栏布局
- **重构：** `App.tsx` 布局逻辑用 react-split-pane 替代固定 flex
- **保持不变：** 现有面板组件（BsaBrowser、PexPanel 等）内部逻辑

---

## 6. Phase 4：底部面板

### 6.1 Tab 合并

| 新 Tab | 合并来源 | 内容 |
|--------|---------|------|
| **概览** | home | 统计卡片 + 进度条 + 翻译状态 |
| **词汇** | vocabulary | 搜索式词汇对列表 |
| **日志** | log | 彩色日志 + 搜索 + 自动滚动 |
| **浏览器** | heuristic + espTree + quests | 三合一树浏览器 |
| **头处理** | headerProc + headerWizard | 规则编辑器 + 批量向导 |

**移除：** pex、dialogs —— 已在 Phase 3 中作为右侧停靠面板提供

### 6.2 浏览器 Tab 设计

三合一浏览器结构：

```
┌─ 浏览器 ───────────────────────────────────────────┐
│ [启发式] [ESP树] [任务]   ← 子 tab 切换             │
├─ 树视图 ───────────┬─ 详情面板 ─────────────────────┤
│ NPC_               │ FormID: 00012345               │
│ ├─ Lydia           │ Record: NPC_                   │
│ ├─ Faendal         │ EDID: Lydia                    │
│ └─ ...             │ Fields: 42                     │
│                    │ Strings: 5 (3 translated)      │
│                    │ [跳到表格] [编辑]               │
└────────────────────┴────────────────────────────────┘
```

### 6.3 视觉改进

| 改进 | 说明 |
|------|------|
| Tab 图标 | 概览=BarChart, 词汇=Book, 日志=Terminal, 浏览器=TreePine, 头处理=Settings |
| 徽标计数 | 日志 tab 显示未读错误数，词汇 tab 显示匹配数 |
| 高度记忆 | 拖拽调整底部面板高度，持久化到 AppConfig |
| 子 tab | 浏览器和头处理内部使用子 tab 切换，不增加主 tab 数量 |

### 6.4 技术实现

- **重构：** `BottomPanel.tsx` 合并 tab 逻辑
- **新组件：**
  - `ExplorerTab.tsx` — 三合一浏览器
  - `HeaderTab.tsx` — 合并 headerProc + headerWizard
- **Store 变化：** `activeBottomTab` 类型从 10 个值缩减到 5 个
- **保持不变：** 各子面板内部逻辑（HeuristicPanel、EspTreePanel 等）

---

## 7. Phase 5：视觉统一

### 7.1 动画与过渡

| 场景 | 动画 | 时长 | 缓动 |
|------|------|------|------|
| 面板打开/关闭 | slide-in/out + opacity | 200ms | ease-out |
| Tab 切换 | fade cross-fade | 150ms | linear |
| 编辑器模式切换 | 平滑宽度/高度过渡 | 250ms | ease-in-out |
| 选中行高亮 | 背景色过渡 | 100ms | linear |
| Toast 通知 | slide-up + auto-dismiss | 300ms in, 2s hold | ease-out |
| 菜单下拉 | slide-down + opacity | 150ms | ease-out |

### 7.2 颜色语义系统

所有主题统一使用以下 CSS 变量：

| 变量 | 用途 | Obsidian 示例 |
|------|------|--------------|
| `--color-primary` | 选中行、活动 tab、主按钮 | `#3b82f6` (blue-500) |
| `--color-success` | 已翻译、完成状态 | `#22c55e` (green-500) |
| `--color-warning` | 部分翻译、待审核 | `#eab308` (yellow-500) |
| `--color-danger` | 未翻译、错误、删除 | `#ef4444` (red-500) |
| `--color-info` | 信息提示、辅助文字 | `#6b7280` (gray-500) |
| `--color-surface` | 面板背景、卡片 | `#1e1e2e` |
| `--color-border` | 边框、分割线 | `#313244` |
| `--color-hover` | 悬停背景 | `#2a2a3c` |

### 7.3 字体与间距

| 属性 | 当前 | 目标 |
|------|------|------|
| 表格行高 | 28px | 32px（更好的触控目标） |
| 面板内边距 | 8px | 12px（呼吸感） |
| 按钮高度 | 28px | 32px（HiDPI 友好） |
| 字体族 | 系统默认 | `Inter, system-ui, sans-serif` |
| 等宽字体 | 无统一 | `JetBrains Mono, Consolas, monospace` |
| 基准字号 | 14px | 14px（不变，使用 rem） |

### 7.4 HiDPI 适配

- 所有尺寸使用 `rem` 替代 `px`（基准 16px）
- 图标统一使用 lucide-react SVG（天然矢量）
- 分割线拖拽手柄增大到 8px（触屏友好）
- 面板最小宽度约束：右侧 ≥ 300px，底部 ≥ 200px
- 响应式断点：1024px（右侧面板折叠为底部扩展）

### 7.5 键盘快捷键增强

| 快捷键 | 功能 | 状态 |
|--------|------|------|
| `Ctrl+1/2/3` | 切换编辑器模式（弹窗/侧栏/内联） | 新增 |
| `Ctrl+\` | 切换右侧面板显示 | 新增 |
| `Ctrl+Shift+L` | 聚焦底部日志 | 新增 |
| `Ctrl+Shift+B` | 聚焦底部面板 | 新增 |
| `F2` | 内联编辑选中行 | 新增 |
| `Escape` 链 | 编辑器→面板→取消选中 | 增强 |
| `Ctrl+S` | 保存当前编辑 | 保持 |
| `Ctrl+Enter` | 保存翻译并跳到下一条 | 增强 |
| `Ctrl+↑/↓` | 跳到上/下一条未翻译 | 增强（侧栏模式自动滚动） |

---

## 8. 错误处理

| 场景 | 处理方式 |
|------|---------|
| 面板组件加载失败 | 显示错误边界 + 降级到 Modal 模式 |
| react-split-pane 尺寸异常 | 恢复默认尺寸（右侧 40%，底部 30%） |
| AppConfig 损坏 | 使用默认配置，不阻塞启动 |
| 编辑器模式切换时有未保存更改 | 确认对话框，保留更改 |
| 面板停靠时空间不足 | 自动切换到浮动模式 |

---

## 9. 测试策略

### 9.1 单元测试

| 测试项 | 说明 |
|--------|------|
| Store 操作 | panelLayout 状态变更、editorMode 切换 |
| 组件渲染 | 各编辑器模式正确渲染、面板容器正确挂载 |
| 快捷键 | 新增快捷键触发正确操作 |

### 9.2 E2E 测试

| 测试项 | 说明 |
|--------|------|
| 编辑器模式切换 | Modal→Sidebar→Inline 切换 + 数据保持 |
| 面板停靠 | 打开/关闭/拖拽调整面板 |
| 底部 tab 切换 | 5 个 tab 正确切换 + 内容加载 |
| 快捷键链 | Escape 链完整执行 |
| 响应式 | 窗口缩小到 < 1024px 时面板自动折叠 |

### 9.3 手动测试

| 测试项 | 说明 |
|--------|------|
| 主题一致性 | 4 个主题下所有新组件颜色正确 |
| 动画流畅度 | 无卡顿、无闪烁 |
| 76K+ 条数据 | 虚拟滚动性能不退化 |
| HiDPI | 150%/200% 缩放下布局正确 |

---

## 10. 迁移与兼容性

### 10.1 渐进迁移

每个 Phase 独立可测，不影响其他功能：

1. Phase 1 完成后：MenuBar 体验改善，其他不变
2. Phase 2 完成后：编辑体验改善，其他不变
3. Phase 3 完成后：面板布局改善，其他不变
4. Phase 4 完成后：底部面板改善，其他不变
5. Phase 5 完成后：视觉统一，整体体验提升

### 10.2 配置迁移

| 配置项 | 变化 | 兼容性 |
|--------|------|--------|
| `editorMode` | 新增 | 旧配置自动使用 `'modal'` 默认值 |
| `panelLayout` | 新增 | 旧配置使用默认布局 |
| `bottomPanelHeight` | 新增 | 旧配置使用 300px 默认值 |
| 现有配置 | 不变 | 完全兼容 |

### 10.3 回滚方案

每个 Phase 的改动是可逆的：
- Phase 1：恢复 MenuBar.tsx
- Phase 2：恢复 EditorPanel.tsx
- Phase 3：移除 react-split-pane，恢复 flex 布局
- Phase 4：恢复 10 tab 版本
- Phase 5：恢复原有 CSS 变量

---

## 附录 A：参考文件

| 文件 | 用途 |
|------|------|
| `ui_reproduction_plan.md` | 原版界面复刻方案 |
| `ui/src/components/MenuBar.tsx` | 现有顶栏 |
| `ui/src/components/EditorPanel.tsx` | 现有编辑器 |
| `ui/src/App.tsx` | 现有布局 |
| `ui/src/stores/appStore.ts` | 状态管理 |
| `docs/feature_comparison.md` | 功能对比 |

## 附录 B：依赖清单

| 依赖 | 版本 | 用途 | 大小 |
|------|------|------|------|
| react-split-pane | ^0.1.92 | 可拖拽分栏布局 | ~3KB gzipped |

无其他新依赖。现有依赖（react-window、zustand、lucide-react、react-hot-toast、react-i18next）保持不变。
