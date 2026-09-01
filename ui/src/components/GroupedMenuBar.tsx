import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useAppStore } from "../stores/appStore";
import { Button, Input } from "./ui";
import {
  loadEsp,
  loadSst,
  saveSst,
  exportXml,
  importXml,
  saveStrings,
  saveEsp,
  tcscConvert,
  tcscBatchConvert,
  updateTranslation,
  loadVocabulary,
  compareSourceDest,
  loadDataConfigs,
  delocalizeEsp,
  loadConfig,
  spellCheckLoad,
  spellCheckToggle,
  spellCheckConfig,
  SUPPORTED_GAME_IDS,
  type BatchProgress,
} from "../api/strings";
import type { LoadSstResponse, XmlImportResponse, SupportedGameId, SstApplyOptions } from "../api/strings";
import { ApplySstDialog } from "./ApplySstDialog";
import { AdvSearchDialog } from "./AdvSearchDialog";
import { open, save } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { requestedGameForLoad } from "../gameContext";
import {
  FolderOpen,
  FileUp,
  FileDown,
  FileCode,
  Save,
  RotateCcw,
  RefreshCw,
  FileArchive,
  Braces,
  Volume2,
  MessagesSquare,
  FileText,
  GitCompare,
  CheckCircle,
  Settings,
  ArrowLeftRight,
  Database,
  Wrench,
  Search,
  Code2,
  Sparkles,
} from "lucide-react";
import toast from "react-hot-toast";
import { useTranslation } from "react-i18next";
import { setI18nLanguage, SUPPORTED_LANGS } from "../i18n";
import { SettingsDialog } from "./SettingsDialog";
import { ToolboxDialog } from "./ToolboxDialog";
import { SpellCheckSettingsDialog } from "./SpellCheckSettingsDialog";
import { MergeSstDialog } from "./MergeSstDialog";

// ============================================================================
// GroupedMenuBar 组件 - 分组菜单栏
// ============================================================================
//
// 将替代现有 MenuBar.tsx，提供分组下拉菜单：
//   1. File      - 文件操作（加载、保存、导入导出）
//   2. Edit      - 编辑操作（撤销、重做、替换）
//   3. Search    - 搜索过滤（状态过滤切换）
//   4. Translate - 翻译操作（编辑、完成、转换）
//   5. Tools     - 工具面板（批处理、BSA、PEX 等）
//   6. View      - 视图选项（面板、编辑器模式）
//
// 工具栏包含：
//   - 搜索框（支持正则表达式）
//   - 状态过滤按钮（已翻译、未完成、已锁定、VMAD）
//   - 文件操作按钮（加载 ESP、加载/保存 SST、保存字符串）
//   - XML 导入导出按钮
//   - 完成按钮
//   - 简繁转换按钮
//   - 工具面板图标按钮（8 个）
//   - 选择下拉框（目标语言、主题、UI 语言）
//   - 状态指示器（解析中、加载中、已修改、ESP 模式、批处理进度）
//
// ============================================================================

// ============================================================================
// 类型定义
// ============================================================================

/**
 * 应用统计信息类型
 * 用于显示 SST 加载或 XML 导入的匹配统计
 */
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

/** 菜单组 ID 类型：6 个菜单组 */
type MenuGroupId = "file" | "edit" | "search" | "translate" | "tools" | "view";

/** 菜单项类型：支持标签、点击、快捷键、禁用、分隔符 */
type MenuItem = {
  label: string;
  onClick?: () => void;
  shortcut?: string;
  disabled?: boolean;
  separator?: boolean;
};

/** 菜单组定义类型 */
type MenuGroup = {
  id: MenuGroupId;
  label: string;
  items: MenuItem[];
};

/** 目标语言代码映射 */
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

/** 目标语言显示名称（备用） */
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

// ============================================================================
// 工具函数
// ============================================================================

/**
 * 获取文件扩展名
 * @param path - 文件路径
 * @returns 小写扩展名（不含点号）
 */
function getPathExt(path: string): string {
  const fileName = path.replace(/\\/g, "/").split("/").pop() ?? "";
  const dotIndex = fileName.lastIndexOf(".");
  return dotIndex >= 0 ? fileName.slice(dotIndex + 1).toLowerCase() : "";
}

/**
 * 格式化应用统计信息
 * @param stats - 应用统计对象
 * @returns 格式化的统计字符串
 */
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

// ============================================================================
// GroupedMenuBar 主组件
// ============================================================================

/**
 * 分组菜单栏组件
 *
 * 职责：
 *   - 渲染 6 个菜单组触发按钮
 *   - 管理下拉面板的打开/关闭状态（互斥）
 *   - 渲染完整工具栏
 *   - 处理所有文件操作（ESP、SST、XML）
 *   - 处理拖放文件操作
 *   - 管理搜索和过滤功能
 *   - 监听批处理进度事件
 */
export function GroupedMenuBar() {
  // ========== 国际化和 Ref ==========
  const { t, i18n } = useTranslation();
  const menuBarRef = useRef<HTMLDivElement | null>(null);

  // ========== 菜单状态 ==========
  const [openGroup, setOpenGroup] = useState<MenuGroupId | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [showToolbox, setShowToolbox] = useState(false);
  const [showSpellCheck, setShowSpellCheck] = useState(false);
  const [showMergeSst, setShowMergeSst] = useState(false);
  const [applySstDialogOpen, setApplySstDialogOpen] = useState(false);
  const [advSearchOpen, setAdvSearchOpen] = useState(false);
  const [pendingSstPath, setPendingSstPath] = useState<string | null>(null);
  const [spellCheckCfg, setSpellCheckCfg] = useState<
    import("../api/strings").SpellCheckConfigDto | null
  >(null);

  const selectedIds = useAppStore((s) => s.selectedIds);
  const items = useAppStore((s) => s.items);

  // ========== 自动恢复拼写检查 ==========
  useEffect(() => {
    loadConfig().then((cfg) => {
      if (!cfg.spellcheck_loaded || !cfg.spellcheck_dictionary) return;
      spellCheckLoad(
        "Bin/x64/libhunspell.dll",
        "SpellCheck/dictionaries",
        cfg.spellcheck_dictionary
      )
        .then((result) => {
          if (cfg.spellcheck_active === false) {
            return spellCheckToggle().then(() =>
              spellCheckConfig("SpellCheck/dictionaries")
            );
          }
          return result;
        })
        .then((result) => {
          if (result) setSpellCheckCfg(result);
        })
        .catch(() => {});
    });
  }, []);

  // ========== 目标语言标签 ==========
  const targetLanguageLabels = useMemo(() => {
    let displayNames: Intl.DisplayNames | null = null;
    try {
      displayNames = new Intl.DisplayNames([i18n.language], {
        type: "language",
      });
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

  // ========== 从 Store 订阅状态 ==========
  const isParsing = useAppStore((s) => s.isParsing);
  const isLoading = useAppStore((s) => s.isLoading);
  const espPath = useAppStore((s) => s.espPath);
  const language = useAppStore((s) => s.language);
  const isDirty = useAppStore((s) => s.isDirty);
  const targetLang = useAppStore((s) => s.targetLang);
  const currentGame = useAppStore((s) => s.currentGame);
  const gameSelectionMode = useAppStore((s) => s.gameSelectionMode);
  const selectedId = useAppStore((s) => s.selectedId);
  const activePanel = useAppStore((s) => s.activePanel);
  const activeRightPanel = useAppStore((s) => s.activeRightPanel);
  const espMode = useAppStore((s) => s.espMode);
  const batchEntries = useAppStore((s) => s.batchEntries);
  const filter = useAppStore((s) => s.filter);
  const useRegex = useAppStore((s) => s.useRegex);
  const statusFilter = useAppStore((s) => s.statusFilter);
  const vmadFilter = useAppStore((s) => s.vmadFilter);
  const theme = useAppStore((s) => s.theme);

  // ========== 从 Store 订阅操作函数 ==========
  const setParsing = useAppStore((s) => s.setParsing);
  const setLoading = useAppStore((s) => s.setLoading);
  const setError = useAppStore((s) => s.setError);
  const setLoadProgress = useAppStore((s) => s.setLoadProgress);
  const setEspLoaded = useAppStore((s) => s.setEspLoaded);
  const setSstLoaded = useAppStore((s) => s.setSstLoaded);
  const loadAllStrings = useAppStore((s) => s.loadAllStrings);
  const setIsDirty = useAppStore((s) => s.setIsDirty);
  const setTargetLang = useAppStore((s) => s.setTargetLang);
  const setGameSelection = useAppStore((s) => s.setGameSelection);
  const reset = useAppStore((s) => s.reset);
  const setActivePanel = useAppStore((s) => s.setActivePanel);
  const setActiveRightPanel = useAppStore((s) => s.setActiveRightPanel);
  const toggleBottomPanel = useAppStore((s) => s.toggleBottomPanel);
  const setEditorMode = useAppStore((s) => s.setEditorMode);
  const setDataConfigs = useAppStore((s) => s.setDataConfigs);
  const setFilter = useAppStore((s) => s.setFilter);
  const setUseRegex = useAppStore((s) => s.setUseRegex);
  const setStatusFilter = useAppStore((s) => s.setStatusFilter);
  const setVmadFilter = useAppStore((s) => s.setVmadFilter);
  const setTheme = useAppStore((s) => s.setTheme);

  // ========== 本地状态 ==========
  const [batchProgress, setBatchProgress] = useState<BatchProgress | null>(
    null
  );

  // ========== Hook：点击外部关闭 ==========
  useEffect(() => {
    const closeMenu = (event: MouseEvent) => {
      if (
        menuBarRef.current &&
        !menuBarRef.current.contains(event.target as Node)
      ) {
        setOpenGroup(null);
      }
    };
    document.addEventListener("mousedown", closeMenu);
    return () => document.removeEventListener("mousedown", closeMenu);
  }, []);

  // ========== Hook：Escape 键关闭 ==========
  useEffect(() => {
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpenGroup(null);
    };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, []);

  // ========== 核心功能函数 ==========

  /**
   * 检查文件是否在批处理列表中，如果是则显示警告
   */
  const warnIfBatchFile = useCallback(
    (path: string) => {
      const normalizedPath = path.replace(/\\/g, "/").toLowerCase();
      const isBatchFile = batchEntries.some(
        (entry) =>
          entry.esp_path.replace(/\\/g, "/").toLowerCase() === normalizedPath
      );
      if (isBatchFile && activePanel !== "batch") {
        toast(t("menu.batchConflictToast"), { icon: "!", duration: 4000 });
      }
    },
    [batchEntries, activePanel, t]
  );

  /**
   * 从指定路径加载 ESP 文件
   */
  const loadEspFromPath = useCallback(
    async (path: string, skipDirtyConfirm = false) => {
      if (!skipDirtyConfirm && isDirty && !confirm(t("app.resetConfirm"))) return;
      warnIfBatchFile(path);

      const espDir = path.replace(/\\/g, "/").split("/").slice(0, -1).join("/");
      const stringsDir = `${espDir}/Strings`;

      setParsing(true);
      setError(null);
      setLoadProgress(null);

      try {
        const unlisten = await listen<any>("esp-load-progress", (event) => {
          setLoadProgress(event.payload);
        });

        try {
          const gameState = useAppStore.getState();
          const requestedGame = requestedGameForLoad(gameState.gameSelectionMode, gameState.currentGame);
          const stats = await loadEsp(path, stringsDir, language, requestedGame);
          setEspLoaded(path, stats, stringsDir);
          await loadAllStrings();
          setIsDirty(false);
          toast.success(
            t("toast.espLoaded", {
              count: stats.total.toLocaleString(),
            })
          );

          const store = useAppStore.getState();
          store.addLog(
            "info",
            `Game context: ${stats.game_id} (${stats.game_source}); Data/${stats.game_id}`,
            "ESP",
          );
          if (stats.game_source === "fallback") {
            store.addLog("warn", "Game auto-detection failed; select a game explicitly and reload the plugin.", "ESP");
            toast.error(
              t("menu.gameDetectionFallback", {
                defaultValue: "Game auto-detection failed. Select a game explicitly and reload the plugin.",
              }),
              { duration: 8000 },
            );
            return;
          }
          if (stats.detected_game_id && stats.detected_game_id !== stats.game_id) {
            store.addLog(
              "warn",
              `Workspace mismatch: selected ${stats.game_id}, plugin reports ${stats.detected_game_id}.`,
              "ESP",
            );
            toast(
              t("menu.gameMismatch", {
                defaultValue: "Selected game {{selected}} differs from plugin detection {{detected}}.",
                selected: stats.game_id,
                detected: stats.detected_game_id,
              }),
              { icon: "!", duration: 6000 },
            );
          }

          if (stats.esp_hash) {
            useAppStore.getState().checkAndPromptRecovery(stats.esp_hash);
          }

          loadVocabulary(
            stringsDir,
            language,
            useAppStore.getState().targetLang,
            stats.game_id
          )
            .then((info) => {
              if (info.pair_count > 0) {
                toast(
                  t("menu.vocabularyLoaded", {
                    pairs: info.pair_count.toLocaleString(),
                    files: info.base_names.length,
                  }),
                  { duration: 3000 }
                );
              }
            })
            .catch(() => {});

          loadDataConfigs(stats.game_id)
            .then((cfg) => {
              setDataConfigs(cfg);
              const fieldCount = Object.keys(cfg.field_size_ref).length;
              toast.success(
                t("menu.dataConfigsLoaded", {
                  ctda: cfg.ctda_funcs.length,
                  fields: fieldCount,
                }),
                { duration: 3000 }
              );
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
    },
    [
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
      setDataConfigs,
    ]
  );

  const handleGameSelectionChange = useCallback(async (value: string) => {
    if (espPath && isDirty && !confirm(t("app.resetConfirm"))) return;
    if (value === "auto") {
      setGameSelection("auto");
    } else {
      setGameSelection("manual", value as SupportedGameId);
    }
    if (espPath) {
      await loadEspFromPath(espPath, true);
    }
  }, [espPath, isDirty, loadEspFromPath, setGameSelection, t]);

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

  const loadSstFromPath = useCallback(
    async (path: string, options?: SstApplyOptions) => {
      if (!espPath) {
        toast.error(t("menu.espBeforeSst"));
        return;
      }

      setLoading(true);
      try {
        const stats = await loadSst(path, options);
        setSstLoaded(path, stats);
        setIsDirty(true);
        toast.success(
          t("toast.sstLoaded", {
            matched: stats.matched,
            unmatched: stats.unmatched,
          }) + formatApplyStats(stats)
        );
        await loadAllStrings();
      } catch (e: any) {
        toast.error(`${t("menu.sstSaveFailed")}: ${e}`);
      } finally {
        setLoading(false);
      }
    },
    [espPath, loadAllStrings, setIsDirty, setLoading, setSstLoaded, t]
  );

  const handleLoadSst = useCallback(async () => {
    const selected = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "SST Dictionary", extensions: ["sst"] }],
    });
    const path = Array.isArray(selected) ? selected[0] : selected;
    if (!path) return;
    setPendingSstPath(path);
    setApplySstDialogOpen(true);
  }, []);

  const handleSaveSst = async () => {
    const sstPath = await save({
      filters: [{ name: "SST Dictionary", extensions: ["sst"] }],
      defaultPath: espPath
        ? espPath
            .replace(/\\/g, "/")
            .replace(/\.es[mp]$/i, `_english_${targetLang}.sst`)
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
        ? espPath
            .replace(/\\/g, "/")
            .replace(/\.es[mp]$/i, `_english_${targetLang}.xml`)
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

  const importXmlFromPath = useCallback(
    async (path: string) => {
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
    },
    [espPath, loadAllStrings, setIsDirty, setLoadProgress, setLoading, t]
  );

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

  const handleSaveStrings = async () => {
    if (espMode && espPath) {
      setLoading(true);
      try {
        const result = await saveEsp({
          path: espPath,
          create_backup: true,
        });
        setIsDirty(false);
        toast.success(
          t("menu.espSaved", { count: result.records_modified })
        );
      } catch (e: any) {
        toast.error(`${t("menu.saveEspFailed")}: ${e}`);
      } finally {
        setLoading(false);
      }
      return;
    }

    const outputDir = await open({
      multiple: false,
      directory: true,
    });
    if (!outputDir) return;

    const baseName = espPath
      ? espPath
          .replace(/\\/g, "/")
          .split("/")
          .pop()
          ?.replace(/\.es[mp]$/i, "") || "Skyrim"
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
        t("menu.stringsSaved", {
          strings: result.strings_count,
          dlstrings: result.dlstrings_count,
          ilstrings: result.ilstrings_count,
          translated: result.translated_count,
        })
      );
    } catch (e: any) {
      toast.error(`${t("menu.saveStringsFailed")}: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  // ========== 编辑器操作 ==========
  const openSelectedEditor = () => {
    if (selectedId === null) {
      toast.error(t("menu.selectStringFirst"));
      return;
    }
    useAppStore.getState().openEditorForItem(selectedId);
  };

  const convertSelectedTranslation = async (
    mode: "to_simplified" | "to_traditional"
  ) => {
    const item = useAppStore.getState().selectedItem;
    if (!item?.translation) {
      toast.error(t("menu.noTranslationToConvert"));
      return;
    }
    try {
      const result = await tcscConvert(item.translation, mode);
      await updateTranslation(item.id, result);
      useAppStore.getState().updateItemTranslation(item.id, result);
      toast.success(
        mode === "to_simplified"
          ? t("menu.tcsc_simplified")
          : t("menu.tcsc_traditional")
      );
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

  // ========== 拖放文件路由 ==========
  const routeDroppedPath = useCallback(
    (path: string) => {
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
        setActiveRightPanel("bsa");
        toast(t("menu.dragDropBsa"), { icon: "📦", duration: 3000 });
        return;
      }
      if (ext === "pex") {
        setActiveRightPanel("pex");
        toast(t("menu.dragDropPex"), { icon: "📜", duration: 3000 });
        return;
      }
      if (ext === "fuz") {
        setActiveRightPanel("fuz");
        toast(t("menu.dragDropFuz"), { icon: "🔊", duration: 3000 });
        return;
      }

      toast.error(t("menu.dragDropUnsupported"));
    },
    [importXmlFromPath, loadEspFromPath, loadSstFromPath, setActiveRightPanel, t]
  );

  // ========== Hook：拖放事件处理 ==========
  useEffect(() => {
    let disposed = false;
    let unlistenDragDrop: (() => void) | null = null;

    try {
      getCurrentWebview()
        .onDragDropEvent((event) => {
          if (event.payload.type !== "drop") return;

          const firstSupportedPath =
            event.payload.paths.find((path) =>
              ["esp", "esm", "sst", "xml"].includes(getPathExt(path))
            ) ?? event.payload.paths[0];

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
          /* 拖放在浏览器/非 Tauri 环境中不可用 */
        });
    } catch {
      /* getCurrentWebview 在非 Tauri 环境中不可用 */
    }

    return () => {
      disposed = true;
      unlistenDragDrop?.();
    };
  }, [routeDroppedPath]);

  // ========== Hook：批处理进度监听 ==========
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

  // ========== 菜单操作 ==========
  /** 关闭菜单并执行操作 */
  const closeAndRun = (action?: () => void) => {
    setOpenGroup(null);
    action?.();
  };

  /** 鼠标移入触发器时切换菜单（仅在已有菜单打开时生效） */
  const handleTriggerEnter = useCallback(
    (groupId: MenuGroupId) => {
      if (openGroup !== null && openGroup !== groupId) {
        setOpenGroup(groupId);
      }
    },
    [openGroup]
  );

  // ========== 菜单组定义（6 组）==========
  const menuGroups: MenuGroup[] = useMemo(
    () => [
      {
        id: "file",
        label: t("menu.file"),
        items: [
          {
            label: t("common.loadEsp"),
            onClick: () => void handleLoadEsp(),
            shortcut: "Ctrl+O",
          },
          {
            label: t("common.loadSst"),
            onClick: () => void handleLoadSst(),
            shortcut: "Ctrl+L",
          },
          {
            label: t("common.saveSst"),
            onClick: () => void handleSaveSst(),
            shortcut: "Ctrl+S",
          },
          {
            label: t("common.saveStrings"),
            onClick: () => void handleSaveStrings(),
          },
          { separator: true, label: "" },
          {
            label: t("menu.mergeSst", { defaultValue: "Merge SST..." }),
            onClick: () => setShowMergeSst(true),
          },
          { separator: true, label: "" },
          {
            label: t("common.exportXml"),
            onClick: () => void handleExportXml(),
          },
          {
            label: t("common.importXml"),
            onClick: () => void handleImportXml(),
          },
          { separator: true, label: "" },
          { label: t("app.resetWorkspace"), onClick: () => reset() },
        ],
      },
      {
        id: "edit",
        label: t("menu.edit", { defaultValue: "Edit" }),
        items: [
          {
            label: t("common.undo", { defaultValue: "Undo" }),
            onClick: () => {
              /* 由 StringTable 处理快捷键，此处仅为菜单入口 */
            },
            shortcut: "Ctrl+Z",
            disabled: true,
          },
          {
            label: t("common.redo", { defaultValue: "Redo" }),
            onClick: () => {
              /* 由 StringTable 处理快捷键 */
            },
            shortcut: "Ctrl+Y",
            disabled: true,
          },
          { separator: true, label: "" },
          {
            label: t("common.replaceAll", { defaultValue: "Replace All" }),
            onClick: () => setShowToolbox(true),
          },
        ],
      },
      {
        id: "search",
        label: t("menu.search", { defaultValue: "Search" }),
        items: [
          {
            label: t("advSearch.title", { defaultValue: "Advanced Search" }),
            onClick: () => setAdvSearchOpen(true),
          },
          { separator: true, label: "" },
          {
            label: t("common.translated"),
            onClick: () =>
              setStatusFilter(statusFilter === "translated" ? null : "translated"),
          },
          {
            label: t("common.incomplete"),
            onClick: () =>
              setStatusFilter(
                statusFilter === "incomplete" ? null : "incomplete"
              ),
          },
          {
            label: t("common.locked"),
            onClick: () =>
              setStatusFilter(statusFilter === "locked" ? null : "locked"),
          },
          { separator: true, label: "" },
          {
            label: vmadFilter ? t("vmad.showAll") : t("vmad.showOnly"),
            onClick: () => setVmadFilter(!vmadFilter),
          },
        ],
      },
      {
        id: "translate",
        label: t("menu.translate"),
        items: [
          {
            label: t("menu.openEditor"),
            onClick: openSelectedEditor,
            shortcut: "Enter",
          },
          {
            label: t("finalize.title"),
            onClick: () => setActivePanel("finalize"),
          },
          { separator: true, label: "" },
          {
            label: t("menu.tcsc_simplified"),
            onClick: () =>
              void convertSelectedTranslation("to_simplified"),
            shortcut: "简",
          },
          {
            label: t("menu.tcsc_traditional"),
            onClick: () =>
              void convertSelectedTranslation("to_traditional"),
            shortcut: "繁",
          },
          { separator: true, label: "" },
          {
            label: t("menu.compare_diff"),
            onClick: () => void compareSourceDestFromMenu("diff"),
          },
          {
            label: t("menu.compare_same"),
            onClick: () => void compareSourceDestFromMenu("same"),
          },
        ],
      },
      {
        id: "tools",
        label: t("menu.tools"),
        items: [
          {
            label: t("batch.title"),
            onClick: () => setActivePanel("batch"),
          },
          { label: t("bsa.title"), onClick: () => setActiveRightPanel("bsa") },
          { label: t("pex.title"), onClick: () => setActiveRightPanel("pex") },
          { label: t("fuz.title"), onClick: () => setActiveRightPanel("fuz") },
          {
            label: t("dialog.title"),
            onClick: () => setActivePanel("dialog"),
          },
          { label: t("mcm.title"), onClick: () => setActivePanel("mcm") },
          {
            label: t("espCompare.title"),
            onClick: () => setActiveRightPanel("espCompare"),
          },
          {
            label: t("dataConfigs.title"),
            onClick: () => setActivePanel("dataConfigs"),
          },
        ],
      },
      {
        id: "view",
        label: t("menu.view", { defaultValue: "View" }),
        items: [
          {
            label: t("menu.toggleBottomPanel"),
            onClick: () => toggleBottomPanel(),
          },
          { separator: true, label: "" },
          {
            label: t("menu.editorModeModal", {
              defaultValue: "Editor Mode: Modal",
            }),
            onClick: () => setEditorMode("modal"),
            shortcut: "Ctrl+1",
          },
          {
            label: t("menu.editorModeSidebar", {
              defaultValue: "Editor Mode: Sidebar",
            }),
            onClick: () => setEditorMode("sidebar"),
            shortcut: "Ctrl+2",
          },
          {
            label: t("menu.editorModeInline", {
              defaultValue: "Editor Mode: Inline",
            }),
            onClick: () => setEditorMode("inline"),
            shortcut: "Ctrl+3",
          },
        ],
      },
    ],
    [
      t,
      handleLoadEsp,
      handleLoadSst,
      handleImportXml,
      handleExportXml,
      handleSaveStrings,
      reset,
      setStatusFilter,
      setVmadFilter,
      vmadFilter,
      statusFilter,
      openSelectedEditor,
      setActivePanel,
      setActiveRightPanel,
      toggleBottomPanel,
      setEditorMode,
    ]
  );

  // ========== 渲染单个菜单组 ==========
  const renderMenuGroup = (group: MenuGroup) => {
    const isOpen = openGroup === group.id;

    return (
      <div
        className={`grouped-menu-group ${isOpen ? "open" : ""}`}
        key={group.id}
      >
        <button
          type="button"
          className="grouped-menu-trigger"
          onClick={() => setOpenGroup(isOpen ? null : group.id)}
          onMouseEnter={() => handleTriggerEnter(group.id)}
          aria-haspopup="menu"
          aria-expanded={isOpen}
        >
          {group.label}
        </button>
        {isOpen && group.items.length > 0 && (
          <div className="grouped-menu-panel" role="menu">
            {group.items.map((item, index) =>
              item.separator ? (
                <div key={`sep-${index}`} className="grouped-menu-separator" />
              ) : (
                <button
                  key={item.label}
                  type="button"
                  className="grouped-menu-item"
                  onClick={() => closeAndRun(item.onClick)}
                  disabled={item.disabled}
                  role="menuitem"
                >
                  <span className="grouped-menu-item-label">
                    {item.label}
                  </span>
                  {item.shortcut && (
                    <span className="grouped-menu-item-shortcut">
                      {item.shortcut}
                    </span>
                  )}
                </button>
              )
            )}
          </div>
        )}
      </div>
    );
  };

  // ========== 主渲染 ==========
  return (
    <div className="grouped-menubar" ref={menuBarRef}>
      {/* 第一行：菜单组 */}
      <div
        className="grouped-menubar-menu-strip"
        role="menubar"
        aria-label="Application menus"
      >
        <div className="grouped-menubar-brand">xTranslator(x64)</div>
        {menuGroups.map(renderMenuGroup)}
      </div>

      {/* 第二行：工具栏 */}
      <div
        className="grouped-menubar-toolbar"
        role="toolbar"
        aria-label="Application actions"
      >
        {/* 搜索框 */}
        <div className="toolbar-group" role="group" aria-label="Search">
          <Input
            size="sm"
            icon={<Search size={14} />}
            placeholder={
              useRegex ? t("common.regexFilter") : t("common.filter")
            }
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            wrapperClassName="menubar-search-input"
            suffix={
              <Button
                variant="ghost"
                size="xs"
                onClick={() => setUseRegex(!useRegex)}
                title={
                  useRegex
                    ? t("table.regexSwitchTip")
                    : t("table.plainSwitchTip")
                }
                active={useRegex}
              >
                <Code2 size={12} />
              </Button>
            }
          />
        </div>

        {/* 状态过滤按钮 */}
        <div
          className="toolbar-group"
          role="group"
          aria-label="Status filters"
        >
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
              onClick={() =>
                setStatusFilter(statusFilter === s.key ? null : s.key)
              }
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

        {/* 文件操作按钮 */}
        <div
          className="toolbar-group toolbar-group-primary"
          role="group"
          aria-label="Files"
        >
          <Button
            variant="primary"
            size="sm"
            icon={<FolderOpen size={14} />}
            onClick={handleLoadEsp}
            disabled={isParsing}
          >
            {t("common.loadEsp")}
          </Button>
          <Button
            size="sm"
            icon={<FileUp size={14} />}
            onClick={handleLoadSst}
            disabled={isLoading || !espPath}
          >
            {t("common.loadSst")}
          </Button>
          <Button
            size="sm"
            icon={<FileDown size={14} />}
            onClick={handleSaveSst}
            disabled={isLoading || !espPath}
          >
            {t("common.saveSst")}
          </Button>
          <Button
            size="sm"
            icon={<Save size={14} />}
            onClick={handleSaveStrings}
            disabled={isLoading || !espPath}
          >
            {t("common.saveStrings")}
          </Button>
        </div>

        {/* XML 按钮 */}
        <div
          className="toolbar-group"
          role="group"
          aria-label="Exchange formats"
        >
          <Button
            size="sm"
            icon={<FileCode size={14} />}
            onClick={handleExportXml}
            disabled={isLoading || !espPath}
          >
            {t("common.exportXml")}
          </Button>
          <Button
            size="sm"
            icon={<FileCode size={14} />}
            onClick={handleImportXml}
            disabled={isLoading || !espPath}
          >
            {t("common.importXml")}
          </Button>
        </div>

        {/* 完成按钮 */}
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
                  const baseName =
                    espPath
                      .replace(/\\/g, "/")
                      .split("/")
                      .pop()
                      ?.replace(/\.[^.]+$/, "") ?? "";
                  const stringsDir = espPath
                    .replace(/\\/g, "/")
                    .split("/")
                    .slice(0, -1)
                    .join("/");
                  const result = await delocalizeEsp({
                    esp_path: espPath,
                    strings_dir: stringsDir,
                    base_name: baseName,
                    language: language,
                    create_backup: true,
                  });
                  toast.success(
                    t("menu.delocalizedEsp", {
                      count: result.new_string_count,
                    })
                  );
                } catch (e: any) {
                  toast.error(
                    t("menu.delocalizeFailed", { error: String(e) })
                  );
                }
              }}
              disabled={isLoading || !espPath}
              title={t("menu.delocalizeEsp")}
            >
              {t("menu.delocalizeEsp")}
            </Button>
          )}
        </div>

        {/* 简繁转换按钮 */}
        <div
          className="toolbar-group"
          role="group"
          aria-label="TCSC conversion"
        >
          <Button
            size="xs"
            onClick={async () => {
              const selectedItem = useAppStore.getState().selectedItem;
              if (!selectedItem?.translation) {
                toast.error(t("menu.noTranslationToConvert"));
                return;
              }
              try {
                const result = await tcscConvert(
                  selectedItem.translation,
                  "to_simplified"
                );
                await updateTranslation(selectedItem.id, result);
                useAppStore
                  .getState()
                  .updateItemTranslation(selectedItem.id, result);
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
                const result = await tcscConvert(
                  selectedItem.translation,
                  "to_traditional"
                );
                await updateTranslation(selectedItem.id, result);
                useAppStore
                  .getState()
                  .updateItemTranslation(selectedItem.id, result);
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
              const hasTranslations = allItems.some(
                (item) => item.translation && item.translation.trim() !== ""
              );
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
                toast.success(
                  t("menu.tcsc_batch_done", { count: updatedIds.length })
                );
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
              const hasTranslations = allItems.some(
                (item) => item.translation && item.translation.trim() !== ""
              );
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
                toast.success(
                  t("menu.tcsc_batch_done", { count: updatedIds.length })
                );
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

        {/* 工具面板图标按钮 */}
        <div
          className="toolbar-group toolbar-icon-group"
          role="group"
          aria-label="Tool panels"
        >
          {(
            [
              {
                id: "batch" as const,
                icon: <RefreshCw size={14} />,
                openLabel: t("menu.openBatchPanel"),
                closeLabel: t("menu.closeBatchPanel"),
                right: false,
              },
              {
                id: "bsa" as const,
                icon: <FileArchive size={14} />,
                openLabel: t("menu.openBsaBrowser"),
                closeLabel: t("menu.closeBsaBrowser"),
                right: true,
              },
              {
                id: "pex" as const,
                icon: <Braces size={14} />,
                openLabel: t("menu.openPexPanel"),
                closeLabel: t("menu.closePexPanel"),
                right: true,
              },
              {
                id: "fuz" as const,
                icon: <Volume2 size={14} />,
                openLabel: t("menu.openVoicePanel"),
                closeLabel: t("menu.closeVoicePanel"),
                right: true,
              },
              {
                id: "dialog" as const,
                icon: <MessagesSquare size={14} />,
                openLabel: t("menu.openDialogView"),
                closeLabel: t("menu.closeDialogView"),
                right: false,
              },
              {
                id: "mcm" as const,
                icon: <FileText size={14} />,
                openLabel: t("menu.openMcmPanel"),
                closeLabel: t("menu.closeMcmPanel"),
                right: false,
              },
              {
                id: "espCompare" as const,
                icon: <GitCompare size={14} />,
                openLabel: t("menu.openEspCompare"),
                closeLabel: t("menu.closeEspCompare"),
                right: true,
              },
              {
                id: "dataConfigs" as const,
                icon: <Database size={14} />,
                openLabel: t("menu.openDataConfigs"),
                closeLabel: t("menu.closeDataConfigs"),
                right: false,
              },
            ] as const
          ).map(({ id, icon, openLabel, closeLabel, right }) => {
            const isActive = right ? activeRightPanel === id : activePanel === id;
            return (
              <Button
                key={id}
                variant="ghost"
                size="sm"
                icon={icon}
                onClick={() => right ? setActiveRightPanel(id) : setActivePanel(id)}
                active={isActive}
                title={isActive ? closeLabel : openLabel}
                aria-label={isActive ? closeLabel : openLabel}
                aria-pressed={isActive}
              />
            );
          })}
        </div>

        {/* 选择下拉框 */}
        <div
          className="toolbar-group toolbar-selects"
          role="group"
          aria-label="Preferences"
        >
          <select
            value={gameSelectionMode === "auto" ? "auto" : currentGame ?? "auto"}
            onChange={(e) => void handleGameSelectionChange(e.target.value)}
            className="lang-select"
            title={t("menu.gameWorkspace", { defaultValue: "Game workspace" })}
            aria-label={t("menu.gameWorkspace", { defaultValue: "Game workspace" })}
          >
            <option value="auto">{t("menu.gameAuto", { defaultValue: "Game: Auto" })}</option>
            {SUPPORTED_GAME_IDS.map((game) => (
              <option key={game} value={game}>{game}</option>
            ))}
          </select>
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
              <option key={code} value={code}>
                {label}
              </option>
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
            title={t("spellcheck.title", {
              defaultValue: "Spell Check",
            })}
            aria-label={t("spellcheck.title", {
              defaultValue: "Spell Check",
            })}
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

        {/* 状态指示器 */}
        <div className="menubar-status-group">
          {isParsing && (
            <span className="menubar-status parsing">{t("app.parsing")}</span>
          )}
          {isLoading && (
            <span className="menubar-status loading">{t("app.loading")}</span>
          )}
          {isDirty && (
            <span
              className="menubar-status dirty"
              title={t("app.unsavedChanges")}
            >
              ●
            </span>
          )}
          {espMode && (
            <span
              className="menubar-status esp-mode"
              title={t("sidebar.espMode")}
            >
              {t("sidebar.espMode")}
            </span>
          )}
          {batchProgress && batchProgress.total_files > 0 && (
            <span
              className="menubar-status batch-progress"
              title={`${batchProgress.message}`}
            >
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

      {/* 对话框 */}
      <SettingsDialog
        open={showSettings}
        onClose={() => setShowSettings(false)}
      />
      <MergeSstDialog
        open={showMergeSst}
        onClose={() => setShowMergeSst(false)}
        onMergeComplete={loadAllStrings}
        espPath={espPath}
      />
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
      <ApplySstDialog
        open={applySstDialogOpen}
        onClose={() => {
          setApplySstDialogOpen(false);
          setPendingSstPath(null);
        }}
        sstPath={pendingSstPath ?? ""}
        selectedCount={selectedIds.size}
        filteredCount={items.length}
        onConfirm={(options: SstApplyOptions) => {
          if (pendingSstPath) {
            const finalOptions: SstApplyOptions = {
              ...options,
              selected_ids:
                options.overwrite_scope === "selection"
                  ? Array.from(selectedIds)
                  : undefined,
              filtered_ids: options.restrict_to_filter
                ? items.map((i) => i.id)
                : undefined,
            };
            loadSstFromPath(pendingSstPath, finalOptions);
          }
          setApplySstDialogOpen(false);
          setPendingSstPath(null);
        }}
      />
      <AdvSearchDialog
        open={advSearchOpen}
        onClose={() => setAdvSearchOpen(false)}
      />
    </div>
  );
}
