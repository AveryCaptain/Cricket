# M0 契约裁决（v1）

日期：2026-10-06
范围：`cricket-protocol` v1 契约
状态：可直接作为 Phase 1 / M0 实现依据；codex 可在此裁决基础上继续 M0 任务，**无需再就以下问题停下来**。

---

## 冲突裁决 4 项

### C1 · `CricketError` 线协议表示
- **裁决**：邻接标签 `#[serde(tag = "kind", content = "message", rename_all = "snake_case")]`；所有变体均为单字符串 newtype。
- **线协议示例**：
  ```json
  {"kind":"network","message":"connection reset"}
  {"kind":"internal","message":"provider adapter panicked and was caught"}
  ```
- **理由**：① 满足"前端按 kind 分支 / 展示 message"的消费模式；② serde round-trip 可用；③ uniffi 侧仍是 `Network(String)`，不受 serde 表示影响（ADR-004 双 derive 同体不受影响）。
- **对原文档的修订**：`docs/01-communication.md` §2 中 `CricketError` 的 `tag = "kind"` 表述更新为"邻接标签 `tag=kind, content=message`"。语义不变，只是表示形式澄清。

### C2 · 无字段 enum 的序列化形态
- **裁决**：**无字段 enum（unit variant 枚举）一律序列化为纯字符串**，不加 `tag` 包裹对象。
- **适用**：`Stage`、`Role`、`FinishReason`、`TaskState`、`AgentMode`、`RouteMode`、`ProviderId` 等。
- **带字段 enum 规则**：`CoreEvent` → `tag = "type"`；`KbScope` → `tag = "scope"`；`CricketError` → `tag = "kind"`。
- **对原文档的修订**："所有 enum 用 serde(tag=…)" 修正为"**带字段 enum 用 tag，无字段 enum 用纯字符串**"。

### C3 · `MessageStart.parent_event_id`
- **裁决**：v1 纳入。
- **字段**：`parent_event_id: Option<String>`；`None` 表示会话首条或无根分支的首条消息。
- **语义**：事件溯源消息树（regenerate / 分支回放）的父指针；结合 `session_id` 唯一确定分支位置。
- **对原文档的修订**：`docs/01` §2 的 `CoreEvent::MessageStart` 补齐 `parent_event_id` 字段。

### C4 · `Capability` 名称与语义
- **裁决**：以 `docs/02` §2 能力位语义为准，统一名称如下：
  | 变体 | 语义 | 对应厂商能力 |
  |---|---|---|
  | `Reasoning` | 思考链流式 | Anthropic thinking / OpenAI reasoning_content / Gemini thought parts |
  | `CachePrompt` | 提示词缓存记账 | Anthropic cache_read/write / OpenAI cached_tokens |
  | `ToolCalls` | 工具/函数调用 | 三家均支持 |
  | `Vision` | 图像输入 | GPT-4o / Claude Sonnet / Gemini |
- **命名修正**：文档中 `CacheLog`（笔误）→ `CachePrompt`；`ToolCall`（单）→ `ToolCalls`（与"并行工具调用"术语一致）。
- **扩展位**：`Other(String)` 保留，用于新增能力不破坏旧版本。

---

## 补齐类型定义（均属 `cricket-protocol`）

> 以下为字段级裁决。所有字段命名采用 **snake_case** 序列化为 JSON；Rust 端 `#[serde(rename_all = "snake_case")]`。全部为 `uniffi::Record`（结构体）。

### A. `AgentSpec`（创建智能体的完整规格）
| 字段 | 类型 | 默认/说明 |
|---|---|---|
| `id` | `Option<String>` | 新建时 `None`，由实现生成 `agt_xxx`；更新时必填 |
| `name` | `String` | 必填，≤64 |
| `description` | `String` | 一行简介，≤140 |
| `system_prompt` | `String` | 主 system prompt，Markdown 分段 |
| `mode` | `AgentMode` | `general / novel / engineering` |
| `model_pref` | `ModelPref` | 首选+降级链 |
| `temperature` | `f32` | 默认 `0.7` |
| `max_output_tokens` | `u32` | 默认 `4096`；`0`=无限制（不推荐） |
| `tool_allowlist` | `Vec<String>` | 工具名白名单；空 = 全部禁用 |
| `skill_allowlist` | `Vec<String>` | Skill id 白名单；空 = 全部禁用 |
| `kb_scopes` | `Vec<KbScope>` | 绑定知识库域，按顺序检索 |
| `config_extra` | `Option<String>` | 透传 JSON 字符串，各模式自解释 |

### B. `AgentBrief`（列表项，轻量）
| 字段 | 类型 | 说明 |
|---|---|---|
| `id` | `String` | |
| `name` | `String` | |
| `description` | `String` | 一行 |
| `mode` | `AgentMode` | |
| `primary_model` | `String` | ModelPref.primary 展示用 |
| `session_count` | `u32` | 活跃会话数（端侧缓存计数） |
| `updated_at_ms` | `i64` | 毫秒时间戳 |

### C. `SessionSnapshot`（会话快照 = 打开会话返回）
| 字段 | 类型 | 说明 |
|---|---|---|
| `session_id` | `String` | |
| `agent_id` | `String` | 所属智能体 |
| `title` | `String` | 首条用户消息摘要或用户命名 |
| `state` | `SessionState` | `Idle / Running / Aborted / Error` |
| `last_event_id` | `i64` | 事件溯源水位（端侧 SQLite 自增 id） |
| `message_count` | `u32` | 消息节点总数（不含变体折叠） |
| `created_at_ms` | `i64` | |
| `updated_at_ms` | `i64` | |
| `summary` | `Option<String>` | 可选的自动摘要（用于列表展示） |

### D. `SessionBrief`（列表项）
| 字段 | 类型 | 说明 |
|---|---|---|
| `session_id` | `String` | |
| `agent_id` | `String` | |
| `title` | `String` | |
| `state` | `SessionState` | |
| `preview` | `String` | 最后一条消息前 80 字预览 |
| `updated_at_ms` | `i64` | |

### E. `ModelPref`（模型偏好 + 降级链）
| 字段 | 类型 | 说明 |
|---|---|---|
| `primary` | `ModelRef` | 首选 |
| `fallbacks` | `Vec<ModelRef>` | 按序降级；空数组 = 不降级 |
| `reasoning_effort` | `Option<String>` | `None`=厂商默认；`"low" / "medium" / "high"` |
| `use_cache` | `bool` | 默认 `true`；关闭则不发送缓存标记 |

**`ModelRef`**（Record，轻量引用）：
| 字段 | 类型 | 说明 |
|---|---|---|
| `provider` | `ProviderId` | 见下 |
| `model` | `String` | 模型名，如 `"gpt-5"` / `"claude-3-7-sonnet"` |

**`ProviderId`**（无字段 enum → 字符串）：`openai` / `anthropic` / `gemini` / `openai_compatible` / `mock`

### F. `DocumentInput`（知识库文档 upsert 输入）
| 字段 | 类型 | 说明 |
|---|---|---|
| `id` | `Option<String>` | `None` 新建，`Some` 更新（按 checksum 增量） |
| `scope` | `KbScope` | global / agent:<id> |
| `title` | `String` | 文档标题 |
| `source` | `Option<String>` | url / 路径 / 手录来源 |
| `content_md` | `String` | Markdown 正文 |
| `tags` | `Vec<String>` | 可选标签，用于过滤 |
| `kind_hint` | `Option<String>` | `text / code / dialogue`，chunk 属性提示 |

### G. `KbHit`（检索命中）
| 字段 | 类型 | 说明 |
|---|---|---|
| `document_id` | `String` | |
| `chunk_seq` | `u32` | 文档内 chunk 序号 |
| `title` | `String` | 文档标题 |
| `heading_path` | `Option<String>` | Markdown 标题路径，如 `"卷一/第三章"` |
| `snippet` | `String` | 高亮摘要（纯文本，`<b>` 标记命中词） |
| `source` | `Option<String>` | 回链来源 |
| `score` | `f32` | 融合分数 0~1，用于排序展示 |

### H. `Attachment`（消息附件，v1 仅文本/代码/图片引用）
| 字段 | 类型 | 说明 |
|---|---|---|
| `id` | `String` | |
| `kind` | `AttachmentKind` | `text / code / image / file` |
| `name` | `String` | 文件名或标题 |
| `mime` | `String` | MIME type |
| `size_bytes` | `u64` | |
| `content_text` | `Option<String>` | 文本/代码附件的内容（≤64KB）；大文件走 url |
| `content_url` | `Option<String>` | 大文件/图片引用地址（服务端或 data:） |
| `language` | `Option<String>` | 代码附件的语言（`rust`/`python`/…） |

**`AttachmentKind`**（无字段 enum → 字符串）：`text` / `code` / `image` / `file`

### I. 配套补充 enum

- **`AgentMode`**：`general` / `novel` / `engineering`
- **`RouteMode`**：`relay` / `direct` / `mock`
- **`SessionState`**：`idle` / `running` / `aborted` / `error`

---

## 对后续编码 AI 的直接指令

1. **以上裁决即契约**：直接作为 `cricket-protocol` 的实现依据；
2. **无需再停下来就这些问题请示**；如遇**文档与本裁决冲突**，以本裁决为准；
3. **文档修订**：M0 完成后由你或我发起 PR 同步修订 `docs/01` §2 与 `docs/02` §2，使之与本裁决一致；M0 内不必等文档修订再写代码（裁决优先）。
4. M0 验收目标保持不变：`cargo test --workspace` 全绿、golden ≥30、clippy -D warnings、deny 通过、CLI mock 冒烟、CI 两平台跑通。
