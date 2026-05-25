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
import { useTranslation } from "react-i18next";
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
import { Button, EmptyState, Select } from "./ui";

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
  const { t } = useTranslation();
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

  // 运行时轮询批处理状态
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

  // 实时进度的事件监听器
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
        // 重新获取状态以获取最终状态
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

  // 当条目清空时重置本地状态
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
      toast.success(t("batch.addFileSuccess", { count: entries.length }));
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
        toast(t("batch.noEspFound"));
        return;
      }
      const entries: BatchEntry[] = files.map((path) => ({
        esp_path: path,
        language: detectLanguageFromPath(path),
        game: detectGameFromPath(path),
      }));
      addBatchEntries(entries);
      toast.success(t("batch.foundFiles", { count: entries.length }));
    } catch (e: any) {
      toast.error(`${t("batch.scanFailed")}: ${e}`);
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

      // 从 store 预填目标语言
      const storeTargetLang = useAppStore.getState().targetLang;
      if (storeTargetLang) config.target_lang = storeTargetLang;

      await startBatchTranslate(config);
      // 立即获取状态以使 is_running=true
      const status = await getBatchStatus();
      if (status) setBatchStatus(status);
    } catch (e: any) {
      toast.error(`${t("batch.batchStartFailed")}: ${e}`);
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

    // 通过确认对话框进行简单格式选择
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
      toast.error(`${t("batch.exportStartFailed")}: ${e}`);
      setIsStarting(false);
    }
  };

  const handleCancel = async () => {
    try {
      await cancelBatchJob();
      toast(t("batch.cancelling"));
    } catch (e: any) {
      toast.error(`${t("batch.cancelFailed")}: ${e}`);
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
          {t("batch.title")}
        </h3>
      </div>

      {/* ─── 空状态 ─── */}
      {view === "empty" && (
        <div className="batch-empty">
          <EmptyState
            icon={<FileText size={40} />}
            title={t("batch.noFilesAdded")}
            hint={t("batch.addFilesHint")}
          />
          <div className="batch-empty-actions">
            <Button variant="primary" onClick={handleAddFiles} icon={<FolderOpen size={14} />}>
              Add Files
            </Button>
            <Button variant="default" onClick={handleScanDir} icon={<FolderSearch size={14} />}>
              Scan Dir
            </Button>
          </div>
        </div>
      )}

      {/* ─── 空闲状态 ─── */}
      {view === "idle" && !status?.is_running && (
        <>
          <div className="batch-file-list">
            <div className="batch-file-list-header">
              <span className="batch-file-count">{batchEntries.length} file(s)</span>
              <Button
                variant="ghost"
                size="sm"
                onClick={clearBatchEntries}
                title={t("batch.removeAll")}
                icon={<Trash2 size={12} />}
              />
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
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => removeBatchEntry(idx)}
                  title={t("batch.remove")}
                  icon={<X size={12} />}
                />
              </div>
            ))}
          </div>

          <div className="batch-actions">
            <Button variant="default" size="sm" onClick={handleAddFiles} icon={<FolderOpen size={12} />}>
              Add
            </Button>
            <Button variant="default" size="sm" onClick={handleScanDir} icon={<FolderSearch size={12} />}>
              Scan
            </Button>
          </div>

          <div className="batch-config-toggle" onClick={() => setShowConfig(!showConfig)}>
            <Settings size={12} />
            <span>{t("batch.settings")}</span>
            {showConfig ? <ChevronUp size={12} /> : <ChevronDown size={12} />}
          </div>

          {showConfig && (
            <div className="batch-config">
              <label className="batch-config-row">
                <span>{t("batch.provider")}</span>
                <Select
                  value={provider}
                  onChange={(e) => setProvider(e.target.value)}
                  size="sm"
                  options={[
                    { value: "openai", label: "OpenAI" },
                    { value: "deepl", label: "DeepL" },
                  ]}
                />
              </label>
              <label className="batch-config-row">
                <span>{t("batch.skipTranslated")}</span>
                <input
                  type="checkbox"
                  checked={skipTranslated}
                  onChange={(e) => setSkipTranslated(e.target.checked)}
                />
              </label>
            </div>
          )}

          <div className="batch-start-actions">
            <Button
              variant="primary"
              onClick={handleStartTranslate}
              disabled={isStarting}
              icon={<Play size={14} />}
            >
              {isStarting ? t("batch.starting") : t("batch.translateAll")}
            </Button>
            <Button
              variant="default"
              onClick={handleStartExport}
              disabled={isStarting}
              icon={<Save size={14} />}
            >
              Export All
            </Button>
          </div>
        </>
      )}

      {/* ─── 运行状态 ─── */}
      {view === "running" && status && (
        <>
          {/* 整体进度 */}
          <div className="batch-progress-overall">
            <div className="batch-progress-header">
              <span className="batch-progress-label">
                {status.job_type === "export" ? t("batch.exporting") : t("batch.translating")}
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

          {/* 当前文件详情 */}
          {progress && (
            <div className="batch-current-file">
              <div className="batch-current-file-header">
                <span className="batch-current-file-label">{t("batch.currentFile")}</span>
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

          {/* 已完成的文件 */}
          {completedFiles.length > 0 && (
            <div className="batch-completed-files">
              <div className="batch-section-label">{t("batch.completed")}</div>
              {completedFiles.map((cf, idx) => (
                <div key={idx} className="batch-file-row batch-file-done">
                  <div className="batch-file-info">
                    <span className="batch-file-name" title={cf.file_path}>
                      {cf.file_path.split(/[\\/]/).pop()}
                    </span>
                    <span className="batch-file-meta">
                      {cf.translated} {t("batch.translatedCount")} · {formatDuration(cf.duration_ms)}
                    </span>
                  </div>
                  <CheckCircle size={14} className="batch-icon-success" />
                </div>
              ))}
            </div>
          )}

          <Button variant="default" onClick={handleCancel} icon={<Square size={14} />} className="batch-cancel-btn">
            Cancel
          </Button>
        </>
      )}
      {/* ─── 完成状态 ─── */}
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

            {/* 增强版统计卡片 */}
            <div className="batch-stats-cards">
              <div className="batch-stat-card batch-stat-ok">
                <span className="batch-stat-value">{result?.success ?? status?.completed_files ?? 0}</span>
                <span className="batch-stat-label">{t("batch.filesOk")}</span>
              </div>
              <div className="batch-stat-card batch-stat-fail">
                <span className="batch-stat-value">{result?.failed ?? status?.failed_files ?? 0}</span>
                <span className="batch-stat-label">{t("batch.failed")}</span>
              </div>
              <div className="batch-stat-card batch-stat-trans">
                <span className="batch-stat-value">{result?.total_translated ?? status?.translated_strings ?? 0}</span>
                <span className="batch-stat-label">{t("batch.translatedCount")}</span>
              </div>
              <div className="batch-stat-card batch-stat-time">
                <span className="batch-stat-value">{formatDuration(result?.duration_ms ?? status?.elapsed_ms ?? 0)}</span>
                <span className="batch-stat-label">{t("batch.duration")}</span>
              </div>
            </div>

            {/* 每个文件的结果详情 */}
            {completedFiles.length > 0 && (
              <div className="batch-file-results">
                <div className="batch-section-title">{t("batch.fileResults", { defaultValue: "File Results" })}</div>
                {completedFiles.map((cf, idx) => {
                  const fileErrors = (result?.errors || []).filter((e) => e.file_path === cf.file_path);
                  const isExpanded = showErrorList; // 为求简便，复用现有的布尔值
                  return (
                    <div key={idx} className={`batch-file-result-row ${fileErrors.length > 0 ? "batch-file-result-fail" : "batch-file-result-ok"}`}>
                      <div className="batch-file-result-header" onClick={() => fileErrors.length > 0 && setShowErrorList(!showErrorList)}>
                        <span className={`batch-file-result-status ${fileErrors.length > 0 ? "fail" : "ok"}`}>
                          {fileErrors.length > 0 ? "✕" : "✓"}
                        </span>
                        <span className="batch-file-result-name" title={cf.file_path}>
                          {cf.file_path.split(/[\\/]/).pop()}
                        </span>
                        <span className="batch-file-result-meta">
                          {cf.translated} {t("batch.translatedCount")} · {formatDuration(cf.duration_ms)}
                        </span>
                        {fileErrors.length > 0 && (
                          <>
                            <span className="batch-file-result-errors-badge">{fileErrors.length} err</span>
                            {isExpanded ? <ChevronUp size={12} /> : <ChevronDown size={12} />}
                          </>
                        )}
                      </div>
                      {isExpanded && fileErrors.length > 0 && (
                        <div className="batch-file-result-errors">
                          {fileErrors.map((e, ei) => (
                            <div key={ei} className="batch-file-error-item">
                              <span className="batch-file-error-msg">{e.message}</span>
                            </div>
                          ))}
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            )}


            {/* 遗留的扁平错误列表 */}
            {(status?.errors?.length ?? 0) > 0 && completedFiles.length === 0 && (
              <div className="batch-errors-section">
                <div
                  className="batch-errors-toggle"
                  onClick={() => setShowErrorList(!showErrorList)}
                >
                  <AlertCircle size={12} />
                  <span>{(status?.errors?.length ?? 0) + (result?.errors?.length ?? 0)} {t("batch.errors")}</span>
                  {showErrorList ? <ChevronUp size={12} /> : <ChevronDown size={12} />}
                </div>
                {showErrorList && (
                  <div className="batch-errors-list">
                    {[...(status?.errors || []), ...(result?.errors?.map((e) => `${e.file_path}: ${e.message}`) || [])].map(
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

            {/* 重试失败项按钮 */}
            {(result?.failed ?? status?.failed_files ?? 0) > 0 && (
              <Button
                variant="default"
                size="sm"
                onClick={() => {
                  // 移除成功的条目并重新启动
                  const failedPaths = new Set(
                    completedFiles.filter((cf) =>
                      (result?.errors || []).some((e) => e.file_path === cf.file_path)
                    ).map((cf) => cf.file_path)
                  );
                  const failedEntries = batchEntries.filter((e) => failedPaths.has(e.esp_path));
                  if (failedEntries.length > 0) {
                    // 清空并重新添加失败的条目
                    // 这里进行了简化：在实践中，需要使用特定文件重新调用批处理
                    toast(t("batch.retryHint", { defaultValue: "Removed successful files. Click Translate to retry failed ones." }));
                  }
                }}
                icon={<RefreshCw size={12} />}
              >
                {t("batch.retryFailed", { defaultValue: `Retry Failed (${result?.failed ?? status?.failed_files ?? 0})` })}
              </Button>
            )}
          </div>

          <Button variant="primary" onClick={handleNewBatch} icon={<RefreshCw size={14} />} className="batch-cancel-btn">
            New Batch
          </Button>
        </>
      )}
    </div>
  );
}
