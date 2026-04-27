# i18n 多语言架构

> 来源：T22 UI i18n 实现

---

## 技术选型

- **框架**：`react-i18next` + `i18next`
- **翻译文件**：`ui/src/locales/<lang>/translation.json`
- **持久化**：localStorage `xtranslator-lang`
- **兜底语言**：`zh-CN`
- **语言注册中心**：`i18n.ts` 中的 `SUPPORTED_LANGS`（10 种语言）

## 添加新语言

```
1. i18n.ts 添加 import <lang> from "./locales/<code>/translation.json"
2. i18n.ts 添加 resources 注册
3. SUPPORTED_LANGS 添加 "<code>": "Native Name" 条目
4. ui/src/locales/<code>/translation.json 翻译全部 key
5. 无需修改组件代码——语言选择器自动读取 SUPPORTED_LANGS
```

## 翻译 key 统计

| 命名空间 | key 数量 | 覆盖范围 |
|----------|----------|----------|
| common | 18 | 按钮标签、过滤器 |
| app | 4 | 加载/解析消息 |
| sidebar | 10 | 文件信息、统计数据 |
| editor | 12 | 编辑器面板 |
| batch | 15 | 批量处理器 |
| bsa | 11 | BSA 浏览器 |
| pex | 9 | PEX 面板 |
| fuz | 10 | 音频面板 |
| dialog | 6 | 对话视图 |
| toast | 14 | 通知消息 |
| **总计** | **109** | — |

## 当前覆盖状态

| 语言 | 代码 | 翻译状态 |
|------|------|----------|
| 中文 | zh-CN | ✅ 完成（109 key） |
| English | en | ✅ 完成（109 key） |
| 日本語 | ja | ⏳ 模板（en copy，待翻译） |
| 한국어 | ko | ⏳ 模板 |
| Français | fr | ⏳ 模板 |
| Deutsch | de | ⏳ 模板 |
| Español | es | ⏳ 模板 |
| Русский | ru | ⏳ 模板 |
| Polski | pl | ⏳ 模板 |
| Português | pt | ⏳ 模板 |

## 预埋策略

T16-T20 新增组件（BsaBrowser、PexPanel、FuzPanel、DialogView）在创建时直接使用 `t('key')`，同步写入 `zh-CN/translation.json`。T22 时只需翻译 `en.json`，无需二次改造组件。

## Strings 去重缓存

`save_with_format` 使用 `HashMap<Vec<u8>, u32>` 缓存已写入的字节序列。相同内容共享数据偏移量。实测文件体积减少约 17%（914KB → 782KB）。
