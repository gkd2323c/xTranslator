import React from "react";

export interface EmptyStateProps {
  icon?: React.ReactNode;
  title: string;
  hint?: string;
  className?: string;
}

export function EmptyState({ icon, title, hint, className = "" }: EmptyStateProps) {
  return (
    <div className={`ui-empty ${className}`.trim()}>
      {icon}
      <div className="ui-empty-title">{title}</div>
      {hint && <div className="ui-empty-hint">{hint}</div>}
    </div>
  );
}
