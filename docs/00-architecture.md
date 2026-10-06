# 00 · 系统整体拓扑与跨端架构

> 目标读者：全体工程成员。本文定义系统边界、进程视图、Cargo Workspace 拆解、四条关键数据流与全部顶层架构决策（ADR）。

---

## 1. 系统拓扑总图

```
┌────────────────────────────────┐   ┌────────────────────────────────┐
│   Windows 客户端 (Tauri v2)      │   │   iOS 客户端 (Swift + SwiftUI)   │
│ ┌────────────────────────────┐ │   │ ┌────────────────────────────┐ │
│ │ WebView2 · Codex 风格 UI    │ │   │ │ SwiftUI 原生渲染层          │ │
│ │ (HTML/CSS/JS, 极简 DOM)     │ │   │ │ (无 Webview)               │ │
│ └──────────▲─────────────────┘ │   │ └──────────▲─────────────────┘ │
│            │ Tauri IPC           │   │            │ UniFFI Swift 绑定  │
│            │ #[command] +       │   │            │ protocol EventSink │
│            │ Channel<CoreEvent> │   │            │ (async/await)      │
│ ┌──────────┴─────────────────┐ │   │ ┌──────────┴─────────────────┐ │
│ │ cricket-app 宿主 (Rust bin) │ │   │ │ CricketCore.xcframework    │ │
│ │  └─ cricket-core 静态链接   │ │   │ │  └─ Rust staticlib (arm64) │ │
│ │    (workspace crates)      │ │   │ │     = 同一套 cricket-core  │ │
│ └────────────────────────────┘ │   │ └────────────────────────────┘ │
└──────────┬─────────────────────┘   └──────────┬─────────────────────┘
           │        HTTPS  REST + SSE(可断线重连) │
           └──────────────┬───────────────────────┘
                          ▼
   ┌─────────────────────────────────────────────────────────┐
   │         Cricket-Server @ 124.223.154.233                │
   │  Caddy (TLS 443, 自动续期, 反代)                          │
   │  └─ cricket-server (Axum + Tokio + Tower)               │
   │      ├ /api/v1/chat/relay      多厂商 SSE 网关转发        │
   │      ├ /api/v1/events          服务端事件推送 (SSE)       │
   │      ├ /api/v1/agents|sessions REST：智能体/会话/同步     │
   │      ├ /api/v1/kb/*            知识库 CRUD + 混合检索    │
   │      ├ /api/v1/tools/*         Web Search / Crawl 代理  │
   │      └ /api/v1/skills/*        远程 Skill 执行代理       │
   │  ├─ PostgreSQL 16 + pgvector + zhparser (正文/向量/图谱)  │
   │  ├─ SearXNG (自托管搜索元引擎, JSON API)                  │
   │  └─ Crawler (scraper + readability 正文抽取)             │
   └────────────────────────┬────────────────────────────────┘
                            │ HTTPS (上游, relay 模式)
            ┌───────────────┼────────────────┐
            ▼               ▼                ▼
       ┌─────────┐     ┌───────────┐    ┌─────────┐
       │  OpenAI │     │ Anthropic │    │ Gemini  │
       └─────────┘     └───────────┘    └─────────┘
```

**两条 LLM 链路**（详见 ADR-002）：
- `relay`（默认）：客户端 → Cricket-Server → 厂商。密钥集中托管、可审计/限流/缓存。
- `direct`（可选）：客户端 → 厂商直连。用于自备密钥、追求极限延迟；需客户端可直连厂商的网络环境。

---

## 2. 进程与体积视图

| 端 | 进程构成 | 预算 |
|----|---------|------|
| Windows | `cricket.exe`（Tauri 宿主 + Core，静态链接）＋ WebView2 渲染子进程（系统共享运行时） | 宿主 < 50 MB RSS；WebView2 ≤ 70 MB（DOM 严格预算控制） |
| iOS | 单进程：App 二进制内嵌 `CricketCore` 静态库 | 全程 < 60 MB（SwiftUI 列表窗口化） |
| Server | Caddy + cricket-server + PostgreSQL + SearXNG（四容器） | 2C4G 基线即可运行；256 并发 SSE |

**体积控制手段（写进 Cargo profile 与 CI 门禁）**：
- 全链 `rustls`，禁止 `native-tls`/`openssl` 依赖入图（`cargo-deny` ban list）。
- `[profile.release] opt-level = "s"`、`lto = "thin"`、`codegen-units = 1`、`strip = true`。
- feature 严格门控：`cricket-novel` / `cricket-code` / `ffi` / `server` 互不拖拽默认依赖。
- CI 产物体积回归门禁：Windows 安装包 > 15 MB、iOS framework > 18 MB 即红灯。

---

## 3. Cargo Workspace 拆解

```
cricket/
├── Cargo.toml                # workspace 根
├── crates/
│   ├── cricket-protocol/     # ★ 全端唯一消息契约（serde + uniffi 双导出，零逻辑）
│   ├── cricket-gateway/      # 多厂商 LLM 网关：3 家适配器 + SSE 解析 + 事件归一化
│   ├── cricket-agent/        # 智能体运行时：会话 Actor、工具循环、消息树、上下文组装
│   ├── cricket-memory/       # 分层知识库客户端：chunking、本地缓存、混合检索 API
│   ├── cricket-tools/       # 工具注册表、JSON Schema 校验、执行沙盒、内置工具
│   ├── cricket-skills/      # Skill manifest(TOML)、本地编排、远程 Skill 签名
│   ├── cricket-novel/        # 小说模式：世界观图谱、人物卡、一致性校验、润色管线
│   ├── cricket-code/         # 代码模式：Goal-DAG、验证环、git2 封装
│   ├── cricket-ffi/          # UniFFI 门面层（feature "ffi"，仅 iOS 构建激活）
│   ├── cricket-server/       # Axum 服务端（bin）+ REST/SSE 路由 + sqlx
│   └── cricket-cli/          # 冒烟与 E2E 驱动器（bin，CI 必用）
├── apps/
│   ├── desktop/              # Tauri v2 Windows 客户端（Rust 宿主 + 前端静态资产）
│   └── ios/                  # Xcode 工程 + Swift Package（CricketCore 绑定）
├── fixtures/golden/           # ★ 三厂商 SSE 录制语料（golden corpus，CI 回放）
└── deploy/                    # docker-compose / Caddyfile / 备份脚本 / 加固清单
```

### 职责与依赖矩阵

| crate | 职责一句话 | 依赖（上层→下层） | 关键三方库 |
|-------|-----------|------------------|------------|
| `cricket-protocol` | 事件/类型/错误契约，三端唯一真源 | serde, uniffi(feature), thiserror | — |
| `cricket-gateway` | 上游协议差异→归一化 `GatewayEvent` | → protocol | reqwest(rustls), eventsource-stream, futures |
| `cricket-agent` | 会话循环：组装→调用→工具→落盘 | → gateway, memory, tools, skills, protocol | tokio, tokio-util(CancellationToken), petgraph |
| `cricket-memory` | KB 客户端（面向服务端 API）+ 本地缓存 | → protocol | rusqlite(FTS5), reqwest |
| `cricket-tools` | 工具注册/校验/执行；web_search、crawl、kb_search、fs、git 门面 | → protocol, memory | jsonschema, reqwest |
| `cricket-skills` | 开放式 Skill 注册（本地编排 + 远程代理） | → tools, protocol | toml, hmac-sha256 |
| `cricket-novel` | 世界观图谱/人物卡/一致性/润色（纯逻辑，无 UI） | → memory, agent, protocol | — |
| `cricket-code` | Goal-DAG/验证环/git2 封装（纯逻辑） | → agent, tools, protocol | git2, petgraph, rusqlite |
| `cricket-ffi` | `#[derive(uniffi)]` 门面：Object/方法/回调/异步 | → agent 及以下全部 | uniffi |
| `cricket-server` | REST/SSE/代理/持久化（私有服务器） | → protocol, gateway | axum, tower, sqlx(postgres), pgvector |
| `cricket-cli` | 无 UI 冒烟：直驱 Core 完成全链路 | → ffi 之外的全部 | clap |

> 约束：依赖只能自上而下，`protocol` 不依赖任何 crate；测试用 `fixtures/golden` 的厂商 SSE 录制语料回放，CI 不依赖外网。

---

## 4. 关键数据流

### Flow A — 用户发送消息（Windows · relay 模式）

```
Composer ⏎
  │ tauri invoke("chat_send")
  ▼
cricket-core: SessionActor(mpsc) 收到 Chat 指令
  1. 上下文组装：system(agent 模板) + KB 检索包 + 世界观/人物包 + 历史消息树折叠
  2. POST /api/v1/chat/relay (SSE)
  3. cricket-server: ProviderAdapter → 上游厂商 SSE → 解析为 GatewayEvent 归一化帧
  4. Core: Emitter（16ms 合帧）扇出 CoreEvent
       ├─ Windows: tauri Channel<CoreEvent> → WebView 增量渲染（Wave Dots 驱动）
       ├─ iOS:     EventSink 回调 → Swift AsyncStream → SwiftUI
       └─ Server:  会话事件日志追加（供断线重连重放）
  5. MessageEnd → 本地 SQLite 事件落盘(事件溯源) + POST /sync/push 摘要上行
```

### Flow B — 工具调用（Function Calling 循环）

```
GatewayEvent::ToolCallStart(name,args)
  ▼
cricket-tools Registry：JSON Schema 校验 → 路由执行
  ├─ 本地工具：git2 / fs / 本地 skill（沙盒白名单内）
  └─ 远程工具：POST /api/v1/tools/web_search | crawl → 服务端代理（限流/缓存）
        ▼ ToolResult(preview ≤ 3 行) → 注入会话 → 二次调用 LLM → 继续 Flow A 步骤 3
```

### Flow C — 分层知识库检索（RAG）

```
用户输入 / Agent 决策触发 kb_search
  ▼ POST /api/v1/kb/search { scope: global|agent:<id>, query, top_k }
  ▼ PostgreSQL: FTS(zhparser 中文) ∥ pgvector HNSW 余弦
  ▼ RRF(k=60) 融合 → top-k chunks → 高亮包 + 出处回链
  ▼ 注入 Flow A 步骤 1 的上下文包（token 预算约束）
```

### Flow D — iOS 侧事件通路（与 A 同构）

```
SwiftUI ⏎ → try await cricket.chatSend(...)
  ▼ cricket-core(静态库内 SessionActor) ……后续与 Flow A 完全一致
  ▼ Emitter → EventSink.onEvent(CoreEvent) → DispatchQueue.main → SwiftUI 状态机
```

---

## 5. 架构决策记录（ADR）

### ADR-001 · Core 与端同进程静态链接（否决 sidecar 进程）
- **决策**：Windows 上 `cricket-core` 直接链入 Tauri 宿主二进制；iOS 上编译为静态框架。**不采用**独立进程 + 本地 socket 的 sidecar 方案。
- **理由**：省一次序列化跳数与一个进程的常驻内存（符合 < 50MB 约束）；Tauri 命令即同步函数调用；UniFFI 天然同进程。
- **代价**：Core panic 会带崩宿主 → 强制约束：FFI 边界 `catch_unwind` 全覆盖，panic 翻译为 `CricketError::Internal`。

### ADR-002 · LLM 路由双模式，默认 relay
- **决策**：默认经服务端转发；`direct` 模式按 Provider 可选开启。
- **理由**：① 密钥不出私有服务器；② 服务端可统一审计/限流/断线重连重放；③ 客户端网络环境对厂商直连不可控（尤其国内），relay 提供单一可控出口。
- **代价**：增加一跳延迟 → 首字节 P50 预算放大 ≤ 120ms（SLO 表）；流式文本带宽极低（KB/s 级），服务器出口无压力。

### ADR-003 · 端侧 SQLite + 服务端 PostgreSQL/pgvector，不引入重中间件
- **决策**：知识库向量索引、图谱、正文全部托管在私有服务器 PostgreSQL 16（pgvector + zhparser）；端侧仅 SQLite（会话事件溯源 + FTS5 离线缓存）。**不引入** Qdrant / ES / Redis。
- **理由**：单 VPS 资源约束下运维面最小；pgvector HNSW 完全满足百万级 chunk；备份 = pg_dump 单命令。

### ADR-004 · 单一契约，双导出，禁止手写第二套类型
- **决策**：所有跨边界类型只定义一次于 `cricket-protocol`，同时 `#[derive(Serialize, Deserialize, uniffi::Enum/Record)]`。线协议（SSE data）即 `CoreEvent` 的 serde 序列化，FFI 即 uniffi 生成。
- **理由**：三处（Tauri JSON / Swift 对象 / 线协议）天然一致，杜绝"类型漂移"这一 FFI 项目最高频缺陷源。

### ADR-005 · 全链 rustls
- **决策**：所有 HTTPS 走 rustls，`cargo-deny` 禁 `openssl`/`native-tls`。
- **理由**：iOS 交叉编译与体积；Windows 免 schannel 版本差异。

### ADR-006 · 会话 = 事件溯源（Event Sourcing）
- **决策**：会话存储 append-only 事件流（`CoreEvent` 序列化）+ 周期折叠快照；消息树（regenerate/分支）由事件流推导。
- **理由**：① regenerate = 从任意消息节点回放重跑，天然正确；② 断线重连重放 = 服务端同款事件日志；③ 调试可"重放昨天那个会话"。

### ADR-007 · 会话即 Actor，取消即 drop
- **决策**：每个会话一个 `tokio::sync::mpsc` Actor；`CancellationToken` 传播到 gateway HTTP body 与工具执行；abort 后 emit `Stage::Aborted`。
- **理由**：多会话并行互不干扰；取消语义唯一、无泄漏。

### ADR-008 · Emitter 统一扇出 + 16ms 合帧
- **决策**：Core 内唯一事件出口 Emitter，`TextDelta/ReasoningDelta` 按 16ms 窗口合帧后扇出至所有 transport（Tauri Channel / UniFFI 回调 / SSE）。
- **理由**：IPC 与 Swift 回调免于高频小包（60fps 渲染上限即 16ms，超频无意义）；三端行为一致。

---

## 6. 端侧存储设计（SQLite）

每端一个 `cricket.db`（Windows: `%APPDATA%/Cricket`；iOS: App 沙盒 `Application Support`）：

```sql
-- 事件溯源：会话真相源
CREATE TABLE session_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,   -- 全局单调，兼作 Last-Event-ID
  session_id TEXT NOT NULL,
  ts_ms INTEGER NOT NULL,
  event_json TEXT NOT NULL               -- CoreEvent serde
);
CREATE INDEX idx_events_session ON session_events(session_id, id);

-- 折叠快照：加速重开
CREATE TABLE session_snapshots (
  session_id TEXT PRIMARY KEY,
  last_event_id INTEGER NOT NULL,
  folded TEXT NOT NULL,                  -- 消息树折叠态
  updated_at INTEGER NOT NULL
);

-- 智能体与配置
CREATE TABLE agents (id TEXT PRIMARY KEY, name TEXT, system_prompt TEXT,
                     config_json TEXT, updated_at INTEGER);
CREATE TABLE goals (id TEXT PRIMARY KEY, objective TEXT, state TEXT, ...); -- 见 02 文档
```

同步策略：**端为真相源（会话数据）**，服务端为镜像 + 全文检索索引；知识库相反：**服务端为真相源**，端持只读缓存。冲突解决 = 单写者模型，天然无冲突。

---

## 7. 与 124.223.154.233 的关系边界

- 该服务器承载：Cricket-Server、PostgreSQL(知识库真相源)、SearXNG、Crawler、备份调度。
- 明确**不承载**：模型自托管推理（一期不做）；第三方多租户（仅私有单用户/小团队）。
- 实测记录（2026-10-06）：TCP 22 可达。TLS 需绑定域名后由 Caddy 自动签发；无域名场景的降级方案见 05 文档 §4。
