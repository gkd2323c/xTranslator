import { useAppStore } from "../stores/appStore";
import { loadEsp, loadSst, saveSst, exportXml, importXml, saveStrings } from "../api/strings";
import { open, save } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { FolderOpen, FileUp, FileDown, FileCode, Save, RotateCcw, RefreshCw, FileArchive, Braces, Volume2, MessagesSquare } from "lucide-react";
import toast from "react-hot-toast";
import { useTranslation } from "react-i18next";
import { setI18nLanguage, SUPPORTED_LANGS } from "../i18n";

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
  const batchEntries = useAppStore((s) => s.batchEntries);

  const handleLoadEsp = async () => {
    if (isDirty && !confirm(t("batch.batchConflict"))) return;

    const espPath = await open({
      multiple: false,
      directory: false,
      filters: [
        { name: "ESP/ESM", extensions: ["esp", "esm"] },
        { name: "All", extensions: ["*"] },
      ],
    });
    if (!espPath) return;

    // Conflict check: warn if selected file is also in batch queue
    const normalizedPath = espPath.replace(/\\/g, "/").toLowerCase();
    const isBatchFile = batchEntries.some(
      (e) => e.esp_path.replace(/\\/g, "/").toLowerCase() === normalizedPath
    );
    if (isBatchFile && !showBatchPanel) {
      toast(
        "This file is also in the batch queue. Changes may be overwritten when the batch runs.",
        { icon: "⚠️", duration: 4000 }
      );
    }

    const espDir = espPath.replace(/\\/g, "/").split("/").slice(0, -1).join("/");
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
        const stats = await loadEsp(espPath, stringsDir, language);
        setEspLoaded(espPath, stats, stringsDir);
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
  };

  const handleLoadSst = async () => {
    const sstPath = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "SST Dictionary", extensions: ["sst"] }],
    });
    if (!sstPath) return;

    setLoading(true);
    try {
      const stats = await loadSst(sstPath);
      setSstLoaded(sstPath, stats);
      setIsDirty(true);
      const semanticStats =
        stats.pending_skipped + stats.old_data_preserved + stats.warning + stats.big_warning;
      toast.success(
        `SST loaded: ${stats.matched} matched, ${stats.unmatched} unmatched` +
          (stats.tier_edid + stats.tier_normalized + stats.tier_vocab + stats.ambiguous > 0
            ? ` (exact: ${stats.tier_exact}, EDID: ${stats.tier_edid}, norm: ${stats.tier_normalized}, vocab: ${stats.tier_vocab}, ambiguous: ${stats.ambiguous})`
            : "") +
          (semanticStats > 0
            ? ` (pending: ${stats.pending_skipped}, oldData: ${stats.old_data_preserved}, warnings: ${stats.warning}/${stats.big_warning})`
            : "")
      );
      await loadAllStrings();
    } catch (e: any) {
      toast.error(`Failed to load SST: ${e}`);
    } finally {
      setLoading(false);
    }
  };

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

  const handleImportXml = async () => {
    const xmlPath = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "XML Import", extensions: ["xml"] }],
    });
    if (!xmlPath) return;

    setLoading(true);
    setLoadProgress(null);

    try {
      const unlisten = await listen<any>("xml-progress", (event) => {
        setLoadProgress(event.payload);
      });

      try {
        const stats = await importXml(xmlPath);
        toast.success(
          t("toast.xmlImported", {
            matched: stats.matched,
            unmatched: stats.unmatched,
            total: stats.total,
          }) +
            (stats.tier_edid + stats.tier_vocab + stats.tier_normalized + stats.ambiguous > 0
              ? ` (exact: ${stats.tier_exact}, EDID: ${stats.tier_edid}, norm: ${stats.tier_normalized}, vocab: ${stats.tier_vocab}, ambiguous: ${stats.ambiguous})`
              : "") +
            (stats.pending_skipped + stats.warning + stats.big_warning > 0
              ? ` (pending: ${stats.pending_skipped}, warnings: ${stats.warning}/${stats.big_warning})`
              : ""),
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
  };

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
      <div className="menubar-actions">
        <button onClick={handleLoadEsp} disabled={isParsing} className="btn btn-primary">
          <FolderOpen size={16} />
          <span>{t("common.loadEsp")}</span>
        </button>
        <button onClick={handleLoadSst} disabled={isLoading || !espPath} className="btn">
          <FileUp size={16} />
          <span>{t("common.loadSst")}</span>
        </button>
        <button onClick={handleSaveSst} disabled={isLoading || !espPath} className="btn">
          <FileDown size={16} />
          <span>{t("common.saveSst")}</span>
        </button>
        <button onClick={handleSaveStrings} disabled={isLoading || !espPath} className="btn">
          <Save size={16} />
          <span>{t("common.saveStrings")}</span>
        </button>
        <div className="menubar-sep" />
        <button onClick={handleExportXml} disabled={isLoading || !espPath} className="btn">
          <FileCode size={16} />
          <span>{t("common.exportXml")}</span>
        </button>
        <button onClick={handleImportXml} disabled={isLoading || !espPath} className="btn">
          <FileCode size={16} />
          <span>{t("common.importXml")}</span>
        </button>
        <div className="menubar-sep" />
        <select
          value={targetLang}
          onChange={(e) => setTargetLang(e.target.value)}
          className="lang-select"
          title="Target language"
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
        <button
          onClick={() => setShowBatchPanel(!showBatchPanel)}
          className={`btn btn-ghost ${showBatchPanel ? "active" : ""}`}
          title={showBatchPanel ? "Close Batch Panel" : "Open Batch Panel"}
        >
          <RefreshCw size={16} />
        </button>
        <button
          onClick={() => setShowBsaBrowser(!showBsaBrowser)}
          className={`btn btn-ghost ${showBsaBrowser ? "active" : ""}`}
          title={showBsaBrowser ? "Close BSA Browser" : "Open BSA Browser"}
        >
          <FileArchive size={16} />
        </button>
        <button
          onClick={() => setShowPexPanel(!showPexPanel)}
          className={`btn btn-ghost ${showPexPanel ? "active" : ""}`}
          title={showPexPanel ? "Close PEX Panel" : "Open PEX Panel"}
        >
          <Braces size={16} />
        </button>
        <button
          onClick={() => setShowFuzPanel(!showFuzPanel)}
          className={`btn btn-ghost ${showFuzPanel ? "active" : ""}`}
          title={showFuzPanel ? "Close Voice Panel" : "Open Voice Panel"}
        >
          <Volume2 size={16} />
        </button>
        <button
          onClick={() => setShowDialogView(!showDialogView)}
          className={`btn btn-ghost ${showDialogView ? "active" : ""}`}
          title={showDialogView ? "Close Dialog View" : "Open Dialog View"}
        >
          <MessagesSquare size={16} />
        </button>
        <select
          value={theme}
          onChange={(e) => setTheme(e.target.value as any)}
          className="lang-select"
          title="Theme"
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
        >
          {Object.entries(SUPPORTED_LANGS).map(([code, label]) => (
            <option key={code} value={code}>{label}</option>
          ))}
        </select>
        <button onClick={() => {
          if (isDirty && !confirm("You have unsaved changes. Reset anyway?")) return;
          reset();
        }} className="btn btn-ghost">
          <RotateCcw size={16} />
        </button>
      </div>
      {isParsing && <span className="menubar-status parsing">Parsing ESP...</span>}
      {isLoading && <span className="menubar-status loading">Loading...</span>}
      {isDirty && <span className="menubar-status dirty" title="Unsaved changes">●</span>}
    </div>
  );
}
