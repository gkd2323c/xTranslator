# Delphi → Rust 不一致修复方案

> 创建: 2026-05-02 | 基于: Delphi 1.6.0 vs Rust 重写交叉分析

---

## 问题全览

### 已确认的不一致（按严重程度排序）

| # | 问题 | 严重程度 | 涉及文件 | 根本原因 |
|:--|:---|:---:|:---|:---|
| 1 | 零 Delphi-vs-Rust 交叉验证 | **严重** | 全局 | 从未在 Delphi 环境下生成参考文件 |
| 2 | 启发式搜索评分系统完全不同 | **高** | `heuristic/mod.rs` vs `TESVT_HeuristicSearch.pas` | Rust 简化为单一 `normalized_similarity`；Delphi 有 6 维复合评分 |
| 3 | CRLF 始终还原为 `\r\n` | **中** | `translation_api/mod.rs:24` | Delphi 还原为 `sstLineBreak`（平台相关），Rust 硬编码 Windows 风格 |
| 4 | 6 个翻译 API 提供商缺失 | **中** | `translation_api/` | 仅实现 OpenAI + DeepL，Delphi 有 8 个 |
| 5 | BSA hash 特殊扩展名未实现 | **低** | `bsa/directory.rs:196` | `.nif`/`.kf`/`.dds`/`.wav` 的位操作被注释掉 |
| 6 | XML EDID 导出 ~5% 差距 | **低** | `xml/mod.rs` | 导出模式判断逻辑可能不完全一致 |
| 7 | 45 个内部诊断标记仅实现子集 | **低** | `types/sky_string.rs` | 多数是 UI 诊断标记，但部分影响匹配逻辑 |
| 8 | 翻译 API 失败重试机制缺失 | **低** | `translation_api/` | Delphi 有 `OnTranslationRetry` 等标志位 |
| 9 | `feature_comparison.md` 标记 Arabic shaping 为"未移植" | **无** | `docs/feature_comparison.md:72` | 实际 `rtl.rs` 已实现 `shape_arabic`/`deshape_arabic` |
| 10 | `toolchain_and_roadmap.md` 标记 SQLite 缓存为"待实现" | **无** | `docs/toolchain_and_roadmap.md:44` | 实际 `sqlite_cache.rs` 已实现 |

---

## 阶段 0: 交叉验证基础设施（P0，1-2 周）

> **目标**: 建立可重复的 Delphi-vs-Rust 自动化对比流程

### T0.1 创建 Delphi 黄金文件

**需要**: Delphi 12.1 CE 环境 + Skyrim.esm

**产出目录**: `tests/fixtures/delphi_golden/`

**需要的文件**:
1. `skyrim_se_export.xml` — Delphi XML Export 功能输出（加载 Skyrim.esm → Export XML）
2. `skyrim_se_export.sst` — Delphi SST Save 输出
3. `skyrim_se_.strings` / `.dlstrings` / `.ilstrings` — Delphi Strings 文件
4. `skyrim_se_heuristic.txt` — 对特定查询的启发式搜索结果(复制粘贴)

**操作步骤 (Delphi 端)**:
```
1. 打开 xTranslator 1.6.0
2. 加载 SkyrimSE\Data\Skyrim.esm
3. File → Export XML → 保存为 delphi_golden/skyrim_se_export.xml
4. 加载 Strings 文件 → 保存为 delphi_golden/skyrim_se_.strings（三格式）
5. File → Save SST → 保存为 delphi_golden/skyrim_se_export.sst
6. 在启发式搜索中搜索 "Retrieve the sword" → 截图并记录结果
```

**说明文件**: `tests/fixtures/delphi_golden/README.md`（包含上述操作步骤）

### T0.2 自动化验证 CLI 子命令

**扩展 `xt-cli`**: 新增 `golden-diff` 子命令

```bash
cargo run -p xt-cli -- golden-diff \
  --rust-xml    rust_output.xml \
  --delphi-xml  delphi_golden/skyrim_se_export.xml \
  --rust-sst    rust_output.sst \
  --delphi-sst  delphi_golden/skyrim_se_export.sst \
  --tolerance   5  # 允许的差异百分比阈值
```

**对比维度**:
- 字符串总数
- str_id 列表交集/差集
- source 文本哈希对比（内容一致性）
- translation 文本哈希对比
- EDID 字段存在性
- REC 属性（id, idMax）一致性

**产出**: `crates/xt-cli/src/golden_diff.rs` + 菜单项

### T0.3 交叉验证报告

**产出模板**: `docs/cross_validation_report.md`

---

## 阶段 1: 启发式搜索参数对齐（P1，2-3 天）

> **目标**: 让 Rust 的启发式搜索产生与 Delphi 等价的结果

### T1.1 移植 Delphi 评分系统

**当前 Rust** (`heuristic/mod.rs`):
```rust
let sim = normalized_similarity(source, s);  // 单一维度: 1 - dist/max_len
```

**Delphi 有多维评分**:

| 评分维度 | Delphi 公式 | 说明 |
|:---|:---|:---|
| 精确哈希匹配 | `score = 0.01 + proxy * 0.05` | 完全相同文本 |
| 同长忽略大小写 | `score = 0.3 + proxy * 0.05` | 仅大小写不同 |
| 单词级 LD=0 | `score = max(score, 0.5)` | 编辑距离为 0 的单词级匹配 |
| LCS 相似度 | `score = sSize * 0.1 + (0.1\|0.55) + proxy * 0.05` | 最长公共子串 |
| proxy 惩罚 | `proxybaseRatio = 0.05` | 每个代理/别名标签的惩罚 |
| 阈值函数 | `ceil(word_count/3) + 1`, cap=25 | 动态阈值 |
| 调整函数 | `if LD <= floor(word_count/15) → 0.55 + LD/10` | 短 LD 放宽 |

**实现方案**: 在 `heuristic/mod.rs` 同级新增 `heuristic/delphi_scoring.rs`:
```rust
pub struct DelphiScoreParams {
    pub proxybase_ratio: f32,     // default 0.05
    pub word_threshold_cap: u32,   // default 25
    pub max_results: usize,        // default 5
}

pub fn delphi_composite_score(
    source: &str, 
    candidate: &str, 
    params: &DelphiScoreParams
) -> f32;
```

### T1.2 对齐默认参数

检查所有调用 `find_similar_translations` 的地方，确保 `min_similarity` 默认值与 Delphi 的 `resultThreshold` 一致。查看 Delphi `TESVT_HeuristicSearch.pas` 中实际调用处的阈值。

### T1.3 回归测试

用 Delphi 黄金文件中记录的启发式搜索结果验证排序一致性。

---

## 阶段 2: CRLF 行为对齐（P1，1 天）

> **目标**: 翻译前后换行符风格保持不变

### T2.1 保留原始换行风格

**当前问题**: `restore_crlf` 始终还原为 `\r\n`
```rust
// 当前: 一律还原为 \r\n
pub fn restore_crlf(text: &str) -> String {
    text.replace(CRLF_TAG, "\r\n")
}
```

**修复方案**: 引入换行风格检测与保留
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrlfStyle { CrLf, Lf, Mixed }

fn detect_crlf_style(text: &str) -> CrlfStyle;

pub fn protect_crlf(text: &str) -> (String, CrlfStyle);

pub fn restore_crlf(text: &str, style: CrlfStyle) -> String;
```

### T2.2 数组翻译模式（记录为未来优化）

Delphi 对 ≤3500 字符的字符串支持数组翻译模式（批量发送，减少 API 调用）。当前 Rust 逐条翻译。此差异不影响兼容性，记录为 T2.2 性能优化（远期）。

### T2.3 重试机制记录

Delphi 有 `OnTranslationRetry` 等标志位支持翻译失败重试一次。当前 Rust 无此逻辑。记录为 T2.3（远期），建议实现指数退避重试。

---

## 阶段 3: 数据格式验证（P2，1 周）

> **目标**: 逐字节验证核心数据格式与 Delphi 一致

### T3.1 BSA Hash 特殊扩展名补全

**文件**: `crates/xt-core/src/bsa/directory.rs:196`

**Delphi 原逻辑** (`TESVT_bsa.pas:248-286`):
```pascal
if ext = '.nif' then i := 1 else
if ext = '.kf'  then i := 2 else
if ext = '.dds' then i := 3 else
if ext = '.wav' then i := 4;
if i <> 0 then begin
  a := byte((i and $FC) shl 5) + byte((result and $FF000000) shr 24);
  b := byte((i and $FE) shl 6) + byte(result and $FF);
  c := byte(i shl 7) + byte((result and $FF00) shr 8);
  result := result - (result and $FF00FFFF);
  result := result + uint32((a shl 24) + b + (c shl 8));
end;
```

**当前 Rust**: 整段被注释为 "暂不实现"。字符串文件查找不受影响（不涉及这些扩展名），但若需要 BSA 归档浏览器正确查找音频/纹理，必须补全。

### T3.2 XML EDID 导出完善

- 验证 `REC` 的 `id`/`idMax` 属性仅在 `IndexMax > 0` 时写入
- 验证纯 Strings 模式下不导出 EDID

### T3.3 Strings 文件写入验证

对照 Delphi golden files 验证 Rust 输出的 `SaveStringsFile` 在去重逻辑、排序、编码上与 Delphi 一致。

### T3.4 ESP 解析一致性

用 Delphi golden files 对比字符串数量、str_id 映射、GMST 过滤。

---

## 阶段 4: 翻译 API 扩展（P3，2-3 周）

### T4.1 Google Translate
### T4.2 Microsoft Translator
### T4.3 其他提供商（按需）

---

## 阶段 5: 内部标记补齐（P4，3-5 天）

### T5.1 影响匹配逻辑的标记 → P0

| 标记 | 影响 | 优先级 |
|:---|:---|:---:|
| `isOneWord` | 单词语匹配策略 | P0 |
| `nTrans` | 歧义匹配标记 | P0 |
| `bHasNumber` | 数字标准化 | P0 |

### T5.2 影响写入安全性的标记 → P1

| 标记 | 影响 |
|:---|:---|
| `stringSizeError` | 字段大小约束检查 |
| `stringCRError` | CR 字符约束检查 |
| `unAuthLineBreak` | 换行符检测 |

### T5.3 补齐到 DTO

---

## 阶段 6: 文档更新（持续）

### T6.1 修复过时文档
### T6.2 新增交叉验证报告
### T6.3 新增缺失功能清单

---

## 执行顺序

```
阶段 0  ──→ 阶段 1  ──→ 阶段 2  ──→ 阶段 3
  │           │           │           │
  │ T0.1 需   │ T1.1 需   │ 可在无     │ 依赖 T0
  │ Delphi    │ 重新设计  │ Delphi 环  │ 黄金文件
  │ 环境      │ 评分结构  │ 境下完成   │
  │           │           │           │
  ▼           ▼           ▼           ▼
 阶段 4  ──→ 阶段 5  ──→ 阶段 6
  │           │           │
  │ 独立于    │ 依赖用户  │ 持续进行
  │ 验证      │ 需求      │
```

**当前状态**: 
- T0.1 需用户协助（Delphi 12.1 CE 环境导出黄金文件）
- T0.2, T0.3, T1, T2, T3, T6 可在 Rust 环境中独立完成
- T4, T5 视用户需求决定

---

## 预估工作量

| 阶段 | 内容 | 预估 | 状态 |
|:---|:---|:--:|:--:|
| 0 | 交叉验证基础 | 1-2 周 | 待开始 |
| 1 | 启发式搜索对齐 | 2-3 天 | 待开始 |
| 2 | CRLF 行为对齐 | 1 天 | 待开始 |
| 3 | 数据格式化验证 | 1 周 | 依赖 T0 |
| 4 | API 提供商扩展 | 2-3 周 | 待开始 |
| 5 | 内部标记补齐 | 3-5 天 | 待开始 |
| 6 | 文档更新 | 持续 | 待开始 |

---

## 变更记录

| 日期 | 变更 |
|:---|:---|
| 2026-05-02 | 初始版本，基于 Delphi 1.6.0 vs Rust 全面交叉分析 |
