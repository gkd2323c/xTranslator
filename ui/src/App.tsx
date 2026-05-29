import { useEffect, useRef, Suspense, lazy } from "react";
import { Toaster } from "react-hot-toast";
import toast from "react-hot-toast";
import { Loader } from "lucide-react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import { setI18nLanguage } from "./i18n";
import { useAppStore } from "./stores/appStore";
import { GroupedMenuBar } from "./components/GroupedMenuBar";
import { SidePanel } from "./components/SidePanel";
import { VocabularyPanel } from "./components/bottom/VocabularyPanel";
import { LogPanel } from "./components/bottom/LogPanel";
import { ExplorerTab } from "./components/bottom/ExplorerTab";
import { HeaderTab } from "./components/bottom/HeaderTab";
import { StringTable } from "./components/StringTable";
import { EditorDialog } from "./components/EditorPanel/index";
import { BatchTranslateBar } from "./components/BatchTranslateBar";
import { RecoveryPromptModal } from "./components/RecoveryPromptModal";
import { StatusBar } from "./components/StatusBar";
import { Modal } from "./components/ui";
import { SplitPaneLayout } from "./components/SplitPaneLayout";
import { RightPanelContainer } from "./components/RightPanelContainer";

// 工具面板懒加载（首次打开时按需加载，减小首屏包体积）
const BatchPanel = lazy(() => import("./components/BatchPanel").then(m => ({ default: m.BatchPanel })));
const DialogView = lazy(() => import("./components/DialogView").then(m => ({ default: m.DialogView })));
const McmPanel = lazy(() => import("./components/McmPanel").then(m => ({ default: m.McmPanel })));
const FinalizePanel = lazy(() => import("./components/FinalizePanel").then(m => ({ default: m.FinalizePanel })));
const DataConfigsPanel = lazy(() => import("./components/DataConfigsPanel").then(m => ({ default: m.DataConfigsPanel })));
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
// ├── GroupedMenuBar (分组菜单栏)
// ├── BatchTranslateBar (批处理进度条)
// ├── app-body
// │   └── app-main
// │       └── SplitPaneLayout (可拖拽分栏布局)
// │           ├── StringTable (虚拟滚动表格，主内容区)
// │           └── app-bottom-panel (底部标签页，可拖拽调整)
// │               ├── SidePanel (统计信息 → overview 标签页)
// │               ├── VocabularyPanel (词汇库)
// │               ├── LogPanel (日志)
// │               ├── ExplorerTab (资源浏览器 → explorer 标签页，Task 14)
// │               └── HeaderTab (头部工具 → header 标签页，Task 14)
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
  const activeRightPanel = useAppStore((s) => s.activeRightPanel);
  const setActiveRightPanel = useAppStore((s) => s.setActiveRightPanel);
  const panelLayout = useAppStore((s) => s.panelLayout);
  const setPanelSize = useAppStore((s) => s.setPanelSize);
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
        // Escape 链：编辑对话框 → 工具面板 → 右侧面板 → 行选择
        if (editorOpen) {
          setEditorOpen(false);
        } else if (activePanel) {
          setActivePanel(null);
        } else if (activeRightPanel) {
          setActiveRightPanel(null);
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
      } else if (e.ctrlKey && e.key === "1") {
        // Ctrl+1 - 切换到模态编辑器模式
        e.preventDefault();
        useAppStore.getState().setEditorMode("modal");
      } else if (e.ctrlKey && e.key === "2") {
        // Ctrl+2 - 切换到侧边栏编辑器模式
        e.preventDefault();
        useAppStore.getState().setEditorMode("sidebar");
      } else if (e.ctrlKey && e.key === "3") {
        // Ctrl+3 - 切换到内联编辑器模式
        e.preventDefault();
        useAppStore.getState().setEditorMode("inline");
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [setSelectedById, undo, redo, editorOpen, setEditorOpen, activePanel, setActivePanel, activeRightPanel, setActiveRightPanel]);

// ── E2E 模拟数据自动初始化 ──
  // 在 Playwright 测试模式 (VITE_E2E=true) 下，如果 base.ts 尚未植入数据（例如没有 fixture 的直接导航），
  // 现在进行植入以便表格能够渲染行。
  useEffect(() => {
    const timeout = setTimeout(() => {
      if (typeof (window as any).__e2eAutoSeed === "function" && !(window as any).__e2eAutoSeeded) {
        (window as any).__e2eAutoSeed();
      }
    }, 300); // 延迟执行以让 base.ts 的植入（若有）优先完成
    return () => clearTimeout(timeout);
  }, []); // 仅在挂载时运行一次

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
      if (cfg.editor_mode) useAppStore.getState().setEditorMode(cfg.editor_mode as any);
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
          /* 静默处理 */
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
      {/* 工具面板（5 个模态框，条件渲染：仅激活时挂载到 DOM） */}
      {/* BSA/PEX/FUZ/Compare 现在作为右侧面板显示，见 RightPanelContainer */}
      {activePanel === "batch" && (
        <Modal open onClose={() => setActivePanel(null)} title={t("batch.title")} size="lg">
          <Suspense fallback={<div className="modal-loading"><Loader size={24} /></div>}><BatchPanel /></Suspense>
        </Modal>
      )}
      {activePanel === "dialog" && (
        <Modal open onClose={() => setActivePanel(null)} title={t("dialog.title")} size="lg">
          <Suspense fallback={<div className="modal-loading"><Loader size={24} /></div>}><DialogView /></Suspense>
        </Modal>
      )}
      {activePanel === "mcm" && (
        <Modal open onClose={() => setActivePanel(null)} title={t("mcm.title")} size="lg">
          <Suspense fallback={<div className="modal-loading"><Loader size={24} /></div>}><McmPanel /></Suspense>
        </Modal>
      )}
      {activePanel === "finalize" && (
        <Modal open onClose={() => setActivePanel(null)} title={t("finalize.title")} size="lg">
          <Suspense fallback={<div className="modal-loading"><Loader size={24} /></div>}><FinalizePanel /></Suspense>
        </Modal>
      )}
      {activePanel === "dataConfigs" && (
        <Modal open onClose={() => setActivePanel(null)} title={t("dataConfigs.title")} size="lg">
          <Suspense fallback={<div className="modal-loading"><Loader size={24} /></div>}><DataConfigsPanel /></Suspense>
        </Modal>
      )}
      
      {/* 菜单栏（分组下拉菜单 + 工具栏） */}
      <GroupedMenuBar />
      
      {/* 字符串级批量翻译进度条 */}
      <BatchTranslateBar />
      
      {/* 主应用区域 */}
      <div className="app-body">
        <main className="app-main">
          <SplitPaneLayout
            rightPanel={activeRightPanel ? <RightPanelContainer /> : null}
            bottomPanel={
              showBottomPanel ? (
                <div className="app-bottom-panel">
                  {/* 标签页按钮 */}
                  <div className="bottom-panel-tabs">
                    {(["overview", "vocabulary", "log", "explorer", "header"] as const).map((tab) => (
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
                    {activeBottomTab === "overview" && <SidePanel />}
                    {activeBottomTab === "vocabulary" && <VocabularyPanel />}
                    {activeBottomTab === "log" && <LogPanel />}
                    {activeBottomTab === "explorer" && <ExplorerTab />}
                    {activeBottomTab === "header" && <HeaderTab />}
                  </div>
                </div>
              ) : null
            }
            rightPanelVisible={!!activeRightPanel}
            bottomPanelVisible={showBottomPanel}
            rightPanelSize={panelLayout.rightPanelSize}
            bottomPanelSize={panelLayout.bottomPanelSize}
            onRightPanelResize={(size) => setPanelSize("right", size)}
            onBottomPanelResize={(size) => setPanelSize("bottom", size)}
          >
            {/* 虚拟滚动表格（主要工作区） */}
            <div className="app-table-area">
              <StringTable />
            </div>
          </SplitPaneLayout>
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
