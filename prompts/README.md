# AI 接力施工提示词包 · 使用指南

> 目的：让"另一个 AI 编码代理"在无人深度参与的情况下，按阶段接力完成 Cricket 项目。提示词与 `docs/04-roadmap.md` 的里程碑一一对应，设计文档（`docs/00–05`）是唯一真源，提示词只负责下达任务与验收门。

---

## 1. 使用规则（务必遵守）

1. **一阶段一会话**：每个阶段文件整段复制（从【提示词开始】到【提示词结束】），粘贴给一个新会话的 AI 代理，并确保该代理对仓库根目录有读写权限。
2. **严格顺序**：Phase 0 → Phase 8 依序执行；**验收门不过不得进入下一阶段**（每个提示词内含可执行的【验收】清单），AI 的完成报告必须人工复核。
3. **分支约定**：每阶段在 `phase/<名称>` 分支上工作，结束时推送；由人评审后合入 `main`（建议开启 PR 分支保护）。
4. **幂等可重跑**：每个提示词都要求 AI 先做状态自检——重复执行同一阶段会"只补缺不重做"，因此中途失败后可直接原样重发。
5. **环境矩阵**：

| 阶段 | 对应里程碑 | 环境要求 |
|------|-----------|---------|
| Phase 0 | 环境就绪 | 任意 OS |
| Phase 1 | M0 基线（契约+网关） | 任意 OS |
| Phase 2 | M1 骨架（Windows 端到端） | **Windows**（Tauri/WebView2） |
| Phase 3 | M2 iOS 合流 | **macOS + Xcode**（非 macOS 环境只做 Rust/CI 侧，提示词内已定义降级路径） |
| Phase 4 | M3 知识与工具 | 任意 OS（Docker 起测试栈） |
| Phase 5 | M4 小说模式 | 任意 OS |
| Phase 6 | M5 代码模式 | 任意 OS |
| Phase 7 | M6 硬化与部署材料 | 任意 OS（服务器实操由人执行） |
| Phase 8 | M7 发布 | Windows + macOS 各一 |
| Phase 9 | 终局审计 | 任意 OS（只读） |

6. **密钥纪律（最高红线）**：任何阶段都**不要**向 AI 提供：服务器 root 密码、厂商 API key、Apple/CI 凭据。所有需要真实凭据的验证步骤都被设计为"人工执行清单"或环境变量注入。对话中已暴露过的服务器密码应在 Phase 7 之前按 `docs/05-deployment.md` §0 完成轮换。

## 2. 阶段文件索引

| 文件 | 内容 | 施工图 |
|------|------|--------|
| [phase-0-env.md](phase-0-env.md) | 环境自检、仓库就绪 | docs/00 §1/§3 |
| [phase-1-m0-baseline.md](phase-1-m0-baseline.md) | Cargo workspace、cricket-protocol、三厂商网关、golden 语料、CI | docs/01 全文、docs/04 §2-M0 |
| [phase-2-m1-skeleton.md](phase-2-m1-skeleton.md) | 内存 spike、Agent 运行时、Tauri 壳、流式聊天、Server relay 骨架 | docs/00 §4/§6、docs/01 §4/§5、docs/03 全文 |
| [phase-3-m2-ios.md](phase-3-m2-ios.md) | UniFFI 门面、xcframework 流水线、SwiftUI 客户端 | docs/01 §3、docs/03 全文 |
| [phase-4-m3-knowledge.md](phase-4-m3-knowledge.md) | pgvector 知识库、混合检索、Web 工具代理、Skill 注册 | docs/02 §3/§5、docs/05 §2(dev) |
| [phase-5-m4-novel.md](phase-5-m4-novel.md) | 世界观图谱、人物卡、一致性校验、润色管线 | docs/02 §4 |
| [phase-6-m5-code.md](phase-6-m5-code.md) | Goal-DAG、验证修复环、git2 集成 | docs/02 §6 |
| [phase-7-m6-hardening.md](phase-7-m6-hardening.md) | SLO 达标、安全加固、生产部署材料 | docs/05 全文、README §3 |
| [phase-8-m7-release.md](phase-8-m7-release.md) | Windows 分发/更新、TestFlight、灰度 | docs/04 §4、docs/05 §7 |
| [phase-9-final-audit.md](phase-9-final-audit.md) | 独立验收审计（只读） | 全部 |

## 3. 中途出问题怎么办

- **AI 报告与设计文档冲突** → 人裁决；若需改设计，修订 `docs/`（文档是真源）后再原样重跑该阶段提示词（幂等）。
- **验收不达标** → 不放行下一阶段；把差距写成追加任务贴在该阶段提示词尾部重发（提示词要求 AI 支持增量补齐）。
- **AI 跑飞/大改契约** → 丢弃该分支，回到上一验收点重跑；提示词内的红线与停止条件就是为这种情况准备的。
- **每阶段结束** → 人工跑一遍提示词中的【验收】命令（不要只信 AI 报告），确认后合并分支。
