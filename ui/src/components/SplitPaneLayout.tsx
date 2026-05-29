import { ReactNode } from "react";
import { SplitPane, Pane } from "react-split-pane";

interface SplitPaneLayoutProps {
  children: ReactNode;           // StringTable（主内容区）
  rightPanel?: ReactNode | null; // 右侧停靠面板（BSA/PEX/FUZ/Compare）
  bottomPanel?: ReactNode | null;// 底部面板（标签页）
  rightPanelVisible: boolean;
  bottomPanelVisible: boolean;
  rightPanelSize?: number;
  bottomPanelSize?: number;
  onRightPanelResize?: (size: number) => void;
  onBottomPanelResize?: (size: number) => void;
}

/**
 * SplitPaneLayout — 基于 react-split-pane 的可拖拽分栏布局
 *
 * 布局结构：
 * ┌──────────────────────────────────────┐
 * │  MainContent  │  RightPanel (可选)   │
 * │               │                      │
 * ├───────────────┴──────────────────────┤
 * │  BottomPanel (可选)                  │
 * └──────────────────────────────────────┘
 *
 * rightPanel 和 bottomPanel 均为可选，可独立显示/隐藏。
 * 面板间分隔线可拖拽调整大小。
 */
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
  // 主内容区（可能包含右侧面板的 SplitPane）
  const mainContent =
    rightPanelVisible && rightPanel ? (
      <SplitPane
        direction="horizontal"
        onResize={(sizes) => onRightPanelResize?.(sizes[0])}
        style={{ position: "relative", height: "100%" }}
      >
        <Pane minSize={300} defaultSize={rightPanelSize}>
          {children}
        </Pane>
        <Pane minSize={200}>{rightPanel}</Pane>
      </SplitPane>
    ) : (
      children
    );

  // 如果有底部面板，用 vertical SplitPane 包裹
  if (bottomPanelVisible && bottomPanel) {
    return (
      <SplitPane
        direction="vertical"
        onResize={(sizes) => onBottomPanelResize?.(sizes[0])}
        style={{ position: "relative", height: "100%" }}
      >
        <Pane minSize={200}>{mainContent}</Pane>
        <Pane minSize={150} defaultSize={bottomPanelSize}>
          {bottomPanel}
        </Pane>
      </SplitPane>
    );
  }

  return <>{mainContent}</>;
}
