
# xTranslator 技术栈重写计划

## 项目概况

xTranslator 是一个成熟的 Bethesda 游戏（Skyrim/Fallout/Starfield）模组翻译工具，目前使用 Delphi 开发，约 **6.7 万行** 代码。

⚠️ **重要提示**：这 6.7 万行 Delphi 代码是经过 10+ 年迭代、包含无数边缘 case 和业务暗逻辑的生产代码，逻辑密度相当于 15-20 万行的 Java/Python。重写意味着重走原作者 10 年的踩坑路。

### 当前技术栈
- **语言**：Delphi (Object Pascal)
- **UI 框架**：VCL (Win32)
- **核心组件**：VirtualTreeView, SynEdit, madExcept, Hunspell
- **平台**：Windows 独占

### 重写目标
- 保持核心功能兼容性
- 实现跨平台支持（Windows/macOS/Linux）
- 更好的性能和内存安全性
- 现代化的开发体验和工具链
- 更易于维护和社区贡献

---

## 推荐技术栈

### 核心层 - Rust
**选择理由**：
- 原生性能，可媲美 Delphi/C++
- 编译期内存安全保证，无 GC 停顿
- 优秀的二进制处理能力，零开销抽象
- 跨平台原生支持（Windows/macOS/Linux）
- 强大的包管理（Cargo）和活跃的社区生态

**关键依赖**：
- `byteorder` - 二进制字节序处理
- `libz-sys` / `flate2` - Zlib 压缩（BSA/BA2）
- `lz4-sys` - LZ4 压缩支持
- `thiserror` / `anyhow` - 错误处理
- `tokio` - 异步运行时（API 翻译和 IO）
- `reqwest` - HTTP 客户端（翻译 API）
- `encoding_rs` + `codepage-rs` - 多编码支持
- `regex` - 正则表达式
- `ahash` - 高性能哈希表
- `rayon` - 数据并行化
- `human-panic` + `sentry` - 崩溃报告（替代 madExcept）
- `hunspell-rs` - 拼写检查绑定（待验证跨平台）
- `bincode` - 高效 IPC 二进制序列化（后期优化用，阶段 0 先用 JSON）
- `tracing` + `tracing-subscriber` - 结构化日志与可观测性
- `directories` - 跨平台配置/数据目录
- `rusqlite` - ESM 缓存数据库（可选）

### UI 层 - Tauri 2.x + React + TypeScript
⚠️ **待验证假设**：Tauri 的 Rust ↔ JS IPC 在处理 10 万+ 条数据时的性能需要实证验证。

**选择理由**：
- 原生窗口封装，体积极小（~10MB vs ~50MB Electron）
- Rust 与 JS 双向桥接，性能优秀
- React 生态成熟，开发效率高
- TypeScript 类型安全

**关键依赖**：
- `TanStack Table` - 高性能虚拟表格（替代 VirtualTreeView，需验证性能）
- `react-window` - 虚拟滚动，处理万级数据
- `TailwindCSS` + `ShadCN UI` - 现代化样式组件
- `Zustand` - 轻量级状态管理
- `Monaco Editor` - 代码高亮编辑器（替代 SynEdit）

### 备选 UI 方案（如果 Tauri 性能不达标）
- **egui** - Rust 原生立即模式 UI，性能极佳
- **Avalonia UI** - C# 跨平台原生 UI，.NET 生态成熟

---

## 项目结构：Cargo Workspace 多 crate ⭐ 新增

```
xTranslator/
├── Cargo.toml                      # Workspace root
├── crates/
│   ├── xt-core/                    # 领域层 + 基础设施层
│   │   ├── src/
│   │   │   ├── esp/               # ESP/ESM 解析
│   │   │   ├── bsa/               # BSA/BA2 归档
│   │   │   ├── pex/               # PEX 脚本解析
│   │   │   ├── strings/           # STRINGS 文件
│   │   │   ├── sst/               # SST 字典格式
│   │   │   ├── encoding/          # 多编码处理
│   │   │   ├── heuristic/         # 启发式匹配算法
│   │   │   └── translation_api/   # 在线翻译 API
│   │   └── Cargo.toml
│   ├── xt-shared/                  # DTO 定义、IPC 序列化格式
│   │   ├── src/
│   │   │   ├── query.rs           # QueryRequest/QueryResponse
│   │   │   ├── dto.rs             # SkyStringDTO, EspPointerDTO 等
│   │   │   └── commands.rs        # Tauri Command 定义
│   │   └── Cargo.toml
│   ├── xt-cli/                     # 命令行工具（黑盒测试驱动）
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   └── commands/          # parse, sst, diff 等子命令
│   │   └── Cargo.toml
│   └── xt-tauri/                   # Tauri 适配层 + 后台任务
│       ├── src/
│       │   ├── main.rs
│       │   ├── commands.rs        # Tauri Command 实现
│       │   ├── state.rs           # 应用状态管理
│       │   └── background.rs      # 后台任务系统
│       ├── Cargo.toml
│       └── ui/                    # React 前端代码
├── docs/                           # 开发文档
│   ├── delphi_analysis.md         # Delphi 代码分析成果
│   ├── esp_format.md              # ESP 格式文档
│   ├── sst_v8_format.md           # SST v8 二进制格式文档
│   └── heuristic_algorithm.md     # 启发式算法伪代码文档
└── tests/                          # 黑盒对比测试套件
```

**设计理由**：
- `xt-core` 可独立编译测试，不依赖 Tauri，CI 更快
- `xt-shared` 保证 IPC 两端类型一致，无序列化错误
- `xt-cli` 作为测试驱动，自动化黑盒对比
- `xt-tauri` 仅包含 UI 适配代码，无业务逻辑

---

## 架构设计

### 四层模块化架构

```
┌─────────────────────────────────────────────────────────────┐
│                     UI 层 (Tauri/React)                      │
│  - 视口级数据请求  - 视图状态管理  - 后台任务进度反馈       │
│  - Zustand：{ totalCount, visibleItems[], viewportOffset }   │
└─────────────────────────────────────────────────────────────┘
                              ↕  IPC 边界（视口分页协议）
┌─────────────────────────────────────────────────────────────┐
│                   应用服务层 (xt-tauri)                      │
│  - 文件加载协调  - 翻译引擎协调  - 撤销/重做  - 缓存管理    │
│  - 后台任务系统（tokio + rayon 分工 + mpsc 进度反馈）        │
│  - 统一查询引擎：筛选/排序/搜索 全在 Rust 完成                │
└─────────────────────────────────────────────────────────────┘
                              ↕
┌─────────────────────────────────────────────────────────────┐
│                    领域层 (xt-core)                          │
│  - SkyString, EspPointer 类型                               │
│  - 相似度算法（Levenshtein/LCS + 关键阈值参数）              │
│  - SST 字典系统  - 翻译 API 适配  - 多编码处理              │
└─────────────────────────────────────────────────────────────┘
                              ↕
┌─────────────────────────────────────────────────────────────┐
│                    基础设施层 (xt-core)                      │
│  - ESP/ESM 解析（各游戏变体对照表）                          │
│  - BSA/BA2 归档  - PEX 脚本解析  - STRINGS 文件处理         │
└─────────────────────────────────────────────────────────────┘
```

---

### 架构关键决策（已明确）

#### A. 状态管理策略 ✅
- **Rust 后端持有唯一数据源**：所有字符串数据、字典缓存、文件状态的唯一真实来源
- **React 前端只持有视图状态**：筛选条件、排序方式、当前选中项、UI 展开状态等
- **数据同步**：增量更新 + 批量变更，最小化 IPC 传输数据量

#### B. IPC 视口分页协议 ⭐ 新增
**核心原则**：后端做一切，前端只显示

```typescript
// 前端请求（只告诉后端"我需要看什么"）
interface QueryRequest {
  file_id: string;
  offset: number;           // 视口起始偏移
  limit: number;            // 视口大小，通常 50-100
  filter?: string;          // 搜索过滤词
  sort?: SortSpec;          // 排序方式
  column_filter?: ColumnFilter;  // 列筛选
}

// 后端响应（只返回当前视口的数据）
interface QueryResponse {
  total_count: number;      // 总记录数（用于滚动条）
  filtered_count: number;   // 筛选后总数
  items: SkyStringDTO[];    // 仅当前视口的 N 条记录
}
```

**设计理由**：
- 避免一次性传输 10 万+ 条记录到 JS 内存
- Rust 的筛选/排序比 JS 快 5-10 倍
- 编辑后只刷新当前视口，或后端推送增量

#### C. 并发模型分工 ⭐ 新增
**tokio + rayon 职责边界**：

| 场景 | 使用 | 执行方式 | 注意事项 |
|------|------|---------|---------|
| 文件加载、API 翻译、网络 IO | `tokio` | Tauri async command | 天然支持 Tauri 运行时 |
| 启发式搜索、字典匹配、大规模数据处理 | `rayon` | `tokio::task::spawn_blocking` | ❌ 禁止在 tokio 事件循环线程中直接调用 `par_iter()`，会阻塞整个 WebView |
| 进度反馈与取消 | `tokio::sync::mpsc` | 生产者/消费者通道 | rayon 任务定期发送进度；前端可发送取消信号 |

**伪代码示例**：
```rust
#[tauri::command]
async fn run_heuristic_search(
  file_id: String, 
  window: tauri::Window,
) -> Result<()> {
  // rayon 任务在 spawn_blocking 中运行
  tokio::task::spawn_blocking(move || {
    let items = load_items(&file_id);
    items.par_iter()  // rayon 并行
      .inspect(|_| report_progress(&window))
      .for_each(|item| process_item(item));
  }).await?;
  Ok(())
}
```

#### D. 序列化策略 ✅
- **不依赖结构体内存布局**：所有二进制读写使用 byteorder 逐字段手动序列化
- **避免 `#[repr(packed)]`**：未对齐访问在 Rust 中是 unsafe，污染大量代码
- **读写分离**：读取器和写入器独立实现，便于测试和验证
- **IPC 序列化**：阶段 0 先用 JSON，bincode 作为后期优化选项，不阻塞 Gate 0

#### E. 配置迁移策略 ⭐ 新增
| 阶段 | 策略 |
|------|------|
| **MVP** | 不做配置迁移，提供首次启动向导重新配置 |
| **后期（阶段 5）** | 写配置导入器，解析原工具的 `res.ini` 和注册表项（Windows 下） |
| **设计原则** | 新版使用独立配置文件，不与旧版共享；用户可选择"导入旧版设置" |

#### F. 应用数据持久化 ⭐ 新增
| 数据类型 | 存储方案 |
|----------|---------|
| 用户配置 | `directories` crate → 跨平台配置目录 + TOML |
| 最近文件列表 | 同上，配置目录内 |
| ESM 字符串缓存 | SQLite (`rusqlite`) 或自定义二进制格式 |
| 崩溃日志 + 运行日志 | `tracing-appender` 滚动日志 + Sentry 上报 |

#### G. 日志与可观测性 ⭐ 新增
- **统一日志框架**：`tracing` 结构化日志
- **Span 追踪**：文件加载、解析、保存等关键流程使用 span
- **双输出**：控制台 + 滚动日志文件
- **黑盒测试辅助**：通过日志 diff 快速定位新旧工具行为差异

#### H. 跨平台 UI 策略 (Level 2) ⭐ 新增
**Level 2 定义**：Windows 为主，架构预留 macOS 适配
- 快捷键：使用 `Cmd` on macOS，`Ctrl` on Windows，通过抽象层统一
- 文件路径：使用 `directories` crate 处理各平台标准路径
- 菜单栏：暂不追求 macOS 全局原生菜单，但架构预留钩子
- 对话框：使用 Tauri 原生对话框，自动适配各平台

---

### 核心数据结构映射

**Delphi → Rust**：

| Delphi 结构 | Rust 对应 | 说明 |
|------------|-----------|------|
| `tSkyStr` | `SkyString` | 源/翻译字符串、双哈希、分词、状态标记 |
| `rEspPointer` | `EspPointer` | 记录/字段签名、formID、索引、哈希 |
| `tRecord/tField` | `Record/Field` | ESP 记录层次结构 |

---

## 实施路线图

### 阶段 0：技术预研 + Delphi 代码深度分析（2-4 周，必须先完成）
⚠️ **关键 Gate**：此阶段不通过，不进入正式重写

#### 0.0 项目脚手架（第 1 周）
- Cargo Workspace 多 crate 结构搭建
- xt-shared DTO 基础定义
- Tauri + React 项目初始化

#### 技术验证任务
| 任务 | 成功标准 |
|------|---------|
| Tauri 大数据性能原型（JSON 序列化） | 加载 10 万条虚拟数据，滚动帧率 ≥ 50fps，筛选响应 ≤ 100ms |
| ESP 解析器 PoC | 正确解析 Skyrim.esm 并导出所有字符串，与原工具 diff 一致率 ≥ 99% |
| SST 双向兼容测试 | Rust 读写的 SST v8 能被 Delphi 版正确读取，反之亦然 |

#### Delphi 代码深度分析（并行执行）⭐ 立即启动
| 任务 | 产出物 | 价值 |
|------|--------|------|
| 精读 `TESVT_typedef.pas` | 核心结构体对照表 + 字段含义注释文档 | 避免 Rust 结构设计偏差 |
| 精读 `TESVT_espDefinition.pas` | 各游戏 ESP 差异对照表 | 避免解析错误 |
| 精读 `TESVT_HeuristicSearch.pas` | 算法伪代码 + 关键阈值参数文档 | 保证翻译结果一致性 |
| 梳理 `TESVT_SSTFunc.pas` | SST v8 格式二进制文档（原项目无文档） | 双向兼容的基石 |

**副产品**：CLI 测试工具
- 同时开发纯 Rust CLI 工具：`cargo run --bin xt-cli`
- 支持 ESP 解析、SST 读写、字符串导出
- 用于自动化黑盒对比测试，几乎不增加额外成本

#### UI 前置验证（并行进行）⭐ 新增
- 原工具主界面/编辑窗口截图采集
- 核心工作流线框图绘制
- 快捷键清单整理
- 跨平台差异点清单（macOS vs Windows 快捷键/路径）

#### Gate 0 决策点
在阶段 0 结束时回答：
1. Tauri 性能是否满足需求？→ 是/否（切换 egui/Avalonia）
2. 核心解析逻辑是否能正确复刻？→ 是/否（需要更多分析时间）
3. SST 双向兼容是否可行？→ 是/否（调整兼容性策略）
4. 架构设计是否合理？→ 是/否（调整 crate 边界）

---

### 阶段 1-6：后续阶段保持原计划不变

---

## 立即执行清单（按优先级排序）⭐ 更新

| 优先级 | 任务 | 预期时间 | 执行人 | 产出物 |
|--------|------|---------|-------|-------|
| **P0** | Delphi 核心代码深度分析 | 1 周 | 立即启动 | 4 份关键文档写入 `/docs` |
| **P0** | 设计并创建 Cargo Workspace 结构 | 2 天 | 立即启动 | 可编译的多 crate 骨架 |
| **P1** | Tauri 大数据性能原型（JSON 序列化 + 视口分页） | 1 周 | Workspace 就绪后 | 性能报告 |
| **P1** | ESP 解析器 PoC + CLI 测试工具 | 1-2 周 | Workspace 就绪后 | 字符串提取 diff 报告 |
| **P1** | SST 双向兼容测试 | 1 周 | Workspace 就绪后 | 新旧互读验证报告 |

---

## 一句话总结

> **计划已经 ready，可以进入阶段 0。现在最大的变量不是技术选型，而是阶段 0 的原型验证 + Delphi 代码分析的投入深度。**

---

*此文档为 xTranslator 项目的重写技术路线图，作为开发实施的指导依据。*

**最后更新**：2024 年，第三轮评审后补充了 Workspace 结构、持久化方案、日志策略、跨平台 Level 2 策略、UI 前置验证、bincode 分阶段引入。
