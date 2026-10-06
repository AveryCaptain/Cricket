# M0 基线预检与待裁决报告

日期：2026-10-06（Asia/Shanghai）

状态：未完成。按本阶段红线“发现文档内部矛盾 → 停止并在报告中列出，不许自行改契约”，已停止实现，等待契约裁决。没有新增业务代码、workspace、依赖或 CI；没有修改设计文档。

## 上下文与状态自检

已读取 `docs/00-architecture.md` §3、§5（ADR-001 至 ADR-008）、§6；`docs/01-communication.md` 全文；`docs/04-roadmap.md` 全文（含 §2 M0、§4 CI、§5 质量门）；补充核对 `docs/02-domain-modules.md` 的 Agent 字段、消息树与 §2 网关运行策略。

`git log --oneline -8` 的本阶段起点为：

```text
36d297c chore: record phase 0 environment
7f46bc5 docs: AI 接力施工提示词包（Phase 0-9，与里程碑一一对应，幂等可重跑）
abc21cf docs: Cricket 设计基线 v1.0 — 架构拓扑、双端通信契约、领域模块、UI 规格与研发计划
```

本地仓库没有 `Cargo.toml`、`Cargo.lock` 或 `crates/`，不存在可增量补齐的 M0 实现，未运行 `cargo test --workspace`。当前存在此前用户变更：`prompts/` 下 11 个已跟踪文件删除；本阶段保留且不提交这些删除。已建立要求的分支 `phase/m0-baseline`。

## 契约冲突（必须先裁决）

1. **CricketError 无法按原定义完成 serde round-trip。** `docs/01-communication.md:105` 要求 `#[serde(tag = "kind", rename_all = "snake_case")]`，同时第 107–113 行定义 `Network(String)`、`Protocol(String)`、`Auth(String)`、`SessionNotFound(String)`、`Tool(String)`、`Internal(String)`。Serde 内部标签不支持字符串 newtype 内容；本机已安装的 Serde 1.0.229 源码 `src/private/ser.rs:165` 明确对 `serialize_str` 返回 `Unsupported::String`，第 87 行错误模板为 `cannot serialize tagged newtype variant {}::{} containing {}`。这不是不能编译的结论，而是错误变体不能序列化并完成验收要求的 round-trip。需要确定错误线协议：例如保留 newtype 并采用邻接标签 `tag="kind", content="message"`，或改为内部标签加命名字段变体 `Network { message: String }`。这些是候选方案，均未采用。

2. **enum 标签总规则与逐类型定义矛盾。** `docs/01-communication.md:117` 规定“所有 enum 用 serde(tag=…)”，但第 44–50 行的 `Role`、`FinishReason` 和第 99–101 行的 `TaskState` 只有 `rename_all="snake_case"`，会序列化成字符串。需要裁决这些无字段 enum 采用字符串还是带标签对象，以及总规则是否只针对带字段 enum；本阶段不自行更改任何线协议表示。

3. **MessageStart 的父节点字段不一致。** `docs/01-communication.md:67` 定义仅有 `message_id`、`role`；`docs/02-domain-modules.md:66` 明确要求 `MessageStart` 携带 `parent_event_id` 用于事件溯源消息树。需要明确该字段是否属于 v1 契约，以及类型、可空性和缺省规则。

4. **Capability 枚举名称不一致。** `docs/01-communication.md:384` 示例列出 `Reasoning/CacheLog/ToolCall`，而 `docs/02-domain-modules.md:97` 列出 `Reasoning/CachePrompt/ParallelToolCalls/Vision`。需要确定准确能力位以及三家适配器的能力含义；不得默默合并成新的公共契约。

## 必需类型定义缺失

任务要求按 `docs/01` §2 逐类型实现 Agent/Session 相关 Record，但该节只包含 Stage、Role、FinishReason、Usage、CoreEvent、KbScope、TaskInfo、TaskState、CricketError，并称“领域承载类型（节选，全集见 02 文档）”。全文与 `docs/02` 核对后：

- `AgentSpec` 的具体字段仅在 `docs/02-domain-modules.md:19` 定义。
- `AgentBrief`、`SessionSnapshot`、`SessionBrief`、`DocumentInput`、`KbHit` 在 `docs/01` §3 只有方法签名引用，没有 Record 字段、类型或 serde 默认规则。
- `Attachment` 在设计文档中未找到定义。
- `ModelPref` 只有 `primary, fallbacks[]` 描述，没有模型引用的字段类型；`AgentMode`、`RouteMode` 有变体名称描述，但没有序列化定义。

需要提供以上类型的字段、类型、可空性、serde 名称/默认规则，才能证明“契约语义完全一致”。本阶段没有按常见做法猜造 Record。

## 执行范围说明

本阶段明确要求不写 UI、不引入 UniFFI 实际构建，并把 macOS job 留给 Phase 3；因此 `docs/04` 中的 Tauri spike、iOS/macOS CI 和服务端业务不会在本阶段实施。optional UniFFI 依赖由当前任务明确要求；其他白名单外依赖不添加。文档中的 `async_trait` 示例与公开 API 返回值会按当前任务的依赖白名单和 Result 要求处理，无需为已有明确指示再次请求授权。

## 验收状态与测试统计

测试用例：0 个新增、0 个运行；通过数不适用，M0 未通过验收。

| 验收项 | 状态 | 证据/原因 |
|---|---|---|
| `cargo test --workspace`，golden ≥30 | 未运行 | 根 Cargo.toml 不存在；契约预检触发停止条件 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 未运行 | 尚无 workspace |
| `cargo deny check bans licenses` | 未运行 | 尚无 workspace/deny 配置 |
| `cargo check -p cricket-protocol --no-default-features` | 未运行 | 尚无 protocol crate |
| mock fixture CLI 冒烟 | 未运行 | 尚无 CLI 或 golden 语料 |
| Ubuntu/Windows CI | 未创建 | 不能宣称未实现的构建、测试通过 |

## 后续建议

先由用户/架构负责人裁决上述冲突并补齐 Record 定义，再在同一 `phase/m0-baseline` 分支按完整 M0 清单继续。所有验收仍为必需，不因本报告缩小范围。后续构建使用 E: 盘临时目录和 target 目录；P0 记录的本地精确 Rust 工具链目录是指向 stable 的 junction，后续不得把 stable 更新后仍声称为 1.99.0，建议空间允许时安装独立的 1.99.0 工具链并验证。
