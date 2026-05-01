import React from "react";

export interface InputProps extends Omit<React.InputHTMLAttributes<HTMLInputElement>, "size" | "prefix"> {
  size?: "sm" | "md";
  icon?: React.ReactNode;
  suffix?: React.ReactNode;
  wrapperClassName?: string;
}

export function Input({
  size = "md",
  icon,
  suffix,
  wrapperClassName = "",
  className = "",
  ...rest
}: InputProps) {
  const inputCls = ["ui-input", size === "sm" ? "ui-input-sm" : "", className].filter(Boolean).join(" ");

  if (!icon && !suffix) {
    return <input className={inputCls} {...rest} />;
  }

  return (
    <div className={`ui-input-wrap ${wrapperClassName}`}>
      {icon && <span className="ui-input-icon">{icon}</span>}
      <input className={inputCls} {...rest} />
      {suffix && <span className="ui-input-suffix">{suffix}</span>}
    </div>
  );
}
