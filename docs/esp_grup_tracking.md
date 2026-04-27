# ESP GRUP 层级与父记录跟踪

> 来源：T20 NPC/对话视图实现中的关键发现

---

## 关键发现

ESP 文件的 GRUP 层级结构隐含了记录的父子关系。**`GrupHeader.s_type` 字段（u32）对于子 GRUP 包含父记录的 FormID。**

```
ESP 层级示例：
┌─────────────────────────────────────┐
│ TES4 (文件头)                       │
│ ┌─ GRUP(QUST) ────────────────────┐ │
│ │  QUST record (form_id=0xABC)    │ │
│ │  ┌─ GRUP(s_type=0xABC) ──────┐ │ │
│ │  │  DIAL records (子记录)     │ │ │
│ │  │  ┌─ GRUP(s_type=0xDEF) ─┐ │ │ │
│ │  │  │  INFO records (孙记录) │ │ │ │
│ │  │  └───────────────────────┘ │ │ │
│ │  └───────────────────────────┘ │ │
│ └─────────────────────────────────┘ │
└─────────────────────────────────────┘
```

## 实现方案

在 `EspParser` 中添加 `current_parent_form_id: u32` 字段，递归解析 GRUP 时保存/恢复父 FormID。

```rust
// 嵌套 GRUP 解析时跟踪父 FormID
let grup_header = GrupHeader::read_from(reader)?;
let saved_parent = self.current_parent_form_id;
if grup_header.s_type != 0 {
    self.current_parent_form_id = grup_header.s_type;
}
// ...递归解析子记录...
self.current_parent_form_id = saved_parent;
```

在创建 `SkyString` 时设置 `parent_form_id`：

```rust
sk.parent_form_id = self.current_parent_form_id;
```

`parent_form_id` 是运行时字段，不持久化到 SST 字典。

## 对话树构建

`build_dialog_tree` 命令根据 `parent_form_id` 将 INFO 字符串按父 DIAL FormID 分组。结果在前端以可展开的对话组显示，支持点击跳转到对应字符串。

## ESP dsize 语义提醒

- **Record** `dsize` 排除 16B record header
- **GRUP** `dsize` 包含 24B header（GenericHeader 8B + GrupHeader 16B）
- 因此 GRUP payload size = `dsize - 24`
