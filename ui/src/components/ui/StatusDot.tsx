import React from "react";

export interface StatusDotProps {
  variant: "translated" | "incomplete" | "locked";
  children: React.ReactNode;
  className?: string;
}

export function StatusDot({ variant, children, className = "" }: StatusDotProps) {
  return (
    <span className={`ui-status-dot ui-status-dot-${variant} ${className}`.trim()}>
      {children}
    </span>
  );
}
