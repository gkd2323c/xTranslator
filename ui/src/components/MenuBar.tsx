import { useCallback, useEffect, useState } from "react";
import { useAppStore } from "../stores/appStore";
import { Button } from "./ui";
import { loadEsp, loadSst, saveSst, exportXml, importXml, saveStrings, saveEsp, tcscConvert, tcscBatchConvert, updateTranslation, loadVocabulary, compareSourceDest, loadDataConfigs, delocalizeEsp, type BatchProgress } from "../api/strings";
import type { LoadSstResponse, XmlImportResponse } from "../api/strings";
import { open, save } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { FolderOpen, FileUp, FileDown, FileCode, Save, RotateCcw, RefreshCw, FileArchive, Braces, Volume2, MessagesSquare, FileText, GitCompare, CheckCircle, Settings, ArrowLeftRight, Database, Wrench } from "lucide-react";
import toast from "react-hot-toast";
import { useTranslation } from "react-i18next";
import { setI18nLanguage, SUPPORTED_LANGS } from "../i18n";
import { SettingsDialog } from "./SettingsDialog";
import { ToolboxDialog } from "./ToolboxDialog";

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
  const [showSettings, setShowSettings] = useState(false);
  const [showToolbox, setShowToolbox] = useState(false);
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
  const activePanel = useAppStore((s) => s.activePanel);
  const setActivePanel = useAppStore((s) => s.setActivePanel);
  const setDataConfigs = useAppStore((s) => s.setDataConfigs);
  const espMode = useAppStore((s) => s.espMode);
  const batchEntries = useAppStore((s) => s.batchEntries);
  const [batchProgress, setBatchProgress] = useState<BatchProgress | null>(null);

  const warnIfBatchFile = useCallback((path: string) => {
    const normalizedPath = path.replace(/\\/g, "/").toLowerCase();
    const isBatchFile = batchEntries.some(
      (entry) => entry.esp_path.replace(/\\/g, "/").toLowerCase() === normalizedPath
    );
    if (isBatchFile && activePanel !== "batch") {
      toast(
        "This file is also in the batch queue. Changes may be overwritten when the batch runs.",
        { icon: "!", duration: 4000 }
      );
    }
  }, [batchEntries, activePanel]);

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

        // Check for unapplied translation cache (crash recovery)
        if (stats.esp_hash) {
          useAppStore.getState().checkAndPromptRecovery(stats.esp_hash);
        }

        // Auto-load vocabulary for heuristic search enrichment
        loadVocabulary(stringsDir, language, useAppStore.getState().targetLang, useAppStore.getState().language === "english" ? "SkyrimSE" : undefined)
          .then((info) => {
            if (info.pair_count > 0) {
              toast(t("menu.vocabularyLoaded", { pairs: info.pair_count.toLocaleString(), files: info.base_names.length }), { duration: 3000 });
            }
          })
          .catch(() => {});

        // Auto-load Data Configs for reference data
        loadDataConfigs(useAppStore.getState().language === "english" ? "SkyrimSE" : "SkyrimSE")
          .then((cfg) => {
            setDataConfigs(cfg);
            const fieldCount = Object.keys(cfg.field_size_ref).length;
            toast.success(t("menu.dataConfigsLoaded", { ctda: cfg.ctda_funcs.length, fields: fieldCount }), { duration: 3000 });
          })
          .catch(() => {});
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
      toast.error(t("menu.espBeforeSst"));
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
      toast.error(`${t("menu.sstSaveFailed")}: ${e}`);
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
      toast.success(t("toast.sstSaved", { path: sstPath }));
    } catch (e: any) {
      toast.error(`${t("menu.sstSaveFailed")}: ${e}`);
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
        toast.success(t("toast.xmlExported", { count }));
      } finally {
        unlisten();
      }
    } catch (e: any) {
      toast.error(`${t("menu.exportFailed")}: ${e}`);
    } finally {
      setLoading(false);
      setLoadProgress(null);
    }
  };

  const importXmlFromPath = useCallback(async (path: string) => {
    if (!espPath) {
      toast.error(t("menu.espBeforeImport"));
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
      toast.error(`${t("menu.importFailed")}: ${e}`);
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
    if (ext === "bsa" || ext === "ba2") {
      setActivePanel("bsa");
      toast(t("menu.dragDropBsa", { defaultValue: "BSA/BA2 file detected — use the BSA Browser panel to open it" }), { icon: "📦", duration: 3000 });
      return;
    }
    if (ext === "pex") {
      setActivePanel("pex");
      toast(t("menu.dragDropPex", { defaultValue: "PEX file detected — use the PEX panel to open it" }), { icon: "📜", duration: 3000 });
      return;
    }
    if (ext === "fuz") {
      setActivePanel("fuz");
      toast(t("menu.dragDropFuz", { defaultValue: "FUZ file detected — use the Voice panel to scan" }), { icon: "🔊", duration: 3000 });
      return;
    }

    toast.error(t("menu.dragDropUnsupported"));
  }, [importXmlFromPath, loadEspFromPath, loadSstFromPath, setActivePanel, t]);

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

  // Listen to batch progress events
  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | null = null;

    listen<BatchProgress>("batch-progress", (event) => {
      if (!disposed) {
        setBatchProgress(event.payload);
      }
    }).then((u) => {
      unlisten = u;
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  const handleSaveStrings = async () => {
    if (espMode && espPath) {
      // ESP mode: save directly to the ESP file
      setLoading(true);
      try {
        const result = await saveEsp({
          path: espPath,
          create_backup: true,
        });
        setIsDirty(false);
        toast.success(t("menu.espSaved", { defaultValue: "ESP saved: {{count}} records modified", count: result.records_modified }));
      } catch (e: any) {
        toast.error(`${t("menu.saveEspFailed", { defaultValue: "Failed to save ESP" })}: ${e}`);
      } finally {
        setLoading(false);
      }
      return;
    }

    // Strings mode: save to external .STRINGS files
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
      toast.success(t("menu.stringsSaved", { strings: result.strings_count, dlstrings: result.dlstrings_count, ilstrings: result.ilstrings_count, translated: result.translated_count }));
    } catch (e: any) {
      toast.error(`${t("menu.saveStringsFailed")}: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="menubar">
      <div className="menubar-brand">xTranslator</div>
      <div className="menubar-actions" role="toolbar" aria-label="Application actions">
        <div className="toolbar-group toolbar-group-primary" role="group" aria-label="Files">
          <Button variant="primary" icon={<FolderOpen size={16} />} onClick={handleLoadEsp} disabled={isParsing}>
            {t("common.loadEsp")}
          </Button>
          <Button icon={<FileUp size={16} />} onClick={handleLoadSst} disabled={isLoading || !espPath}>
            {t("common.loadSst")}
          </Button>
          <Button icon={<FileDown size={16} />} onClick={handleSaveSst} disabled={isLoading || !espPath}>
            {t("common.saveSst")}
          </Button>
          <Button icon={<Save size={16} />} onClick={handleSaveStrings} disabled={isLoading || !espPath}>
            {t("common.saveStrings")}
          </Button>
        </div>

        <div className="toolbar-group" role="group" aria-label="Exchange formats">
          <Button icon={<FileCode size={16} />} onClick={handleExportXml} disabled={isLoading || !espPath}>
            {t("common.exportXml")}
          </Button>
          <Button icon={<FileCode size={16} />} onClick={handleImportXml} disabled={isLoading || !espPath}>
            {t("common.importXml")}
          </Button>
        </div>

        <div className="toolbar-group" role="group" aria-label="Finalize">
          <Button
            variant="primary"
            icon={<CheckCircle size={16} />}
            onClick={() => setActivePanel("finalize")}
            disabled={isLoading || !espPath}
            active={activePanel === "finalize"}
            title={t("finalize.title")}
          >
            {t("finalize.title")}
          </Button>
          {espMode && (
            <Button
              variant="ghost"
              icon={<FileCode size={16} />}
              onClick={async () => {
                if (!espPath) return;
                try {
                  const baseName = espPath.replace(/\\/g, "/").split("/").pop()?.replace(/\.[^.]+$/, "") ?? "";
                  const stringsDir = espPath.replace(/\\/g, "/").split("/").slice(0, -1).join("/");
                  const result = await delocalizeEsp({
                    esp_path: espPath,
                    strings_dir: stringsDir,
                    base_name: baseName,
                    language: language,
                    create_backup: true,
                  });
                  toast.success(`Delocalized: ${result.new_string_count} strings`);
                } catch (e: any) {
                  toast.error(`Delocalize failed: ${e}`);
                }
              }}
              disabled={isLoading || !espPath}
              title={t("menu.delocalizeEsp", { defaultValue: "Delocalize ESP" })}
            >
              {t("menu.delocalizeEsp", { defaultValue: "Delocalize" })}
            </Button>
          )}
        </div>

        <div className="toolbar-group" role="group" aria-label="TCSC conversion">
          <Button
            onClick={async () => {
              const selectedItem = useAppStore.getState().selectedItem;
              if (!selectedItem?.translation) {
                toast.error(t("menu.noTranslationToConvert"));
                return;
              }
              try {
                const result = await tcscConvert(selectedItem.translation, "to_simplified");
                await updateTranslation(selectedItem.id, result);
                useAppStore.getState().updateItemTranslation(selectedItem.id, result);
                toast.success(t("menu.tcsc_simplified"));
              } catch (e: any) {
                toast.error(`${t("menu.tcscFailed")}: ${e}`);
              }
            }}
            disabled={isLoading}
            title={t("menu.tcsc_simplified")}
          >
            {t("menu.tcsc_simplified")}
          </Button>
          <Button
            onClick={async () => {
              const selectedItem = useAppStore.getState().selectedItem;
              if (!selectedItem?.translation) {
                toast.error("No translation to convert");
                return;
              }
              try {
                const result = await tcscConvert(selectedItem.translation, "to_traditional");
                await updateTranslation(selectedItem.id, result);
                useAppStore.getState().updateItemTranslation(selectedItem.id, result);
                toast.success(t("menu.tcsc_traditional"));
              } catch (e: any) {
                toast.error(`${t("menu.tcscFailed")}: ${e}`);
              }
            }}
            disabled={isLoading}
            title={t("menu.tcsc_traditional")}
          >
            {t("menu.tcsc_traditional")}
          </Button>
          <Button
            variant="ghost"
            onClick={async () => {
              const allItems = useAppStore.getState().allItems;
              const hasTranslations = allItems.some((item) => item.translation && item.translation.trim() !== "");
              if (!hasTranslations) {
                toast.error(t("menu.noTranslationsToConvert"));
                return;
              }
              const confirmed = window.confirm(
                t("menu.tcsc_batch_confirm_simplified", {
                  defaultValue: `Convert ALL ${allItems.filter((i) => i.translation).length} translations to Simplified Chinese?`,
                })
              );
              if (!confirmed) return;
              try {
                const updatedIds = await tcscBatchConvert("to_simplified");
                await useAppStore.getState().loadAllStrings();
                toast.success(
                  t("menu.tcsc_batch_done", { defaultValue: `Converted ${updatedIds.length} translations` })
                );
              } catch (e: any) {
                toast.error(`${t("menu.batchTcscFailed")}: ${e}`);
              }
            }}
            disabled={isLoading || !espPath}
            title={t("menu.tcsc_batch_simplified", { defaultValue: "Batch: Convert all to Simplified Chinese" })}
          >
            {t("menu.tcsc_batch_simplified", { defaultValue: "简↹" })}
          </Button>
          <Button
            variant="ghost"
            onClick={async () => {
              const allItems = useAppStore.getState().allItems;
              const hasTranslations = allItems.some((item) => item.translation && item.translation.trim() !== "");
              if (!hasTranslations) {
                toast.error(t("menu.noTranslationsToConvert"));
                return;
              }
              const confirmed = window.confirm(
                t("menu.tcsc_batch_confirm_traditional", {
                  defaultValue: `Convert ALL ${allItems.filter((i) => i.translation).length} translations to Traditional Chinese?`,
                })
              );
              if (!confirmed) return;
              try {
                const updatedIds = await tcscBatchConvert("to_traditional");
                await useAppStore.getState().loadAllStrings();
                toast.success(
                  t("menu.tcsc_batch_done", { defaultValue: `Converted ${updatedIds.length} translations` })
                );
              } catch (e: any) {
                toast.error(`${t("menu.batchTcscFailed")}: ${e}`);
              }
            }}
            disabled={isLoading || !espPath}
            title={t("menu.tcsc_batch_traditional", { defaultValue: "Batch: Convert all to Traditional Chinese" })}
          >
            {t("menu.tcsc_batch_traditional", { defaultValue: "繁↹" })}
          </Button>
          <Button
            variant="ghost"
            icon={<ArrowLeftRight size={16} />}
            onClick={async () => {
              if (!espPath) return;
              try {
                const count = await compareSourceDest("diff");
                await useAppStore.getState().loadAllStrings();
                toast.success(
                  t("menu.compare_diff_done", { defaultValue: `Tagged ${count} strings where source ≠ translation` })
                );
              } catch (e: any) {
                toast.error(String(e));
              }
            }}
            disabled={isLoading || !espPath}
            title={t("menu.compare_diff", { defaultValue: "Tag: source ≠ translation" })}
          >
            ≠
          </Button>
          <Button
            variant="ghost"
            icon={<ArrowLeftRight size={16} />}
            onClick={async () => {
              if (!espPath) return;
              try {
                const count = await compareSourceDest("same");
                await useAppStore.getState().loadAllStrings();
                toast.success(
                  t("menu.compare_same_done", { defaultValue: `Tagged ${count} strings where source = translation` })
                );
              } catch (e: any) {
                toast.error(String(e));
              }
            }}
            disabled={isLoading || !espPath}
            title={t("menu.compare_same", { defaultValue: "Tag: source = translation" })}
          >
            ＝
          </Button>
        </div>

        <div className="toolbar-group toolbar-icon-group" role="group" aria-label="Tool panels">
          {([
            { id: "batch" as const, icon: <RefreshCw size={16} />, openKey: "menu.openBatchPanel", closeKey: "menu.closeBatchPanel" },
            { id: "bsa" as const, icon: <FileArchive size={16} />, openKey: "menu.openBSABrowser", closeKey: "menu.closeBSABrowser" },
            { id: "pex" as const, icon: <Braces size={16} />, openKey: "menu.openPEXPanel", closeKey: "menu.closePEXPanel" },
            { id: "fuz" as const, icon: <Volume2 size={16} />, openKey: "menu.openVoicePanel", closeKey: "menu.closeVoicePanel" },
            { id: "dialog" as const, icon: <MessagesSquare size={16} />, openKey: "menu.openDialogView", closeKey: "menu.closeDialogView" },
            { id: "mcm" as const, icon: <FileText size={16} />, openKey: "menu.openMCMPanel", closeKey: "menu.closeMCMPanel" },
            { id: "espCompare" as const, icon: <GitCompare size={16} />, openKey: "menu.openESPCompare", closeKey: "menu.closeESPCompare" },
            { id: "dataConfigs" as const, icon: <Database size={16} />, openKey: "menu.openDataConfigs", closeKey: "menu.closeDataConfigs" },
          ]).map(({ id, icon, openKey, closeKey }) => (
            <Button
              key={id}
              variant="ghost"
              size="sm"
              icon={icon}
              onClick={() => setActivePanel(id)}
              active={activePanel === id}
              title={activePanel === id ? t(closeKey) : t(openKey)}
              aria-label={activePanel === id ? t(closeKey) : t(openKey)}
              aria-pressed={activePanel === id}
            />
          ))}
        </div>

        <div className="toolbar-group toolbar-selects" role="group" aria-label="Preferences">
          <select
            value={targetLang}
            onChange={(e) => setTargetLang(e.target.value)}
            className="lang-select"
            title={t("menu.targetLanguage")}
            aria-label={t("menu.targetLanguage")}
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
            <option value="light">Light</option>
            <option value="dark">Dark</option>
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
          <Button
            variant="ghost"
            size="sm"
            icon={<Settings size={16} />}
            onClick={() => setShowSettings(true)}
            title={t("settings.title", { defaultValue: "Settings" })}
            aria-label={t("settings.title", { defaultValue: "Settings" })}
          />
          <Button
            variant="ghost"
            size="sm"
            icon={<Wrench size={16} />}
            onClick={() => setShowToolbox(true)}
            title="Toolbox"
            aria-label="Toolbox"
          />
          <Button
            variant="ghost"
            size="sm"
            icon={<RotateCcw size={16} />}
            onClick={() => {
              if (isDirty && !confirm(t("app.resetConfirm"))) return;
              reset();
            }}
            title={t("app.resetWorkspace")}
            aria-label={t("app.resetWorkspace")}
          />
        </div>
      </div>
      {isParsing && <span className="menubar-status parsing">{t("app.parsing")}</span>}
      {isLoading && <span className="menubar-status loading">{t("app.loading")}</span>}
      {isDirty && <span className="menubar-status dirty" title={t("app.unsavedChanges")}>●</span>}
      {espMode && <span className="menubar-status esp-mode" title={t("sidebar.espMode", { defaultValue: "ESP write-back mode" })}>ESP</span>}
      {batchProgress && batchProgress.total_files > 0 && (
        <span className="menubar-status batch-progress" title={`${batchProgress.message}`}>
          Batch: {batchProgress.strings_translated}/{batchProgress.total_strings} ({batchProgress.current_file}/{batchProgress.total_files} files)
        </span>
      )}
      <SettingsDialog open={showSettings} onClose={() => setShowSettings(false)} />
      <ToolboxDialog
        open={showToolbox}
        onClose={() => setShowToolbox(false)}
        selectedIds={Array.from(useAppStore.getState().selectedIds)}
        onApplied={() => {
          useAppStore.getState().loadAllStrings();
          setShowToolbox(false);
        }}
      />
    </div>
  );
}
