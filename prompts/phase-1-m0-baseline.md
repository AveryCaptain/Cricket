# Phase 1 · M0 基线（契约、网关与离线回归）

<!-- 复制【提示词开始】至【提示词结束】之间的全部内容，粘贴给执行 AI -->

【提示词开始】

你是 Cricket 项目的核心 Rust 工程师。本阶段目标：完成 **M0 基线**——Cargo workspace、统一消息契约 `cricket-protocol`、三厂商网关 `cricket-gateway`、golden 离线语料、`cricket-cli` 冒烟与 CI。本阶段**不写任何 UI、不碰服务端业务功能、不引入 uniffi 实际构建**。

【上下文加载（必须先完成）】
1. 读 `docs/00-architecture.md` §3（crate 职责与依赖矩阵）、§5（ADR-001 ~ ADR-008）、§6（端侧 SQLite 设计）。
2. 读 `docs/01-communication.md` **全文**——这是本阶段的施工图，尤其 §2（契约类型全集）与 §6（厂商差异映射表）。
3. 读 `docs/04-roadmap.md` §2 的 M0 小节与 §5（质量门）。
4. 状态自检：`git log --oneline`、`git status`、（若存在）`cargo test --workspace`。若发现 M0 已被部分完成（存在 `crates/` 目录），逐项对照本提示词【验收】清单**只补缺不重做**。

【任务】
1. **Cargo workspace**：根 `Cargo.toml`（workspace；`[profile.release] opt-level="s"`、`lto="thin"`、`codegen-units=1`、`strip=true`），按 `docs/00` §3 布局创建 11 个成员 crate 骨架：`cricket-protocol`、`cricket-gateway`、`cricket-agent`、`cricket-memory`、`cricket-tools`、`cricket-skills`、`cricket-novel`、`cricket-code`、`cricket-ffi`、`cricket-server`、`cricket-cli`。每个 crate 初始只有 `lib.rs` + 模块级 doc 注释（一句话职责，照抄 docs/00 §3 矩阵）。依赖方向严格按矩阵：只许自上而下，`cricket-protocol` 不依赖任何 crate。
2. **cricket-protocol（本阶段核心）**：
   - 按 `docs/01` §2 **逐类型**实现：`Stage`、`Role`、`FinishReason`、`Usage`、`CoreEvent`、`KbScope`、`TaskInfo`、`TaskState`、`CricketError`，以及 Agent/Session 相关 Record（`AgentSpec`、`AgentBrief`、`SessionSnapshot`、`SessionBrief`、`DocumentInput`、`KbHit`、`Attachment`、`ModelPref`、`AgentMode`、`RouteMode`）。
   - uniffi 门控：所有跨边界类型用 `#[cfg_attr(feature = "ffi", derive(uniffi::…))]`，uniffi 为 **optional** 依赖——默认构建绝不引入 uniffi（ADR-004/005）。serde tag/rename 规则与文档逐字一致。
   - 每个 enum/struct 至少一个 serde round-trip 单测（serde_json 序列化→反序列化→字段全断言）。
3. **cricket-gateway**：
   - `ProviderAdapter` trait 与 `Capability` 能力位（按 `docs/01` §6）；实现 OpenAI / Anthropic / Gemini 三个适配器 + `openai_compatible` 泛化适配器。
   - SSE 解析基于 `eventsource-stream` + `reqwest`（`rustls-tls`）；全链禁止 native-tls/openssl（ADR-005）。
   - 运行策略按 `docs/02` §2：重试（250ms 指数退避 ≤3 次 + jitter）、降级链（ModelPref fallbacks）、usage 记账事件。
4. **golden 语料库 `fixtures/golden/`**：三家厂商各 ≥10 条**离线 SSE 语料**（覆盖：纯文本、思考链/Reasoning 分流、工具调用增量参数、中断、超长流），语料是手工构造的文本文件，但帧格式必须严格符合 `docs/01` §6 映射表所描述的各厂商真实格式。配套回放测试：读语料 → adapter normalize → 断言 `GatewayEvent` 序列。全部测试**无外网可跑**。
5. **cricket-cli**：clap 命令 `chat --provider mock --fixture <语料路径>`：用 MockProvider 回放语料，驱动 gateway→Emitter→stdout 按行打印归一化事件，证明端到端管线通。预留 `--provider openai|anthropic|gemini --live`（读环境变量密钥，默认关闭，CI 永不跑 live）。
6. **Emitter 骨架**（放在 cricket-agent，供后续阶段复用）：统一扇出 trait + 16ms 合帧（ADR-008：TextDelta/ReasoningDelta 合帧，Start/End/Stage 直通），带单测证明合帧语义（60 条小 delta @1ms 间隔 → 输出 ≤5 帧）。

【红线】
- 新增第三方依赖仅限：tokio、serde、serde_json、thiserror、reqwest(rustls)、eventsource-stream、futures、clap、（测试用）anyhow/tokio-test。超出此清单必须先在报告中写明理由并等确认（本阶段先记录，不擅自加）。
- 契约类型与 `docs/01` §2 语义完全一致；**发现文档内部矛盾 → 停止并在报告中列出，不许自行改契约**。
- 导出路径禁 `unwrap/expect`（ADR-001 精神）；所有公开 API 返回 `Result<_, CricketError>`。
- 不修改 `docs/`、`prompts/`、`.gitattributes`。

【验收（全部通过才算完成）】
1. `cargo test --workspace` 全绿，其中 golden 回放 ≥30 个用例。
2. `cargo clippy --workspace --all-targets -- -D warnings` 零告警。
3. `cargo deny check bans licenses` 通过（ban：native-tls、openssl）。
4. `cargo check -p cricket-protocol --no-default-features` 成功（证明 uniffi 是 optional）。
5. `cargo run -p cricket-cli -- chat --provider mock --fixture <任一语料>` 打印出归一化事件流。
6. CI：`.github/workflows/ci.yml` 按 `docs/04` §4——ubuntu job：fmt --check、clippy、test、`cargo check --target x86_64-pc-windows-msvc`（防 FFI/平台类型漂移）、cargo-deny；windows-latest job：build smoke。uniffi 生成基线与 macos job 留给 Phase 3。

【工作方式】
分支 `phase/m0-baseline`；每个任务单元完成后立即 conventional commit（`feat:`/`test:`/`ci:`/`build:`）；结束时 push。完成报告：① 变更摘要；② 测试统计（用例数/通过数）；③ 【验收】逐项打勾附命令输出证据；④ 遗留问题与建议。

【提示词结束】
