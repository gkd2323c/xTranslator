import { create } from "zustand";
import type { SkyStringDTO, LoadEspResponse, LoadSstResponse, BatchEntry, BatchStatus, DataConfigsDto, RecoveryInfo } from "../api/strings";
import { getAllStrings, getStringsChunk, getStringsCount, queryStrings, updateTranslation, batchUpdateTranslations, startStringBatchTranslate, cancelStringBatchTranslate, checkPendingCache, applyTranslationCache, discardTranslationCache } from "../api/strings";
import { saveConfig } from "../api/strings";
import toast from "react-hot-toast";
import i18n from "../i18n";

// 主题类型
export type Theme = "obsidian" | "dark" | "light" | "slate" | "auto";

// 工具面板类型（单选，互斥）
// 用于管理 9 个工具对话框的显示状态
export type ActivePanel =
  | "batch"      // 批处理面板
  | "bsa"        // BSA 浏览器
  | "pex"        // PEX 脚本编辑器
  | "fuz"        // FUZ 音频扫描
  | "dialog"     // 对话树
  | "mcm"        // MCM 配置
  | "espCompare" // ESP 对比
  | "finalize"   // 最终化工作流
  | "dataConfigs"// 数据配置
  | null;        // 无面板打开

// 底部标签页类型
export type BottomTabId =
  | "home"         // 主页（统计信息）
  | "vocabulary"   // 词汇库
  | "heuristic"    // 启发式搜索
  | "espTree"      // ESP 记录树
  | "pex"          // PEX 脚本
  | "quests"       // 任务
  | "dialogs"      // 对话
  | "log"          // 日志
  | "headerProc"   // 头部处理器
  | "headerWizard";// 头部向导


// 日志级别
export type LogLevel = "info" | "warn" | "error";

// 日志条目
export interface LogEntry {
  id: number;
  timestamp: Date;
  level: LogLevel;
  message: string;
  source?: string;
}

const THEME_STORAGE_KEY = "xtranslator-theme";

// 检测系统是否偏好深色主题
function getSystemPrefersDark(): boolean {
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

// 将主题设置解析为实际的 CSS 类名
// - "auto" → 根据系统偏好返回 "obsidian" 或 "light"
// - "dark" → 映射为 "obsidian"（Delphi 兼容）
function resolveTheme(theme: Theme): string {
  if (theme === "auto") {
    return getSystemPrefersDark() ? "obsidian" : "light";
  }
  if (theme === "dark") return "obsidian";
  return theme;
}

// 加载进度信息
interface LoadProgress {
  // 加载阶段："reading_defs", "loading_strings", "parsing", "finalizing"
  stage: string;
  // 当前进度值
  current: number;
  // 总进度值
  total: number;
  // 百分比 (0-100)
  percentage: number;
  // 用户可读的消息
  message: string;
}

// 撤销栈条目
interface UndoEntry {
  // 字符串 ID
  id: number;
  // 修改前的翻译
  oldTranslation: string;
  // 修改前的状态
  oldStatus: string;
}

const MAX_UNDO_STACK = 100;

// 应用全局状态
///
// 设计原则：
// - `allItems` 是完整数据集（从后端加载）
// - `items` 是过滤/排序后的显示集（用于虚拟滚动）
// - 侧边栏统计基于 `allItems`，不受过滤影响
// - 选择操作使用 `selectedId`（稳定 ID），而非数组索引
interface AppState {
  // ── 数据集 ──
  // 完整的字符串列表（从后端加载，不受过滤影响）
  allItems: SkyStringDTO[];
  // 过滤+排序后的显示集（用于虚拟滚动）
  items: SkyStringDTO[];
  // 总记录数（未过滤）
  total: number;
  // 过滤后的记录数
  filtered: number;

  // ── 加载状态 ──
  // 是否正在加载数据
  isLoading: boolean;
  // 是否正在解析 ESP 文件
  isParsing: boolean;
  // 错误消息（为 null 表示无错误）
  error: string | null;
  // 加载进度信息（用于显示进度条）
  loadProgress: LoadProgress | null;

  // ── 文件信息 ──
  // 当前打开的 ESP 文件路径
  espPath: string | null;
  // 当前打开的 SST 字典路径
  sstPath: string | null;
  // Strings 文件所在目录
  stringsDir: string | null;
  // 源语言（通常 "english"）
  language: string;
  // 目标语言（如 "chinese"）
  targetLang: string;

  // ── 加载统计 ──
  // ESP 加载响应（包含解析统计）
  espStats: LoadEspResponse | null;
  // SST 加载响应（包含匹配统计）
  sstStats: LoadSstResponse | null;

  // ── 数据配置 ──
  // 游戏数据配置（CTDA 函数、字段大小等）
  dataConfigs: DataConfigsDto | null;

  // ── ESP 模式 ──
  // 是否启用 ESP 直接回写模式（vs 外部 .STRINGS 文件）
  espMode: boolean;

  // ── 过滤和排序 ──
  // 搜索过滤词
  filter: string;
  // 是否使用正则表达式过滤
  useRegex: boolean;
  // 替换文本（用于批量替换）
  replaceText: string;
  // 状态过滤："translated" / "incomplete" / "locked" / null
  statusFilter: string | null;
  // 记录类型过滤（如 "DIAL", "INFO"）
  recordFilter: string | null;
  // 是否仅显示 VMAD 脚本字符串
  vmadFilter: boolean;
  // Strings 文件类型过滤：null=全部, 0=.STRINGS, 1=.DLSTRINGS, 2=.ILSTRINGS
  listIndex: number | null;
  // 排序字段（如 "source", "translation", "form_id"）
  sortField: string;
  // 排序方向
  sortDir: "asc" | "desc";

  // ── 选择 ──
  // 当前选中的字符串 ID（稳定 ID，不是数组索引）
  selectedId: number | null;
  // 当前选中的字符串对象（缓存，避免重复查找）
  selectedItem: SkyStringDTO | null;

  // ── 主题 ──
  // 主题设置
  theme: Theme;
  // 主题标签（用于 CSS 类名）
  themeLabel: string;

  // ── 脏标志 ──
  // 是否有未保存的翻译修改
  isDirty: boolean;

  // ── 缓存 ──
  // ESP 文件的 SHA-256 哈希（用于翻译缓存关联）
  espHash: string | null;

  // ── 恢复提示 ──
  // 是否显示恢复模态框
  showRecoveryModal: boolean;
  // 恢复信息（待应用的缓存翻译）
  recoveryInfo: RecoveryInfo | null;

  // ── 日志系统 ──
  // 应用日志消息列表（最多 500 条）
  logs: LogEntry[];


  // ── 撤销/重做 ──
  // 撤销栈（最多 100 条）
  undoStack: UndoEntry[];
  // 重做栈
  redoStack: UndoEntry[];

  // ── 面板系统 ──
  // 当前打开的工具面板（单选，互斥）
  activePanel: ActivePanel;
  // 当前活跃的底部标签页
  activeBottomTab: BottomTabId;
  // 是否显示底部面板
  showBottomPanel: boolean;
  // 编辑对话框是否打开
  editorOpen: boolean;

  // ── 批处理 ──
  // 批处理文件列表
  batchEntries: BatchEntry[];
  // 批处理状态
  batchStatus: BatchStatus | null;

  // ── 字符串级批量翻译 ──
  // 选中的字符串 ID 集合
  selectedIds: Set<number>;
  // 批量翻译状态："idle" | "running" | "cancelling" | "completed" | "cancelled"
  batchState: "idle" | "running" | "cancelling" | "completed" | "cancelled";
  // 批量翻译进度
  batchProgress: { completed: number; total: number };
  // 批量翻译错误列表
  batchErrors: { strId: number; error: string }[];
  // 批量翻译并发数
  batchConcurrency: number;

  // ── 操作方法 ──
  // 设置完整数据集
  setAllItems: (items: SkyStringDTO[]) => void;
  // 设置加载状态
  setLoading: (loading: boolean) => void;
  // 设置解析状态
  setParsing: (parsing: boolean) => void;
  // 设置错误消息
  setError: (error: string | null) => void;
  setLoadProgress: (progress: LoadProgress | null) => void;
  setEspLoaded: (path: string, stats: LoadEspResponse, stringsDir?: string) => void;
  setSstLoaded: (path: string, stats: LoadSstResponse) => void;
  setTargetLang: (lang: string) => void;
  setFilter: (filter: string) => void;
  setFilterNow: (filter: string) => void;
  setUseRegex: (use: boolean) => void;
  setReplaceText: (text: string) => void;
  setStatusFilter: (status: string | null) => void;
  setRecordFilter: (record: string | null) => void;
  setVmadFilter: (enabled: boolean) => void;
  setListIndex: (index: number | null) => void;
  setSort: (field: string, dir?: "asc" | "desc") => void;
  replaceAll: () => Promise<void>;
  undo: () => Promise<void>;
  redo: () => Promise<void>;
  setSelectedById: (id: number | null) => void;
  updateItemTranslation: (id: number, translation: string) => void;
  applyIncrementalUpdate: (updatedIds: number[]) => void;
  setIsDirty: (dirty: boolean) => void;
  setTheme: (theme: Theme) => void;
  cycleTheme: () => void;
  reapplyTheme: () => void;
  selectNextRow: () => void;
  toggleSelectId: (id: number) => void;
  clearSelection: () => void;
  startBatchTranslation: () => Promise<void>;
  cancelBatchTranslation: () => Promise<void>;
  setBatchConcurrency: (n: number) => void;
  checkAndPromptRecovery: (espHash: string) => Promise<void>;
  applyRecovery: () => Promise<void>;
  discardRecovery: () => Promise<void>;
  closeRecoveryModal: () => void;
  selectPrevRow: () => void;
  loadAllStrings: () => Promise<void>;
  setActivePanel: (panel: ActivePanel) => void;
  setActiveBottomTab: (tab: BottomTabId) => void;
  // ── 日志 ──
  addLog: (level: LogLevel, message: string, source?: string) => void;
  clearLogs: () => void;

  toggleBottomPanel: () => void;
  setEditorOpen: (open: boolean) => void;
  openEditorForItem: (id: number) => void;
  setDataConfigs: (configs: DataConfigsDto | null) => void;
  setEspMode: (espMode: boolean) => void;
  setBatchEntries: (entries: BatchEntry[]) => void;
  addBatchEntries: (entries: BatchEntry[]) => void;
  removeBatchEntry: (index: number) => void;
  clearBatchEntries: () => void;
  setBatchStatus: (status: BatchStatus | null) => void;
  reset: () => void;
}

function getInitialTheme(): Theme {
  try {
    const stored = localStorage.getItem(THEME_STORAGE_KEY);
    if (stored === "dark") return "obsidian"; // migrate legacy
    if (stored === "obsidian" || stored === "light" || stored === "slate" || stored === "auto") return stored;
    if (stored === "gray") return "slate"; // legacy migration
  } catch { /* localStorage unavailable */ }
  return "obsidian";
}

const THEME_LABELS: Record<Theme, string> = { obsidian: "Obsidian", dark: "Obsidian", light: "Light", slate: "Slate", auto: "Auto" };
const THEME_NEXT: Record<Theme, Theme> = { obsidian: "slate", slate: "light", light: "auto", auto: "obsidian", dark: "obsidian" };

function applyFilterAndSort(
  allItems: SkyStringDTO[],
  filter: string,
  useRegex: boolean,
  statusFilter: string | null,
  recordFilter: string | null,
  vmadFilter: boolean,
  listIndex: number | null,
  sortField: string,
  sortDir: "asc" | "desc"
): SkyStringDTO[] {
  let result = allItems;

  // List index filter (STRINGS/DLSTRINGS/ILSTRINGS)
  if (listIndex !== null) {
    result = result.filter((item) => item.list_index === listIndex);
  }

  // Record type filter
  if (recordFilter) {
    result = result.filter((item) => item.record_sig === recordFilter);
  }

  // Status filter
  if (statusFilter) {
    result = result.filter((item) => item.status === statusFilter);
  }

  // VMAD filter
  if (vmadFilter) {
    result = result.filter((item) => item.is_vmad);
  }

  // Text filter
  if (filter) {
    if (useRegex) {
      try {
        const regex = new RegExp(filter, "i");
        result = result.filter(
          (item) =>
            regex.test(item.source) ||
            regex.test(item.translation) ||
            regex.test(item.record_sig)
        );
      } catch {
        // Invalid regex — treat as no match
        return [];
      }
    } else {
      const ft = filter.toLowerCase();
      result = result.filter(
        (item) =>
          item.source.toLowerCase().includes(ft) ||
          item.translation.toLowerCase().includes(ft) ||
          item.record_sig.toLowerCase().includes(ft)
      );
    }
  }

  // Sort
  const isAsc = sortDir === "asc";
  result = [...result].sort((a, b) => {
    let cmp = 0;
    switch (sortField) {
      case "id":
        cmp = a.id - b.id;
        break;
      case "source":
        cmp = a.source.localeCompare(b.source);
        break;
      case "record_sig":
        cmp = a.record_sig.localeCompare(b.record_sig);
        break;
      default:
        cmp = a.id - b.id;
    }
    return isAsc ? cmp : -cmp;
  });

  return result;
}

export function computeTranslationProgress(allItems: SkyStringDTO[]): { translated: number; total: number } {
  const total = allItems.length;
  const translated = allItems.filter((s) => s.translation && s.translation.trim() !== '').length;
  return { translated, total };
}

// Debounce timer for filter input (shared across store instances)
let filterDebounceTimer: ReturnType<typeof setTimeout> | null = null;
const FILTER_DEBOUNCE_MS = 150;

// In E2E test mode, the auto-init effect sets a synthetic espPath so loadAllStrings
// can proceed even without a real file on disk.
const E2E_SENTINEL_ESP = "http://localhost/e2e-test.esp";

export const useAppStore = create<AppState>((set, get) => ({
  // E2E mode only: inject mock data directly into the store.
  // Called by App.tsx on mount in E2E test mode.
  __e2eInjectMock: (mockItems: SkyStringDTO[]) => {
    const state = get();
    if (state.allItems.length > 0) return; // already injected
    console.log("[E2E __e2eInjectMock] called, items count:", mockItems.length);
    const items = applyFilterAndSort(
      mockItems,
      state.filter,
      state.useRegex,
      state.statusFilter,
      state.recordFilter,
      state.vmadFilter,
      state.listIndex,
      state.sortField,
      state.sortDir
    );
    set({
      allItems: mockItems,
      items,
      total: mockItems.length,
      filtered: mockItems.length,
      isLoading: false,
      isParsing: false,
      error: null,
      loadProgress: null,
      espPath: E2E_SENTINEL_ESP,
      sstPath: null,
      stringsDir: null,
      espStats: {
        total: mockItems.length,
        compressed_records: 0,
        strings_loaded: mockItems.length,
        parse_time_ms: 5,
        record_counts: { INFO: Math.floor(mockItems.length * 0.6), QUST: Math.floor(mockItems.length * 0.25), DIAL: Math.floor(mockItems.length * 0.15) },
        cached: false,
        esp_hash: "mock_hash_abc123",
      },
    });
  },

  allItems: [],
  items: [],
  total: 0,
  filtered: 0,
  isLoading: false,
  isParsing: false,
  error: null,
  loadProgress: null,
  espPath: null,
  sstPath: null,
  stringsDir: null,
  language: "english",
  targetLang: "chinese",
  espStats: null,
  sstStats: null,
  filter: "",
  useRegex: false,
  replaceText: "",
  statusFilter: null,
  recordFilter: null,
  vmadFilter: false,
  listIndex: null,
  sortField: "id",
  sortDir: "asc",
  selectedId: null,
  selectedItem: null,
  theme: getInitialTheme(),
  themeLabel: THEME_LABELS[getInitialTheme()],
  isDirty: false,
  espHash: null,
  showRecoveryModal: false,
  recoveryInfo: null,
  undoStack: [],
  redoStack: [],
  activePanel: null,
  logs: [],

  activeBottomTab: "home",
  showBottomPanel: true,
  editorOpen: false,
  batchEntries: [],
  batchStatus: null,
  selectedIds: new Set<number>(),
  batchState: "idle" as "idle" | "running" | "cancelling" | "completed" | "cancelled",
  batchProgress: { completed: 0, total: 0 },
  batchErrors: [],
  batchConcurrency: 3,
  dataConfigs: null,
  espMode: false,

  setAllItems: (allItems) => {
    const state = get();
    const items = applyFilterAndSort(
      allItems,
      state.filter,
      state.useRegex,
      state.statusFilter,
      state.recordFilter,
      state.vmadFilter,
      state.listIndex,
      state.sortField,
      state.sortDir
    );
    set({
      allItems,
      items,
      total: allItems.length,
      filtered: items.length,
    });
  },

  setLoading: (isLoading) => set({ isLoading }),
  setParsing: (isParsing) => set({ isParsing }),
  setError: (error) => set({ error }),
  setLoadProgress: (loadProgress) => set({ loadProgress }),

  setEspLoaded: (espPath, espStats, stringsDir) =>
    set({ espPath, espStats, stringsDir, sstStats: null, espHash: espStats.esp_hash || null }),

  setSstLoaded: (sstPath, sstStats) => set({ sstPath, sstStats }),

  setTargetLang: (targetLang) => set({ targetLang }),

  setFilter: (filter) => {
    // Debounced: update filter text immediately for responsive input, defer re-filter
    set({ filter });
    if (filterDebounceTimer) clearTimeout(filterDebounceTimer);
    filterDebounceTimer = setTimeout(() => {
      const state = get();
      const items = applyFilterAndSort(
        state.allItems,
        state.filter,
        state.useRegex,
        state.statusFilter,
        state.recordFilter,
        state.vmadFilter,
        state.listIndex,
        state.sortField,
        state.sortDir
      );
      set({ items, filtered: items.length });
    }, FILTER_DEBOUNCE_MS);
  },

  setFilterNow: (filter) => {
    const state = get();
    const items = applyFilterAndSort(
      state.allItems,
      filter,
      state.useRegex,
      state.statusFilter,
      state.recordFilter,
      state.vmadFilter,
      state.listIndex,
      state.sortField,
      state.sortDir
    );
    set({ filter, items, filtered: items.length });
  },

  setUseRegex: (useRegex) => {
    const state = get();
    const items = applyFilterAndSort(
      state.allItems,
      state.filter,
      useRegex,
      state.statusFilter,
      state.recordFilter,
      state.vmadFilter,
      state.listIndex,
      state.sortField,
      state.sortDir
    );
    set({ useRegex, items, filtered: items.length });
  },

  setReplaceText: (replaceText) => set({ replaceText }),

  replaceAll: async () => {
    const state = get();
    if (!state.filter || !state.replaceText) {
      toast.error(i18n.t("toast.bothSearchReplaceRequired"));
      return;
    }

    let regex: RegExp;
    try {
      regex = new RegExp(state.filter, state.useRegex ? "gi" : "gi");
    } catch {
      toast.error(i18n.t("toast.invalidRegex", { pattern: state.filter }));
      return;
    }

    const candidates = applyFilterAndSort(
      state.allItems,
      state.filter,
      state.useRegex,
      state.statusFilter,
      state.recordFilter,
      state.vmadFilter,
      state.listIndex,
      state.sortField,
      state.sortDir
    );

    if (candidates.length === 0) {
      toast(i18n.t("toast.noMatchingFound"));
      return;
    }

    const confirmed = window.confirm(i18n.t("toast.replaceConfirm", { from: state.filter, to: state.replaceText, count: candidates.length }));
    if (!confirmed) return;

    const toastId = toast.loading(i18n.t("toast.replacingCount", { count: candidates.length }));

    // Build batch updates: collect all replacements first
    const updates: [number, string][] = [];
    for (const item of candidates) {
      const target = item.translation || item.source;
      const replaced = target.replace(regex, state.replaceText);
      if (replaced !== target) {
        updates.push([item.id, replaced]);
      }
    }

    let changed = 0;
    if (updates.length > 0) {
      try {
        changed = await batchUpdateTranslations(updates);
      } catch (e: any) {
        console.error("Batch replace failed:", e);
        toast.error(`${i18n.t("toast.replaceFailed")}: ${e}`);
        toast.dismiss(toastId);
        return;
      }
    }

    // Update local state directly (no full reload needed)
    if (changed > 0) {
      const updatedMap = new Map(updates);
      const newAllItems = state.allItems.map((item) => {
        const newTrans = updatedMap.get(item.id);
        if (newTrans !== undefined) {
          return { ...item, translation: newTrans, status: newTrans ? "translated" : "incomplete" };
        }
        return item;
      });
      const items = applyFilterAndSort(
        newAllItems,
        state.filter,
        state.useRegex,
        state.statusFilter,
        state.recordFilter,
        state.vmadFilter,
        state.listIndex,
        state.sortField,
        state.sortDir
      );
      set({ allItems: newAllItems, items, filtered: items.length, isDirty: true });
    }

    toast.dismiss(toastId);
    if (changed > 0) {
      toast.success(i18n.t("toast.replaceResult", { count: changed.toLocaleString() }));
    } else {
      toast(i18n.t("toast.noStringsChanged"));
    }
  },

  undo: async () => {
    const state = get();
    if (state.undoStack.length === 0) {
      toast(i18n.t("toast.undoNothing"));
      return;
    }
    const entry = state.undoStack[0];
    const newUndo = state.undoStack.slice(1);

    // Record current state in redo
    const currentItem = state.allItems.find((i) => i.id === entry.id);
    const redoEntry: UndoEntry = {
      id: entry.id,
      oldTranslation: currentItem?.translation || "",
      oldStatus: currentItem?.status || "incomplete",
    };
    const newRedo = [redoEntry, ...state.redoStack];

    // Revert via IPC
    try {
      await updateTranslation(entry.id, entry.oldTranslation);
    } catch {
      toast.error(i18n.t("toast.undoFailed"));
      return;
    }

    // Apply locally without recording another undo
    const newAllItems = state.allItems.map((item) =>
      item.id === entry.id
        ? { ...item, translation: entry.oldTranslation, status: entry.oldStatus }
        : item
    );
    const items = applyFilterAndSort(
      newAllItems,
      state.filter,
      state.useRegex,
      state.statusFilter,
      state.recordFilter,
      state.vmadFilter,
      state.listIndex,
      state.sortField,
      state.sortDir
    );
    const selectedItem = state.selectedId === entry.id
      ? { ...state.selectedItem!, translation: entry.oldTranslation, status: entry.oldStatus }
      : state.selectedItem;
    set({ allItems: newAllItems, items, filtered: items.length, selectedItem, undoStack: newUndo, redoStack: newRedo });
  },

  redo: async () => {
    const state = get();
    if (state.redoStack.length === 0) {
      toast(i18n.t("toast.redoNothing"));
      return;
    }
    const entry = state.redoStack[0];
    const newRedo = state.redoStack.slice(1);

    // Record current state in undo
    const currentItem = state.allItems.find((i) => i.id === entry.id);
    const undoEntry: UndoEntry = {
      id: entry.id,
      oldTranslation: currentItem?.translation || "",
      oldStatus: currentItem?.status || "incomplete",
    };
    const newUndo = [undoEntry, ...state.undoStack].slice(0, MAX_UNDO_STACK);

    // Revert via IPC
    try {
      await updateTranslation(entry.id, entry.oldTranslation);
    } catch {
      toast.error(i18n.t("toast.redoFailed"));
      return;
    }

    // Apply locally without recording another undo
    const newAllItems = state.allItems.map((item) =>
      item.id === entry.id
        ? { ...item, translation: entry.oldTranslation, status: entry.oldStatus }
        : item
    );
    const items = applyFilterAndSort(
      newAllItems,
      state.filter,
      state.useRegex,
      state.statusFilter,
      state.recordFilter,
      state.vmadFilter,
      state.listIndex,
      state.sortField,
      state.sortDir
    );
    const selectedItem = state.selectedId === entry.id
      ? { ...state.selectedItem!, translation: entry.oldTranslation, status: entry.oldStatus }
      : state.selectedItem;
    set({ allItems: newAllItems, items, filtered: items.length, selectedItem, undoStack: newUndo, redoStack: newRedo });
  },

  setStatusFilter: (statusFilter) => {
    const state = get();
    const items = applyFilterAndSort(
      state.allItems,
      state.filter,
      state.useRegex,
      statusFilter,
      state.recordFilter,
      state.vmadFilter,
      state.listIndex,
      state.sortField,
      state.sortDir
    );
    set({ statusFilter, items, filtered: items.length });
  },

  setRecordFilter: (recordFilter) => {
    const state = get();
    const items = applyFilterAndSort(
      state.allItems,
      state.filter,
      state.useRegex,
      state.statusFilter,
      recordFilter,
      state.vmadFilter,
      state.listIndex,
      state.sortField,
      state.sortDir
    );
    set({ recordFilter, items, filtered: items.length });
  },

  setVmadFilter: (vmadFilter) => {
    const state = get();
    const items = applyFilterAndSort(
      state.allItems,
      state.filter,
      state.useRegex,
      state.statusFilter,
      state.recordFilter,
      vmadFilter,
      state.listIndex,
      state.sortField,
      state.sortDir
    );
    set({ vmadFilter, items, filtered: items.length });
  },

  setListIndex: (listIndex) => {
    const state = get();
    const items = applyFilterAndSort(
      state.allItems,
      state.filter,
      state.useRegex,
      state.statusFilter,
      state.recordFilter,
      state.vmadFilter,
      listIndex,
      state.sortField,
      state.sortDir
    );
    set({ listIndex, items, filtered: items.length });
  },

  setSort: (field, dir) => {
    const state = get();
    const sortDir = dir || (state.sortField === field && state.sortDir === "asc" ? "desc" : "asc");
    const sortField = field;
    const items = applyFilterAndSort(
      state.allItems,
      state.filter,
      state.useRegex,
      state.statusFilter,
      state.recordFilter,
      state.vmadFilter,
      state.listIndex,
      sortField,
      sortDir
    );
    set({ sortField, sortDir, items, filtered: items.length });
  },

  setEditorOpen: (open) => set({ editorOpen: open }),
  openEditorForItem: (id) => {
    const state = get();
    const item = state.allItems.find((i) => i.id === id) || null;
    set({ selectedId: id, selectedItem: item, editorOpen: true });
  },
  setSelectedById: (id) => {
    if (id === null) {
      set({ selectedId: null, selectedItem: null });
      return;
    }
    const state = get();
    const item = state.allItems.find((i) => i.id === id) || null;
    set({ selectedId: id, selectedItem: item });
  },

  updateItemTranslation: (id, translation) => {
    const state = get();

    // Record undo entry before mutation
    const oldItem = state.allItems.find((i) => i.id === id);
    if (oldItem && oldItem.translation !== translation) {
      const entry: UndoEntry = {
        id,
        oldTranslation: oldItem.translation,
        oldStatus: oldItem.status,
      };
      const newUndo = [entry, ...state.undoStack].slice(0, MAX_UNDO_STACK);
      set({ undoStack: newUndo, redoStack: [] });
    }

    // Apply translation mutation
    const newAllItems = state.allItems.map((item) =>
      item.id === id
        ? { ...item, translation, status: translation ? "translated" : "incomplete" }
        : item
    );
    const items = applyFilterAndSort(
      newAllItems,
      state.filter,
      state.useRegex,
      state.statusFilter,
      state.recordFilter,
      state.vmadFilter,
      state.listIndex,
      state.sortField,
      state.sortDir
    );
    const selectedItem = state.selectedId === id
      ? { ...state.selectedItem!, translation, status: translation ? "translated" : "incomplete" }
      : state.selectedItem;
    set({ allItems: newAllItems, items, filtered: items.length, selectedItem, isDirty: true });
  },

  applyIncrementalUpdate: (_updatedIds) => {
    // After XML import, fetch fresh data for the updated IDs from allItems
    // Since the backend already mutated the strings, we need to reload all
    // (incremental would require a get_strings_by_ids command)
    // For now, full reload is simpler and reliable
    get().loadAllStrings();
    set({ isDirty: true });
  },

  setIsDirty: (isDirty) => set({ isDirty }),

  setActivePanel: (panel) => {
    const current = get().activePanel;
    // Toggle off if clicking the same panel
    set({ activePanel: current === panel ? null : panel });
  },
  setActiveBottomTab: (tab) => set({ activeBottomTab: tab, showBottomPanel: true }),
  toggleBottomPanel: () => set((s) => ({ showBottomPanel: !s.showBottomPanel })),
  setDataConfigs: (dataConfigs) => set({ dataConfigs }),

  // ── 日志 ──
  addLog: (level, message, source) =>
    set((s) => {
      const entry: LogEntry = {
        id: Date.now() + (s.logs.length > 0 ? s.logs[0].id + 1 - Date.now() : 0),
        timestamp: new Date(),
        level,
        message,
        source,
      };
      const logs = [entry, ...s.logs].slice(0, 500);
      return { logs };
    }),
  clearLogs: () => set({ logs: [] }),


  setEspMode: (espMode) => {
    set({ espMode });
    saveConfig({ esp_mode: espMode }).catch(() => {});
  },

  setBatchEntries: (batchEntries) => set({ batchEntries }),

  addBatchEntries: (entries) => {
    const state = get();
    const existingPaths = new Set(state.batchEntries.map((e) => e.esp_path.toLowerCase().replace(/\\/g, "/")));
    const newEntries = entries.filter(
      (e) => !existingPaths.has(e.esp_path.toLowerCase().replace(/\\/g, "/"))
    );

    // Conflict check: warn if batch entry matches currently loaded ESP
    if (state.espPath) {
      const loadedEspNorm = state.espPath.replace(/\\/g, "/").toLowerCase();
      const hasConflict = newEntries.some(
        (e) => e.esp_path.replace(/\\/g, "/").toLowerCase() === loadedEspNorm
      );
      if (hasConflict) {
        setTimeout(
          () =>
            toast(i18n.t("toast.batchConflict"), { icon: "!", duration: 4000 }),
          100
        );
      }
    }

    set({ batchEntries: [...state.batchEntries, ...newEntries] });
  },

  removeBatchEntry: (index) => {
    const entries = get().batchEntries.filter((_, i) => i !== index);
    set({ batchEntries: entries });
  },

  clearBatchEntries: () => set({ batchEntries: [], batchStatus: null }),

  setBatchStatus: (batchStatus) => set({ batchStatus }),

  setTheme: (theme) => {
    try { localStorage.setItem(THEME_STORAGE_KEY, theme); } catch { /* ok */ }
    document.documentElement.setAttribute("data-theme", resolveTheme(theme));
    set({ theme, themeLabel: THEME_LABELS[theme] });
    saveConfig({ theme }).catch(() => {});
  },

  cycleTheme: () => {
    const current = get().theme;
    const next = THEME_NEXT[current];
    try { localStorage.setItem(THEME_STORAGE_KEY, next); } catch { /* ok */ }
    document.documentElement.setAttribute("data-theme", resolveTheme(next));
    set({ theme: next, themeLabel: THEME_LABELS[next] });
  },

  reapplyTheme: () => {
    const theme = get().theme;
    document.documentElement.setAttribute("data-theme", resolveTheme(theme));
  },

  selectNextRow: () => {
    const state = get();
    if (state.items.length === 0) return;
    const currentIndex = state.items.findIndex((i) => i.id === state.selectedId);
    const nextIndex = currentIndex < state.items.length - 1 ? currentIndex + 1 : currentIndex;
    if (nextIndex !== currentIndex) {
      const item = state.items[nextIndex];
      set({ selectedId: item.id, selectedItem: item });
    }
  },

  selectPrevRow: () => {
    const state = get();
    if (state.items.length === 0) return;
    const currentIndex = state.items.findIndex((i) => i.id === state.selectedId);
    const prevIndex = currentIndex > 0 ? currentIndex - 1 : 0;
    if (prevIndex !== currentIndex) {
      const item = state.items[prevIndex];
      set({ selectedId: item.id, selectedItem: item });
    }
  },

  loadAllStrings: async () => {
    const state = get();
    if (!state.espPath) {
      // E2E test mode: check if E2E auto-seed has run (flag set as 'true' boolean)
      const e2eSeeded = (window as any).__e2eAutoSeeded === true;
      if (e2eSeeded) {
        set({ espPath: E2E_SENTINEL_ESP });
      } else {
        return; // real usage: no path, not in E2E mode → bail
      }
    }
    set({ isLoading: true });

    // E2E mode: use the real mock Tauri API to get data, then inject into store
    if ((window as any).__e2eAutoSeeded === true) {
      try {
        const count = await getStringsCount();
        const mockItems: SkyStringDTO[] = [];
        const CHUNK_SIZE = 25000;
        const totalChunks = Math.ceil(count / CHUNK_SIZE);

        // Fetch all chunks via mock Tauri IPC (synchronous in E2E mode)
        for (let i = 0; i < totalChunks; i++) {
          const offset = i * CHUNK_SIZE;
          const limit = Math.min(CHUNK_SIZE, count - offset);
          const chunk = await getStringsChunk(offset, limit);
          mockItems.push(...chunk);
        }

        const items = applyFilterAndSort(
          mockItems,
          state.filter,
          state.useRegex,
          state.statusFilter,
          state.recordFilter,
          state.vmadFilter,
          state.listIndex,
          state.sortField,
          state.sortDir
        );
        set({
          allItems: mockItems,
          items,
          total: mockItems.length,
          filtered: mockItems.length,
          isLoading: false,
          espStats: {
            total: mockItems.length,
            compressed_records: 0,
            strings_loaded: mockItems.length,
            parse_time_ms: 5,
            record_counts: { INFO: Math.floor(mockItems.length * 0.6), QUST: Math.floor(mockItems.length * 0.25), DIAL: Math.floor(mockItems.length * 0.15) },
            cached: false,
            esp_hash: "mock_hash_abc123",
          },
        });
        return;
      } catch (e: any) {
        console.error("E2E mock IPC failed:", e);
        set({ isLoading: false });
        return;
      }
    }
    try {
      const count = await getStringsCount();
      const CHUNK_SIZE = 25000;
      const CONCURRENCY = 3;
      const totalChunks = Math.ceil(count / CHUNK_SIZE);
      const allItems: SkyStringDTO[] = [];

      for (let round = 0; round < Math.ceil(totalChunks / CONCURRENCY); round++) {
        const start = round * CONCURRENCY;
        const batch: Promise<SkyStringDTO[]>[] = [];
        for (let i = start; i < Math.min(start + CONCURRENCY, totalChunks); i++) {
          const offset = i * CHUNK_SIZE;
          const limit = Math.min(CHUNK_SIZE, count - offset);
          batch.push(getStringsChunk(offset, limit));
        }
        const results = await Promise.all(batch);
        results.forEach((chunk) => allItems.push(...chunk));
      }

      get().setAllItems(allItems);
      toast.success(i18n.t("toast.loadedStrings", { count: allItems.length.toLocaleString() }));
    } catch (e: any) {
      console.error("Chunked loading failed:", e);
      toast.error(i18n.t("toast.loadingFailed") + ": " + e);
      // Fallback 1: try single-shot (may work for small datasets)
      try {
        const allItems = await getAllStrings();
        get().setAllItems(allItems);
        toast.success(i18n.t("toast.loadedFallback", { count: allItems.length.toLocaleString() }));
      } catch (e2: any) {
        console.error("Single-shot fallback also failed:", e2);
        // Fallback 2: paginated query
        try {
          const response = await queryStrings({
            file_id: state.espPath || "",
            offset: 0,
            limit: 100,
            filter: state.filter || undefined,
            sort_field: state.sortField,
            sort_dir: state.sortDir,
            status_filter: state.statusFilter || undefined,
          });
          set({
            items: response.items,
            total: response.total,
            filtered: response.filtered,
            allItems: [],
          });
        } catch (e3: any) {
          console.error("All fallbacks failed:", e3);
          toast.error(i18n.t("toast.allFallbacksFailed"));
        }
      }
    } finally {
      set({ isLoading: false });
    }
  },

  toggleSelectId: (id) => {
    const state = get();
    const newSet = new Set(state.selectedIds);
    if (newSet.has(id)) {
      newSet.delete(id);
    } else {
      newSet.add(id);
    }
    set({ selectedIds: newSet });
  },

  clearSelection: () => set({ selectedIds: new Set() }),

  setBatchConcurrency: (n) => set({ batchConcurrency: n }),

  startBatchTranslation: async () => {
    const state = get();
    if (state.selectedIds.size === 0) return;

    const ids = Array.from(state.selectedIds);
    set({
      batchState: "running",
      batchProgress: { completed: 0, total: ids.length },
      batchErrors: [],
    });

    try {
      await startStringBatchTranslate(ids, state.batchConcurrency);
    } catch (e: any) {
      toast.error(e?.toString() || "Batch translation failed");
      set({ batchState: "idle" });
    }
  },

  cancelBatchTranslation: async () => {
    set({ batchState: "cancelled" });
    try {
      await cancelStringBatchTranslate();
    } catch (e: any) {
      toast.error(e?.toString() || "Cancel failed");
    }
  },

  checkAndPromptRecovery: async (espHash) => {
    try {
      const resp = await checkPendingCache(espHash);
      if (resp.recovery) {
        set({ showRecoveryModal: true, recoveryInfo: resp.recovery, espHash });
      }
    } catch (e: any) {
      console.error("Recovery check failed:", e);
    }
  },

  applyRecovery: async () => {
    const state = get();
    if (!state.espHash) return;
    try {
      const result = await applyTranslationCache(state.espHash);
      toast.success(`Recovered ${result.applied_count} translations`);
      set({ showRecoveryModal: false, recoveryInfo: null });
      // Reload strings to reflect recovered translations
      await get().loadAllStrings();
    } catch (e: any) {
      toast.error(`Recovery failed: ${e}`);
    }
  },

  discardRecovery: async () => {
    const state = get();
    if (!state.espHash) return;
    try {
      await discardTranslationCache(state.espHash);
      set({ showRecoveryModal: false, recoveryInfo: null });
    } catch (e: any) {
      toast.error(`Discard failed: ${e}`);
    }
  },

  closeRecoveryModal: () => {
    set({ showRecoveryModal: false, recoveryInfo: null });
  },

  reset: () =>
    set({
      allItems: [],
      items: [],
      total: 0,
      filtered: 0,
      espPath: null,
      sstPath: null,
      stringsDir: null,
      espStats: null,
      sstStats: null,
      filter: "",
      useRegex: false,
      replaceText: "",
      statusFilter: null,
      recordFilter: null,
      vmadFilter: false,
      listIndex: null,
      selectedId: null,
      selectedItem: null,
      isDirty: false,
      espHash: null,
      showRecoveryModal: false,
      recoveryInfo: null,
      undoStack: [],
      redoStack: [],
      targetLang: "chinese",
      dataConfigs: null,
      activePanel: null,
      activeBottomTab: "home",
    }),
}));

// E2E helper: expose the raw zustand store so tests can inject state directly
if (typeof window !== "undefined") {
  (window as any).__zustandStore = useAppStore;
}
