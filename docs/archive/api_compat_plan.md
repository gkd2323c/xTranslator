# API 翻译器兼容计划

> **日期**: 2026-04-29
> **基线**: 153 测试通过, ApiTranslator.txt 解析完成, CRLF 保护已完成

## 目标

将 Delphi 原版 API 翻译功能中的关键特性移植到 Rust 重写。

## 任务列表

### A1 — CRLF 保护 ✅ 已完成

| 项目 | 说明 |
|------|------|
| 问题 | 多行文本发送给 API 时，换行符可能被吞掉或转换 |
| Delphi 做法 | 翻译前 `\r\n` → `<L_F>` 标签，翻译后还原 |
| 方案 | `OpenAIProvider` 和 `DeepLProvider` 的 `translate()` 中自动处理 |
| 位置 | `translation_api/mod.rs` (`protect_crlf`/`restore_crlf`), `openai.rs`, `deepl.rs` |

### A2 — HTTP 代理支持 ⚠️ 部分完成

| 项目 | 说明 |
|------|------|
| 问题 | 国内用户无法直连 OpenAI/DeepL API |
| Delphi 做法 | `commonApiPrefs.ini` 中 `Proxy_Server/Port/Username/Password`，`setProxyREST()` |
| 已完成 | `AppConfig` 已有 proxy 字段 + `build_proxy()`/`build_client()` 函数 + `save_config`/`load_config` IPC |
| 待完成 | `OpenAIProvider`/`DeepLProvider` 仍使用 `Client::new()`，需替换为 `build_client()`；无 proxy 设置 UI |

### A3 — 用户偏好设置 UI

| 项目 | 说明 |
|------|------|
| 问题 | 用户无法自定义 OpenAI Model/URL/Query |
| Delphi 做法 | `Settings` 对话框 `ValueListEditor` 编辑 `OpenAI_Key/Model/URL/Query`，保存到 `commonApiPrefs.ini` |
| 方案 | 扩展 `set_openai_api_key` 命令 + 新增 `set_api_preference` 命令；前端 Settings 面板 |
| 位置 | `commands.rs` IPC, `config.rs` 持久化 |

### A4 — 批量翻译 + 速率限制

| 项目 | 说明 |
|------|------|
| 问题 | 翻译大量字符串时逐个调用 API，无批量优化和限流保护 |
| Delphi 做法 | `ArrayLimit` 分批、`ArrayTimePause` 间隔、`ArrayMaxCharPerMin` 上限、`SingleTimePause` 单次间隔 |
| 方案 | `BatchExecutor` 集成 config 的速率限制参数 |
| 位置 | `batch.rs`, `config.rs` |

## 执行顺序

```
A1 (CRLF保护) → A2 (代理) → A3 (偏好UI) → A4 (批量+限流)
```

## 验证

- `cargo test -p xt-core --lib` 全部通过
- `cargo build -p xtranslator-tauri` 零错误
- `npx tsc --noEmit` 零错误
