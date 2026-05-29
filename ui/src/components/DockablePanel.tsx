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
