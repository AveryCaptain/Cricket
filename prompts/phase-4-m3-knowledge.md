# Phase 4 · M3 知识与工具面（pgvector 知识库 / Web 工具代理 / Skill 注册）

<!-- 复制【提示词开始】至【提示词结束】之间的全部内容，粘贴给执行 AI -->

【提示词开始】

你是 Cricket 的后端工程师。本阶段目标：完成 **M3 知识与工具面**——分层知识库（RAG 混合检索）、Web Search/Crawl 代理、开放式 Skill 注册。施工图：`docs/02` §3（KB）与 §5（工具/Skill）、`docs/00` §4 Flow C、`docs/05` §2（本阶段做 dev 变体）。

【上下文加载（必须先完成）】
1. 读 `docs/02-domain-modules.md` §3 全部（分层归属、服务端 DDL、chunking 算法、混合检索管线、CRUD 协议）与 §5 全部（内置工具表、Skill manifest、安全模型）。
2. 读 `docs/05-deployment.md` §1（端口纪律：数据库/内部服务只绑 127.0.0.1）与 §2（compose 结构，做 dev 变体）。
3. 自检：`cargo test --workspace` 全绿再开工；部分完成则只补缺。

【任务】
1. **迁移**（sqlx migrate）：`kb_documents` / `kb_chunks` DDL 照 `docs/02` §3.2 逐字落地（含 scope CHECK 约束、hnsw 余弦索引、gin tsvector 索引）；zhparser 不可用时回退 pg_trgm 的**能力开关**（部署期探测，代码双路径都留测试）。
2. **嵌入服务抽象**：`EmbeddingProvider` trait + OpenAI 兼容实现（`/v1/embeddings`，reqwest/rustls）+ 维度配置 `KB_EMBED_DIM`（默认 1024）；提供确定性 mock 实现供全部测试使用（真实嵌入只走人工验证清单）。
3. **chunking**：按 `docs/02` §3.3——Markdown 标题层级切分、代码围栏原子、超长递归二分 ≤512 token、相邻重叠 64、对白/正文打 `attrs.kind` 标记、`checksum` 增量（更新文档时只重嵌入受影响 chunk）。
4. **混合检索端点** `POST /api/v1/kb/search`：FTS ∥ 向量 → 命中 chunk 的 `seq±1` 邻接合并 → RRF(k=60) → top_k=8 → 回链 document/heading_path。配套 golden 检索集 `fixtures/retrieval/`（≥20 个中文 query→期望命中对）。
5. **KB CRUD + 变更广播**：upsert（checksum 判增改）/delete/list/read 端点 + `KbChanged{scope}` SSE 广播；端侧 `cricket-memory`：服务端客户端 + SQLite 缓存镜像（LRU ≤500 摘要）+ 失效处理。
6. **工具代理端点**：`/api/v1/tools/web_search`（转发 SearXNG JSON API）、`/api/v1/tools/crawl`（抓取 → readability 式正文抽取 → Markdown 化，可选入 KB）；tower 限流每会话 30 req/min；robots 尊重 + 域名黑名单。
7. **cricket-tools**：注册表 + jsonschema 参数校验 + `AgentSpec.tool_allowlist` 执行点 + 端侧路径逃逸检查（canonicalize 前缀匹配）；内置 `kb_search`/`web_search`/`web_crawl`/`fs_read`/`fs_write`/`fs_glob`。
8. **cricket-skills**：TOML manifest 解析（以 `docs/02` §5.2 示例为 schema 依据）+ 本地编排引擎（`prompt`/`invoke` 步骤交替、`{{}}` 模板插值、skill 声明工具即其最大权限）+ 远程 executor HMAC 签名（`X-Cricket-Signature: t=…,v1=…`，±300s 时间窗）。
9. **dev 栈**：`deploy/docker-compose.dev.yml`（postgres pgvector + searxng + server + 可选 caddy dev）；一键起栈脚本 + healthz 探活；CI 增加栈级集成测试（GitHub Actions service 容器或 compose action）。
10. **usage_ledger**（`docs/02` §2 记账）：服务端累计 + `/api/v1/usage` 查询端点，为端侧 Inspector 提供数据。

【红线】
- Postgres/SearXNG 只绑 127.0.0.1；密钥只进环境变量（compose 用 secrets 文件挂载，占位符模板入库，真值不入库）。
- 检索质量的任何改动（分词/权重/融合参数）必须跑 golden 检索集量化对比，报告给出前后命中率。
- 端侧文件工具在 iOS 沙盒规则尚未落地前，路径检查以注入的 root 校验器为准（Phase 6 完善）。

【验收】
1. dev 栈一键起 + 集成测试全绿（无外网依赖：SearXNG 不可达时测试走 stub）。
2. KB 端到端（上传→切分→mock 嵌入→检索）本地测量脚本记录 P95 ≤800ms，写入 `reports/latency-m3.md`。
3. golden 检索集 ≥20/20 命中预期（mock 嵌入 + 真实 Postgres FTS）。
4. Skill：本地编排 e2e 绿；远程 HMAC 三个负例全过（时间窗超时、签名篡改、重放）。
5. `cargo test --workspace` + `cargo clippy -D warnings` + `cargo deny` 全绿。

【工作方式】
分支 `phase/m3-knowledge`；小步提交；结束 push 并输出：① 变更摘要；② 检索质量对比表；③ 延迟数据；④ 【验收】打勾；⑤ 遗留问题。

【提示词结束】
