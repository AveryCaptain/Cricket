# Cricket（鸣蛩）— 跨平台智能体系统设计基线

> 极轻量（Low-Footprint）· 高并发 · 低延迟的跨端双向协同 Agent 客户端与服务端，**全栈 Rust**。

| 项 | 值 |
|---|---|
| 版本 | v1.0（设计基线，指导 M0 起步） |
| 日期 | 2026-10-06 |
| 私有服务器 | `124.223.154.233`（SSH 22 实测可达；部署方案见 05 号文档） |
| 凭据安全 | ⚠️ 初始 root 密码已在对话/文档流程中明文出现过，**上线前必须轮换并停用密码登录**，改用 SSH ed25519 密钥（见 05 号文档加固清单第 1 条） |

## 1. 文档索引

| # | 文档 | 内容 |
|---|------|------|
| 0 | [docs/00-architecture.md](docs/00-architecture.md) | 系统拓扑、Cargo Workspace 拆解、关键数据流、架构决策记录（ADR） |
| 1 | [docs/01-communication.md](docs/01-communication.md) | UniFFI 导出规范、Tauri IPC 绑定、SSE 线协议、统一消息契约（CoreEvent） |
| 2 | [docs/02-domain-modules.md](docs/02-domain-modules.md) | 多厂商网关 / 分层知识库 RAG / 工具与 Skill / 小说 / 代码(Goal-DAG+Git) 模块架构与存储模型 |
| 3 | [docs/03-ui-spec.md](docs/03-ui-spec.md) | Codex 风格 UI 设计规范、波浪等待动画（Wave Dots）与精确计时器实现规格 |
| 4 | [docs/04-roadmap.md](docs/04-roadmap.md) | 16 周里程碑计划、验收标准、CI/CD 流水线、风险登记册 |
| 5 | [docs/05-deployment.md](docs/05-deployment.md) | 服务器安全加固、部署拓扑（Docker Compose + Caddy）、备份与监控 |
| — | [prompts/README.md](prompts/README.md) | **AI 接力施工提示词包**：Phase 0–9 每阶段一份独立提示词（幂等可重跑、先读文档后施工、验收门不过不放行） |

## 2. 核心定位

**一句话**：把一个 Rust 无头核心（Cricket-Core）编译进 Windows（Tauri v2）与 iOS（SwiftUI）两端，接上一个私有化 Rust 服务端（Axum），形成"端侧极轻、服务端自治"的双向协同智能体系统。

```
        ┌──────────────────┐        ┌──────────────────┐
        │ Windows 客户端    │        │  iOS 客户端       │
        │ Tauri v2 + Core  │        │  SwiftUI + Core   │
        └────────┬─────────┘        └────────┬─────────┘
                 │      HTTPS  REST + SSE      │
                 └───────────────┬─────────────┘
                                 ▼
                  Cricket-Server @ 124.223.154.233
                  (持久化 · 私有知识库 · 工具代理)
                                 │ HTTPS
                 ┌───────────────┼───────────────┐
                 ▼               ▼               ▼
              OpenAI         Anthropic        Gemini
```

## 3. 量化目标（SLO 摘要）

| 指标 | 目标 | 度量方式 |
|------|------|----------|
| Windows 安装包体积 | ≤ 15 MB（WebView2 系统自带，不计入） | bundler 产物 |
| Windows 宿主进程常驻 RSS | **< 50 MB**（Tauri app 进程；WebView2 子进程单列预算 ≤ 70 MB） | perfmon WorkingSet |
| iOS 静态框架体积 | ≤ 18 MB（arm64） | xcframework 尺寸 |
| 冷启动至首屏 | Win < 1.5 s / iOS < 1.2 s | 启动脚本计时 |
| 首 token 延迟（relay 模式） | P50 ≤ 上游直连 + 120 ms | 服务端直方图 |
| 流式渲染 | 60 fps，主线程每帧 ≤ 2 ms | devtools performance |
| 知识库检索端到端 | P95 ≤ 800 ms | 客户端 trace |
| 服务端并发 SSE 流 | ≥ 256（按 2C4G 最低基线设计） | k6 压测 |
| 会话事件可靠性 | 断线重连零丢失（Last-Event-ID 重放） | 混沌测试 |

## 4. 技术栈矩阵

| 层 | 选型 | 说明 |
|----|------|------|
| 核心库 | Rust workspace，crates.io 稳定版为基线（以开工当日最新稳定版锁定） | `tokio` / `serde` / `reqwest(rustls)` / `axum` / `tower` |
| Windows 客户端 | **Tauri v2**（WebView2 系统级 Webview） | Core 静态链接进宿主进程，IPC 走 `tauri::ipc::Channel` |
| iOS 客户端 | **uniffi-rs** 生成 Swift 绑定 + Swift Package 静态框架 | UI 全部 SwiftUI 原生渲染 |
| 服务端 | Axum + Tokio + Tower | PostgreSQL 16 + pgvector + SearXNG + Crawler |
| 端侧存储 | SQLite（事件溯源 + FTS5） | 零常驻服务，冷启动即用 |
| Git 集成 | `git2-rs`（libgit2） | Diff 分析 / 自动 commit / 分支策略 |
| UI | Codex/IDE 风：暗色高对比、JetBrains Mono + 微软雅黑（iOS 降级 PingFang SC） | 波浪等待动画 = 阶段文字 + Wave Dots + 精确计时器 |

## 5. 阅读路径建议

- **决策者 / 架构评审**：00 → 04（约 20 分钟）
- **端侧 / FFI 工程师**：00 → 01 → 03
- **后端 / 领域工程师**：00 → 02 → 05
- **新成员 onboarding**：本 README → 00 → 对应方向文档
