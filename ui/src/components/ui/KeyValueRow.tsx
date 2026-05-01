import React from "react";

export interface KeyValueRowProps {
  label: string;
  value: React.ReactNode;
  labelClassName?: string;
  valueClassName?: string;
  clickable?: boolean;
  onClick?: () => void;
  className?: string;
}

export function KeyValueRow({
  label,
  value,
  labelClassName = "",
  valueClassName = "",
  clickable,
  onClick,
  className = "",
}: KeyValueRowProps) {
  const cls = ["ui-kv-row", clickable ? "ui-kv-row-clickable" : "", className].filter(Boolean).join(" ");

  return (
    <div className={cls} onClick={clickable ? onClick : undefined} role={clickable ? "button" : undefined}>
      <span className={`ui-kv-label ${labelClassName}`.trim()}>{label}</span>
      <span className={`ui-kv-value ${valueClassName}`.trim()}>{value}</span>
    </div>
  );
}
