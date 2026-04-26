import { useState, useEffect, useRef } from "react";
import { useAppStore } from "../stores/appStore";
import {
  startBatchTranslate,
  startBatchExport,
  getBatchStatus,
  cancelBatchJob,
  listEspFiles,
} from "../api/strings";
import type {
  BatchEntry,
  BatchStatus,
  BatchProgress,
  BatchFileComplete,
  BatchComplete,
} from "../api/strings";
import { open } from "@tauri-apps/plugin-dialog";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import toast from "react-hot-toast";
import {
  FolderOpen,
  FolderSearch,
  X,
  Play,
  Square,
  FileText,
  CheckCircle,
  AlertCircle,
  Trash2,
  Settings,
  ChevronDown,
  ChevronUp,
  RefreshCw,
  Save,
} from "lucide-react";

type PanelView = "empty" | "idle" | "running" | "complete";

function getView(
  entries: BatchEntry[],
  status: BatchStatus | null,
): PanelView {
  if (status?.is_running) return "running";
  if (status?.is_completed || status?.is_failed) return "complete";
  if (entries.length === 0) return "empty";
  return "idle";
}

function detectGameFromPath(filePath: string): string | undefined {
  const lower = filePath.toLowerCase();
  if (lower.includes("skyrim")) return "SkyrimSE";
  if (lower.includes("fallout4") || lower.includes("fallout 4")) return "Fallout4";
  if (lower.includes("starfield")) return "Starfield";
  if (lower.includes("fallout76") || lower.includes("fallout 76")) return "Fallout76";
  if (lower.includes("falloutnv") || lower.includes("fallout nv") || lower.includes("new vegas"))
    return "FalloutNV";
  if (lower.includes("oblivion")) return "Oblivion";
  if (lower.includes("morrowind")) return "Morrowind";
  return undefined;
}

function detectLanguageFromPath(filePath: string): string | undefined {
  const lower = filePath.toLowerCase();
  if (lower.includes("english")) return "english";
  if (lower.includes("chinese")) return "chinese";
  if (lower.includes("japanese")) return "japanese";
  if (lower.includes("french")) return "french";
  if (lower.includes("german")) return "german";
  if (lower.includes("spanish")) return "spanish";
  if (lower.includes("italian")) return "italian";
  if (lower.includes("russian")) return "russian";
  return undefined;
}

function formatDuration(ms: number): string {
  const s = Math.floor(ms / 1000);
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  const sec = s % 60;
  return `${m}m ${sec}s`;
}

export function BatchPanel() {
  const batchEntries = useAppStore((s) => s.batchEntries);
  const batchStatus = useAppStore((s) => s.batchStatus);
  const setBatchStatus = useAppStore((s) => s.setBatchStatus);
  const addBatchEntries = useAppStore((s) => s.addBatchEntries);
  const removeBatchEntry = useAppStore((s) => s.removeBatchEntry);
  const clearBatchEntries = useAppStore((s) => s.clearBatchEntries);

  const [provider, setProvider] = useState("openai");
  const [skipTranslated, setSkipTranslated] = useState(true);
  const [progressDetail, setProgressDetail] = useState<BatchProgress | null>(null);
  const [completedFiles, setCompletedFiles] = useState<BatchFileComplete[]>([]);
  const [batchResult, setBatchResult] = useState<BatchComplete | null>(null);
  const [isStarting, setIsStarting] = useState(false);
  const [showConfig, setShowConfig] = useState(false);
  const [showErrorList, setShowErrorList] = useState(false);

  const view = getView(batchEntries, batchStatus);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const unlistenersRef = useRef<UnlistenFn[]>([]);

  // Poll batch status while running
  useEffect(() => {
    if (batchStatus?.is_running) {
      pollRef.current = setInterval(async () => {
        try {
          const status = await getBatchStatus();
          if (status) setBatchStatus(status);
        } catch (e) {
          console.error("Failed to poll batch status:", e);
        }
      }, 1000);
    }
    return () => {
      if (pollRef.current) {
        clearInterval(pollRef.current);
        pollRef.current = null;
      }
    };
  }, [batchStatus?.is_running, setBatchStatus]);

  // Event listeners for real-time progress
  useEffect(() => {
    let active = true;
    const setup = async () => {
      const unlisteners: UnlistenFn[] = [];

      const p = await listen<BatchProgress>("batch-progress", (event) => {
        if (!active) return;
        setProgressDetail(event.payload);
      });
      unlisteners.push(p);

      const f = await listen<BatchFileComplete>("batch-file-complete", (event) => {
        if (!active) return;
        setCompletedFiles((prev) => [...prev, event.payload]);
      });
      unlisteners.push(f);

      const c = await listen<BatchComplete>("batch-complete", (event) => {
        if (!active) return;
        setBatchResult(event.payload);
        // Re-fetch status for definitive state
        getBatchStatus().then((status) => {
          if (active && status) setBatchStatus(status);
        });
        setIsStarting(false);
      });
      unlisteners.push(c);

      unlistenersRef.current = unlisteners;
    };
    setup();
    return () => {
      active = false;
      unlistenersRef.current.forEach((fn) => fn());
      unlistenersRef.current = [];
    };
  }, [setBatchStatus]);

  // Reset local state when entries clear
  useEffect(() => {
    if (batchEntries.length === 0 && view !== "running") {
      setProgressDetail(null);
      setCompletedFiles([]);
      setBatchResult(null);
    }
  }, [batchEntries.length, view]);

  const handleAddFiles = async () => {
    const files = await open({
      multiple: true,
      filters: [{ name: "ESP/ESM", extensions: ["esp", "esm"] }],
    });
    if (!files) return;
    const paths = Array.isArray(files) ? files : [files];
    const entries: BatchEntry[] = paths.map((path) => ({
      esp_path: path,
      language: detectLanguageFromPath(path),
      game: detectGameFromPath(path),
    }));
    addBatchEntries(entries);
    if (entries.length > 0) {
      toast.success(`Added ${entries.length} file(s)`);
    }
  };

  const handleScanDir = async () => {
    const dir = await open({
      directory: true,
      multiple: false,
    });
    if (!dir) return;

    try {
      const files = await listEspFiles(dir);
      if (files.length === 0) {
        toast("No ESP/ESM files found in directory");
        return;
      }
      const entries: BatchEntry[] = files.map((path) => ({
        esp_path: path,
        language: detectLanguageFromPath(path),
        game: detectGameFromPath(path),
      }));
      addBatchEntries(entries);
      toast.success(`Found ${entries.length} file(s)`);
    } catch (e: any) {
      toast.error(`Failed to scan directory: ${e}`);
    }
  };

  const handleStartTranslate = async () => {
    if (batchEntries.length === 0) return;
    setIsStarting(true);
    setProgressDetail(null);
    setCompletedFiles([]);
    setBatchResult(null);

    try {
      const config = {
        entries: batchEntries,
        provider: provider === "openai" ? "openai" : "deepl",
        target_lang: undefined as string | undefined,
        skip_translated: skipTranslated ? true : undefined,
      };

      // Pre-fill target language from store
      const storeTargetLang = useAppStore.getState().targetLang;
      if (storeTargetLang) config.target_lang = storeTargetLang;

      await startBatchTranslate(config);
      // Immediately fetch status to get is_running=true
      const status = await getBatchStatus();
      if (status) setBatchStatus(status);
    } catch (e: any) {
      toast.error(`Failed to start batch: ${e}`);
      setIsStarting(false);
    }
  };

  const handleStartExport = async () => {
    if (batchEntries.length === 0) return;

    const outDir = await open({
      directory: true,
      multiple: false,
    });
    if (!outDir) return;

    // Simple format selection via confirm dialog
    const formatXml = confirm("OK = XML format, Cancel = SST format");
    const exportFormat = formatXml ? "xml" : "sst";

    setIsStarting(true);
    setProgressDetail(null);
    setCompletedFiles([]);
    setBatchResult(null);

    try {
      await startBatchExport(batchEntries, outDir, exportFormat);
      const status = await getBatchStatus();
      if (status) setBatchStatus(status);
    } catch (e: any) {
      toast.error(`Failed to start export: ${e}`);
      setIsStarting(false);
    }
  };

  const handleCancel = async () => {
    try {
      await cancelBatchJob();
      toast("Cancelling batch job...");
    } catch (e: any) {
      toast.error(`Failed to cancel: ${e}`);
    }
  };

  const handleNewBatch = () => {
    clearBatchEntries();
    setProgressDetail(null);
    setCompletedFiles([]);
    setBatchResult(null);
    setShowErrorList(false);
  };

  const status = batchStatus;
  const progress = progressDetail;
  const result = batchResult;

  return (
    <div className="sidepanel batch-panel">
      <div className="sidepanel-section">
        <h3>
          <RefreshCw size={16} />
          Batch Processor
        </h3>
      </div>

      {/* ─── Empty State ─── */}
      {view === "empty" && (
        <div className="batch-empty">
          <FileText size={40} opacity={0.2} />
          <p>No files added</p>
          <p className="sidepanel-hint">Add ESP/ESM files to process</p>
          <div className="batch-empty-actions">
            <button onClick={handleAddFiles} className="btn btn-primary">
              <FolderOpen size={14} />
              Add Files
            </button>
            <button onClick={handleScanDir} className="btn">
              <FolderSearch size={14} />
              Scan Dir
            </button>
          </div>
        </div>
      )}

      {/* ─── Idle State ─── */}
      {view === "idle" && !status?.is_running && (
        <>
          <div className="batch-file-list">
            <div className="batch-file-list-header">
              <span className="batch-file-count">{batchEntries.length} file(s)</span>
              <button
                onClick={clearBatchEntries}
                className="btn btn-ghost btn-sm"
                title="Remove all"
              >
                <Trash2 size={12} />
              </button>
            </div>
            {batchEntries.map((entry, idx) => (
              <div key={idx} className="batch-file-row">
                <div className="batch-file-info">
                  <span className="batch-file-name" title={entry.esp_path}>
                    {entry.esp_path.split(/[\\/]/).pop()}
                  </span>
                  <span className="batch-file-meta">
                    {entry.game || "auto"} · {entry.language || "auto"}
                  </span>
                </div>
                <button
                  onClick={() => removeBatchEntry(idx)}
                  className="btn btn-ghost btn-sm"
                  title="Remove"
                >
                  <X size={12} />
                </button>
              </div>
            ))}
          </div>

          <div className="batch-actions">
            <button onClick={handleAddFiles} className="btn btn-sm">
              <FolderOpen size={12} />
              Add
            </button>
            <button onClick={handleScanDir} className="btn btn-sm">
              <FolderSearch size={12} />
              Scan
            </button>
          </div>

          <div className="batch-config-toggle" onClick={() => setShowConfig(!showConfig)}>
            <Settings size={12} />
            <span>Settings</span>
            {showConfig ? <ChevronUp size={12} /> : <ChevronDown size={12} />}
          </div>

          {showConfig && (
            <div className="batch-config">
              <label className="batch-config-row">
                <span>Provider</span>
                <select
                  value={provider}
                  onChange={(e) => setProvider(e.target.value)}
                  className="lang-select"
                >
                  <option value="openai">OpenAI</option>
                  <option value="deepl">DeepL</option>
                </select>
              </label>
              <label className="batch-config-row">
                <span>Skip translated</span>
                <input
                  type="checkbox"
                  checked={skipTranslated}
                  onChange={(e) => setSkipTranslated(e.target.checked)}
                />
              </label>
            </div>
          )}

          <div className="batch-start-actions">
            <button
              onClick={handleStartTranslate}
              disabled={isStarting}
              className="btn btn-primary"
            >
              <Play size={14} />
              {isStarting ? "Starting..." : "Translate All"}
            </button>
            <button
              onClick={handleStartExport}
              disabled={isStarting}
              className="btn"
            >
              <Save size={14} />
              Export All
            </button>
          </div>
        </>
      )}

      {/* ─── Running State ─── */}
      {view === "running" && status && (
        <>
          {/* Overall progress */}
          <div className="batch-progress-overall">
            <div className="batch-progress-header">
              <span className="batch-progress-label">
                {status.job_type === "export" ? "Exporting..." : "Translating..."}
              </span>
              <span className="batch-progress-pct">
                {status.total_strings > 0
                  ? Math.round((status.translated_strings / status.total_strings) * 100)
                  : status.completed_files > 0
                    ? Math.round((status.completed_files / status.total_files) * 100)
                    : 0}
                %
              </span>
            </div>
            <div className="batch-progress-bar-bg">
              <div
                className="batch-progress-bar-fill"
                style={{
                  width:
                    status.total_strings > 0
                      ? `${(status.translated_strings / status.total_strings) * 100}%`
                      : `${(status.completed_files / status.total_files) * 100}%`,
                }}
              />
            </div>
            <div className="batch-progress-stats">
              <span>
                {status.completed_files}/{status.total_files} files
              </span>
              <span>
                {status.translated_strings}/{status.total_strings} strings
              </span>
              <span>{formatDuration(status.elapsed_ms)}</span>
            </div>
          </div>

          {/* Current file detail */}
          {progress && (
            <div className="batch-current-file">
              <div className="batch-current-file-header">
                <span className="batch-current-file-label">Current file</span>
                <span className="batch-current-file-stage">{progress.stage}</span>
              </div>
              <div className="batch-current-file-name" title={progress.file_path}>
                {progress.file_path.split(/[\\/]/).pop()}
              </div>
              <div className="batch-current-file-progress">
                <div className="batch-progress-bar-bg" style={{ height: 4 }}>
                  <div
                    className="batch-progress-bar-fill"
                    style={{
                      width:
                        progress.total_strings > 0
                          ? `${(progress.strings_translated / progress.total_strings) * 100}%`
                          : "0%",
                    }}
                  />
                </div>
                <div className="batch-progress-stats" style={{ fontSize: 10 }}>
                  <span>
                    {progress.strings_translated}/{progress.total_strings}
                  </span>
                  {progress.message && <span>{progress.message}</span>}
                </div>
              </div>
            </div>
          )}

          {/* Completed files */}
          {completedFiles.length > 0 && (
            <div className="batch-completed-files">
              <div className="batch-section-label">Completed</div>
              {completedFiles.map((cf, idx) => (
                <div key={idx} className="batch-file-row batch-file-done">
                  <div className="batch-file-info">
                    <span className="batch-file-name" title={cf.file_path}>
                      {cf.file_path.split(/[\\/]/).pop()}
                    </span>
                    <span className="batch-file-meta">
                      {cf.translated} translated · {formatDuration(cf.duration_ms)}
                    </span>
                  </div>
                  <CheckCircle size={14} className="batch-icon-success" />
                </div>
              ))}
            </div>
          )}

          <button onClick={handleCancel} className="btn" style={{ marginTop: 12 }}>
            <Square size={14} />
            Cancel
          </button>
        </>
      )}

      {/* ─── Complete State ─── */}
      {view === "complete" && (status || result) && (
        <>
          <div className="batch-result-summary">
            {(result || status) && (
              <>
                {(status?.is_cancelled || result?.is_cancelled) && (
                  <div className="batch-result-banner cancelled">
                    <AlertCircle size={16} />
                    Cancelled
                  </div>
                )}
                {!status?.is_cancelled && !result?.is_cancelled && status?.is_failed && (
                  <div className="batch-result-banner failed">
                    <AlertCircle size={16} />
                    Failed
                  </div>
                )}
                {!status?.is_cancelled && !result?.is_cancelled && !status?.is_failed && (
                  <div className="batch-result-banner success">
                    <CheckCircle size={16} />
                    Complete
                  </div>
                )}
              </>
            )}

            <div className="batch-result-stats">
              <div className="batch-result-stat">
                <span className="batch-result-stat-value">
                  {result?.success ?? status?.completed_files ?? 0}
                </span>
                <span className="batch-result-stat-label">Files OK</span>
              </div>
              <div className="batch-result-stat">
                <span className="batch-result-stat-value">
                  {result?.failed ?? status?.failed_files ?? 0}
                </span>
                <span className="batch-result-stat-label">Failed</span>
              </div>
              <div className="batch-result-stat">
                <span className="batch-result-stat-value">
                  {result?.total_translated ?? status?.translated_strings ?? 0}
                </span>
                <span className="batch-result-stat-label">Translated</span>
              </div>
              <div className="batch-result-stat">
                <span className="batch-result-stat-value">
                  {formatDuration(result?.duration_ms ?? status?.elapsed_ms ?? 0)}
                </span>
                <span className="batch-result-stat-label">Duration</span>
              </div>
            </div>

            {/* Errors */}
            {(status?.errors?.length ?? 0) > 0 && (
              <div className="batch-errors-section">
                <div
                  className="batch-errors-toggle"
                  onClick={() => setShowErrorList(!showErrorList)}
                >
                  <AlertCircle size={12} />
                  <span>{(status?.errors?.length ?? 0) + (result?.errors?.length ?? 0)} error(s)</span>
                  {showErrorList ? <ChevronUp size={12} /> : <ChevronDown size={12} />}
                </div>
                {showErrorList && (
                  <div className="batch-errors-list">
                    {[...(status?.errors || []), ...(result?.errors.map((e) => `${e.file_path}: ${e.message}`) || [])].map(
                      (err, idx) => (
                        <div key={idx} className="batch-error-item">
                          {err}
                        </div>
                      ),
                    )}
                  </div>
                )}
              </div>
            )}
          </div>

          <button onClick={handleNewBatch} className="btn btn-primary" style={{ marginTop: 12 }}>
            <RefreshCw size={14} />
            New Batch
          </button>
        </>
      )}
    </div>
  );
}
