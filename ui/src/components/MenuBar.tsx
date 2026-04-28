import { useCallback, useEffect } from "react";
import { useAppStore } from "../stores/appStore";
import { loadEsp, loadSst, saveSst, exportXml, importXml, saveStrings } from "../api/strings";
import type { LoadSstResponse, XmlImportResponse } from "../api/strings";
import { open, save } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { FolderOpen, FileUp, FileDown, FileCode, Save, RotateCcw, RefreshCw, FileArchive, Braces, Volume2, MessagesSquare, FileText, GitCompare } from "lucide-react";
import toast from "react-hot-toast";
import { useTranslation } from "react-i18next";
import { setI18nLanguage, SUPPORTED_LANGS } from "../i18n";

type ApplyStats = Pick<
  LoadSstResponse | XmlImportResponse,
  | "tier_exact"
  | "tier_edid"
  | "tier_normalized"
  | "tier_vocab"
  | "ambiguous"
  | "pending_skipped"
  | "old_data_preserved"
  | "warning"
  | "big_warning"
>;

function getPathExt(path: string): string {
  const fileName = path.replace(/\\/g, "/").split("/").pop() ?? "";
  const dotIndex = fileName.lastIndexOf(".");
  return dotIndex >= 0 ? fileName.slice(dotIndex + 1).toLowerCase() : "";
}

function formatApplyStats(stats: ApplyStats): string {
  const tierTotal =
    stats.tier_exact +
    stats.tier_edid +
    stats.tier_normalized +
    stats.tier_vocab +
    stats.ambiguous;
  const semanticTotal =
    stats.pending_skipped +
    stats.old_data_preserved +
    stats.warning +
    stats.big_warning;

  const tierStats =
    tierTotal > 0
      ? ` (exact: ${stats.tier_exact}, EDID: ${stats.tier_edid}, norm: ${stats.tier_normalized}, vocab: ${stats.tier_vocab}, ambiguous: ${stats.ambiguous})`
      : "";
  const semanticStats =
    semanticTotal > 0
      ? ` (pending: ${stats.pending_skipped}, oldData: ${stats.old_data_preserved}, warnings: ${stats.warning}/${stats.big_warning})`
      : "";

  return `${tierStats}${semanticStats}`;
}

export function MenuBar() {
  const { t, i18n } = useTranslation();
  const isParsing = useAppStore((s) => s.isParsing);
  const isLoading = useAppStore((s) => s.isLoading);
  const espPath = useAppStore((s) => s.espPath);
  const language = useAppStore((s) => s.language);
  const isDirty = useAppStore((s) => s.isDirty);
  const targetLang = useAppStore((s) => s.targetLang);
  const setParsing = useAppStore((s) => s.setParsing);
  const setLoading = useAppStore((s) => s.setLoading);
  const setError = useAppStore((s) => s.setError);
  const setLoadProgress = useAppStore((s) => s.setLoadProgress);
  const setEspLoaded = useAppStore((s) => s.setEspLoaded);
  const setSstLoaded = useAppStore((s) => s.setSstLoaded);
  const loadAllStrings = useAppStore((s) => s.loadAllStrings);
  const setIsDirty = useAppStore((s) => s.setIsDirty);
  const setTargetLang = useAppStore((s) => s.setTargetLang);
  const reset = useAppStore((s) => s.reset);
  const theme = useAppStore((s) => s.theme);
  const setTheme = useAppStore((s) => s.setTheme);
  const showBatchPanel = useAppStore((s) => s.showBatchPanel);
  const setShowBatchPanel = useAppStore((s) => s.setShowBatchPanel);
  const showBsaBrowser = useAppStore((s) => s.showBsaBrowser);
  const setShowBsaBrowser = useAppStore((s) => s.setShowBsaBrowser);
  const showPexPanel = useAppStore((s) => s.showPexPanel);
  const setShowPexPanel = useAppStore((s) => s.setShowPexPanel);
  const showFuzPanel = useAppStore((s) => s.showFuzPanel);
  const setShowFuzPanel = useAppStore((s) => s.setShowFuzPanel);
  const showDialogView = useAppStore((s) => s.showDialogView);
  const setShowDialogView = useAppStore((s) => s.setShowDialogView);
  const showMcmPanel = useAppStore((s) => s.showMcmPanel);
  const setShowMcmPanel = useAppStore((s) => s.setShowMcmPanel);
  const showEspCompare = useAppStore((s) => s.showEspCompare);
  const setShowEspCompare = useAppStore((s) => s.setShowEspCompare);
  const batchEntries = useAppStore((s) => s.batchEntries);

  const warnIfBatchFile = useCallback((path: string) => {
    const normalizedPath = path.replace(/\\/g, "/").toLowerCase();
    const isBatchFile = batchEntries.some(
      (entry) => entry.esp_path.replace(/\\/g, "/").toLowerCase() === normalizedPath
    );
    if (isBatchFile && !showBatchPanel) {
      toast(
        "This file is also in the batch queue. Changes may be overwritten when the batch runs.",
        { icon: "!", duration: 4000 }
      );
    }
  }, [batchEntries, showBatchPanel]);

  const loadEspFromPath = useCallback(async (path: string) => {
    if (isDirty && !confirm(t("batch.batchConflict"))) return;
    warnIfBatchFile(path);

    const espDir = path.replace(/\\/g, "/").split("/").slice(0, -1).join("/");
    const stringsDir = `${espDir}/Strings`;

    setParsing(true);
    setError(null);
    setLoadProgress(null);

    try {
      // 监听进度事件
      const unlisten = await listen<any>("esp-load-progress", (event) => {
        setLoadProgress(event.payload);
      });

      try {
        const stats = await loadEsp(path, stringsDir, language);
        setEspLoaded(path, stats, stringsDir);
        await loadAllStrings();
        setIsDirty(false);
        toast.success(`Loaded ${stats.total.toLocaleString()} ${t('sidebar.totalStrings').toLowerCase()}`);
      } finally {
        unlisten();
      }
    } catch (e: any) {
      setError(e.toString());
      toast.error(`${t('app.loading')} ${e}`);
    } finally {
      setParsing(false);
      setLoadProgress(null);
    }
  }, [
    isDirty,
    language,
    loadAllStrings,
    setError,
    setEspLoaded,
    setIsDirty,
    setLoadProgress,
    setParsing,
    t,
    warnIfBatchFile,
  ]);

  const handleLoadEsp = useCallback(async () => {
    const selected = await open({
      multiple: false,
      directory: false,
      filters: [
        { name: "ESP/ESM", extensions: ["esp", "esm"] },
        { name: "All", extensions: ["*"] },
      ],
    });
    const path = Array.isArray(selected) ? selected[0] : selected;
    if (!path) return;

    await loadEspFromPath(path);
  }, [loadEspFromPath]);

  const loadSstFromPath = useCallback(async (path: string) => {
    if (!espPath) {
      toast.error("Load an ESP before loading an SST dictionary.");
      return;
    }

    setLoading(true);
    try {
      const stats = await loadSst(path);
      setSstLoaded(path, stats);
      setIsDirty(true);
      toast.success(
        `SST loaded: ${stats.matched} matched, ${stats.unmatched} unmatched` +
          formatApplyStats(stats)
      );
      await loadAllStrings();
    } catch (e: any) {
      toast.error(`Failed to load SST: ${e}`);
    } finally {
      setLoading(false);
    }
  }, [espPath, loadAllStrings, setIsDirty, setLoading, setSstLoaded]);

  const handleLoadSst = useCallback(async () => {
    const selected = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "SST Dictionary", extensions: ["sst"] }],
    });
    const path = Array.isArray(selected) ? selected[0] : selected;
    if (!path) return;

    await loadSstFromPath(path);
  }, [loadSstFromPath]);

  const handleSaveSst = async () => {
    const sstPath = await save({
      filters: [{ name: "SST Dictionary", extensions: ["sst"] }],
      defaultPath: espPath
        ? espPath.replace(/\\/g, "/").replace(/\.es[mp]$/i, `_english_${targetLang}.sst`)
        : "translation.sst",
    });
    if (!sstPath) return;

    setLoading(true);
    try {
      await saveSst(sstPath);
      setIsDirty(false);
      toast.success(`SST saved to ${sstPath}`);
    } catch (e: any) {
      toast.error(`Failed to save SST: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  const handleExportXml = async () => {
    const xmlPath = await save({
      filters: [{ name: "XML Export", extensions: ["xml"] }],
      defaultPath: espPath
        ? espPath.replace(/\\/g, "/").replace(/\.es[mp]$/i, `_english_${targetLang}.xml`)
        : "translation.xml",
    });
    if (!xmlPath) return;

    setLoading(true);
    setLoadProgress(null);

    try {
      const unlisten = await listen<any>("xml-progress", (event) => {
        setLoadProgress(event.payload);
      });

      try {
        const count = await exportXml({ path: xmlPath, dest_lang: targetLang });
        setIsDirty(false);
        toast.success(`XML exported: ${count} entries`);
      } finally {
        unlisten();
      }
    } catch (e: any) {
      toast.error(`Failed to export XML: ${e}`);
    } finally {
      setLoading(false);
      setLoadProgress(null);
    }
  };

  const importXmlFromPath = useCallback(async (path: string) => {
    if (!espPath) {
      toast.error("Load an ESP before importing XML.");
      return;
    }

    setLoading(true);
    setLoadProgress(null);

    try {
      const unlisten = await listen<any>("xml-progress", (event) => {
        setLoadProgress(event.payload);
      });

      try {
        const stats = await importXml(path);
        toast.success(
          t("toast.xmlImported", {
            matched: stats.matched,
            unmatched: stats.unmatched,
            total: stats.total,
          }) + formatApplyStats(stats),
          { duration: 6000 }
        );
        setIsDirty(true);
        await loadAllStrings();
      } finally {
        unlisten();
      }
    } catch (e: any) {
      toast.error(`Failed to import XML: ${e}`);
    } finally {
      setLoading(false);
      setLoadProgress(null);
    }
  }, [espPath, loadAllStrings, setIsDirty, setLoadProgress, setLoading, t]);

  const handleImportXml = useCallback(async () => {
    const selected = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "XML Import", extensions: ["xml"] }],
    });
    const path = Array.isArray(selected) ? selected[0] : selected;
    if (!path) return;

    await importXmlFromPath(path);
  }, [importXmlFromPath]);

  const routeDroppedPath = useCallback((path: string) => {
    const ext = getPathExt(path);
    if (ext === "esp" || ext === "esm") {
      void loadEspFromPath(path);
      return;
    }
    if (ext === "sst") {
      void loadSstFromPath(path);
      return;
    }
    if (ext === "xml") {
      void importXmlFromPath(path);
      return;
    }

    toast.error("Drop an ESP/ESM, SST, or XML file.");
  }, [importXmlFromPath, loadEspFromPath, loadSstFromPath]);

  useEffect(() => {
    let disposed = false;
    let unlistenDragDrop: (() => void) | null = null;

    try {
      getCurrentWebview()
        .onDragDropEvent((event) => {
          if (event.payload.type !== "drop") return;

          const firstSupportedPath =
            event.payload.paths.find((path) => ["esp", "esm", "sst", "xml"].includes(getPathExt(path))) ??
            event.payload.paths[0];

          if (firstSupportedPath) {
            routeDroppedPath(firstSupportedPath);
          }
        })
        .then((unlisten) => {
          if (disposed) {
            unlisten();
          } else {
            unlistenDragDrop = unlisten;
          }
        })
        .catch(() => {
          /* Drag/drop is unavailable in plain browser previews. */
        });
    } catch {
      /* Tauri webview metadata is unavailable in plain browser previews. */
    }

    return () => {
      disposed = true;
      unlistenDragDrop?.();
    };
  }, [routeDroppedPath]);

  const handleSaveStrings = async () => {
    const outputDir = await open({
      multiple: false,
      directory: true,
    });
    if (!outputDir) return;

    const baseName = espPath
      ? espPath.replace(/\\/g, "/").split("/").pop()?.replace(/\.es[mp]$/i, "") || "Skyrim"
      : "Skyrim";

    setLoading(true);
    try {
      const result = await saveStrings({
        output_dir: outputDir,
        target_lang: targetLang,
        base_name: baseName,
      });
      setIsDirty(false);
      toast.success(
        `Strings saved: ${result.strings_count} + ${result.dlstrings_count} + ${result.ilstrings_count} entries (${result.translated_count} translated)`
      );
    } catch (e: any) {
      toast.error(`Failed to save Strings: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="menubar">
      <div className="menubar-brand">xTranslator</div>
      <div className="menubar-actions" role="toolbar" aria-label="Application actions">
        <div className="toolbar-group toolbar-group-primary" role="group" aria-label="Files">
          <button type="button" onClick={handleLoadEsp} disabled={isParsing} className="btn btn-primary">
            <FolderOpen size={16} />
            <span>{t("common.loadEsp")}</span>
          </button>
          <button type="button" onClick={handleLoadSst} disabled={isLoading || !espPath} className="btn">
            <FileUp size={16} />
            <span>{t("common.loadSst")}</span>
          </button>
          <button type="button" onClick={handleSaveSst} disabled={isLoading || !espPath} className="btn">
            <FileDown size={16} />
            <span>{t("common.saveSst")}</span>
          </button>
          <button type="button" onClick={handleSaveStrings} disabled={isLoading || !espPath} className="btn">
            <Save size={16} />
            <span>{t("common.saveStrings")}</span>
          </button>
        </div>

        <div className="toolbar-group" role="group" aria-label="Exchange formats">
          <button type="button" onClick={handleExportXml} disabled={isLoading || !espPath} className="btn">
            <FileCode size={16} />
            <span>{t("common.exportXml")}</span>
          </button>
          <button type="button" onClick={handleImportXml} disabled={isLoading || !espPath} className="btn">
            <FileCode size={16} />
            <span>{t("common.importXml")}</span>
          </button>
        </div>

        <div className="toolbar-group toolbar-icon-group" role="group" aria-label="Tool panels">
          <button
            type="button"
            onClick={() => setShowBatchPanel(!showBatchPanel)}
            className={`btn btn-ghost ${showBatchPanel ? "active" : ""}`}
            title={showBatchPanel ? "Close Batch Panel" : "Open Batch Panel"}
            aria-label={showBatchPanel ? "Close Batch Panel" : "Open Batch Panel"}
            aria-pressed={showBatchPanel}
          >
            <RefreshCw size={16} />
          </button>
          <button
            type="button"
            onClick={() => setShowBsaBrowser(!showBsaBrowser)}
            className={`btn btn-ghost ${showBsaBrowser ? "active" : ""}`}
            title={showBsaBrowser ? "Close BSA Browser" : "Open BSA Browser"}
            aria-label={showBsaBrowser ? "Close BSA Browser" : "Open BSA Browser"}
            aria-pressed={showBsaBrowser}
          >
            <FileArchive size={16} />
          </button>
          <button
            type="button"
            onClick={() => setShowPexPanel(!showPexPanel)}
            className={`btn btn-ghost ${showPexPanel ? "active" : ""}`}
            title={showPexPanel ? "Close PEX Panel" : "Open PEX Panel"}
            aria-label={showPexPanel ? "Close PEX Panel" : "Open PEX Panel"}
            aria-pressed={showPexPanel}
          >
            <Braces size={16} />
          </button>
          <button
            type="button"
            onClick={() => setShowFuzPanel(!showFuzPanel)}
            className={`btn btn-ghost ${showFuzPanel ? "active" : ""}`}
            title={showFuzPanel ? "Close Voice Panel" : "Open Voice Panel"}
            aria-label={showFuzPanel ? "Close Voice Panel" : "Open Voice Panel"}
            aria-pressed={showFuzPanel}
          >
            <Volume2 size={16} />
          </button>
          <button
            type="button"
            onClick={() => setShowDialogView(!showDialogView)}
            className={`btn btn-ghost ${showDialogView ? "active" : ""}`}
            title={showDialogView ? "Close Dialog View" : "Open Dialog View"}
            aria-label={showDialogView ? "Close Dialog View" : "Open Dialog View"}
            aria-pressed={showDialogView}
          >
            <MessagesSquare size={16} />
          </button>
          <button
            type="button"
            onClick={() => setShowMcmPanel(!showMcmPanel)}
            className={`btn btn-ghost ${showMcmPanel ? "active" : ""}`}
            title={showMcmPanel ? "Close MCM Panel" : "Open MCM Panel"}
            aria-label={showMcmPanel ? "Close MCM Panel" : "Open MCM Panel"}
            aria-pressed={showMcmPanel}
          >
            <FileText size={16} />
          </button>
          <button
            type="button"
            onClick={() => setShowEspCompare(!showEspCompare)}
            className={`btn btn-ghost ${showEspCompare ? "active" : ""}`}
            title={showEspCompare ? "Close ESP Compare" : "Open ESP Compare"}
            aria-label={showEspCompare ? "Close ESP Compare" : "Open ESP Compare"}
            aria-pressed={showEspCompare}
          >
            <GitCompare size={16} />
          </button>
        </div>

        <div className="toolbar-group toolbar-selects" role="group" aria-label="Preferences">
          <select
            value={targetLang}
            onChange={(e) => setTargetLang(e.target.value)}
            className="lang-select"
            title="Target language"
            aria-label="Target language"
          >
            <option value="chinese">Chinese</option>
            <option value="japanese">Japanese</option>
            <option value="korean">Korean</option>
            <option value="french">French</option>
            <option value="german">German</option>
            <option value="spanish">Spanish</option>
            <option value="italian">Italian</option>
            <option value="russian">Russian</option>
            <option value="polish">Polish</option>
            <option value="portuguese">Portuguese</option>
            <option value="brazilian">Brazilian</option>
            <option value="czech">Czech</option>
            <option value="hungarian">Hungarian</option>
          </select>
          <select
            value={theme}
            onChange={(e) => setTheme(e.target.value as any)}
            className="lang-select"
            title="Theme"
            aria-label="Theme"
          >
            <option value="auto">Auto</option>
            <option value="dark">Dark</option>
            <option value="light">Light</option>
            <option value="gray">Gray</option>
          </select>
          <select
            value={i18n.language}
            onChange={(e) => setI18nLanguage(e.target.value)}
            className="lang-select"
            title={t("common.language")}
            aria-label={t("common.language")}
          >
            {Object.entries(SUPPORTED_LANGS).map(([code, label]) => (
              <option key={code} value={code}>{label}</option>
            ))}
          </select>
          <button
            type="button"
            onClick={() => {
              if (isDirty && !confirm("You have unsaved changes. Reset anyway?")) return;
              reset();
            }}
            className="btn btn-ghost"
            title="Reset workspace"
            aria-label="Reset workspace"
          >
            <RotateCcw size={16} />
          </button>
        </div>
      </div>
      {isParsing && <span className="menubar-status parsing">Parsing ESP...</span>}
      {isLoading && <span className="menubar-status loading">Loading...</span>}
      {isDirty && <span className="menubar-status dirty" title="Unsaved changes">●</span>}
    </div>
  );
}
