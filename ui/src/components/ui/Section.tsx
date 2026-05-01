import React from "react";

export interface SectionProps {
  icon?: React.ReactNode;
  title: string;
  children: React.ReactNode;
  className?: string;
}

export function Section({ icon, title, children, className = "" }: SectionProps) {
  return (
    <div className={`ui-section ${className}`.trim()}>
      <div className="ui-section-header">
        {icon}
        <span className="ui-section-title">{title}</span>
      </div>
      {children}
    </div>
  );
}
