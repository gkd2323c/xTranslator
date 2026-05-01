
export interface ProgressBarProps {
  value: number;
  max: number;
  variant?: "default" | "gradient";
  size?: "sm" | "md";
  showLabel?: boolean;
  label?: string;
  className?: string;
}

export function ProgressBar({
  value,
  max,
  variant = "default",
  size = "md",
  showLabel,
  label,
  className = "",
}: ProgressBarProps) {
  const pct = max > 0 ? Math.round((value / max) * 100) : 0;

  return (
    <div className={`ui-progress-wrap ${className}`.trim()}>
      {(showLabel || label) && (
        <div className="ui-progress-header">
          <span>{label}</span>
          <span className="ui-progress-value">{pct}%</span>
        </div>
      )}
      <div className="ui-progress-track" style={{ height: size === "sm" ? 4 : 6 }}>
        <div
          className={`ui-progress-fill ${variant === "gradient" ? "ui-progress-fill-gradient" : ""}`}
          style={{ width: `${pct}%` }}
        />
      </div>
    </div>
  );
}
