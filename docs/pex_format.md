# PEX 脚本二进制格式

> 来源：T17 PEX 字符串提取实现中的逆向分析
> 参考：Delphi `TESVT_scriptPex.pas`（1,466 行）

---

## 格式来源

基于 Delphi `TESVT_scriptPex.pas`（1,466 行）逆向分析。PEX 是 Bethesda Papyrus 脚本语言的编译后二进制格式。

## 完整二进制布局

```
┌──────────────────────────────────────────────┐
│ PEX File Layout                              │
├──────────────────────────────────────────────┤
│ Magic:      u32 = 0xFA57C0DE                 │
│ Major:      u8  = 3                          │
│ Minor:      u8  = 10 (Skyrim SE)             │
│ GameID:     u16                              │
│ CompileTime: u64                             │
├──────────────────────────────────────────────┤
│ StringTable                                  │
│   count:    u16                              │
│   for each: length(u16) + data(bytes) (UTF-8)│
├──────────────────────────────────────────────┤
│ DebugInfo                                    │
│   mod_time: u64                              │
│   count:    u16                              │
│   for each: length(u16) + data(bytes)        │
├──────────────────────────────────────────────┤
│ UserFlags                                    │
│   count:    u16                              │
│   for each: name_idx(u16) + flag_idx(u8)     │
├──────────────────────────────────────────────┤
│ ObjectInfos                                  │
│   count:    u16                              │
│   for each object:                           │
│     name_idx:       u16                      │
│     body_size:      u32                      │
│     ┌─ Body ──────────────────────────────┐ │
│     │ parent_class_idx: u16                │ │
│     │ doc_string_idx:    u16 ← DebugString │ │
│     │ user_flags_count: u16                │ │
│     │ auto_state_name_idx: u16             │ │
│     │ variables: count(u16) + per-var      │ │
│     │   name_idx, type_idx, flags(u32)     │ │
│     │   doc_idx(u16) ← DebugString         │ │
│     │   user_flags(u32)                    │ │
│     │   value_data (type-dependent skip)   │ │
│     │ guards: count(u16) + per-guard       │ │
│     │ property_groups: count(u16)          │ │
│     │   per-group: name_idx, doc_idx ←     │ │
│     │   per-property: name_idx, type_idx,  │ │
│     │     doc_idx ← DebugString            │ │
│     │ states: count(u16) + per-state        │ │
│     │   per-function: name_idx, return_idx │ │
│     │     doc_idx ← DebugString             │ │
│     │     params, locals, instructions      │ │
│     └──────────────────────────────────────┘ │
└──────────────────────────────────────────────┘
```

## 可翻译字符串提取来源

| 类型 | 来源 | Delphi 字段 |
|------|------|------------|
| **DebugString** | Object.docString / Function.docString / Property.docString / Var.sDoc | `pObj.docString`, `pFunc.docString` |
| **PropertyName** | Property name（非空且语义不是代码标识符） | `pProp.name` |

## 值类型跳过表

```
ValueType 描述
    0      None
    1      Byte (u8)
    2      Integer (u32)
    3      Float (f32)
    4      Bool (u8)
    5      String (u16 index into StringTable)
    6-8    Array/Struct (count(u16) + recursive)
   11      BoolArray (count + packed bytes)
   12      IntArray
   13      FloatArray
   14      StringArray
```

## 指令操作码-to-参数映射

| 操作码 | 参数数 | 示例 |
|--------|--------|------|
| 0x00..0x0D, 0x12, 0x16, 0x1B, 0x1E, 0x22..0x24, 0x26, 0x28..0x2C | 0 | NOP, RETURN, CAST |
| 0x0E, 0x10..0x11, 0x13..0x14, 0x1A, 0x1C..0x1D, 0x1F..0x21, 0x25, 0x27 | 1 | JUMP(u16), CALLSTATIC(u16) |
| 0x15, 0x17..0x19, 0x2B, 0x2D | 2 | ARRAY_CREATE(u16, u16) |

## 写回约束

`compile_pex_strings` 因需重建 StringTable 偏移量及维护二进制兼容性，列为 v2 功能。当前使用 XML 导出作为过渡方案，可被现有 `import_xml` 流程复用。

## 边界情况

- String table 索引为 0-based（`string_table[idx]`，与 Delphi `pexStringList[id]` 索引方式不同需注意）
- 值类型跳过器需递归处理 Array/Struct 类型（type 6-8）
- BoolArray（type 11）使用位压缩，需根据 count 计算字节数
