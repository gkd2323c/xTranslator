import { useState } from "react";
import { useTranslation } from "react-i18next";
import { headerBatchProcess, type HeaderBatchConfig, type HeaderBatchProgress, type HeaderBatchComplete } from "../../api/strings";
import { listen } from "@tauri-apps/api/event";
import toast from "react-hot-toast";
import { Button } from "../ui";
import { Play } from "lucide-react";

export function HeaderWizardPanel() {
  const { t } = useTranslation();
  const [sourceDir, setSourceDir] = useState("");
  const [gameId, setGameId] = useState("SkyrimSE");
  const [dataDir, setDataDir] = useState("Data");
  const [createBackup, setCreateBackup] = useState(true);
  const [running, setRunning] = useState(false);
  const [progress, setProgress] = useState<HeaderBatchProgress | null>(null);
  const [complete, setComplete] = useState<HeaderBatchComplete | null>(null);

  const formatProgressMessage = (batchProgress: HeaderBatchProgress) => {
    switch (batchProgress.stage) {
      case "parsing":
        return t("headerWizard.progressStage.parsing");
      case "applying":
        return t("headerWizard.progressStage.applying", { count: batchProgress.detail_count ?? 0 });
      case "complete":
        return t("headerWizard.progressStage.complete", { count: batchProgress.detail_count ?? 0 });
      case "error":
        return batchProgress.message
          ? t("headerWizard.progressStage.errorWithReason", { error: batchProgress.message })
          : t("headerWizard.progressStage.error");
      default:
        return batchProgress.message;
    }
  };

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
      setProgress(event.payload);
    });

    const unlisten2 = await listen<HeaderBatchComplete>("header-batch-complete", (event) => {
      setComplete(event.payload);
      setRunning(false);
    });

    try {
      const result = await headerBatchProcess(config);
      setComplete(result);
    } catch (e: any) {
      toast.error(t("headerWizard.batchFailed", { error: String(e) }));
      setRunning(false);
    } finally {
      unlisten();
      unlisten2();
    }
  };

  return (
    <div style={{ padding: "8px", height: "100%", display: "flex", flexDirection: "column", gap: "8px", fontSize: "13px" }}>
      <div style={{ fontWeight: 600 }}>{t("headerWizard.title")}</div>

      <div style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
        <label style={{ fontSize: "12px", color: "var(--color-muted)" }}>{t("headerWizard.sourceDir")}</label>
        <input className="ui-input" type="text" style={{ fontSize: "12px", padding: "2px 6px" }}
          value={sourceDir} onChange={(e) => setSourceDir(e.target.value)}
          placeholder={t("headerWizard.sourceDirPlaceholder")} />
      </div>

      <div style={{ display: "flex", gap: "8px" }}>
        <div style={{ display: "flex", flexDirection: "column", gap: "4px", flex: 1 }}>
          <label style={{ fontSize: "12px", color: "var(--color-muted)" }}>{t("headerWizard.game")}</label>
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
          <label style={{ fontSize: "12px", color: "var(--color-muted)" }}>{t("headerWizard.dataDir")}</label>
          <input className="ui-input" type="text" style={{ fontSize: "12px", padding: "2px 6px" }}
            value={dataDir} onChange={(e) => setDataDir(e.target.value)}
            placeholder={t("headerWizard.dataDirPlaceholder")} />
        </div>
      </div>

      <label style={{ display: "flex", alignItems: "center", gap: "4px", fontSize: "12px", cursor: "pointer" }}>
        <input type="checkbox" checked={createBackup} onChange={(e) => setCreateBackup(e.target.checked)} />
        {t("headerWizard.createBackup")}
      </label>

      <div style={{ display: "flex", gap: "4px" }}>
        <Button variant="primary" size="sm" onClick={startBatch} loading={running} disabled={!sourceDir}>
          <Play size={12} /> {t("headerWizard.startBatch")}
        </Button>
      </div>

      {/* Progress */}
      {progress && (
        <div style={{ background: "var(--color-surface-raised)", padding: "8px", borderRadius: "4px", fontSize: "12px" }}>
          <div style={{ marginBottom: "4px" }}>
            {t("headerWizard.progressFile", { current: progress.current, total: progress.total, file: progress.file_path })}
          </div>
          <div style={{ height: "4px", background: "var(--color-border)", borderRadius: "2px", overflow: "hidden" }}>
            <div style={{
              height: "100%", width: `${(progress.current / progress.total) * 100}%`,
              background: progress.stage === "error" ? "var(--color-danger)" : "var(--color-accent)",
              transition: "width 0.3s",
            }} />
          </div>
          <div style={{ marginTop: "4px", color: "var(--color-muted)" }}>
            {formatProgressMessage(progress)}
            {progress.strings_matched > 0 && ` (${t("headerWizard.matchedTotal", { count: progress.strings_matched })})`}
          </div>
        </div>
      )}

      {/* Result */}
      {complete && (
        <div style={{ background: "var(--color-surface-raised)", padding: "8px", borderRadius: "4px", fontSize: "12px" }}>
          <div style={{ fontWeight: 600, marginBottom: "4px" }}>{t("headerWizard.complete")}</div>
          <div>{t("headerWizard.filesSummary", { success: complete.success, failed: complete.failed, total: complete.total_files })}</div>
          <div>{t("headerWizard.stringsMatched", { count: complete.total_strings_matched })}</div>
          <div>{t("headerWizard.duration", { seconds: (complete.duration_ms / 1000).toFixed(1) })}</div>
          {complete.errors.length > 0 && (
            <details style={{ marginTop: "4px" }}>
              <summary style={{ cursor: "pointer", color: "var(--color-danger)" }}>{t("headerWizard.errors", { count: complete.errors.length })}</summary>
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
