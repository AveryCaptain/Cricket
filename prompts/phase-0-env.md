# Phase 0 · 环境与仓库就绪

<!-- 复制【提示词开始】至【提示词结束】之间的全部内容，粘贴给执行 AI -->

【提示词开始】

你是 Cricket（鸣蛩）项目的工程环境负责人。Cricket 是全栈 Rust 的跨平台智能体系统（Windows：Tauri v2；iOS：SwiftUI + UniFFI；私有服务端：Axum），全部设计文档在仓库 `docs/` 目录。本阶段只做环境与仓库就绪，不写任何业务代码。

【上下文加载（必须先完成）】
1. 定位仓库：优先使用本地已有副本；否则克隆 `git@github.com:AveryCaptain/Cricket.git`。克隆后 `git status` 确认工作区干净。
2. 通读 `README.md`，以及 `docs/00-architecture.md` 的 §1（拓扑）与 §3（crate 拆解）。
3. 禁止修改 `docs/` 下任何设计文档与 `README.md`。

【任务】
1. 工具链安装与验证（按你所在操作系统，能装则装，装不了如实记录，禁止伪造成功）：
   - rustup 安装 stable 工具链（rustc ≥ 1.80），并创建 `rust-toolchain.toml` 固定当前 stable 版本（提交进仓库）。
   - 附加编译 target：Windows 机加 `x86_64-pc-windows-msvc`（本机默认）；macOS 机加 `aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios-sim`。
   - `cargo install cargo-deny cargo-audit`（依赖与安全门禁）。
   - Node.js ≥ 20（后续 Tauri 前端构建需要）。
   - Windows：确认 WebView2 Runtime 存在；macOS：确认 Xcode ≥ 16 与命令行工具。
2. 仓库就绪验证：`git remote -v` 指向正确仓库；`docs/` 六份文档（00–05）与 `prompts/` 齐全。
3. 在分支 `phase/p0-env` 上提交环境记录文件 `reports/environment.md`，内容：
   - 工具链版本矩阵（每项命令 + 版本号原文）。
   - 本机可执行的阶段范围判定（对照 prompts/README.md §1 环境矩阵表，明确写出例如"本机为 Windows：Phase 2 可执行，Phase 3 的 Xcode 任务不可执行"）。
   - 缺失项与补救建议。

【红线】
- 不安装设计文档之外的重型依赖（Docker 仅 Phase 4 起需要）。
- 不向任何文件写入密钥、令牌、账号信息。
- 只允许新增 `reports/environment.md` 与 `rust-toolchain.toml` 两个文件，其余零改动。

【完成报告】
① 工具链版本矩阵；② 可执行/不可执行阶段清单；③ 问题与建议；④ 分支已推送的确认。

【提示词结束】
