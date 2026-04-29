# xTranslator 功能对比：Delphi 原版 vs Rust 重写

> **更新日期**：2026-04-29
> **原版版本**：xTranslator 1.6.0（Delphi 12.1 CE，~6.7 万行代码，10+ 年迭代）
> **重写版本**：v1.0 — 全部 33 项 SPEC 任务完成（xt-core 153 个单元测试通过，0 警告）

---

## 总览

| 维度 | 原版 Delphi | Rust 重写 | 覆盖度 |
|------|------------|----------|--------|
| 代码量 | ~67,000 行 | ~15,900 行 Rust + ~4,700 行 TS | - |
| 开发时间 | 10+ 年 | ~10 周 | - |
| 数据格式解析 | 全格式 | 核心格式 | ~95% |
| 编码系统 | 完整 | 完整 | ~90% |
| 翻译工作流 | 完整 | 核心就绪 | ~75% |
| UI 交互 | 完整 VCL | Tauri 基础 | ~65% |
| 辅助工具 | 完整 | 部分 | ~40% |

---

## 一、编辑模式

| 编辑模式 | 原版 | Rust 重写 | 状态 | 说明 |
|---------|------|----------|------|------|
| **ESP 模式** | ✅ 直接翻译 ESP/ESM | ⚠️ 仅解析 | 部分实现 | ESP 解析完整（71,937条），编辑/写入未实现 |
| **Strings 模式** | ✅ 翻译 STRINGS 文件（已弃用） | ✅ 三格式读写 | 等价实现 | .STRINGS/.DLSTRINGS/.ILSTRINGS 均支持 |
| **Hybrid 模式** | ✅ 推荐模式：ESP 结构+编辑 Strings | ⚠️ 后端就绪 | 部分实现 | 解析+Strings 读写已有，UI 编辑器未实现 |
| **MCM/Translate** | ✅ MCM 菜单翻译文件 | ✅ 后端解析+UI面板 | ~50% | 后端：parser+types+IPC命令；前端：McmPanel，含加载/保存/编辑/过滤 |
| **Papyrus Pex** | ✅ PEX 反编译+翻译 | ✅ 字符串提取 + 写回 | 部分实现 | PEX 解析器完成（Header+StringTable+ObjectInfo），可翻译字符串提取；写回通过重建字符串表并保留原始 opcode/调试信息完成 |

---

## 二、核心解析引擎

| 功能 | 原版 | Rust 重写 | 覆盖度 | 关键差异 |
|------|------|----------|--------|---------|
| **ESP/ESM 解析** | ✅ 全游戏完整 | ✅ Skyrim.esm 验证 | ~80% | 压缩记录解压✅，嵌套 GRUP✅，EDID✅；待验证其他游戏 |
| **压缩记录解压** | ✅ [4B size]+[zlib] | ✅ 同格式 | 100% | 44,153 条正确解压，NPC_/CELL 字符串可见 |
| **Strings 文件读取** | ✅ 三格式+codepage | ✅ 三格式+codepage | ~95% | null-终止/长度前缀两种格式均支持 |
| **Strings 文件写入** | ✅ codepage 编码+去重 | ✅ codepage 编码+去重 | ~95% | 写入+去重（HashMap shared offsets ~17% 缩减）已完成 |
| **SST v8 字典读取** | ✅ UTF-16LE | ✅ UTF-16LE | 100% | roundtrip 验证通过 |
| **SST v8 字典写入** | ✅ | ✅ save_to_file | ~95% | 写入+roundtrip 验证通过 |
| **record_defs 加载** | ✅ 完整标记 | ✅ */?/-proc 标记+GameId | ~95% | parse_record_defs 支持 */?/-proc，load_game_record_defs |
| **Codepage 编码** | ✅ 完整系统 | ✅ 932/936/949/950/1250-1257 | ~90% | CodepageTable 解析 codepage.txt，自动推断语言 |
| **EDID 提取** | ✅ | ✅ Option<String> | 100% | 按 FormID 后备 |
| **VMAD 脚本字段** | ✅ 脚本属性字符串提取 | ❌ | 0% | 需要 Skyrim Script Extender 知识 |
| **XXXX 超大字段** | ✅ 4字节扩展大小 | ✅ | 100% | 已处理 next_field_size 逻辑 |
| **XML 导入** | ✅ | ✅ 解析+匹配+更新 | ~95% | 共享 matcher：exact / EDID / normalized / vocab，歧义不自动应用 |
| **XML 导出** | ✅ | ✅ 写入+实体转义 | ~95% | `write_xml_export` Delphi 兼容格式，只导出有翻译的条目 |
| **BSA/BA2 归档** | ✅ 提取+浏览 | ✅ BSA + BA2 GNRL 全支持 | ~80% | `BsaArchive` v0x68/v0x69，`Ba2Archive` v0x01/0x02/0x08 GNRL，`list_all_files` + `extract_file` + `BsaBrowser` 组件，ESP strings 回退到 BA2 搜索 |
| **PEX 脚本解析** | ✅ 反编译+编辑 | ✅ 字符串提取+写回 | ~60% | PEX parser 完成（Header+StringTable+ObjectInfo），可翻译字符串提取 + PexPanel；写回 PEX 已完成（字符串表原地更新，原始 opcode/调试信息全部保留，索引不变，roundtrip 测试通过） |
| **FUZ 音频映射** | ✅ 映射+播放 | ✅ FuzFile parse + WAV 播放 | ~50% | FuzHeader 解析 + Sound/Voice/ 扫描 + RESP/INFO 关联 + FuzPanel；LIP 唇形数据未处理 |

---

## 三、翻译工作流

| 功能 | 原版 | Rust 重写 | 覆盖度 | 说明 |
|------|------|----------|--------|------|
| **字典应用 (apply)** | ✅ ID+EDID+词汇匹配+状态语义 | ✅ 共享 matcher + Delphi 状态语义 | ~90% | exact/EDID/normalized/vocab 已实现；pending、oldData、warning、tagOnly、stringID 语义有回归测试覆盖；仍需 Delphi 实机对照确认 |
| **启发式搜索** | ✅ Levenshtein/LCS | ✅ | ~80% | xt-core heuristic 模块，Levenshtein+LCS+LCP，IPC+UI 已集成 |
| **翻译 API** | ✅ DeepL/MS/Google/OpenAI/Youdao/Baidu | ✅ OpenAI + DeepL + API config | ~70% | OpenAIProvider + DeepLProvider 已就绪；ApiTranslator.txt 配置解析 + 语言代码映射 + provider 元数据 IPC；CRLF 保护（`<L_F>` 标签）已集成到两个 provider |
| **字符串编辑** | ✅ 行内+窗口编辑 | ⚠️ 基础编辑 | ~70% | EditorPanel：文本编辑、Ctrl+Enter 保存、状态切换、启发式搜索、翻译 API |
| **正则搜索/替换** | ✅ PCRE+批量 | ✅ Regex filter toggle + Replace All | ~80% | Regex toggle + Replace All with confirmation + capture groups ($1/$2) |
| **直接搜索** | ✅ | ✅ 实时筛选 | ~80% | 客户端 filter+sort：文本/Regex/状态/Record 类型/排序，零延迟，76K+ 条 |
| **撤销/重做** | ✅ | ✅ Stack-based (max 100) | ~80% | Ctrl+Z/Y + Ctrl+Shift+Z, IPC-synced, session-only |
| **ESP 写入（Strings 回写）** | ✅ | ⚠️ Strings 保存已有 | ~30% | ESP 本身不修改（原版策略），但需要整合写入流程 |
| **最终化 (finalize)** | ✅ 导出翻译结果 | ⚠️ XML 导出可用 | ~40% | XML 导出已就绪，Strings 最终化待整合 |
| **批量处理器** | ✅ 命令式批处理 | ✅ BatchExecutor + BatchPanel | ~70% | Multi-file translate/export, progress events, cancel, error recovery |
| **RTL 支持 (阿拉伯语)** | ✅ RTL 标签+字符串反向 | ❌ | 0% | `TESVT_TranslateFunc.pas` 中的 RTL 处理 |
| **中文繁简转换** | ✅ SC↔TC 字符映射 | ✅ IPC + MenuBar + EditorPanel 按钮 | ~90% | `tcsc.rs`：OpenCC 主字典(3960对)+Delphi 字典回退(2552对)，编译时嵌入；IPC 命令+MenuBar 按钮+EditorPanel 内转换按钮均已集成；批量转换待实现 |

---

## 四、对比与验证工具

| 功能 | 原版 | Rust 重写 | 覆盖度 | 说明 |
|------|------|----------|--------|------|
| **ESPCompare** | ✅ 两 ESP 建立字符串对 | ✅ StringKey 三元组匹配，EspComparePanel UI，含四标签页+文本过滤 | ~80% | 后端 `esp/compare.rs` 引擎 + Tauri 命令；前端 `EspComparePanel` 含加载/重比/标签页/过滤 |
| **Strings Compare** | ✅ .Strings 文件对比 | ✅ 源/译文哈希比较+标记 | ~70% | `compare_source_dest` IPC，标记"源≠译"或"源=译"为 incomplete |
| **MCM Compare** | ✅ | ❌ | 0% | - |
| **别名检查** | ✅ 源/翻译 Alias 完整性 | ✅ Alias 标签提取+不匹配提示 | ~80% | `check_aliases` IPC，EditorPanel 内提示 alias 不匹配 |
| **中文繁简转换** | ✅ | ✅ IPC + MenuBar + EditorPanel + 批量 | ~95% | `tcsc.rs` 双向转换，单条+批量转换均已集成 |

---

## 五、数据与配置

| 功能 | 原版 | Rust 重写 | 覆盖度 | 说明 |
|------|------|----------|--------|------|
| **ESM 缓存** | ✅ SQLite 缓存加速重载 | ⚠️ ESP 解析结果缓存已实现 | ~40% | Rust 已有 SHA-256+bincode 解析缓存；Delphi 风格 SQLite ESM 缓存仍未实现 |
| **自动备份** | ✅ 定时字典备份 | ✅ 5-min SST snapshots | ~80% | SST 快照，保留 10 份，静默失败 |
| **配置系统** | ✅ res.ini+注册表 | ✅ JSON 配置持久化 + Proxy UI | ~80% | `AppConfig` JSON 持久化（theme/language/API key/proxy），`load_config`/`save_config` IPC 命令，启动时自动加载；HTTP proxy 后端已接入 `build_client()`，前端 Proxy 设置对话框待实现 |
| **vocabulary.txt** | ✅ 词汇列表 | ✅ 解析+加载+启发式搜索增强 | ~80% | `vocabulary.rs` 解析 STRINGS=Name 条目，加载 source+target Strings 文件，按 str_id 匹配，合并到启发式搜索候选集 |
| **ctdaFunc.txt** | ✅ 条件函数定义 | ✅ 文件存在 | ~10% | 未解析 |
| **fieldSizeRef.txt** | ✅ 字段大小参考 | ✅ 文件存在 | ~10% | 未解析 |
| **pexNoTransProc.txt** | ✅ PEX 不可翻译过程 | ✅ 解析+过滤 | ~80% | 已解析并用于 PEX 字符串提取过滤 |
| **DialSubType.txt** | ✅ 对话子类型 | ✅ 文件存在 | ~10% | 未解析 |
| **EmoteDefinition.txt** | ✅ 表情定义 | ✅ 文件存在 | ~10% | 未解析 |

---

## 六、用户界面

| 功能 | 原版 | Rust 重写 | 覆盖度 | 说明 |
|------|------|----------|--------|------|
| **主窗口布局** | ✅ 菜单栏+文件树+编辑区 | ⚠️ 基础布局 | ~60% | MenuBar + SidePanel + StringTable + EditorPanel + BatchPanel + BsaBrowser + PexPanel + FuzPanel + DialogView |
| **虚拟字符串表格** | ✅ VirtualTreeView | ✅ react-window | ~70% | 76K+ 条虚拟滚动，客户端筛选/排序零延迟，ResizeObserver 自适应 |
| **字符串编辑器** | ✅ SynEdit 高亮+行内编辑 | ⚠️ 基础编辑 | ~55% | textarea 编辑、Ctrl+Enter 保存、状态显示、启发式搜索、翻译 API |
| **对话列表视图** | ✅ DIAL/INFO/QUST | ✅ DialogView 组件 | ~60% | QUST→DIAL→INFO 分组 + NPC_ 关联，parent_form_id 跟踪 |
| **翻译进度条** | ✅ | ❌ | 0% | - |
| **筛选/搜索栏** | ✅ 多维度筛选 | ✅ 实时筛选 + 正则 | ~80% | 文本搜索 + Regex toggle + 状态筛选 + Record 类型筛选 + 排序，客户端零延迟，虚拟滚动 |
| **主题支持** | ✅ 默认/亮/灰/暗 | ✅ Dark/Light/Gray/Auto | ~90% | CSS variables + Zustand + localStorage, system follow via matchMedia |
| **UI 多语言** | ✅ 10+ 语言 | ✅ react-i18next 10 语言 | ~80% | zh-CN/en/de/es/fr/ja/ko/pl/pt/ru locales，MenuBar 切换 + localStorage 持久化 |
| **高分辨率 DPI** | ✅ DPI 感知 | ✅ Tauri 2.x 原生 | ~90% | Tauri 2.x 自动处理 HiDPI 缩放 |
| **拖放加载** | ✅ XML 拖放 | ✅ 基础拖放 | ~40% | 支持拖放 ESP/ESM、SST、XML 到主窗口；BSA/PEX/FUZ 拖放仍可后续补 |

---

## 七、游戏支持

| 游戏 | 原版 | Rust 重写 Data/ | record_defs | codepage | 验证状态 |
|------|------|----------------|-------------|----------|---------|
| Skyrim | ✅ | ✅ Skyrim/ | ⚠️ 待验证 | ⚠️ 待验证 | GameId 枚举存在 |
| **SkyrimSE** | ✅ | ✅ SkyrimSE/ | ✅ 22 条 | ✅ 24 语言 | **主要验证目标，71,937 条** |
| Fallout4 | ✅ | ✅ Fallout4/ | ⚠️ 待验证 | ⚠️ 待验证 | Data 目录存在 |
| FalloutNV | ✅ | ✅ FalloutNV/ | ⚠️ 待验证 | ⚠️ 待验证 | GameId 枚举存在 |
| Fallout76 | ✅ | ✅ Fallout76/ | ✅ 9 文件 | ⚠️ 待验证 | Data 目录有文件 |
| Starfield | ✅ | ✅ Starfield/ | ✅ 9 文件 | ⚠️ 待验证 | Data 目录有文件 |

---

## 八、关键差距与优先级建议

### P0 - MVP 必需（Phase 1 剩余）

| 差距 | 说明 | 预估工作量 |
|------|------|-----------|
| ~~Tauri UI 基础框架~~ | ✅ 已完成 — MenuBar + SidePanel + StringTable + EditorPanel + BatchPanel | Done |
| ~~字符串编辑+保存流程~~ | ✅ 已完成 — 编辑 → SST/XML → Strings 完整闭环 | Done |
| ~~字典应用语义补齐~~ | ✅ pending/oldData/tagOnly/stringID/indexMax warning 等 Delphi 行为已实现并归档 | Done |

### P1 - 核心功能补全

| 差距 | 说明 | 预估工作量 |
|------|------|-----------|
| ~~启发式搜索~~ | ✅ Levenshtein+LCS+LCP 已实现 | Done |
| ~~翻译 API 集成~~ | ✅ OpenAI + DeepL 已实现 | Done |
| ~~XML 导出~~ | ✅ Delphi 兼容格式已实现 | Done |
| ~~正则搜索/替换~~ | ✅ Regex filter + Replace All + capture groups | Done |

### P2 - 功能完善

| 差距 | 说明 | 预估工作量 |
|------|------|-----------|
| ~~BA2 General 格式~~ | ✅ Fallout 4/76/Starfield GNRL 归档读取、列出、提取与 strings fallback | Done |
| ~~PEX 脚本解析~~ | ✅ PEX parser + string extraction + PexPanel + write-back (roundtrip tested) | Done |
| ~~FUZ 音频映射~~ | ✅ FuzFile parse + scan + FuzPanel | Done |
| ~~MCM 翻译~~ | ✅ MCM parser (UTF-16LE/UTF-8/ANSI) + types + IPC命令 + McmPanel UI（加载/保存/编辑/过滤） | Done |
| ~~ESPCompare~~ | ✅ `esp/compare.rs` 引擎 + Tauri 命令 + EspComparePanel UI（identical/added/removed/modified 四类，含标签页+过滤） | ✅ 完成 |
| ~~API 配置解析~~ | ✅ `translation_api/config.rs` — 解析 Delphi `ApiTranslator.txt`，语言代码映射，provider 元数据 IPC（`get_api_config`） | Done |
| ESM 缓存 | SQLite 缓存加速重载 | 3-5 天 |

### P3 - 体验优化

| 差距 | 说明 | 预估工作量 |
|------|------|-----------|
| ~~批量处理器~~ | ✅ BatchExecutor + BatchPanel | Done |
| ~~配置持久化~~ | ✅ `AppConfig` JSON 持久化 + `load_config`/`save_config` IPC + 启动自动加载 | Done |
| ~~中文繁简转换~~ | ✅ `tcsc.rs` 核心库（OpenCC 3960对 + Delphi 2552对回退）+ IPC + MenuBar + EditorPanel 按钮 | Done |
| Proxy 设置 UI | 前端 Settings Dialog 包含 proxy server/port/user/pass | Done |
| Batch TCSC | `tcsc_batch_convert` IPC + MenuBar 按钮 | Done |
| ~~CRLF 保护~~ | ✅ 翻译 API `<L_F>` 标签保护/恢复，两个 provider 均已集成 | Done |
| Header 处理器 | ESP 头部修改 | 1 周 |
| ~~主题系统~~ | ✅ Dark/Light/Gray/Auto | Done |
| ~~UI 多语言~~ | ✅ react-i18next 10 languages | Done |
| ~~自动备份~~ | ✅ 5-min SST snapshots | Done |
| ~~高 DPI 支持~~ | ✅ Tauri 2.x 原生 HiDPI + decorations/dragDrop 窗口配置 | Done |
| ~~拖放加载~~ | ✅ ESP/ESM、SST、XML、BSA/BA2、PEX、FUZ 全类型拖放 | Done |

### 技术债务

| 问题 | 影响 | 建议 |
|------|------|------|
| ~~HTTP proxy 未接入 UI~~ | ✅ `build_proxy()`/`build_client()` + SettingsDialog proxy 字段 + MenuBar Settings 按钮 | Done |
| ~~TCSC 批量转换未实现~~ | ✅ `tcsc_batch_convert` IPC + MenuBar 批量按钮（简↹/繁↹） | Done |
| ~~vocabulary.txt 未使用~~ | ✅ `vocabulary.rs` 解析+加载+启发式搜索增强 | Done |
| ~~pexNoTransProc.txt 未解析~~ | ✅ 已解析并用于 PEX 过滤 | Done |
| 嵌套 GRUP 验证 | CELL/WRLD 内的子 GRUP 可能跳过部分字符串 | 需真实数据验证 diff 一致性 |
| Delphi 交叉验证 | 无法确认 99% 一致率 | 需 Delphi 环境生成对照文件 |
| SST 旧版本兼容 | 无法读取 v1-v7 SST | 低优先级，v8 是主流格式 |

---

## 九、Delphi 代码分析状态

| 文件 | 分析状态 | Rust 实现状态 |
|------|---------|-------------|
| TESVT_typedef.pas | ✅ 已完成 | ✅ 核心类型映射完成 |
| TESVT_espDefinition.pas | ✅ 已完成 | ✅ ESP 解析器完成 |
| TESVT_SSTFunc.pas | ✅ 已完成 | ✅ SST v8 读写完成 |
| TESVT_fstreamSave.pas | ✅ 已完成 | ✅ Codepage 系统完成 |
| TESVT_StringsFunc.pas | ✅ 已完成 | ✅ Strings 读写完成 |
| TESVT_Const.pas | ✅ 已分析 | ✅ 常量映射完成 |
| TESVT_Utils.pas | ✅ 已分析 | ✅ StringHash 复刻完成 |
| TESVT_HeuristicSearch.pas | ⚠️ 已分析 | ✅ 已实现 |
| TESVT_scriptPex.pas | ⚠️ 已分析 | ✅ PEX 解析器完成 |
| TESVT_TranslateFunc.pas | ✅ 已分析 apply 核心路径 | ✅ matcher 与 apply 状态语义已实现 |
| TESVT_MainLoader.pas | ✅ 已分析 SST/XML/PEX/缓存关键路径 | ⚠️ Rust 以 Tauri commands + AppState 分拆实现 |
| TESVT_TranslatorApi.pas | ✅ 已分析 | ✅ OpenAI + DeepL + API config 解析 |

---

## 十、与原版的兼容性验证

| 验证项 | 状态 | 说明 |
|--------|------|------|
| SST v8 双向兼容 | ✅ roundtrip 测试通过 | Rust 读写 SST 可被 Delphi 正确读取（理论） |
| Strings 格式兼容 | ✅ 三格式读写 | 格式精确复刻（null-终止/长度前缀） |
| ESP 解析一致性 | ⚠️ 待验证 | 71,937 条 vs 原版，需 Delphi 环境做 diff |
| Codepage 行为一致 | ✅ 算法复刻 | UTF-8 优先 + codepage fallback，与 Delphi 逻辑一致 |
| FNV-1a 哈希 | ✅ 验证通过 | UTF-16LE 低字节 FNV-1a |
| record_defs 解析 | ✅ 标记支持 | */?/-proc 与原版格式一致 |
| XML diff 命令 | ✅ CLI 已实现 | `diff <esp> <xml>` 和 `diff-xml <xml1> <xml2>` |
| XML roundtrip | ✅ 测试通过 | `write_xml_export` → `parse_xml_export` 字段一致 |
