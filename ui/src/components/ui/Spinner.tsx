import { Loader2 } from "lucide-react";

export interface SpinnerProps {
  size?: number;
  className?: string;
}

export function Spinner({ size = 16, className = "" }: SpinnerProps) {
  return <Loader2 size={size} className={`ui-spin ${className}`.trim()} />;
}
