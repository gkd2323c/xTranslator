/**
 * EditorPanel/index.tsx — 编辑器模式路由
 *
 * 根据 store 中的 editorMode 状态切换不同编辑器模式：
 *   - modal：弹窗模式（EditorModal）
 *   - sidebar：侧边栏模式（EditorSidebar）
 *   - inline：内联模式（Task 7）
 */

import { useAppStore } from "../../stores/appStore";
import { EditorModal } from "./EditorModal";
import { EditorSidebar } from "./EditorSidebar";

export type EditorMode = "modal" | "sidebar" | "inline";

export interface EditorPanelProps {
  open: boolean;
  onClose: () => void;
}

export function EditorDialog({ open, onClose }: EditorPanelProps) {
  // editorMode 状态将在 Task 8 添加到 store，当前使用类型断言避免类型错误
  const editorMode = ((useAppStore((s) => s as any).editorMode as string) || "modal") as EditorMode;

  // 根据编辑器模式选择对应组件
  if (editorMode === "sidebar") {
    return <EditorSidebar open={open} onClose={onClose} />;
  }
  // Inline (Task 7) 将在后续任务中添加
  return <EditorModal open={open} onClose={onClose} />;
}
