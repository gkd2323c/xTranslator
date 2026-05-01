# Data Stack

## Overview
桌面文件处理工具，无传统数据库。数据持久化通过文件系统缓存（bincode）和 Bethesda 原生格式（SST/XML）实现。运行时状态保存在内存中。

## Database
**无需传统数据库**

所有数据以文件形式存储：
- ESP 解析缓存：`%LOCALAPPDATA%/xTranslator/cache/` 下以 SHA-256 哈希命名的 bincode 文件，最多保留 50 个条目
- 翻译导入导出：XML 文件（Delphi 兼容的 UTF-8）
- 字符串表：Bethesda `.STRINGS` / `.DLSTRINGS` / `.ILSTRINGS` 格式

## ORM / Database Client
**不适用**

使用 Rust `bincode` + `serde` 进行缓存序列化，`quick-xml` 处理 XML 导入导出。无 ORM 需求。
