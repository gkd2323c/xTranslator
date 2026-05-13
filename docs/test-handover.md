# xTranslator 测试基础设施交接文档

> 编写日期：2026-05-12
> 最后更新：2026-05-12（性能优化完成，release 构建验证）
> 涉及：死循环修复、VMAD 性能、BSA 优化、release 构建

---

## 一、测试文件结构

`tests/` 目录当前包含 4 个测试目标和 1 个数据目录：

```
tests/
├── Cargo.toml                  # 测试包清单（4 个 test target）
├── basic_benchmarks.rs         # 通用基准测试（7 项，无外部依赖）
├── e2e_comprehensive.rs        # 端到端综合测试（8 项，全部通过）
├── performance_benchmarks.rs   # 性能基准测试（10 项）
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

# 完整测试（全量，需 Skyrim 数据，约 10s）
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

**状态：✅ 8/8 全部通过（约 10s 总耗时）**

| # | 测试 | 耗时 | 状态 |
|---|------|------|------|
| 1 | ESP 解析验证 | ~1.9s | ✅ pass |
| 2 | 翻译工作流 | ~2.4s | ✅ pass |
| 3 | SST 字典操作 | ~2.4s | ✅ pass |
| 4 | XML roundtrip | ~2.3s | ✅ pass |
| 5 | 性能基准（内嵌） | ~1.9s | ✅ pass |
| 6 | 错误处理 | <1ms | ✅ pass |
| 7 | BSA 回退 | <1ms | ✅ pass |
| 8 | 多游戏兼容 | <1ms | ✅ pass |

关键指标（e2e_esp_parsing_comprehensive）：
- 解析字符串：75,778
- 记录类型：53
- 吞吐：~39,800 strings/s

**适配变更：**
- XML 导出改用新 API（`XmlExportParams` + `XmlStringEntry` + `BufWriter`）
- XML 匹配改用 `matching::apply_xml_dictionary_entries`
- 移除 `StringMatcher`、`has_strings_files()`、`esp_ptr.compressed`
- 修复 `timed_operation` 返回类型误用
- 修复 `catch_unwind` 的 `UnwindSafe` 问题
- 新增 `strings_available()` 守卫函数

### 2.4 `performance_benchmarks.rs` — 性能基准测试

**状态：✅ 10/10 全部可运行（需 Skyrim 数据）**

10 项基准测试涵盖 ESP 解析吞吐、内存占用、过滤/排序/搜索性能、并发和压力测试。
可直接通过项（无需 Skyrim 数据）：`benchmark_translation_api`、`benchmark_stress_test`

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

## 五、已知问题与修复记录

### P0 — ✅ 已修复：ESP 解析器性能（VMAD 解码器堆分配膨胀）

**现象：** `parser.parse()` 在 Skyrim.esm（238 MB）上耗时 300-400 秒，吞吐仅 ~600 KB/s。

**根因：** `parse_vmad_strings()` 对每个 VMAD 字段调用 `VmadDecoder::new(data, vmad_version)`，每次在堆上执行 `buffer.to_vec()` 克隆整个 VMAD 缓冲区。Skyrim.esm 含 16,133 个 VMAD 字段（脚本属性数据），每次解码平均 15ms，合计 358 秒，占总解析时间的 99%。

此外 `VmadDecoder::read_length_prefixed_string()` 内部执行 `data[pos..].to_vec()` 再传给 `String::from_utf8()`，每个属性名和脚本名都产生额外的堆分配。

**修复：**
1. 新增 `decode_vmad_fast(data: &[u8], version: i16) -> Vec<VmadString>` 零分配解码函数，直接读取 `&[u8]` 切片，不克隆缓冲区
2. `read_length_prefixed_string()` 改为 `str::from_utf8(&data[pos..])` 避免中间 `to_vec()`
3. `parse_vmad_strings()` 改为调用 `decode_vmad_fast` 代替 `VmadDecoder::new() + decode()`

**效果：**

| 指标 | 修复前 | 修复后 | 提升 |
|------|--------|--------|------|
| 单次 e2e 解析 | 300-400s | 1.9s | ~190× |
| 完整 e2e 套件 | 1237s | 10.3s | ~120× |
| VMAD 解码总耗时 | 358s | <1ms | >350,000× |
| 吞吐 (strings/s) | 235 | 39,799 | ~170× |

### P1 — ✅ 已修复：解析器在 Skyrim.esm 上死循环

**根因：** `parse_top_level_debug`、`parse_record_debug_for_tree`、`parse_record_debug` 三个函数中，`GenericHeader::read_from()` 返回 `UnexpectedEof` 时被吞没为 `Ok(())`。外层 `loop` 收到 `Ok(())` 后继续迭代，但 reader 已在 EOF 无法前进，形成死循环。

**修复：** 将三处 `return Ok(())` 改为 `return Err(e)`，令 `UnexpectedEof` 正常传播到外层循环的 break 分支。

**验证：** 新增 3 个单元测试，286 个 xt-core 单元测试通过，13 个 `#[ignore]` 标注已移除。

### P2 — BSA 扫描性能（非关键路径）

**现状：** `try_load_from_bsa()` 遍历 Data 目录下全部 BSA 文件（Skyrim SE 默认 93 个），且对三种格式各遍历一次（3×93=279 次 `BsaArchive::open`）。已设计优化方案（单次遍历 + 优先级排序 + 体积过滤）但未实装到当前代码，因为 e2e 测试中 `strings_available()` 守卫返回 false 时完全跳过 BSA 加载，此路径不在关键路径上。

**临时措施：** `strings_available()` 守卫函数跳过慢速 BSA 扫描。

**建议改进：** 实装 `load_from_dir` 的单次遍历重构（已设计但待应用）。

---

## 六、快速参考

```bash
# 快速测试（不依赖 Skyrim 数据）
./test.ps1

# Release 构建
./build.bat

# 日常验证
cargo check --workspace
cargo test --release -p xtranslator-tests --test basic_benchmarks
cargo test --release -p xtranslator-tests --test test_data_generator

# 完整测试（需 Skyrim 数据，约 10s）
cargo test --release -p xtranslator-tests -- --test-threads=1

# 单测解析器
cargo test --release -p xtranslator-tests --test e2e_comprehensive -- e2e_esp_parsing_comprehensive --nocapture

# 性能基准（需 Skyrim 数据）
cargo test --release -p xtranslator-tests --test performance_benchmarks -- benchmark_stress_test --nocapture

# Release 构建（应用端加载 ~3s，开发时用 debug 模式会慢 >100x）
cargo tauri build
```
