## Why

SPEC.md 记录了 4 个影响核心功能的 Bug：启发式搜索依赖的规范化字段始终为空，EDID 哈希未计算，压缩记录统计缺失。这些 Bug 阻碍了 SST 字典匹配和启发式搜索功能的正常使用。当前工作区已经有部分修复代码，但需要系统化完成并验证。

## What Changes

- 修复 `EspParser.compressed_records` 计数器始终为 0 的问题
- 修复 `EspPointer.edid_hash` 始终为 0 的问题（在 ESP 解析时计算 Editor ID 的 FNV-1a 哈希）
- 新增 `normalization` 模块，实现 Unicode 大小写折叠 + 标点规范化
- 修复 `SkyString.source_normalized` 和 `normalized_hash` 始终为 None 的问题
- 修复 `SkyString.word_hashes` 始终为空的问题（创建时自动分词计算哈希）
- **BREAKING**: `SkyString::new()` 新增 `record_sig` 和 `field_sig` 参数
- 新增 `EspPointer::null()` 便捷构造函数

## Capabilities

### New Capabilities

- `string-normalization`: 源字符串规范化功能，用于启发式搜索和模糊匹配

### Modified Capabilities

- `esp-parsing`: 新增压缩记录统计和 EDID 哈希计算
- `sky-string-core`: SkyString 构造时自动计算规范化字段和分词哈希

## Impact

- `crates/xt-core/src/normalization.rs` (新增模块)
- `crates/xt-core/src/types/sky_string.rs`
- `crates/xt-core/src/types/esp_pointer.rs`
- `crates/xt-core/src/esp/parser.rs`
- `crates/xt-core/src/sst/v8.rs` (SST 读取时需要更新构造调用)
- `crates/xt-core/src/xml/mod.rs` (XML 导入时需要更新构造调用)
- `crates/xt-core/src/testing/generator.rs` (测试生成器需要更新构造调用)
- `src-tauri/src/commands.rs`
