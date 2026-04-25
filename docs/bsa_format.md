# BSA/BA2 归档格式分析

## 背景

Delphi 原版 xTranslator 的 `TESVT_bsa.pas`（1,478 行）实现了完整的 BSA/BA2 归档读取，用于：
- 从 BSA 中提取 `.STRINGS` / `.DLSTRINGS` / `.ILSTRINGS`
- 浏览归档内容（BSA Browser）
- PEX 脚本和 FUZ 音频的提取

Rust 重写第一阶段只需 **Strings 文件提取**，无需完整的归档浏览器或注入功能。

---

## BSA 格式结构（SkyrimSE 使用版本 0x69）

### 文件头（36 bytes）

```
Offset  Size    Field
0x00    4       Magic: "BSA\0"
0x04    4       Version: u32 (0x67=Oblivion, 0x68=Skyrim, 0x69=SkyrimSE)
0x08    4       HeaderSize: u32 (always 0x24)
0x0C    4       ArchiveFlags: u32
0x10    4       FolderCount: u32
0x14    4       FileCount: u32
0x18    4       TotalFolderNameLength: u32
0x1C    4       TotalFileNameLength: u32
0x20    4       FileFlags: u32
```

### ArchiveFlags 关键位

| Flag      | Value      | 说明 |
|-----------|------------|------|
| COMPRESSFILES | 0x0004 | 归档默认压缩（文件标志反转） |
| PREFIXFULLFILENAMES | 0x0100 | 文件名前缀到数据 |

### File 压缩标志

```
BSAFILE_COMPRESS = 0x40000000
```

**压缩判断逻辑**（Delphi `GetFileCompressedFlag`）：
```pascal
result := (aSize and BSAFILE_COMPRESS) <> 0;
if result then aSize := aSize and not BSAFILE_COMPRESS;
if (bfFlags and BSAARCHIVE_COMPRESSFILES) <> 0 then
  result := not result;  // 全局压缩时反转
```

即：**单个文件标志 XOR 全局标志** = 是否压缩。

---

## 目录结构

### Folder 记录（每个 FolderCount 条目）

```
SSE (v0x69):
  Hash:      u64
  FileCount: u32
  Unk32:     u32
  Offset:    i64  ← 指向文件夹的文件记录位置

Skyrim (v0x68):
  Hash:      u64
  FileCount: u32
  Offset:    u32
```

### File 记录（每个 FileCount 条目）

```
Hash:   u64
Size:   u32  （含 BSAFILE_COMPRESS 标志）
Offset: u32  （指向文件数据位置）
```

### 文件名表

所有文件记录之后，按文件夹分组存储：
```
[Folder1Name]\0 [File1Name]\0 [File2Name]\0 ...
[Folder2Name]\0 [File1Name]\0 ...
```

---

## BSAhash64 算法

Delphi 使用自定义哈希算法定位文件，**必须 100% 复刻**。

```pascal
function BSAhash64(s, ext: string): uInt64;
// s = filename without extension, lowercase
// ext = extension with '.', lowercase
```

### 算法步骤

1. **基础值**：`result = len(s) << 16`
2. **首字符**：`result += byte(s[0]) << 24`
3. **末字符**：`result += byte(s[last])`
4. **倒数第二**：如果长度 > 2，`result += byte(s[last-1]) << 8`
5. **中间部分**：如果长度 > 3，`result += StrToNum(s[1..len-2]) << 32`
   - `StrToNum`：逐字符 `result = result * 0x1003F + byte(c)`
6. **扩展名**：`result += StrToNum(ext) << 32`
7. **特殊扩展名调整**（.nif/.kf/.dds/.wav）：
   - 按扩展名类型调整 result 的低字节

### 查找流程

1. 计算 `folder_hash = BSAhash64(folder_name, "")`
2. 在 Folder 表中二分查找 `folder_hash`
3. 计算 `file_hash = BSAhash64(filename, ext)`
4. 在该 Folder 的 File 表中二分查找 `file_hash`

---

## 数据提取

### 压缩格式

| 版本 | 压缩算法 |
|------|---------|
| Skyrim (0x68) | zlib |
| SkyrimSE (0x69) | **LZ4** |

### SSE 压缩数据布局

```
[4 bytes: decompressed_size]
[LZ4 compressed data]
```

解压使用 `lz4` crate 的 `lz4::block::decompress`。

### 前缀文件名（SSE）

如果 `ArchiveFlags & PREFIXFULLFILENAMES`：
- 数据开头包含完整文件路径（以 null 结尾）
- 实际数据偏移 = 数据开始 + 文件名长度 + 1

---

## 与 Delphi 的差异点

| 项目 | Delphi | Rust 计划 |
|------|--------|-----------|
| 缓存 | `TwbReadOnlyCachedFileStream` 自定义缓存 | 使用标准 `BufReader` + `File` |
| 注入 | 完整支持 BSA/BA2 文件注入 | **第一阶段不实现** |
| BA2 | 完整支持（纹理/普通两种格式） | **第一阶段不实现** |
| 哈希查找 | 预构建 `TStringList` + `Objects` | `HashMap<u64, BsaFolder>` |

---

## 实现范围（第一阶段）

### 支持
- ✅ BSA v0x68 (Skyrim) 和 v0x69 (SkyrimSE)
- ✅ zlib / LZ4 解压
- ✅ 按 `folder/filename.ext` 路径提取文件
- ✅ Strings 文件自动加载集成

### 不支持（后续阶段）
- ❌ BA2 格式（Fallout 4/76/Starfield）
- ❌ BSA 文件注入/修改
- ❌ 归档浏览器 UI
- ❌ 文件列表导出
- ❌ FUZ/PEX 提取

---

## 参考

- Delphi 源码：`TESVT_bsa.pas`（基于 xEdit 的 `wbBSA.pas`）
- xEdit: https://github.com/TES5Edit/TES5Edit
- UESP BSA Format: https://en.uesp.net/wiki/Skyrim_Mod:Archive_File_Format
- BA2 Format: https://en.uesp.net/wiki/Fallout_4_Mod:Archive_File_Format
