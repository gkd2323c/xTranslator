# xTranslator - Rust 重写版

[![License: MPL-2.0](https://img.shields.io/badge/License-MPL--2.0-brightgreen.svg)](LICENSE)

一款现代化的基于 Rust 的 Bethesda 游戏模组翻译工具（Skyrim、Skyrim SE、Fallout 4、Starfield）。这是原 Delphi xTranslator 工具的完整重写版本，采用 Tauri 桌面 UI 和 React 前端。

## 功能特性

### 核心功能
- **ESP/ESM 解析**：加载和解析 Bethesda ESP/ESM 插件文件
- **ESP 写入**：ESP 记录树 + 重建 + 序列化（支持 XXXX 超大字段管理、zlib 重新压缩、写入前自动备份）
- **字符串文件**：支持 `.STRINGS`、`.DLSTRINGS`、`.ILSTRINGS` 格式（带去重，~17% 体积缩减）
- **BSA 归档支持**：从 Bethesda 归档文件提取和加载字符串（.bsa、.ba2）
- **XML 导入/导出**：兼容 Delphi xTranslator XML 格式（UTF-8、实体转义）
- **SST 字典**：完全兼容 Delphi xTranslator 的 v8 双向格式（UTF-16LE、FNV-1a、24B EspPointer）
- **启发式搜索**：使用 Levenshtein 距离、LCS 和 LCP 算法查找相似翻译字符串
- **代码页回退**：UTF-8 为主，失败时回退到 Windows 代码页（932/936/949/950/1250-1257）
- **文本标准化**：字符串标准化（NFKC）和分词，用于启发式搜索和翻译一致性
- **TCSC 转换**：繁体/简体中文转换，使用 OpenCC 字典（3960 对）+ Delphi 回退（2552 对）
- **配置持久化**：JSON 配置文件在重启后保留（主题、语言、API 密钥、代理）
- **API 配置**：解析 Delphi `ApiTranslator.txt` 以获取提供商元数据、语言代码解析和查询模板
- **CRLF 保护**：`<L_F>` 标签保护/恢复周期，用于翻译 API 调用

### 翻译 API
- **OpenAI 兼容**：OpenAI、DeepSeek 和其他聊天补全 API 提供商（支持提示模板）
- **DeepL**：支持免费和专业 API（根据 API 密钥自动检测）
- **Baidu**：百度翻译 API（AppId + Key）
- **Youdao**：有道翻译 API（AppKey + SecretKey）
- **Azure**：Microsoft Translator API（Key 鉴权）
- **Google**：Google Cloud Translation API（Key 鉴权）

### 高级功能
- **GMST:DATA 过滤**：自动检测可翻译 vs 数字 GMST 记录
- **ESP 比较**：轻量级仅比较提取器，具有标准化 FormID + 字段出现匹配
- **记录类型过滤**：按记录类型过滤字符串（INFO、QUST 等）
- **状态过滤**：按翻译状态过滤（已翻译/不完整/锁定）
- **虚拟渲染**：高效处理大型字符串列表（76K+ 项目）
- **分块加载**：批量数据加载（每批 25K 项目，concurrency 3，~2MB JSON）
- **正则搜索/替换**：完整正则表达式，支持捕获组（$1/$2），跨过滤项目全部替换
- **拼写检查**：基于 Hunspell 的拼写检查，支持标签感知断词、建议和忽略列表
- **主题系统**：Obsidian / Slate / Light / Auto 主题，使用 CSS 变量 + localStorage 持久化
- **撤销/重做**：基于栈（最大 100），Ctrl+Z/Y，IPC 同步
- **自动备份**：5 分钟 SST 快照，轮换最后 10 个
- **批量处理器**：多文件顺序 ESP 翻译/导出，具有进度事件和取消功能
- **BSA/BA2 归档浏览器**：浏览、预览和提取 BSA v0x68/v0x69 和 BA2 通用归档中的文件
- **PEX 脚本翻译**：解析 Papyrus 脚本，提取可翻译字符串，并在保留二进制结构的同时写入更新后的字符串表
- **FUZ 音频映射**：将对话字符串映射到 WAV 音频并播放
- **NPC/对话视图**：按 QUST→DIAL→INFO 分组的对话树，与 NPC 关联
- **多语言 UI**：10 种语言（zh-CN、en、de、es、fr、ja、ko、pl、pt、ru）

## 项目结构

```
xTranslator/
├── crates/
│   ├── xt-core/         # 核心库：ESP 解析器 + 记录树 + 写入、字符串、SST、XML、BSA、启发式搜索
│   ├── xt-shared/       # IPC 共享 DTO，后端和前端之间
│   └── xt-cli/          # CLI 工具（遗留，已被 Tauri UI 取代）
├── src-tauri/           # Tauri 2.x 桌面应用后端
├── ui/                  # React + Vite 前端
├── Data/                # 重写版本使用的共享游戏定义
├── docs/                # 文档
└── legacy/original-delphi/ # 保留作为参考的原始 Delphi 项目
```

## 项目状态

**v1.0.0 正式版已发布！** 🎉

重写版本已覆盖主要桌面翻译工作流。`SPEC.md` 跟踪 **100 个已完成任务**，涵盖解析、编辑、ESP 写入、比较工具、归档支持、翻译 API、配置持久化和语言工具。

所有核心功能已实现并通过测试：
- ✅ ESP 解析 + 记录树 + 写入（T42-T45）
- ✅ 字符串读写 + 去重（~17% 体积缩减）
- ✅ SST v8 双向兼容
- ✅ XML 导入/导出（Delphi 兼容）
- ✅ BSA v0x68/v0x69 + BA2 通用归档支持
- ✅ PEX 脚本解析 + 字符串提取 + 写回
- ✅ FUZ 音频映射
- ✅ MCM 翻译文件支持
- ✅ ESP 比较引擎
- ✅ 翻译 API（6 个提供商：OpenAI、DeepL、Baidu、Youdao、Azure、Google）
- ✅ 启发式搜索（Levenshtein + LCS + LCP）
- ✅ 配置持久化（JSON + 代理设置）
- ✅ TCSC 繁简转换（OpenCC + Delphi 回退）
- ✅ 批量处理器 + 取消
- ✅ 自动备份（5 分钟 SST 快照）
- ✅ 撤销/重做（基于栈，最大 100）
- ✅ 虚拟滚动（react-window v2，76K+ 条）
- ✅ 10 语言 i18n UI
- ✅ 主题系统（Obsidian / Slate / Light / Auto）

当前版本已覆盖核心桌面翻译能力。后续工作将聚焦于：跨游戏真实数据验证、Delphi 实机交叉验证、以及 UI 体验持续打磨。

## 文档

从 [`docs/README.md`](docs/README.md) 开始查看组织的文档地图。最常用的项目参考是：

- [`SPEC.md`](SPEC.md) — 规范目标、约束、接口、不变量和任务
- [`ARCHITECTURE.md`](ARCHITECTURE.md) — 实现架构和 IPC/数据流注释
- [`docs/feature_comparison.md`](docs/feature_comparison.md) — Delphi 对等性和剩余差距
- [`docs/release_qa.md`](docs/release_qa.md) — 可重用发布 QA 检查清单

## 构建与测试

### 先决条件
- Rust 1.70+（2021 版）
- Node.js 18+ 和 npm
- Tauri CLI：`cargo install tauri-cli`

### 命令

```bash
# 完整后端构建
cargo build -p xtranslator-tauri

# 核心库测试（无外部依赖）
cargo test -p xt-core --lib

# 运行单个测试
cargo test -p xt-core --lib test_name_here

# E2E 测试（需要安装 Skyrim SE）
cargo test -p xt-core --test e2e_real_data

# TypeScript 检查
cd ui && npx tsc --noEmit

# 前端开发服务器
cd ui && npm run dev

# 完整 Tauri 应用（先运行 Vite 开发服务器）
cargo run -p xtranslator-tauri
```

### 一键开发启动

```powershell
# 从项目根目录 — 自动启动 Vite + Tauri
.\dev.ps1
```

此脚本：
1. 终止任何陈旧的 `node` / `xtranslator-tauri` 进程
2. 在后台作业中启动 Vite 开发服务器（`:5173`）
3. 等待端口 5173 准备就绪（最大 30 秒）
4. 启动 `cargo run -p xtranslator-tauri`
5. 当 Tauri 退出时清理后台作业

## 架构

### 后端-前端 IPC
- **DTO 源头**：`crates/xt-shared/src/dto.rs` 定义 Rust 结构体；`ui/src/api/strings.ts` 在 TypeScript 中镜像它们
- **数据策略**：ESP 加载 → 前端通过 `get_strings_chunk` 分块获取（每批 25K 项目，concurrency 3，~2MB JSON）→ 客户端过滤/排序/滚动
- **按 ID 更新**：`update_translation` 接收 `u32 id` 并在 Vec 中查找字符串。前端使用 `selectedId`（非数组索引）— 过滤/排序后索引无效
- **数据刷新**：SST 加载 / XML 导入 → 后端变异 `AppState.strings` → 前端重新调用 `loadAllStrings()` 以刷新块。单个翻译更新 → 前端本地 `updateItemTranslation(id, text)`（零 IPC）

### 数据格式（Bethesda）
- **字符串文件**：`.STRINGS` = 空终止；`.DLSTRINGS` / `.ILSTRINGS` = 4 字节长度前缀
- **ESP 压缩记录**：`[4 字节解压缩大小 LE] + [zlib 数据]`。解析子记录前解压缩
- **ESP dsize 语义**：记录 `dsize` **排除** 16B 记录头；GRUP `dsize` **包括** 自己的 24B 头（GenericHeader 8B + GrupHeader 16B）
- **代码页回退**：UTF-8 为主；解码失败时，回退到通过 `CodepageTable` 的 Windows 代码页（932/936/949/950/1250-1257）

### 状态值
- `"translated"` — 有非空翻译
- `"incomplete"` — 部分/进行中
- `"locked"` — 不可翻译（例如 GMST 数字 DATA 字段）

### GMST:DATA 过滤
GMST 记录包含一个 `DATA` 字段，可以是：
- **数字**（int/float）— 过滤掉，不可翻译
- **字符串引用**（当 EDID 以 `s` 开头时）— 保留并通过字符串文件解析

过滤逻辑：在 ESP 解析期间，如果 GMST 记录的 `EDID` 字段以 `'s'` 开头，则其 `DATA` 字段被视为字符串 ID 并在 `.STRINGS` 中查找。否则（EDID 以 `f`/`i`/`b` 开头或缺失），DATA 字段假定为数字并跳过。

### 启发式搜索
- 仅搜索已标记为 `translated` 的字符串
- 使用 Levenshtein 距离 + LCS + LCP
- 默认阈值：0.5 相似度，最大 5 个结果
- 后端：`crates/xt-core/src/heuristic/mod.rs`；IPC：`heuristic_search` 命令

### XML 导入/导出
- **导出**：`export_xml` 命令 → `write_xml_export()` → Delphi 兼容 UTF-8 XML，具有实体转义
- **导入**：`import_xml` 命令 → `parse_xml_file()` → `import_xml_to_sky_strings()` — 通过 `(str_id, record_sig, field_sig)` 三元组匹配。返回 `XmlImportResponse { matched, unmatched, total, updated_ids }`

### ESP 写入
- **记录树**：完整的内存解析树（`EspField` → `EspRecord` → `EspGrup` → `EspFile`）在 ESP 解析期间构建
- **写入命令**：
  - `save_esp`：应用翻译 → 重建记录（XXXX 管理、zlib 重新压缩）→ 序列化 → 保存（可选备份）
  - `finalize_esp`：应用 SST 翻译 → 重建 → 序列化 → 导出 .STRINGS/.DLSTRINGS/.ILSTRINGS
  - `delocalize_esp`：将本地化 ESP 转换为去本地化格式（从 1 开始的顺序 ID）
- **备份**：任何 ESP 写入前创建 `.backup.<timestamp>`（可配置）
- **XXXX 处理**：字段大小跨越 65535 边界时自动插入/移除
- **模块**：`crates/xt-core/src/esp/record_tree.rs`, `src/esp/parser.rs`

## 已知限制

- **E2E 测试**：需要 Skyrim SE 安装在 `D:\SteamLibrary\steamapps\common\Skyrim Special Edition\Data\Skyrim.esm`
- **记录定义加载**：尽力而为；如果 `Data/<Game>/record_defs` 缺失，解析器回退到通用解析
- **BA2 归档**：支持通用归档；纹理特定 BA2 变体和归档注入目前有意超出范围

## 致谢

原始 xTranslator 由 McGuffin 和贡献者开发。此 Rust 重写版本保留了原始 Delphi 工具的功能和精神，同时现代化了代码库和 UI。

### 第三方组件（原始）
- SynEdit: https://github.com/SynEdit/SynEdit
- VirtualStringTree: Mike Lischke (www.soft-gems.net)
- Diff: http://www.angusj.com/delphi/textdiff.html (Angus Johnson)
- HtmlViewer: https://github.com/BerndGabriel/HtmlViewer
- ZLibex: http://www.dellapasqua.com and xEdit
- LZ4: https://github.com/atelierw/LZ4Delphi and xEdit
- OmniXML: https://github.com/mremec/omnixml
- PCRE Regex: http://www.regular-expressions.info/delphi.html
- Hunspell: https://github.com/hunspell/hunspell

### 翻译 API 参考
- DeepL: https://www.deepl.com/translator
- OpenAI: https://api.openai.com/

## 许可证

MPL-2.0 许可证。详见 [LICENSE](LICENSE)。
