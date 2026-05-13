import { useState, useEffect, useCallback, ReactElement } from "react";
import { List } from "react-window";
import { useAppStore } from "../stores/appStore";
import { ArrowUpDown, Replace, Edit3, Copy, Filter } from "lucide-react";
import type { SkyStringDTO } from "../api/strings";
import { useTranslation } from "react-i18next";
import { Input, Button, Spinner } from "./ui";
import { ContextMenu } from "./ContextMenu";

// ============================================================================
// StringTable 组件 - 虚拟滚动字符串表格
// ============================================================================
// 
// 职责：
//   - 使用 react-window v2 实现高性能虚拟滚动表格
//   - 显示 ESP 文件中的所有字符串条目（STRINGS/DLSTRINGS/ILSTRINGS）
//   - 支持搜索、过滤、排序、替换等操作
//   - 处理键盘导航（↑↓ 移动，Enter 编辑）
//   - 提供右键菜单快捷操作
//
// 核心特性：
//   - 虚拟滚动：只渲染可见行，支持 10K+ 条目无卡顿
//   - 搜索高亮：支持正则表达式，实时高亮匹配文本
//   - 状态指示：● 已翻译，◆ 已锁定，○ 未翻译
//   - 替换功能：支持正则表达式分组替换（$1, $2...）
//   - 列表过滤：按 STRINGS/DLSTRINGS/ILSTRINGS 分类显示
//
// 性能优化：
//   - 虚拟滚动减少 DOM 节点数量
//   - 使用 overscanCount=20 预加载行
//   - 搜索高亮使用 dangerouslySetInnerHTML 避免重复转义
//   - 状态选择器精确订阅，避免不必要的重新渲染
//
// 键盘快捷键：
//   - ↑/↓：上下移动选中行
//   - Enter：打开编辑器编辑当前行
//   - Ctrl+C：复制源文本或翻译文本（右键菜单）
//   - F12：按 FormID 过滤（右键菜单）
//
// ============================================================================

// 虚拟表格行高度（像素），必须与 CSS 中的行高一致
const ROW_HEIGHT = 32;

// 虚拟行组件的数据接口
// 包含表格数据和事件回调，通过 react-window 的 rowProps 传递给每一行
interface RowData {
  items: SkyStringDTO[];           // 当前显示的字符串列表（已过滤）
  selectedId: number | null;       // 当前选中的字符串 ID
  filter: string;                  // 搜索过滤文本
  onSelect: (id: number) => void;  // 行点击事件
  onDoubleClick: (id: number) => void;  // 行双击事件（打开编辑器）
  onContextMenu: (e: React.MouseEvent, item: SkyStringDTO) => void;  // 右键菜单事件
}

// ============================================================================
// 工具函数：HTML 转义和搜索高亮
// ============================================================================

/**
 * 转义 HTML 特殊字符，防止 XSS 攻击
 * 
 * 转义规则：
 *   & → &amp;
 *   < → &lt;
 *   > → &gt;
 * 
 * 用途：在使用 dangerouslySetInnerHTML 前，先转义用户输入的文本
 * 
 * @param s - 原始字符串
 * @returns 转义后的 HTML 安全字符串
 * 
 * 示例：
 *   escapeHtml("<script>alert('xss')</script>")
 *   // 返回: "&lt;script&gt;alert('xss')&lt;/script&gt;"
 */
function escapeHtml(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

/**
 * 在文本中高亮搜索关键词
 * 
 * 功能：
 *   1. 转义 HTML 特殊字符（防止 XSS）
 *   2. 将搜索文本转换为正则表达式（支持正则语法）
 *   3. 用 <mark> 标签包装匹配的文本
 *   4. 返回 HTML 字符串，用于 dangerouslySetInnerHTML
 * 
 * 搜索特性：
 *   - 不区分大小写（gi 标志）
 *   - 支持正则表达式（如 "test|demo" 匹配 test 或 demo）
 *   - 自动转义正则特殊字符
 * 
 * @param text - 要高亮的文本
 * @param filter - 搜索关键词（支持正则表达式）
 * @returns HTML 字符串，包含 <mark> 标签的高亮部分
 * 
 * 示例：
 *   highlightText("Hello World", "world")
 *   // 返回: "Hello <mark class=\"search-highlight\">World</mark>"
 *   
 *   highlightText("test123demo", "\\d+")
 *   // 返回: "test<mark class=\"search-highlight\">123</mark>demo"
 */
function highlightText(text: string, filter: string): string {
  if (!filter) return escapeHtml(text);
  // 转义正则特殊字符，使用占位符避免冲突
  const escaped = filter.replace(/[.*+?^${}()|[\]\\]/g, '\\PLACEHOLDER');
  const regex = new RegExp(`(${escaped})`, 'gi');
  return escapeHtml(text).replace(regex, '<mark class="search-highlight">$1</mark>');
}

// ============================================================================
// VirtualRow 组件 - react-window v2 虚拟行渲染器
// ============================================================================
//
// 职责：
//   - 渲染单个表格行（由 react-window 的 List 组件调用）
//   - 处理行的选中、双击、右键菜单事件
//   - 显示字符串的状态、ID、源文本、翻译文本等信息
//   - 支持搜索高亮和悬停效果
//
// react-window v2 API：
//   - rowComponent：行渲染函数（替代 v1 的 children）
//   - rowProps：传递给每一行的数据对象
//   - ariaAttributes：无障碍属性（aria-posinset, aria-setsize, role）
//   - style：行的位置和大小（由 react-window 计算）
//
// 状态指示符：
//   ● (●) - 已翻译（translated）
//   ◆ (◆) - 已锁定（locked）
//   ○ (○) - 未翻译（untranslated）
//   + VMAD 标记 - 脚本字符串（is_vmad=true）
//
// 性能优化：
//   - 只渲染可见行，不渲染整个列表
//   - 使用 dangerouslySetInnerHTML 避免重复转义
//   - 使用 classList 操作 CSS 类，避免重新渲染
//
function VirtualRow(props: {
  ariaAttributes: {
    "aria-posinset": number;
    "aria-setsize": number;
    role: "listitem";
  };
  index: number;
  style: React.CSSProperties;
  items: SkyStringDTO[];
  selectedId: number | null;
  filter: string;
  onSelect: (id: number) => void;
  onDoubleClick: (id: number) => void;
  onContextMenu: (e: React.MouseEvent, item: SkyStringDTO) => void;
}): ReactElement | null {
  // 解构 props，获取行数据和事件处理函数
  const { index, style, items, selectedId, filter, onSelect, onDoubleClick, onContextMenu } = props;
  const item = items[index];
  
  // 如果行数据不存在，返回 null（虚拟滚动边界情况）
  if (!item) return null;

  // 判断当前行是否被选中
  const isSelected = selectedId === item.id;
  
  return (
    <div
      // react-window 计算的行位置和大小
      style={style}
      // CSS 类：status-{status} 用于样式，virtual-row-selected 用于选中状态
      className={`virtual-row status-${item.status} ${isSelected ? "virtual-row-selected" : ""}`}
      // 点击事件：选中该行
      onClick={() => onSelect(item.id)}
      // 双击事件：打开编辑器
      onDoubleClick={() => onDoubleClick(item.id)}
      // 右键菜单事件
      onContextMenu={(e) => onContextMenu(e, item)}
      // 鼠标进入：添加悬停效果（仅当未选中时）
      onMouseEnter={(e) => {
        if (!isSelected) {
          e.currentTarget.classList.add("virtual-row-hover");
        }
      }}
      // 鼠标离开：移除悬停效果
      onMouseLeave={(e) => {
        if (!isSelected) {
          e.currentTarget.classList.remove("virtual-row-hover");
        }
      }}
    >
      {/* 状态指示符列 */}
      <div className="row-cell row-cell-status-icon" title={`${item.record_sig}:${item.field_sig} #${item.form_id}`}>
        <span className={`status-dot status-${item.status}${item.is_vmad ? " status-vmad" : ""}`}>
          {/* 根据翻译状态显示不同的符号 */}
          {item.status === "translated" ? "●" : item.status === "locked" ? "◆" : "○"}
        </span>
      </div>
      
      {/* EDID 列：记录类型和字段名 */}
      <div className="row-cell row-cell-edid" title={`${item.record_sig}:${item.field_sig}`}>
        {item.record_sig}:{item.field_sig}
      </div>
      
      {/* ID 列：字符串 ID */}
      <div className="row-cell row-cell-id">{item.id}</div>
      
      {/* 源文本列：支持搜索高亮 */}
      <div className="row-cell text-cell source-text" title={item.source}>
        <span dangerouslySetInnerHTML={{ __html: highlightText(item.source, filter) }} />
      </div>
      
      {/* 翻译文本列：支持搜索高亮，未翻译时显示 "—" */}
      <div className="row-cell text-cell trans-text" title={item.translation}>
        <span dangerouslySetInnerHTML={{ __html: highlightText(item.translation || "—", filter) }} />
      </div>
      
      {/* LD 列：字符串长度差异（用于检测翻译质量） */}
      <div className="row-cell row-cell-ld">
        {item.ld > 0 ? item.ld : "—"}
      </div>
    </div>
  );
}

// ============================================================================
// StringTable 主组件
// ============================================================================
//
// 职责：
//   - 管理表格的整体状态和数据流
//   - 从 Zustand store 订阅数据和操作
//   - 处理键盘导航和右键菜单
//   - 渲染虚拟滚动列表和工具栏
//
// 数据流：
//   1. 从 store 订阅 allItems（完整数据）和 items（过滤后数据）
//   2. 用户操作（搜索、排序、过滤）更新 store
//   3. store 更新触发组件重新渲染
//   4. 虚拟滚动只渲染可见行
//
// 关键 hooks：
//   - useEffect 1：监听键盘事件（↑↓ 导航，Enter 编辑）
//   - useEffect 2：ESP 加载时自动加载字符串
//
export function StringTable() {
  // 国际化
  const { t } = useTranslation();
  
  // ========== 从 store 订阅状态 ==========
  // 文件路径和数据
  const espPath = useAppStore((s) => s.espPath);
  const allItems = useAppStore((s) => s.allItems);      // 完整数据集
  const items = useAppStore((s) => s.items);            // 过滤后的显示数据
  const isLoading = useAppStore((s) => s.isLoading);
  
  // 搜索和过滤
  const filter = useAppStore((s) => s.filter);          // 搜索关键词
  const replaceText = useAppStore((s) => s.replaceText); // 替换文本
  const listIndex = useAppStore((s) => s.listIndex);    // 列表类型过滤（0=STRINGS, 1=DLSTRINGS, 2=ILSTRINGS）
  
  // 选中和统计
  const selectedId = useAppStore((s) => s.selectedId);  // 当前选中的字符串 ID
  const total = useAppStore((s) => s.total);            // 总条目数
  const filtered = useAppStore((s) => s.filtered);      // 过滤后的条目数
  
  // ========== 从 store 订阅操作函数 ==========
  const loadAllStrings = useAppStore((s) => s.loadAllStrings);
  const setFilter = useAppStore((s) => s.setFilter);
  const setReplaceText = useAppStore((s) => s.setReplaceText);
  const setSort = useAppStore((s) => s.setSort);
  const setListIndex = useAppStore((s) => s.setListIndex);
  const setSelectedById = useAppStore((s) => s.setSelectedById);
  const selectNextRow = useAppStore((s) => s.selectNextRow);
  const selectPrevRow = useAppStore((s) => s.selectPrevRow);
  const replaceAll = useAppStore((s) => s.replaceAll);
  const openEditorForItem = useAppStore((s) => s.openEditorForItem);

  // 右键菜单状态
  const [ctxMenu, setCtxMenu] = useState<{ x: number; y: number; item: SkyStringDTO } | null>(null);

  /**
   * 右键菜单事件处理
   * 
   * 功能：
   *   1. 阻止默认右键菜单
   *   2. 选中被右键点击的行
   *   3. 显示上下文菜单
   */
  const handleContextMenu = useCallback((e: React.MouseEvent, item: SkyStringDTO) => {
    e.preventDefault();
    setSelectedById(item.id);
    setCtxMenu({ x: e.clientX, y: e.clientY, item });
  }, [setSelectedById]);

  /**
   * 键盘事件处理
   * 
   * 支持的快捷键：
   *   - ↑：选中上一行
   *   - ↓：选中下一行
   *   - Enter：打开编辑器编辑当前行
   */
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        selectNextRow();
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        selectPrevRow();
      } else if (e.key === "Enter" && selectedId !== null) {
        e.preventDefault();
        openEditorForItem(selectedId);
      }
    },
    [selectNextRow, selectPrevRow, selectedId, openEditorForItem]
  );

  /**
   * Hook 1：监听全局键盘事件
   * 
   * 功能：
   *   - 在组件挂载时添加键盘事件监听
   *   - 在组件卸载时移除监听
   *   - 依赖于 handleKeyDown，当其改变时重新注册
   */
  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);

  /**
   * Hook 2：自动加载字符串数据
   * 
   * 功能：
   *   - 当 ESP 文件路径改变且数据为空时，自动加载字符串
   *   - 避免重复加载（检查 allItems.length）
   * 
   * 依赖：
   *   - espPath：ESP 文件路径
   *   - allItems.length：已加载的数据量
   *   - loadAllStrings：加载函数
   */
  useEffect(() => {
    if (espPath && allItems.length === 0) {
      loadAllStrings();
    }
  }, [espPath, allItems.length, loadAllStrings]);

  // 排序处理函数
  const handleSort = (field: string) => setSort(field);
  
  // 行选中处理函数
  const handleSelect = (id: number) => setSelectedById(id);

  // 构建虚拟行的数据对象，传递给 react-window
  const rowData: RowData = {
    items,
    selectedId,
    filter,
    onSelect: handleSelect,
    onDoubleClick: (id) => openEditorForItem(id),
    onContextMenu: handleContextMenu,
  };

  // ========== 加载状态 ==========
  // 当数据加载中且还没有数据时，显示加载指示器
  if (isLoading && allItems.length === 0) {
    return (
      <div className="string-table-wrapper">
        <div className="table-loading">
          <Spinner size={24} />
          <span>{t("table.loading")}</span>
        </div>
      </div>
    );
  }

  // ========== 主渲染 ==========
  return (
    <div className="string-table-wrapper">
      {/* 列表类型过滤标签：All / STRINGS / DLSTRINGS / ILSTRINGS */}
      <div className="list-index-tabs">
        {[
          { key: null, label: t("common.all", { defaultValue: "All" }) },
          { key: 0, label: "STRINGS" },
          { key: 1, label: "DLSTRINGS" },
          { key: 2, label: "ILSTRINGS" },
        ].map((tab) => (
          <button
            key={tab.label}
            className={`list-index-tab ${listIndex === tab.key ? "list-index-tab-active" : ""}`}
            onClick={() => setListIndex(tab.key)}
          >
            {tab.label}
          </button>
        ))}
      </div>
      
      {/* 工具栏：显示统计信息和替换功能 */}
      <div className="table-toolbar">
        {/* 第一行：显示过滤后的条目数 / 总条目数 */}
        <div className="table-toolbar-row">
          <div className="table-info">
            {filtered.toLocaleString()} / {total.toLocaleString()}
            {/* 如果没有加载数据，显示错误提示 */}
            {allItems.length === 0 && !isLoading && (
              <span className="table-info-error">{t("common.noDataLoaded")}</span>
            )}
          </div>
        </div>
        
        {/* 第二行：搜索时显示替换功能 */}
        {filter && (
          <div className="table-toolbar-row">
            <Replace size={14} className="replace-icon" />
            <Input
              size="sm"
              placeholder="Replacement text (use $1, $2 for groups)..."
              value={replaceText}
              onChange={(e) => setReplaceText(e.target.value)}
              // 按 Enter 键执行替换
              onKeyDown={(e) => { if (e.key === "Enter" && replaceText) replaceAll(); }}
              wrapperClassName="replace-input-wrap"
            />
            <Button
              variant="default"
              size="sm"
              onClick={replaceAll}
              disabled={!replaceText}
            >
              Replace All
            </Button>
          </div>
        )}
      </div>

      {/* 表格头部：列标题 */}
      <div className="virtual-table-header">
        <div className="header-cell" style={{ width: 28 }} />
        <div className="header-cell" style={{ width: 100 }} onClick={() => handleSort("record_sig")}>
          EDID <ArrowUpDown size={10} />
        </div>
        <div className="header-cell" style={{ width: 60 }} onClick={() => handleSort("id")}>
          ID <ArrowUpDown size={10} />
        </div>
        <div className="header-cell" style={{ flex: 1 }}>{t("table.source")}</div>
        <div className="header-cell" style={{ flex: 1 }}>{t("table.translation")}</div>
        <div className="header-cell" style={{ width: 40 }}>LD</div>
      </div>

      {/* 虚拟滚动列表容器 */}
      <div className="virtual-list-container">
        <List<RowData>
          // react-window v2 API：行渲染组件
          rowComponent={VirtualRow}
          // 行数量
          rowCount={items.length}
          // 行高度（像素）
          rowHeight={ROW_HEIGHT}
          // 传递给每一行的数据对象
          rowProps={rowData}
          // 预加载行数：提前渲染 20 行以优化滚动体验
          overscanCount={20}
          // 容器样式：填充整个父容器
          style={{ height: "100%", width: "100%" }}
        />
      </div>
      
      {/* 右键菜单 */}
      {ctxMenu && (
        <ContextMenu
          x={ctxMenu.x}
          y={ctxMenu.y}
          onClose={() => setCtxMenu(null)}
          items={[
            // 编辑选项
            {
              label: t("table.ctxEdit", { defaultValue: "Edit" }),
              icon: <Edit3 size={14} />,
              shortcut: "Enter",
              onClick: () => setSelectedById(ctxMenu.item.id),
            },
            { separator: true, label: "" },
            // 复制源文本
            {
              label: t("table.ctxCopySource", { defaultValue: "Copy Source" }),
              icon: <Copy size={14} />,
              shortcut: "Ctrl+C",
              onClick: () => navigator.clipboard.writeText(ctxMenu.item.source),
            },
            // 复制翻译文本
            {
              label: t("table.ctxCopyTranslation", { defaultValue: "Copy Translation" }),
              icon: <Copy size={14} />,
              onClick: () => navigator.clipboard.writeText(ctxMenu.item.translation || ""),
              disabled: !ctxMenu.item.translation,
            },
            { separator: true, label: "" },
            // 按 FormID 过滤
            {
              label: t("table.ctxFilterFormId", { defaultValue: "Filter by FormID" }),
              icon: <Filter size={14} />,
              shortcut: "F12",
              onClick: () => setFilter(ctxMenu.item.form_id),
            },
          ]}
        />
      )}
    </div>
  );
}
