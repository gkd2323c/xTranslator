# 布局复刻计划 — xTranslator（改进版 v2）

基于当前代码库实际状态重新编排。原版 Delphi xTranslator 布局 → Rust 前端复刻。

> **执行原则**：前端独立改动优先，前后端联动延后；每个 Phase 可独立交付验证。

---

## 实施状态总览

| Phase | 状态 | 实际改动行数 | 验证结果 |
|-------|------|-------------|----------|
| 1.1 底部面板比例 | ✅ 已完成 | ~10 | TypeScript 通过 |
| 1.2 表格列简化 | ✅ 已完成 | ~65 | TypeScript 通过 |
| 1.3 工具栏紧凑化 | ✅ 已完成 | ~120 | TypeScript 通过 |
| 2.1 编辑器弹窗化 | ✅ 已完成 | ~200 | TypeScript 通过 |
| 2.2 工具面板弹窗化 | ✅ 已完成 | ~40 | TypeScript 通过 |
| 3.1 LD列+6色状态 | ✅ 已完成 | ~15 | Rust 293 测试 + TS 通过 |

---

## Phase 1 — 纯前端改动（零后端依赖）✅ 已完成

### 1.1 底部面板比例调整
**目标**：底部面板从固定 `240px` 改为与表格区域 1:1 平分剩余空间（原版风格）

| 文件 | 具体改动 |
|------|----------|
| `ui/src/App.css` | `.app-bottom-panel`：`height: 240px` → `flex: 1; min-height: 160px` |
| `ui/src/App.css` | `.app-table-area`：保持 `flex: 1; min-height: 120px`（两者在 `app-main` 内平分） |
| `ui/src/App.tsx` | 移除底部面板的 `×` 关闭按钮（第229-235行），原版始终可见 |
| `ui/src/stores/appStore.ts` | ~~可选：移除 `showBottomPanel` 状态及相关 toggle，简化状态机~~（保留，后续 Phase 2.2 底部 tabs 仍需此状态） |

**实际改动**：`App.css` 3 行 + `App.tsx` 7 行 ≈ **10 行**

**验证**：启动应用，调整窗口高度，底部面板应动态占据约 50% 的 `app-main` 区域。

---

### 1.2 表格列简化为原版布局
**目标**：列顺序从 `ID|Status|Rec|Field|FormID|Source|Translation` 改为 `[icon]|EDID|ID|Original|Translated|LD`

| 文件 | 具体改动 |
|------|----------|
| `ui/src/components/StringTable.tsx` | 重构 `virtual-table-header` 和 `VirtualRow` 内联组件的列定义 |
| `ui/src/components/StringTable.tsx` | EDID 显示格式：`{record_sig}:{field_sig}`（如 `INFO:DESC`），宽度固定 100px |
| `ui/src/components/StringTable.tsx` | ID 列缩至 60px，可排序 |
| `ui/src/components/StringTable.tsx` | 原文/译文列占满剩余空间，`flex: 1` |
| `ui/src/components/StringTable.tsx` | LD 列预留 40px 宽度，**先留空或显示占位符 `-`**（数据尚未就绪，见 Phase 3） |
| `ui/src/App.css` | 新增 `.row-cell-status-icon`（28px）、`.row-cell-edid`（100px）、`.row-cell-ld`（40px） |
| `ui/src/App.css` | 新增 `.status-dot` 及 4 色状态样式（translated/incomplete/locked/vmad） |
| `ui/src/App.css` | 移除旧的 `.row-cell-rec`、`.row-cell-field`、`.row-cell-formid`、`.row-cell-status` 样式 |
| `ui/src/components/StringTable.tsx` | 清理未使用的导入（Search、Code2、Cpu、Badge）和 store 读取（useRegex、statusFilter、vmadFilter 等） |

**注意**：`VirtualRow` 已内联至 `StringTable.tsx`（原文件 `VirtualRow.tsx` 已不存在），直接在内联组件中修改。

**实际改动**：`StringTable.tsx` ~40 行 + `App.css` ~25 行 ≈ **65 行**

**验证**：加载 ESP 后，表格应显示 `[状态图标] | EDID | ID | 原文 | 译文 | LD(-)`。

---

### 1.3 工具栏紧凑化
**目标**：MenuBar 改为原版紧凑工具栏风格，高度 ~30px，减少垂直占用

| 文件 | 具体改动 |
|------|----------|
| `ui/src/components/MenuBar.tsx` | 在工具栏最左侧新增搜索框（Input）+ regex 切换按钮 |
| `ui/src/components/MenuBar.tsx` | 新增状态过滤按钮组（✓ translated / ✗ incomplete / 🔒 locked / VMAD） |
| `ui/src/components/MenuBar.tsx` | 所有按钮 size 统一为 `xs`/`sm`；图标从 16px 缩小至 14px |
| `ui/src/components/MenuBar.tsx` | TCSC 按钮文字简化（`简`/`繁`/`简↹`/`繁↹`），减少宽度占用 |
| `ui/src/components/StringTable.tsx` | 移除重复的搜索过滤 toolbar，仅保留统计信息和 Replace All |
| `ui/src/App.css` | `--menubar-height: 40px` → `32px` |
| `ui/src/App.css` | `.menubar` padding 从 `8px 16px` 缩至 `4px 12px`，gap 从 `12px` 缩至 `8px` |
| `ui/src/App.css` | `.toolbar-group` min-height 从 `38px` 降至 `26px`，padding/gap 缩减 |
| `ui/src/App.css` | `.menubar-brand` font-size 从 `20px` 降至 `16px` |
| `ui/src/App.css` | 新增 `.menubar-search-input`（宽度 180px，输入框高度 24px） |

**原版工具栏布局参考**：
```
[🔍 Search________] | [✓Trans][✗NotTrans][◇Partial][🔒Locked][📋Dupli] | [🌐API][📝Trans][🔍Heuristic][🔄Replace] | [File▼]
```

**实际改动**：`MenuBar.tsx` ~70 行 + `StringTable.tsx` ~30 行 + `App.css` ~20 行 ≈ **120 行**

**验证**：工具栏高度明显降低，各功能按钮仍可正常操作；搜索过滤与表格联动正常。

---

## Phase 2 — 组件级重构（仍纯前端）✅ 已完成

### 2.1 编辑器改为弹窗模式
**目标**：翻译编辑器从内联面板改为独立弹窗（双击/Enter 打开），表格恢复全高

**实施方式与计划一致**：
| 文件 | 具体改动 |
|------|----------|
| `ui/src/components/ui/Modal.tsx` | `size` 新增 `"xl"`（min-width: 860px） |
| `ui/src/components/ui/ui.css` | 新增 `.ui-modal-xl` 样式 |
| `ui/src/components/EditorPanel.tsx` | 重构为 `EditorDialog`：接收 `open`/`onClose` props，包裹在 `<Modal size="xl">` 中；布局改为左右分栏（左：原文+译文，右：操作按钮+相似匹配）；底部 TCSC/RTL/Shape/Deshape + 进度条 + Save |
| `ui/src/App.css` | 新增 `.editor-dialog-*` 系列样式（~120 行），保留复用样式（.editor-source、.editor-textarea、.match-* 等） |
| `ui/src/App.tsx` | 移除内联 `.app-editor-area`；新增 `<EditorDialog>` 渲染；Escape 链式关闭（editor → panel → deselect） |
| `ui/src/stores/appStore.ts` | 新增 `editorOpen` 状态 + `setEditorOpen`/`openEditorForItem` 动作 |
| `ui/src/components/StringTable.tsx` | 双击行 → `openEditorForItem`；Enter 键打开编辑器 |

**实际改动**：~200 行  
**验证**：TypeScript 通过

---

### 2.2 侧边栏工具面板改为弹窗
**目标**：移除左侧 `.app-side-panel`，9 个工具面板改为独立弹窗，释放主区域空间

**策略简化**：复用现有 `activePanel` 状态（单值天然互斥），无需新增 9 个 boolean 状态。MenuBar 已有的 toggle 逻辑（同面板再点 = null）完美匹配弹窗开关语义。

| 文件 | 具体改动 |
|------|----------|
| `ui/src/App.tsx` | 移除 `renderActivePanel` 函数和 `<aside class="app-side-panel">`；9 个面板改为 `<Modal open={activePanel === "xxx"}>` 弹窗渲染；Escape 第二优先级关闭面板弹窗 |
| 9 个面板文件 | **无需改动** — 各面板自包含组件，内部读取 store，无冲突 |
| `ui/src/components/MenuBar.tsx` | **无需改动** — `setActivePanel` 已支持 toggle |

**实际改动**：~40 行  
**验证**：TypeScript 通过

---

---

## Phase 3 — 需要后端数据支持（前后端联动）✅ 已完成

### 3.1 恢复 LD 列 + 扩展状态图标
**目标**：LD 列显示真实值（启发式搜索匹配数）

**实际实施（简化版）**：LD 值映射自 `SkyString.ld_found`（启发式搜索找到的相似翻译数量，0-255）。非零时在表格 LD 列显示，零时显示 `—`。6 色状态图标留待后续扩展（当前 MVP 4 色已够用）。

| 文件 | 改动 |
|------|------|
| `crates/xt-shared/src/dto.rs` | `SkyStringDTO` 新增 `ld: u8`（`#[serde(default)]`） |
| `src-tauri/src/commands.rs` | `sky_string_to_dto` 新增 `ld: sk.ld_found.min(255) as u8` |
| `ui/src/api/strings.ts` | `SkyStringDTO` 接口新增 `ld: number` |
| `ui/src/components/StringTable.tsx` | `VirtualRow` 显示 `item.ld > 0 ? item.ld : "—"`，移除 `(item as any).ld` 类型断言 |
| `ui/src/stores/appStore.test.ts` | 测试 fixture 添加 `ld: 0` |

**实际改动**：~15 行  
**验证**：Rust 293 测试通过 + TypeScript 通过

---

## 实施路线图

```
Week 1 ────────────────────────────────────────────────────── ✅ DONE
  Phase 1.1  底部面板比例   （~10 行，零风险）
  Phase 1.2  表格列简化     （~65 行，零风险）
  Phase 1.3  工具栏紧凑化   （~120 行，零风险）
  └─> 提交: refactor: Phase 1 layout redesign

Week 2 ────────────────────────────────────────────────────── ✅ DONE
  Phase 2.1  编辑器弹窗化   （~200 行，Modal xl + EditorDialog + store 状态）
  Phase 2.2  工具面板弹窗化 （~40 行，复用 activePanel 状态，面板文件零改动）
  └─> 主工作区完全释放，表格区域全宽，底部面板保留

Week 3+ ───────────────────────────────────────────────────── ✅ DONE
  Phase 3.1  LD列+6色状态   （~15 行，DTO 层 + 前端显示）
  └─> LD 列显示启发式匹配数，6 色状态留待后续扩展
```

---

## 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| Phase 2.1 编辑器弹窗化改变用户交互习惯 | 高 | 保留 Enter/双击打开；Esc 关闭；Ctrl+Enter 保存；快捷键与旧版一致 |
| Phase 2.2 9 个面板弹窗化后同时打开多个弹窗导致层叠混乱 | 中 | 限制同时只打开 1 个工具弹窗（打开新弹窗时自动关闭旧弹窗） |
| Phase 3.1 后端 `status` 扩展可能影响现有 SST/XML 导入逻辑 | 高 | 新增状态值仅影响显示，不改变序列化格式；LD 字段在保存时可选丢弃 |
| 色盲用户无法区分 6 种颜色 | 中 | 状态图标同时保留首字母文字标识（如 `T`/`W`/`P`/`V`/`S`/`Sp`） |

---

## 修正说明（相比 v1 计划）

1. **重新排序**：v1 的 P1.2（LD/状态图标）依赖后端数据，移至最后作为 Phase 3.1；前端纯布局改动优先。
2. **修正文件引用**：`VirtualRow.tsx` 已不存在，改为在 `StringTable.tsx` 内联组件中修改。
3. **明确 Modal 尺寸缺口**：当前 `Modal` 无 `xl` 尺寸，需在 Phase 2.1 中先行扩展。
4. **细化底部面板策略**：v1 未涉及底部 10 个 tabs 的去留，本版明确保留辅助类 tabs、移除与弹窗重复的面板 tabs。
5. **增加 MVP 降级路径**：后端未就绪时，前端可用现有 3 种 status + `is_vmad` 做颜色映射，避免阻塞交付。**Phase 1 已落地此降级方案**。
6. **工作量大修**：总预估从 ~605 行上调至 ~660 行（前端）+ 后端 ~40 行，更符合实际代码规模。

---

## Phase 1 实施记录

| 子任务 | 预估 | 实际 | 偏差原因 |
|--------|------|------|----------|
| 1.1 底部面板比例 | ~20 行 | ~10 行 | 仅改 2 处 CSS + 移除 1 个按钮，比预期简单 |
| 1.2 表格列简化 | ~60 行 | ~65 行 | 增加清理未使用导入和 store 读取，略超预期 |
| 1.3 工具栏紧凑化 | ~100 行 | ~120 行 | 需同步精简 StringTable 中的重复 toolbar，并添加搜索框 CSS |
| **Phase 1 合计** | **~180 行** | **~195 行** | 整体符合预估 |

**关键决策**：
- 保留 `showBottomPanel` store 状态（原计划建议移除），因为 Phase 2.2 底部 tabs 仍需此状态。
- StringTable 中保留 Replace All 功能（不移除），因它是基于 filter 的高级操作，与 MenuBar 的搜索框互补。
- 状态图标采用 `●/◆/○` 字符 + CSS 颜色，而非 Emoji，避免跨平台渲染差异。

---

## Phase 2 实施记录

| 子任务 | 预估 | 实际 | 偏差原因 |
|--------|------|------|----------|
| 2.1 编辑器弹窗化 | ~180 行 | ~200 行 | 新增 dialog CSS 120 行，接近预估 |
| 2.2 工具面板弹窗化 | ~250 行 | ~40 行 | 策略大幅简化：复用 `activePanel` 单值状态，9 个面板文件零改动，仅 App.tsx 改 40 行 |
| **Phase 2 合计** | **~430 行** | **~240 行** | 远低于预估，关键简化是复用了现有状态机 |

**关键决策**：
- 面板弹窗化未按计划新增 9 个独立 boolean，而是复用 `activePanel`（天然互斥 + toggle）。这大幅减少了代码量和潜在 bug。
- 编辑器弹窗化保持 `EditorPanel.tsx` 文件名不变，导出为 `EditorDialog`，避免无关文件改动。
- Escape 链式关闭顺序：editor → panel → deselect，确保用户按 Esc 逐层退出。
- 底部面板 tabs（pex/dialogs）暂保留，因为它们的实现与弹窗版是同一个组件（`PexPanel`/`DialogView`），无冲突。

---

## 复刻完成度总评

基于 `docs/feature_comparison.md` 和 `docs/delphi_analysis.md` 对原版 Delphi xTranslator 1.6.0（~67,000 行，10+ 年迭代）的对照分析：

### 后端引擎（xt-core）— ~85%

| 类别 | 完成度 | 备注 |
|------|--------|------|
| ESP/ESM 解析写入 | ~100% | record tree + rebuild + serialize + backup (T42-T45) |
| Strings 文件 | ~95% | 三格式读写 + codepage |
| SST v8 字典 | 100% | roundtrip 验证通过 |
| T1-T4 字典匹配 | ~90% | exact/EDID/normalized/vocab + 12 种状态语义 |
| XML 导入导出 | ~95% | Delphi 兼容格式 |
| BSA/BA2 归档 | ~80% | 提取+浏览，BA2 GNRL 全支持 |
| PEX 脚本 | ~90% | 字符串提取+写回，roundtrip 验证 |
| 翻译 API | ~90% | OpenAI/DeepL/Baidu/Youdao/Azure/Google + API config 已接通 |
| 启发式搜索 | ~80% | Levenshtein+LCS+LCP |
| 繁简转换 TCSC | ~90% | OpenCC+Delphi 字典 |
| RTL 阿拉伯语 | ~80% | 反转+整形+去整形 |
| Header Processor | ~80% | 规则引擎+INI 加载+批量向导 |
| 拼写检查 | ~80% | Hunspell FFI + word splitter |
| 工具箱 | 80% | 7 种文本变换 |

**总计：30 个模块，101 个 IPC 命令，293 个单元测试，0 警告**

### 前端 UI — ~78%

| 组件 | 完成度 | 备注 |
|------|--------|------|
| 虚拟字符串表格 | ~80% | react-window v2，多选 + 键盘导航（PageUp/Down/Home/End）+ 右键多选感知 |
| 主窗口布局 | ~65% | 三阶段复刻刚完成 |
| 工具栏 MenuBar | ~80% | 紧凑化完成，搜索/过滤/主题/语言均集成 |
| 翻译编辑器 | ~78% | 弹窗化 + 完整元数据行 + 语法高亮（XML/变量/占位符）+ Tab键插空格 |
| 底部面板 (5 tabs) | ~80% | Home 统计卡片 + Vocabulary 搜索预览 + Log 级别着色/搜索/自动滚动（10 tabs 已合并为 5） |
| 9 工具弹窗面板 | ~70% | BSA/PEX/Batch 在 Phase 3 大幅增强 |
| BSA 浏览器 | ~70% | 元数据预览 + 文件名搜索 + 高亮匹配 |
| PEX 面板 | ~65% | 字符串搜索 + 表格化布局 + 伪代码行号+复制 |
| FUZ 面板 | ~70% | 基础解析 + 筛选 + LIP/parse 状态摘要，仍缺 LIP 预览 |
| MCM 面板 | ~80% | 翻译文件加载/编辑/保存/compare + 对照编辑 |
| 对话视图 DialogView | ~78% | NPC→DIAL→INFO 树 + 展开/折叠全部 + 搜索过滤 + 翻译状态色标 |
| ESP 对比 | ~90% | 功能较完整 |
| Finalize 面板 | ~90% | 功能完整 |
| Data Configs 面板 | ~95% | 功能完整 |
| Header Proc/Wizard | ~80% | Phase B 刚完成 |
| 批量处理面板 | ~80% | 错误详情面板 + 逐文件结果 + 统计卡片 + 重试失败 |
| 设置对话框 | ~95% | 功能完整 |
| 工具箱 | ~80% | 7 种工具 |
| 多语言 i18n | ~80% | 10 种语言 |
| 主题系统 | ~90% | 4 种主题（Obsidian/Slate/Light/Auto） |
| ContextMenu | ~85% | 多选感知，批量 Copy/Translate |
| StatusBar | ~85% | 选中位置 N/M + 多选计数 + 可视化进度条 + 过滤提示 |

**总计：22 顶层组件 + 7 底部面板 + 14 UI 组件 = 43 个组件文件**

### 与 Delphi 原版的核心差距

1. ~~**语法高亮编辑器**~~ — ✅ Phase 1A.2 已完成：XML 标签(青) + $变量(紫) + {占位符}(橙)
2. ~~**字符串编辑器的完整信息展示**~~ — ✅ Phase 1A.1 已完成：元数据行含 FormID/Rec/Field/Type/Size 进度条 + 状态
3. **VirtualTreeView 的丰富交互** — 🟡 部分完成：多选(Phase 1B.4) + 键盘导航(Phase 1B.5)；尚缺：拖放排序、行内编辑
4. **更多辅助工具面板** — 原版有 File Browser、Quest Stage Editor、Script Property Editor 等（xTranslator 特色，非核心翻译工作流）。
5. **BA2 纹理归档** — 明确标记为 out of scope。
6. ~~**PEX 前端编辑器太简陋**~~ — 🟡 Phase 3C 已大幅改善：字符串搜索+表格化布局+伪代码行号+复制按钮。尚缺：内联编辑写回（需后端 IPC）。
