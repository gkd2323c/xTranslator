# xTranslator 开发路线图

> 基于 Delphi 1.6.0 原版（~67,000 行）与 Rust 重写版（~20,600 行）的逐文件对比分析
> 更新日期：2026-05-05

---

## 总览

当前 Rust 重写版已完成 SPEC 全部 81 项任务（核心功能 100% 覆盖）。以下基于 Delphi 原版代码树分析，按优先级排列可继续开发的方向。

| 优先级 | 数量 | 预估总工作量 | 说明 |
|--------|------|-------------|------|
| P0 - 影响使用体验 | 4 项 | ~1 周 | 缺失的翻译 API 提供商 |
| P1 - 核心功能补全 | ~~3~~ 1 项 | ~5-6 周 | Header Processor 规则模板系统（核心引擎已完成 ✅） |
| P2 - 实用工具增强 | ~~4~~ 3 项 | ~1.5 周 | ~~拼写检查、工具箱、SST 合并~~ 工具箱已完成 ✅ |
| P3 - UI/UX 完善 | 3 项 | ~1 周 | RTL 预览、HTML 导出、协作系统 |
| P4 - 质量保证 | 4 项 | ~3-4 周 | 跨游戏验证、Delphi 交叉验证 |
| P5 - 低优先级 | 2 项 | ~1 周 | SST 旧版兼容、命令脚本编辑器 |

---

## P0 — 翻译 API 提供商补齐

**当前状态：** 4/8 已实现（OpenAI + DeepL + Baidu + Youdao）。

| # | 提供商 | Delphi 实现 | 说明 | 状态 |
|---|--------|-----------|------|------|
| P0.1 | **百度翻译** | `TranslatorApi.pas` — AppId+Key→MD5 签名，GET 请求 | 国内用户最常用 | ✅ 已完成 |
| P0.2 | **有道翻译** | `TranslatorApi.pas` — AppKey+Secret→MD5 签名，GET 请求 | 国内常用 | ✅ 已完成 |
| P0.3 | **MS Azure Translator** | `TranslatorApi.pas` — `Ocp-Apim-Subscription-Key` 认证，POST JSON 数组 | 企业级 | 2-3 天 |
| P0.4 | **Google Translate** | `TranslatorApi.pas` — 无密钥（旧端点），fake array 模式 | 备用 | 1-2 天 |

**实现路径：** 在 `crates/xt-core/src/translation_api/` 下新增对应 `*_provider.rs`，注册到 `build_provider()` 工厂函数。每个 provider 复用现有的 CRLF 保护（`<L_F>` 标签）和 proxy builder。需要更新 `ApiTranslator.txt` 解析逻辑以支持新 provider 的配置格式。

---

## P1 — Header Processor / 规则模板系统

**当前状态：** Rust `header_processor.rs` 核心引擎已完成（规则匹配、INI 加载/保存、apply 流程）。
`HeaderProcessorPanel` 提供了基础的加载/列表/禁用/应用 UI。
Delphi 原版更复杂的 Editor UI（VirtualStringTree、关键词列表编辑、正则编辑、拖放排序、模板系统、预处理选项）待后续实现。

### P1.1 规则编辑器（核心）— `FormData` 对应

| 功能 | 说明 | 状态 |
|------|------|------|
| 分层规则树 | `VirtualStringTree` 节点，可拖拽排序，复选框启用/禁用 | 基础实现：列表 + 勾选 |
| 规则定义 | record_sig、field_sig、关键词列表、tag_id、header 文本、正则、布尔标记 | ✅ 核心完成 |
| 文件 I/O | INI 格式 `[StartRule]...[EndRule]`，与 Delphi 兼容 | ✅ 完成（`from_ini_text`/`to_ini_text`） |
| 搜索/过滤 | 按 header 文本、记录名、关键词过滤规则 | 待实现 |
| 批量执行 | 对加载的字符串执行已启用规则 | ✅ 完成（`header_rules_apply` IPC） |

### P1.2 模板管理器 — `Templates` 对应

| 功能 | 说明 | 状态 |
|------|------|------|
| 命名模板 | 保存/加载规则的启用/禁用/回退状态作为命名模板 | 待实现 |
| 拖放编辑 | 左右列表拖放管理关键词 | 待实现 |

### P1.3 批量向导 — `HeaderWizard` 对应

| 功能 | 说明 | 状态 |
|------|------|------|
| 多文件批处理 | 选定源文件夹和语言，按模板批量处理 ESP | 待实现 |
| BSA 注入器 | `tinjector` 类：bsaName + filesName + 按语言限制 | 待实现 |
| MCM 模式 | MCM 菜单文件翻译切换 | 待实现 |

### P1.4 预处理选项 — `preProcessingOpts` 对应

| 功能 | 说明 | 状态 |
|------|------|------|
| 处理选项 UI | `key=value` 网格编辑器 | 待实现 |

---

## P2 — 编辑/翻译增强工具

### P2.1 拼写检查 — `SpellCheck.pas` (499 行)

**当前状态：** 完全未实现。Rust 生态有 `hunspell-rs` crate 可用。

| 功能 | 说明 | 预估 |
|------|------|------|
| Hunspell 集成 | 使用 `hunspell-rs` + OpenOffice/Mozilla 词典文件 | 1-2 天 |
| 文本分词 | `<tag>` 识别（跳过标签内单词），首字母大写/全大写忽略选项 | 1 天 |
| Fault-ratio 锁定 | 错误率超过阈值自动禁用下划线（避免精灵语等专有名词洪水） | 1 天 |
| 缓存系统 | 正确/错误单词哈希缓存，持久化忽略列表 | 1 天 |
| 建议系统 | `Suggest()` 获取拼写建议列表 | 0.5 天 |
| UI 集成 | EditorPanel 内拼写错误高亮/下划线，右键建议菜单 | 1-2 天 |

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

### P2.3 SST 合并 — `MergeSst.pas`

| 功能 | 说明 | 预估 |
|------|------|------|
| SST 字典合并 | 将多个 SST 字典合并为一个，处理冲突（以较新/用户选择覆盖） | 2-3 天 |

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

Phase B (Week 3-7): P1 Header Processor (下一步)
    └─ P1.1 规则编辑器 → P1.2 模板管理 → P1.3 批量向导 → P1.4 预处理

Phase C (Week 8-9): P2.1 拼写检查 + P2.3 SST 合并
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
| `TESVT_TranslatorApi.pas` | 1,280 | 8 个翻译 API 提供商 | 4/8 (OpenAI, DeepL, Baidu, Youdao) |
| `TESVT_FormData.pas` | 1,774 | Header Processor 规则编辑器 | 核心引擎+基础 UI 已完成 (header_processor.rs) |
| `TESVT_FastSearch.pas` | 708 | 30+ 比较器 + 二分搜索 | 部分 (matching.rs 覆盖核心) |
| `TESVT_SpellCheck.pas` | 499 | Hunspell/MS Word 拼写检查 | 未实现 |
| `TESVT_HeaderWizard.pas` | 493 | 多文件批处理向导 | 未实现 (P1) |
| `TESVT_VMAD.pas` | 404 | VMAD 脚本属性提取和写回 | ~70% |
| `TESVT_Templates.pas` | 288 | 规则模板管理器 | 未实现 (P1) |
| `TESVT_Colab.pas` + `ColabFilter.pas` | 252 | 团队协作翻译 | 未实现 |
| `TESVT_ToolBox.pas` | 59 | 7 种文本转换工具 | ✅ 已完成 (toolbox.rs) |
| `TESVT_commandProcessor.pas` | 110 | 命令脚本编辑器 | 未实现 |
| `TESVT_preProcessingOpts.pas` | 108 | 批次预处理选项 | 未实现 (P1) |
| `TESVT_MergeSst.pas` | 96 | SST 字典合并 | 未实现 |
| `TESVT_DialHTML.pas` | 73 | 对话树 HTML 导出 | 未实现 |
| `TESVT_RtlPreview.pas` | 63 | RTL 实时预览工具 | 部分 (核心完成) |
| `TESVT_EspStruct.pas` | 55 | ESP 结构 hex dump | 部分 |
