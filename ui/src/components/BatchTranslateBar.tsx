import { useAppStore } from "../stores/appStore";
import { Play, Square } from "lucide-react";
import { Button } from "./ui";

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
    <div className="batch-translate-bar">
      {!isRunning ? (
        <>
          <label className="batch-bar-label">Concurrency:</label>
          <input
            type="range"
            min={1}
            max={10}
            value={batchConcurrency}
            onChange={(e) => setBatchConcurrency(Number(e.target.value))}
            className="batch-bar-range"
            disabled={isRunning}
          />
          <span className="batch-bar-concurrency">{batchConcurrency}</span>
          <Button
            variant="default"
            size="sm"
            icon={<Play size={14} />}
            disabled={!hasSelection}
            onClick={startBatchTranslation}
            title={hasSelection ? `Batch translate ${selectedIds.size} strings` : "Select strings first"}
          >
            Batch ({selectedIds.size})
          </Button>
        </>
      ) : (
        <>
          <span className="batch-bar-progress">
            {batchProgress.completed}/{batchProgress.total} done
          </span>
          <Button
            variant="default"
            size="sm"
            icon={<Square size={14} />}
            onClick={cancelBatchTranslation}
            title="Cancel batch"
          >
            Cancel
          </Button>
        </>
      )}
    </div>
  );
}
