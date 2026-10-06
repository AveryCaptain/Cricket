# Cricket P0 环境记录

记录日期：2026-10-06（Asia/Shanghai）
操作系统：Windows 11（`Windows 10.0.26200 (Windows 11 CoreCountrySpecific) [64-bit]`）
分支：`phase/p0-env`
仓库：`git@github.com:AveryCaptain/Cricket.git`

## 工具链版本矩阵

以下版本号均为本机命令原文输出；没有把未成功的安装伪装成成功。

| 类别 | 命令 | 版本/结果原文 |
|---|---|---|
| Rustup | `rustup --version` | `rustup 1.29.1 (d95a37b6a 2026-08-13)` |
| Rust 编译器 | `rustc --version` | `rustc 1.99.0 (b940084d 2026-09-28)` |
| Rust 主机 | `rustc --version --verbose` | `host: x86_64-pc-windows-msvc`; `release: 1.99.0`; `LLVM version: 23.1.1` |
| Cargo | `cargo --version` | `cargo 1.99.0 (5f94df478 2026-08-27)` |
| 当前 toolchain | `rustup show active-toolchain` | `stable-x86_64-pc-windows-msvc (default)` |
| 已安装 target | `rustup target list --installed` | `x86_64-pc-windows-msvc` |
| 依赖门禁 | `cargo deny --version` | `cargo-deny 0.20.2` |
| 安全审计 | `cargo audit --version` | `cargo-audit-audit 0.22.2` |
| Node.js | `node --version` | `v22.22.2` |
| npm | `npm --version` | `11.4.1` |
| Git | `git --version` | `git version 2.49.0.windows.1` |
| WebView2 | 注册表 `Microsoft Edge WebView2 Runtime` | `154.0.4258.53` |

安装动作：通过 `winget install --id Rustlang.Rustup --exact --accept-source-agreements --accept-package-agreements` 安装 Rustup；`rustup default stable`、`rustup update stable` 和 `rustup target add x86_64-pc-windows-msvc` 成功。随后 `cargo install cargo-deny cargo-audit` 中 `cargo-deny` 成功；`cargo-audit` 首次因 C: 盘空间不足（`os error 112`）失败，迁移 Cargo 临时/构建目录到 E: 盘并使用单并发重试后成功安装。

`rust-toolchain.toml` 将项目固定到当前 stable 的实际版本 `1.99.0`，并声明 Windows MSVC target。

## 仓库就绪核验

- `git remote get-url origin`：`git@github.com:AveryCaptain/Cricket.git`，与要求一致。
- `docs/00-architecture.md` 至 `docs/05-deployment.md` 六份文档均存在。
- `README.md` 已通读；`docs/00-architecture.md` §1（系统拓扑）与 §3（Cargo Workspace/crate 拆解）已通读。
- `prompts/README.md` 的环境矩阵已从 `HEAD` 核对：Phase 0 任意 OS，Phase 1 任意 OS，Phase 2 Windows，Phase 3 macOS + Xcode，Phase 4–7 任意 OS，Phase 8 Windows + macOS，Phase 9 任意 OS。
- 当前工作树在开始时已存在用户变更：`prompts/` 下 11 个已跟踪文件被删除；本阶段未恢复、修改或删除这些文件。因而工作树中的 `prompts/` 当前不齐全，但 `HEAD` 中的提示词包为 11 个文件。该既有变更未纳入本阶段提交。
- `README.md`、`docs/` 和上述既有删除均未被本阶段修改。

## 阶段范围判定

本机为 Windows 11，且 WebView2 Runtime、Rust MSVC 工具链和 Node.js 已就绪：

- **可执行**：Phase 0；Phase 1；Phase 2（Windows/Tauri/WebView2）；Phase 4–7；Phase 9。
- **部分可执行**：Phase 8 可执行 Windows 分发侧，macOS/TestFlight 侧需另一台 macOS 主机。
- **不可执行**：Phase 3 的 Xcode/SwiftUI/UniFFI iOS 构建任务，以及 Phase 8 的 macOS 发布任务；本机不是 macOS，无法提供 Xcode ≥16 和 iOS SDK 验证。

Docker 未安装也未安装；按设计文档约束，Docker 属于 Phase 4 起的测试栈依赖，本阶段不安装。

## 缺失项与补救建议

1. **macOS/Xcode/iOS SDK 缺失（环境限制）**：在 macOS 主机安装并验证 Xcode ≥16、命令行工具和 iOS 模拟器 SDK，再执行 Phase 3；Phase 8 的 macOS 签名、TestFlight 与发布验证也必须在那里完成。
2. **工作树中的 `prompts/` 被既有变更删除**：在进入后续阶段前，从版本库恢复该目录并确认 `git status` 干净；本阶段没有擅自覆盖用户变更。
3. **系统盘空间曾不足**：本次已将 Rust 构建临时目录迁至 E: 盘完成安装；后续 CI/本机构建应保留足够临时空间，或继续设置 `TEMP`/`CARGO_TARGET_DIR` 到空间充足的磁盘。
4. **未执行业务构建**：仓库当前没有要求本阶段实现的 Cargo workspace/业务代码；Phase 1 开始后再执行对应构建和测试门禁。

## 提交边界

本阶段允许新增且准备提交的文件只有：

- `rust-toolchain.toml`
- `reports/environment.md`
