# xTranslator v1.1 下一步行动方案

> **日期**：2026-04-29
> **当前版本**：v1.0 完成（41 项 SPEC 任务），v1.1 进行中
> **代码规模**：~16K 行 Rust + ~5K 行 TypeScript

---

## 一、已完成工作回顾

### Phase 1 — 基础补全（已完成）

| 功能 | IPC 命令 | 状态 |
|------|---------|------|
| vocabulary.txt 集成 | `load_vocabulary` | ✅ 解析 STRINGS=Name 条目，加载 source+target Strings，启发式搜索增强 |
| pexNoTransProc.txt 过滤 | 内嵌 `load_no_trans_procs()` | ✅ 过滤不可翻译的 PEX 过程参数 |
| HiDPI 支持 | N/A | ✅ Tauri 2.x 原生 HiDPI + decorations/dragDrop 窗口配置 |
| 拖放扩展 | N/A | ✅ BSA/BA2、PEX、FUZ 文件拖放路由 |

### Phase 2 — 对比与验证（部分完成）

| 功能 | IPC 命令 | 状态 |
|------|---------|------|
| 源/译文哈希比较 | `compare_source_dest` | ✅ diff/same 两种模式，MenuBar ≠/＝ 按钮 |
| Alias 标签完整性检查 | `check_aliases` | ✅ 正则提取 `<Alias=...>` 等标签，EditorPanel 不匹配提示 |
| MCM Compare | — | ❌ 待实现 |
| VMAD 脚本字段提取 | — | ❌ 待实现 |

---

## 二、下一步工作计划

按优先级排列，预估工作量以「天」为单位（1 天 ≈ 4-6 小时专注编码）。

### P2-A：MCM Compare（预估 2-3 天）

**目标**：加载另一份 MCM 翻译文件，按 `(rHash, index)` 匹配条目，将翻译复制到当前数据。

**Delphi 参考**：`tMcMData.doCompareMCM(l, fProc)` — 遍历当前 MCM 条目列表，对每个条目在词汇基列表中按 `(esp.rHash, esp.index)` 查找匹配项，将匹配项的源文本复制为译文。

**实现步骤**：

1. **后端** — `src-tauri/src/commands.rs`
   - 新增 `mcm_compare` 命令：接受 MCM 文件路径 + 覆盖策略选项
   - 解析目标 MCM 文件（复用 `xt_core::mcm::parse_mcm_file`）
   - 遍历 `AppState.strings`，按 `(hash, esp_ptr.index)` 在目标 MCM 条目中查找匹配
   - 匹配成功则复制翻译，设置状态为 `validated` 或 `incomplete_trans`
   - 返回 `{ matched: u32, unmatched: u32, updated_ids: Vec<u32> }`

2. **前端** — `ui/src/components/McmPanel.tsx`
   - 添加"Compare MCM"按钮
   - 使用 `@tauri-apps/plugin-dialog` 打开文件选择器
   - 调用 `mcm_compare` IPC
   - 刷新字符串列表

3. **测试**
   - 单元测试：构造两个 McmEntry 列表，验证匹配逻辑
   - 集成测试：通过 IPC 验证端到端流程

**覆盖策略选项**（对应 Delphi 的 `RadioGroup1`）：

| 值 | 含义 |
|----|------|
| `all` | 覆盖所有匹配项 |
| `no_trans` | 仅覆盖未翻译的项 |
| `no_trans_and_partial` | 覆盖未翻译 + 部分翻译 |
| `partial_only` | 仅覆盖部分翻译 |

---

### P2-B：VMAD 脚本字段提取（预估 5-7 天）

**目标**：从 ESP 记录的 VMAD 子记录中提取脚本属性字符串，使其可翻译。

**Delphi 参考**：`TESVT_VMAD.pas` 中的 `tVMADDecoder` — 解析 VMAD 二进制格式，提取类型为 12（String）和 StringArray 的属性值，创建 `tVMADString` 对象关联到 `tSkyStr`。

**技术要点**：

1. **VMAD 二进制格式**（基于 Delphi 源码逆向）：
   ```
   Header: version(i16) + objType(i16) + scriptCount(i16)
   Scripts: scriptName(len+bytes) + propCount(i16) + properties[]
   Property: name(len+bytes) + type(u8) + status(u8) + value
   Types: 1=Null, 2=Object, 3=String, 4=Int, 5=Float, 6=Bool, 
          7=Variable, 11=Struct, 12=StringArray, 13=IntArray, 
          14=FloatArray, 15=BoolArray, 17=ArrayStruct(FO4)
   Fragments: version 1-5=TES5, 6=FO4
   ```

2. **实现步骤**：
   - 在 `xt-core` 新增 `vmad.rs` 模块
   - `VmadDecoder::new(buffer: &[u8], version: i16)` 构造
   - `VmadDecoder::decode()` → `Vec<VmadString>`（提取所有字符串类型属性）
   - `VmadString { script_name, prop_name, value, offset, length }`
   - 集成到 ESP 解析流程：遇到 VMAD 字段时调用 decoder
   - 将提取的字符串添加到 `AppState.strings` 中，标记 `isVMADString` 内部参数

3. **写回**：修改 VMAD buffer 中对应 offset 的字符串内容，保持二进制结构不变

**风险**：VMAD 格式复杂，Fragment 处理因游戏而异，需充分测试。

---

### P2-C：Data 配置文件解析（预估 1-2 天）

**目标**：解析 `Data/<Game>/` 下的辅助配置文件，用于翻译验证和 UI 增强。

| 文件 | 格式 | 用途 | 前端展示 |
|------|------|------|---------|
| `ctdaFunc.txt` | `ID=FuncName:{Params}` | CTDA 条件函数定义，用于条件表达式可读化 | ESP 记录详情中的 CTDA 解码 |
| `fieldSizeRef.txt` | `REC:FIELD:AuthCR=MaxSize` | 字段最大长度参考 + 是否允许换行 | 编辑器长度校验、换行警告 |
| `DialSubType.txt` | `CODE=SubTypeName` | 对话子类型名称映射 | DialogView 中显示子类型标签 |
| `EmoteDefinition.txt` | `FormID=EmoteName` | 表情定义 FormID→名称映射 | FUZ 面板中显示表情名称 |

**实现步骤**：

1. 在 `xt-core` 新增 `data_config.rs` 模块
2. 各文件解析函数：`parse_ctda_func()`, `parse_field_size_ref()`, `parse_dial_sub_type()`, `parse_emote_definition()`
3. `AppState` 中存储解析结果（启动时加载，按 GameId 分组）
4. 新增 IPC 命令 `get_data_configs` 返回前端需要的配置数据
5. 前端在相应面板中使用

---

### P3-A：翻译进度条（预估 1 天）

**目标**：批量翻译时显示实时进度。

**实现方案**：
- 后端 `translate_string` 命令已支持单条翻译
- 批量翻译通过 `start_batch_translate` 实现，已有进度事件
- 缺失的是 **EditorPanel 单条翻译** 的视觉反馈
- 方案：翻译按钮点击后显示 spinner，翻译完成后自动消失
- 进阶：批量翻译时在 MenuBar 区域显示进度条（复用 BatchPanel 已有逻辑）

---

### P3-B：ESM SQLite 缓存（预估 3-5 天）

**目标**：用 SQLite 缓存加速 ESM 重载，区别于当前的 SHA-256+bincode ESP 解析缓存。

**Delphi 参考**：`loadLocalizedEspCache` — 将解析后的字符串数据存入 SQLite，下次加载同一 ESM 时直接从缓存读取。

**注意**：当前已有 SHA-256+bincode 缓存（`cache.rs`），功能覆盖类似。SQLite 缓存的优势是可增量更新和跨版本兼容。**建议暂缓**，待性能瓶颈确认后再实施。

---

## 三、优先级排序

| 优先级 | 功能 | 预估 | 理由 |
|--------|------|------|------|
| **1** | MCM Compare | 2-3 天 | 用户高频需求，实现简单，Delphi 参考清晰 |
| **2** | Data 配置文件解析 | 1-2 天 | 低风险高收益，增强 UI 可读性 |
| **3** | 翻译进度条 | 1 天 | 体验优化，工作量小 |
| **4** | VMAD 脚本字段提取 | 5-7 天 | 高价值但复杂，需充分测试 |
| **5** | ESM SQLite 缓存 | 3-5 天 | 性能优化，当前缓存已可用 |

**建议执行顺序**：MCM Compare → Data 配置文件 → 翻译进度条 → VMAD → SQLite 缓存

---

## 四、技术债务与质量保障

| 项目 | 当前状态 | 建议行动 |
|------|---------|---------|
| 嵌套 GRUP 验证 | 未用真实数据 diff 一致性 | 用 Delphi 生成对照文件做 diff |
| Delphi 交叉验证 | 无法确认 99% 一致率 | 搭建 Delphi 环境，生成 Skyrim.esm 对照输出 |
| SST v1-v7 兼容 | 仅支持 v8 | 低优先级，v8 是主流格式 |
| 单元测试覆盖 | xt-core 153 个测试 | 新增模块需配套测试 |
| 编译警告 | 0 warnings | 保持 |

---

## 五、里程碑定义

### v1.1 Release Criteria

- [x] Phase 1 全部完成（vocabulary、pexNoTransProc、HiDPI、拖放）
- [x] Strings Compare（源/译文哈希比较）
- [x] Alias Check（标签完整性检查）
- [ ] MCM Compare
- [ ] Data 配置文件解析（至少 ctdaFunc + fieldSizeRef）
- [ ] 翻译进度条
- [ ] 全量编译 0 warnings + TypeScript 0 errors
- [ ] `cargo test -p xt-core --lib` 全部通过

### v1.2 Release Criteria（远期）

- [ ] VMAD 脚本字段提取
- [ ] ESP 直接编辑模式
- [ ] ESM SQLite 缓存
- [ ] 多游戏 record_defs 验证
