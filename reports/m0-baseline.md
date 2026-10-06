# M0 基线交付与验收报告

日期：2026-10-06（Asia/Shanghai）

状态：M0 实现及六项验收均已通过。代码提交 `58cd973` 的 Actions run `37436211765` 已完成，`core` 与 `windows` 两个 job 均为 `success`。下面的预检冲突记录保留为历史，已由用户裁决解除，不再阻塞 M0。

## 交付摘要

- Cargo workspace 共 11 个成员，保留文档规定的单向依赖与 release profile；其余领域 crate、FFI 和 server 仅职责骨架。
- `cricket-protocol` 实现全部任务类型，以及裁决补充的 ModelRef、ProviderId、SessionState、AttachmentKind；每个类型均有完整 JSON/字段 round-trip 测试。
- 最新用户裁决优先：Stage 保留 `tag="stage"` 与 CallingTool.name；KbScope 使用 `tag="scope"`；CoreEvent 使用 `tag="type"`，MessageStart 含可选 parent_event_id；所有 CricketError 变体含 String，使用邻接 `tag="kind", content="message"`；unit enum 输出 snake_case 字符串。
- AgentSpec 采用用户新增的 `reports/m0-contract-ruling.md` 字段；Option 缺省为 None 并省略序列化、Vec 缺省为空且保留空数组、ModelPref.use_cache 缺省为 true；温度默认 0.7、max_output_tokens 默认 4096。
- UniFFI 遗留处理后由 Cargo.lock 锁定为 0.32.2，仅 optional/ffi feature；补齐受 feature 门控的 `setup_scaffolding!`，全 feature Rust 检查及测试已通过。未生成 Swift/iOS 构建产物；默认依赖图仍无 UniFFI。
- OpenAI、Anthropic、Gemini、OpenAI-compatible 适配器；reqwest 默认功能关闭且启用 rustls，原生 SSE 由 eventsource-stream 解析；工具调用聚合、Reasoning 分流、usage 快照与最终账本均已实现。
- 连接前 429/5xx/网络错误最多重试 3 次（初次请求另计），250ms 指数退避加 0–100ms jitter；按 ModelPref 降级；流开始后错误归一化为 retryable Error，终止流且不重发。
- 30 个手工构造原生 SSE 文件和逐事件期望 JSON，每家 10 个；另有 OpenAI-compatible 回放用例，合计 golden 测试 31 个。测试通过 7-byte 分块解析，覆盖 UTF-8 分块、Reasoning、工具参数增量/整帧、多工具、中断、长流、usage 和结束原因。
- Emitter 单一扇出接口与 16ms 定时合帧；控制事件先冲刷 delta，再直通；sink panic 转 CricketError::Internal。
- clap CLI mock → gateway → Emitter → stdout JSON 行；真实调用须显式 --live 和 --model，凭据只读取环境变量，CI 无 live 调用。
- Ubuntu CI 含 fmt、clippy、workspace tests、无 FFI protocol check、完整 Windows MSVC target check、cargo-deny 和 CLI smoke；Windows CI 含 workspace build 与 CLI smoke。macOS/UniFFI 基线留给 Phase 3。

## 最新测试统计及命令证据

共 82 个非空测试：protocol 34、gateway 策略 10、golden 31、Emitter 4、CLI 集成 3；全部通过，0 失败。

| 验收 | 结果 | 输出证据 |
|---|---|---|
| `cargo test --workspace --offline --locked` | 通过 | `34 passed; 0 failed`、`10 passed; 0 failed`、`31 passed; 0 failed`、`4 passed; 0 failed`、`3 passed; 0 failed` |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 通过 | `Finished dev profile`，零告警 |
| `cargo fmt --all -- --check` | 通过 | 退出码 0，无格式差异 |
| `cargo deny check bans licenses` | 通过 | `bans ok, licenses ok`；offline 重跑结果相同 |
| `cargo check -p cricket-protocol --no-default-features --locked` | 通过 | `Finished dev profile`，退出码 0 |
| `cargo run -p cricket-cli --locked -- chat --provider mock --fixture fixtures/golden/openai/tool.sse` | 通过 | 打印 message_start、tool_call_start/args/end、message_end 和 stream_closed |
| `cargo run -p cricket-cli --locked -- chat --provider mock --fixture fixtures/golden/gemini/reasoning.sse` | 通过 | reasoning_delta 与 text_delta 分离，message_end usage 为 input=11/output=17/cached=3/reasoning=5 |
| `cargo build --workspace --locked` | Windows 本机通过 | `Finished dev profile`，退出码 0 |
| `cargo check --workspace --target x86_64-pc-windows-msvc --locked` | Windows 本机与 Ubuntu CI 通过 | Ubuntu job 的 Windows type and dependency check 步骤成功 |
| CI `core` + `windows` | 通过，两个 job 均 completed/success | https://github.com/AveryCaptain/Cricket/actions/runs/37436211765 ，head SHA：58cd973213e64b0732bf591aaa8ad511b4657c76 |

补充 `cargo audit`：M0 初始锁文件曾包含 optional UniFFI 链上的 bincode 1.3.3（RUSTSEC-2025-0141）与 paste 1.0.15（RUSTSEC-2024-0436）。用户要求处理遗留后已升级 UniFFI 至 0.32.2，两个包均已移出锁文件，没有屏蔽 advisory。

## 遗留处理（2026-10-06）

- UniFFI 0.32.2 升级及 scaffolding 标记修复已提交；默认 feature 测试与全 feature 测试均为 82/82，JSON round-trip 不变。
- `cargo check --workspace --all-features --locked` 通过；`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` 通过。
- `cargo deny check bans licenses` 扩大为完整 feature 图，结果 `bans ok, licenses ok`，无告警；仅对已选定的六个 UniFFI 框架包精确允许 MPL-2.0，不给其他包放宽许可证范围。
- 本机 cargo-audit 的内置 HTTP 更新曾遇到 GitHub 网络错误；系统 Git `fetch origin` 成功，数据库 HEAD/FETCH_HEAD 均为 `ef6173cbc5c50ec8166f9a5b28f07834144373ee`（2026-10-03）。`cargo audit --no-fetch --deny warnings` 退出码 0，零漏洞、零告警。CI 新增不带 no-fetch 的在线严格审计，独立验证数据库刷新。
- CI 新增 optional UniFFI Rust 兼容性检查、全 feature 许可检查和 `cargo audit --deny warnings`。未新增 Swift 生成或 macOS 构建，Phase 3 双端产物验证仍需 macOS/Xcode。
- `prompts/` 的删除属于用户既有工作树变更；已询问是否恢复，在收到确认前保留现状。没有把这些删除纳入提交。
- docs/ 与 README 继续保持原样；契约裁决以 `reports/m0-contract-ruling.md` 和用户最新 Stage/KbScope/默认值裁决为准。同步修改受此前“不修改设计文档”约束，未擅自执行。

## 边界与后续事项

- 没有修改 docs/、prompts/、.gitattributes 或 README.md；工作树此前已有 prompts 的 11 个删除仍保留且未提交。
- 直接第三方依赖均在白名单内，UniFFI 是本阶段明确要求的 optional 例外；所有版本锁入 Cargo.lock，无其他依赖引入。私有 publish=false workspace 的许可证声明未擅自选定；deny 仍检查全部第三方依赖许可，允许本地 path wildcard。
- 导出/公开方法返回 Result 或携带 Result 的异步 future，生产路径没有 `.unwrap()`/`.expect()`；测试可使用失败即 panic 的断言。Default/serde 等标准派生 trait 按其既有签名实现。
- Linux 的 MSVC check 使用 LLVM clang-cl、freestanding 内建函数及 ring 提供的 RING_CORE_NOSTDLIBINC 路径，生成真实目标对象；它是 check 门，不替代原生 Windows 编译/链接 smoke。没有新增 SDK 或 Cargo 第三方依赖。
- SSE 文件结尾的空行是原生事件分隔符，不能按普通 Markdown 格式清理；未改 .gitattributes。
- 未调用真实厂商服务，未写入任何凭据；mock CLI 与回放测试全部离线，策略集成测试只访问 127.0.0.1 临时端口。
- 文档同步修订和 Phase 3 UniFFI 实际构建留给后续阶段；本报告和用户裁决作为 M0 实现依据。
- 分支 `phase/m0-baseline`，代码和每个任务单元的 conventional commits 均已推送；最终报告与用户裁决文件同分支提交。代码核验对应上述 head SHA，报告提交不改变 Rust、依赖或 CI。

## 历史预检

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

## 历史阻塞时的验收状态

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
