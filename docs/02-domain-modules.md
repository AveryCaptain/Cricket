# 02 · 特化模块架构设计（网关 / 会话 / 知识库 / 工具 / 小说 / 代码）

> 目标读者：领域工程师。本文给出每个模块的职责边界、核心数据模型（DDL）、状态机与关键算法。存储模型分两端：**服务端 PostgreSQL（知识库真相源）**、**端侧 SQLite（会话/任务真相源）**。

---

## 1. Agent 运行时（cricket-agent）—— 一切模式的地基

### 1.1 分层模型

```
Agent（智能体）= System Prompt + 配置模板(model/温度/工具白名单/知识域) + Skill 绑定
  └── Session（多轮会话窗口）★ 一个 Agent 可并行开 N 个 Session
        └── Turn（轮次）= 用户输入 → [阶段机流转] → 消息树产出
              └── Message（消息树节点，含 regenerate 变体分支）
```

```rust
pub struct AgentSpec {
    pub name: String,
    pub system_prompt: String,           // 预设智能体的核心人格/职责
    pub model_pref: ModelPref,            // 首选模型 + 降级链
    pub temperature: f32,
    pub tool_allowlist: Vec<String>,     // 工具白名单（安全边界）
    pub kb_scopes: Vec<KbScope>,         // 绑定 Global / Agent 私有知识库
    pub mode: AgentMode,                 // General | Novel | Engineering
    pub max_output_tokens: u32,
}
```

### 1.2 会话 Actor 循环（伪代码）

```rust
async fn session_actor(mut rx: mpsc::Receiver<SessionCmd>, ctx: SessionCtx) {
    let mut tree = MessageTree::load(&ctx.store).await;       // 事件溯源折叠恢复
    while let Some(cmd) = rx.recv().await {
        match cmd {
            SessionCmd::Chat { text, .. } => {
                emit(Stage::Planning);                        // ← 等待动画的"阶段文字"
                let pack = ctx.assemble_context(&tree).await;  // system+KB+世界观察+历史
                emit(Stage::RetrievingKnowledge / Reasoning…); // 由实际阶段真实驱动
                let mut stream = ctx.gateway.stream(pack).await?;
                loop_instrumented(&mut stream, &mut tree, &ctx).await;   // 工具循环（§3.4）
                tree.persist_events(&ctx.store).await;         // append-only
                ctx.sync_push().await;                        // 服务端镜像
            }
            SessionCmd::Abort => { ctx.token.cancel(); }      // ADR-007：取消即令牌传播
            SessionCmd::Regenerate { message_id } => {
                tree.branch_at(message_id);                   // 消息树回放语义
                /* 以 parent 上下文重跑一轮 */
            }
        }
    }
}
```

### 1.3 消息树与 Regenerate

```
user: u1 ── assistant: a1(变体0, 采纳)          [主线]
                 └─ a1'(变体1, regenerate)
                 └─ a1''(变体2)
user: u2 ── assistant: a2(基于 a1 采纳变体)  ……
```

- 存储：事件溯源推导（`MessageStart` 携带 `parent_event_id`），不单独建表，天然支持任意节点回放重生成。
- 前端语义：变体横向切换（‹ 2/3 ›），regenerate 时旧消息灰化冻结、新流在其下生成；采纳哪个变体即哪个变体成为后续上下文。

### 1.4 工具循环（与 Reasoning 分离的严格顺序）

```
GatewayEvent::ToolCallStart → emit CoreEvent 原样透传（前端渲染工具卡片）
    ↓
cricket-tools::Registry.dispatch(call)
    ├─ schema 校验（jsonschema crate，失败即注入错误 ToolResult 让模型自纠）
    ├─ 白名单检查（AgentSpec.tool_allowlist）
    ├─ 执行：本地沙盒 or 远程代理（§5）
    └─ emit ToolResult{preview ≤ 3 行, duration_ms}
    ↓
结果以 role=Tool 注入会话 → 再次调用网关 → 直到 FinishReason::Stop 或 abort
```

护栏：每 Turn 工具调用 ≤ 12 次硬上限；循环引用检测（同名同参调用直接复用上次结果）。

---

## 2. 多厂商统一网关（cricket-gateway）

- 适配器矩阵与归一化映射已在 01 文档 §6 定义，此处补充**运行策略**。

| 策略 | 规格 |
|------|------|
| 重试 | 幂等的连接失败/429/5xx 指数退避（250ms 起 ×2，≤3 次，jitter）；流开始后的错误不重发请求，转 `Error{retryable:true}` 由用户重试 |
| 降级链 | `ModelPref{ primary, fallbacks[] }`；primary 不可用按链降级，事件中带 `provider` 字段供 UI 提示 |
| 计费记账 | `Usage` 累计落账（端侧 SQLite `usage_ledger` + 服务端镜像），Inspector 面板实时显示成本（§03） |
| 密钥管理 | relay 模式密钥仅存服务端 `.env`（见 05 文档）；direct 模式密钥存 OS keychain（Win Credential Manager / iOS Keychain），**严禁明文落盘** |
| 能力位 | `Capability::{Reasoning, CachePrompt, ParallelToolCalls, Vision}`；不支持的能力在请求组装期被剔除而非运行时报错 |

---

## 3. 分层知识库（cricket-memory + 服务端）

### 3.1 分层与归属

```
Global KB（全体公共）───────────┐
                               ├─→ 检索时合并（可按 Agent 配置调整权重）
Agent KB（智能体私有）─────────┘
```

### 3.2 服务端 PostgreSQL 模型（真相源）

```sql
CREATE TABLE kb_documents (
  id UUID PRIMARY KEY,
  scope_kind TEXT NOT NULL CHECK (scope_kind IN ('global','agent')),
  agent_id UUID,                              -- agent 域必填，global 为 NULL
  title TEXT NOT NULL,
  source TEXT,                                -- url / 路径 / 手录
  content_md TEXT NOT NULL,
  checksum TEXT NOT NULL,                     -- 内容指纹，增量更新判重
  version INT NOT NULL DEFAULT 1,
  created_at TIMESTAMPTZ DEFAULT now(),
  updated_at TIMESTAMPTZ DEFAULT now(),
  CHECK ((scope_kind='agent') = (agent_id IS NOT NULL))
);

CREATE TABLE kb_chunks (
  id UUID PRIMARY KEY,
  document_id UUID REFERENCES kb_documents ON DELETE CASCADE,
  seq INT NOT NULL,                           -- 文档内有序 → 相邻 chunk 合并召回
  heading_path TEXT,                          -- Markdown 标题路径（"卷一/第三章"）
  content TEXT NOT NULL,
  tsv tsvector,                               -- zhparser 中文分词全文索引
  embedding vector(1024) NOT NULL,            -- 维度=配置项，默认对齐 bge-m3；换模型需重建
  attrs JSONB DEFAULT '{}'                    -- {kind: 'text'|'code'|'dialogue', ...}
);
CREATE INDEX ON kb_chunks USING hnsw (embedding vector_cosine_ops);
CREATE INDEX ON kb_chunks USING gin (tsv);
CREATE INDEX ON kb_documents (scope_kind, agent_id);
```

### 3.3 Chunking 算法（Markdown 感知）

1. 按标题层级切块，块内代码围栏（```）**原子不切**；
2. 超长块递归二分至 ≤ 512 token，相邻重叠 64 token；
3. 对白段与正文段分别标记 `attrs.kind`（小说模式检索加权用）；
4. `checksum = sha256(chunk)`，文档更新时 diff 出受影响 chunk，最小化重建向量。

### 3.4 混合检索管线（服务端 `/api/v1/kb/search`）

```
query ──┬─→ FTS (ts_rank)                        → 排名 A
        ├─→ 向量 (1-cosine)                      → 排名 B
        └─→ 相邻合并: 命中 chunk 的 seq±1 并入候选
RRF 融合: score = Σ 1/(60 + rank_i)
→ top_k=8 → 回链 document/heading_path → ContextPack（token 预算裁剪，见 §4.5）
```

中文分词：首选 `zhparser`（SCWS）；若服务器镜像无法安装扩展，回退 `pg_trgm` 相似度（能力开关，部署期探测）。

### 3.5 增删改查协议

| 操作 | 语义 |
|------|------|
| upsert | 按 `checksum` 判增改；增量重切分；diff 后仅重嵌入受影响 chunk |
| delete | 文档级（CASCADE）或 chunk 级 |
| list/read | 服务端分页，端侧 SQLite 缓存镜像（LRU ≤ 500 文档摘要） |
| 变更广播 | 服务端发 `KbChanged{scope}` SSE → 各端缓存失效 |

---

## 4. 小说模式 · Creative Suite（cricket-novel）

### 4.1 世界观持久记忆：World-Setting Graph

图模型 = **节点（设定实体）+ 类型化边（关系）+ 叙事时间线**，全量入库且动态更新：

```sql
CREATE TABLE world_nodes (
  id UUID PRIMARY KEY,
  agent_id UUID NOT NULL,                 -- 挂在小说智能体（私有知识域）
  kind TEXT NOT NULL CHECK (kind IN
    ('location','organization','faction','item','rule','concept','timeline_event')),
  name TEXT NOT NULL,
  aliases TEXT[] DEFAULT '{}',            -- 别名：检索与交叉引用的核心
  body TEXT NOT NULL,                     -- 设定正文（Markdown，支持引用其他节点 [[id]]）
  salience REAL DEFAULT 0.5,              -- 0~1，检索加权（主线设定 > 一次性细节）
  embedding vector(1024),
  tsv tsvector,
  version INT NOT NULL DEFAULT 1,         -- 乐观锁，动态更新并发安全
  created_at TIMESTAMPTZ DEFAULT now(),
  updated_at TIMESTAMPTZ DEFAULT now()
);
CREATE INDEX ON world_nodes USING hnsw (embedding vector_cosine_ops);

CREATE TABLE world_edges (
  id UUID PRIMARY KEY,
  src UUID REFERENCES world_nodes ON DELETE CASCADE,
  dst UUID REFERENCES world_nodes ON DELETE CASCADE,
  relation TEXT NOT NULL,                 -- located_in / belongs_to / at_war_with / created_by …
  citation TEXT,                          -- 出处（"第12章 · 灯下对话"）→ 交叉引用可回查
  weight REAL DEFAULT 1.0,
  CHECK (src <> dst)
);
```

**交叉引用**：正文内 `[[节点名]]` 语法；写作时被提及实体自动高亮；每 Turn 结束由后台抽取任务提议"将新提及实体建边/新建节点"→ 用户一键确认（半自动，防止图被幻觉污染）。

### 4.2 类脑人物卡片：Persona Cards

```sql
CREATE TABLE personas (
  id UUID PRIMARY KEY,
  agent_id UUID NOT NULL,
  name TEXT NOT NULL,
  aliases TEXT[] DEFAULT '{}',
  core_traits JSONB NOT NULL,        -- {说话风格, 口头禅, 禁忌, 恐惧, 欲望}
  arc JSONB NOT NULL DEFAULT '[]',    -- 性格弧光: [{stage:'觉醒期', span:'ch1-12',
                                     --   summary:'…', voice_shift:'语速变快、多用短句'}]
  relations JSONB NOT NULL DEFAULT '[]', -- [{peer:'…', relation:'宿敌', history:[event_id…]}]
  voice_sample TEXT,                 -- 1~3 段代表性对白（风格锚点）
  salience REAL DEFAULT 0.7,
  embedding vector(1024), tsv tsvector
);

CREATE TABLE persona_events (         -- 历史事件记忆链（情节记忆）
  id UUID PRIMARY KEY,
  agent_id UUID NOT NULL,
  persona_id UUID REFERENCES personas ON DELETE CASCADE,
  session_id UUID,                    -- 溯源到产生该事件的会话
  occurred_seq BIGINT NOT NULL,      -- 叙事内序号（世界内时间，而非现实时间）
  summary TEXT NOT NULL,             -- "在第 14 章为救妹妹向敌对组织低头"
  impact REAL DEFAULT 0.5,           -- 对性格/关系的冲击度 → 决定是否重写 arc 摘要
  embedding vector(1024)
);
```

**记忆链动力学**：新事件 `impact ≥ 0.7` → 触发后台 LLM 任务**重写该人物 arc 的当前 stage 摘要**（性格不是静态表，而是被事件持续重构的链）；关系网边随事件增量追加 `history`。

### 4.3 一致性校验管线（Consistency Audit）

```
草稿章节
 ─①→ 实体抽取（LLM 结构化输出：人物/地点/规则/时间断言）
 ─②→ 图谱召回（向量+FTS+别名匹配 → 每实体 top-3 节点 + 1-hop 邻边）
 ─③→ 逐对裁决（LLM 比对断言 vs 节点正文/边/timeline → conflict | consistent | unknown）
 ─④→ ConsistencyReport { conflicts: [{claim, evidence, node_path, suggestion}],
                          missing_refs: [...], timeline_drift: [...] }
 ─⑤→ UI diff 视图：接受建议 = 回写图谱（version+1 乐观锁）；拒绝 = 标记为"作者豁免"
```

阈值：`unknown` 比例 > 30% 时提示"该章节涉及大量未建档设定"，引导补录而不是硬判冲突。

### 4.4 风格化润色（细粒度）

润色是**受限重写**而非洗稿，操作算子化：

| 算子 | 语义 | 约束 |
|------|------|------|
| `tone_shift{target}` | 声调迁移（冷峻↔温润） | 保留所有实体名与对白引号结构 |
| `pacing_pack` | 节奏压缩（删冗余副词/合并短句） | `max_change_ratio ≤ 0.4` |
| `sensory_boost` | 感官密度增强（五感补写） | 只插入不删除 |
| `dialogue_ratio{target}` | 对白/叙述比调整 | 对白说话人归属不得改变 |
| `style_lint` | 一致性风格静态检查（禁词表、重复句式、标点风格） | 纯规则，零 LLM |

输出一律 diff 视图（增/删/改三色），逐段接受。

### 4.5 上下文包（Context Pack）组装次序与预算

| 次序 | 内容 | 默认 token 预算 |
|---|---|---|
| 1 | System：智能体人格 + 风格指南（禁词/视角/时态） | ≤ 600 |
| 2 | World：按"与当前草稿相关性×salience"取 top 节点 | ≤ 1,500 |
| 3 | Personas：出场人物卡（traits + arc 当前段 + 关系摘要） | ≤ 1,200 |
| 4 | 事件链：相关人物近 N 条高 impact 事件 | ≤ 800 |
| 5 | 正文尾部：当前章节尾段续写窗口 | ≤ 2,000 |
| — | 合计 | ≤ 6,100（余量给输出，长文续写走滚动窗口） |

超预算裁剪顺序：4 → 3 中低 salience → 2 中低相关度；绝不裁剪 1 与 5。

---

## 5. 工具调用与开放 Skill（cricket-tools / cricket-skills）

### 5.1 内置工具表

| 工具 | 执行地 | 说明 |
|------|--------|------|
| `kb_search` | 服务端 | §3.4 混合检索 |
| `web_search` | 服务端 | → SearXNG（自托管，JSON API；多引擎聚合，客户端 IP 不外泄） |
| `web_crawl` | 服务端 | 抓取 → 正文抽取（readability 式）→ Markdown 化 → 入 KB（可选） |
| `world_graph_query` / `persona_query` / `world_upsert` | 服务端 | 小说模式专用（§4） |
| `fs_read/write/glob` | 端侧 | 白名单根目录内（Win：用户指定工作区；iOS：App 沙盒） |
| `git_*` | 端侧 | §6.4 封装子集 |
| `shell_run` | 端侧（默认关） | 仅 Windows、显式开启 + 命令黑名单（rm -rf/format/注册表写…）+ 超时 60s |

### 5.2 Skill = 声明式清单（开放式注册，本地或远程）

```toml
# skills/novel-consistency.toml
[skill]
id = "novel-consistency"
name = "章节一致性校验"
version = 1
scope = "agent"                     # global | agent
executor = "local"                  # local=端侧编排 | remote=服务端代理
triggers = ["校验一致性", "check consistency"]   # 可被自然语言触发
tools = ["world_graph_query", "persona_query"]   # 声明本 skill 的最小权限

[skill.input_schema]                # JSON Schema（同时暴露为 function calling 参数）
type = "object"
properties.chapter = { type = "string" }
required = ["chapter"]

[[skill.steps]]                    # 编排：prompt 模板与工具链交替
prompt = "对 {{chapter}} 执行 §4.3 管线步骤①…"
[[skill.steps]]
invoke = "world_graph_query"
with = { q = "{{steps[0].entities}}", top_k = 3 }
[[skill.steps]]
prompt = "基于步骤②结果输出结构化冲突报告…"
```

**远程 Skill**：`executor = "remote"` + `endpoint = "https://<server>/api/v1/skills/<id>/invoke"`；请求带 `X-Cricket-Signature: t=<unix_ts>,v1=<hmac_sha256>` 防重放（±300s 窗口）与篡改。Skill 仓库 = 端侧 `skills/` 目录（热加载，文件即注册）+ 服务端目录（团队共享）。

### 5.3 安全模型

- 工具白名单由 AgentSpec 声明；Skill 只能调用自己的 `tools` 声明集（最小权限）。
- 端侧文件工具路径逃逸检查（canonicalize 前缀匹配）；服务端工具统一限流（每会话 30 req/min）+ 爬虫 robots 尊重 + 域名黑名单。

---

## 6. 代码模式 · Engineering Suite（cricket-code）

### 6.1 Goal-DAG 总览

```
用户 Goal（自然语言目标 + done_criteria）
  ─①Planner(LLM, JSON Schema 约束输出)→ Task DAG（petgraph 管理）
  ─②Executor→ 依拓扑序调度（并发上限 k=4）
  ─③每任务: LLM 产出变更 → Verify 步骤运行 → 失败进入修复环
  ─④全 DAG Done → 汇总报告 + git 提交序列（§6.4）
```

### 6.2 状态机（持久化于端侧 SQLite，可中断续跑）

```
            ┌────────┐
   plan ▶   │Pending │
            └───┬────┘
        前驱Done ▼
            ┌────────┐  fail且attempts<3
            │ Ready  │◀───────────────┐
            └───┬────┘                ┌┴────────┐
            run ▼                 │ Repair  │ (LLM读取失败输出→重跑)
            ┌────────┐  fail       └┬────────┘
            │Running │──────────────┘
            └───┬────┘
            done ▼
            ┌─────────┐ pass   ┌──────┐
            │Verifying│───────▶│ Done │
            └───┬─────┘       └──────┘
       fail且预算耗尽 │  ┌────────┐   ┌────────┐
                    ▼  │Blocked │──▶│ Failed │(用户裁决后) / 用户解锁→Ready
                  ┌───────┘        └────────┘
```

```sql
-- 端侧 SQLite（会话真相源同库）
CREATE TABLE goals (
  id TEXT PRIMARY KEY, objective TEXT, constraints TEXT,
  done_criteria TEXT, state TEXT, session_id TEXT, repo_root TEXT,
  created_at INTEGER
);
CREATE TABLE tasks (
  id TEXT PRIMARY KEY, goal_id TEXT NOT NULL, title TEXT,
  prompt TEXT,                          # 执行提示（含该任务专属上下文）
  depends_on TEXT NOT NULL,             # JSON array
  verify TEXT NOT NULL,                 # JSON: [{cmd, expect_exit:0, timeout_s:60}] 或 [{llm_rubric}]
  state TEXT NOT NULL, attempts INT DEFAULT 0,
  result TEXT                           # JSON: {summary, files_touched[]}
);
CREATE TABLE verify_runs (
  id TEXT PRIMARY KEY, task_id TEXT NOT NULL,
  cmd TEXT, exit_code INT, stdout_tail TEXT, created_at INTEGER
);
```

### 6.3 Planner 输出契约（JSON Schema 硬约束）

```jsonc
{
  "goal_id": "g_2f8a",
  "risk_notes": "…",
  "tasks": [
    {
      "id": "t1", "title": "修复解析器边界条件",
      "depends_on": [],
      "prompt": "在 parser.rs …",
      "verify": [ {"cmd": "cargo test -p parser -- boundary", "expect_exit": 0, "timeout_s": 120} ]
    }
  ]
}
```

- Planner 收到的输入：Goal + done_criteria + 仓库指纹（结构树、语言、测试框架探测）+ **可选**首轮探索性对话（Agent 自主问询，"自主循环验证"= Executor 的 verify 环 + Planner 的复盘重规划钩子：若 > 30% 任务 Blocked，允许 Planner 基于失败报告生成增量 DAG）。
- Verify 失败修复环：`attempts < 3` 时把 `verify_runs` 尾部输出注入 Repair prompt；耗尽 → `Blocked` + 生成障碍报告，**不静默失败**。

### 6.4 Git 集成（git2-rs，端侧执行）

```rust
pub trait GitHost {
    fn status(&self) -> Result<WorktreeStatus>;                       // 干净度检测（操作前置条件）
    fn diff(&self, base: Option<Oid>, mode: DiffMode) -> Result<Vec<FileDiff>>; // stat+patch
    fn commit_generated(&self, task: &TaskId) -> Result<Oid>;    // diff+files→LLM→Conventional Commit
    fn branch_create(&self, goal: &GoalId) -> Result<BranchRef>;
    fn checkout(&self, target: &str, guard: DirtyGuard) -> Result<()>;       // 脏树拒绝
    fn log_graph(&self, limit: u32) -> Result<Vec<CommitInfo>>;
}
```

| 项 | 策略 |
|----|------|
| 分支 | 每个 Goal 一条 `cricket/goal-<goal_id>`；任务级 WIP commit：`wip(task-<id>): <title>`；Goal 完成产出**规整后的 squash-merge commit** |
| 自动 commit message | `git diff --stat` + 变更文件 Top-N 完整 patch（截断预算 8k token）→ LLM 生成 Conventional Commits（type/scope/subject/body），`git commit` 由 git2 执行，作者身份 = 用户 git config（不伪造） |
| 安全边界 | ① 任何写操作前 `status()` 发现脏树 → 停止并报告（绝不 stash 用户改动）；② 不执行 `push`（一期只本地，推送交还用户）；③ iOS 仅允许 App 沙盒 `Documents/repos/` 内仓库（`fs_read` 等工具同规则） |
| Diff UI | 服务端无关；端侧渲染 FileDiff 列表（03 文档组件规格） |

### 6.5 代码模式工具集（进 AgentSpec 白名单）

`fs_glob` · `fs_read` · `fs_write`（带"变更预览→应用"两段提交）· `git_diff` · `git_commit_generated` · `shell_run`（可选）· `task_report`（DAG 状态写入）

---

## 7. 模块间依赖与"模式"装配

```
AgentMode::General    → agent + memory + tools(基础)
AgentMode::Novel      → + cricket-novel（世界工具集 + ContextPack §4.5 + 校验/润色 Skill）
AgentMode::Engineering→ + cricket-code（Goal-DAG + git 工具集 + verify 环）
```

装配点：`AgentSpec.mode` 在 Session 打开时决定工具白名单、ContextPack 组装器与 Inspector 面板（03 文档）。特化模块**不改 Agent 主循环**，只做"工具 + 上下文 + 事件"三件事——这是防止范围蔓延的架构红线（对应 04 文档风险 R9）。
