import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useAppStore } from "../stores/appStore";
import { Button, Input } from "./ui";
import { loadEsp, loadSst, saveSst, exportXml, importXml, saveStrings, saveEsp, tcscConvert, tcscBatchConvert, updateTranslation, loadVocabulary, compareSourceDest, loadDataConfigs, delocalizeEsp, type BatchProgress } from "../api/strings";
import type { LoadSstResponse, XmlImportResponse } from "../api/strings";
import { open, save } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { FolderOpen, FileUp, FileDown, FileCode, Save, RotateCcw, RefreshCw, FileArchive, Braces, Volume2, MessagesSquare, FileText, GitCompare, CheckCircle, Settings, ArrowLeftRight, Database, Wrench, Search, Code2, Sparkles } from "lucide-react";
import toast from "react-hot-toast";
import { useTranslation } from "react-i18next";
import { setI18nLanguage, SUPPORTED_LANGS } from "../i18n";
import { SettingsDialog } from "./SettingsDialog";
import { ToolboxDialog } from "./ToolboxDialog";
import { SpellCheckSettingsDialog } from "./SpellCheckSettingsDialog";

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

type MenuId = "file" | "translate" | "options" | "tools" | "wizards";

type MenuItem = {
  label: string;
  onClick?: () => void;
  shortcut?: string;
  disabled?: boolean;
  separator?: boolean;
};

const TARGET_LANGUAGE_CODES: Record<string, string> = {
  chinese: "zh",
  japanese: "ja",
  korean: "ko",
  french: "fr",
  german: "de",
  spanish: "es",
  italian: "it",
  russian: "ru",
  polish: "pl",
  portuguese: "pt",
  brazilian: "pt-BR",
  czech: "cs",
  hungarian: "hu",
};

const TARGET_LANGUAGE_FALLBACKS: Record<string, string> = {
  chinese: "Chinese",
  japanese: "Japanese",
  korean: "Korean",
  french: "French",
  german: "German",
  spanish: "Spanish",
  italian: "Italian",
  russian: "Russian",
  polish: "Polish",
  portuguese: "Portuguese",
  brazilian: "Brazilian Portuguese",
  czech: "Czech",
  hungarian: "Hungarian",
};

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
  const menuStripRef = useRef<HTMLDivElement | null>(null);
  const [openMenu, setOpenMenu] = useState<MenuId | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [showToolbox, setShowToolbox] = useState(false);
  const [showSpellCheck, setShowSpellCheck] = useState(false);
  const [spellCheckCfg, setSpellCheckCfg] = useState<import("../api/strings").SpellCheckConfigDto | null>(null);
  const targetLanguageLabels = useMemo(() => {
    let displayNames: Intl.DisplayNames | null = null;
    try {
      displayNames = new Intl.DisplayNames([i18n.language], { type: "language" });
    } catch {
      displayNames = null;
    }

    return Object.fromEntries(
      Object.entries(TARGET_LANGUAGE_CODES).map(([key, code]) => [
        key,
        displayNames?.of(code) ?? TARGET_LANGUAGE_FALLBACKS[key] ?? code,
      ])
    ) as Record<string, string>;
  }, [i18n.language]);
  const isParsing = useAppStore((s) => s.isParsing);
  const isLoading = useAppStore((s) => s.isLoading);
  const espPath = useAppStore((s) => s.espPath);
  const language = useAppStore((s) => s.language);
  const isDirty = useAppStore((s) => s.isDirty);
  const targetLang = useAppStore((s) => s.targetLang);
  const selectedId = useAppStore((s) => s.selectedId);
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
  const toggleBottomPanel = useAppStore((s) => s.toggleBottomPanel);
  const setDataConfigs = useAppStore((s) => s.setDataConfigs);
  const espMode = useAppStore((s) => s.espMode);
  const batchEntries = useAppStore((s) => s.batchEntries);
  const filter = useAppStore((s) => s.filter);
  const useRegex = useAppStore((s) => s.useRegex);
  const statusFilter = useAppStore((s) => s.statusFilter);
  const vmadFilter = useAppStore((s) => s.vmadFilter);
  const setFilter = useAppStore((s) => s.setFilter);
  const setUseRegex = useAppStore((s) => s.setUseRegex);
  const setStatusFilter = useAppStore((s) => s.setStatusFilter);
  const setVmadFilter = useAppStore((s) => s.setVmadFilter);
  const [batchProgress, setBatchProgress] = useState<BatchProgress | null>(null);

  useEffect(() => {
    const closeMenu = (event: MouseEvent) => {
      if (menuStripRef.current && !menuStripRef.current.contains(event.target as Node)) {
        setOpenMenu(null);
      }
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpenMenu(null);
    };
    document.addEventListener("mousedown", closeMenu);
    window.addEventListener("keydown", closeOnEscape);
    return () => {
      document.removeEventListener("mousedown", closeMenu);
      window.removeEventListener("keydown", closeOnEscape);
    };
  }, []);

  const warnIfBatchFile = useCallback((path: string) => {
    const normalizedPath = path.replace(/\\/g, "/").toLowerCase();
    const isBatchFile = batchEntries.some(
      (entry) => entry.esp_path.replace(/\\/g, "/").toLowerCase() === normalizedPath
    );
    if (isBatchFile && activePanel !== "batch") {
      toast(t("menu.batchConflictToast"), { icon: "!", duration: 4000 });
    }
  }, [batchEntries, activePanel, t]);

  const loadEspFromPath = useCallback(async (path: string) => {
    if (isDirty && !confirm(t("app.resetConfirm"))) return;
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
        toast.success(t("toast.espLoaded", { count: stats.total.toLocaleString() }));

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
      toast.error(`${t("toast.loadingFailed")}: ${e}`);
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
      toast.success(t("toast.sstLoaded", { matched: stats.matched, unmatched: stats.unmatched }) + formatApplyStats(stats));
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
      toast(t("menu.dragDropBsa"), { icon: "📦", duration: 3000 });
      return;
    }
    if (ext === "pex") {
      setActivePanel("pex");
      toast(t("menu.dragDropPex"), { icon: "📜", duration: 3000 });
      return;
    }
    if (ext === "fuz") {
      setActivePanel("fuz");
      toast(t("menu.dragDropFuz"), { icon: "🔊", duration: 3000 });
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
        toast.success(t("menu.espSaved", { count: result.records_modified }));
      } catch (e: any) {
        toast.error(`${t("menu.saveEspFailed")}: ${e}`);
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

  const closeAndRun = (action?: () => void) => {
    setOpenMenu(null);
    action?.();
  };

  const openSelectedEditor = () => {
    if (selectedId === null) {
      toast.error(t("menu.selectStringFirst"));
      return;
    }
    useAppStore.getState().openEditorForItem(selectedId);
  };

  const convertSelectedTranslation = async (mode: "to_simplified" | "to_traditional") => {
    const item = useAppStore.getState().selectedItem;
    if (!item?.translation) {
      toast.error(t("menu.noTranslationToConvert"));
      return;
    }
    try {
      const result = await tcscConvert(item.translation, mode);
      await updateTranslation(item.id, result);
      useAppStore.getState().updateItemTranslation(item.id, result);
      toast.success(mode === "to_simplified" ? t("menu.tcsc_simplified") : t("menu.tcsc_traditional"));
    } catch (e: any) {
      toast.error(`${t("menu.tcscFailed")}: ${e}`);
    }
  };

  const compareSourceDestFromMenu = async (mode: "diff" | "same") => {
    if (!espPath) return;
    try {
      const count = await compareSourceDest(mode);
      await useAppStore.getState().loadAllStrings();
      toast.success(
        mode === "diff"
          ? t("menu.compare_diff_done", { count })
          : t("menu.compare_same_done", { count })
      );
    } catch (e: any) {
      toast.error(String(e));
    }
  };

  const menuDefinitions: Array<{ id: MenuId; label: string; items: MenuItem[] }> = [
    {
      id: "file",
      label: t("menu.file"),
      items: [
        { label: t("common.loadEsp"), onClick: () => void handleLoadEsp(), shortcut: "Ctrl+O" },
        { label: t("common.loadSst"), onClick: () => void handleLoadSst(), shortcut: "Ctrl+L" },
        { label: t("common.saveSst"), onClick: () => void handleSaveSst(), shortcut: "Ctrl+S" },
        { label: t("common.saveStrings"), onClick: () => void handleSaveStrings() },
        { separator: true, label: "" },
        { label: t("common.exportXml"), onClick: () => void handleExportXml() },
        { label: t("common.importXml"), onClick: () => void handleImportXml() },
        { separator: true, label: "" },
        { label: t("app.resetWorkspace"), onClick: () => reset() },
      ],
    },
    {
      id: "translate",
      label: t("menu.translate"),
      items: [
        { label: t("menu.openEditor"), onClick: openSelectedEditor, shortcut: "Enter" },
        { label: t("finalize.title"), onClick: () => setActivePanel("finalize") },
        { separator: true, label: "" },
        { label: t("menu.tcsc_simplified"), onClick: () => void convertSelectedTranslation("to_simplified"), shortcut: "简" },
        { label: t("menu.tcsc_traditional"), onClick: () => void convertSelectedTranslation("to_traditional"), shortcut: "繁" },
        { separator: true, label: "" },
        { label: t("menu.compare_diff"), onClick: () => void compareSourceDestFromMenu("diff") },
        { label: t("menu.compare_same"), onClick: () => void compareSourceDestFromMenu("same") },
      ],
    },
    {
      id: "options",
      label: t("menu.options"),
      items: [
        { label: t("settings.title"), onClick: () => setShowSettings(true) },
        { label: t("menu.toolbox"), onClick: () => setShowToolbox(true) },
        { label: t("spellcheck.title", { defaultValue: "Spell Check" }), onClick: () => setShowSpellCheck(true) },
        { separator: true, label: "" },
        { label: t("sidebar.espMode"), onClick: () => useAppStore.getState().setEspMode(!espMode) },
        { label: t("menu.toggleBottomPanel"), onClick: () => toggleBottomPanel() },
        { label: t("app.resetWorkspace"), onClick: () => reset() },
      ],
    },
    {
      id: "tools",
      label: t("menu.tools"),
      items: [
        { label: t("batch.title"), onClick: () => setActivePanel("batch") },
        { label: t("bsa.title"), onClick: () => setActivePanel("bsa") },
        { label: t("pex.title"), onClick: () => setActivePanel("pex") },
        { label: t("fuz.title"), onClick: () => setActivePanel("fuz") },
        { label: t("dialog.title"), onClick: () => setActivePanel("dialog") },
        { label: t("mcm.title"), onClick: () => setActivePanel("mcm") },
        { label: t("espCompare.title"), onClick: () => setActivePanel("espCompare") },
        { label: t("dataConfigs.title"), onClick: () => setActivePanel("dataConfigs") },
      ],
    },
    {
      id: "wizards",
      label: t("menu.wizards"),
      items: [
        { label: t("bottomTabs.headerProc"), onClick: () => useAppStore.getState().setActiveBottomTab("headerProc") },
        { label: t("bottomTabs.headerWizard"), onClick: () => useAppStore.getState().setActiveBottomTab("headerWizard") },
        { separator: true, label: "" },
        { label: t("bottomTabs.home"), onClick: () => useAppStore.getState().setActiveBottomTab("home") },
      ],
    },
  ];

  const renderMenu = (menu: { id: MenuId; label: string; items: MenuItem[] }) => (
    <div className={`menubar-menu ${openMenu === menu.id ? "open" : ""}`} key={menu.id}>
      <button
        type="button"
        className="menubar-menu-trigger"
        onClick={() => setOpenMenu(openMenu === menu.id ? null : menu.id)}
        aria-haspopup="menu"
        aria-expanded={openMenu === menu.id}
      >
        {menu.label}
      </button>
      {openMenu === menu.id && (
        <div className="menubar-menu-panel" role="menu">
          {menu.items.map((item, index) =>
            item.separator ? (
              <div key={`sep-${index}`} className="menubar-menu-separator" />
            ) : (
              <button
                key={item.label}
                type="button"
                className="menubar-menu-item"
                onClick={() => closeAndRun(item.onClick)}
                disabled={item.disabled}
                role="menuitem"
              >
                <span className="menubar-menu-item-label">{item.label}</span>
                {item.shortcut && <span className="menubar-menu-item-shortcut">{item.shortcut}</span>}
              </button>
            )
          )}
        </div>
      )}
    </div>
  );

  return (
    <div className="menubar">
      <div className="menubar-topline" ref={menuStripRef}>
        <div className="menubar-brand">xTranslator(x64)</div>
        <div className="menubar-menu-strip" role="menubar" aria-label="Application menus">
          {menuDefinitions.map(renderMenu)}
        </div>
      </div>
      <div className="menubar-actions" role="toolbar" aria-label="Application actions">
        <div className="toolbar-group" role="group" aria-label="Search">
          <Input
            size="sm"
            icon={<Search size={14} />}
            placeholder={useRegex ? t("common.regexFilter") : t("common.filter")}
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            wrapperClassName="menubar-search-input"
            suffix={
              <Button
                variant="ghost"
                size="xs"
                onClick={() => setUseRegex(!useRegex)}
                title={useRegex ? t("table.regexSwitchTip") : t("table.plainSwitchTip")}
                active={useRegex}
              >
                <Code2 size={12} />
              </Button>
            }
          />
        </div>

        <div className="toolbar-group" role="group" aria-label="Status filters">
          {[
            { key: "translated", label: "✓", title: t("common.translated") },
            { key: "incomplete", label: "✗", title: t("common.incomplete") },
            { key: "locked", label: "🔒", title: t("common.locked") },
          ].map((s) => (
            <Button
              key={s.key}
              variant="ghost"
              size="xs"
              active={statusFilter === s.key}
              onClick={() => setStatusFilter(statusFilter === s.key ? null : s.key)}
              title={s.title}
            >
              {s.label}
            </Button>
          ))}
          <Button
            variant="ghost"
            size="xs"
            active={vmadFilter}
            onClick={() => setVmadFilter(!vmadFilter)}
            title={vmadFilter ? t("vmad.showAll") : t("vmad.showOnly")}
          >
            VMAD
          </Button>
        </div>

        <div className="toolbar-group toolbar-group-primary" role="group" aria-label="Files">
          <Button variant="primary" size="sm" icon={<FolderOpen size={14} />} onClick={handleLoadEsp} disabled={isParsing}>
            {t("common.loadEsp")}
          </Button>
          <Button size="sm" icon={<FileUp size={14} />} onClick={handleLoadSst} disabled={isLoading || !espPath}>
            {t("common.loadSst")}
          </Button>
          <Button size="sm" icon={<FileDown size={14} />} onClick={handleSaveSst} disabled={isLoading || !espPath}>
            {t("common.saveSst")}
          </Button>
          <Button size="sm" icon={<Save size={14} />} onClick={handleSaveStrings} disabled={isLoading || !espPath}>
            {t("common.saveStrings")}
          </Button>
        </div>

        <div className="toolbar-group" role="group" aria-label="Exchange formats">
          <Button size="sm" icon={<FileCode size={14} />} onClick={handleExportXml} disabled={isLoading || !espPath}>
            {t("common.exportXml")}
          </Button>
          <Button size="sm" icon={<FileCode size={14} />} onClick={handleImportXml} disabled={isLoading || !espPath}>
            {t("common.importXml")}
          </Button>
        </div>

        <div className="toolbar-group" role="group" aria-label="Finalize">
          <Button
            variant="primary"
            size="sm"
            icon={<CheckCircle size={14} />}
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
              size="sm"
              icon={<FileCode size={14} />}
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
                  toast.success(t("menu.delocalizedEsp", { count: result.new_string_count }));
                } catch (e: any) {
                  toast.error(t("menu.delocalizeFailed", { error: String(e) }));
                }
              }}
              disabled={isLoading || !espPath}
              title={t("menu.delocalizeEsp")}
            >
              {t("menu.delocalizeEsp")}
            </Button>
          )}
        </div>

        <div className="toolbar-group" role="group" aria-label="TCSC conversion">
          <Button
            size="xs"
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
            简
          </Button>
          <Button
            size="xs"
            onClick={async () => {
              const selectedItem = useAppStore.getState().selectedItem;
              if (!selectedItem?.translation) {
                toast.error(t("menu.noTranslationToConvert"));
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
            繁
          </Button>
          <Button
            variant="ghost"
            size="xs"
            onClick={async () => {
              const allItems = useAppStore.getState().allItems;
              const hasTranslations = allItems.some((item) => item.translation && item.translation.trim() !== "");
              if (!hasTranslations) {
                toast.error(t("menu.noTranslationsToConvert"));
                return;
              }
              const confirmed = window.confirm(
                t("menu.tcsc_batch_confirm_simplified", {
                  count: allItems.filter((i) => i.translation).length,
                })
              );
              if (!confirmed) return;
              try {
                const updatedIds = await tcscBatchConvert("to_simplified");
                await useAppStore.getState().loadAllStrings();
                toast.success(t("menu.tcsc_batch_done", { count: updatedIds.length }));
              } catch (e: any) {
                toast.error(`${t("menu.batchTcscFailed")}: ${e}`);
              }
            }}
            disabled={isLoading || !espPath}
            title={t("menu.tcsc_batch_simplified")}
          >
            简↹
          </Button>
          <Button
            variant="ghost"
            size="xs"
            onClick={async () => {
              const allItems = useAppStore.getState().allItems;
              const hasTranslations = allItems.some((item) => item.translation && item.translation.trim() !== "");
              if (!hasTranslations) {
                toast.error(t("menu.noTranslationsToConvert"));
                return;
              }
              const confirmed = window.confirm(
                t("menu.tcsc_batch_confirm_traditional", {
                  count: allItems.filter((i) => i.translation).length,
                })
              );
              if (!confirmed) return;
              try {
                const updatedIds = await tcscBatchConvert("to_traditional");
                await useAppStore.getState().loadAllStrings();
                toast.success(t("menu.tcsc_batch_done", { count: updatedIds.length }));
              } catch (e: any) {
                toast.error(`${t("menu.batchTcscFailed")}: ${e}`);
              }
            }}
            disabled={isLoading || !espPath}
            title={t("menu.tcsc_batch_traditional")}
          >
            繁↹
          </Button>
          <Button
            variant="ghost"
            size="xs"
            icon={<ArrowLeftRight size={14} />}
            onClick={async () => {
              if (!espPath) return;
              try {
                const count = await compareSourceDest("diff");
                await useAppStore.getState().loadAllStrings();
                toast.success(t("menu.compare_diff_done", { count }));
              } catch (e: any) {
                toast.error(String(e));
              }
            }}
            disabled={isLoading || !espPath}
            title={t("menu.compare_diff")}
          >
            ≠
          </Button>
          <Button
            variant="ghost"
            size="xs"
            icon={<ArrowLeftRight size={14} />}
            onClick={async () => {
              if (!espPath) return;
              try {
                const count = await compareSourceDest("same");
                await useAppStore.getState().loadAllStrings();
                toast.success(t("menu.compare_same_done", { count }));
              } catch (e: any) {
                toast.error(String(e));
              }
            }}
            disabled={isLoading || !espPath}
            title={t("menu.compare_same")}
          >
            ＝
          </Button>
        </div>

        <div className="toolbar-group toolbar-icon-group" role="group" aria-label="Tool panels">
          {([
            { id: "batch" as const, icon: <RefreshCw size={14} />, openLabel: t("menu.openBatchPanel"), closeLabel: t("menu.closeBatchPanel") },
            { id: "bsa" as const, icon: <FileArchive size={14} />, openLabel: t("menu.openBsaBrowser"), closeLabel: t("menu.closeBsaBrowser") },
            { id: "pex" as const, icon: <Braces size={14} />, openLabel: t("menu.openPexPanel"), closeLabel: t("menu.closePexPanel") },
            { id: "fuz" as const, icon: <Volume2 size={14} />, openLabel: t("menu.openVoicePanel"), closeLabel: t("menu.closeVoicePanel") },
            { id: "dialog" as const, icon: <MessagesSquare size={14} />, openLabel: t("menu.openDialogView"), closeLabel: t("menu.closeDialogView") },
            { id: "mcm" as const, icon: <FileText size={14} />, openLabel: t("menu.openMcmPanel"), closeLabel: t("menu.closeMcmPanel") },
            { id: "espCompare" as const, icon: <GitCompare size={14} />, openLabel: t("menu.openEspCompare"), closeLabel: t("menu.closeEspCompare") },
            { id: "dataConfigs" as const, icon: <Database size={14} />, openLabel: t("menu.openDataConfigs"), closeLabel: t("menu.closeDataConfigs") },
          ]).map(({ id, icon, openLabel, closeLabel }) => (
            <Button
              key={id}
              variant="ghost"
              size="sm"
              icon={icon}
              onClick={() => setActivePanel(id)}
              active={activePanel === id}
              title={activePanel === id ? closeLabel : openLabel}
              aria-label={activePanel === id ? closeLabel : openLabel}
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
            {Object.entries(TARGET_LANGUAGE_CODES).map(([key]) => (
              <option key={key} value={key}>
                {targetLanguageLabels[key]}
              </option>
            ))}
          </select>
          <select
            value={theme}
            onChange={(e) => setTheme(e.target.value as any)}
            className="lang-select"
            title={t("common.theme")}
            aria-label={t("common.theme")}
          >
            <option value="obsidian">{t("theme.obsidian")}</option>
            <option value="slate">{t("theme.slate")}</option>
            <option value="light">{t("theme.light")}</option>
            <option value="auto">{t("theme.auto")}</option>
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
            size="xs"
            icon={<Settings size={14} />}
            onClick={() => setShowSettings(true)}
            title={t("settings.title")}
            aria-label={t("settings.title")}
          />
          <Button
            variant="ghost"
            size="xs"
            icon={<Wrench size={14} />}
            onClick={() => setShowToolbox(true)}
            title={t("menu.toolbox")}
            aria-label={t("menu.toolbox")}
          />
          <Button
            variant="ghost"
            size="xs"
            icon={<Sparkles size={14} />}
            onClick={() => setShowSpellCheck(true)}
            title={t("spellcheck.title", { defaultValue: "Spell Check" })}
            aria-label={t("spellcheck.title", { defaultValue: "Spell Check" })}
            className={spellCheckCfg?.active ? "menubar-btn-active" : ""}
          />
          <Button
            variant="ghost"
            size="xs"
            icon={<RotateCcw size={14} />}
            onClick={() => {
              if (isDirty && !confirm(t("app.resetConfirm"))) return;
              reset();
            }}
            title={t("app.resetWorkspace")}
            aria-label={t("app.resetWorkspace")}
          />
        </div>
        <div className="menubar-status-group">
          {isParsing && <span className="menubar-status parsing">{t("app.parsing")}</span>}
          {isLoading && <span className="menubar-status loading">{t("app.loading")}</span>}
          {isDirty && <span className="menubar-status dirty" title={t("app.unsavedChanges")}>●</span>}
          {espMode && <span className="menubar-status esp-mode" title={t("sidebar.espMode")}>{t("sidebar.espMode")}</span>}
          {batchProgress && batchProgress.total_files > 0 && (
            <span className="menubar-status batch-progress" title={`${batchProgress.message}`}>
              {t("batch.progressSummary", {
                translated: batchProgress.strings_translated,
                total: batchProgress.total_strings,
                current: batchProgress.current_file,
                files: batchProgress.total_files,
              })}
            </span>
          )}
        </div>
      </div>
      <SettingsDialog open={showSettings} onClose={() => setShowSettings(false)} />
      <SpellCheckSettingsDialog
        open={showSpellCheck}
        onClose={() => setShowSpellCheck(false)}
        dllPath="Bin/x64/libhunspell.dll"
        dictDir="SpellCheck/dictionaries"
        onConfigChanged={setSpellCheckCfg}
      />
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
