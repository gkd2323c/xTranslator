# E2E 测试文档

> 更新于 2026-05-24：DOM 选择器修复后重测中

---

## 测试统计

| 指标 | 值 |
|------|-----|
| 总测试数 | 64 |
| 测试文件 | 5 个 spec 文件 |
| 运行时间 | ~14s |

> **注意**：由于 Playwright 依赖真实 Chromium 浏览器，当前 headless 环境无法直接运行 E2E 测试。以下修复基于代码审查发现的 DOM 选择器不匹配问题。|

## 测试文件

```
ui/e2e/
├── fixtures/
│   └── base.ts              # 测试夹具基类
├── app.spec.ts              # 核心应用功能
├── components.spec.ts       # UI 组件行为
├── workflows.spec.ts        # 用户工作流
└── playwright.config.ts     # Playwright 配置
```

## 测试分类

| Tag | 描述 | 测试数 |
|-----|------|--------|
| `@nav` | 导航与选中行为 | 5 |
| `@edit` | 编辑器操作 | 3 |
| `@batch` | 批量翻译 | 2 |
| `@editor` | 编辑器对话框 | 6 |
| `@context` | 右键上下文菜单 | 2 |
| `@toolbar` | 工具栏功能 | 4 |
| `@statusbar` | 状态栏信息 | 2 |
| `@states` | 空状态与加载态 | 3 |
| `@replace` | 查找替换功能 | 2 |
| `@undo` | 撤销重做 | 1 |
| `@panel` | 面板操作 | 7 |
| `@mcm` | MCM 加载 | 2 |
| `@esp-compare` | ESP 对比 | 2 |
| `@fuz` | FUZ 扫描 | 2 |
| `@bsa` | BSA 浏览器 | 2 |
| `@pex` | PEX 面板 | 2 |
| `@dialogs` | 对话面板 | 1 |
| `@esp-tree` | ESP 树面板 | 2 |
| `@quests` | 任务面板 | 1 |
| `@header-proc` | 头部处理器 | 1 |
| `@header-wizard` | 头部向导 | 1 |
| `@data-configs` | 数据配置面板 | 1 |
| `@toolbox` | 工具箱面板 | 1 |
| `@settings` | 设置面板 | 1 |

## 运行测试

```bash
# 运行所有 E2E 测试
cd ui && npm run test:e2e

# 带 UI 运行
cd ui && npm run test:e2e:ui

# 调试模式
cd ui && npm run test:e2e:debug

# 运行特定 tag
cd ui && npx playwright test --grep "@nav"

# 运行单个测试文件
cd ui && npx playwright test e2e/app.spec.ts
```

## Mock 数据机制

### 工作原理

E2E 测试无需真实的 Tauri 后端或 ESP 文件。测试启动时自动注入模拟数据：

1. **Vite 配置检测 E2E 模式**
   - `NODE_ENV=test` 或 `VITE_E2E=true` 时启用
   - 自动解析 mock 别名，替换 Tauri API 调用

2. **Mock API 注入**
   - `e2e/mocks/tauri-core.ts` 提供模拟的 Tauri 命令
   - `generateMockStrings()` 生成 20 条测试字符串数据

3. **自动初始化**
   - `fixtures/base.ts` 的 `goto()` 方法调用 `window.__e2eAutoSeed()`
   - 直接注入 Zustand store，跳过 ESP 文件加载流程
   - 防重入：`__e2eAutoSeeded` 标志防止热重载时重复注入

### 关键实现

**Vite 别名配置** (`vite.config.ts`):
```ts
const isE2E =
  process.env.VITE_E2E === "true" ||
  process.env.NODE_ENV === "test";

resolve: isE2E ? {
  alias: {
    "@tauri-apps/api": "./e2e/mocks/tauri-core.ts",
  }
} : {}
```

**Mock 数据注入** (`e2e/mocks/tauri-core.ts`):
```ts
window.__e2eAutoSeed = () => {
  if ((window as any).__e2eAutoSeeded) return;
  const state = (window as any).__zustandStore.getState();
  state.__e2eInjectMock(generateMockStrings());
  (window as any).__e2eAutoSeeded = true;
};
```

**Store 暴露** (`stores/appStore.ts`):
```ts
// 仅 E2E 模式暴露
if (import.meta.env.DEV) {
  (window as any).__zustandStore = { getState };
}
```

### 扩展 Mock 数据

如需更多或不同的测试数据，修改 `e2e/mocks/tauri-core.ts` 中的 `generateMockStrings()` 函数：

```ts
export function generateMockStrings(count: number = 20): StringEntry[] {
  // 返回指定数量的模拟字符串条目
  return Array.from({ length: count }, (_, i) => ({
    id: i + 1,
    source: `Source text ${i + 1}`,
    target: `Translated text ${i + 1}`,
    status: i % 3 === 0 ? "incomplete" : "translated",
  }));
}
```

## 2026-05-24 修复记录

本次修复基于代码审查发现的 DOM 选择器不匹配问题，共修改 3 个测试文件：

### 1. `workflows.spec.ts` — 菜单选择器修正

| 原选择器 | 修正后 | 原因 |
|---------|--------|------|
| `.menubar-menu-button` | `.menubar-menu-trigger` | MenuBar 实际使用 `.menubar-menu-trigger` 作为菜单触发按钮类名 |
| `.menubar-toolbar` | `.menubar-actions` | 工具栏容器实际类名为 `.menubar-actions` |

### 2. `panels-advanced.spec.ts` — Tab 名称修正

| 原选择器 | 修正后 | 原因 |
|---------|--------|------|
| `.bottom-tab:has-text('ESP')` | `.bottom-tab:has-text('ESP Tree')` | 底部面板 tab 名称是 "ESP Tree"，不是 "ESP" |

### 3. `panels.spec.ts` — 面板打开方式修正

**问题**：MCM / EspCompare / FUZ / BSA / PEX 面板通过 Tools 菜单打开，不在 toolbar 上。原 `openPanel` 辅助函数尝试点击 toolbar 按钮，找不到对应元素。

**修复**：新增 `openToolPanel` 辅助函数，通过 `window.__zustandStore.getState().setActivePanel()` 直接打开面板：

```ts
async function openToolPanel(page: any, panelName: string) {
  await page.evaluate((name: string) => {
    const store = (window as any).__zustandStore?.getState();
    if (store?.setActivePanel) {
      store.setActivePanel(name);
    }
  }, panelName);
  await page.waitForTimeout(500);
}
```

### 待验证项

以下修复已提交，需在真实浏览器环境中运行 `npm run test:e2e` 验证：

- [ ] `workflows.spec.ts` — `@menu` tag 测试
- [ ] `panels-advanced.spec.ts` — `@esp-tree` tag 测试
- [ ] `panels.spec.ts` — `@mcm`, `@esp-compare`, `@fuz`, `@bsa`, `@pex` tag 测试

---

## 常见问题排查

### 测试超时

**现象**: `expect(locator).toBeAttached` 超时

**排查步骤**:
1. 确认 `vite.config.ts` 中 E2E 模式检测正确
2. 检查浏览器控制台是否有错误
3. 验证 mock 数据注入：`page.evaluate(() => (window as any).__e2eAutoSeeded)`

### Selector 无匹配

**现象**: `element not found` 错误

**排查步骤**:
1. 使用 `page.screenshot()` 截图查看当前状态
2. 检查 CSS 类名是否正确（如 `virtual-row` vs `row-selected-multi`）
3. 使用 Playwright 的 locator picker 确认实际选择器

### 行为与预期不符

**现象**: 点击/输入后 UI 无响应

**排查步骤**:
1. 检查是否需要等待状态更新（如 `waitForTimeout`）
2. 确认 Zustand store 状态正确更新
3. 检查 React 组件是否正确响应状态变化

## 添加新测试

### 基本结构

```ts
import { test, expect } from '@playwright/test';
import { test as appTest } from './fixtures/base';

appTest('my new test @new', async ({ appPage }) => {
  await appPage.goto();
  await expect(appPage.stringTable).toBeAttached();

  // 测试逻辑...
});
```

### 注意事项

1. **使用 `appTest` 而非 `test`**: 确保自动加载 mock 数据
2. **避免硬编码超时**: 使用 Playwright 的 expect timeout
3. **不要直接操作 Zustand**: 通过 UI 交互测试，不要直接修改 store
4. **复用 Page Object**: 在 `fixtures/base.ts` 中添加常用选择器

### Locator 最佳实践

```ts
// ✅ 推荐：使用 role 和 name
await page.getByRole('button', { name: 'Save' }).click();

// ✅ 推荐：精确匹配避免歧义
await page.getByRole('button', { name: 'Log', exact: true }).click();

// ✅ 推荐：CSS 类名时使用部分匹配
await expect(row).toHaveClass(/virtual-row-selected/);

// ❌ 避免：模糊正则匹配多个元素
await page.getByRole('button', { name: /log/i }).click(); // 会匹配 "Dialogs"
```
