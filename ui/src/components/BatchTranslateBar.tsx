import { useAppStore } from "../stores/appStore";
import { Play, Square } from "lucide-react";

export function BatchTranslateBar() {
  const selectedIds = useAppStore((s) => s.selectedIds);
  const batchState = useAppStore((s) => s.batchState);
  const batchProgress = useAppStore((s) => s.batchProgress);
  const batchConcurrency = useAppStore((s) => s.batchConcurrency);
  const startBatchTranslation = useAppStore((s) => s.startBatchTranslation);
  const cancelBatchTranslation = useAppStore((s) => s.cancelBatchTranslation);
  const setBatchConcurrency = useAppStore((s) => s.setBatchConcurrency);

  const isRunning = batchState === "running";
  const hasSelection = selectedIds.size > 0;

  return (
    <div className="batch-translate-bar" style={{ display: "flex", alignItems: "center", gap: 8 }}>
      {!isRunning ? (
        <>
          <label style={{ fontSize: 12, opacity: 0.7 }}>Concurrency:</label>
          <input
            type="range"
            min={1}
            max={10}
            value={batchConcurrency}
            onChange={(e) => setBatchConcurrency(Number(e.target.value))}
            style={{ width: 60 }}
            disabled={isRunning}
          />
          <span style={{ fontSize: 11, minWidth: 16 }}>{batchConcurrency}</span>
          <button
            className="toolbar-btn"
            disabled={!hasSelection}
            onClick={startBatchTranslation}
            title={hasSelection ? `Batch translate ${selectedIds.size} strings` : "Select strings first"}
          >
            <Play size={14} />
            <span>Batch ({selectedIds.size})</span>
          </button>
        </>
      ) : (
        <>
          <span style={{ fontSize: 12 }}>
            {batchProgress.completed}/{batchProgress.total} done
          </span>
          <button
            className="toolbar-btn"
            onClick={cancelBatchTranslation}
            title="Cancel batch"
          >
            <Square size={14} />
            <span>Cancel</span>
          </button>
        </>
      )}
    </div>
  );
}
