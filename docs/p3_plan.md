# P3 完善计划 — xTranslator

> **日期**: 2026-04-29
> **基线**: 31/31 SPEC 任务完成, 134 测试通过

## 目标

补齐"编辑→翻译→输出可用文件"完整闭环的剩余缺口，提升日常使用体验。

## 任务列表

### T1 — 配置持久化 (P3-config)

| 项目 | 说明 |
|------|------|
| 问题 | OpenAI/DeepL API Key 仅存内存，重启丢失 |
| 方案 | 写入 `%APPDATA%/xTranslator/config.json`（Windows），`~/.config/xTranslator/config.json`（Linux/macOS） |
| 字段 | `openai_api_key`, `deepl_api_key`, `current_provider`, `theme`, `language` |
| 前端 | App 启动时 an load；设置修改时即时写入 |
| 安全 | API Key 不记录到 git；.gitignore `config.json` |

### T2 — 翻译进度条 (P3-progress)

| 项目 | 说明 |
|------|------|
| 问题 | 用户不知道当前翻译进度 |
| 方案 | SidePanel 顶部或 EditorPanel 底部增加进度百分比条 |
| 数据 | `translated_count / total` (不含 locked) |
| 样式 | CSS var 驱动的 `<progress>` 条，与主题系统联动 |

### T3 — Finalize 工作流 (P3-finalize)

| 项目 | 说明 |
|------|------|
| 问题 | 编辑完成后缺少"一键输出"最终化流程 |
| 内容 | 展示翻译完成度、列出输出文件预览、一键生成 Strings + SST + XML |
| 入口 | MenuBar "Finalize" 按钮或 SidePanel 顶部操作区 |

### T4 — 繁简转换 (P3-tcsc)

| 项目 | 说明 |
|------|------|
| 问题 | 中文翻译组常有简繁混用 |
| 方案 | `xt-core` 新增 `tcsc` 模块；前端 EditorPanel 增加转换按钮 |
| 策略 | 基于 Unicode 字符映射表 (OpenCC 简化版) |
| 实现 | 两个函数：`to_simplified(text)`, `to_traditional(text)` |

## 执行顺序

```
T1 (配置持久化) → T2 (进度条) → T3 (Finalize) → T4 (繁简)
```

## 验证

每个任务完成后：
- `cargo test -p xt-core --lib` 全部通过
- `cargo build -p xtranslator-tauri` 零错误
- `npx tsc --noEmit` 零错误
- 手动启动 Tauri 验证功能可用
