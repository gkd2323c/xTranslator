import { useState } from "react";
import { headerBatchProcess, type HeaderBatchConfig, type HeaderBatchProgress, type HeaderBatchComplete } from "../../api/strings";
import { listen } from "@tauri-apps/api/event";
import toast from "react-hot-toast";
import { Button } from "../ui";
import { Play } from "lucide-react";

export interface WizardProgress {
  current: number;
  total: number;
  filePath: string;
  stringsMatched: number;
  stage: string;
  message: string;
}

export function HeaderWizardPanel() {
  const [sourceDir, setSourceDir] = useState("");
  const [gameId, setGameId] = useState("SkyrimSE");
  const [dataDir, setDataDir] = useState("Data");
  const [createBackup, setCreateBackup] = useState(true);
  const [running, setRunning] = useState(false);
  const [progress, setProgress] = useState<WizardProgress | null>(null);
  const [complete, setComplete] = useState<HeaderBatchComplete | null>(null);

  const startBatch = async () => {
    if (!sourceDir || !gameId || !dataDir) return;
    setRunning(true);
    setProgress(null);
    setComplete(null);

    const config: HeaderBatchConfig = {
      source_dir: sourceDir,
      game_id: gameId,
      data_dir: dataDir,
      create_backup: createBackup,
    };

    const unlisten = await listen<HeaderBatchProgress>("header-batch-progress", (event) => {
      setProgress({
        current: event.payload.current,
        total: event.payload.total,
        filePath: event.payload.file_path,
        stringsMatched: event.payload.strings_matched,
        stage: event.payload.stage,
        message: event.payload.message,
      });
    });

    const unlisten2 = await listen<HeaderBatchComplete>("header-batch-complete", (event) => {
      setComplete(event.payload);
      setRunning(false);
    });

    try {
      const result = await headerBatchProcess(config);
      setComplete(result);
    } catch (e: any) {
      toast.error(`Batch failed: ${e}`);
      setRunning(false);
    } finally {
      unlisten();
      unlisten2();
    }
  };

  return (
    <div style={{ padding: "8px", height: "100%", display: "flex", flexDirection: "column", gap: "8px", fontSize: "13px" }}>
      <div style={{ fontWeight: 600 }}>Header Batch Wizard</div>

      <div style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
        <label style={{ fontSize: "12px", color: "var(--color-muted)" }}>Source Directory (ESP/ESM files)</label>
        <input className="ui-input" type="text" style={{ fontSize: "12px", padding: "2px 6px" }}
          value={sourceDir} onChange={(e) => setSourceDir(e.target.value)}
          placeholder="e.g. D:\Mods\MyPlugin\esps" />
      </div>

      <div style={{ display: "flex", gap: "8px" }}>
        <div style={{ display: "flex", flexDirection: "column", gap: "4px", flex: 1 }}>
          <label style={{ fontSize: "12px", color: "var(--color-muted)" }}>Game</label>
          <select className="ui-input" style={{ fontSize: "12px", padding: "2px 4px" }}
            value={gameId} onChange={(e) => setGameId(e.target.value)}>
            <option value="SkyrimSE">Skyrim SE</option>
            <option value="Skyrim">Skyrim</option>
            <option value="Fallout4">Fallout 4</option>
            <option value="FalloutNV">Fallout NV</option>
            <option value="Fallout76">Fallout 76</option>
            <option value="Starfield">Starfield</option>
          </select>
        </div>
        <div style={{ display: "flex", flexDirection: "column", gap: "4px", flex: 1 }}>
          <label style={{ fontSize: "12px", color: "var(--color-muted)" }}>Data Directory</label>
          <input className="ui-input" type="text" style={{ fontSize: "12px", padding: "2px 6px" }}
            value={dataDir} onChange={(e) => setDataDir(e.target.value)}
            placeholder="Data" />
        </div>
      </div>

      <label style={{ display: "flex", alignItems: "center", gap: "4px", fontSize: "12px", cursor: "pointer" }}>
        <input type="checkbox" checked={createBackup} onChange={(e) => setCreateBackup(e.target.checked)} />
        Create backup before saving
      </label>

      <div style={{ display: "flex", gap: "4px" }}>
        <Button variant="primary" size="sm" onClick={startBatch} loading={running} disabled={!sourceDir}>
          <Play size={12} /> Start Batch
        </Button>
      </div>

      {/* Progress */}
      {progress && (
        <div style={{ background: "var(--color-surface-raised)", padding: "8px", borderRadius: "4px", fontSize: "12px" }}>
          <div style={{ marginBottom: "4px" }}>
            {progress.current}/{progress.total}: {progress.filePath}
          </div>
          <div style={{ height: "4px", background: "var(--color-border)", borderRadius: "2px", overflow: "hidden" }}>
            <div style={{
              height: "100%", width: `${(progress.current / progress.total) * 100}%`,
              background: progress.stage === "error" ? "var(--color-danger)" : "var(--color-accent)",
              transition: "width 0.3s",
            }} />
          </div>
          <div style={{ marginTop: "4px", color: "var(--color-muted)" }}>
            {progress.message}
            {progress.stringsMatched > 0 && ` (${progress.stringsMatched} matched)`}
          </div>
        </div>
      )}

      {/* Result */}
      {complete && (
        <div style={{ background: "var(--color-surface-raised)", padding: "8px", borderRadius: "4px", fontSize: "12px" }}>
          <div style={{ fontWeight: 600, marginBottom: "4px" }}>Batch Complete</div>
          <div>Files: {complete.success} success / {complete.failed} failed / {complete.total_files} total</div>
          <div>Strings matched: {complete.total_strings_matched}</div>
          <div>Duration: {(complete.duration_ms / 1000).toFixed(1)}s</div>
          {complete.errors.length > 0 && (
            <details style={{ marginTop: "4px" }}>
              <summary style={{ cursor: "pointer", color: "var(--color-danger)" }}>Errors ({complete.errors.length})</summary>
              {complete.errors.map((e, i) => (
                <div key={i} style={{ color: "var(--color-danger)", marginTop: "2px" }}>{e}</div>
              ))}
            </details>
          )}
        </div>
      )}
    </div>
  );
}
