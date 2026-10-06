# Phase 8 · M7 发布（Windows 分发 / iOS TestFlight / 灰度）

<!-- 复制【提示词开始】至【提示词结束】之间的全部内容，粘贴给执行 AI -->

【提示词开始】

你是 Cricket 的发布工程师。本阶段目标：完成 **M7 内测发布**——Windows 安装包与自动更新、iOS TestFlight、灰度观测闭环。施工图：`docs/04` §4（CI/CD）与 §2-M7、`docs/05` §7（发布与回滚次序）。

【凭据边界】Apple 开发者账号、tauri updater 签名密钥、CI secrets 一律由**人**注入 CI 环境（GitHub Secrets）——你只写流程与占位符，不接触任何真实凭据。

【上下文加载（必须先完成）】
1. 读 `docs/04-roadmap.md` §4 CI/CD 与 §2 M7 小节；`docs/05-deployment.md` §7 发布与回滚（**顺序纪律：服务端先（兼容旧端 ≥2 周）→ Windows → iOS**）。
2. 自检：起点测试全绿；部分完成只补缺。

【任务】
1. **版本规约**：workspace 版本单点管理（`[workspace.package] version`），tag `v*` 触发三端同版本发布。
2. **Windows 分发**：tauri updater 配置（NSIS 安装包 + 静态 JSON 更新 feed；feed 产物由 `deploy/` 脚本生成、上传服务端 `/static/` 路径结构按 `docs/05` §2 Caddyfile）；CI release workflow：tag → 构建 → **签名**（密钥从 Secrets 注入）→ 产物归档 + feed 更新 job。更新回滚：feed 指回旧版本即回滚。
3. **iOS**：fastlane lane（build → upload to TestFlight），App 图标/Info.plist/权限文案齐全；CI macos tag job 调用 fastlane（凭据缺失时 job 标记 `skipped` 并在报告中列为待人工项）。
4. **自托管遥测（最小可用）**：server 增 `POST /api/v1/telemetry/crash`（schema 校验 + 入 PostgreSQL 独立表）+ 一个只读看板页（crash-free 会话率、按版本聚合）；端侧**默认 opt-in**、设置内可关；**红线：零对话内容、零知识库内容**（只传崩溃栈/版本/平台/会话时长）。
5. **灰度 runbook** `docs/release-runbook.md`：10 人灰度分组与顺序、版本冻结规则、P0 定义（崩溃率 >0.5% 或数据损坏）与回滚触发条件、crash-free ≥99.5% 观测口径与周期、发布前检查单（对照 `docs/05` §7 顺序纪律）。
6. **发布就绪审计**：对 `README.md` §3 SLO 表逐项做发布前复测，结果入 `reports/release-readiness.md`。

【红线】
- 遥测隐私红线（`docs/05` §1-6 一致）：不上传对话与知识库内容——CI 加一个静态断言测试（遥测 payload schema 不含文本字段）。
- 更新 feed 必须签名校验（tauri updater 签名），密钥只存在 CI Secrets。
- 服务端先行的发布顺序不可打破；compose 迁移仍遵守"只增不删两版"。

【验收】
1. tag 触发全链 CI 绿：Windows 安装包 + 已签名更新 feed 产出；iOS TestFlight 构建产出（或明确标注"待人工注入 secrets 后触发"，并在待办里给出精确的 Secrets 名称清单）。
2. 崩溃上报 e2e：端（mock 崩溃）→ server 入库 → 看板显示，全绿。
3. `docs/release-runbook.md` 完整可执行；`reports/release-readiness.md` SLO 复测全绿。
4. 全仓测试/clippy/deny/gitleaks 全绿。

【工作方式】
分支 `phase/m7-release`；结束 push 并输出：① 发布产物清单；② 待人工执行项（Secrets/账号类，含精确名称）；③ 灰度计划摘要；④ 【验收】打勾。

【提示词结束】
