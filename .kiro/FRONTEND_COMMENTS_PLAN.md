# 前端代码注释补充计划

## 📋 现状总结

### 前端代码分析
- **总文件数** 30+ 个
- **注释覆盖** 极低（仅 2 个文件已补充）
- **复杂度** 高（虚拟滚动、状态管理、事件处理）
- **优先级** 明确（已分类）

### 已完成
1. ✓ `ui/src/api/strings.ts` - IPC 包装函数（第二批）
2. ✓ `ui/src/stores/appStore.ts` - 状态管理（第一批）
3. ✓ `ui/src/App.tsx` - 主应用组件（本次）

### 待补充
- 第一优先级：3 个文件（StringTable、EditorPanel、MenuBar）
- 第二优先级：5 个文件（SidePanel、BatchPanel 等）
- 第三优先级：15+ 个文件（底部面板、工具面板、UI 组件）

---

## 🎯 第一优先级（关键，建议立即补充）

### 1. StringTable.tsx - 虚拟滚动表格
**文件大小:** ~400 行  
**复杂度:** 高  
**关键概念:**
- react-window v2 API（rowComponent、rowCount、rowHeight）
- 虚拟滚动原理（只渲染可见行）
- 搜索高亮实现
- 状态指示符（●/◆/○）
- VMAD 标记处理

**需要补充的内容:**
```typescript
// 1. 组件职责说明
// 2. ROW_HEIGHT 常量说明
// 3. VirtualRow 组件说明
// 4. highlightText() 函数说明
// 5. 状态指示符说明
// 6. 右键菜单集成说明
// 7. 性能优化说明
```

### 2. EditorPanel.tsx - 编辑对话框
**文件大小:** ~600 行  
**复杂度:** 很高  
**关键功能:**
- 本地翻译编辑
- 启发式搜索
- 自动翻译
- 拼写检查
- 字段大小验证
- 别名检查
- 简繁体转换
- 快捷键处理

**需要补充的内容:**
```typescript
// 1. 组件职责说明
// 2. 本地翻译状态管理
// 3. 保存流程说明
// 4. 启发式搜索集成
// 5. 翻译 API 集成
// 6. 拼写检查集成
// 7. 字段大小验证
// 8. 别名检查
// 9. TCSC 转换
// 10. 快捷键处理
```

### 3. MenuBar.tsx - 菜单栏
**文件大小:** ~800 行  
**复杂度:** 很高  
**关键菜单:**
- File: 打开、保存、导出、导入
- Translate: SST、Strings、批处理
- Options: 设置、工具箱、拼写检查
- Tools: 9 个工具面板
- Wizards: 向导

**需要补充的内容:**
```typescript
// 1. 菜单结构说明
// 2. 文件操作流程
// 3. ESP 加载流程
// 4. SST 加载/保存流程
// 5. XML 导入/导出流程
// 6. 批处理流程
// 7. 工具菜单说明
// 8. 向导菜单说明
// 9. 进度事件处理
```

---

## 📊 优先级详细说明

### 为什么这个顺序？

**第一优先级（App、StringTable、EditorPanel、MenuBar）**
- 这 4 个文件是应用的核心
- 控制全局流程和用户交互
- 复杂度最高，最需要文档
- 新开发者首先需要理解这些

**第二优先级（SidePanel、BatchPanel 等）**
- 重要但相对独立
- 功能明确，复杂度中等
- 可以在理解核心后学习

**第三优先级（底部面板、工具面板、UI 组件）**
- 功能相对独立
- 复杂度较低
- 可以按需补充

---

## 💡 注释风格指南

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

### Hook 注释模板
```typescript
/// Hook 名称 - 简短描述
///
/// 功能：
/// - 功能 1
/// - 功能 2
///
/// 依赖：
/// - dep1: 说明
/// - dep2: 说明
///
/// 副作用：
/// - 副作用 1
/// - 副作用 2
useEffect(() => {
  // ...
}, [dep1, dep2]);
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

## 📈 工作量估计

| 优先级 | 文件数 | 平均行数 | 注释行数 | 工作量 | 预计时间 |
|--------|--------|---------|---------|--------|---------|
| 第一 | 4 | 500 | 150 | 高 | 2-3 小时 |
| 第二 | 5 | 300 | 100 | 中 | 1-2 小时 |
| 第三 | 15+ | 200 | 50 | 低 | 2-3 小时 |

**总计:** ~30 个文件，~600 行注释，5-8 小时

---

## ✅ 建议行动计划

### 本次（已完成）
- [x] 前端代码分析
- [x] 优先级分类
- [x] App.tsx 注释补充
- [x] 生成分析报告

### 下一步（建议）
- [ ] 补充 StringTable.tsx 注释
- [ ] 补充 EditorPanel.tsx 注释
- [ ] 补充 MenuBar.tsx 注释
- [ ] 更新索引文档

### 后续（可选）
- [ ] 补充第二优先级文件
- [ ] 补充第三优先级文件
- [ ] 创建前端开发指南
- [ ] 创建组件库文档

---

## 📚 相关文档

### 已生成
- `.kiro/FRONTEND_COMMENTS_ANALYSIS.md` - 详细分析报告
- `.kiro/FRONTEND_COMMENTS_PLAN.md` - 本文档

### 参考
- `.kiro/COMMENTS_INDEX.md` - 完整索引
- `.kiro/COMMENT_QUICK_REFERENCE.md` - 快速参考
- `AGENTS.md` - 项目架构

---

## 🔍 关键概念速查

### 虚拟滚动（StringTable）
```typescript
// react-window v2 API
<List
  rowComponent={VirtualRow}
  rowCount={items.length}
  rowHeight={32}
  rowProps={{ items, selectedId, ... }}
/>

// 只渲染可见行，性能优化
// 行高固定为 32px
```

### 状态管理（AppStore）
```typescript
// Zustand 选择器模式
const field = useAppStore((s) => s.field);

// 避免不必要的重新渲染
// 只订阅需要的字段
```

### 事件处理（App）
```typescript
// 快捷键链
Escape → 关闭编辑 → 关闭面板 → 取消选择

// 后端事件
"batch-string-progress" → 更新翻译
"batch-string-complete" → 显示完成提示
```

---

**计划日期:** 2026-05-13  
**状态:** 待执行  
**优先级:** 高
