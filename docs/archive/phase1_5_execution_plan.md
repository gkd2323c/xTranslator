# Phase 1.5 Tauri UI 基础框架 - 详细执行方案（已完成归档）

> **状态**：✅ 全部完成，已合并到主分支。
>
> 实际完成时间：2025-04（历史文档，保留供参考）

> **原始目标**：从 CLI 工具升级为可交互的桌面应用，能加载 ESP+Strings，显示翻译列表，编辑翻译，保存 SST。
> **前提**：底层引擎已全部就绪（ESP解析、Strings读写、SST读写、Codepage、XML解析）。
> **验收标准**：用户能在界面上完成一次完整的翻译闭环：加载 → 查看 → 编辑 → 保存。

---

## 1. 当前状态

### Tauri 后端 (`src-tauri/`)
- `AppState`：持有虚拟数据（10万条）
- `query_strings_command`：查询虚拟数据
- `get_stats`：返回统计信息
- **缺失**：加载真实 ESP、加载 SST、保存 SST

### React 前端 (`ui/`)
- `App.tsx`：基础表格 + 分页 + 筛选 + 排序（虚拟数据）
- `api/strings.ts`：IPC API 封装（queryStrings, getStats）
- **缺失**：文件加载、字符串编辑、保存流程、标签页切换

---

## 2. 任务分解与执行顺序

### Week 5 - Day 1-2：IPC 命令扩展（后端）

#### 任务 1.5.1：`load_esp` 命令

**目标**：从 CLI 的 `parse_esp` 功能迁移到 Tauri IPC。

**实现**：
```rust
#[tauri::command]
pub async fn load_esp(
    state: tauri::State<'_, Arc<AppState>>,
    esp_path: String,
    strings_dir: Option<String>,
) -> Result<LoadEspResponse, String> {
    // 1. 创建 EspParser
    // 2. 加载 Strings 文件
    // 3. 解析 ESP
    // 4. 将结果存入 AppState
    // 5. 返回统计信息
}
```

**验收标准**：
- [ ] 前端调用 `load_esp("D:/.../Skyrim.esm", "D:/.../Strings")` 成功
- [ ] 返回 total/filtered/compressed_records 等统计
- [ ] 解析后 `query_strings_command` 返回真实数据而非虚拟数据
- [ ] Skyrim.esm 249MB 解析时间在 5 秒内

#### 任务 1.5.2：`load_sst` 命令

**目标**：加载 SST 字典并匹配到已解析的 ESP 数据。

**实现**：
```rust
#[tauri::command]
pub async fn load_sst(
    state: tauri::State<'_, Arc<AppState>>,
    sst_path: String,
) -> Result<LoadSstResponse, String> {
    // 1. 读取 SST 文件
    // 2. 匹配 strId+record+field
    // 3. 更新 translation 和 params
    // 4. 返回匹配统计
}
```

**验收标准**：
- [ ] 加载 SST 后 translation 字段被填充
- [ ] 返回 matched/unmatched 统计
- [ ] `query_strings_command` 返回的 status 正确更新

#### 任务 1.5.3：`save_sst` 命令

**目标**：将当前编辑状态保存为 SST 文件。

**实现**：
```rust
#[tauri::command]
pub async fn save_sst(
    state: tauri::State<'_, Arc<AppState>>,
    sst_path: String,
) -> Result<(), String> {
    // 1. 从 AppState 取出数据
    // 2. 写入 SST v8 格式
    // 3. 返回成功/失败
}
```

**验收标准**：
- [ ] 保存的 SST 可被 `xt-cli sst read` 正确读取
- [ ] 保存后文件大小合理
- [ ] 保存后重新加载，数据一致

#### 任务 1.5.4：`update_translation` 命令

**目标**：前端编辑单条翻译后更新后端状态。

**实现**：
```rust
#[tauri::command]
pub async fn update_translation(
    state: tauri::State<'_, Arc<AppState>>,
    id: u32,
    translation: String,
) -> Result<(), String> {
    // 1. 查找对应 SkyString
    // 2. 更新 translation
    // 3. 更新 hash_trans
    // 4. 更新 params 状态
}
```

**验收标准**：
- [ ] 更新后 `query_strings_command` 返回新翻译
- [ ] 更新后 status 从 incomplete → translated
- [ ] 更新后保存 SST，内容正确

---

### Week 5 - Day 3-4：主窗口布局与文件加载（前端）

#### 任务 1.5.5：菜单栏 + 文件加载对话框

**目标**：实现文件菜单，支持加载 ESP 和 SST。

**实现**：
```tsx
// components/MenuBar.tsx
- File 菜单
  - Load ESP... (调用 tauri dialog.open)
  - Load SST... (调用 tauri dialog.open)
  - Save SST... (调用 tauri dialog.save)
  - Recent Files (最近文件列表)
```

**技术点**：
- 使用 `@tauri-apps/api/dialog` 打开文件选择器
- 使用 `@tauri-apps/api/path` 处理路径
- 菜单用原生 HTML/CSS 或轻量组件库

**验收标准**：
- [ ] 点击 Load ESP 弹出文件选择器，过滤 .esm/.esp
- [ ] 选择文件后调用 `load_esp`，显示加载进度
- [ ] 加载成功后表格显示真实数据
- [ ] 错误时显示友好提示（如 Strings 文件缺失）

#### 任务 1.5.6：左侧文件树 / 统计面板

**目标**：显示当前加载的文件信息和统计。

**实现**：
```tsx
// components/SidePanel.tsx
- 当前加载的 ESP 文件名
- Strings 文件加载状态（3个）
- 统计信息
  - Total strings
  - Translated / Incomplete / Locked
  - Compressed records
- Record 类型分布（柱状图或列表）
```

**验收标准**：
- [ ] 加载 ESP 后左侧显示文件名和统计
- [ ] 统计信息随数据更新
- [ ] 点击统计数字可筛选对应状态

---

### Week 5 - Day 5-7：虚拟表格与字符串编辑（前端）

#### 任务 1.5.7：虚拟表格（TanStack Table）

**目标**：替换当前简单表格为虚拟滚动表格，支持 7 万条数据流畅滚动。

**实现**：
```tsx
// 使用 @tanstack/react-table + react-window
// 或 @tanstack/react-virtual
```

**列设计**：
| 列 | 宽度 | 内容 | 可排序 |
|----|------|------|--------|
| ID | 60px | 序号 | ✅ |
| Status | 80px | 状态徽章 | ✅ |
| Record | 80px | NPC_/DIAL 等 | ✅ |
| Field | 80px | FULL/NAM1 等 | ✅ |
| FormID | 100px | 0x000123AB | ✅ |
| EDID | 120px | Editor ID | ✅ |
| Source | 弹性 | 源字符串 | ✅ |
| Translation | 弹性 | 翻译/编辑框 | - |

**验收标准**：
- [ ] 7 万条数据滚动无卡顿（<16ms 帧时间）
- [ ] 列可拖拽调整宽度
- [ ] 列可排序（点击表头）
- [ ] 行高固定，虚拟滚动正确计算

#### 任务 1.5.8：行内编辑

**目标**：双击或 F2 进入编辑模式，修改 Translation。

**实现**：
```tsx
// 两种模式：
// 1. 行内编辑：点击 Translation 单元格变为 input
// 2. 底部编辑区：选中行后在底部面板编辑
```

**建议采用底部编辑区**：
- 选中行后，底部显示 Source + Translation 编辑框
- 支持多行文本（.DLSTRINGS）
- 实时保存（debounce 500ms）或手动保存

**验收标准**：
- [ ] 选中行后底部显示编辑器
- [ ] 修改 Translation 后调用 `update_translation`
- [ ] 修改后状态徽章实时更新
- [ ] 支持多行文本（回车换行）

---

### Week 6 - Day 1-2：筛选/搜索栏

#### 任务 1.5.9：高级筛选栏

**目标**：实现多维度筛选，超越当前简单文本过滤。

**实现**：
```tsx
// components/FilterBar.tsx
- 文本搜索框（source + translation）
- 状态筛选：All / Translated / Incomplete / Locked
- Record 类型筛选：下拉选择 NPC_/DIAL/QUST 等
- Field 筛选：FULL/DESC/NAM1 等
- 清除筛选按钮
```

**验收标准**：
- [ ] 文本搜索支持 source 和 translation 同时匹配
- [ ] 状态筛选单选，实时更新表格
- [ ] Record 筛选多选或下拉
- [ ] 筛选组合正确（AND 逻辑）
- [ ] 筛选后分页重置到第 1 页

#### 任务 1.5.10：筛选持久化

**目标**：筛选条件保存在 URL query 或 localStorage。

**实现**：
```tsx
// 使用 URL search params 或 localStorage
// 刷新页面后筛选条件不丢失
```

**验收标准**：
- [ ] 刷新页面后筛选条件保留
- [ ] 筛选条件可分享（URL 可复制）

---

### Week 6 - Day 3-5：打磨与验收

#### 任务 1.5.11：错误处理与用户体验

**目标**：完善的错误提示和加载状态。

**实现**：
- Toast 通知系统（加载成功/失败/保存成功）
- 加载进度条（ESP 解析进度）
- 空状态提示（无数据时）
- 快捷键支持（Ctrl+O 打开，Ctrl+S 保存，F2 编辑）

**验收标准**：
- [ ] 所有 IPC 调用有 loading 状态
- [ ] 错误时有 Toast 提示
- [ ] 空数据时有友好提示
- [ ] 支持 Ctrl+O / Ctrl+S / F2

#### 任务 1.5.12：性能验收

**目标**：确保 UI 性能满足需求。

**测试项**：
| 场景 | 目标 | 测试方法 |
|------|------|---------|
| 初始加载 ESP | < 5s | 加载 Skyrim.esm |
| 翻页 | < 100ms | 点击 Next 按钮 |
| 筛选 | < 200ms | 输入关键词 |
| 滚动 | 60fps | 快速滚动 7 万条 |
| 内存占用 | < 200MB | Chrome DevTools |

---

## 3. 技术选型确认

| 组件 | 选型 | 理由 |
|------|------|------|
| 表格 | TanStack Table + react-window | 虚拟滚动，大数据量 |
| 状态管理 | Zustand | 轻量，跨组件共享 |
| UI 组件 | Headless UI / shadcn/ui | 可定制，无障碍 |
| 通知 | react-hot-toast | 轻量 Toast |
| 图标 | Lucide React | 轻量 SVG |
| 编辑器 | 原生 textarea（第一阶段） | 简单可靠，后续换 Monaco |

---

## 4. 文件变更计划

### 新增文件

```
src-tauri/src/
  commands.rs          # 扩展：load_esp, load_sst, save_sst, update_translation
  state.rs             # 新增：AppState 扩展，EspFileState

ui/src/
  components/
    MenuBar.tsx        # 新增：文件菜单
    SidePanel.tsx      # 新增：统计面板
    FilterBar.tsx      # 新增：筛选栏
    StringTable.tsx    # 新增：虚拟表格
    EditorPanel.tsx    # 新增：底部编辑区
    StatusBadge.tsx    # 新增：状态徽章
  stores/
    appStore.ts        # 新增：Zustand 状态管理
  hooks/
    useEspLoader.ts    # 新增：加载 ESP 逻辑
    useQuery.ts        # 新增：查询 + 分页
```

### 修改文件

```
src-tauri/src/
  main.rs              # 注册新命令

ui/src/
  App.tsx              # 重构：整合所有组件
  api/strings.ts       # 扩展：新增 API 函数
  App.css              # 更新：新布局样式
```

---

## 5. 风险与应对

| 风险 | 概率 | 影响 | 应对 |
|------|------|------|------|
| IPC 大数据传输慢 | 中 | 高 | 分页加载，后端筛选，只传当前页 |
| 虚拟表格实现复杂 | 中 | 中 | 先用简单表格，再优化为虚拟滚动 |
| 文件对话框权限问题 | 低 | 高 | 使用 Tauri dialog API，测试多平台 |
| 内存泄漏 | 低 | 高 | 使用 React DevTools Profiler 检查 |

---

## 6. 每日 Checklist

### Day 1
- [ ] `load_esp` 命令实现
- [ ] `load_esp` 与 `query_strings_command` 整合

### Day 2
- [ ] `load_sst` 命令实现
- [ ] `save_sst` 命令实现
- [ ] `update_translation` 命令实现

### Day 3
- [ ] MenuBar 组件
- [ ] 文件对话框集成
- [ ] 加载进度显示

### Day 4
- [ ] SidePanel 统计面板
- [ ] AppState 状态管理

### Day 5
- [ ] StringTable 虚拟表格基础
- [ ] 列定义和排序

### Day 6
- [ ] EditorPanel 底部编辑区
- [ ] 行内编辑功能

### Day 7
- [ ] FilterBar 筛选栏
- [ ] 多维度筛选

### Day 8
- [ ] 错误处理 + Toast
- [ ] 快捷键支持

### Day 9
- [ ] 性能测试与优化
- [ ] Bug 修复

### Day 10
- [ ] 最终验收测试
- [ ] 文档更新

---

## 7. 验收标准总结

**P0（必须完成）**：
1. 用户能点击菜单加载 ESP 文件
2. 表格显示解析出的字符串（7万条+）
3. 用户能选中一行并编辑 Translation
4. 编辑后能保存为 SST 文件
5. 保存的 SST 可被 Delphi 读取（验证兼容性）

**P1（尽力完成）**：
1. 虚拟滚动表格（60fps）
2. 高级筛选栏
3. 快捷键支持
4. 加载 SST 并匹配翻译

**P2（时间允许）**：
1. 主题切换
2. 最近文件列表
3. 列宽拖拽调整

---

> **下一步行动**：确认此方案后，开始 Day 1 任务：`load_esp` 命令实现。
