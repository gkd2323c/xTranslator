/**
 * EditorPanel/index.tsx — 编辑器模式路由
 *
 * 根据 store 中的 editorMode 状态切换不同编辑器模式：
 *   - modal：弹窗模式（EditorModal）
 *   - sidebar：侧边栏模式（EditorSidebar）
 *   - inline：内联模式（EditorInline）
 */

import { useAppStore } from "../../stores/appStore";
import { EditorModal } from "./EditorModal";
import { EditorSidebar } from "./EditorSidebar";
import { EditorInline } from "./EditorInline";

export interface EditorPanelProps {
  open: boolean;
  onClose: () => void;
}

/**
 * EditorDialog — 根据 editorMode 路由到对应编辑器组件
 */
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
