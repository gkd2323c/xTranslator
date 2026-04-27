# BSA 归档格式分析

> 来源：T16 BSA 档案浏览器实现中的逆向分析
> 参考：Delphi `TESVT_bsa.pas`、xEdit `wbBSA.pas`

---

## 目录结构

BSA v0x68（Skyrim）和 v0x69（SSE）使用 `BSAhash64` 哈希算法进行文件名查找。

```
BsaArchive
├── header: BsaHeader (version, flags, folder_count, file_count)
├── directory: BsaDirectory
│   ├── folders: Vec<BsaFolder>
│   │   ├── name: String          // 文件夹名（如 "strings"）
│   │   ├── hash: u64             // hash64 值
│   │   └── files: Vec<BsaFileRecord>
│   │       ├── hash: u64         // 文件 hash
│   │       ├── raw_size: u32     // 解压后大小
│   │       ├── offset: u32       // 文件数据偏移
│   │       └── name: String      // 文件名
│   └── folder_map: HashMap<hash, index>
└── path: PathBuf                  // 归档文件路径
```

## 压缩标记

`archive_flags & 0x0004` 仅表示**归档级别**启用了压缩。个别文件可能仍以未压缩形式存储（不常见的流文件场景）。因此 `list_all_files()` 中的 `compressed` 字段是经验判断而非严格确定值——实际压缩状态需在提取时检测。

## 实例隔离

`load_esp` 流程已自动从 BSA 提取 Strings 文件。T16 的 BsaBrowser 使用**独立的 BsaArchive 实例**，避免浏览操作污染当前翻译会话的 AppState 数据。

## 性能

100MB+ BSA 文件加载时间取决于文件数量和目录结构复杂度。`list_all_files()` 的 f64 哈希查找为 O(1)，构建全量文件列表为 O(files)。测试环境 Skyrim - Interface.bsa（8,000+ 文件）加载在 <500ms 内完成。
