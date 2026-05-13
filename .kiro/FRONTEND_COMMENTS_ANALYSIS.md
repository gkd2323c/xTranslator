# 前端代码注释需求分析

## 📊 前端代码现状

### 目录结构
```
ui/src/
├── api/
│   └── strings.ts ✓ (已补充)
├── components/
│   ├── bottom/ (7 个底部面板)
│   ├── ui/ (UI 基础组件)
│   ├── App.tsx (主应用)
│   ├── MenuBar.tsx (菜单栏)
│   ├── StringTable.tsx (虚拟滚动表格)
│   ├── EditorPanel.tsx (编辑对话框)
│   ├── SidePanel.tsx (侧边栏)
│   ├── StatusBar.tsx (状态栏)
│   ├── BatchPanel.tsx (批处理面板)
│   ├── BatchTranslateBar.tsx (批处理进度条)
│   ├── RecoveryPromptModal.tsx (恢复提示)
│   ├── SettingsDialog.tsx (设置对话框)
│   ├── ToolboxDialog.tsx (工具箱对话框)
│   ├── SpellCheckSettingsDialog.tsx (拼写检查设置)
│   ├── ContextMenu.tsx (右键菜单)
│   ├── ProgressBar.tsx (进度条)
│   ├── BsaBrowser.tsx (BSA 浏览器)
│   ├── PexPanel.tsx (PEX 脚本面板)
│   ├── FuzPanel.tsx (FUZ 音频面板)
│   ├── DialogView.tsx (对话树面板)
│   ├── McmPanel.tsx (MCM 配置面板)
│   ├── EspComparePanel.tsx (ESP 对比面板)
│   ├── FinalizePanel.tsx (最终化面板)
│   └── DataConfigsPanel.tsx (数据配置面板)
├── stores/
│   └── appStore.ts ✓ (已补充)
├── locales/ (多语言)
├── App.tsx (主应用入口)
├── main.tsx (应用启动)
└── i18n.ts (国际化)
```

### 注释覆盖现状

| 类别 | 文件数 | 注释覆盖 | 优先级 |
|------|--------|---------|--------|
| 已补充 | 2 | 高 | - |
| 核心组件 | 5 | 低 | 高 |
| 底部面板 | 7 | 低 | 中 |
| 工具面板 | 8 | 低 | 中 |
| UI 基础 | 多 | 低 | 低 |
| 工具函数 | 多 | 低 | 低 |

---

## 🎯 核心组件注释需求

### 1. App.tsx - 主应用组件
**现状:** 无注释  
**行数:** ~300 行  
**复杂度:** 高

**需要补充的内容:**
- 组件职责说明
- 关键 hooks 的用途（useEffect 链）
- 事件监听器说明（ESP 加载、SST 加载、批处理进度等）
- 自动备份机制
- 快捷键处理逻辑
- 主题和语言加载流程

**关键逻辑:**
```typescript
// 1. 键盘事件处理（Escape、Ctrl+Z/Y）
// 2. 配置加载（主题、语言、API Key）
// 3. 事件监听（esp-load-progress、sst-load-progress 等）
// 4. 自动备份定时器
// 5. 恢复提示模态框
```

---

### 2. StringTable.tsx - 虚拟滚动表格
**现状:** 无注释  
**行数:** ~400 行  
**复杂度:** 高

**需要补充的内容:**
- 虚拟滚动原理说明
- react-window v2 API 使用
- 行组件的渲染逻辑
- 搜索高亮实现
- 状态指示符说明
- 右键菜单集成
- 性能优化说明

**关键概念:**
```typescript
// 1. ROW_HEIGHT = 32px（虚拟滚动的行高）
// 2. VirtualRow 组件（react-window v2 行渲染）
// 3. highlightText() - 搜索词高亮
// 4. 状态指示符：● (translated), ◆ (locked), ○ (incomplete)
// 5. VMAD 标记（脚本字符串特殊处理）
```

---

### 3. EditorPanel.tsx - 编辑对话框
**现状:** 无注释  
**行数:** ~600 行  
**复杂度:** 很高

**需要补充的内容:**
- 编辑对话框的完整工作流
- 本地翻译状态管理
- 保存流程（IPC 调用）
- 启发式搜索集成
- 翻译 API 集成
- 拼写检查集成
- 字段大小验证
- 别名检查
- TCSC 转换（简繁体）
- 快捷键处理（Ctrl+Enter 保存、Ctrl+↑/↓ 导航）

**关键功能:**
```typescript
// 1. 本地翻译编辑（localTrans 状态）
// 2. 启发式搜索（heuristicSearch）
// 3. 自动翻译（translateString）
// 4. 拼写检查（spellCheckText）
// 5. 字段大小警告（fieldSizeWarning）
// 6. 别名检查（checkAliases）
// 7. 简繁体转换（tcscConvert）
// 8. 上/下导航（jumpToUntranslated）
```

---

### 4. MenuBar.tsx - 菜单栏
**现状:** 无注释  
**行数:** ~800 行  
**复杂度:** 很高

**需要补充的内容:**
- 菜单结构说明
- 文件操作流程（打开、保存、导出、导入）
- ESP 加载流程
- SST 加载/保存流程
- XML 导入/导出流程
- 批处理流程
- 工具菜单说明
- 向导菜单说明
- 进度事件处理

**关键菜单:**
```typescript
// 1. File: 打开 ESP、保存 SST、导出 XML、导入 XML
// 2. Translate: 加载 SST、保存 Strings、批处理
// 3. Options: 设置、工具箱、拼写检查
// 4. Tools: BSA 浏览、PEX 编辑、FUZ 扫描、对话树、MCM、ESP 对比
// 5. Wizards: 最终化、数据配置、头部处理
```

---

### 5. SidePanel.tsx - 侧边栏
**现状:** 无注释  
**行数:** ~200 行  
**复杂度:** 中

**需要补充的内容:**
- 侧边栏的统计信息说明
- 各统计项的含义
- 数据来源（allItems vs items）
- 刷新机制

---

## 🎨 底部面板注释需求

### 底部面板列表
1. **VocabularyPanel.tsx** - 词汇库面板
   - 词汇库的加载和显示
   - 搜索和过滤
   - 复制功能

2. **HeuristicPanel.tsx** - 启发式搜索面板
   - 相似度搜索
   - 结果排序
   - 应用翻译

3. **EspTreePanel.tsx** - ESP 记录树面板
   - 记录树的展示
   - 层级导航
   - 字段查看

4. **QuestsPanel.tsx** - 任务面板
   - 任务列表
   - 任务详情
   - 对话关联

5. **LogPanel.tsx** - 日志面板
   - 日志输出
   - 日志级别
   - 日志清空

6. **HeaderProcessorPanel.tsx** - 头部处理器面板
   - 规则管理
   - 规则应用
   - 批处理

7. **HeaderWizardPanel.tsx** - 头部向导面板
   - 向导流程
   - 规则生成
   - 预览

---

## 🛠️ 工具面板注释需求

### 工具面板列表
1. **BatchPanel.tsx** - 批处理面板
   - 文件列表管理
   - 批处理配置
   - 进度跟踪

2. **BatchTranslateBar.tsx** - 批处理进度条
   - 进度显示
   - 取消按钮
   - 错误显示

3. **BsaBrowser.tsx** - BSA 浏览器
   - 文件列表
   - 文件提取
   - 预览

4. **PexPanel.tsx** - PEX 脚本面板
   - 脚本解析
   - 字符串提取
   - 编译

5. **FuzPanel.tsx** - FUZ 音频面板
   - 音频扫描
   - 映射显示
   - 导出

6. **DialogView.tsx** - 对话树面板
   - 对话树构建
   - 对话导航
   - 文本显示

7. **McmPanel.tsx** - MCM 配置面板
   - 配置加载
   - 条目编辑
   - 保存

8. **EspComparePanel.tsx** - ESP 对比面板
   - 文件对比
   - 差异显示
   - 合并

9. **FinalizePanel.tsx** - 最终化面板
   - 最终化流程
   - 输出配置
   - 进度跟踪

10. **DataConfigsPanel.tsx** - 数据配置面板
    - 配置加载
    - 配置显示
    - 配置编辑

---

## 📋 UI 基础组件注释需求

### UI 组件列表
- **Button.tsx** - 按钮组件
- **Input.tsx** - 输入框组件
- **Textarea.tsx** - 文本区域组件
- **Modal.tsx** - 模态框组件
- **Badge.tsx** - 徽章组件
- **ProgressBar.tsx** - 进度条组件
- **Spinner.tsx** - 加载动画组件
- **Dropdown.tsx** - 下拉菜单组件
- **Tabs.tsx** - 标签页组件

**注释需求:** 低（基础组件，用途明确）

---

## 🔧 工具函数注释需求

### 工具函数位置
- **ContextMenu.tsx** - 右键菜单处理
- **RecoveryPromptModal.tsx** - 恢复提示
- **SettingsDialog.tsx** - 设置对话框
- **ToolboxDialog.tsx** - 工具箱对话框
- **SpellCheckSettingsDialog.tsx** - 拼写检查设置
- **StatusBar.tsx** - 状态栏

---

## 📈 优先级排序

### 第一优先级（关键，需立即补充）
1. **App.tsx** - 主应用入口，控制全局流程
2. **StringTable.tsx** - 虚拟滚动表格，性能关键
3. **EditorPanel.tsx** - 编辑对话框，功能复杂
4. **MenuBar.tsx** - 菜单栏，功能众多

### 第二优先级（重要，建议补充）
1. **SidePanel.tsx** - 侧边栏统计
2. **BatchPanel.tsx** - 批处理面板
3. **BatchTranslateBar.tsx** - 批处理进度
4. **VocabularyPanel.tsx** - 词汇库面板
5. **HeuristicPanel.tsx** - 启发式搜索面板

### 第三优先级（可选，后续补充）
1. 其他底部面板（EspTree、Quests、Log、HeaderProcessor、HeaderWizard）
2. 其他工具面板（BsaBrowser、PexPanel、FuzPanel、DialogView、McmPanel、EspCompare、Finalize、DataConfigs）
3. UI 基础组件
4. 工具函数和对话框

---

## 💡 注释风格建议

### 组件注释模板
```typescript
/// 组件名称 - 简短描述
///
/// 职责：
/// - 职责 1
/// - 职责 2
///
/// Props：
/// - prop1: 说明
/// - prop2: 说明
///
/// 关键状态：
/// - state1: 说明
/// - state2: 说明
///
/// 关键函数：
/// - function1(): 说明
/// - function2(): 说明
///
/// 设计要点：
/// - 要点 1
/// - 要点 2
export function ComponentName() {
  // ...
}
```

### 函数注释模板
```typescript
/// 函数名称 - 简短描述
///
/// 参数：
/// - param1: 说明
/// - param2: 说明
///
/// 返回：
/// - 返回值说明
///
/// 例子：
/// ```typescript
/// const result = functionName(arg1, arg2);
/// ```
function functionName(param1: Type1, param2: Type2): ReturnType {
  // ...
}
```

---

## 📊 补充工作量估计

| 优先级 | 文件数 | 平均行数 | 注释行数 | 工作量 |
|--------|--------|---------|---------|--------|
| 第一 | 4 | 500 | 150 | 高 |
| 第二 | 5 | 300 | 100 | 中 |
| 第三 | 15+ | 200 | 50 | 低 |

**总计:** ~30 个文件，~600 行注释

---

## ✅ 建议行动

### 立即行动（本次）
- [ ] 补充第一优先级的 4 个文件
- [ ] 更新索引文档

### 短期行动（1-2 周）
- [ ] 补充第二优先级的 5 个文件
- [ ] 补充关键底部面板

### 中期行动（1-2 月）
- [ ] 补充第三优先级的文件
- [ ] 补充 UI 基础组件
- [ ] 创建前端开发指南

---

**分析日期:** 2026-05-13  
**分析者:** Kiro  
**状态:** 待执行
