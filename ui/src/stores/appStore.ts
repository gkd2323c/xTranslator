import { create } from "zustand";
import type { SkyStringDTO, LoadEspResponse, LoadSstResponse, BatchEntry, BatchStatus, DataConfigsDto } from "../api/strings";
import { getAllStrings, getStringsChunk, getStringsCount, queryStrings, updateTranslation } from "../api/strings";
import { saveConfig } from "../api/strings";
import toast from "react-hot-toast";
import i18n from "../i18n";

export type Theme = "dark" | "light" | "gray" | "auto";

const THEME_STORAGE_KEY = "xtranslator-theme";

function getSystemPrefersDark(): boolean {
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

function resolveTheme(theme: Theme): "dark" | "light" | "gray" {
  if (theme === "auto") {
    return getSystemPrefersDark() ? "dark" : "light";
  }
  return theme;
}

interface LoadProgress {
  stage: string;
  current: number;
  total: number;
  percentage: number;
  message: string;
}

interface UndoEntry {
  id: number;
  oldTranslation: string;
  oldStatus: string;
}

const MAX_UNDO_STACK = 100;

interface AppState {
  // Full dataset (all strings from backend)
  allItems: SkyStringDTO[];
  // Filtered + sorted view for display
  items: SkyStringDTO[];
  total: number;
  filtered: number;

  // Loading state
  isLoading: boolean;
  isParsing: boolean;
  error: string | null;
  loadProgress: LoadProgress | null;

  // File info
  espPath: string | null;
  sstPath: string | null;
  stringsDir: string | null;
  language: string;
  targetLang: string;

  // Load stats
  espStats: LoadEspResponse | null;
  sstStats: LoadSstResponse | null;

  // Data configs (CTDA, field sizes, etc.)
  dataConfigs: DataConfigsDto | null;
  showDataConfigsPanel: boolean;

  // Filter / sort
  filter: string;
  useRegex: boolean;
  replaceText: string;
  statusFilter: string | null;
  recordFilter: string | null;
  vmadFilter: boolean;
  sortField: string;
  sortDir: "asc" | "desc";

  // Selection (by item id, not array index)
  selectedId: number | null;
  selectedItem: SkyStringDTO | null;

  // Theme
  theme: Theme;
  themeLabel: string;

  // Dirty state (unsaved translation changes)
  isDirty: boolean;

  // Undo/Redo
  undoStack: UndoEntry[];
  redoStack: UndoEntry[];

  // Batch processor
  showBatchPanel: boolean;
  batchEntries: BatchEntry[];
  batchStatus: BatchStatus | null;

  // BSA Browser
  showBsaBrowser: boolean;

  // PEX Panel
  showPexPanel: boolean;

  // FUZ Panel
  showFuzPanel: boolean;

  // Dialog View
  showDialogView: boolean;

  // MCM Panel
  showMcmPanel: boolean;

  // ESP Compare Panel
  showEspCompare: boolean;

  // Finalize Panel
  showFinalizePanel: boolean;

  // Actions
  setAllItems: (items: SkyStringDTO[]) => void;
  setLoading: (loading: boolean) => void;
  setParsing: (parsing: boolean) => void;
  setError: (error: string | null) => void;
  setLoadProgress: (progress: LoadProgress | null) => void;
  setEspLoaded: (path: string, stats: LoadEspResponse, stringsDir?: string) => void;
  setSstLoaded: (path: string, stats: LoadSstResponse) => void;
  setTargetLang: (lang: string) => void;
  setFilter: (filter: string) => void;
  setUseRegex: (use: boolean) => void;
  setReplaceText: (text: string) => void;
  setStatusFilter: (status: string | null) => void;
  setRecordFilter: (record: string | null) => void;
  setVmadFilter: (enabled: boolean) => void;
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
  selectPrevRow: () => void;
  loadAllStrings: () => Promise<void>;
  setShowBatchPanel: (show: boolean) => void;
  setShowBsaBrowser: (show: boolean) => void;
  setShowPexPanel: (show: boolean) => void;
  setShowFuzPanel: (show: boolean) => void;
  setShowDialogView: (show: boolean) => void;
  setShowMcmPanel: (show: boolean) => void;
  setShowEspCompare: (show: boolean) => void;
  setShowFinalizePanel: (show: boolean) => void;
  setDataConfigs: (configs: DataConfigsDto | null) => void;
  setShowDataConfigsPanel: (show: boolean) => void;
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
    if (stored === "light" || stored === "gray" || stored === "dark" || stored === "auto") return stored;
  } catch { /* localStorage unavailable */ }
  return "auto";
}

const THEME_LABELS: Record<Theme, string> = { dark: "Dark", light: "Light", gray: "Gray", auto: "Auto" };
const THEME_NEXT: Record<Theme, Theme> = { dark: "light", light: "gray", gray: "auto", auto: "dark" };

function applyFilterAndSort(
  allItems: SkyStringDTO[],
  filter: string,
  useRegex: boolean,
  statusFilter: string | null,
  recordFilter: string | null,
  vmadFilter: boolean,
  sortField: string,
  sortDir: "asc" | "desc"
): SkyStringDTO[] {
  let result = allItems;

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

export const useAppStore = create<AppState>((set, get) => ({
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
  sortField: "id",
  sortDir: "asc",
  selectedId: null,
  selectedItem: null,
  theme: getInitialTheme(),
  themeLabel: THEME_LABELS[getInitialTheme()],
  isDirty: false,
  undoStack: [],
  redoStack: [],
  showBatchPanel: false,
  batchEntries: [],
  batchStatus: null,
  showBsaBrowser: false,
  showPexPanel: false,
  showFuzPanel: false,
  showDialogView: false,

  showMcmPanel: false,
  showEspCompare: false,
  showFinalizePanel: false,
  dataConfigs: null,
  showDataConfigsPanel: false,

  setAllItems: (allItems) => {
    const state = get();
    const items = applyFilterAndSort(
      allItems,
      state.filter,
      state.useRegex,
      state.statusFilter,
      state.recordFilter,
      state.vmadFilter,
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
    set({ espPath, espStats, stringsDir, sstStats: null }),

  setSstLoaded: (sstPath, sstStats) => set({ sstPath, sstStats }),

  setTargetLang: (targetLang) => set({ targetLang }),

  setFilter: (filter) => {
    const state = get();
    const items = applyFilterAndSort(
      state.allItems,
      filter,
      state.useRegex,
      state.statusFilter,
      state.recordFilter,
      state.vmadFilter,
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

    let changed = 0;
    for (const item of candidates) {
      const target = item.translation || item.source;
      const replaced = target.replace(regex, state.replaceText);
      if (replaced !== target) {
        try {
          await updateTranslation(item.id, replaced);
          changed++;
        } catch (e: any) {
          console.error(`Replace failed for id=${item.id}:`, e);
        }
      }
    }

    // Reload all strings to sync with backend state
    await get().loadAllStrings();

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
      state.sortField,
      state.sortDir
    );
    set({ vmadFilter, items, filtered: items.length });
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
      sortField,
      sortDir
    );
    set({ sortField, sortDir, items, filtered: items.length });
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

  setShowBatchPanel: (showBatchPanel) => set({
    showBatchPanel,
    showBsaBrowser: false,
    showPexPanel: false,
    showFuzPanel: false,
    showDialogView: false,
    showMcmPanel: false,
    showEspCompare: false,
  }),
  setShowBsaBrowser: (showBsaBrowser) => set({
    showBatchPanel: false,
    showBsaBrowser,
    showPexPanel: false,
    showFuzPanel: false,
    showDialogView: false,
    showMcmPanel: false,
    showEspCompare: false,
  }),
  setShowPexPanel: (showPexPanel) => set({
    showBatchPanel: false,
    showBsaBrowser: false,
    showPexPanel,
    showFuzPanel: false,
    showDialogView: false,
    showMcmPanel: false,
    showEspCompare: false,
  }),
  setShowFuzPanel: (showFuzPanel) => set({
    showBatchPanel: false,
    showBsaBrowser: false,
    showPexPanel: false,
    showFuzPanel,
    showDialogView: false,
    showMcmPanel: false,
    showEspCompare: false,
  }),
  setShowDialogView: (showDialogView) => set({
    showBatchPanel: false,
    showBsaBrowser: false,
    showPexPanel: false,
    showFuzPanel: false,
    showDialogView,
    showMcmPanel: false,
    showEspCompare: false,
  }),
  setShowMcmPanel: (showMcmPanel) => set({
    showBatchPanel: false,
    showBsaBrowser: false,
    showPexPanel: false,
    showFuzPanel: false,
    showDialogView: false,
    showMcmPanel,
    showEspCompare: false,
  }),
  setShowEspCompare: (showEspCompare) => set({
    showBatchPanel: false,
    showBsaBrowser: false,
    showPexPanel: false,
    showFuzPanel: false,
    showDialogView: false,
    showMcmPanel: false,
    showEspCompare,
  }),
  setShowFinalizePanel: (showFinalizePanel) => set({
    showBatchPanel: false,
    showBsaBrowser: false,
    showPexPanel: false,
    showFuzPanel: false,
    showDialogView: false,
    showMcmPanel: false,
    showEspCompare: false,
    showFinalizePanel,
  }),
  setDataConfigs: (dataConfigs) => set({ dataConfigs }),
  setShowDataConfigsPanel: (showDataConfigsPanel) => set({
    showBatchPanel: false,
    showBsaBrowser: false,
    showPexPanel: false,
    showFuzPanel: false,
    showDialogView: false,
    showMcmPanel: false,
    showEspCompare: false,
    showFinalizePanel: false,
    showDataConfigsPanel,
  }),

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
    if (!state.espPath) return;
    set({ isLoading: true });
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
            file_id: state.espPath,
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
      selectedId: null,
      selectedItem: null,
      isDirty: false,
      undoStack: [],
      redoStack: [],
      targetLang: "chinese",
      dataConfigs: null,
      showDataConfigsPanel: false,
    }),
}));
