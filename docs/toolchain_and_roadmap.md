# 编译工具链 & v2 路线图

---

## 依赖架构

```
xt-core (lib)
├── byteorder       # LE 字节序读写
├── flate2          # zlib 解压（ESP 压缩记录）
├── lz4             # LZ4 解压（SSE BSA v0x69）
├── reqwest         # HTTP 客户端（翻译 API）
├── encoding_rs     # 编码检测与转换
├── quick-xml       # XML 解析/生成
├── sha2/bincode    # ESP 解析结果缓存
└── async-trait     # 异步 trait 支持

xtranslator-tauri (bin)
├── tauri 2.x       # 桌面框架
├── tokio           # 异步运行时
└── serde/serde_json # 序列化
```

## 警告清零

项目已完成全量编译警告清零（0 warnings），修复内容：

| 类别 | 数量 | 示例 |
|------|------|------|
| unused imports | 6 | SeekFrom, BsaFileRecord, Read, Seek, anyhow, Serialize, async_trait |
| dead_code 方法 | 3 | parse_top_level, parse_record, parse_record_fields (suppress) |
| unused variables | 2 | folder → _folder, updated_ids → _updated_ids |
| 字段未读 | 1 | BatchJobState Running 字段 (suppress) |
| Cyrillic 混淆 | 1 | `param_сount` 中的 Cyrillic 'с' → ASCII 'c' |

## v2 展望

| 项目 | 说明 |
|------|------|
| BA2 General 格式 | Fallout 4/76/Starfield 的通用 BA2 读取、列出、提取与 strings fallback |
| PEX 写回 | `compile_pex_strings` — 重建 StringTable，写回二进制 PEX |
| 完整反编译 | Papyrus 指令集全量反编译为可读伪代码 |
| ESP 模式编辑 | 直接编辑 ESP 文件中的字符串（当前策略：修改 Strings 文件） |
| Delphi 风格 ESM 缓存 | SQLite 缓存加速重载（区别于当前 SHA-256+bincode ESP 解析缓存） |
| MCM 翻译 | 自定义 txt 格式的 MCM 菜单翻译文件导入 |
| ESPCompare | 两个 ESP 文件对比建字符串对 |
