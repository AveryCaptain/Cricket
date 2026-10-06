# 01 · Rust-Core 与双端通信设计（UniFFI / Tauri IPC / SSE 线协议）

> 目标读者：端侧与核心工程师。本文是**唯一**的类型契约与传输规范，任何新增跨边界类型必须先改 `cricket-protocol` 再实现。

---

## 1. 通信矩阵总览

| 边界 | 机制 | 数据形态 | 关键约束 |
|------|------|----------|----------|
| Windows UI ↔ Core | Tauri v2 `#[tauri::command]`（下行）+ `tauri::ipc::Channel<CoreEvent>`（上行） | command 参数/返回 serde JSON；Channel 直推 `CoreEvent` | 同进程直调，无网络开销；`Channel` 高频推流 |
| iOS UI ↔ Core | UniFFI 生成的 Swift 协议：`Cricket` 对象方法（async/await）+ `EventSink` 回调接口 | `uniffi::Record/Enum` 映射为 Swift struct/enum | 同进程；回调非主线程，Swift 侧自行 dispatch |
| 客户端 ↔ Server | HTTPS REST（控制面）+ SSE（数据面） | 线协议 = `CoreEvent` serde tagged JSON | `Last-Event-ID` 断线重放；PAT Bearer 认证 |
| Core ↔ LLM 上游 | relay：服务端代发；direct：Core 直发 | 厂商原生 SSE → `GatewayEvent` 归一化 | 厂商差异全部终结在 `cricket-gateway` |

**核心不变量**：三种 transport（Channel / EventSink / SSE wire）传输的都是同一 `CoreEvent` 的序列化，不允许任何 transport 出现私有事件类型。

---

## 2. 统一消息契约 `cricket-protocol`（核心类型全集）

```rust
//! crates/cricket-protocol/src/lib.rs —— 全端唯一契约（编译期同时生成 serde 与 uniffi 双绑定）

use serde::{Deserialize, Serialize};

/// ══════ 阶段机：驱动端侧"阶段文字 + 波浪点 + 计时器"等待动画 ══════
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, uniffi::Enum)]
#[serde(tag = "stage", rename_all = "snake_case")]
pub enum Stage {
    Connecting,               // "连接模型网关…"
    Planning,                 // "规划回复结构…"
    RetrievingKnowledge,      // "检索知识库…"
    Reasoning,                // "深度思考中…"
    CallingTool { name: String }, // "调用工具 {name}…"
    Executing,                // "执行任务…"（代码模式）
    Verifying,                // "验证执行结果…"（代码模式）
    Streaming,                // "生成中…"
    Done,
    Aborted,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
#[serde(rename_all = "snake_case")]
pub enum Role { System, User, Assistant, Tool }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason { Stop, Length, ToolCalls, ContentFilter, Error, Aborted }

#[derive(Debug, Clone, Default, Serialize, Deserialize, uniffi::Record)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_tokens: u64,     // Anthropic cache_read / OpenAI cached_tokens
    pub reasoning_tokens: u64,
}

/// ══════ 核心事件流：一切流式交互的原子单位 ══════
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Enum)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CoreEvent {
    SessionStarted { session_id: String },
    /// 阶段切换（等待动画的文字与形态由此驱动；每次仅且必须 emit 一次）
    StageChanged { stage: Stage },
    MessageStart { message_id: String, role: Role },
    /// Reasoning/Thought 通道：与 TextDelta 严格分离传输
    ReasoningDelta { message_id: String, text: String },
    TextDelta     { message_id: String, text: String },
    ToolCallStart { message_id: String, call_id: String, name: String },
    ToolCallArgs  { call_id: String, args_json_delta: String },  // 增量 JSON 片段
    ToolCallEnd   { call_id: String, args_json: String },
    ToolResult    { call_id: String, ok: bool, duration_ms: u64, preview: String },
    MessageEnd    { message_id: String, finish: FinishReason, usage: Usage },
    /// 代码模式：Goal-DAG 任务状态推送（见 02 文档）
    TaskUpdated  { task: TaskInfo },
    /// 知识库变更广播（多端同步）
    KbChanged    { scope: KbScope },
    Error        { message: String, retryable: bool, provider: Option<String> },
    StreamClosed,
}

/// ══════ 领域承载类型（节选，全集见 02 文档）══════
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Enum)]
#[serde(tag = "scope", rename_all = "snake_case")]
pub enum KbScope { Global, Agent { agent_id: String } }

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct TaskInfo {
    pub goal_id: String,
    pub task_id: String,
    pub title: String,
    pub state: TaskState,
    pub attempt: u8,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
#[serde(rename_all = "snake_case")]
pub enum TaskState { Pending, Ready, Running, Verifying, Repair, Blocked, Failed, Done, Skipped }

/// ══════ FFI/IPC 统一错误 ══════
#[derive(Debug, thiserror::Error, Serialize, Deserialize, uniffi::Error)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CricketError {
    #[error("网络错误: {0}")]  Network(String),
    #[error("协议错误: {0}")]  Protocol(String),
    #[error("鉴权失败: {0}")]  Auth(String),
    #[error("会话不存在: {0}")] SessionNotFound(String),
    #[error("工具执行失败: {0}")] Tool(String),
    #[error("已取消")]         Cancelled,
    #[error("内部错误: {0}")]  Internal(String),
}
```

> **规则**：① 所有 enum 用 `serde(tag=…)`，保证线协议自描述、向前兼容靠"只增不改"；② uniffi derive 与 serde derive 必须出现在同一类型上（ADR-004）；③ `uniffi::Enum` 带字段的变体会生成 Swift 关联值 enum / JS 端经 JSON 透传。

---

## 3. UniFFI 导出规范（iOS）

### 3.1 导出面（cricket-ffi）

```rust
//! crates/cricket-ffi/src/lib.rs（仅 feature = "ffi" 时编译 uniffi derive）
use crate::protocol::*;
use std::sync::Arc;

#[derive(uniffi::Object)]
pub struct Cricket {
    core: Arc<cricket_agent::CoreHandle>,
}

/// 事件回调：Rust → Swift。fire-and-forget，不允许返回值（UniFFI 回调接口限制）。
#[uniffi::export(callback_interface)]
pub trait EventSink: Send + Sync {
    fn on_event(&self, event: CoreEvent);
}

#[derive(uniffi::Record)]
pub struct CoreConfig {
    pub server_url: String,
    pub auth_token: String,
    pub route_mode: RouteMode,          // Relay | Direct
    pub data_dir: String,               // iOS 传 Application Support 路径
    pub locale: String,
}

#[uniffi::export(async_runtime = "tokio")]
impl Cricket {
    #[uniffi::constructor]
    pub fn new(config: CoreConfig, sink: Box<dyn EventSink>) -> Result<Self, CricketError> {
        // catch_unwind 全覆盖（ADR-001）；sink 包一层 EmitterSink 接入统一扇出
    }

    // ── Agent / Session ──────────────────────────────────────
    pub async fn agent_create(&self, spec: AgentSpec) -> Result<String, CricketError>;
    pub async fn agent_list(&self) -> Result<Vec<AgentBrief>, CricketError>;
    pub async fn session_open(&self, agent_id: String, title: Option<String>)
        -> Result<SessionSnapshot, CricketError>;
    pub async fn session_list(&self, agent_id: String) -> Result<Vec<SessionBrief>, CricketError>;
    pub async fn chat_send(&self, session_id: String, text: String) -> Result<(), CricketError>;
    pub async fn regenerate(&self, session_id: String, message_id: String) -> Result<(), CricketError>;
    pub async fn abort(&self, session_id: String) -> Result<(), CricketError>;

    // ── 知识库 ────────────────────────────────────────────────
    pub async fn kb_upsert(&self, scope: KbScope, doc: DocumentInput) -> Result<String, CricketError>;
    pub async fn kb_delete(&self, scope: KbScope, doc_id: String) -> Result<(), CricketError>;
    pub async fn kb_search(&self, scope: KbScope, query: String, top_k: u8) -> Result<Vec<KbHit>, CricketError>;

    // ── 工具 / Skill ──────────────────────────────────────────
    pub async fn skill_register(&self, manifest_toml: String) -> Result<String, CricketError>;
    pub async fn skill_invoke(&self, skill_id: String, args_json: String) -> Result<String, CricketError>;

    // ── 特化模式 ─────────────────────────────────────────────
    pub async fn goal_submit(&self, session_id: String, objective: String) -> Result<String, CricketError>;
    pub async fn goal_abort(&self, goal_id: String) -> Result<(), CricketError>;
    pub async fn novel_graph_query(&self, agent_id: String, q: String) -> Result<Vec<WorldNodeBrief>, CricketError>;
}
```

### 3.2 FFI 安全规范（硬性）

| 规则 | 说明 |
|------|------|
| R1 类型边界 | 跨 FFI 只传 `String/整数/布尔/Vec/Record/Enum`；任何 `&ref`、trait 对象句柄之外的 Rust 结构禁止外泄 |
| R2 句柄 | 面向对象用 `Arc` 包裹导出，UniFFI 句柄表管理生命周期；Swift 侧无需手动释放 |
| R3 panic 布防 | 所有导出方法体 `catch_unwind`，panic → `CricketError::Internal`；**禁止** `unwrap/expect` 出现在导出路径 |
| R4 回调线程 | `EventSink.on_event` 从 Emitter 专用线程调用（非主线程）；Swift 实现内 `DispatchQueue.main.async` 切换（模板代码由生成器附带的 `CricketEventDispatcher` 提供） |
| R5 回调频率 | Emitter 已做 16ms 合帧，Swift 回调频率 ≤ 60/s；禁止在回调中做重活 |
| R6 数据所有权 | 回调传入的 event 为副本，Swift 持有任意时长无 UAF 风险（uniffi 序列化边界保证） |
| R7 async 覆盖 | `#[uniffi::export(async_runtime = "tokio")]` 启用 async；方法内禁止阻塞式 IO |
| R8 版本 | 锁定 uniffi 版本并提交 `swift-generated` 基线进仓库；升级 uniffi 视为 breaking change，需双端同步 |

### 3.3 构建流水线（CI 固化脚本）

```bash
# ① 三目标编译静态库（feature ffi 关闭无关模块）
for T in aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios-sim; do
  cargo build -p cricket-ffi --release --features ffi --target $T
done

# ② 生成 Swift 绑定（proc-macro 模式，从库中反推 UDL 等价物）
uniffi-bindgen generate \
  --library target/aarch64-apple-ios/release/libcricket_ffi.a \
  --language swift --config cricket-ffi/uniffi.toml --out-dir bindings/swift

# ③ 模拟器双架构 lipo 合一
lipo-create \
  target/aarch64-apple-ios-sim/release/libcricket_ffi.a \
  target/x86_64-apple-ios-sim/release/libcricket_ffi.a \
  -create -output libcricket_ffi_sim.a

# ④ 组装 xcframework
xcodebuild -create-xcframework \
  -library target/aarch64-apple-ios/release/libcricket_ffi.a -headers bindings/swift \
  -library libcricket_ffi_sim.a -headers bindings/swift \
  -output CricketCore.xcframework
```

产物以 **Swift Package（binaryTarget）** 形式接入 Xcode 工程，`Package.swift` 声明 `.binaryTarget(name: "CricketCore", path: "CricketCore.xcframework")`。

### 3.4 Swift 侧消费范式

```swift
// 桥接器：回调 → AsyncStream（SwiftUI 专用）
final class UiEventSink: EventSink {
    let stream: AsyncStream<CoreEvent>
    private let cont: AsyncStream<CoreEvent>.Continuation
    init() { (stream, cont) = AsyncStream.makeStream(); }
    func onEvent(event: CoreEvent) {
        cont.yield(event)                      // 值拷贝，线程安全
    }
}

@MainActor final class SessionModel: ObservableObject {
    @Published var messages: [MessageVM] = []
    @Published var stage: Stage?              // 驱动等待动画
    @Published var elapsedMs: Int = 0
    private var cricket: Cricket?
    private let sink = UiEventSink()

    func start() async throws {
        cricket = try Cricket(config: cfg, sink: sink)
        for await ev in sink.stream { consume(ev) }   // MainActor 消费
    }
    private func consume(_ ev: CoreEvent) { /* 纯状态机：映射到 messages/stage/elapsed */ }
}
```

---

## 4. Windows / Tauri v2 绑定规范

### 4.1 command 面（与 §3.1 UniFFI 方法一一对应）

```rust
//! apps/desktop/src/commands.rs —— 方法名/参数与 UniFFI 严格同构（review checklist 项）
use tauri::ipc::Channel;
use cricket_protocol::{CoreEvent, SessionSnapshot, CricketError};

#[tauri::command]
async fn session_open(
    state: tauri::State<'_, CoreState>,
    agent_id: String,
    title: Option<String>,
    on_event: Channel<CoreEvent>,          // Tauri v2 IPC 通道
) -> Result<SessionSnapshot, String> {
    state.core.session_open(agent_id, title, on_event.into()).await
        .map_err(|e: CricketError| e.to_string())
}

#[tauri::command]
async fn chat_send(state: tauri::State<'_, CoreState>, session_id: String, text: String)
    -> Result<(), String> { /* … */ }

#[tauri::command]
async fn abort(state: tauri::State<'_, CoreState>, session_id: String) -> Result<(), String> { /* … */ }
// … kb_* / skill_* / goal_* / novel_* 同构映射
```

- `Channel<CoreEvent>` 需要 `CoreEvent: Serialize`（已满足）；Core 内部将 `Channel` 适配为 Emitter 的一个 sink（与 EventSink、SSE writer 同一 trait）。
- **JS 侧**：`const ch = new Channel(); ch.onmessage = dispatch; invoke('session_open', { agentId, title, onEvent: ch })`。

### 4.2 事件节流与渲染契约

| 项 | 规格 |
|----|------|
| 合帧 | Core Emitter 16ms 窗口合并 TextDelta/ReasoningDelta（ADR-008），端侧零节流代码 |
| 计时器 | 端侧本地 `performance.now()` 计时（**不依赖**网络事件到达频率），100ms 节流刷新（见 03 文档 §5） |
| 阶段文字 | 由 `StageChanged` 驱动，与网络无关地保持上次值直至新阶段到达 |
| 取消 | `Esc` → invoke `abort` → 期待 `StageChanged{Aborted}` + `StreamClosed`；300ms 超时则 UI 标记"网络滞留"并允许强制丢弃 |
| 错误 | `Error{retryable:true}` → UI 显示重试按钮（重试=对最后一条用户消息 `regenerate`） |

### 4.3 Windows 前端架构约束（保障 < 50MB）

- **无虚拟 DOM 重框架**：原生 ES Modules + `preact`（≤ 4KB gz）或纯手写 store；禁止 React/Vue 全家桶。
- 消息列表虚拟滚动（自研 200 行 IntersectionObserver 窗口化，见 03 文档 §7）。
- 全部静态资产随包分发，运行时零 CDN 请求；字体子集化（JetBrains Mono latin+punct 子集，woff2 ≤ 90KB）。

---

## 5. SSE 线协议（客户端 ↔ Cricket-Server）

### 5.1 路由表

| Method & Path | 用途 | 响应 |
|---|---|---|
| `POST /api/v1/chat/relay` | LLM 网关转发（流式） | SSE（`GatewayEvent`→`CoreEvent` 帧） |
| `GET  /api/v1/sessions/{id}/events?since=<eid>` | 会话事件重放 | SSE（断线续传） |
| `POST /api/v1/sessions` / `GET …` | 会话 CRUD / 列表 | JSON |
| `POST /api/v1/agents` … | 智能体 CRUD | JSON |
| `POST /api/v1/kb/{scope}/documents` | 知识库增删改查 | JSON |
| `POST /api/v1/kb/search` | 混合检索 | JSON |
| `POST /api/v1/tools/web_search` · `/tools/crawl` | 工具代理 | JSON |
| `POST /api/v1/skills/{id}/invoke` | 远程 Skill | JSON / SSE |
| `POST /api/v1/sync/push` · `GET /api/v1/sync/pull` | 会话镜像同步 | JSON |
| `GET  /healthz` | 存活探针 | 204 |

认证：`Authorization: Bearer <PAT>`（服务端签发的设备级长令牌 + 可选短期 access token 刷新，全链 TLS）。

### 5.2 SSE 帧格式（线协议 = CoreEvent 序列化，ADR-004）

```
: keep-alive 心跳（15s 间隔，Caddy/nginx 禁用自有缓冲）

id: 1042                        ← 全局单调事件号，断线重连凭据
event: stage_changed
data: {"type":"stage_changed","stage":{"stage":"retrieving_knowledge"}}

id: 1043
event: reasoning_delta
data: {"type":"reasoning_delta","message_id":"m_7","text":"先分析用户意图…"}

id: 1044
event: text_delta
data: {"type":"text_delta","message_id":"m_7","text":"鸣蛩"}

id: 1045
event: tool_call_start
data: {"type":"tool_call_start","message_id":"m_7","call_id":"c_1","name":"kb_search"}
```

**断线重连**：客户端带 `Last-Event-ID: 1045` 重连 `/sessions/{id}/events`；服务端为每条流维护 `ring buffer`（≥ 2048 事件 + 30 分钟 TTL）重放补齐，保证零丢失（SLO 表）。

### 5.3 服务端实现要点（cricket-server）

```rust
// axum SSE：每连接一个 task；事件号来自服务端 per-session 单调序列
use axum::response::sse::{Sse, Event as SseEvent, KeepAlive};

async fn chat_relay(State(st): State<AppState>, claims: Claims,
                    Json(req): Json<RelayRequest>) -> Sse<impl Stream<Item = Result<SseEvent, Infallible>>> {
    let upstream = st.gateway.stream(req, claims).await;   // GatewayEvent 流
    let numbered = SseNumberer::new(st.event_log(req.session_id.clone()), upstream);
    Sse::new(numbered).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}
```

---

## 6. 上游厂商差异终结层（cricket-gateway）

内部归一化事件 `GatewayEvent` 与 `CoreEvent` 中流式部分同构，**厂商差异到此为止**：

| 内部事件 | OpenAI（Chat Completions 流） | Anthropic（Messages 流） | Gemini（streamGenerateContent, alt=sse） |
|---|---|---|---|
| `TextDelta` | `delta.content` | `content_block_delta` (text_delta) | `candidates[0].content.parts[].text`（非 thought） |
| `ReasoningDelta` | `delta.reasoning_content`（R1 兼容族）/ Responses API reasoning 摘要 | `content_block_delta` (thinking_delta)，需开启 `thinking.budget_tokens` | `parts[].text` 且 `thought:true`（thinkingConfig includeThoughts） |
| `ToolCallStart` | `delta.tool_calls[i]` 首帧（携带 id/name/index） | `content_block_start` (type=tool_use) | `functionCall` part |
| `ToolCallArgs` | `delta.tool_calls[i].arguments` 增量字符串 | `input_json_delta` 增量 | `functionCall.args` 整帧对象 |
| `Usage` | `usage`（需 `stream_options.include_usage`） | `message_delta.usage` + `message_start` 输入数 | `usageMetadata` |
| `Finish` | `finish_reason` | `stop_reason` | `finishReason` |

适配器 trait（三家实现 + 预留 `openai_compatible` 一网打尽兼容厂商）：

```rust
#[async_trait::async_trait]
pub trait ProviderAdapter: Send + Sync {
    fn id(&self) -> &'static str;
    async fn stream(&self, req: NormalizedRequest) -> Result<GatewayStream, CricketError>;
    fn normalize_delta(&self, raw: RawSseFrame) -> Vec<GatewayEvent>;  // 差异终结点
    fn supports(&self, cap: Capability) -> bool;                        // Reasoning/CacheLog/ToolCall 能力位
}
```

**回归测试**：`fixtures/golden/` 存放三家真实 SSE 抓包（每家 ≥ 10 条：纯文本/思考链/工具调用/中断/超长），CI 离线回放断言归一化输出 → 上游接口变更时只需补录语料，不发版先红。

---

## 7. 版本与兼容策略

| 机制 | 规格 |
|------|------|
| 协议版本 | `X-Cricket-Protocol: 1`；服务端只增不改字段（serde 默认值容忍缺字段） |
| FFI 版本 | `Cricket.buildInfo()` 返回 git hash + protocol 版本；App 与 Server 协议差大版本时引导升级 |
| 灰度 | 服务端 `/api/v1/version` 返回最低兼容端版本；端侧 < 最低版本 → 强制升级页 |
| 变更纪律 | 新增事件类型 = minor；语义修改 = major + 双端同步发版（Windows 静默更，iOS 走审核） |

---

## 8. 评审清单（新增跨边界能力时的 review gate）

- [ ] 类型已进 `cricket-protocol`，serde + uniffi 双 derive 同体？
- [ ] 线协议 / FFI / Tauri 三 transport 均可无损承载？（无 transport 私有字段）
- [ ] 事件在 Emitter 合帧规则下不产生语义歧义？（Delta 类合并；Start/End/State 类直通）
- [ ] 取消语义覆盖？（abort 后所有下游请求随 CancellationToken 终止）
- [ ] golden 语料已补录？
- [ ] 三端（Win/iOS/Server）最小实现或 No-op 占位均已交付？
