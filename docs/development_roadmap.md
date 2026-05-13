# xTranslator 开发路线图

> 基于 Delphi 1.6.0 原版（~67,000 行）与 Rust 重写版（~20,600 行）的逐文件对比分析
> 更新日期：2026-05-05

---

## 总览

当前 Rust 重写版已完成 SPEC 全部 100 项任务（核心功能 100% 覆盖）。以下基于 Delphi 原版代码树分析，按优先级排列可继续开发的方向。

| 优先级 | 数量 | 预估总工作量 | 说明 |
|--------|------|-------------|------|
| P0 - 影响使用体验 | 4 项 | ~1 周 | 缺失的翻译 API 提供商 — 全部完成 ✅ |
| P1 - 核心功能补全 | ~~3~~ 0 项 | — | ~~Header Processor 规则模板系统~~ 全部完成 ✅ |
| P2 - 实用工具增强 | 0 项 | ✅ 全部完成 | 拼写检查、工具箱、SST 合并 |
| P3 - UI/UX 完善 | 3 项 | ~1 周 | RTL 预览、HTML 导出、协作系统 |
| P4 - 质量保证 | 4 项 | ~3-4 周 | 跨游戏验证、Delphi 交叉验证 |
| P5 - 低优先级 | 2 项 | ~1 周 | SST 旧版兼容、命令脚本编辑器 |

---

## P0 — 翻译 API 提供商补齐

**当前状态：** 6/8 已实现（OpenAI + DeepL + Baidu + Youdao + Azure + Google）。

| # | 提供商 | Delphi 实现 | 说明 | 状态 |
|---|--------|-----------|------|------|
| P0.1 | **百度翻译** | `TranslatorApi.pas` — AppId+Key→MD5 签名，GET 请求 | 国内用户最常用 | ✅ 已完成 |
| P0.2 | **有道翻译** | `TranslatorApi.pas` — AppKey+Secret→MD5 签名，GET 请求 | 国内常用 | ✅ 已完成 |
| P0.3 | **MS Azure Translator** | `TranslatorApi.pas` — `Ocp-Apim-Subscription-Key` 认证，POST JSON 数组 | 企业级 | ✅ 已完成 |
| P0.4 | **Google Translate** | `TranslatorApi.pas` — 无密钥（旧端点），fake array 模式 | 备用 | ✅ 已完成 |

**实现路径：** 在 `crates/xt-core/src/translation_api/` 下新增对应 `*_provider.rs`，注册到 `build_provider()` 工厂函数。每个 provider 复用现有的 CRLF 保护（`<L_F>` 标签）和 proxy builder。需要更新 `ApiTranslator.txt` 解析逻辑以支持新 provider 的配置格式。

---

## P1 — Header Processor / 规则模板系统 ✅ 已完成

**当前状态：** 全部完成。核心引擎 + 规则编辑器 + 模板管理器 + 批量向导 + 预处理选项均已实现。

### P1.1 规则编辑器（核心）— ✅ 已完成

| 功能 | 说明 | 状态 |
|------|------|------|
| 分层规则树 | 列表 + 勾选 + 展开详细视图 + 搜索/过滤 | ✅ |
| 规则定义 | record_sig、field_sig、关键词列表、tag_id、header 文本、正则、布尔标记 | ✅ |
| 文件 I/O | INI 格式 `[StartRule]...[EndRule]`，与 Delphi 兼容 | ✅ |
| 搜索/过滤 | 按 header 文本、rSig、fSig、EDID、关键词过滤 | ✅ |
| 批量执行 | 对加载的字符串执行已启用规则（含 regex 替换） | ✅ |
| 规则编辑 | 添加/删除/上移/下移/点击内联编辑 | ✅ |

### P1.2 模板管理器 — ✅ 已完成

| 功能 | 说明 | 状态 |
|------|------|------|
| 命名模板 | 保存/加载规则集的启用/禁用状态作为命名 INI 模板 | ✅ |
| 模板管理 | list/save/load/delete templates via TemplateManager | ✅ |

### P1.3 批量向导 — ✅ 已完成

| 功能 | 说明 | 状态 |
|------|------|------|
| 多文件批处理 | 选定源文件夹，按规则批量扫描处理 ESP/ESM | ✅ |
| 进度事件 | `header-batch-progress` / `header-batch-complete` IPC events | ✅ |
| HeaderWizardPanel | 源目录输入、游戏选择、进度条、结果汇总 | ✅ |

### P1.4 预处理选项 — ✅ 已完成

| 功能 | 说明 | 状态 |
|------|------|------|
| 选项存储 | `PreProcessingOpts` key-value INI 格式 | ✅ |
| IPC | load/list/set/delete/save 命令 | ✅ |
| UI 编辑器 | 可折叠 key-value 网格，添加/删除/编辑 | ✅ |

---

## P2 — 编辑/翻译增强工具

### P2.1 拼写检查 — `SpellCheck.pas` (499 行) ✅ 已完成

实施文件：`crates/xt-core/src/spell.rs`（核心）+ `ui/src/components/EditorPanel.tsx`（前端）
IPC 命令：`spell_check_load/unload/toggle/config/text/suggestions/ignore`

| 功能 | 状态 |
|------|------|
| Hunspell DLL 动态加载 | ✅ |
| 标签感知分词（跳过 `<tag>` 内容） | ✅ |
| 首字母大写/多字母大写忽略选项 | ✅ |
| Hash 缓存（正确/错误单词 FNV-1a 哈希） | ✅ |
| Fault-ratio 锁定（>30% 错误率自动禁用） | ✅ |
| 持久化忽略列表（load/save） | ✅ |
| Suggest() 建议系统 | ✅ |
| UI 集成（拼写错误 chip 列表 + 右键建议） | ✅ |

### P2.2 工具箱 — `ToolBox.pas` (7 种工具) ✅ 已完成

实施文件：`crates/xt-core/src/toolbox.rs`（核心）+ `ui/src/components/ToolboxDialog.tsx`（前端）
IPC 命令：`toolbox_transform`。支持按字符串 ID 或全量操作，可选择目标（原文/译文/两者）。
分词器保留 `<tag>` 内容不变，Alias 修复从原文提取标签并替换译文对应标签。

| # | 工具 | 说明 | 状态 |
|---|------|------|------|
| 1 | 全部大写 | UpperCase 所选字符串 | ✅ |
| 2 | 全部小写 | LowerCase 所选字符串 | ✅ |
| 3 | 首单词大写 | 仅首字母大写 | ✅ |
| 4 | 每词首字母大写 | Title Case | ✅ |
| 5 | 修复 Alias | 格式化 `<Alias=...>` 标签 | ✅ |
| 6 | 添加头部 | 字符串前添加指定文本 | ✅ |
| 7 | 修整字符串 | Trim 首尾空白 | ✅ |
| - | 例外词列表 | 编辑 `lWordExceptionList` | 待实现 |

### P2.3 SST 合并 — `MergeSst.pas` ✅ 已完成

实施文件：`crates/xt-core/src/sst/v8.rs`（`SstDictionary::merge_from()`）
IPC 命令：`sst_merge`

| 功能 | 状态 |
|------|------|
| SST 字典合并 | ✅ |
| 三元组匹配 (str_id, record_sig, field_sig) | ✅ |
| 冲突处理（overwrite 参数） | ✅ |
| Master list 合并（去重） | ✅ |
| Colab labels 合并（去重） | ✅ |
| 合并统计（added/updated/overwritten/skipped） | ✅ |

---

## P3 — UI/UX 完善

### P3.1 RTL 实时预览 — `RtlPreview.pas`

**当前状态：** Rust `rtl.rs` 核心算法已实现（阿拉伯整形/去整形、块反转、符号镜像），EditorPanel 有 RTL 按钮。但缺少独立的实时预览工具。

| 功能 | 说明 | 预估 |
|------|------|------|
| 分屏预览 | 左侧源文本，右侧 RTL 渲染结果，实时更新 | 1 天 |
| 行宽调节 | TrackBar 调节换行宽度 | 0.5 天 |
| BiDi 切换 | 左右 memo 独立 RTL/LTR 模式切换 | 0.5 天 |
| 阿拉伯整形 | 复选框控制是否调用 shaping | 0.5 天 |

### P3.2 对话 HTML 导出 — `DialHTML.pas`

| 功能 | 说明 | 预估 |
|------|------|------|
| HTML 渲染 | 对话树数据导出为 HTML 用于审核/打印 | 1 天 |

### P3.3 协作翻译系统 — `Colab.pas` + `ColabFilter.pas`

| 功能 | 说明 | 预估 |
|------|------|------|
| 协作标签分配 | 为字符串分配协作槽位（1..MAXCOLAB_ID），可编辑槽位标签 | 1-2 天 |
| 协作过滤 | 按槽位三态过滤（关闭/包含/排除），颜色编码 | 1-2 天 |

---

## P4 — 验证与质量保证

### P4.1 跨游戏 ESP 解析验证

**当前状态：** 主要用 SkyrimSE 的 `Skyrim.esm`（71,937 条）验证。其他游戏解析精度未知。

| 游戏 | 验证项 | 预估 |
|------|--------|------|
| Skyrim (LE) | 使用 `Skyrim.esm` 验证 | 1-2 天 |
| Fallout 4 | 使用 `Fallout4.esm` 验证 | 1-2 天 |
| Fallout NV | 使用 `FalloutNV.esm` 验证 | 1-2 天 |
| Fallout 76 | 使用 `SeventySix.esm` 验证 | 1-2 天 |
| Starfield | 使用 `Starfield.esm` 验证 | 2-3 天 |

### P4.2 嵌套 GRUP 验证

| 验证项 | 说明 | 预估 |
|--------|------|------|
| CELL/WRLD 子 GRUP | 验证嵌套 GRUP 内字符串提取完整性 | 3-5 天 |

### P4.3 Delphi 交叉验证

| 验证项 | 说明 | 预估 |
|--------|------|------|
| 字符串提取一致性 | 同 ESP 文件提取字符串数量 diff | 2-3 天 |
| SST 读写兼容 | 双向读写验证 | 1-2 天 |
| XML 导入导出一致 | roundtrip 内容验证 | 1-2 天 |

### P4.4 VMAD 写回完善

**当前状态：** `feature_comparison.md` 标记 ~70%，`str_id < 0` 定位 + `edid_hash` 关联已实现。

| 补充项 | 说明 | 预估 |
|--------|------|------|
| buffer 切片重建 | `exportData` 逻辑：将修改后的 UTF-8 字符串按正确偏移写回 VMAD 二进制 | 2-3 天 |
| 片段处理 | PERK/PACK/SCEN/INFO/QUST 片段中嵌套脚本解析 | 1-2 天 |

---

## P5 — 低优先级

### P5.1 SST 旧版本兼容

| 格式 | 说明 | 预估 |
|------|------|------|
| SST v1-v7 | 读取旧版 SST 字典格式（v8 是当前主流） | 3-5 天 |

### P5.2 命令脚本编辑器 — `commandProcessor.pas`

| 功能 | 说明 | 预估 |
|------|------|------|
| 脚本编辑器 | SynEdit 风格编辑，命令模板双击插入，`BatchprocessorPath` 存储 | 2-3 天 |

---

## 实现顺序建议

```
Phase A (Week 1-2): P0 翻译 API + P2.2 工具箱 ✅ 已完成
    └─ 百度翻译 ✅ → 有道翻译 ✅
    └─ 7 种文本转换工具 ✅

Phase B (Week 3-7): P1 Header Processor ✅ 已完成
    └─ P1.1 规则编辑器 ✅ → P1.2 模板管理 ✅ → P1.3 批量向导 ✅ → P1.4 预处理 ✅

Phase C (Week 8-9): P2.1 拼写检查 UI + P2.3 SST 合并 (下一步)
    └─ Hunspell 集成 → 缓存/建议 → UI 高亮

Phase D (Week 10-11): P3 UI 完善
    └─ P3.1 RTL 预览 → P3.2 HTML 导出 → P3.3 协作系统

Phase E (Week 12-15): P4 验证与质量保证
    └─ 跨游戏 ESP 验证 → 嵌套 GRUP → Delphi 交叉验证 → VMAD 完善

Phase F (按需): P5 低优先级
    └─ SST 旧版兼容 → 命令脚本编辑器
```

---

## 参考文件

| Delphi 文件 | 行数 | 功能 | Rust 覆盖 |
|------------|------|------|----------|
| `TESVT_TranslatorApi.pas` | 1,280 | 8 个翻译 API 提供商 | 6/8 (OpenAI, DeepL, Baidu, Youdao, Azure, Google) |
| `TESVT_FormData.pas` | 1,774 | Header Processor 规则编辑器 | ✅ 已完成 (header_processor.rs + HeaderProcessorPanel) |
| `TESVT_FastSearch.pas` | 708 | 30+ 比较器 + 二分搜索 | 部分 (matching.rs 覆盖核心) |
| `TESVT_SpellCheck.pas` | 499 | Hunspell/MS Word 拼写检查 | 后端已完成，缺 UI |
| `TESVT_HeaderWizard.pas` | 493 | 多文件批处理向导 | ✅ 已完成 (header_batch_process + HeaderWizardPanel) |
| `TESVT_VMAD.pas` | 404 | VMAD 脚本属性提取和写回 | ~70% |
| `TESVT_Templates.pas` | 288 | 规则模板管理器 | ✅ 已完成 (TemplateManager) |
| `TESVT_Colab.pas` + `ColabFilter.pas` | 252 | 团队协作翻译 | 未实现 |
| `TESVT_ToolBox.pas` | 59 | 7 种文本转换工具 | ✅ 已完成 (toolbox.rs) |
| `TESVT_commandProcessor.pas` | 110 | 命令脚本编辑器 | 未实现 |
| `TESVT_preProcessingOpts.pas` | 108 | 批次预处理选项 | ✅ 已完成 (PreProcessingOpts) |
| `TESVT_MergeSst.pas` | 96 | SST 字典合并 | 未实现 |
| `TESVT_DialHTML.pas` | 73 | 对话树 HTML 导出 | 未实现 |
| `TESVT_RtlPreview.pas` | 63 | RTL 实时预览工具 | 部分 (核心完成) |
| `TESVT_EspStruct.pas` | 55 | ESP 结构 hex dump | 部分 |
