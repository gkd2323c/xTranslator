# xTranslator 测试基础设施交接文档

> 编写日期：2026-05-12
> 涉及：测试文件清理、API 适配、问题记录

---

## 一、测试文件结构

`tests/` 目录当前包含 4 个测试目标和 1 个数据目录：

```
tests/
├── Cargo.toml                  # 测试包清单（4 个 test target）
├── basic_benchmarks.rs         # [新增] 通用基准测试（7 项，无外部依赖）
├── e2e_comprehensive.rs        # 端到端综合测试（8 项，5 项已忽略）
├── performance_benchmarks.rs   # 性能基准测试（10 项，8 项已忽略）
├── test_data_generator.rs      # [重构] 合成测试数据生成器
├── fixtures/                   # 测试夹具数据
└── pics/                       # UI 测试截图
```

### 测试运行命令

```bash
# 通用基准测试（无需 Skyrim 数据，推荐日常运行）
cargo test --release -p xtranslator-tests --test basic_benchmarks

# 数据生成器测试（无需 Skyrim 数据）
cargo test --release -p xtranslator-tests --test test_data_generator

# 完整测试（含忽略项列表）
cargo test --release -p xtranslator-tests

# 仅运行非忽略项
cargo test --release -p xtranslator-tests -- --skip ignored
```

---

## 二、各测试文件说明

### 2.1 `basic_benchmarks.rs` — 通用基准测试

**状态：✅ 全部通过（7/7）**

7 项独立于 xt-core 的 Rust 标准库性能基准，不依赖 Skyrim 数据文件：

| # | 测试 | 规模 | 用途 |
|---|------|------|------|
| 1 | 字符串操作 | 100K | filter / sort / case-insensitive search / HashMap insert |
| 2 | 内存分配模式 | 100K | Vec 预分配 / String concat / HashMap 预分配 |
| 3 | 正则操作 | 10K | simple / complex / multi-pattern |
| 4 | 文件 I/O 模拟 | 50K lines | 写 ~2MB 文件 + 读回 |
| 5 | JSON 序列化 | 10K 对象 | 序列化 + 反序列化 roundtrip |
| 6 | 并发操作 | 50K | 4 线程过滤 |
| 7 | 扩展性分析 | 1K/10K/100K | 排序时间随规模增长比率 |

运行方式：`cargo test --release --test basic_benchmarks`

### 2.2 `test_data_generator.rs` — 测试数据生成器

**状态：✅ 全部通过（5/5）**

类型从 `[[bin]]` 改为 `[[test]]`（原没有 `main` 函数）。

提供 `TestDataGenerator` 结构体，可独立生成合成数据用于复现测试：

- `generate_sky_strings(count)` — 生成 N 条模拟字符串
- `generate_sst_dictionary()` — 写入 SST 字典
- `generate_xml_export()` — 写入 XML 导出
- `generate_strings_file()` — 写入 Strings 文件
- `generate_vocabulary_file()` — 写入词汇表

**适配变更：** 全面适配新版 `SkyString`/`EspPointer` API（见第四节）。

### 2.3 `e2e_comprehensive.rs` — 端到端综合测试

**状态：⚠️ 3/8 通过，5 项因解析器死循环已忽略**

| # | 测试 | 状态 | 原因 |
|---|------|------|------|
| 1 | ESP 解析验证 | 🔇 ignored | 解析器在 Skyrim.esm 上死循环 |
| 2 | 翻译工作流 | 🔇 ignored | 同上 |
| 3 | SST 字典操作 | 🔇 ignored | 同上 |
| 4 | XML roundtrip | 🔇 ignored | 同上 |
| 5 | 性能基准（内嵌） | 🔇 ignored | 同上 |
| 6 | 错误处理 | ✅ pass | — |
| 7 | BSA 回退 | ✅ pass | — |
| 8 | 多游戏兼容 | ✅ pass | — |

**适配变更：**
- XML 导出改用新 API（`XmlExportParams` + `XmlStringEntry` + `BufWriter`）
- XML 匹配改用 `matching::apply_xml_dictionary_entries`
- 移除 `StringMatcher`、`has_strings_files()`、`esp_ptr.compressed`
- 修复 `timed_operation` 返回类型误用
- 修复 `catch_unwind` 的 `UnwindSafe` 问题
- 新增 `strings_available()` 守卫函数

### 2.4 `performance_benchmarks.rs` — 性能基准测试（旧版）

**状态：⚠️ 2/10 通过，8 项因解析器死循环已忽略**

仅 `benchmark_translation_api` 和 `benchmark_stress_test` 可通过。

**适配变更：**
- `benchmark_stress_test` 完整重写（`SkyString::new()` 构造器）
- 修复所有字段类型：`[u8; 4]`、`i32`、`u8`、`SkyStringParams`
- 移除 `search_index::SearchIndex`、`StringParams`
- 新增 `maybe_load_strings()` + `strings_available()` 守卫函数

---

## 三、Cargo 配置变更

### `tests/Cargo.toml`

```diff
- [[bin]]
- name = "test_data_generator"
+ [[test]]
+ name = "test_data_generator"

- [[test]] ... simple_performance  (已删除)
- [[test]] ... basic_performance   (已删除)
- [[test]] ... standalone_performance (已删除)
- [[test]] ... simple_bench        (已删除)
- [[test]] ... basic_perf          (已删除)
- [[test]] ... minimal_perf        (已删除)
+ [[test]]
+ name = "basic_benchmarks"
+ path = "basic_benchmarks.rs"

- [dev-dependencies] criterion    (已删除，criterion bench 已移除)
- [[bench]] performance_benchmarks (已删除)
```

删除的 6 个文件是同一轮试错中产生的重复性能测试，已合并到 `basic_benchmarks.rs`。

---

## 四、API 适配对照表

以下是从旧版 API 迁移到新版时需要修改的要点：

### SkyString 字段变更

| 旧字段 | 旧类型 | 新字段/替代 | 新类型 |
|--------|--------|-------------|--------|
| `record_sig` | `String` | `record_sig` | `[u8; 4]` |
| `field_sig` | `String` | `field_sig` | `[u8; 4]` |
| `form_id` | `String` | `esp_ptr.form_id` | `u32` |
| `status` | `String` | `params` + `internal_params` | `SkyStringParams` + `SkyStringInternalParams` |
| `str_id` | `u32` | `esp_ptr.str_id` | `i32` |
| `is_vmad` | `bool` | `internal_params.IS_VMAD_STRING` | `u64` 标志位 |
| `ld` | `int` | 已重命名为 `ld_result` / `ld_found` | `f32` / `i32` |
| `params` | `StringParams` | `params` | `SkyStringParams` |
| `search_index` | `SearchIndex` | `word_hashes` / `source_normalized` | `Vec<u32>` / `Option<String>` |

### EspPointer 字段变更

| 旧字段 | 旧类型 | 新字段/替代 | 新类型 |
|--------|--------|-------------|--------|
| `str_id` | `u32` | `str_id` | `i32` |
| `record_sig` | `String` | `record_sig` | `[u8; 4]` |
| `field_sig` | `String` | `field_sig` | `[u8; 4]` |
| `compressed` | `bool` | 已移除（不再需要） | — |
| — | — | `form_id` | `u32` |
| — | — | `index` | `u16` |
| — | — | `index_max` | `u16` |
| — | — | `edid_hash` | `u32` |

### 其他重要 API 变更

| 旧 API | 新 API |
|--------|--------|
| `StringMatcher::apply_xml_translations()` | `matching::apply_xml_dictionary_entries()` |
| `xml::write_xml_export(&[SkyString], &Path, &str)` | `xml::write_xml_export(&mut W, &XmlExportParams, &[XmlStringEntry])` |
| `parser.has_strings_files()` | 已移除（strings_files 始终加载） |
| `StringParams::new()` | `SkyStringParams::new()` |

---

## 五、已知问题

### P1 — 解析器在 Skyrim.esm 上死循环

**现象：** `parser.parse(&mut file)` 在读取 `Skyrim.esm`（17MB）时陷入无限循环。

**位置：** `crates/xt-core/src/esp/parser.rs` 中的 `parse_top_level_debug()` 调用链。

**推测原因：** 顶层循环 `loop { match self.parse_top_level_debug(...) }` 在遇到某些记录类型时未能推进文件读取指针，导致反复解析同一位置。

**后续排查思路：**
1. 用 `cargo test --release --test performance_benchmarks -- benchmark_esp_parsing --nocapture` 触发，加日志定位卡在哪一步
2. 检查 `GenericHeader::read_from()` 是否在特定数据上返回 `Ok` 但不消耗字节
3. 检查 `parse_record_debug_for_tree()` 的递归路径是否在某些 GRUP 类型上进入死循环

**临时措施：** 相关 13 个测试标记为 `#[ignore = "parser hangs on this Skyrim.esm (infinite loop in parse loop)"]`。

### P2 — BSA 扫描性能

**现象：** `try_load_from_bsa()` 遍历 Data 目录下全部 93 个 BSA 文件（含多 GB 纹理 BSA），在 Skyrim 默认安装下极慢。

**临时措施：** 新增 `strings_available()` 守卫函数，检查 `Skyrim_english.STRINGS` 是否以独立文件存在。不存在时跳过 BSA 扫描，用空 `StringsFiles` 继续测试。

**建议改进：** `try_load_from_bsa()` 应优先扫描名称包含 "Interface" 或 "Misc" 的 BSA，或维护一个已知含 strings 文件的 BSA 优先级列表。

### P3 — 未来展望

1. **解析器修复后**：去掉 13 个 `#[ignore]` 标记，重新验证 E2E 测试
2. **`basic_benchmarks.rs` 可扩展**：当前 7 项基准覆盖标准库操作，后续可增加与 xt-core 集成的基准（如 ESP 解析后过滤/排序性能对比）
3. **测试数据生成器可复用**：`TestDataGenerator` 已适配最新 API，可直接用于编写新的集成测试

---

## 六、快速参考

```bash
# 日常验证
cargo check --workspace
cargo test --release -p xtranslator-tests --test basic_benchmarks
cargo test --release -p xtranslator-tests --test test_data_generator

# 完整测试（忽略已知失败）
cargo test --release -p xtranslator-tests

# 手动执行特定性能测试
cargo test --release -p xtranslator-tests --test performance_benchmarks -- benchmark_stress_test --nocapture
cargo test --release -p xtranslator-tests --test performance_benchmarks -- benchmark_translation_api --nocapture

# 解析 Skyrim.esm（用于调试死循环）
cargo test --release -p xtranslator-tests --test e2e_comprehensive -- e2e_esp_parsing_comprehensive --nocapture
```
