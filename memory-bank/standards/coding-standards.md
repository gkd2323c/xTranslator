# Coding Standards

## Overview
Rust 后端遵循 Cargo workspace 约定，TypeScript 前端遵循 React 社区标准。所有新代码不添加注释（除非明确要求）。

## Code Formatting

**Rust**: `rustfmt` (默认配置)
**TypeScript**: Prettier (默认配置)

## Linting

**Rust**: `clippy` (默认 lints)
**TypeScript**: `tsc --noEmit` + ESLint (`@typescript-eslint/recommended`)
**Strictness**: TypeScript `strict: true`

## Naming Conventions

### Rust

| 元素 | 约定 | 示例 |
|------|------|------|
| 变量/函数 | snake_case | `load_esp`, `cache_payload` |
| 类型/结构体 | PascalCase | `AppState`, `SkyString` |
| 模块/文件 | snake_case | `commands.rs`, `mod.rs` |
| 常量 | UPPER_SNAKE (screaming) | 遵循 Rust 标准 |

### TypeScript

| 元素 | 约定 | 示例 |
|------|------|------|
| 变量/函数 | camelCase | `loadAllStrings`, `selectedId` |
| 组件/类型 | PascalCase | `SidePanel`, `AppStore` |
| Hooks | camelCase `use` 前缀 | `useAppStore` |
| 常量 | UPPER_SNAKE | `API_URL` |
| 组件文件 | PascalCase | `SidePanel.tsx` |
| 工具文件 | kebab-case | `date-utils.ts` |

## File Organization

```
crates/
  xt-core/          # 核心库：ESP 解析、翻译 API、缓存等
  xt-shared/        # IPC DTO（Rust 端真实来源）
  xt-cli/           # CLI 工具
src-tauri/          # Tauri 后端：commands.rs, main.rs
ui/
  src/
    main.tsx        # 入口
    api/            # IPC 调用封装 + TypeScript DTO 镜像
    stores/         # Zustand stores
    components/     # React 组件
```

**约定**:
- DTO 真实来源：`crates/xt-shared/src/dto.rs`；前端镜像：`ui/src/api/strings.ts`
- 测试与源码同目录（Rust: `#[cfg(test)] mod tests`；TS: `.test.ts` 或 `__tests__/`）

## Testing Strategy

**Rust**: `cargo test -p xt-core --lib` (单元测试), `cargo test -p xt-core --test e2e_real_data` (E2E)
**TypeScript**: `npx tsc --noEmit` (类型检查), Vitest (单元/组件测试)

| 类型 | 工具 | 说明 |
|------|------|------|
| Rust 单元 | `cargo test` | 核心库测试，无需外部依赖 |
| Rust E2E | `cargo test --test e2e_real_data` | 需要 Skyrim.esm |
| TS 类型检查 | `tsc --noEmit` | 编译时类型验证 |
| TS 单元 | Vitest | React 组件、Zustand store、hooks |

**测试命名**: Rust: `test_name_here`，TS: `it('should...')` 或 `test('when X, then Y')`

## Error Handling

**Rust**: `anyhow` 用于通用错误传播，`thiserror` 用于自定义错误枚举。使用 `?` 操作符传播错误。

**TypeScript**: try/catch 用于异步操作，组件级错误边界用于 React。

## Logging

**Rust**: `log` crate + `tracing` 用于结构化日志，Tauri 前端通过 `console.log` 输出到 devtools。
**格式**: 结构化（Rust），文本（JS console）

| 级别 | 使用场景 |
|------|----------|
| error | 解析失败、API 调用失败 |
| warn | 缓存未命中、降级行为 |
| info | 文件加载、翻译完成 |
| debug | 详细解析信息（仅开发环境） |

**禁止记录**: API 密钥、敏感文件路径。
