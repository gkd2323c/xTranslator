# 编译工具链 & v2 路线图

---

## 依赖架构

```
xt-core (lib)
├── byteorder       # LE 字节序读写
├── flate2          # zlib 解压（ESP 压缩记录）
├── lz4             # LZ4 解压（SSE BSA v0x69）
├── reqwest         # HTTP 客户端（翻译 API，支持 proxy）
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

| 项目 | 说明 | 状态 |
|------|------|------|
| ~~BA2 General 格式~~ | ✅ Fallout 4/76/Starfield GNRL 类型读取、列出、提取与 strings fallback | ✅ 完成 |
| ~~PEX 写回~~ | ✅ 字符串表原地更新（索引不变），原始 opcode/调试信息全部保留，roundtrip 测试通过 | ✅ 完成 |
| 完整反编译 | Papyrus 指令集全量反编译为可读伪代码 | 待实现 |
| ESP 模式编辑 | 直接编辑 ESP 文件中的字符串（当前策略：修改 Strings 文件） | 待实现 |
| Delphi 风格 ESM 缓存 | SQLite 缓存加速重载（区别于当前 SHA-256+bincode ESP 解析缓存） | 待实现 |
| TCSC IPC+UI | 核心库完成，需添加 `tcsc_convert` IPC 命令 + MenuBar 按钮 | 待实现 |
| HTTP proxy 接入 | `build_proxy()`/`build_client()` 已定义但 provider 使用 `Client::new()`，需替换 + 添加 proxy 设置 UI | 待实现 |
| ~~MCM 翻译~~ | ✅ MCM parser (UTF-16LE/UTF-8/ANSI) + types + IPC命令 + McmPanel UI（加载/保存/编辑/过滤） | ✅ 完成 |
| ~~ESPCompare~~ | ✅ 两个 ESP 文件对比建字符串对（identical/added/removed/modified 四类） | ✅ 完成 |
| ~~TCSC 繁简转换~~ | ✅ OpenCC 主字典(3960对)+Delphi 字典回退(2552对)，编译时嵌入 | ✅ 完成（核心库，IPC+UI 待集成） |
| ~~API 配置解析~~ | ✅ 解析 Delphi `ApiTranslator.txt`，语言代码映射，provider 元数据 IPC | ✅ 完成 |
| ~~配置持久化~~ | ✅ `AppConfig` JSON 持久化（theme/language/API key/proxy），启动自动加载 | ✅ 完成 |
| ~~CRLF 保护~~ | ✅ 翻译 API `<L_F>` 标签保护/恢复，两个 provider 均已集成 | ✅ 完成 |
