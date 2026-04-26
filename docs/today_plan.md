# 今日工作计划 — 2026-04-26

按优先级依次推进，完成后更新项目文档。

## 1. 主题系统 (P3, 3-4h) ✅ 完成

> 目标：暗/亮/灰三主题切换

| # | 步骤 | 文件 | 状态 |
|---|------|------|------|
| 1a | `:root[data-theme="light"]` / `:root[data-theme="gray"]` CSS 变量定义 | `ui/src/App.css` | ✅ |
| 1b | Zustand store 添加 `theme` + `setTheme` + `cycleTheme` | `ui/src/stores/appStore.ts` | ✅ |
| 1c | `App.tsx` 设置 `<html data-theme={theme}>` | `ui/src/App.tsx` | ✅ |
| 1d | MenuBar 主题切换按钮 (Sun/Moon/Cloud icons) | `ui/src/components/MenuBar.tsx` | ✅ |
| 1e | 亮色主题下隐藏 body::before/::after 装饰 | `ui/src/App.css` | ✅ |

## 2. 正则搜索/替换 (P1, 5-6h) ✅ 完成

> 目标：regex filter + replace all

| # | 步骤 | 文件 | 状态 |
|---|------|------|------|
| 2a | Store: `useRegex`, `replaceText` 字段 + setters | `ui/src/stores/appStore.ts` | ✅ |
| 2b | `applyFilterAndSort` 支持 regex 匹配 (带 try/catch) | `ui/src/stores/appStore.ts` | ✅ |
| 2c | Store: `replaceAll` action | `ui/src/stores/appStore.ts` | ✅ |
| 2d | 搜索栏：regex 切换按钮、替换输入框、"Replace All" 按钮 | `ui/src/components/StringTable.tsx` | ✅ |
| 2e | 替换确认对话框 + toast 进度提示 | `ui/src/stores/appStore.ts` | ✅ |

## 3. Strings 写入去重 (Tech Debt, 2-3h) ✅ 完成

> 目标：相同内容共享偏移量，缩小文件 ~17%

| # | 步骤 | 文件 | 状态 |
|---|------|------|------|
| 3a | `save_with_format`: HashMap 缓存字节序列 → 共用偏移 | `crates/xt-core/src/strings/mod.rs` | ✅ |
| 3b | 添加去重测试 (3 条目，2 条相同内容 → 文件小于 55 字节) | `crates/xt-core/src/strings/mod.rs` | ✅ |
| 3c | `cargo test -p xt-core --lib` — 77/77 通过 | — | ✅ |

## 4. 文档同步 ✅

| # | 步骤 | 状态 |
|---|------|------|
| 4a | SPEC.md T21 主题系统 → x, T23 正则搜索 → x, T24 去重 → x | ✅ |
| 4b | PLAN.md 添加已实现功能 | ✅ |

## 统计

- 修改文件：7 个 (App.css, App.tsx, appStore.ts, MenuBar.tsx, StringTable.tsx, mod.rs, SPEC.md, PLAN.md)
- 新增测试：1 个 (test_save_deduplication)
- 现有测试：77/77 通过
- TypeScript 编译：通过
