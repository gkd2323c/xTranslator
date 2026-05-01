---
intent: 001-batch-translation
phase: inception
status: complete
created: 2026-05-01T12:00:00Z
updated: 2026-05-01T12:30:00Z
---

# Requirements: 批量翻译

## Intent Overview

为 xTranslator 添加批量 AI 翻译能力。用户可选中多条未翻译字符串，一键调用翻译 API（OpenAI/DeepL），翻译在后台异步执行，UI 不阻塞。单条手动翻译也保持可用。翻译结果实时持久化到磁盘，意外中断不丢失进度。

## Business Goals

| Goal | Success Metric | Priority |
|------|----------------|----------|
| 批量翻译效率提升 | 一次操作翻译 N 条，无需逐条点击 | Must |
| UI 不阻塞 | 翻译过程中可继续浏览、筛选、编辑其他条目 | Must |
| 数据不丢失 | 意外退出后恢复，已翻译结果全部保留 | Must |
| 手动单条翻译 | 已有的单条 API 翻译功能不受影响 | Must |

---

## Functional Requirements

### FR-1: 批量翻译
- **Description**: 用户选中多个条目后，触发批量翻译。后台异步发送 API 请求，逐条更新翻译结果。翻译结果实时写入独立翻译缓存文件和内存中的字符串列表。
- **Acceptance Criteria**:
  - 选中 N 条未翻译字符串 → 点击"批量翻译" → N 条逐条调用 API 并显示翻译结果
  - 每条翻译完成后，结果立即写入独立翻译缓存文件
  - 单条失败时记录错误信息并通知用户，不阻塞剩余翻译
- **Priority**: Must

### FR-2: 非阻塞 UI
- **Description**: 翻译过程中，前端 UI 保持可交互。用户可以滚动表格、切换筛选、编辑其他条目。
- **Acceptance Criteria**:
  - 翻译在后台运行时，表格可正常滚动/筛选/排序
  - 翻译完成的条目实时更新显示，无需刷新
- **Priority**: Must

### FR-3: 独立翻译缓存文件
- **Description**: 翻译结果写入独立的批量翻译缓存文件（与 ESP cache 分离），每条完成后立即追加写入（append-only journal），不等待全批次完成。
- **Acceptance Criteria**:
  - 缓存文件格式为 append-only journal，记录每条翻译的 `(str_id, source, translated, timestamp)`
  - 翻译到第 50 条时强制退出 → 重启后前 50 条结果在缓存文件中
  - 写入操作不阻塞后续翻译请求
- **Priority**: Must

### FR-4: 翻译进度与取消
- **Description**: 显示批量翻译进度（已完成/总数），支持中途取消。取消后已完成的结果保留。
- **Acceptance Criteria**:
  - 进度条/计数器显示 "12/50 已完成"
  - 点击取消 → 停止发送新请求，已完成的结果不丢失
- **Priority**: Should

### FR-5: 并发控制
- **Description**: 用户可调并发数（1-10），界面提供滑块或输入框。默认并发 3。
- **Acceptance Criteria**:
  - 并发数影响框（slider/input）在批量翻译前可调整
  - 翻译进行中不可更改并发数
- **Priority**: Should

### FR-6: 错误处理与重试
- **Description**: 单条翻译 API 调用失败时，记录错误信息并通知用户，不阻塞剩余翻译。自动重试临时故障。
- **Acceptance Criteria**:
  - API 报错时自动重试（最多 3 次，指数退避 1s/2s/4s）
  - 重试 3 次仍失败 → 记录该条目为失败，跳过继续
  - 翻译完成后汇总显示：成功 N 条，失败 M 条（含错误原因）
- **Priority**: Must

### FR-7: 崩溃恢复
- **Description**: 应用重启后检测是否存在未完成的批量翻译缓存文件，提示用户是否恢复。
- **Acceptance Criteria**:
  - 启动时扫描翻译缓存文件，若有已翻译但未应用到 ESP 的记录 → 弹窗提示 "发现 N 条未应用的翻译，是否恢复？"
  - 用户确认 → 将缓存中的翻译应用到 ESP 字符串列表并清除缓存
  - 用户拒绝 → 保留缓存文件，可稍后手动恢复
- **Priority**: Should

### FR-8: 单条手动翻译保持
- **Description**: 现有的单条 API 翻译（选中一条 → 点击翻译）继续可用，不受批量模式影响。
- **Acceptance Criteria**:
  - 单条翻译功能与批量翻译可同时进行，互不阻塞
- **Priority**: Must

---

## Non-Functional Requirements

### Performance
| Requirement | Metric | Target |
|-------------|--------|--------|
| 单条翻译延迟 | API 调用耗时 | < 5s (取决于 API 提供商) |
| 批量翻译吞吐 | 并发 N 条/秒 | 受限于 API rate limit |
| UI 响应 | 翻译期间交互延迟 | < 100ms（后台分发到 Web Worker 或 Tauri async） |

### Reliability
| Requirement | Metric | Target |
|-------------|--------|--------|
| 中断恢复 | 重启后已翻译数据完整性 | 100%（逐条实时写入） |
| API 重试 | 临时故障自动重试 | 最多 3 次，指数退避 |

---

## Constraints

### Technical Constraints
- 前端通过 Tauri IPC 调用后端 Rust 命令，不能直接发 HTTP 请求
- 翻译 API Key 仅存内存，不持久化（安全约束保持）
- 使用现有翻译 Provider 接口（OpenAI + DeepL），不引入新的 API 客户端
- ESP 缓存格式保持兼容（bincode + SHA-256 哈希）

### Business Constraints
- 桌面应用，无需服务端部署
- 仅支持已配置 API Key 的 Provider

---

## Assumptions

| Assumption | Risk if Invalid | Mitigation |
|------------|-----------------|------------|
| 用户已配置 API Key | 批量翻译无法执行 | 翻译前检查，无 Key 时提示配置 |
| API 服务可用 | 翻译失败 | 自动重试 + 错误提示 |
| 翻译结果非空（API 返回有效翻译） | 空结果覆盖已有翻译 | 空结果跳过不写入 |

---

## Open Questions

| Question | Owner | Due Date | Resolution |
|----------|-------|----------|------------|
| 批量翻译是否需要翻译记忆/词典去重？ | TBD | TBD | Pending |
| 进度信息持久化（崩溃恢复后知道哪些已完成）？ | TBD | TBD | Resolved: append-only journal 文件 + 启动时检测恢复 |
