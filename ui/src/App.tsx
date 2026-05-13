import { useEffect, useRef } from "react";
import { Toaster } from "react-hot-toast";
import toast from "react-hot-toast";
import { Loader } from "lucide-react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import { setI18nLanguage } from "./i18n";
import { useAppStore } from "./stores/appStore";
import { MenuBar } from "./components/MenuBar";
import { SidePanel } from "./components/SidePanel";
import { BatchPanel } from "./components/BatchPanel";
import { BsaBrowser } from "./components/BsaBrowser";
import { PexPanel } from "./components/PexPanel";
import { FuzPanel } from "./components/FuzPanel";
import { DialogView } from "./components/DialogView";
import { McmPanel } from "./components/McmPanel";
import { EspComparePanel } from "./components/EspComparePanel";
import { FinalizePanel } from "./components/FinalizePanel";
import { DataConfigsPanel } from "./components/DataConfigsPanel";
import { VocabularyPanel } from "./components/bottom/VocabularyPanel";
import { HeuristicPanel } from "./components/bottom/HeuristicPanel";
import { EspTreePanel } from "./components/bottom/EspTreePanel";
import { QuestsPanel } from "./components/bottom/QuestsPanel";
import { LogPanel } from "./components/bottom/LogPanel";
import { HeaderProcessorPanel } from "./components/bottom/HeaderProcessorPanel";
import { HeaderWizardPanel } from "./components/bottom/HeaderWizardPanel";
import { StringTable } from "./components/StringTable";
import { EditorDialog } from "./components/EditorPanel";
import { BatchTranslateBar } from "./components/BatchTranslateBar";
import { RecoveryPromptModal } from "./components/RecoveryPromptModal";
import { StatusBar } from "./components/StatusBar";
import { Modal } from "./components/ui";
import { autoBackupSst, loadConfig, setOpenAiApiKey, setDeeplApiKey, setBaiduApiKey, setYoudaoApiKey, setAzureApiKey, setTranslationProvider } from "./api/strings";
import "./App.css";
import "./components/ui/ui.css";

// 自动备份间隔（5 分钟）
const AUTO_BACKUP_INTERVAL_MS = 5 * 60 * 1000;

// 主应用组件 - xTranslator 的根组件
///
// 职责：
// - 管理全局应用布局（菜单栏、主表格、底部面板、状态栏）
// - 协调所有工具面板的显示/隐藏
// - 处理全局快捷键（Escape、Ctrl+Z/Y）
// - 加载和应用用户配置（主题、语言、API Key）
// - 监听后端事件（ESP 加载、SST 加载、批处理进度）
// - 管理自动备份定时器
///
// 布局结构：
// ```
// App
// ├── Toaster (toast 通知)
// ├── RecoveryPromptModal (恢复提示)
// ├── EditorDialog (编辑对话框)
// ├── 9× Modal (工具面板)
// ├── MenuBar (菜单栏)
// ├── BatchTranslateBar (批处理进度条)
// ├── app-body
// │   └── app-main
// │       ├── app-table-area → StringTable (虚拟滚动表格)
// │       └── app-bottom-panel (底部标签页)
// │           ├── SidePanel (统计信息)
// │           ├── VocabularyPanel (词汇库)
// │           ├── HeuristicPanel (启发式搜索)
// │           ├── EspTreePanel (记录树)
// │           ├── PexPanel (PEX 脚本)
// │           ├── QuestsPanel (任务)
// │           ├── DialogView (对话)
// │           ├── LogPanel (日志)
// │           ├── HeaderProcessorPanel (头部处理)
// │           └── HeaderWizardPanel (头部向导)
// ├── StatusBar (状态栏)
// └── app-overlay (加载覆盖层)
// ```
///
// 关键事件监听：
// - "batch-string-progress" - 字符串级批量翻译进度
// - "batch-string-complete" - 字符串级批量翻译完成
///
// 快捷键：
// - Escape - 关闭编辑对话框 → 关闭工具面板 → 取消选择
// - Ctrl+Z / Cmd+Z - 撤销
// - Ctrl+Y / Cmd+Y / Ctrl+Shift+Z - 重做
function App() {
  const { t } = useTranslation();
  
  // ── 从全局状态中提取所需的字段和方法 ──
  // 使用 Zustand 的选择器模式，只订阅需要的字段，避免不必要的重新渲染
  const setSelectedById = useAppStore((s) => s.setSelectedById);
  const isLoading = useAppStore((s) => s.isLoading);
  const isParsing = useAppStore((s) => s.isParsing);
  const loadProgress = useAppStore((s) => s.loadProgress);
  const theme = useAppStore((s) => s.theme);
  const activePanel = useAppStore((s) => s.activePanel);
  const setActivePanel = useAppStore((s) => s.setActivePanel);
  const activeBottomTab = useAppStore((s) => s.activeBottomTab);
  const showBottomPanel = useAppStore((s) => s.showBottomPanel);
  const editorOpen = useAppStore((s) => s.editorOpen);
  const setEditorOpen = useAppStore((s) => s.setEditorOpen);
  const isDirty = useAppStore((s) => s.isDirty);
  const sstPath = useAppStore((s) => s.sstPath);
  const undo = useAppStore((s) => s.undo);
  const redo = useAppStore((s) => s.redo);
  const reapplyTheme = useAppStore((s) => s.reapplyTheme);
  const backupIdRef = useRef<string | null>(null);

  // 全局快捷键处理
  ///
  // 快捷键链：
  // 1. Escape - 关闭编辑对话框（如果打开）
  // 2. Escape - 关闭工具面板（如果打开）
  // 3. Escape - 取消行选择
  // 4. Ctrl+Z / Cmd+Z - 撤销翻译修改
  // 5. Ctrl+Y / Cmd+Y / Ctrl+Shift+Z - 重做翻译修改
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        // Escape 链：编辑对话框 → 工具面板 → 行选择
        if (editorOpen) {
          setEditorOpen(false);
        } else if (activePanel) {
          setActivePanel(null);
        } else {
          setSelectedById(null);
        }
      } else if ((e.ctrlKey || e.metaKey) && e.key === "z" && !e.shiftKey) {
        // Ctrl+Z / Cmd+Z - 撤销
        e.preventDefault();
        undo();
      } else if ((e.ctrlKey || e.metaKey) && (e.key === "y" || (e.key === "z" && e.shiftKey))) {
        // Ctrl+Y / Cmd+Y / Ctrl+Shift+Z - 重做
        e.preventDefault();
        redo();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [setSelectedById, undo, redo, editorOpen, setEditorOpen, activePanel, setActivePanel]);

  // 应用启动时加载配置
  ///
  // 从后端加载保存的配置，包括：
  // - 主题设置（obsidian / dark / light / slate / auto）
  // - 语言设置
  // - API Key（OpenAI、DeepL、百度、有道、Azure）
  // - 当前翻译提供方
  // - ESP 模式开关
  useEffect(() => {
    loadConfig().then((cfg) => {
      if (cfg.theme) useAppStore.getState().setTheme(cfg.theme as any);
      if (cfg.language) setI18nLanguage(cfg.language);
      if (cfg.openai_api_key) setOpenAiApiKey(cfg.openai_api_key);
      if (cfg.deepl_api_key) setDeeplApiKey(cfg.deepl_api_key);
      if (cfg.baidu_app_id && cfg.baidu_key) setBaiduApiKey(cfg.baidu_app_id, cfg.baidu_key);
      if (cfg.youdao_app_key && cfg.youdao_secret_key) setYoudaoApiKey(cfg.youdao_app_key, cfg.youdao_secret_key);
      if (cfg.azure_key) setAzureApiKey(cfg.azure_key);
      if (cfg.current_provider) setTranslationProvider(cfg.current_provider);
      if (cfg.esp_mode !== undefined) useAppStore.getState().setEspMode(cfg.esp_mode);
    }).catch(() => {});
  }, []);

  // 主题变化时重新应用 CSS 类
  ///
  // 当用户切换主题或系统主题变化时，更新 DOM 的主题类
  useEffect(() => {
    reapplyTheme();
  }, [theme, reapplyTheme]);

  // 监听后端事件：字符串级批量翻译进度
  ///
  // 事件流：
  // 1. "batch-string-progress" - 单个字符串翻译完成
  //    - 更新该字符串的翻译
  //    - 更新进度计数器
  // 2. "batch-string-complete" - 整个批处理完成
  //    - 显示完成提示（成功/失败统计）
  //    - 更新批处理状态
  useEffect(() => {
    const unlisten = listen<{ str_id: number; translated: string; error: string | null; completed: number; total: number }>(
      "batch-string-progress",
      (event) => {
        const store = useAppStore.getState();
        store.updateItemTranslation(event.payload.str_id, event.payload.translated || "");
        useAppStore.setState({
          batchProgress: {
            completed: event.payload.completed,
            total: event.payload.total,
          },
        });
      }
    );

    const unlistenComplete = listen<{ total: number; succeeded: number; failed: number; errors: any[] }>(
      "batch-string-complete",
      (event) => {
        const { succeeded, failed } = event.payload;
        if (failed > 0) {
          toast(t("batch.completeWithFailures", { succeeded, failed }));
        } else {
          toast.success(t("batch.completeSuccess", { succeeded }));
        }
        useAppStore.setState({ batchState: "completed" });
      }
    );

    return () => {
      unlisten.then((u) => u());
      unlistenComplete.then((u) => u());
    };
  }, []);

  // 监听系统主题变化
  ///
  // 当系统主题从浅色切换到深色（或反之）时，
  // 如果应用设置为 "auto" 主题，则自动更新应用主题
  useEffect(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = () => {
      if (theme === "auto") reapplyTheme();
    };
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, [theme, reapplyTheme]);

  // 自动备份定时器
  ///
  // 每 5 分钟检查一次：
  // - 如果有未保存的修改（isDirty）
  // - 且 SST 文件已加载
  // - 则自动备份 SST 文件
  ///
  // 备份策略：
  // - 最多保留 10 个备份
  // - 超过限制时自动删除最旧的备份
  // - 备份文件名格式：{filename}.backup.{timestamp}
  useEffect(() => {
    const interval = setInterval(async () => {
      if (!sstPath) return;
      if (isDirty) {
        try {
          const res = await autoBackupSst({ sst_path: sstPath, max_backups: 10 });
          if (res.backup_path) {
            backupIdRef.current = toast.success(
              `${t("toast.autoBackupSaved")} (${res.total_backups})`,
              { duration: 2000 }
            );
          }
        } catch {
          /* silent */
        }
      }
    }, AUTO_BACKUP_INTERVAL_MS);
    return () => {
      clearInterval(interval);
      if (backupIdRef.current !== null) toast.dismiss(backupIdRef.current);
    };
  }, [sstPath, isDirty, t]);

  // 计算应用是否被锁定（显示加载覆盖层）
  // 当 ESP 加载或解析中时，禁用用户交互
  const isLocked = isLoading || isParsing;

  return (
    <div className="app">
      {/* Toast 通知容器 */}
      <Toaster position="top-right" />
      
      {/* 恢复提示模态框（SST 加载失败时显示） */}
      <RecoveryPromptModal />
      
      {/* 编辑对话框（单字符串编辑） */}
      <EditorDialog open={editorOpen} onClose={() => setEditorOpen(false)} />
      
      {/* 工具面板（9 个模态框，单选互斥） */}
      <Modal open={activePanel === "batch"} onClose={() => setActivePanel(null)} title={t("batch.title")} size="lg"><BatchPanel /></Modal>
      <Modal open={activePanel === "bsa"} onClose={() => setActivePanel(null)} title={t("bsa.title")} size="lg"><BsaBrowser /></Modal>
      <Modal open={activePanel === "pex"} onClose={() => setActivePanel(null)} title={t("pex.title")} size="lg"><PexPanel /></Modal>
      <Modal open={activePanel === "fuz"} onClose={() => setActivePanel(null)} title={t("fuz.title")} size="lg"><FuzPanel /></Modal>
      <Modal open={activePanel === "dialog"} onClose={() => setActivePanel(null)} title={t("dialog.title")} size="lg"><DialogView /></Modal>
      <Modal open={activePanel === "mcm"} onClose={() => setActivePanel(null)} title={t("mcm.title")} size="lg"><McmPanel /></Modal>
      <Modal open={activePanel === "espCompare"} onClose={() => setActivePanel(null)} title={t("espCompare.title")} size="lg"><EspComparePanel /></Modal>
      <Modal open={activePanel === "finalize"} onClose={() => setActivePanel(null)} title={t("finalize.title")} size="lg"><FinalizePanel /></Modal>
      <Modal open={activePanel === "dataConfigs"} onClose={() => setActivePanel(null)} title={t("dataConfigs.title")} size="lg"><DataConfigsPanel /></Modal>
      
      {/* 菜单栏（文件、翻译、选项、工具、向导） */}
      <MenuBar />
      
      {/* 字符串级批量翻译进度条 */}
      <BatchTranslateBar />
      
      {/* 主应用区域 */}
      <div className="app-body">
        <main className="app-main">
          {/* 虚拟滚动表格（主要工作区） */}
          <div className="app-table-area">
            <StringTable />
          </div>
          
          {/* 底部面板（10 个标签页，可折叠） */}
          {showBottomPanel && (
            <>
              {/* 分割线（可拖动调整高度） */}
              <div className="app-bottom-splitter" />
              
              <div className="app-bottom-panel">
                {/* 标签页按钮 */}
                <div className="bottom-panel-tabs">
                  {(["home", "vocabulary", "heuristic", "espTree", "pex", "quests", "dialogs", "log", "headerProc", "headerWizard"] as const).map((tab) => (
                    <button
                      key={tab}
                      className={`bottom-tab ${activeBottomTab === tab ? "bottom-tab-active" : ""}`}
                      onClick={() => useAppStore.getState().setActiveBottomTab(tab)}
                    >
                      {t(`bottomTabs.${tab}`)}
                    </button>
                  ))}

                </div>
                
                {/* 标签页内容（条件渲染） */}
                <div className="bottom-panel-content">
                  {activeBottomTab === "home" && <SidePanel />}
                  {activeBottomTab === "vocabulary" && <VocabularyPanel />}
                  {activeBottomTab === "heuristic" && <HeuristicPanel />}
                  {activeBottomTab === "espTree" && <EspTreePanel />}
                  {activeBottomTab === "pex" && <PexPanel />}
                  {activeBottomTab === "quests" && <QuestsPanel />}
                  {activeBottomTab === "dialogs" && <DialogView />}
                  {activeBottomTab === "log" && <LogPanel />}
                  {activeBottomTab === "headerProc" && <HeaderProcessorPanel />}
                  {activeBottomTab === "headerWizard" && <HeaderWizardPanel />}
                </div>
              </div>
            </>
          )}
        </main>
      </div>
      
      {/* 状态栏（文件信息、统计、快捷键提示） */}
      <StatusBar />
      
      {/* 加载覆盖层（ESP 加载或解析中时显示） */}
      {isLocked && (
        <div className="app-overlay">
          <Loader size={40} className="app-overlay-spinner" />
          <p className="app-overlay-message">
            {loadProgress?.message || (isParsing ? t("app.parsing") : t("app.processing"))}
          </p>
          {loadProgress && loadProgress.total > 0 && (
            <div className="app-overlay-progress">
              <div
                className="app-overlay-progress-bar"
                style={{ width: `${loadProgress.percentage}%` }}
              />
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export default App;
