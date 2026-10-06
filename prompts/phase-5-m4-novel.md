# Phase 5 · M4 小说模式（世界观图谱 / 人物卡 / 一致性校验 / 润色）

<!-- 复制【提示词开始】至【提示词结束】之间的全部内容，粘贴给执行 AI -->

【提示词开始】

你是 Cricket 的领域工程师。本阶段目标：完成 **M4 小说模式 Creative Suite**——世界观持久图谱、类脑人物卡、一致性校验管线、风格化润色。施工图：`docs/02` §4 全部；装配红线：`docs/02` §7（**特化不改主循环，只做"工具 + 上下文 + 事件"三件事**）。

【上下文加载（必须先完成）】
1. 读 `docs/02-domain-modules.md` §4 全部：§4.1 world_nodes/world_edges DDL、§4.2 personas/persona_events DDL、§4.3 一致性校验五步管线、§4.4 润色算子表、§4.5 ContextPack 组装次序与预算表。
2. 读 §7 装配红线；读 `docs/03` §4（Inspector 图谱视图组件）。
3. 自检：起点测试全绿再开工；部分完成只补缺。

【任务】
1. **服务端迁移**：`world_nodes`/`world_edges`/`personas`/`persona_events` 四表（DDL 照 §4.1/§4.2 逐字：hnsw+gin 索引、`aliases` 数组、`version` 乐观锁、`occurred_seq` 叙事序号、`salience`/`impact` 权重）。
2. **服务端端点**：world CRUD/query（向量+FTS+别名匹配、1-hop 邻边扩展）、persona CRUD、persona_events 追加；`impact ≥ 0.7` 触发后台 arc 摘要重写任务（LLM 步走可注入 trait，默认 mock）。
3. **cricket-novel**（纯逻辑 crate，全部 LLM 依赖注入化，默认 mock 可测）：
   - **ContextPack 组装器**：严格按 §4.5 次序与预算（System ≤600 / World ≤1500 / Personas ≤1200 / 事件链 ≤800 / 正文尾部 ≤2000；超预算裁剪顺序 4→3 低 salience→2 低相关，**1 与 5 永不裁剪**）。
   - **一致性校验管线**五步：实体抽取 → 图谱召回 → 逐对裁决 → ConsistencyReport（conflicts/missing_refs/timeline_drift）→ 应用回写（version+1 乐观锁）。unknown 比例 >30% 提示补录而非硬判冲突。
   - **润色算子** 5 个：tone_shift / pacing_pack / sensory_boost / dialogue_ratio / style_lint，全部输出 diff 数据模型（增/删/改三色），`max_change_ratio ≤ 0.4` 强制约束；style_lint 纯规则零 LLM。
   - **交叉引用**：`[[节点名]]` 解析 + 实体提及抽取 + **半自动建边提议流**（提议 → 用户确认 → 落库；禁止自动直写图，防幻觉污染）。
4. **工具注册**：`world_graph_query` / `persona_query` / `world_upsert` 进入 cricket-tools（`AgentMode::Novel` 白名单），经现有工具循环接入。
5. **双端 UI**：小说模式 Inspector——图谱视图（节点点击 → 编辑抽屉）、一致性报告 diff 视图（三色 + 逐段接受/拒绝，接受即回写 version+1）、润色算子面板（选区触发、diff 预览）。样式沿用 docs/03 tokens。
6. **验收语料** `fixtures/novel-sample/`：≥3 万字样例小说 + 20 个注入矛盾清单（人物死亡后复活、地点归属冲突、时间线倒流、规则违背等），端到端测试走 mock LLM 路径；另附真实 key 的人工验证脚本（不进 CI）。

【红线】
- 图谱写操作必须走提议→确认流，除非调用方显式 `confirmed=true`（服务端仍校验乐观锁）。
- **Agent 主循环零改动**：本阶段一切能力以工具 + ContextPack + 事件接入，改动 `cricket-agent` 核心循环前必须停下报告。
- 润色是受限重写：操作算子越界（超出表内约束）即为 bug，宁可拒绝不可放行。
- LLM 提示词文本集中在 `cricket-novel/src/prompts/` 单文件管理，便于 diff 与调优。

【验收】
1. mock 路径单测/集成测试全绿；一致性管线五步各有独立 fixture 测试；20 个注入矛盾在 mock 裁决下报告结构 100% 正确（召回率依赖真实 LLM 的项单独标注）。
2. ContextPack 预算测试：超长输入下裁剪顺序符合 §4.5，System 与正文尾部永不被裁。
3. 润色 diff 三色数据模型完整、change_ratio 约束生效（超限拒绝用例）。
4. （有 key 时人工跑）真实管线对样例章节产出报告，人工抽查矛盾召回 ≥90%，结果记 `reports/m4-novel-check.md`；无 key 标注待验证。
5. `cargo test --workspace` + clippy + deny 全绿；双端 UI 走查截图入报告。

【工作方式】
分支 `phase/m4-novel`；结束 push 并输出：① 管线各步测试覆盖表；② 人工抽查结果（如已跑）；③ 【验收】打勾；④ 遗留问题。

【提示词结束】
