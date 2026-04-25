# ESP/ESM 文件格式分析

**分析来源**: TESVT_espDefinition.pas 深度精读
**分析日期**: 2024（初版）, 2025-04-23（压缩记录补充）
**Rust 实现**: `crates/xt-core/src/esp/parser.rs`, `crates/xt-core/src/esp/header.rs`

---

## 基本文件结构

ESP/ESM 文件由三种基本元素组成：

```
[TES4 Record]          <- 文件头 (只有一个)
[GRUP]                 <- 组（包含多个 Record）
  [Record]
    [Field]
    [Field]
  [Record]
    [Field]
[GRUP]
  ...
```

---

## 数据结构大小

### rGenericHeader (8 bytes)
```
[name: [u8; 4]]     // 签名，如 "TES4", "GRUP", "INFO"
[dsize: u32 LE]     // 数据大小（不含此 header）
```

### rGenericHeaderData (16 bytes, TES5)
```
[flags: u32 LE]
[formID: u32 LE]
[version: u32 LE]
[fVersion: u16 LE]   // TES5 特有
[vInfo: u16 LE]      // TES5 特有
```

### rGrupheader (16 bytes, TES5)
```
[sIdent: [u8; 4]]    // 组标识
[sType: u32 LE]
[sTStamp: u16 LE]
[param1: u16 LE]
[param2: u16 LE]     // TES5 特有
[param3: u16 LE]     // TES5 特有
```

### hField (6 bytes)
```
[name: [u8; 4]]      // 字段签名
[dsize: u16 LE]      // 数据大小
```

---

## 解析流程

### 1. 读取 TES4 Record
```
- 读取 rGenericHeader (8 bytes)
- 确认 name == "TES4"
- 读取 rGenericHeaderData (16 bytes)
- 读取 header.dsize 字节的数据（字段列表）
```

### 2. 循环读取 GRUP / Record
```
while position < file_size:
  - 读取 rGenericHeader (8 bytes)
  - if name == "GRUP":
    - 读取 rGrupheader (16 bytes)
    - grup_end = position + header.dsize
    - 循环读取组内 Record 直到 position == grup_end
  - else:
    - 读取 rGenericHeaderData (16 bytes)
    - 读取 record 数据（字段列表）
```

### 3. 读取字段列表
```
while position < record_end:
  - 读取 hField (6 bytes)
  - if name == "XXXX":
    - 读取 4 bytes 作为 nextFieldSize
    - 读取下一个字段（使用 nextFieldSize 作为大小）
  - else:
    - 读取 header.dsize 字节数据
```

---

## 压缩记录处理

### 压缩标志

Record 的 `flags` 字段第 18 位（`flags & 0x00040000`）表示该记录是否压缩。

```pascal
function getCompressedFlag(f: cardinal): boolean;
begin
  result := f and $00040000 <> 0;
end;
```

### 压缩数据格式

压缩记录的 `dsize` 字节数据结构如下：

```
偏移 0-3:   decompressedSize (u32 LE)  ← 解压后的数据大小
偏移 4+:    zlib 压缩数据              ← 标准 zlib 格式
```

**关键**: 前 4 字节是解压后大小的前缀，**不是** 12 字节 header，也**不是**裸 zlib 数据。

### 实际数据示例（Skyrim.esm）

| 记录类型 | 数量 | 前几字节 | decompressedSize | zlib 级别 |
|---------|------|---------|-----------------|----------|
| NAVM | 15,966 | `3c 47 00 00 78 da ...` | 0x473c = 18,236 | 0x78DA = Default |
| LAND | 15,563 | `fe 44 00 00 78 da ...` | 0x44FE = 17,662 | 0x78DA = Default |
| CELL | 7,506 | `d1 00 00 00 78 da ...` | 0x00D1 = 209 | 0x78DA = Default |
| NPC_ | 5,118 | `50 01 00 00 78 da ...` | 0x0150 = 336 | 0x78DA = Default |

zlib 压缩级别标识（偏移 4-5，即 zlib header）：
- `0x78 0x01` = Level 1 (Fastest)
- `0x78 0x5E` = Level 2-8 (介于)
- `0x78 0x9C` = Level 6 (Default)
- `0x78 0xDA` = Level 9 (Maximum)

### Delphi 解压逻辑（TESVT_espDefinition.pas:1719-1768）

```pascal
procedure trecord.getrawdata(b: tbytes; currentEspLoader: Pointer; bREFR, bVMAD: boolean);
var
  destBuffer: tbytes;
  decompressedSize: cardinal;
  compressionlvl: word;
begin
  include(params, compressed);
  startpos := 0;

  // 1. 读取前 4 字节 = decompressedSize
  getBufferData(b, @decompressedSize, startpos, sizeOf(cardinal), length(b));
  if (decompressedSize = 0) or (length(b) < 4) then
  begin
    rawrecord := true;   // 解压失败时标记为 raw record
    exit;
  end;

  // 2. 分配解压缓冲区
  setlength(destBuffer, decompressedSize);

  // 3. 从偏移 4 开始解压，大小 = dsize - 4
  //    注意：Delphi 中 header.dsize 包含了 4 字节大小前缀
  try
    DecompressToUserBuf(@b[4], header.dsize - sizeOf(cardinal), @destBuffer[0], decompressedSize);
  except
    rawrecord := true;   // zlib 解压失败时也标记为 raw
    exit;
  end;

  // 4. 解压成功，解析字段
  getFieldfromBuffer(destBuffer, currentEspLoader, bREFR, bVMAD, false);

  // 5. 记录压缩级别
  move(b[4], compressionlvl, 2);
  case compressionlvl of
    $0178: zcomp := zcFastest;   // 0x78 0x01
    $DA78: zcomp := zcMax;       // 0x78 0xDA (字节交换后)
  else    zcomp := zcDefault;
  end;
end;
```

### Rust 实现（crates/xt-core/src/esp/parser.rs）

```rust
/// 格式：[4字节 decompressedSize (u32 LE)] + [zlib 压缩数据]
/// 参考 Delphi: DecompressToUserBuf(@b[4], header.dsize - sizeOf(cardinal), ...)
fn decompress_bethesda_record(data: &[u8]) -> Result<Vec<u8>> {
    // 1. 读取前 4 字节 = decompressedSize
    let decompressed_size = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
    
    // 2. 从偏移 4 开始 zlib 解压
    let compressed = &data[4..];
    let mut decoder = ZlibDecoder::new(compressed);
    let mut decompressed = Vec::with_capacity(decompressed_size);
    decoder.read_to_end(&mut decompressed)?;
    
    Ok(decompressed)
}
```

### 重要注意事项

1. **dsize 含义差异**: 在 Rust 实现中，`Record.dsize` 不包含 `RecordHeaderData`(16字节)，但包含 4 字节的 decompressedSize 前缀。Delphi 中 `header.dsize` 含义相同。
2. **解压失败处理**: Delphi 标记 `rawrecord := true` 并跳过解析。Rust 当前打印警告并跳过。
3. **解压后大小验证**: 可选验证 `decompressed.len() == decompressed_size`，不匹配时打印警告。
4. **最大解压大小**: 建议设置安全阈值（如 100MB），防止恶意文件导致 OOM。

---

## dsize 含义对照表（重要）

这是最容易出错的地方，Rust 实现曾因此 Bug 多次调整：

| 结构 | dsize 含义 | 实际数据大小 | 说明 |
|------|-----------|-------------|------|
| TES4 Record | 字段数据大小 | `dsize` | 不含 RecordHeaderData |
| 普通 Record | 字段数据大小 | `dsize` | 不含 RecordHeaderData；压缩记录包含 4 字节大小前缀 |
| GRUP | 整个 GRUP 块大小 | `dsize - 8 - 16 = dsize - 24` | 含 GenericHeader(8) + GrupHeader(16) |

**读取偏移计算**：

```
Record:  读取 RecordHeaderData(16B) → 读取 dsize 字节数据
GRUP:    读取 GrupHeader(16B) → 读取 (dsize - 24) 字节数据
```

---

## 可翻译字段定义

定义文件: `_recorddefs.txt`

格式: `Def_:FieldName=RecordType=ListIndex[flags]`

| 字段 | 记录 | 类型 | 说明 |
|------|------|------|------|
| FULL | **** | 0 | 名称（所有记录类型） |
| DESC | **** | 1 | 描述（所有记录类型） |
| NAM1 | INFO | 2 | 对话文本 |
| RNAM | INFO | 0 | 对话回复文本 |
| CNAM | QUST | 1 | 任务名称 |
| CNAM | BOOK | 1 | 书籍文本 |
| DNAM | MGEF | 0 | 魔法效果描述 |
| SHRT | NPC_ | 0 | NPC 简称 |
| ITXT | MESG | 0 | 消息文本 |
| DATA | GMST | 0-proc1 | 游戏设置值（需特殊处理） |
| EPFD | PERK | 0-proc2 | 技能效果数据（需特殊处理） |
| EPF2 | PERK | 0-proc4 | 技能效果文本（需特殊处理） |
| TNAM | WOOP | 0 | 词缀名称 |
| NNAM | QUST | 0 | 任务目标名称 |
| BPTN | BPTD | 0 | 身体部位名称 |
| MNAM | FACT | 0 | 阵营名称 |
| FNAM | FACT | 0 | 阵营描述 |
| RDMP | REGN | 0 | 区域名称 |

注意：`****` 是通配符，匹配所有记录类型。`FULL=****` 表示任何记录的 FULL 字段都可翻译。
注意：带 `-proc1`/`-proc2`/`-proc4` 标志的字段需要特殊处理逻辑。

ListIndex:
- 0: strings（普通字符串）→ 对应 `.STRINGS` 文件
- 1: dlStrings（长字符串）→ 对应 `.DLSTRINGS` 文件
- 2: ilStrings（本地化字符串）→ 对应 `.ILSTRINGS` 文件

---

## 字符串字段提取

对于可翻译字段，字段数据 buffer 的内容是：
- 如果记录是 localized（有 STRINGS 文件）：buffer 包含 string ID (uint32 LE)
- 如果记录是 delocalized：buffer 直接包含字符串（UTF-8 或指定代码页）

### Rust 实现的提取流程

```
1. 从 recorddefs.txt 加载可翻译字段定义
2. 解析 ESP 记录，匹配 (record_sig, field_sig, listIndex)
3. 从字段数据中读取 4 字节 string_id (u32 LE)
4. 用 (listIndex, string_id) 查找对应的 Strings 文件
5. 如果找到文本，创建 SkyString 并设置 INCOMPLETE_TRANS 标志
6. 如果未找到，使用 <ID:string_id> 占位符
```

### 解析数据统计（Skyrim.esm，含压缩记录解压）

```
Time: 3.82s
Total strings: 71,937
  INFO:NAM1   34,427  (对话条目)
  DIAL:FULL    5,170  (对话主题)
  ARMO:DESC    2,752  (护甲描述)
  ARMO:FULL    2,623  (护甲名称)
  WEAP:DESC    2,484  (武器描述)
  WEAP:FULL    2,451  (武器名称)
  NPC_:FULL    2,159  (NPC 名称，来自压缩记录)
  INFO:RNAM    1,441  (对话回应文本)
  QUST:FULL    1,286  (任务名称)
  ...
Groups: 118, Records: 819,311
Compressed records: 44,153 (NAVM/LAND/CELL/NPC_ all decompressed)
```

### 压缩记录对字符串提取的影响

| 指标 | 修复前 | 修复后 | 差异 |
|------|--------|--------|------|
| 总字符串 | 68,935 | 71,937 | +3,002 |
| NPC_ 记录 | 0 | 2,419 | +2,419 |
| NPC_:FULL | 0 | 2,159 | +2,159 |
| CELL 记录 | 0 | 583 | +583 |
| 解析时间 | 2.08s | 3.82s | +1.74s (zlib 解压) |
