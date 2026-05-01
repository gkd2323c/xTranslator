import React from "react";
import { Loader2 } from "lucide-react";

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "default" | "primary" | "ghost";
  size?: "xs" | "sm" | "md";
  icon?: React.ReactNode;
  loading?: boolean;
  active?: boolean;
}

export function Button({
  variant = "default",
  size = "md",
  icon,
  loading,
  active,
  className = "",
  children,
  disabled,
  ...rest
}: ButtonProps) {
  const cls = [
    "ui-btn",
    `ui-btn-${size}`,
    variant === "primary" ? "ui-btn-primary" : variant === "ghost" ? "ui-btn-ghost" : "",
    active ? "ui-btn-active" : "",
    loading ? "ui-btn-loading" : "",
    className,
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <button className={cls} disabled={disabled || loading} {...rest}>
      {loading ? <Loader2 size={size === "xs" ? 12 : 14} className="ui-spin" /> : icon}
      {children && <span>{children}</span>}
    </button>
  );
}
