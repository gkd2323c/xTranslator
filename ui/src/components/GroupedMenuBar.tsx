import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

// ============================================================================
// GroupedMenuBar 组件 - 分组菜单栏骨架
// ============================================================================
//
// 将替代现有 MenuBar.tsx，提供分组下拉菜单：
//   1. File      - 文件操作（加载、保存、导入导出）
//   2. Edit      - 编辑操作（查找、替换、撤销）
//   3. Search    - 搜索过滤（正则、状态过滤）
//   4. Translate - 翻译操作（编辑、完成、转换）
//   5. Tools     - 工具面板（批处理、BSA、PEX 等）
//   6. View      - 视图选项（主题、语言、面板布局）
//
// 骨架阶段：菜单项定义为空数组，工具栏区域为空 div。
// 完整菜单定义将在 Task 2 中添加。
//
// ============================================================================

// ============================================================================
// 类型定义
// ============================================================================

/** 菜单组 ID 类型：6 个菜单组 */
type MenuGroupId = "file" | "edit" | "search" | "translate" | "tools" | "view";

/** 菜单项类型：支持标签、点击、快捷键、禁用、分隔符 */
type MenuItem = {
  label: string;
  onClick?: () => void;
  shortcut?: string;
  disabled?: boolean;
  separator?: boolean;
};

/** 菜单组定义类型 */
type MenuGroup = {
  id: MenuGroupId;
  label: string;
  items: MenuItem[];
};

// ============================================================================
// GroupedMenuBar 主组件
// ============================================================================

/**
 * 分组菜单栏组件
 *
 * 职责：
 *   - 渲染 6 个菜单组触发按钮
 *   - 管理下拉面板的打开/关闭状态（互斥）
 *   - 点击外部关闭菜单
 *   - Escape 键关闭菜单
 *   - 悬停切换已打开的菜单（鼠标移入其他触发器时切换）
 *   - 渲染工具栏占位区域
 */
export function GroupedMenuBar() {
  // ========== 国际化和 Ref ==========
  const { t } = useTranslation();
  const menuBarRef = useRef<HTMLDivElement | null>(null);

  // ========== 菜单状态 ==========
  const [openGroup, setOpenGroup] = useState<MenuGroupId | null>(null);

  // ========== 菜单组定义（骨架：空 items）==========
  const menuGroups: MenuGroup[] = [
    { id: "file", label: t("menu.file"), items: [] },
    { id: "edit", label: t("menu.edit", { defaultValue: "Edit" }), items: [] },
    { id: "search", label: t("menu.search", { defaultValue: "Search" }), items: [] },
    { id: "translate", label: t("menu.translate"), items: [] },
    { id: "tools", label: t("menu.tools"), items: [] },
    { id: "view", label: t("menu.view", { defaultValue: "View" }), items: [] },
  ];

  // ========== Hook：点击外部关闭 ==========
  /**
   * 监听点击事件，点击菜单栏外部时关闭菜单。
   */
  useEffect(() => {
    const closeMenu = (event: MouseEvent) => {
      if (menuBarRef.current && !menuBarRef.current.contains(event.target as Node)) {
        setOpenGroup(null);
      }
    };
    document.addEventListener("mousedown", closeMenu);
    return () => document.removeEventListener("mousedown", closeMenu);
  }, []);

  // ========== Hook：Escape 键关闭 ==========
  /**
   * 监听 Escape 键，按下时关闭当前打开的菜单。
   */
  useEffect(() => {
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpenGroup(null);
    };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, []);

  // ========== 菜单操作 ==========
  /** 关闭菜单并执行操作 */
  const closeAndRun = (action?: () => void) => {
    setOpenGroup(null);
    action?.();
  };

  /** 鼠标移入触发器时切换菜单（仅在已有菜单打开时生效） */
  const handleTriggerEnter = useCallback(
    (groupId: MenuGroupId) => {
      if (openGroup !== null && openGroup !== groupId) {
        setOpenGroup(groupId);
      }
    },
    [openGroup]
  );

  // ========== 渲染单个菜单组 ==========
  /**
   * 渲染单个菜单组：触发按钮 + 下拉面板
   *
   * @param group - 菜单组定义
   * @returns 菜单组 JSX 元素
   */
  const renderMenuGroup = (group: MenuGroup) => {
    const isOpen = openGroup === group.id;

    return (
      <div
        className={`grouped-menu-group ${isOpen ? "open" : ""}`}
        key={group.id}
      >
        <button
          type="button"
          className="grouped-menu-trigger"
          onClick={() => setOpenGroup(isOpen ? null : group.id)}
          onMouseEnter={() => handleTriggerEnter(group.id)}
          aria-haspopup="menu"
          aria-expanded={isOpen}
        >
          {group.label}
        </button>
        {isOpen && group.items.length > 0 && (
          <div className="grouped-menu-panel" role="menu">
            {group.items.map((item, index) =>
              item.separator ? (
                <div key={`sep-${index}`} className="grouped-menu-separator" />
              ) : (
                <button
                  key={item.label}
                  type="button"
                  className="grouped-menu-item"
                  onClick={() => closeAndRun(item.onClick)}
                  disabled={item.disabled}
                  role="menuitem"
                >
                  <span className="grouped-menu-item-label">{item.label}</span>
                  {item.shortcut && (
                    <span className="grouped-menu-item-shortcut">{item.shortcut}</span>
                  )}
                </button>
              )
            )}
          </div>
        )}
      </div>
    );
  };

  // ========== 主渲染 ==========
  return (
    <div className="grouped-menubar" ref={menuBarRef}>
      {/* 第一行：菜单组 */}
      <div className="grouped-menubar-menu-strip" role="menubar" aria-label="Application menus">
        <div className="grouped-menubar-brand">xTranslator(x64)</div>
        {menuGroups.map(renderMenuGroup)}
      </div>

      {/* 第二行：工具栏（骨架阶段为空） */}
      <div className="grouped-menubar-toolbar" role="toolbar" aria-label="Application actions">
        {/* 工具栏内容将在 Task 2 中填充 */}
      </div>
    </div>
  );
}
