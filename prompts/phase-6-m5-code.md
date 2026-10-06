# Phase 6 · M5 代码模式（Goal-DAG / 验证修复环 / git2 集成）

<!-- 复制【提示词开始】至【提示词结束】之间的全部内容，粘贴给执行 AI -->

【提示词开始】

你是 Cricket 的工程化领域工程师。本阶段目标：完成 **M5 代码模式 Engineering Suite**——Goal→Task DAG 自动拆解、自主循环验证（执行→verify→修复）、git2 原生集成。施工图：`docs/02` §6 全部；装配红线：`docs/02` §7。

【上下文加载（必须先完成）】
1. 读 `docs/02-domain-modules.md` §6 全部：§6.1 总览、§6.2 状态机与端侧 SQLite DDL（goals/tasks/verify_runs）、§6.3 Planner JSON 输出契约、§6.4 GitHost trait 与分支/安全策略、§6.5 工具集。
2. 读 §7 装配红线；自检起点测试全绿；部分完成只补缺。

【任务】
1. **Goal-DAG 核心（cricket-code）**：
   - SQLite 持久化照 §6.2 DDL；**进程重启续跑**：恢复 ready/running 集合与状态机（必须有用例：执行中 kill 进程 → 重启 → 从持久态正确继续）。
   - Planner：LLM 调用强 JSON Schema 校验（§6.3 契约），解析失败重试 ≤2 后降级为单任务粗粒度 DAG 并在事件中说明原因。
   - Executor：petgraph 拓扑序 + 并发上限 4（Semaphore）；verify 步骤两类——`cmd + expect_exit + timeout_s` 与 `llm_rubric`，顺序执行；失败 → **修复环**（attempts<3，把 verify_runs 尾部输出注入 Repair prompt）→ 预算耗尽转 Blocked + 障碍报告（不静默失败）；>30% 任务 Blocked 时触发 Planner 复盘钩子生成**增量 DAG**。
   - 全程 `TaskUpdated` 事件经既有 CoreEvent 通道推送（消息通道不许新增私有事件类型）。
2. **GitHost（git2-rs 封装）**：`docs/02` §6.4 六方法（status/diff/commit_generated/branch_create/checkout/log_graph）：
   - **脏树保护**：任何写操作前 status 不干净即中止并报告，绝不 stash/丢弃用户改动。
   - 分支策略照 §6.4：`cricket/goal-<id>` 分支、任务级 `wip(task-<id>)` 提交、完成产出 squash-merge commit。
   - commit message 生成：diff --stat + Top-N patch（8k token 截断）→ LLM → Conventional Commits；作者身份取用户 git config，**禁止伪造**。
   - **不实现 push**（只本地，推送交还用户）；路径 root 校验器由调用方注入（iOS 沙盒 `Documents/repos/` 规则）。
3. **工具集注册**（`AgentMode::Engineering` 白名单）：`fs_glob`/`fs_read`/`fs_write`（两段提交：变更预览→应用）/`git_diff`/`git_commit_generated`/`task_report`/`shell_run`（**默认关闭**，开启需显式配置 + 命令黑名单 rm/format/del 注册表写等 + 60s 超时）。
4. **双端 UI**：Inspector——DAG 视图（分层树 + TaskState 状态色 + attempt 计数）、diff 浏览器（FileDiff 列表）、WIP commit 时间线；样式沿用 docs/03 tokens。
5. **验收仓库 `fixtures/sample-repo/`**：一个 Rust 小项目，含 3 个红测试 + 1 个缺失 feature；端到端测试：Goal = "让测试全绿并实现 X"——mock LLM 路径验证状态机全迁移与修复环；真实 key 路径验证真完成且提交序列干净。

【红线】
- git2 一切写操作必须过 DirtyGuard；shell_run 默认关闭且任何代码路径不得自动开启。
- DAG 与 git 操作通过 SessionActor 命令接入，**不改主循环**；事件只用既有 TaskUpdated。
- verify 的 cmd 执行默认在仓库根 + 显式 allowlist 内，禁止任意命令注入执行（黑名单 + 超时双保险）。
- iOS 端 git/fs 一律沙盒内（root 校验器强制），越界即错误。

【验收】
1. 状态机迁移矩阵测试全绿：Pending→Ready→Running→Verifying→Done/Repair×3→Blocked/Failed/Skipped 每条边有用例；修复环预算与 Blocked 报告正确。
2. kill-and-resume 续跑用例绿（重启后状态与 ready 集合恢复正确）。
3. 脏树保护、路径逃逸、shell 黑名单三类安全测试全过。
4. sample-repo 真实路径（有 key 时）：测试全绿 + 提交历史符合策略（cricket/goal-* 分支 + wip(task-*) + 最终 squash），运行日志摘录入 `reports/m5-code-run.md`；无 key 则 mock 路径结论 + 待人工清单。
5. `cargo test --workspace` + clippy + deny 全绿；双端 UI 走查截图入报告。

【工作方式】
分支 `phase/m5-code`；结束 push 并输出：① 状态机覆盖矩阵；② 安全用例结果；③ sample-repo 运行结论；④ 【验收】打勾；⑤ 遗留问题。

【提示词结束】
