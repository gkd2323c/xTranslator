import React from "react";

export interface SelectOption {
  value: string;
  label: string;
}

export interface SelectProps extends Omit<React.SelectHTMLAttributes<HTMLSelectElement>, "size"> {
  size?: "sm" | "md";
  options: SelectOption[];
}

export function Select({ size = "md", options, className = "", ...rest }: SelectProps) {
  const cls = ["ui-select", size === "sm" ? "ui-select-sm" : "", className].filter(Boolean).join(" ");

  return (
    <select className={cls} {...rest}>
      {options.map((opt) => (
        <option key={opt.value} value={opt.value}>
          {opt.label}
        </option>
      ))}
    </select>
  );
}
