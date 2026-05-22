# xTranslator 开发路线图

> 基于 Delphi 1.6.0 原版（~67,000 行）与 Rust 重写版（~20,600 行）的逐文件对比分析
> 更新日期：2026-05（第 5 版：VMAD 片段处理 + 启发式搜索增强）

---

## 总览

当前 Rust 重写版已完成 SPEC 全部 100 项任务（核心功能 100% 覆盖）。以下基于 Delphi 原版代码树分析，按优先级排列可继续开发的方向。

| 优先级 | 数量 | 预估总工作量 | 说明 |
|--------|------|-------------|------|
| P0 - 影响使用体验 | 4 项 | ~1 周 | 缺失的翻译 API 提供商 — 全部完成 ✅ |
| P1 - 核心功能补全 | ~~3~~ 0 项 | — | ~~Header Processor 规则模板系统~~ 全部完成 ✅ |
| P2 - 实用工具增强 | 0 项 | ✅ 全部完成 | 拼写检查、工具箱、SST 合并 |
| P3 - UI/UX 完善 | 0 项 | ✅ 全部完成 | RTL 预览、对话 HTML 导出、协作标签 |
| P4 - 质量保证 | 2 项待完成 | ~3-4 周 | 跨游戏验证、Delphi 交叉验证（嵌套 GRUP / VMAD 写回已完成） |
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
| 配置持久化（字典选择/active状态自动恢复） | ✅ |

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

实施文件：`crates/xt-core/src/sst/v8.rs`（`SstDictionary::merge_from()`）+ `ui/src/components/MergeSstDialog.tsx` + `ui/src/components/MenuBar.tsx`
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

### P3.1 RTL 实时预览 — `RtlPreview.pas` ✅ 已完成

实施文件：`ui/src/components/RTLPreview.tsx`（前端）+ `commands.rs`（`rtl_preview` 命令）
RTL 核心算法已在 `crates/xt-core/src/rtl.rs` 实现。

| 功能 | 状态 |
|------|------|
| 文本输入 + 实时预览（防抖 300ms） | ✅ |
| RTL 反向开关 | ✅ |
| 阿拉伯整形开关 | ✅ |
| 行宽调节（数字输入） | ✅ |
| RTL 方向渲染（direction: rtl） | ✅ |
| 符号镜像（自动跟随 reverse） | ✅ |

### P3.2 对话 HTML 导出 — `DialHTML.pas` ✅ 已完成

实施文件：`crates/xt-core/src/dial_html.rs`
IPC 命令：`export_dial_html`

| 功能 | 状态 |
|------|------|
| 对话树构建（DIAL → INFO 按 parent_form_id 分组） | ✅ |
| HTML 渲染（暗色主题，中英对照） | ✅ |
| 统计信息（主题数、回复数） | ✅ |

### P3.3 协作翻译系统 — `Colab.pas` + `ColabFilter.pas` ✅ 已完成

实施文件：`ui/src/components/ColabPanel.tsx`（前端）+ `commands.rs`（colab_* 命令）
数据模型：`SkyString.colab_id: u8`、`SstDictionary.colab_labels` 已预存在。

| 功能 | 状态 |
|------|------|
| 协作槽位分配（1-8，点击即分配选中字符串） | ✅ |
| 颜色编码（8 色区分槽位） | ✅ |
| 标签列表（colab_get_labels） | ✅ |
| 三态过滤（关闭/包含/排除，colab_filter） | ✅ |
| 槽位验证（colab_set_label） | ✅ |

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

### P4.2 嵌套 GRUP 验证 ✅ 已通过

验证结果（Skyrim.esm）：
- 118 个顶层 GRUP，50,376 个子 GRUP
- CELL: 583 strings ✅
- WRLD: 36 strings ✅
- REFR（仅存在于 CELL 子 GRUP 中）: 405 strings ✅
- 解析器正确处理深度嵌套的 GRUP 结构

### P4.3 Delphi 交叉验证

| 验证项 | 说明 | 状态 |
|--------|------|------|
| 字符串提取一致性 | 同 ESP 文件提取字符串数量 diff | ⏸ 阻塞 — 需 Delphi 1.6.0 运行环境 |
| SST 读写兼容 | 双向读写验证 | ⏸ Rust roundtrip 已验证（含 merge）；Delphi golden SST 已入库，实际交叉读写需 Delphi 环境 |
| XML 导入导出一致 | roundtrip 内容验证 | ✅ Rust XML roundtrip 测试覆盖 |
| **验证流程框架** | L1/L2/L3 三级验证标准化 | ✅ `docs/validation_procedure.md` |

### P4.4 VMAD 写回完善 ✅ 已完成

实施文件：`crates/xt-core/src/vmad.rs`（`write_vmad_string`）+ `commands.rs`（save_esp 管道）

| 补充项 | 说明 | 状态 |
|--------|------|------|
| write_vmad_string 便捷函数 | 封装 write_back，调用方无需管理 VmadDecoder | ✅ |
| 多 VMAD 字符串支持 | vmad_index 按 (form_id, record_sig) 索引 Vec | ✅ |
| save_esp 集成 | VMAD 字段通过 write_vmad_string 精确定位替换 | ✅ |
| 片段处理 | PERK/PACK/SCEN/INFO/QUST 片段中嵌套脚本解析 | 1-2 天 |

---

## P5 — 低优先级

### P5.1 SST 旧版本兼容 ✅ 已完成

| 格式 | 说明 | 状态 |
|------|------|------|
| SST v1-v7 | 读取旧版 SST 字典格式（v8 是当前主流） | ✅ 已完成 (v8.rs + SstVersion) |

### P5.2 命令脚本编辑器 — `commandProcessor.pas` ⏭️ 暂时跳过

| 功能 | 说明 | 预估 |
|------|------|------|
| 脚本编辑器 | SynEdit 风格编辑，命令模板双击插入，`BatchprocessorPath` 存储 | 2-3 天 |

> ⏭️ 暂时跳过：低优先级，与现有 BatchPanel 功能有重叠，可后续按需实现。

---

## 实现顺序建议

```
Phase A (Week 1-2): P0 翻译 API + P2.2 工具箱 ✅ 已完成
    └─ 百度翻译 ✅ → 有道翻译 ✅
    └─ 7 种文本转换工具 ✅

Phase B (Week 3-7): P1 Header Processor ✅ 已完成
    └─ P1.1 规则编辑器 ✅ → P1.2 模板管理 ✅ → P1.3 批量向导 ✅ → P1.4 预处理 ✅

Phase C (Week 8-9): P2.1 拼写检查 UI + P2.3 SST 合并 ✅ 已完成
    └─ ✅ 拼写检查运行时UI已接通，配置持久化已完成（dictionary/loaded/active 自动恢复）
    └─ ✅ SST 合并前后端链路已接通（文件选择、overwrite 策略、统计结果、自动刷新）

Phase D (Week 10-11): P3 UI 完善 ✅ 已完成
    └─ P3.1 RTL 预览 ✅ → P3.2 HTML 导出 ✅ → P3.3 协作系统 ✅

Phase E (Week 12-15): P4 验证与质量保证
    └─ 跨游戏 ESP 验证 → 嵌套 GRUP ✅ → Delphi 交叉验证 → VMAD 完善 ✅
    └─ P4.2 嵌套 GRUP ✅（118 顶层 GRUP, 50,376 子 GRUP）
    └─ P4.4 VMAD 片段处理 ✅（PERK/PACK/SCEN/INFO/QUST 片段跳过 + 写回保留）

Phase F (v1.1.0): P5 功能补全
    └─ 工具箱例外词列表 ✅ → SST 旧版兼容 ✅ → 命令脚本编辑器 ⏭️
```

---

## 参考文件

| Delphi 文件 | 行数 | 功能 | Rust 覆盖 |
|------------|------|------|----------|
| `TESVT_TranslatorApi.pas` | 1,280 | 8 个翻译 API 提供商 | 6/8 (OpenAI, DeepL, Baidu, Youdao, Azure, Google) |
| `TESVT_FormData.pas` | 1,774 | Header Processor 规则编辑器 | ✅ 已完成 (header_processor.rs + HeaderProcessorPanel) |
| `TESVT_FastSearch.pas` | 708 | 30+ 比较器 + 二分搜索 | ✅ Delphi 评分集成 (词级哈希/LCS/LCP/代理惩罚→`delphi_scoring.rs`) |
| `TESVT_SpellCheck.pas` | 499 | Hunspell/MS Word 拼写检查 | ✅ 运行时UI已接通，配置自动恢复 |
| `TESVT_HeaderWizard.pas` | 493 | 多文件批处理向导 | ✅ 已完成 (header_batch_process + HeaderWizardPanel) |
| `TESVT_VMAD.pas` | 404 | VMAD 脚本属性提取和写回 | ✅ 已完成（含 PERK/PACK/SCEN/INFO/QUST 片段处理） |
| `TESVT_Templates.pas` | 288 | 规则模板管理器 | ✅ 已完成 (TemplateManager) |
| `TESVT_Colab.pas` + `ColabFilter.pas` | 252 | 团队协作翻译 | ✅ 已完成 (colab_* + ColabPanel) |
| `TESVT_ToolBox.pas` | 59 | 7 种文本转换工具 | ✅ 已完成 (toolbox.rs) |
| `TESVT_commandProcessor.pas` | 110 | 命令脚本编辑器 | 未实现 |
| `TESVT_preProcessingOpts.pas` | 108 | 批次预处理选项 | ✅ 已完成 (PreProcessingOpts) |
| `TESVT_MergeSst.pas` | 96 | SST 字典合并 | ✅ 已完成 (sst_merge + MergeSstDialog) |
| `TESVT_DialHTML.pas` | 73 | 对话树 HTML 导出 | ✅ 已完成 (export_dial_html) |
| `TESVT_RtlPreview.pas` | 63 | RTL 实时预览工具 | ✅ 已完成 (rtl_preview + RTLPreview) |
| `TESVT_EspStruct.pas` | 55 | ESP 结构 hex dump | 部分 |
