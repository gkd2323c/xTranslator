import React from "react";

export interface BadgeProps {
  variant: "translated" | "incomplete" | "locked" | "script";
  size?: "sm" | "md";
  className?: string;
  children: React.ReactNode;
}

export function Badge({ variant, size = "md", className = "", children }: BadgeProps) {
  const cls = ["ui-badge", size === "sm" ? "ui-badge-sm" : "", `ui-badge-${variant}`, className]
    .filter(Boolean)
    .join(" ");

  return <span className={cls}>{children}</span>;
}
