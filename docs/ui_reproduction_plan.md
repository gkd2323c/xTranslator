# 原版界面复刻方案

> 基于 `legacy/original-delphi/` 的窗体代码和 `tests/pics/` 的截图，整理当前 Rust/Tauri 前端的 UI 复刻路线。

## 目标

- 先复刻信息密度、控件层级和交互节奏，再做像素级细节。
- 复用现有功能和 IPC，优先重排前端结构，不重做后端。
- 主窗口优先，工具窗和低频弹窗后置。

## 参考依据

- 原版窗体：`legacy/original-delphi/TESVT_*.dfm`
- 原版逻辑：`legacy/original-delphi/TESVT_*.pas`
- 截图样本：`tests/pics/`
- 当前实现：`ui/src/App.tsx`、`ui/src/components/*`

## 现状对比

| 原版区域 | 当前实现 | 结论 |
|---|---|---|
| 主窗口 `TESVT_main.dfm` | `App.tsx` + `MenuBar.tsx` + `StringTable.tsx` + `SidePanel.tsx` | 功能齐，但视觉密度仍偏现代 |
| 搜索/编辑 `TESVT_search.dfm` / `TESVT_searchandreplace.dfm` | `EditorPanel.tsx`、表格 Replace All | 可用，但布局不够原版化 |
| 设置总窗 `TESVT_LangPref.dfm` | `SettingsDialog.tsx` | 功能覆盖较多，页面结构需继续贴近原版 |
| 批处理器 `TESVT_commandProcessor.dfm` | `BatchPanel.tsx` | 功能有了，界面还偏“工具面板” |
| Header Processor `TESVT_FormData.dfm` | `HeaderProcessorPanel.tsx` | 这是当前最接近但仍需重构的一块 |
| Header Wizard `TESVT_HeaderWizard.dfm` | `HeaderWizardPanel.tsx` | 可用，但离原版向导式布局还远 |
| BSA 浏览器 `TESVT_Browser.dfm` / `TESVT_bsa.pas` | `BsaBrowser.tsx` | 可用，树+列表的层级还可再贴近 |
| ESP 对比 `TESVT_EspCompareOpts.dfm` | `EspComparePanel.tsx` | 当前更像结果面板，缺原版选项感 |
| Regex 工具 `TESVT_regex.dfm` | 当前无独立页面 | 需要补独立工具窗 |
| SpellCheck `TESVT_spOptions.dfm` | 当前无独立页面 | 需要补独立设置窗 |
| Codepage `TESVT_Codepage.dfm` / `TESVT_ChooseCP.dfm` | 当前无独立页面 | 需要补独立对话框 |
| OldDialogStyle `TESVT_OldDialog.dfm` | 当前无独立页面 | 需要补兼容窗 |

## 复刻顺序

### P0 主窗口

- 统一顶栏密度，恢复原版“菜单 + 紧凑工具条”的层级。
- 调整字符串表列顺序、宽度和选中态，贴近 VCL `VirtualStringTree`。
- 强化底部 tabs、状态条和颜色语义。

### P1 高频编辑链路

- 把编辑窗进一步靠近原版 `SearchandEdit`：左源文、右译文、侧边操作、底部状态区。
- 把 `Set_Options` 还原成更接近原版的多页签总设置窗。
- 让批处理器、Header Processor、Header Wizard 的控件层级更像原版，而不是纯功能面板。

### P2 工具与辅助窗

- 补独立 `Regex`、`SpellCheck`、`Codepage`、`OldDialogStyle`、`ToolBox` 类窗口。
- 对 `BSA`、`ESP Compare`、`ESP/BSA` 相关窗体做视觉统一。
- 仅在 UI 需要时补少量 IPC，不为“看起来像”而重做核心逻辑。

### P3 细节统一

- 字体、间距、按钮尺寸、菜单快捷键提示、图标密度统一。
- 尽量使用同一套复古风格的控件样式，而不是混用现代卡片风。
- 对照截图逐个补齐 hover、disabled、选中和分割线效果。

## 当前已有截图

- 主窗口
- 搜索/编辑窗
- `Set_Options`
- `BatchProcessor`
- `Header Processor`
- `FormRegex`

## 仍建议补的截图

- `TESVT_Browser.dfm`
- `TESVT_EspCompareOpts.dfm`
- `TESVT_spOptions.dfm`
- `TESVT_Codepage.dfm`
- `TESVT_ChooseCP.dfm`
- `TESVT_OldDialog.dfm`
- `TESVT_ToolBox.dfm`
- `TESVT_ToolboxOptions.dfm`

## 验收标准

- 主窗口一眼看上去还是“老派工具型 VCL”，不是现代后台面板。
- 高频窗体的控件位置、密度和操作路径与原版一致。
- 新增 UI 不破坏现有功能和现有 IPC。
- 每补一个窗体，都能在 `tests/pics/` 找到对应截图或补充截图。
