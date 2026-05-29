# UI 深度打磨实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 全面复刻原版 Delphi xTranslator 的信息密度和工作流，同时引入现代 UI 改进（动画、键盘快捷键、响应式布局、HiDPI），实现超越原版的用户体验。

**Architecture:** 5 个独立 Phase 按序推进，每个 Phase 独立可测。Phase 1 重组顶栏，Phase 2 重构编辑器为三种模式，Phase 3 引入 react-split-pane 可停靠面板系统，Phase 4 合并底部 tabs，Phase 5 统一视觉风格。所有 Phase 保持现有 IPC/DTO/zustand 模式不变。

**Tech Stack:** React 18, TypeScript, Zustand, react-window v2, react-split-pane (新增), lucide-react, react-hot-toast, react-i18next, Tauri 2.x

---

## 文件结构

### 新建文件

| 文件 | 职责 |
|------|------|
| `ui/src/components/GroupedMenuBar.tsx` | Phase 1: 分组折叠菜单栏 |
| `ui/src/components/EditorPanel/EditorCore.tsx` | Phase 2: 共享编辑逻辑（元数据、高亮、快捷键） |
| `ui/src/components/EditorPanel/EditorModal.tsx` | Phase 2: 弹窗模式编辑器 |
| `ui/src/components/EditorPanel/EditorSidebar.tsx` | Phase 2: 侧栏模式编辑器 |
| `ui/src/components/EditorPanel/EditorInline.tsx` | Phase 2: 内联模式编辑器 |
| `ui/src/components/EditorPanel/index.tsx` | Phase 2: 编辑器模式路由 |
| `ui/src/components/SplitPaneLayout.tsx` | Phase 3: 分栏布局容器 |
| `ui/src/components/DockablePanel.tsx` | Phase 3: 可停靠面板容器 |
| `ui/src/components/RightPanelContainer.tsx` | Phase 3: 右侧面板管理器 |
| `ui/src/components/bottom/ExplorerTab.tsx` | Phase 4: 三合一浏览器 |
| `ui/src/components/bottom/HeaderTab.tsx` | Phase 4: 合并头处理面板 |

### 修改文件

| 文件 | 变化 |
|------|------|
| `ui/src/stores/appStore.ts` | 新增 `editorMode`, `panelLayout`, `activeRightPanel` 状态 |
| `ui/src/App.tsx` | 用 `SplitPaneLayout` 替换固定 flex 布局，用 `GroupedMenuBar` 替换 `MenuBar` |
| `ui/src/App.css` | 新增分栏、动画、颜色语义 CSS 变量 |
| `ui/src/api/strings.ts` | 新增 `saveConfig` 字段（editorMode, panelLayout） |

---

## Phase 1：顶栏重组

### Task 1: GroupedMenuBar 基础组件

**Files:**
- Create: `ui/src/components/GroupedMenuBar.tsx`
- Modify: `ui/src/App.tsx` (替换 MenuBar 导入)

- [ ] **Step 1: 创建 GroupedMenuBar 骨架**

```tsx
// ui/src/components/GroupedMenuBar.tsx
import { useState, useRef, useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { useAppStore } from "../stores/appStore";
import { Search, Code2 } from "lucide-react";
import { Input, Button } from "./ui";

type MenuGroup = "file" | "edit" | "search" | "translate" | "tools" | "view";

interface MenuItem {
  label: string;
  onClick?: () => void;
  shortcut?: string;
  disabled?: boolean;
  separator?: boolean;
}

interface MenuDefinition {
  id: MenuGroup;
  label: string;
  icon: React.ReactNode;
  items: MenuItem[];
}

export function GroupedMenuBar() {
  const { t } = useTranslation();
  const [openGroup, setOpenGroup] = useState<MenuGroup | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  // 点击外部关闭菜单
  useEffect(() => {
    const handleClick = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setOpenGroup(null);
      }
    };
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpenGroup(null);
    };
    document.addEventListener("mousedown", handleClick);
    window.addEventListener("keydown", handleEscape);
    return () => {
      document.removeEventListener("mousedown", handleClick);
      window.removeEventListener("keydown", handleEscape);
    };
  }, []);

  const closeAndRun = (action?: () => void) => {
    setOpenGroup(null);
    action?.();
  };

  // TODO: 菜单定义将在 Step 2 添加
  const menuDefinitions: MenuDefinition[] = [];

  return (
    <div className="grouped-menubar" ref={menuRef}>
      <div className="grouped-menubar-topline">
        <div className="menubar-brand">xTranslator(x64)</div>
        <nav className="grouped-menubar-groups" role="menubar">
          {menuDefinitions.map((group) => (
            <div
              key={group.id}
              className={`menubar-group ${openGroup === group.id ? "open" : ""}`}
            >
              <button
                type="button"
                className="menubar-group-trigger"
                onClick={() => setOpenGroup(openGroup === group.id ? null : group.id)}
                onMouseEnter={() => {
                  if (openGroup !== null) setOpenGroup(group.id);
                }}
                aria-haspopup="menu"
                aria-expanded={openGroup === group.id}
              >
                {group.label}
              </button>
              {openGroup === group.id && (
                <div className="menubar-group-panel" role="menu">
                  {group.items.map((item, idx) =>
                    item.separator ? (
                      <div key={`sep-${idx}`} className="menubar-menu-separator" />
                    ) : (
                      <button
                        key={item.label}
                        type="button"
                        className="menubar-menu-item"
                        onClick={() => closeAndRun(item.onClick)}
                        disabled={item.disabled}
                        role="menuitem"
                      >
                        <span className="menubar-menu-item-label">{item.label}</span>
                        {item.shortcut && (
                          <span className="menubar-menu-item-shortcut">{item.shortcut}</span>
                        )}
                      </button>
                    )
                  )}
                </div>
              )}
            </div>
          ))}
        </nav>
      </div>
      {/* 工具栏区域将在 Step 3 添加 */}
      <div className="grouped-menubar-toolbar" />
    </div>
  );
}
```

- [ ] **Step 2: 运行类型检查验证无错误**

Run: `cd ui && npx tsc --noEmit`
Expected: 无新增错误（GroupedMenuBar 尚未被使用）

- [ ] **Step 3: 提交**

```bash
git add ui/src/components/GroupedMenuBar.tsx
git commit -m "feat(ui): add GroupedMenuBar skeleton component"
```

### Task 2: GroupedMenuBar 菜单定义与工具栏

**Files:**
- Modify: `ui/src/components/GroupedMenuBar.tsx`

- [ ] **Step 1: 添加完整菜单定义和工具栏**

在 `GroupedMenuBar.tsx` 的 `menuDefinitions` 数组中填入完整定义。在 `return` 的 `.grouped-menubar-toolbar` div 中添加搜索框、状态过滤、TCSC 按钮、主题/语言选择器。

菜单分组逻辑：
- **文件**：打开 ESP、加载/保存 SST、保存 Strings、合并 SST、导入/导出 XML、重置
- **编辑**：撤销、重做、替换全部
- **搜索**：（搜索框和状态过滤直接在工具栏，不放入菜单）
- **翻译**：打开编辑器、完成、简繁转换、源目标比较
- **工具**：Batch、BSA、PEX、FUZ、Dialog、MCM、ESP Compare、Data Configs
- **视图**：底面板切换

工具栏保留：搜索框（含正则按钮）、状态过滤按钮（✓/✗/🔒/VMAD）、TCSC 简/繁按钮、批量 TCSC、主题选择、语言选择。

所有 IPC 调用和 store 操作从现有 `MenuBar.tsx` 复制，保持完全相同的逻辑。

- [ ] **Step 2: 运行类型检查**

Run: `cd ui && npx tsc --noEmit`
Expected: PASS

- [ ] **Step 3: 提交**

```bash
git add ui/src/components/GroupedMenuBar.tsx
git commit -m "feat(ui): add GroupedMenuBar menu definitions and toolbar"
```

### Task 3: 替换 MenuBar 并集成到 App

**Files:**
- Modify: `ui/src/App.tsx` (import GroupedMenuBar 替换 MenuBar)
- Modify: `ui/src/App.css` (添加 .grouped-menubar 样式)

- [ ] **Step 1: 在 App.tsx 中替换 MenuBar**

将 `import { MenuBar } from "./components/MenuBar"` 改为 `import { GroupedMenuBar } from "./components/GroupedMenuBar"`，将 `<MenuBar />` 改为 `<GroupedMenuBar />`。

- [ ] **Step 2: 添加 GroupedMenuBar CSS**

在 `App.css` 中添加 `.grouped-menubar` 相关样式。关键样式：
- `.grouped-menubar`：display flex, height 32px
- `.menubar-group-trigger`：padding 4px 12px, hover 背景色
- `.menubar-group-panel`：position absolute, z-index 100, 最小宽度 200px
- `.menubar-group:hover`：背景色过渡 150ms

- [ ] **Step 3: 运行类型检查和前端测试**

Run: `cd ui && npx tsc --noEmit && npm run test`
Expected: PASS

- [ ] **Step 4: 手动验证**

Run: `.\dev.ps1`
验证：菜单分组可点击展开，搜索框可输入，TCSC 按钮可点击，主题/语言可切换。

- [ ] **Step 5: 提交**

```bash
git add ui/src/App.tsx ui/src/App.css ui/src/components/GroupedMenuBar.tsx
git commit -m "feat(ui): replace MenuBar with GroupedMenuBar in App"
```

---

## Phase 2：编辑体验

### Task 4: EditorCore 共享编辑逻辑

**Files:**
- Create: `ui/src/components/EditorPanel/EditorCore.tsx`
- Create: `ui/src/components/EditorPanel/index.tsx`

- [ ] **Step 1: 提取 EditorCore**

从现有 `EditorPanel.tsx` 提取共享逻辑到 `EditorCore.tsx`：

```tsx
// ui/src/components/EditorPanel/EditorCore.tsx
import { useState, useEffect, useCallback, useMemo, useRef } from "react";
import { useAppStore, computeTranslationProgress } from "../../stores/appStore";
import { updateTranslation, heuristicSearch, translateString, setApiKey, tcscConvert, rtlReverse, shapeArabic, deshapeArabic, checkAliases, spellCheckText, spellCheckSuggestions, spellCheckIgnore, type HeuristicMatchDTO, type AliasCheckResult, type SpellCheckResultDto, type SpellFaultDto } from "../../api/strings";
import { replaceUtf8ByteRange } from "../../utils/utf8";
import toast from "react-hot-toast";
import { useTranslation } from "react-i18next";

// 高亮正则（复用现有）
const HIGHLIGHT_REGEX = /(<\/?[A-Za-z][^>]*>)|(\$\w+(?:\.\w+)*)|(\{[^}]+\})/g;

function escapeHtml(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

export function highlightTags(text: string): string {
  let lastIndex = 0;
  const parts: string[] = [];
  let match: RegExpExecArray | null;
  while ((match = HIGHLIGHT_REGEX.exec(text)) !== null) {
    if (match.index > lastIndex) parts.push(escapeHtml(text.slice(lastIndex, match.index)));
    if (match[1]) parts.push(`<span class="tag-highlight">${escapeHtml(match[1])}</span>`);
    else if (match[2]) parts.push(`<span class="tag-variable">${escapeHtml(match[2])}</span>`);
    else if (match[3]) parts.push(`<span class="tag-placeholder">${escapeHtml(match[3])}</span>`);
    lastIndex = match.index + match[0].length;
  }
  if (lastIndex < text.length) parts.push(escapeHtml(text.slice(lastIndex)));
  return parts.join("");
}

export interface EditorCoreState {
  localTrans: string;
  setLocalTrans: (v: string) => void;
  isSaving: boolean;
  handleSave: () => Promise<void>;
  matches: HeuristicMatchDTO[];
  isSearching: boolean;
  handleHeuristicSearch: () => Promise<void>;
  isTranslating: boolean;
  handleTranslate: () => Promise<void>;
  aliasResult: AliasCheckResult | null;
  spellResult: SpellCheckResultDto | null;
  selectedFaultIdx: number | null;
  suggestions: string[];
  handleSelectFault: (idx: number) => Promise<void>;
  handleApplySuggestion: (s: string) => void;
  handleIgnoreWord: (w: string) => Promise<void>;
  jumpToUntranslated: (dir: "next" | "prev") => void;
  applyMatch: (translation: string) => void;
  fieldSizeWarning: { max: number; current: number } | null;
  translationProgress: { translated: number; total: number };
}

export function useEditorCore(): EditorCoreState {
  const { t } = useTranslation();
  const selectedItem = useAppStore((s) => s.selectedItem);
  const selectedId = useAppStore((s) => s.selectedId);
  const language = useAppStore((s) => s.language);
  const targetLang = useAppStore((s) => s.targetLang);
  const updateItemTranslation = useAppStore((s) => s.updateItemTranslation);
  const setSelectedById = useAppStore((s) => s.setSelectedById);
  const allItems = useAppStore((s) => s.allItems);
  const items = useAppStore((s) => s.items);
  const dataConfigs = useAppStore((s) => s.dataConfigs);

  const translationProgress = useMemo(() => computeTranslationProgress(allItems), [allItems]);
  const [localTrans, setLocalTrans] = useState("");
  const [isSaving, setIsSaving] = useState(false);
  const [isSearching, setIsSearching] = useState(false);
  const [matches, setMatches] = useState<HeuristicMatchDTO[]>([]);
  const [isTranslating, setIsTranslating] = useState(false);
  const [aliasResult, setAliasResult] = useState<AliasCheckResult | null>(null);
  const [spellResult, setSpellResult] = useState<SpellCheckResultDto | null>(null);
  const [selectedFaultIdx, setSelectedFaultIdx] = useState<number | null>(null);
  const [suggestions, setSuggestions] = useState<string[]>([]);
  const spellTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const fieldSizeWarning = useMemo(() => {
    if (!selectedItem || !dataConfigs?.field_size_ref || !localTrans) return null;
    const key = `${selectedItem.record_sig}:${selectedItem.field_sig}`.toUpperCase();
    const info = dataConfigs.field_size_ref[key];
    if (!info) return null;
    const byteLen = new TextEncoder().encode(localTrans).length;
    return byteLen > info.max_size ? { max: info.max_size, current: byteLen } : null;
  }, [selectedItem, dataConfigs, localTrans]);

  useEffect(() => {
    setLocalTrans(selectedItem?.translation || "");
    setMatches([]);
    setAliasResult(null);
    setSpellResult(null);
    setSelectedFaultIdx(null);
    setSuggestions([]);
    if (selectedItem) checkAliases(selectedItem.id).then(setAliasResult).catch(() => {});
  }, [selectedId]);

  // 拼写检查（防抖 500ms）
  const doSpellCheck = useCallback(async (text: string) => {
    if (!text) { setSpellResult(null); return; }
    try {
      const result = await spellCheckText(text);
      setSpellResult(result.active ? result : null);
    } catch { /* 静默 */ }
  }, []);

  useEffect(() => {
    if (spellTimerRef.current) clearTimeout(spellTimerRef.current);
    spellTimerRef.current = setTimeout(() => doSpellCheck(localTrans), 500);
    return () => { if (spellTimerRef.current) clearTimeout(spellTimerRef.current); };
  }, [localTrans, doSpellCheck]);

  const handleSelectFault = useCallback(async (index: number) => {
    if (!spellResult) return;
    setSelectedFaultIdx(index);
    try { setSuggestions(await spellCheckSuggestions(spellResult.faults[index].word)); }
    catch { setSuggestions([]); }
  }, [spellResult]);

  const handleApplySuggestion = useCallback((suggestion: string) => {
    if (selectedFaultIdx === null || !spellResult) return;
    const fault = spellResult.faults[selectedFaultIdx];
    setLocalTrans(replaceUtf8ByteRange(localTrans, fault.start_byte, fault.end_byte, suggestion));
    setSelectedFaultIdx(null);
    setSuggestions([]);
  }, [selectedFaultIdx, spellResult, localTrans]);

  const handleIgnoreWord = useCallback(async (word: string) => {
    try {
      await spellCheckIgnore(word);
      setSpellResult((prev) => prev ? { ...prev, faults: prev.faults.filter((f) => f.word !== word) } : null);
      setSelectedFaultIdx(null);
      setSuggestions([]);
      toast.success(t("spellcheck.wordIgnored", { defaultValue: "Word added to ignore list" }));
    } catch (e: any) { toast.error(`${t("spellcheck.ignoreFailed")}: ${e}`); }
  }, []);

  const handleSave = useCallback(async () => {
    if (selectedId === null || !selectedItem) return;
    setIsSaving(true);
    try {
      await updateTranslation(selectedItem.id, localTrans);
      updateItemTranslation(selectedItem.id, localTrans);
      toast.success(t("editor.translationSaved"));
    } catch (e: any) { toast.error(`${t("editor.saveFailed")}: ${e}`); }
    finally { setIsSaving(false); }
  }, [selectedId, selectedItem, localTrans, updateItemTranslation]);

  const handleHeuristicSearch = useCallback(async () => {
    if (!selectedItem || selectedItem.status === "translated") return;
    setIsSearching(true);
    try {
      const results = await heuristicSearch({ source: selectedItem.source, min_similarity: 0.4, max_results: 5 });
      setMatches(results);
      if (results.length === 0) toast(t("editor.noSimilarFound"));
    } catch (e: any) { toast.error(`${t("editor.searchFailed")}: ${e}`); }
    finally { setIsSearching(false); }
  }, [selectedItem]);

  const handleTranslate = useCallback(async () => {
    if (!selectedItem) return;
    setIsTranslating(true);
    try {
      const result = await translateString({ text: selectedItem.source, source_lang: language, target_lang: targetLang });
      setLocalTrans(result);
      toast.success(t("editor.machineTranslationDone"));
    } catch (e: any) { toast.error(`${t("editor.translationFailed")}: ${e}`); }
    finally { setIsTranslating(false); }
  }, [selectedItem, language, targetLang]);

  const jumpToUntranslated = useCallback((direction: "next" | "prev") => {
    if (!selectedId || items.length === 0) return;
    const currentIdx = items.findIndex((i) => i.id === selectedId);
    if (currentIdx === -1) return;
    const step = direction === "next" ? 1 : -1;
    for (let i = currentIdx + step; i >= 0 && i < items.length; i += step) {
      if (!items[i].translation) { setSelectedById(items[i].id); return; }
    }
    toast(t("editor.noMoreUntranslated"), { icon: "ℹ️" });
  }, [selectedId, items, setSelectedById]);

  const applyMatch = (translation: string) => {
    setLocalTrans(translation);
    toast.success(t("editor.translationCopied"));
  };

  return { localTrans, setLocalTrans, isSaving, handleSave, matches, isSearching, handleHeuristicSearch, isTranslating, handleTranslate, aliasResult, spellResult, selectedFaultIdx, suggestions, handleSelectFault, handleApplySuggestion, handleIgnoreWord, jumpToUntranslated, applyMatch, fieldSizeWarning, translationProgress };
}
```

- [ ] **Step 2: 创建 EditorPanel/index.tsx 路由**

```tsx
// ui/src/components/EditorPanel/index.tsx
import { useAppStore } from "../../stores/appStore";
import { EditorModal } from "./EditorModal";
import { EditorSidebar } from "./EditorSidebar";
import { EditorInline } from "./EditorInline";

export type EditorMode = "modal" | "sidebar" | "inline";

export interface EditorPanelProps {
  open: boolean;
  onClose: () => void;
}

export function EditorDialog({ open, onClose }: EditorPanelProps) {
  const editorMode = useAppStore((s) => s.editorMode);

  switch (editorMode) {
    case "sidebar":
      return <EditorSidebar open={open} onClose={onClose} />;
    case "inline":
      return <EditorInline open={open} onClose={onClose} />;
    default:
      return <EditorModal open={open} onClose={onClose} />;
  }
}
```

- [ ] **Step 3: 运行类型检查**

Run: `cd ui && npx tsc --noEmit`
Expected: PASS（EditorModal/EditorSidebar/EditorInline 尚未创建，index.tsx 会报错 — 先注释掉未创建的导入）

- [ ] **Step 4: 提交**

```bash
git add ui/src/components/EditorPanel/
git commit -m "refactor(ui): extract EditorCore shared logic from EditorPanel"
```

### Task 5: EditorModal 模式

**Files:**
- Create: `ui/src/components/EditorPanel/EditorModal.tsx`

- [ ] **Step 1: 创建 EditorModal**

将现有 `EditorPanel.tsx` 的渲染逻辑迁移到 `EditorModal.tsx`，使用 `useEditorCore()` hook 替代本地状态。保持 Modal xl 包装和所有现有 UI 不变。

- [ ] **Step 2: 更新 index.tsx 导入**

取消注释 `EditorModal` 导入，验证类型检查通过。

- [ ] **Step 3: 替换 App.tsx 中的导入**

将 `import { EditorDialog } from "./components/EditorPanel"` 改为 `import { EditorDialog } from "./components/EditorPanel/index"`。

- [ ] **Step 4: 运行类型检查和测试**

Run: `cd ui && npx tsc --noEmit && npm run test`
Expected: PASS

- [ ] **Step 5: 提交**

```bash
git add ui/src/components/EditorPanel/EditorModal.tsx ui/src/components/EditorPanel/index.tsx ui/src/App.tsx
git commit -m "feat(ui): add EditorModal mode (migrated from EditorPanel)"
```

### Task 6: EditorSidebar 模式

**Files:**
- Create: `ui/src/components/EditorPanel/EditorSidebar.tsx`

- [ ] **Step 1: 创建 EditorSidebar**

侧栏模式：右侧固定 40% 宽度，不使用 Modal。内容与 EditorModal 相同但布局为垂直分割（源文上方、译文下方、操作按钮侧边）。

```tsx
// ui/src/components/EditorPanel/EditorSidebar.tsx
import { useEditorCore, highlightTags } from "./EditorCore";
import { useAppStore } from "../../stores/appStore";
import { useTranslation } from "react-i18next";
import { Button, Textarea, Badge, ProgressBar } from "../ui";
import { Save, Search, Languages, ArrowRight, Copy, ArrowUp, ArrowDown, Sparkles, X } from "lucide-react";
import type { EditorPanelProps } from "./index";

export function EditorSidebar({ open, onClose }: EditorPanelProps) {
  const { t } = useTranslation();
  const selectedItem = useAppStore((s) => s.selectedItem);
  const core = useEditorCore();

  if (!open || !selectedItem) return null;

  return (
    <div className="editor-sidebar">
      <div className="editor-sidebar-header">
        <span className="editor-sidebar-title">
          #{selectedItem.id} {selectedItem.record_sig}:{selectedItem.field_sig}
        </span>
        <button className="editor-sidebar-close" onClick={onClose}><X size={16} /></button>
      </div>
      {/* 元数据行 */}
      <div className="editor-meta-row">
        <span className="editor-meta-tag"><span className="editor-meta-label">FormID:</span><span className="editor-meta-value mono">{selectedItem.form_id}</span></span>
        <span className="editor-meta-tag"><span className="editor-meta-label">Rec:</span><span className="editor-meta-value">{selectedItem.record_sig}</span></span>
        <Badge variant={selectedItem.status === "translated" ? "translated" : "incomplete"}>{selectedItem.status}</Badge>
      </div>
      {/* 源文 */}
      <div className="editor-source">
        <label>{t("common.source")}</label>
        <div className="editor-source-text" dangerouslySetInnerHTML={{ __html: highlightTags(selectedItem.source) }} />
      </div>
      {/* 译文 */}
      <div className="editor-translation">
        <label>{t("common.translation")}</label>
        <Textarea value={core.localTrans} onChange={(e) => core.setLocalTrans(e.target.value)} rows={6} className="editor-textarea" autoFocus />
      </div>
      {/* 操作按钮 */}
      <div className="editor-sidebar-actions">
        <Button size="sm" onClick={core.handleSave} loading={core.isSaving} icon={<Save size={14} />}>{t("editor.save")}</Button>
        <Button size="sm" variant="ghost" onClick={core.handleHeuristicSearch} disabled={core.isSearching}><Search size={14} /></Button>
        <Button size="sm" variant="ghost" onClick={core.handleTranslate} disabled={core.isTranslating}><Languages size={14} /></Button>
        <Button size="sm" variant="ghost" onClick={() => core.jumpToUntranslated("prev")}><ArrowUp size={14} /></Button>
        <Button size="sm" variant="ghost" onClick={() => core.jumpToUntranslated("next")}><ArrowDown size={14} /></Button>
      </div>
      {/* 进度条 */}
      <ProgressBar value={core.translationProgress.translated} max={core.translationProgress.total} variant="gradient" size="sm" showLabel />
    </div>
  );
}
```

- [ ] **Step 2: 添加 EditorSidebar CSS**

在 `App.css` 中添加：
```css
.editor-sidebar {
  width: 40%;
  min-width: 300px;
  border-left: 1px solid var(--color-border);
  display: flex;
  flex-direction: column;
  overflow-y: auto;
  padding: 12px;
}
```

- [ ] **Step 3: 运行类型检查**

Run: `cd ui && npx tsc --noEmit`
Expected: PASS

- [ ] **Step 4: 提交**

```bash
git add ui/src/components/EditorPanel/EditorSidebar.tsx ui/src/App.css
git commit -m "feat(ui): add EditorSidebar mode for continuous editing"
```

### Task 7: EditorInline 模式

**Files:**
- Create: `ui/src/components/EditorPanel/EditorInline.tsx`

- [ ] **Step 1: 创建 EditorInline**

内联模式：在选中行下方展开编辑区（~120px 高度）。使用 `useEditorCore()` hook。

```tsx
// ui/src/components/EditorPanel/EditorInline.tsx
import { useEditorCore, highlightTags } from "./EditorCore";
import { useAppStore } from "../../stores/appStore";
import { useTranslation } from "react-i18next";
import { Button, Textarea, Badge } from "../ui";
import { Save, ArrowUp, ArrowDown } from "lucide-react";
import type { EditorPanelProps } from "./index";

export function EditorInline({ open, onClose }: EditorPanelProps) {
  const { t } = useTranslation();
  const selectedItem = useAppStore((s) => s.selectedItem);
  const core = useEditorCore();

  if (!open || !selectedItem) return null;

  return (
    <div className="editor-inline">
      <div className="editor-inline-row">
        <div className="editor-inline-source" dangerouslySetInnerHTML={{ __html: highlightTags(selectedItem.source) }} />
        <Textarea
          value={core.localTrans}
          onChange={(e) => core.setLocalTrans(e.target.value)}
          onKeyDown={(e) => { if (e.ctrlKey && e.key === "Enter") core.handleSave(); }}
          rows={3}
          className="editor-inline-textarea"
          autoFocus
        />
        <div className="editor-inline-actions">
          <Button size="xs" onClick={core.handleSave} loading={core.isSaving} icon={<Save size={12} />} />
          <Button size="xs" variant="ghost" onClick={() => core.jumpToUntranslated("prev")}><ArrowUp size={12} /></Button>
          <Button size="xs" variant="ghost" onClick={() => core.jumpToUntranslated("next")}><ArrowDown size={12} /></Button>
          <Badge variant={selectedItem.status === "translated" ? "translated" : "incomplete"} size="sm">{selectedItem.status}</Badge>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: 添加 EditorInline CSS**

```css
.editor-inline {
  border-top: 1px solid var(--color-border);
  background: var(--color-surface);
  padding: 8px 12px;
  animation: slideDown 200ms ease-out;
}
@keyframes slideDown {
  from { max-height: 0; opacity: 0; }
  to { max-height: 200px; opacity: 1; }
}
```

- [ ] **Step 3: 运行类型检查**

Run: `cd ui && npx tsc --noEmit`
Expected: PASS

- [ ] **Step 4: 提交**

```bash
git add ui/src/components/EditorPanel/EditorInline.tsx ui/src/App.css
git commit -m "feat(ui): add EditorInline mode for inline editing"
```

### Task 8: 编辑器模式切换与持久化

**Files:**
- Modify: `ui/src/stores/appStore.ts` (新增 editorMode 状态)
- Modify: `ui/src/components/GroupedMenuBar.tsx` (添加模式切换按钮)

- [ ] **Step 1: 在 appStore 中添加 editorMode**

在 `AppState` 接口中添加：
```typescript
editorMode: "modal" | "sidebar" | "inline";
setEditorMode: (mode: "modal" | "sidebar" | "inline") => void;
```

在 store 实现中添加：
```typescript
editorMode: "modal",
setEditorMode: (editorMode) => {
  set({ editorMode });
  saveConfig({ editor_mode: editorMode }).catch(() => {});
},
```

在 `reset()` 中添加 `editorMode: "modal"`。

在 `App.tsx` 的 `loadConfig` 中添加：
```typescript
if (cfg.editor_mode) useAppStore.getState().setEditorMode(cfg.editor_mode);
```

- [ ] **Step 2: 在 GroupedMenuBar 视图菜单中添加模式切换**

```typescript
{ label: t("editor.modeModal"), onClick: () => setEditorMode("modal"), shortcut: "Ctrl+1" },
{ label: t("editor.modeSidebar"), onClick: () => setEditorMode("sidebar"), shortcut: "Ctrl+2" },
{ label: t("editor.modeInline"), onClick: () => setEditorMode("inline"), shortcut: "Ctrl+3" },
```

- [ ] **Step 3: 在 App.tsx 中添加 Ctrl+1/2/3 快捷键**

在全局 `useEffect` 键盘处理中添加：
```typescript
if (e.ctrlKey && e.key === "1") { e.preventDefault(); useAppStore.getState().setEditorMode("modal"); }
if (e.ctrlKey && e.key === "2") { e.preventDefault(); useAppStore.getState().setEditorMode("sidebar"); }
if (e.ctrlKey && e.key === "3") { e.preventDefault(); useAppStore.getState().setEditorMode("inline"); }
```

- [ ] **Step 4: 运行类型检查和测试**

Run: `cd ui && npx tsc --noEmit && npm run test`
Expected: PASS

- [ ] **Step 5: 手动验证**

Run: `.\dev.ps1`
验证：Ctrl+1/2/3 切换模式，编辑器从弹窗变为侧栏/内联，重启后模式保持。

- [ ] **Step 6: 删除旧 EditorPanel.tsx**

确认新 EditorPanel/ 目录完全替代后，删除 `ui/src/components/EditorPanel.tsx`。

- [ ] **Step 7: 提交**

```bash
git add ui/src/stores/appStore.ts ui/src/App.tsx ui/src/components/GroupedMenuBar.tsx
git rm ui/src/components/EditorPanel.tsx
git commit -m "feat(ui): add editor mode switching (Ctrl+1/2/3) with persistence"
```

---

## Phase 3：面板系统

### Task 9: 安装 react-split-pane

**Files:**
- Modify: `ui/package.json`

- [ ] **Step 1: 安装依赖**

Run: `cd ui && npm install react-split-pane`
Expected: 成功安装，package.json 更新

- [ ] **Step 2: 验证安装**

Run: `cd ui && npx tsc --noEmit`
Expected: PASS

- [ ] **Step 3: 提交**

```bash
git add ui/package.json ui/package-lock.json
git commit -m "chore(ui): add react-split-pane dependency"
```

### Task 10: SplitPaneLayout 组件

**Files:**
- Create: `ui/src/components/SplitPaneLayout.tsx`
- Modify: `ui/src/App.tsx` (用 SplitPaneLayout 替换固定布局)

- [ ] **Step 1: 创建 SplitPaneLayout**

```tsx
// ui/src/components/SplitPaneLayout.tsx
import { ReactNode } from "react";
import SplitPane from "react-split-pane";

interface SplitPaneLayoutProps {
  children: ReactNode;           // StringTable
  rightPanel?: ReactNode | null; // 右侧面板
  bottomPanel?: ReactNode | null;// 底部面板
  rightPanelVisible: boolean;
  bottomPanelVisible: boolean;
  rightPanelSize?: number;
  bottomPanelSize?: number;
  onRightPanelResize?: (size: number) => void;
  onBottomPanelResize?: (size: number) => void;
}

export function SplitPaneLayout({
  children,
  rightPanel,
  bottomPanel,
  rightPanelVisible,
  bottomPanelVisible,
  rightPanelSize = 400,
  bottomPanelSize = 300,
  onRightPanelResize,
  onBottomPanelResize,
}: SplitPaneLayoutProps) {
  const mainContent = rightPanelVisible && rightPanel ? (
    <SplitPane
      split="vertical"
      minSize={300}
      defaultSize={rightPanelSize}
      onChange={(size) => onRightPanelResize?.(size as number)}
      style={{ position: "relative" }}
    >
      {children}
      {rightPanel}
    </SplitPane>
  ) : children;

  if (bottomPanelVisible && bottomPanel) {
    return (
      <SplitPane
        split="horizontal"
        minSize={200}
        defaultSize={bottomPanelSize}
        onChange={(size) => onBottomPanelResize?.(size as number)}
        primary="second"
        style={{ position: "relative" }}
      >
        {mainContent}
        {bottomPanel}
      </SplitPane>
    );
  }

  return <>{mainContent}</>;
}
```

- [ ] **Step 2: 重构 App.tsx 布局**

将 `app-main` 中的 `app-table-area` + `app-bottom-panel` 用 `SplitPaneLayout` 包裹：

```tsx
<SplitPaneLayout
  rightPanel={rightPanelVisible ? <RightPanelContainer /> : null}
  bottomPanel={showBottomPanel ? <BottomPanelContent /> : null}
  rightPanelVisible={rightPanelVisible}
  bottomPanelVisible={showBottomPanel}
  rightPanelSize={panelLayout.rightPanelSize}
  bottomPanelSize={panelLayout.bottomPanelSize}
  onRightPanelResize={(size) => setPanelSize("right", size)}
  onBottomPanelResize={(size) => setPanelSize("bottom", size)}
>
  <div className="app-table-area">
    <StringTable />
  </div>
</SplitPaneLayout>
```

- [ ] **Step 3: 运行类型检查**

Run: `cd ui && npx tsc --noEmit`
Expected: PASS

- [ ] **Step 4: 提交**

```bash
git add ui/src/components/SplitPaneLayout.tsx ui/src/App.tsx
git commit -m "feat(ui): add SplitPaneLayout with react-split-pane"
```

### Task 11: DockablePanel 与 RightPanelContainer

**Files:**
- Create: `ui/src/components/DockablePanel.tsx`
- Create: `ui/src/components/RightPanelContainer.tsx`
- Modify: `ui/src/stores/appStore.ts` (添加 panelLayout 状态)

- [ ] **Step 1: 在 appStore 中添加 panelLayout**

```typescript
// 新增状态
activeRightPanel: ActivePanel;  // 右侧面板中当前活动的面板
panelLayout: {
  rightPanelSize: number;
  bottomPanelSize: number;
  rightPanelVisible: boolean;
};

// 新增操作
setActiveRightPanel: (panel: ActivePanel) => void;
setPanelSize: (target: "right" | "bottom", size: number) => void;
toggleRightPanel: () => void;
```

- [ ] **Step 2: 创建 DockablePanel**

```tsx
// ui/src/components/DockablePanel.tsx
import { ReactNode } from "react";
import { X } from "lucide-react";

interface DockablePanelProps {
  title: string;
  icon?: ReactNode;
  onClose: () => void;
  children: ReactNode;
}

export function DockablePanel({ title, icon, onClose, children }: DockablePanelProps) {
  return (
    <div className="dockable-panel">
      <div className="dockable-panel-header">
        <span className="dockable-panel-title">
          {icon && <span className="dockable-panel-icon">{icon}</span>}
          {title}
        </span>
        <button className="dockable-panel-close" onClick={onClose} aria-label="Close panel">
          <X size={14} />
        </button>
      </div>
      <div className="dockable-panel-content">
        {children}
      </div>
    </div>
  );
}
```

- [ ] **Step 3: 创建 RightPanelContainer**

```tsx
// ui/src/components/RightPanelContainer.tsx
import { Suspense, lazy } from "react";
import { useAppStore } from "../stores/appStore";
import { DockablePanel } from "./DockablePanel";
import { Loader } from "lucide-react";
import { useTranslation } from "react-i18next";

const BsaBrowser = lazy(() => import("./BsaBrowser").then(m => ({ default: m.BsaBrowser })));
const PexPanel = lazy(() => import("./PexPanel").then(m => ({ default: m.PexPanel })));
const FuzPanel = lazy(() => import("./FuzPanel").then(m => ({ default: m.FuzPanel })));
const EspComparePanel = lazy(() => import("./EspComparePanel").then(m => ({ default: m.EspComparePanel })));

const PANEL_TITLES: Record<string, string> = {
  bsa: "bsa.title",
  pex: "pex.title",
  fuz: "fuz.title",
  espCompare: "espCompare.title",
};

const PANEL_COMPONENTS: Record<string, React.ComponentType> = {
  bsa: BsaBrowser,
  pex: PexPanel,
  fuz: FuzPanel,
  espCompare: EspComparePanel,
};

export function RightPanelContainer() {
  const { t } = useTranslation();
  const activeRightPanel = useAppStore((s) => s.activeRightPanel);
  const setActiveRightPanel = useAppStore((s) => s.setActiveRightPanel);

  if (!activeRightPanel) return null;

  const PanelComponent = PANEL_COMPONENTS[activeRightPanel];
  if (!PanelComponent) return null;

  return (
    <DockablePanel
      title={t(PANEL_TITLES[activeRightPanel] || activeRightPanel)}
      onClose={() => setActiveRightPanel(null)}
    >
      <Suspense fallback={<div className="modal-loading"><Loader size={24} /></div>}>
        <PanelComponent />
      </Suspense>
    </DockablePanel>
  );
}
```

- [ ] **Step 4: 添加 DockablePanel CSS**

```css
.dockable-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}
.dockable-panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 8px;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-surface);
  min-height: 32px;
}
.dockable-panel-content {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}
```

- [ ] **Step 5: 运行类型检查**

Run: `cd ui && npx tsc --noEmit`
Expected: PASS

- [ ] **Step 6: 提交**

```bash
git add ui/src/components/DockablePanel.tsx ui/src/components/RightPanelContainer.tsx ui/src/stores/appStore.ts ui/src/App.css
git commit -m "feat(ui): add DockablePanel and RightPanelContainer"
```

### Task 12: 集成右侧面板到 App

**Files:**
- Modify: `ui/src/App.tsx`
- Modify: `ui/src/components/GroupedMenuBar.tsx` (工具菜单使用 activeRightPanel)

- [ ] **Step 1: 更新 App.tsx 使用 SplitPaneLayout + RightPanelContainer**

将 Task 10 的 SplitPaneLayout 集成实际的 RightPanelContainer。工具面板（BSA/PEX/FUZ/Compare）不再使用 Modal，改为通过 `activeRightPanel` 停靠到右侧。

保留其余面板（Batch/Dialog/MCM/Finalize/DataConfigs）为 Modal。

- [ ] **Step 2: 更新 GroupedMenuBar 工具菜单**

BSA/PEX/FUZ/Compare 的菜单项改为设置 `activeRightPanel` 而非 `activePanel`。

- [ ] **Step 3: 运行类型检查和手动验证**

Run: `cd ui && npx tsc --noEmit`
Run: `.\dev.ps1`
验证：点击 BSA 工具按钮 → 右侧面板出现 BSA 浏览器，表格宽度自动缩小。

- [ ] **Step 4: 提交**

```bash
git add ui/src/App.tsx ui/src/components/GroupedMenuBar.tsx
git commit -m "feat(ui): integrate right-side dockable panels (BSA/PEX/FUZ/Compare)"
```

---

## Phase 4：底部面板

### Task 13: 更新 BottomTabId 类型

**Files:**
- Modify: `ui/src/stores/appStore.ts`

- [ ] **Step 1: 更新 BottomTabId 类型**

将 10 个 tab 合并为 5 个：

```typescript
export type BottomTabId =
  | "overview"    // 合并 home
  | "vocabulary"  // 保持
  | "log"         // 保持
  | "explorer"    // 合并 heuristic + espTree + quests
  | "header";     // 合并 headerProc + headerWizard
```

- [ ] **Step 2: 更新所有引用 BottomTabId 的地方**

更新 `appStore.ts` 中的 `activeBottomTab` 默认值、`reset()` 中的值。更新 `App.tsx` 中的 tab 渲染逻辑。

- [ ] **Step 3: 运行类型检查**

Run: `cd ui && npx tsc --noEmit`
Expected: 会有编译错误，因为旧 tab ID 不再有效。修复所有引用。

- [ ] **Step 4: 提交**

```bash
git add ui/src/stores/appStore.ts ui/src/App.tsx
git commit -m "refactor(ui): consolidate BottomTabId from 10 to 5 tabs"
```

### Task 14: ExplorerTab 和 HeaderTab

**Files:**
- Create: `ui/src/components/bottom/ExplorerTab.tsx`
- Create: `ui/src/components/bottom/HeaderTab.tsx`
- Modify: `ui/src/App.tsx`

- [ ] **Step 1: 创建 ExplorerTab**

```tsx
// ui/src/components/bottom/ExplorerTab.tsx
import { useState } from "react";
import { HeuristicPanel } from "./HeuristicPanel";
import { EspTreePanel } from "./EspTreePanel";
import { QuestsPanel } from "./QuestsPanel";
import { useTranslation } from "react-i18next";

type SubTab = "heuristic" | "espTree" | "quests";

export function ExplorerTab() {
  const { t } = useTranslation();
  const [subTab, setSubTab] = useState<SubTab>("heuristic");

  return (
    <div className="explorer-tab">
      <div className="explorer-sub-tabs">
        {(["heuristic", "espTree", "quests"] as const).map((tab) => (
          <button
            key={tab}
            className={`explorer-sub-tab ${subTab === tab ? "active" : ""}`}
            onClick={() => setSubTab(tab)}
          >
            {t(`bottomTabs.${tab}`)}
          </button>
        ))}
      </div>
      <div className="explorer-content">
        {subTab === "heuristic" && <HeuristicPanel />}
        {subTab === "espTree" && <EspTreePanel />}
        {subTab === "quests" && <QuestsPanel />}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: 创建 HeaderTab**

```tsx
// ui/src/components/bottom/HeaderTab.tsx
import { useState } from "react";
import { HeaderProcessorPanel } from "./HeaderProcessorPanel";
import { HeaderWizardPanel } from "./HeaderWizardPanel";
import { useTranslation } from "react-i18next";

type SubTab = "processor" | "wizard";

export function HeaderTab() {
  const { t } = useTranslation();
  const [subTab, setSubTab] = useState<SubTab>("processor");

  return (
    <div className="header-tab">
      <div className="header-sub-tabs">
        <button className={`header-sub-tab ${subTab === "processor" ? "active" : ""}`} onClick={() => setSubTab("processor")}>
          {t("bottomTabs.headerProc")}
        </button>
        <button className={`header-sub-tab ${subTab === "wizard" ? "active" : ""}`} onClick={() => setSubTab("wizard")}>
          {t("bottomTabs.headerWizard")}
        </button>
      </div>
      <div className="header-content">
        {subTab === "processor" && <HeaderProcessorPanel />}
        {subTab === "wizard" && <HeaderWizardPanel />}
      </div>
    </div>
  );
}
```

- [ ] **Step 3: 更新 App.tsx 底部面板渲染**

将 10 个 tab 渲染替换为 5 个 tab 渲染：

```tsx
{activeBottomTab === "overview" && <SidePanel />}
{activeBottomTab === "vocabulary" && <VocabularyPanel />}
{activeBottomTab === "log" && <LogPanel />}
{activeBottomTab === "explorer" && <ExplorerTab />}
{activeBottomTab === "header" && <HeaderTab />}
```

Tab 按钮添加 lucide-react 图标。

- [ ] **Step 4: 添加底部面板 CSS**

- [ ] **Step 5: 运行类型检查和手动验证**

Run: `cd ui && npx tsc --noEmit && npm run test`
Run: `.\dev.ps1`
验证：5 个 tab 正确切换，Explorer 子 tab 工作正常，Header 子 tab 工作正常。

- [ ] **Step 6: 提交**

```bash
git add ui/src/components/bottom/ExplorerTab.tsx ui/src/components/bottom/HeaderTab.tsx ui/src/App.tsx
git commit -m "feat(ui): consolidate bottom panel to 5 tabs with ExplorerTab and HeaderTab"
```

---

## Phase 5：视觉统一

### Task 15: CSS 变量与颜色语义系统

**Files:**
- Modify: `ui/src/App.css` (或主题 CSS 文件)

- [ ] **Step 1: 定义语义化 CSS 变量**

在每个主题的选择器中添加：

```css
[data-theme="obsidian"] {
  --color-primary: #3b82f6;
  --color-success: #22c55e;
  --color-warning: #eab308;
  --color-danger: #ef4444;
  --color-info: #6b7280;
  --color-surface: #1e1e2e;
  --color-border: #313244;
  --color-hover: #2a2a3c;
}
```

为 slate、light、auto 主题定义对应的值。

- [ ] **Step 2: 替换硬编码颜色值**

搜索 CSS 中的硬编码颜色值，替换为 CSS 变量。优先替换：选中行背景、边框、面板背景。

- [ ] **Step 3: 运行手动验证**

Run: `.\dev.ps1`
验证：4 个主题下颜色一致性。

- [ ] **Step 4: 提交**

```bash
git add ui/src/App.css
git commit -m "feat(ui): add semantic CSS color variables for all themes"
```

### Task 16: 动画与过渡

**Files:**
- Modify: `ui/src/App.css`

- [ ] **Step 1: 添加过渡动画**

```css
/* 面板打开/关闭 */
.dockable-panel { animation: fadeSlideIn 200ms ease-out; }
@keyframes fadeSlideIn {
  from { opacity: 0; transform: translateX(20px); }
  to { opacity: 1; transform: translateX(0); }
}

/* Tab 切换 */
.bottom-panel-content { animation: fadeIn 150ms linear; }
@keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }

/* 选中行高亮 */
.string-table-row { transition: background-color 100ms linear; }

/* 菜单下拉 */
.menubar-group-panel { animation: menuSlideDown 150ms ease-out; }
@keyframes menuSlideDown {
  from { opacity: 0; transform: translateY(-4px); }
  to { opacity: 1; transform: translateY(0); }
}
```

- [ ] **Step 2: 提交**

```bash
git add ui/src/App.css
git commit -m "feat(ui): add panel and menu transition animations"
```

### Task 17: 字体与间距统一

**Files:**
- Modify: `ui/src/App.css`

- [ ] **Step 1: 统一字体族**

```css
:root {
  --font-sans: Inter, system-ui, -apple-system, sans-serif;
  --font-mono: "JetBrains Mono", Consolas, "Courier New", monospace;
}
body { font-family: var(--font-sans); }
.mono, .editor-meta-value.mono { font-family: var(--font-mono); }
```

- [ ] **Step 2: 调整间距**

```css
.bottom-panel-tabs .bottom-tab { height: 32px; padding: 0 12px; }
.dockable-panel-content { padding: 12px; }
.string-table-row { height: 32px; } /* 从 28px 增大 */
.btn { height: 32px; } /* 从 28px 增大 */
```

- [ ] **Step 3: 提交**

```bash
git add ui/src/App.css
git commit -m "feat(ui): unify font families and spacing to 32px baseline"
```

### Task 18: 快捷键增强

**Files:**
- Modify: `ui/src/App.tsx`

- [ ] **Step 1: 添加新快捷键**

在 App.tsx 的全局 `useEffect` 键盘处理中添加：

```typescript
// Ctrl+\ 切换右侧面板
if (e.ctrlKey && e.key === "\\") {
  e.preventDefault();
  useAppStore.getState().toggleRightPanel();
}
// Ctrl+Shift+L 聚焦日志
if (e.ctrlKey && e.shiftKey && e.key === "L") {
  e.preventDefault();
  useAppStore.getState().setActiveBottomTab("log");
}
// Ctrl+Shift+B 聚焦底部面板
if (e.ctrlKey && e.shiftKey && e.key === "B") {
  e.preventDefault();
  useAppStore.getState().toggleBottomPanel();
}
// F2 内联编辑
if (e.key === "F2" && useAppStore.getState().selectedId !== null) {
  e.preventDefault();
  useAppStore.getState().setEditorMode("inline");
  useAppStore.getState().setEditorOpen(true);
}
```

- [ ] **Step 2: 增强 Escape 链**

确保 Escape 链按以下顺序执行：编辑器 → 右侧面板 → 底面板 → 取消选中。

- [ ] **Step 3: 运行类型检查**

Run: `cd ui && npx tsc --noEmit`
Expected: PASS

- [ ] **Step 4: 手动验证**

Run: `.\dev.ps1`
验证：所有新快捷键工作正常。

- [ ] **Step 5: 提交**

```bash
git add ui/src/App.tsx
git commit -m "feat(ui): add keyboard shortcuts (Ctrl+\\, Ctrl+Shift+L/B, F2)"
```

---

## 自审结果

**1. 规范覆盖：**
- Phase 1 (顶栏重组): Task 1-3 ✅
- Phase 2 (编辑体验): Task 4-8 ✅
- Phase 3 (面板系统): Task 9-12 ✅
- Phase 4 (底部面板): Task 13-14 ✅
- Phase 5 (视觉统一): Task 15-18 ✅

**2. 占位符扫描：** Task 2 有 "TODO: 菜单定义将在 Step 2 添加" 注释，这是合理的临时标记（该步骤中完成）。

**3. 类型一致性：**
- `EditorMode = "modal" | "sidebar" | "inline"` 在 Task 8 定义，Task 4-7 使用 ✅
- `BottomTabId` 在 Task 13 更新，Task 14 使用 ✅
- `activeRightPanel` 在 Task 11 定义，Task 12 使用 ✅
- `panelLayout` 在 Task 11 定义，Task 10/12 使用 ✅
