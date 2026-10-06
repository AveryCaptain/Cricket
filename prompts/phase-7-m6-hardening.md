# Phase 7 · M6 硬化与部署（SLO 达标 / 安全加固 / 生产部署材料）

<!-- 复制【提示词开始】至【提示词结束】之间的全部内容，粘贴给执行 AI -->

【提示词开始】

你是 Cricket 的性能与运维工程师。本阶段目标：完成 **M6 硬化与部署**——SLO 全表达标、安全加固、生产部署材料齐备。施工图：`docs/05` 全文、`README.md` §3（SLO 表）、`docs/03` §7（性能预算）、`docs/04` §5（质量门）。

【重要边界】服务器 `124.223.154.233` 的**实际操作（SSH 登录、装栈、演练）不在本提示词范围**：你只产出代码、脚本与 runbook，由人按文档在服务器执行。你**没有也不会被提供**服务器凭据——绝不在任何文件、脚本、示例、报告中写入真实 IP 凭据、密码、API key、PAT（`docs/05` §3 纪律）。

【上下文加载（必须先完成）】
1. 读 `docs/05-deployment.md` 全文；`README.md` §3 SLO 表；`docs/03` §7/§8；`docs/04` §5。
2. 自检：起点测试全绿；部分完成只补缺。

【任务】

**A. 性能达标（代码侧，全部可离线验证）**
1. SLO 测量脚本 `scripts/measure-slo.ps1` 与 `scripts/measure-slo.sh`：宿主 RSS、WebView2 进程 RSS、冷启动至首屏、流式帧主线程耗时、KB 检索 P95（本地 dev 栈）、mock 下 256 并发 SSE 流。每项输出机器可读结果。
2. 内存专项审计：JetBrains Mono 子集核对（latin+标点 woff2 ≤90KB，两端）；消息流节点窗口 ≤2000 实测；SQLite mmap/页缓存调优；变体折叠不入渲染树复核。
3. criterion benches：gateway 吞吐（事件/秒）、事件折叠、chunking 速度——入库 `benches/`。
4. 不达标项按"只减不增"优化；仍不达标 → 报告写明差距与取舍建议，**不许静默修改 SLO 表或测试断言**。

**B. 安全加固（代码侧）**
1. gitleaks 接入 CI（全历史扫描）；`.env.example` 占位模板与 `.gitignore` 覆盖核对。
2. PAT 生命周期：签发/吊销端点 + 端侧引导流程（签名密钥 `CRICKET_PAT_SIGNING_KEY` 只在服务器 .env）。
3. Skill HMAC 审计测试补全（时间窗/篡改/重放三负例复核）；tower 全局限流、请求体上限、SSE 连接数上限（≥256 并发达标前提下设上限防耗尽）。

**C. 生产部署材料（产出物，人执行）**
1. `deploy/` 生产套件按 `docs/05` §2 全文：`docker-compose.yml`（postgres-zh 自定义镜像 Dockerfile 编译 zhparser、server、searxng、caddy）、`Caddyfile`（**`flush_interval -1`** SSE 直通、h2/h3、HSTS）、`backup.sh` + cron 模板、`restore-drill.md` 恢复演练 runbook。
2. `scripts/provision.sh`：服务器加固**幂等脚本**（对应 `docs/05` §0 三步与 §1 清单：密钥轮换提示但脚本内不交互处理 root 密码、ed25519 引导、禁密码登录、ufw 22/80/443、fail2ban、unattended-upgrades、chrony、journald 限额）。脚本自检可 `--dry-run`。
3. `reports/m6-acceptance.md`：`docs/05` §8 验收清单逐项勾选——代码侧可验证项自动跑并附证据，服务器侧项标"待人工执行"。

【红线】
- 任何文件不得出现真实凭据（服务器密码、厂商 key、PAT、TLS 私钥）——gitleaks 是你的验收门之一。
- 回滚纪律：sqlx 迁移**只增不删两版**（旧端 iOS 需继续工作 ≥2 周，`docs/05` §7）。
- 服务器操作一律"产出 runbook 待人执行"，不假设你拥有访问权。

【验收】
1. SLO 表逐项测量数据入 `reports/m6-acceptance.md`；代码侧项全部达标（宿主 <50MB / WebView2 ≤70MB / 产物 ≤15MB / framework ≤18MB / mock 256 并发 SSE 等）。
2. 生产 compose 在本地 docker 环境 dry-run 通过（env 用占位符），`Caddyfile` 语法校验过。
3. gitleaks / cargo audit / cargo deny / clippy / 全部测试全绿。
4. `restore-drill.md` 步骤可直接照抄执行；`provision.sh --dry-run` 输出清单完整。

【工作方式】
分支 `phase/m6-hardening`；结束 push 并输出：① SLO 数据表；② 未达标项与建议；③ 待人工执行清单（服务器侧，含 `docs/05` §0 密钥轮换提醒）；④ 【验收】打勾。

【提示词结束】
