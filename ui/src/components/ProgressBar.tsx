import React from 'react';

interface ProgressBarProps {
  translated: number;
  total: number;
  className?: string;
}

export const ProgressBar: React.FC<ProgressBarProps> = ({ translated, total, className }) => {
  const percent = total > 0 ? Math.round((translated / total) * 100) : 0;
  return (
    <div className={`progress-bar-container ${className || ''}`}>
      <progress value={percent} max={100} />
      <span className="progress-label">{translated}/{total} ({percent}%)</span>
    </div>
  );
};