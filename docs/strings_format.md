# Strings 文件格式分析

**分析来源**: TESVT_StringsFunc.pas + TESVT_fstreamSave.pas 深度精读
**分析日期**: 2025-04-23
**Rust 实现**: `crates/xt-core/src/strings/mod.rs`

---

## 概述

Bethesda 游戏使用独立的 Strings 文件存储本地化文本，ESP/ESM 记录中只保存 4 字节的 `string_id` 引用。三种扩展名对应不同的列表索引：

| 扩展名 | listIndex | 格式 | 用途 |
|--------|-----------|------|------|
| `.STRINGS` | 0 | null 终止字符串 | 普通短字符串（名称、描述等） |
| `.DLSTRINGS` | 1 | 4 字节长度前缀 | 长字符串（书籍全文、任务描述等） |
| `.ILSTRINGS` | 2 | 4 字节长度前缀 | 本地化字符串（对话文本等） |

**关键发现**: `.STRINGS` 和 `.DLSTRINGS`/`.ILSTRINGS` 的字符串数据区格式不同，这是之前 Rust 实现的主要 Bug 之一。

**文件命名规则**: `<plugin_name>_<language>.<extension>`

例如: `skyrim_english.STRINGS`, `skyrim_english.DLSTRINGS`, `skyrim_english.ILSTRINGS`

---

## 文件结构（三种格式共享）

```
┌─────────────────────────────────────┐
│ Header                              │
│   count: u32 LE                     │  ← 条目数量
│   data_size: u32 LE                 │  ← 数据区总字节数
├─────────────────────────────────────┤
│ Directory (count * 8 bytes)         │
│   entry[0]: { id: u32, offset: u32 }│  ← id=字符串ID, offset=数据区内偏移
│   entry[1]: { id: u32, offset: u32 }│
│   ...                               │
│   entry[N-1]: { id: u32, offset: u32}│
├─────────────────────────────────────┤
│ Data Section (data_size bytes)       │  ← 偏移从此区域起始算起
│   .STRINGS:   null 终止字符串       │
│   .DLSTRINGS: [length][data][null]  │
│   .ILSTRINGS: [length][data][null]  │
└─────────────────────────────────────┘
```

### 计算公式

```
data_start = 8 + count * 8
data[i] 位于文件偏移 = data_start + entry[i].offset
```

---

## .STRINGS 格式详解（listIndex=0）

字符串以 null 字节（`0x00`）终止。

```
数据区示例:
offset 0:  'W' 'i' 'n' 'd' 'h' 'e' 'l' 'm' 0x00
offset 9:  'S' 'o' 'l' 'i' 't' 'u' 'd' 'e' 0x00
...
```

### Delphi 读取代码（TESVT_StringsFunc.pas:279-293）

```pascal
// listIndex=0 时: readExtraInt=false
fstream.read(c, SizeOf(c));
while (fstream.Position < fstream.size) and (c <> #0) do
begin
  tmps := tmps + c;
  fstream.read(c, SizeOf(c));
end;
```

逐字节读取直到遇到 `#0`（null 终止符）。

### Rust 实现

```rust
StringsFormat::NullTerminated => {
    let start = offset;
    let mut end = start;
    while end < data.len() && data[end] != 0 {
        end += 1;
    }
    decode_bytes(&data[start..end])
}
```

---

## .DLSTRINGS / .ILSTRINGS 格式详解（listIndex=1,2）

字符串以 4 字节长度前缀开始，后跟 `(length-1)` 字节内容和 1 字节 null 终止符。

```
数据区示例:
offset 0:  [0x1E 0x00 0x00 0x00]  ← length=30 (u32 LE)
offset 4:  'T' 'h' 'i' 's' ' ' 'i' 's' ' ' 'a' ' ' 'l' 'o' 'n' 'g' ' ' 't' 'e' 'x' 't' ... (29 bytes)
offset 33: 0x00                      ← null 终止符
offset 34: [下一个字符串的 length...]
```

**注意**: `length` 包含 null 终止符，实际字符串内容长度 = `length - 1`。

### Delphi 读取代码（TESVT_StringsFunc.pas:279-283）

```pascal
// listIndex>0 时: readExtraInt=true
fstream.read(stringsize, SizeOf(stringsize));     // 4字节长度
SetString(tmps, nil, stringsize - 1);             // 分配 length-1 字节
fstream.read(pointer(tmps)^, stringsize - 1);     // 读取 length-1 字节内容
```

**关键**: `stringsize` 是包含 null 终止符的总长度，实际字符串内容 = `stringsize - 1` 字节。

### Rust 实现

```rust
StringsFormat::LengthPrefixed => {
    let str_len = u32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]) as usize;
    let content_len = str_len.saturating_sub(1);  // length - 1
    let start = offset + 4;
    let end = start + content_len;
    decode_bytes(&data[start..end])
}
```

---

## Codepage 编码系统

### 配置文件格式（Data/<game>/codepage.txt）

```
# 格式: language=primary_codepage[,fallback_codepage]
english=utf8,1252
chinese=utf8
japanese=utf8,932
korean=utf8,949
polish=utf8,1250
russian=utf8,1251
```

**规则**:
- 主编码：读取和保存时使用
- 降级编码：仅当 UTF-8 解码失败时使用（仅读取时）
- `utf8` = codepage 65001

### Delphi Codepage 映射（TESVT_fstreamSave.pas:248-354）

| Codepage | 编码类型 | 读取函数 | 写入函数 |
|----------|---------|---------|---------|
| 65001 (0) | UTF-8 | `rawStringtoStringUTF8` + fallback | `WriteStringUtf8` |
| 1 | UTF-16 (MCM) | fallback to UTF-8 | `WriteStringUtf8` |
| 932 | Shift-JIS | `SetCodePage(rbs, 932)` | `WriteString932` |
| 936 | GBK | `SetCodePage(rbs, 936)` | `WriteString936` |
| 949 | Korean | `SetCodePage(rbs, 949)` | `WriteString950`(⚠️ 注意：949 未列在 codepage 映射中，可能用 950) |
| 950 | Big5 | `SetCodePage(rbs, 950)` | `WriteString950` |
| 1250 | Central European | `SetCodePage(rbs, 1250)` | `WriteString1250` |
| 1251 | Cyrillic | `SetCodePage(rbs, 1251)` | `WriteString1251` |
| 1252 | Western | `SetCodePage(rbs, 1252)` | `WriteString1252` |
| 1253 | Greek | `SetCodePage(rbs, 1253)` | `WriteString1253` |
| 1254 | Turkish | `SetCodePage(rbs, 1254)` | `WriteString1254` |
| 1256 | Arabic | `SetCodePage(rbs, 1256)` | `WriteString1256` |
| 0xFFFF | ForceDeloc | `string(rbs)` (原始字节) | `WriteStringForceDeloc` |

### Delphi 读取流程（TESVT_StringsFunc.pas:228-311）

```
1. parseStrings(filename)
   → getcodepage(filename) 根据 language 确定编码
   → fstream.loadfromfile(filename)

2. parseStringsEx(fstream, ..., codepage)
   → 读取 count + data_size 头部
   → 读取 count 个 (id, offset) 目录条目到 ltmp
   → 记录 dataPos = fstream.Position

3. 对每个条目:
   → fstream.seek(dataPos + pId^.offset)
   → if listIndex > 0: 读取 4 字节 stringsize + (stringsize-1) 字节内容
   → if listIndex == 0: 逐字节读直到 #0
   → rawStringtoString(tmps, codepage, bFallback) 解码

4. rawStringtoString(rbs, codepage):
   → if codepage.isUtf8: rawStringtoStringUTF8(rbs, codepage, bFallback)
   → else: SetCodePage(rbs, codepage.c) → string(rbs)
```

### Delphi 写入流程（TESVT_StringsFunc.pas:326-430）

```
1. saveStringFile(filename, listArray, listIndex)
   → getcodepage(filename) 确定编码
   → 对 listArray[listIndex] 按 hash_trans+trans 排序去重
   → 按 strId 排序

2. 对每个条目:
   → fstream.Position := tmpdatapos (定位到数据区)
   → 检查是否已有相同翻译的条目（去重）
   → 如有: 复用之前的偏移 (stringPos = sxpTmp.pos)
   → 如无: codepage.f(sk.strans, fstream, WriteextraInt) 写入字符串
   → 记录 stringPos
   → 回到目录区写入 (sk.esp.strId, stringPos)

3. 写入头部: count + data_size
4. fstream.SaveToFile(filename + strList[listIndex])
```

**写入时的去重**: 相同翻译文本只存储一次，多个条目共享偏移。

### 写入时的长度前缀处理

```pascal
// WriteextraInt = (listIndex > 0)
function WriteStringUtf8(s: string; fstream: tmemorystream; stringheader: boolean; bzero: boolean = true): integer;
begin
  tmpRaw := utf8encode(s) + #0;  // UTF-8 编码 + null 终止
  sizeRaw := min(MAXSIZESTRING_GLOBALCAP, Length(tmpRaw));
  Result := sizeRaw;
  if stringheader then              // stringheader = WriteextraInt
    fstream.Write(Result, SizeOf(Result));  // 写入 4 字节长度前缀
  fstream.Write(tmpRaw[1], sizeRaw);
end;
```

**关键**: 
- `.STRINGS` (listIndex=0): `WriteextraInt=false`，不写长度前缀
- `.DLSTRINGS/.ILSTRINGS` (listIndex>0): `WriteextraInt=true`，写 4 字节长度前缀
- 所有格式都以 null 终止符结尾

---

## rId 结构（目录条目）

```pascal
rId = record
  id: integer;       // 4 字节 LE, 字符串 ID
  offset: integer;   // 4 字节 LE, 数据区内的字节偏移
end;
```

大小: 8 字节/条目

目录中条目的顺序不一定按 id 排序，但 Delphi 在写入时按 `strId` 排序。

---

## Rust 实现状态

| 功能 | 状态 | 说明 |
|------|------|------|
| .STRINGS 读取 | ✅ | null 终止格式 |
| .DLSTRINGS/.ILSTRINGS 读取 | ✅ | 4 字节长度前缀格式 |
| 格式自动检测 | ✅ | 按文件扩展名检测 |
| UTF-8 解码 | ✅ | primary 编码 |
| Codepage 配置解析 | ✅ | `CodepageTable::parse()` 支持 SkyrimSE/FO4/Starfield 等格式 |
| Windows codepage fallback | ✅ | 932/936/949/950/1250-1257 (encoding_rs) |
| Codepage 自动推断 | ✅ | 从文件名提取语言名，查询 codepage 配置表 |
| Strings 写入 | ✅ | 使用 codepage 编码写入 |
| 写入去重 | ⚠️ | 未实现 (相同翻译共享偏移) |

---

## 参考：Delphi 常量定义（TESVT_Const.pas:177）

```pascal
strList: array [0 .. 2] of String = ('.strings', '.dlstrings', '.ilstrings');
strListLabel: array [0 .. 2] of String = ('STRINGS', 'DLSTRINGS', 'ILSTRINGS');
```

---

## 参考：实际文件数据统计（Skyrim.esm）

| 文件 | 条目数 | 数据大小 | 格式 |
|------|--------|---------|------|
| skyrim_english.STRINGS | 30,301 | 539,369 bytes | null 终止 |
| skyrim_english.DLSTRINGS | 2,686 | 2,239,599 bytes | 长度前缀 |
| skyrim_english.ILSTRINGS | 34,427 | 2,252,761 bytes | 长度前缀 |

总计: 67,414 条字符串条目