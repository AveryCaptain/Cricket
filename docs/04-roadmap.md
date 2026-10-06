# 04 · 全周期研发计划（Milestones / Roadmaps）

> 目标读者：项目负责人与全体成员。节奏假设：核心团队 3 人（1 后端/核心 + 1 Windows 端 + 1 iOS 端，架构师全职跟审），16 周到达内测。所有验收门为**可测量**的硬指标，不达标不进下一里程碑。

---

## 1. 里程碑总览

| 里程碑 | 周次 | 主题 | 核心交付 | 出口验收门 |
|--------|------|------|----------|------------|
| **M0 基线** | W1–2 | 地基 | workspace 脚手架、`cricket-protocol` 契约 v1、三家厂商适配器、golden 语料库（30 用例）、`cricket-cli` 冒烟 | ① golden 回放 30/30 绿 ② `cargo-deny` 全绿 ③ CI 三平台矩阵跑通 ④ **Windows 内存 spike 实测报告**（风险 R1 前置） |
| **M1 骨架** | W3–5 | 端到端一条线 | Session Actor + 事件溯源落盘、Tauri 壳 + 流式聊天（Win）、Server relay 骨架（Axum+SSE+Last-Event-ID） | Windows 端到端聊天含 Reasoning 分流；宿主进程 RSS < 80MB（向 50 收敛的中间门）；断线 30s 内重连零丢失 |
| **M2 双端** | W6–7 | iOS 合流 | xcframework 流水线、SwiftUI 聊天（含三件套等待动画）、多会话/abort/regenerate、双端功能对齐矩阵 | 双端 40 项功能矩阵 ≥ 90% 对齐；iOS framework ≤ 18MB；uniffi round-trip 测试 100% |
| **M3 知识与工具** | W8–10 | RAG + 工具面 | PostgreSQL+pgvector 上线、chunking/混合检索、KB CRUD UI、SearXNG+Crawler、Skill 注册 v1、usage 记账与 Inspector | KB 端到端 P95 ≤ 800ms；web_search 端到端可用；Skill 本地/远程双模式 e2e |
| **M4 小说** | W11–12 | Creative Suite | 世界观图谱（CRUD+交叉引用）、人物卡、事件记忆链、一致性校验管线、润色算子 + diff UI | 校验管线对样例章节产出结构化报告；润色 diff 接受率可统计；图谱误污染率 < 5%（半自动确认机制生效） |
| **M5 代码** | W13–14 | Engineering Suite | Goal-DAG 状态机 + planner、验证修复环、git2 封装与 diff UI、自动 commit message、分支策略 | 示例仓库（含失败测试）自主完成"修复并绿"任务且产生干净提交序列；脏树保护用例全过；DAG 中断后可续跑 |
| **M6 硬化** | W15 | 达标冲刺 | < 50MB 专项、性能剖析、安全加固（05 文档清单）、部署上线到 124.223.154.233、灾备演练 | SLO 表（README §3）全项达标；备份恢复演练通过；安全清单 100% 勾选 |
| **M7 内测** | W16 | 发布 | Windows 安装包（NSIS 自动更新）、iOS TestFlight、灰度 10 人、崩溃/遥测（自托管，无第三方 SDK） | 内测零 P0；首周 crash-free ≥ 99.5%；收集清单驱动 backlog v2 |

---

## 2. 各里程碑任务拆解（按 crate）

### M0 · W1–2（契约与网关先行，**不写一行 UI**）
- `cricket-protocol`：§01.2 全部类型 + serde/uniffi 双 derive + 30 个契约单测。
- `cricket-gateway`：OpenAI / Anthropic / Gemini 三适配器；SSE 解析（eventsource-stream）；重试/降级/能力位。
- `fixtures/golden`：每家 ≥ 10 条真实抓包（纯文本/思考链/工具调用/中断/长流）。
- `cricket-cli`：`chat --agent x --model y` 直驱 Core 的冒烟路径。
- CI：fmt + clippy(-D warnings) + test + `cargo check --target aarch64-apple-ios`（不依赖 mac 也在 Linux 跑，防 FFI 类型漂移）+ `cargo-deny`（ban openssl/native-tls）。
- **spike（风险 R1 前置）**：空 Tauri 壳 + 5k 消息节点压测，出 Windows 实测内存报告，决定 WebView2 预算策略。

### M1 · W3–5（Windows 一条线打穿）
- `cricket-agent`：SessionActor、Emitter（16ms 合帧）、消息树、abort（CancellationToken 全链）。
- `cricket-memory`（本地部分）：SQLite 事件溯源 + 快照折叠。
- `cricket-server`：relay 端点 + 事件日志 + ring buffer 重放 + `/healthz`。
- `apps/desktop`：Tauri 壳、Channel 事件桥、消息流虚拟滚动、三件套等待动画（§03.5）第一版。
- 交付演示：Windows 上与 Anthropic 模型流式对话，思考链折叠展开、Esc 中断、断网 30s 重连补帧。

### M2 · W6–7（iOS 合流，共用同一 Core）
- `cricket-ffi`：`#[uniffi::export]` 门面 + EventSink 回调桥 + catch_unwind 覆盖（R3）。
- CI（macos runner）：三目标构建 + lipo + xcframework + Swift Package 产物归档。
- `apps/ios`：SwiftUI 聊天页、WaveDots TimelineView、消息树模型、多会话侧栏。
- 双端对齐矩阵：40 项（发送/流式/折叠/工具卡/abort/regenerate/变体/计时/暗色…）逐项打勾，UI 走查截图存档。

### M3 · W8–10
- `cricket-server`：`kb_documents/kb_chunks` DDL 迁移（sqlx migrate）、混合检索端点、web_search/crawl 代理、限流中间件（tower::limit）。
- `cricket-memory`：服务端客户端 + 缓存镜像 + `KbChanged` 失效。
- `cricket-skills`：TOML manifest、本地编排引擎、远程 HMAC 签名。
- `cricket-tools`：JSON Schema 校验、白名单、沙盒路径检查。
- 服务器侧：Postgres/pgvector/zhparser、SearXNG、Crawler 容器化（先在测试环境，M6 才切生产）。

### M4 · W11–12（`cricket-novel`）
- DDL（world_nodes/edges、personas、persona_events）+ 别名检索 + 半自动建边确认流。
- ContextPack 组装器（§02.4.5 预算表）。
- 一致性校验管线（抽取→召回→裁决→报告→回写）+ 润色算子 5 个 + diff UI。
- 验收用例固定：一部 ≥ 3 万字样例小说 + 20 个注入型矛盾（人物死亡后复活、地点归属冲突…），报告召回率 ≥ 90%。

### M5 · W13–14（`cricket-code`）
- Goal-DAG：planner（JSON Schema 约束）、拓扑执行（并发 4）、verify/repair 环、SQLite 持久化与续跑。
- `git2` 封装：status/diff/commit/branch/checkout + 脏树保护；commit message LLM 生成。
- Inspector：DAG 视图 + diff 浏览 + WIP commit 时间线。
- 验收仓库：`sample-repo/`（Rust 小项目，含 3 个红测试 + 1 个 feature 缺失），Goal 描述"让测试全绿并实现 X"，全自主完成。

### M6 · W15（硬化与上线，细节见 05 文档）
- 性能：wasm 无关、直接 `tracy-client` + `framegraph` 抓流式渲染路径；内存逐项削（字体子集、索引窗口化、sqlite mmap 调优）。
- 安全：服务器加固清单、PAT 令牌生命周期、Skill HMAC、（若开 shell_run）命令黑名单审计。
- 部署：生产 compose 栈上 124.223.154.233，DNS + Caddy TLS，备份 cron，恢复演练一次。

### M7 · W16（发布）
- Tauri updater（NSIS + 静态 JSON feed 自托管在服务器）；fastlane TestFlight；崩溃报告自托管 endpoint（server 一并承载，无第三方 SDK）。

---

## 3. 团队与分工（3 人基线 + 架构师）

| 角色 | 覆盖 | 关键产出 |
|------|------|----------|
| 后端/核心（1） | protocol, gateway, agent, server, 部署 | M0–M3 主力，M4/M5 领域模型 |
| Windows 端（1） | desktop(Tauri+前端), tools 沙盒, git2 | M0 spike + M1 主力，M3 UI，M5 diff UI |
| iOS 端（1） | ffi, ios app, 动效 | M2 主力，此后双端并行铺 UI |
| 架构师（兼任） | 评审、ADR、契约把关 | 每里程碑验收门裁决 |

原则：**契约评审不过不写实现**（01 文档 §8 清单）；任何端不许私改协议。

---

## 4. CI/CD 流水线

```yaml
# .github/workflows/ci.yml（要点）
on: [pull_request]
jobs:
  core:
    runs-on: ubuntu-latest
    steps: [rust-toolchain stable, fmt --check, clippy -D warnings,
            cargo test --workspace,                    # 含 golden 回放
            cargo check --target aarch64-apple-ios,     # FFI 类型漂移防线
            cargo check --target x86_64-pc-windows-msvc,
            cargo-deny check bans licenses]
  windows:
    runs-on: windows-latest
    steps: [tauri build（smoke）, 产物体积断言 ≤ 15MB]
  ios:
    runs-on: macos-latest
    steps: [uniffi-bindgen 生成并 diff 基线,  # swift 绑定变更必须显式提交
            三目标构建 + xcframework 组装, 体积断言 ≤ 18MB,
            swift test（EventSink round-trip）]
  server:
    runs-on: ubuntu-latest
    steps: [cargo test -p cricket-server, docker build, compose 起栈 + healthz 探活]
release: tag → [Windows NSIS 上传 + 更新 feed, iOS TestFlight(fastlane), server 镜像 push + 服务器 compose pull --no-deps && up]
```

---

## 5. 质量门与度量

| 维度 | 门 |
|------|----|
| 测试 | 契约类型 100% 有 serde round-trip 测试；golden 语料 ≥ 30 且每次上游变更必补 |
| 性能 | criterion benches：gateway 吞吐、SQLite 折叠、chunking 速度；SLO 每里程碑复测 |
| 体积 | CI 产物断言（15MB / 18MB） |
| 静态 | clippy -D warnings；`cargo audit`；`cargo-shear` 清未用依赖 |
| 端到端 | `cricket-cli` 场景 20 条（聊天/中断/工具/KB/DAG/离线重连） |

---

## 6. 风险登记册（Top 10）

| # | 风险 | P×I | 缓解 / 预案 |
|---|------|-----|------------|
| R1 | WebView2 实际内存超预算（< 50MB 承诺） | 高×高 | **M0 spike 前置实测**；DOM 预算 + 虚拟滚动 + 禁重框架；若宿主达标而渲染进程超标，与用户重定义口径（宿主 < 50MB，渲染进程单列）——技术事实提前透明 |
| R2 | UniFFI async/回调边界 case（版本锁定后仍有坑） | 中×中 | 版本锁死 + 回调只做 fire-and-forget（R5 禁返回值）+ 统一 `CricketEventDispatcher` 模板；最后防线：降级为轮询接口 `drain_events()`（契约已预留） |
| R3 | 上游厂商接口/SSE 变更（Gemini 尤甚） | 中×高 | golden 回放 + 适配器隔离（§01.6）；变更只改适配器补语料，不发全端版（relay 模式下**服务端热修即全端生效**——relay 默认的核心收益） |
| R4 | 国内网络对厂商直连不可用 | 高×中 | relay 为默认（ADR-002）；direct 模式标注"需自备网络条件" |
| R5 | iOS 沙盒限制 git/fs 工具可用性 | 低×高 | 只允许沙盒 `Documents/repos/`；与预期用户（iOS 轻使用、Windows 重工程）匹配；文档明示 |
| R6 | 中文检索质量不达标（zhparser 装不上 / 分词差） | 中×中 | 部署期探测回退 pg_trgm；chunk 内加入 heading_path 提权；A/B golden 检索集量化 |
| R7 | LLM 成本失控（DAG/校验管线多轮放大） | 中×中 | usage_ledger 记账 + Inspector 实时显示；DAG 修复环预算硬上限（attempts<3）；每会话日预算软限制 |
| R8 | 单点服务器故障丢知识库 | 低×高 | 每日 pg_dump + 异地对象存储 + 每月恢复演练（05 文档 §5）；端侧 KB 缓存可只读续命 |
| R9 | 范围蔓延（小说/代码特化无限膨胀） | 高×中 | 架构红线：特化不改主循环（02 §7）；里程碑冻结 + backlog v2 承接 M4/M5 之外的想法 |
| R10 | 凭据安全事故（root 密码已在沟通中暴露） | 中×高 | **M0 第 0 项任务**：轮换密码 → ed25519 密钥 → 禁用密码登录（05 文档 §2）；服务端密钥只进 `.env`/vault，不入库不入图 |

---

## 7. 裁决机制

- 契约变更：架构师 + 受影响端负责人双签（01 文档 §8 清单）。
- SLO 未达：M6 设"达标日"（W15 周四），未达项进**只减不增**清单（砍功能换预算）。
- 每里程碑结束 1 页复盘（ADR 增补 + 风险登记册更新），进 `docs/` 版本化。
