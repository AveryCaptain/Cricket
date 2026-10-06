# Phase 2 · M1 骨架（Windows 端到端流式聊天一条线）

<!-- 复制【提示词开始】至【提示词结束】之间的全部内容，粘贴给执行 AI -->

【提示词开始】

你是 Cricket 的核心与 Windows 端工程师。本阶段目标：完成 **M1 骨架**——内存 spike 前置、Agent 运行时（会话 Actor/事件溯源）、Tauri v2 壳与 Codex 风 UI、服务端 relay 骨架，打通"Windows 上流式聊天"一条线。施工图：`docs/00` §4/§6、`docs/01` §2/§4/§5、`docs/03` 全文、`docs/04` §2-M1。

【上下文加载（必须先完成）】
1. 读 `docs/00-architecture.md` §4（Flow A/B/D）、§5 ADR-002/006/007/008、§6（端侧 SQLite DDL）。
2. 读 `docs/01-communication.md` §2（CoreEvent 语义与阶段机）、§4（Tauri 绑定规范：command 面/事件节流/前端约束）、§5（SSE 线协议）。
3. 读 `docs/03-ui-spec.md` **全文**（布局线框、design tokens、等待三件套、交互、性能预算）。
4. 自检：`git log --oneline`；`cargo test --workspace` 必须全绿后再开工（M0 基线）；若本阶段已部分完成，对照【验收】只补缺。

【任务】
1. **【Spike 前置，风险 R1】** 空的 Tauri v2 壳（`apps/desktop`，纳入 workspace）：静态渲染 5000 条消息节点 + 你自研的虚拟滚动，实测并记录：宿主进程（cricket.exe）RSS 与 WebView2 子进程 RSS（任务管理器/perfmon），数据写 `reports/memory-spike-windows.md`。若宿主 > 50MB 或 WebView2 > 70MB → 先按 `docs/03` §7 优化（DOM 窗口化 ≤2000 节点、去重渲染、纯文本 patch），复测后再继续；仍超标就在报告中给出裁剪方案。
2. **cricket-agent**：
   - `SessionActor`：每会话一个 tokio mpsc actor（ADR-007）；命令：`Chat`/`Abort`/`Regenerate{message_id}`。
   - 取消：`tokio_util::sync::CancellationToken` 全链传播（gateway HTTP body、工具执行、Emitter flush），abort 后 emit `Stage::Aborted` + `StreamClosed`。
   - Emitter 落地（Phase 1 骨架补全为多 sink 扇出）：`TauriSink`（`tauri::ipc::Channel<CoreEvent>` 适配）、`LogSink`（测试用）。阶段机严格按 `docs/01` §2：每阶段切换发一次 `StageChanged`（等待动画的驱动源）。
   - 事件溯源存储：`docs/00` §6 的 `session_events`/`session_snapshots` DDL（rusqlite，bundled），append-only + 折叠快照 + `MessageTree`（regenerate = 从消息树节点回放重跑，`docs/02` §1.3）。
   - 工具循环骨架：`docs/02` §1.4（上限 12 次/轮、同名同参复用、Schema 校验失败注入错误结果）。
3. **cricket-server**：Axum + tower：
   - `POST /api/v1/chat/relay`：body 带 provider/model/messages/tools/stream，服务端调 gateway 转发 SSE（线协议 = CoreEvent 序列化帧，帧格式照 `docs/01` §5.2）。
   - `GET /api/v1/sessions/{id}/events?since=`：ring buffer（≥2048 事件/30min TTL）+ `Last-Event-ID` 重放。
   - `/healthz`（204）；认证：`Authorization: Bearer <PAT>`，PAT 读服务端环境变量 `CRICKET_PAT`（单令牌即可，多用户不做）。
   - 15s keep-alive 心跳（`docs/01` §5.2）。
4. **apps/desktop**（严格按 docs/03）：
   - command 面 = `docs/01` §4.1：`session_open`（接 `Channel<CoreEvent>`）/`chat_send`/`abort`/`regenerate`，方法名与参数同构。
   - 前端**零框架**：原生 ES Modules（禁 React/Vue；preact 可选），样式用 CSS 变量逐条落地 `docs/03` §2 tokens（色板/字体栈/间距/圆角/动效时长），布局按 §3 线框（Toolbar 36 / AgentRail 240 / SessionTabs 28 / Inspector 320 / StatusBar 22）。
   - 消息流虚拟滚动（≤2000 节点窗口化，IntersectionObserver 驱动）。
   - **等待三件套**（`docs/03` §5）：`StageChanged` 驱动阶段文字（150ms crossfade，文案映射表 §5.2）+ WaveDots（1.2s 周期正弦 keyframe 近似、90° 错相 stagger=0.15s，reduced-motion 降级呼吸态）+ 精确计时器（`performance.now()` 本地计时、100ms 节流、`mm:ss.d` 格式、tabular-nums）。
   - ReasoningAccordion：生成中展开限 40vh、ReasoningDelta 停 1.5s 自动折叠为摘要头；ToolCallCard 占位渲染；Esc abort（UI 即时进"停止中…"，收 Aborted 事件收尾）。
5. **双路由**（ADR-002）：`CoreConfig.route_mode = Relay | Direct`；加 `Mock` 路由（无网络，回放语料）保证全链 CI 可测。
6. 扩展 CI：windows job 增加 Tauri 产物构建 + 安装包体积断言 ≤15MB；server job 增加 compose 之外的最小 `cargo test -p cricket-server`（用 axum test 框架）。

【红线】
- 前端禁运行时外链资源与虚拟 DOM 重框架（`docs/01` §4.3）。
- 会话数据端为真相源：服务端只做镜像，**禁止**服务端反向改写端数据（`docs/00` §6 同步策略）。
- UI 颜色/字号/动效参数必须与 docs/03 tokens 完全一致，不许自创。
- 事件帧必须与 `docs/01` §5.2 帧格式逐字段一致（id/event/data 三要素）。

【验收】
1. Windows 端到端（mock 路由）：流式聊天 + 思考链折叠展开 + Esc 中断 + 阶段文字随 StageChanged 变化 + WaveDots 动画 + 计时器走字——全部可操作录屏或截图证据。
2. 断线重连测试：模拟网络中断 30s（断 server 或断网）→ 重连带 `Last-Event-ID` → 事件零丢失（写自动化测试用例证明重放正确）。
3. 内存数据：宿主 RSS < 80MB（向 50MB 收敛的中间门），WebView2 ≤ 70MB，写入 spike 报告。
4. `cargo test --workspace` 全绿；`cargo clippy --workspace --all-targets -- -D warnings` 零告警；Tauri 产物 ≤15MB。
5. （有厂商 key 时人工跑一次）真实 key 三家各实测一轮流式含 reasoning 模型，行为差异记录 `reports/provider-notes.md`；无 key 则在报告标注"待人工验证"。

【工作方式】
分支 `phase/m1-skeleton`；小步提交；结束 push 并输出：① 变更摘要；② 【验收】逐项打勾与证据；③ 内存 spike 数据表；④ 遗留问题。

【提示词结束】
