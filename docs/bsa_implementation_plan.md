# BSA 支持实施计划（已完成）

> **状态**：✅ 全部完成，已合并到主分支。
> 
> 实施时间：2026-04-25（1 天内完成）

## 目标

解决 Rust 版 ESP 解析时 Source 文本显示为 `<ID:N>` 的问题，原因是 **Strings 文件打包在 BSA 归档中**，Rust 当前只加载独立文件。

## 现状

- Delphi 原版：`TESVT_bsa.pas` 完整支持 BSA/BA2 读取，Strings 可从 BSA 自动提取
- Rust 当前：`StringsFiles::load_from_dir` 只扫描独立文件，BSA 中的 Strings 被忽略
- 影响：SkyrimSE 的 `Skyrim_english.STRINGS` 等文件实际在 `Skyrim - Misc.bsa` 中，导致大量 `<ID:N>` 占位符

## 实施阶段（全部完成）

### Phase 1：BSA 解析核心 ✅

**新增模块**：`crates/xt-core/src/bsa/`

| 文件 | 职责 |
|------|------|
| `mod.rs` | 公共接口：`BsaArchive::open()`, `extract_file()` |
| `header.rs` | BSA 头部结构、版本检测、ArchiveFlags 解析 |
| `directory.rs` | Folder/File 记录解析、BSAhash64 算法、目录构建 |
| `extraction.rs` | 数据提取：压缩检测、zlib/LZ4 解压、前缀文件名处理 |

**关键实现**：
1. `bsa_hash64(name, ext)` — 100% 复刻 Delphi 算法（中间切片 `&bytes[1..len-2]` 匹配 `copy(s,2,len-3)`）
2. `BsaArchive::open(path)` — 解析目录、构建 HashMap
3. `extract_file(folder/filename.ext)` — 哈希查找 + 解压 + 返回 Vec<u8>

**依赖新增**：`lz4 = "1.24"`（SSE BSA 使用 LZ4 压缩）

### Phase 2：Strings 自动加载集成 ✅

**修改文件**：
- `crates/xt-core/src/esp/parser.rs` — `StringsFiles::load_from_dir` 添加 BSA fallback
- `crates/xt-core/src/strings/mod.rs` — 新增 `StringsFile::load_from_bytes`
- `src-tauri/src/commands.rs` — `load_esp` 命令：stringsDir 加载失败后回退到 ESP 目录扫描 BSA

**集成逻辑**：
```
加载 Strings 流程：
1. 尝试独立文件（strings/ 目录）
2. 如果缺失/0个文件，扫描 ESP 所在目录的 *.bsa
3. 对每个 BSA，尝试提取 strings/{base}_{lang}.STRINGS
4. 成功提取 → 加载到 StringsFiles
5. 优先独立文件（覆盖 BSA 中的同名文件，符合 Bethesda 规则）
```

### Phase 3：端到端验证 ✅

- `cargo test -p xt-core` — 80 个测试全部通过
- E2E 测试：Skyrim.esm 解析后字符串查找返回真实英文（如 "The Ratway Vaults"）
- `<ID:N>` 占位符问题解决

## 时间线（实际）

| 阶段 | 计划 | 实际 | 产出 |
|------|------|------|------|
| Phase 1 | Day 1-2 | 4h | `bsa/` 模块 + 6 个测试通过 |
| Phase 2 | Day 3 | 2h | Strings 自动加载 + Tauri 回退 |
| Phase 3 | Day 3.5 | 1h | diff 验证 + 文档更新 |

## 关键 Bug 修复记录

| 问题 | 原因 | 修复 |
|------|------|------|
| BSAhash64 不匹配 | 中间切片范围错误 `&bytes[1..len-1]` | 改为 `&bytes[1..len-2]` 匹配 Delphi `copy(s,2,len-3)` |
| 溢出 panic | Rust debug 模式不回绕 | 全部改为 `wrapping_add`/`wrapping_shl` |
| 文件夹名带 null | SSE length prefix 含 null terminator | 解析时 trim trailing null |
| 前端传错目录 | `MenuBar.tsx` 传 `Data/Strings` 而非 `Data/` | 后端 `load_esp` 增加 0 文件回退到 ESP 目录 |

## 文档更新（已完成）

- [x] `docs/bsa_format.md` — BSA 格式详细分析
- [x] `docs/feature_comparison.md` — 标记 BSA 为"已实现"
- [x] `ARCHITECTURE.md` — 更新模块列表和数据流
- [x] `PLAN.md` — 新建项目概览文档

---

*最后更新：2026-04-25*
