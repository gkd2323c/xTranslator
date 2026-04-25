# SST 二进制格式逆向分析

**分析来源**: TESVT_SSTFunc.pas + TESVT_Const.pas 深度精读  
**分析日期**: 2024  
**版本**: v8 (当前), 兼容 v2-v7

---

## 版本魔数

| 版本 | 魔数 (Hex) | ASCII | 说明 |
|------|-----------|-------|------|
| v1 | $32555353 | "SSU2" | 最早版本，无 header |
| v2 | $33555353 | "SSU3" | + strID/formID |
| v3 | $34555353 | "SSU4" | + index |
| v4 | $35555353 | "SSU5" | + indexMax/rHash + 占位 flag |
| v5 | $36555353 | "SSU6" | + edidHash |
| v6 | $37555353 | "SSU6" | + colabId |
| v7 | $38555353 | "SSU6" | + colab label |
| **v8** | **$39555353** | **"SSU7"** | **+ master list** |

**注意**: Delphi 小端序存储，`$39555353` 文件中的字节顺序是 `53 55 53 39` ("SUS9")。

---

## SST v8 文件格式

### 1. 文件头 (5 bytes)

```
[Magic: u32 LE]       // $39555353
[v4 Flag: u8]         // 占位符，总是 0
```

### 2. Master List (v8+)

```
[Count: i32 LE]
for each master:
  [StrSize: i32 LE]   // UTF-16 字节数 = len * 2
  [String: UTF-16LE bytes]
```

### 3. Colab Label List (v7+)

```
[Count: i32 LE]
for each label:
  [ColabID: i32 LE]   // Object[i] 的值
  [StrSize: i32 LE]   // UTF-16 字节数 = len * 2
  [String: UTF-16LE bytes]
```

### 4. 字符串条目 (循环到 EOF)

```
[ListIndex: u8]       // 0=strings, 1=dlstrings, 2=ilstrings

[EspPointerLite: 24 bytes]
  [strId: i32 LE]
  [formID: u32 LE]
  [rName: [u8; 4]]    // record signature
  [fName: [u8; 4]]    // field signature
  [index: u16 LE]
  [indexMax: u16 LE]
  [rHash: u32 LE]     // edid hash

[colabId: u8]         // v6+
[sparams: u8]         // sStrParams 位掩码 (8 flags)

[SourceSize: i32 LE]  // UTF-16 字节数
[Source: UTF-16LE bytes]

[TransSize: i32 LE]   // UTF-16 字节数
[Translation: UTF-16LE bytes]
```

---

## 版本差异兼容性

### v2 读取差异
- 不读 rName/fName
- 不读 index
- 不读 indexMax/rHash
- 无 colabId

### v3 读取差异
- + index

### v4 读取差异
- + indexMax + rHash
- 跳过 v4 Flag byte
- 清理 deprecatedParam1/deprecatedParam2

### v5 读取差异
- + rName (record signature)

### v6 读取差异
- + colabId

### v7 读取差异
- + colab label list

### v8 读取差异
- + master list (在 colab label 之前)

---

## 关键编码信息

### 字符串编码
- **Delphi**: UnicodeString = UTF-16LE
- `sizeof(char)` = 2 bytes
- `length(s)` = 字符数
- `length(s) * sizeof(char)` = 字节数
- `SetString(s, nil, ssize div sizeof(char))` = 分配 `ssize/2` 个字符，读取 `ssize` 字节填充

### Rust 对应
```rust
// Delphi UnicodeString -> Rust String
fn read_delphi_string<R: Read>(reader: &mut R) -> Result<String> {
    let size = reader.read_i32::<LittleEndian>()?;
    if size <= 0 {
        return Ok(String::new());
    }
    let mut buf = vec![0u8; size as usize];
    reader.read_exact(&mut buf)?;
    // UTF-16LE -> String
    Ok(UTF_16LE.decode(&buf, DecoderTrap::Strict)
        .map_err(|e| Error::new(ErrorKind::InvalidData, e))?)
}

// Rust String -> Delphi UnicodeString
fn write_delphi_string<W: Write>(writer: &mut W, s: &str) -> Result<()> {
    let encoded = UTF_16LE.encode(s, EncoderTrap::Strict)
        .map_err(|e| Error::new(ErrorKind::InvalidData, e))?;
    writer.write_i32::<LittleEndian>(encoded.len() as i32)?;
    writer.write_all(&encoded)?;
    Ok(())
}
```

---

## 保存时过滤逻辑

Delphi `prepareSSTFile` 的过滤规则（需要复刻）：

1. **fproc 过滤**: 用户自定义比较函数
2. **colabId 过滤**: 仅保存指定 colabId 的条目
3. **空参数处理**: `sparams = []` 时，colabId > 0 → 设置 pending
4. **跳过空字符串**: `isEmpty` 且 colabId = 0
5. **跳过锁定**: `lockedStatus` (pexNoTrans / VMAD + lockedTrans)
6. **跳过警告**: 含 isDeleted/lowwarning/warning/bigwarning/nTrans
7. **跳过未翻译**: sparams 不含 [translated, lockedTrans, incompleteTrans, validated, pending]
8. **清理 lockedTrans**: 含 lockedTrans 时移除 translated/incompleteTrans/validated
9. **恢复 OldData**: unusedInSST 且非 pending → 设置 oldData

---

## sparams (sStrParams) 位定义

```delphi
sStrParam = (
  translated,       // bit 0
  lockedTrans,      // bit 1
  incompleteTrans,  // bit 2
  validated,        // bit 3
  deprecatedParam1, // bit 4 (未使用)
  deprecatedParam2, // bit 5 (未使用)
  oldData,          // bit 6
  pending           // bit 7
);
```

对应 Rust: `SkyStringParams(pub u8)`

---

## 验证测试矩阵

| 测试场景 | 说明 |
|---------|------|
| Roundtrip | Rust 写 → Rust 读，内容一致 |
| Delphi→Rust | Delphi 写的 SST → Rust 读，内容一致 |
| Rust→Delphi | Rust 写的 SST → Delphi 读，内容一致 |
| 版本兼容 | Rust 读 v2/v3/v4/v5/v6/v7 SST |
| 空文件 | 0 条字符串的 SST |
| 特殊字符 | 含 emoji/中文/阿拉伯文的字符串 |
| Master/Colab | 含 master list 和 colab label 的 SST |

---

## 实现风险

1. **UTF-16LE 编码**: Delphi 的 UnicodeString 和 Rust 的 UTF-16LE 可能有 BOM/替换字符差异
2. **String 长度**: Delphi 的 `length(s)` 返回字符数，不是字节数
3. **SetString 行为**: `SetString(s, nil, n)` 分配的字符串可能有未初始化内容，依赖 Read 完全填充
4. **对齐**: `rEspPointerLite` 是 `packed record`，在 Delphi 中无对齐填充，24 字节精确
