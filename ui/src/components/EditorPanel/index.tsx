/**
 * EditorPanel/index.tsx — 编辑器模式路由
 *
 * 根据 store 中的 editorMode 状态切换不同编辑器模式：
 *   - modal：弹窗模式（EditorModal，Task 5）
 *   - sidebar：侧边栏模式（Task 6）
 *   - inline：内联模式（Task 7）
 *
 * 当前仅渲染占位符，EditorModal 将在 Task 5 中创建。
 */

// EditorModal will be created in Task 5
// import { EditorModal } from "./EditorModal";

export type EditorMode = "modal" | "sidebar" | "inline";

export interface EditorPanelProps {
  open: boolean;
  onClose: () => void;
}

export function EditorDialog({ open: _open, onClose: _onClose }: EditorPanelProps) {
  // editorMode state will be added to the store in Task 8.
  // const editorMode = useAppStore((s) => s.editorMode);

  // For now, render a placeholder. EditorModal will replace this in Task 5.
  return <div>EditorModal placeholder (Task 5)</div>;
}
