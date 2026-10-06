# Phase 3 · M2 iOS 合流（UniFFI + xcframework + SwiftUI）

<!-- 复制【提示词开始】至【提示词结束】之间的全部内容，粘贴给执行 AI -->

【提示词开始】

你是 Cricket 的 iOS 与 FFI 工程师。本阶段目标：完成 **M2 双端合流**——`cricket-ffi` UniFFI 门面、xcframework 构建流水线、SwiftUI 客户端，达到与 Windows 端 ≥90% 的功能对齐。施工图：`docs/01` §3 全文、`docs/03` 全文、`docs/04` §2-M2。

【环境前置】本阶段需要 **macOS + Xcode ≥16 + rust apple targets**。若当前环境**不是 macOS**：只执行任务 1–3（Rust 侧与 CI），把任务 4–6 落成 `docs/pending-ios.md` 待办清单后报告停止——禁止在非 macOS 环境硬做或伪造 iOS 构建。

【上下文加载（必须先完成）】
1. 读 `docs/01-communication.md` §3 全文——**尤其是 §3.2 的 FFI 硬性规则 R1–R8（逐条都要能对照自查）**、§3.3 构建流水线四步、§3.4 Swift 消费范式（UiEventSink→AsyncStream→MainActor）。
2. 读 `docs/03-ui-spec.md` 全文（与 Windows 同一套 tokens/布局/三件套/交互）。
3. 自检：`cargo test --workspace` 全绿再开工；若本阶段部分完成，对照【验收】只补缺。

【任务】
1. **cricket-ffi**：按 `docs/01` §3.1 实现完整导出面——`Cricket` 对象（构造器 new(config, sink) + agent/session/chat/abort/regenerate/kb/skill/goal/novel 方法族）、`EventSink` 回调接口（fire-and-forget，禁止返回值）、`CoreConfig`。全部导出方法体 `catch_unwind` 包裹，panic 翻译为 `CricketError::Internal`（R3）。uniffi 依赖在 feature "ffi" 内。
2. **构建流水线**：`scripts/build-xcframework.sh` 按 `docs/01` §3.3 四步（三 target 编译 → `uniffi-bindgen generate` → 模拟器双架构 lipo → `xcodebuild -create-xcframework`），uniffi 版本锁定并记录在 `cricket-ffi/Cargo.toml` 与报告里；`uniffi.toml` 提交。
3. **CI（macos runner）**：生成 Swift 绑定并与 `bindings/swift/` 基线 diff（绑定变更必须显式提交进 PR——这是 FFI 契约纪律）；构建 xcframework + 体积断言 ≤18MB；`swift test` 跑 EventSink round-trip（构造 Cricket → 发 mock chat → 收集事件序列 → 断言完整一致）。
4. **apps/ios**：Xcode 工程通过 Swift Package `binaryTarget` 接入 `CricketCore.xcframework`；`UiEventSink` → `AsyncStream` 桥 + `CricketEventDispatcher` 模板（回调从 Emitter 线程 → MainActor，R4/R5）。
5. **SwiftUI UI**（与 Windows 功能对齐，样式用 docs/03 同一套 tokens 的 SwiftUI 等价物）：聊天页（List 窗口化）、等待三件套（WaveDots 用 `TimelineView` 精确正弦实现 `docs/03` §5.3 Swift 代码，计时器与波浪共用一个 tick；`accessibilityReduceMotion` 降级呼吸态）、ReasoningAccordion、多会话侧栏（NavigationSplitView）、abort/regenerate、快捷键映射表（`docs/03` §6.4 的 iOS 等价物：外接键盘快捷键 + 手势替代，产出映射表）。
6. **双端对齐矩阵**：`reports/parity-matrix.md`——40 项功能（发送/流式/折叠/工具卡/abort/regenerate/变体切换/计时/暗色/快捷键…）逐项：Windows 状态 / iOS 状态 / 双端截图路径；不足 36 项达标时在报告中明确列出缺口。

【红线】
- FFI 规则 R1–R8（`docs/01` §3.2）逐条自查，报告必须附对照表。
- 禁止在 Swift 层手写第二套事件类型——一律使用 uniffi 生成的 `CoreEvent`（ADR-004）。
- 回调内禁止重活/阻塞；Swift 侧 MainActor 消费；跨边界只传值类型（R1/R6）。
- Rust 侧不得为取悦 Swift 而改动协议语义；发现契约问题 → 报告，不擅改 `docs/`。

【验收】
1. `scripts/build-xcframework.sh` 一键成功产出 xcframework，体积 ≤18MB（附数据）。
2. CI macos job 全绿：绑定 diff 基线、体积断言、`swift test` round-trip 100%。
3. iOS 模拟器演示：mock 路由流式聊天 + 三件套动画 + 折叠展开 + 中断/重生成，附截图/录屏路径。
4. 对齐矩阵 ≥36/40 项达标。
5. `cargo test --workspace` 仍全绿（Rust 侧零回归）。

【工作方式】
分支 `phase/m2-ios`；结束 push 并输出：① R1–R8 自查对照表；② 对齐矩阵；③ xcframework 体积与构建耗时；④ 遗留问题（含"待 macOS 环境补做"清单，若适用）。

【提示词结束】
