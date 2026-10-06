# 03 · 端侧 Codex 风格 UI 与波浪等待动画实现规格

> 目标读者：端侧工程师与设计师。风格基线：**Codex / IDE**——暗色高对比、紧凑信息密度、无边框悬浮容器、精炼工具栏。实现端：Windows（WebView2 内 HTML/CSS/JS，零重框架）与 iOS（SwiftUI 原生）。本文所有 token 双端必须一致落地。

---

## 1. 设计原则

1. **信息密度优先**：间距服务于扫读效率，13px 基准字号，不留"呼吸感冗余"。
2. **状态可见**：任何超过 300ms 的等待必须有阶段反馈（本规范 §5 的三件套），禁止无解释的 spinner。
3. **流式优先**：UI 的第一公民是"正在生成的消息"，渲染路径（§7）为它让路。
4. **键盘优先**：高频操作全部有快捷键（§6.4），鼠标是第二路径。

---

## 2. Design Tokens（双端统一）

### 2.1 色板（暗色 · 高对比）

| Token | 值 | 用途 | 对比度 vs bg-surface |
|-------|----|----|----|
| `bg-base` | `#0B0E13` | 应用底 | — |
| `bg-surface` | `#10141B` | 面板/消息容器 | — |
| `bg-elevated` | `#151B24` | 悬浮层/代码块 | — |
| `bg-hover` | `#1B2330` | 悬停态 | — |
| `border` | `#232C3A` | 1px 容器描边 | — |
| `border-soft` | `#1A212C` | 内部分隔线 | — |
| `fg-primary` | `#E8EDF4` | 正文 | 15.6:1 |
| `fg-secondary` | `#9AA6B8` | 次要/阶段文字 | 7.0:1 |
| `fg-muted` | `#5D6B7E` | 辅助（≥ 大字号才允许正文用） | 3.6:1（仅装饰性） |
| `accent` | `#4DA3FF` | 主强调：波浪点/链接/进行中 | — |
| `success` | `#3FB950` | 完成/校验通过 | — |
| `warning` | `#D6A431` | 阻塞/部分成功 | — |
| `error` | `#F8564F` | 失败/错误 | — |
| `reasoning` | `#8A7BE8` | 思考链专属紫 | — |
| `tool` | `#2FBFA8` | 工具调用专属青 | — |

> 正文对比度全部 ≥ 7:1（超过 WCAG AA 的 4.5:1 要求一档），`fg-muted` 只用于装饰性文本与禁用态。

### 2.2 字体栈

| Token | Windows | iOS |
|-------|---------|-----|
| `font-mono` | `"JetBrains Mono", "Cascadia Mono", Consolas, monospace` | `"JetBrains Mono"`（bundle woff2/ttf 子集）→ 系统 `SF Mono` 兜底 |
| `font-ui` | `"Microsoft YaHei", "PingFang SC", "Noto Sans CJK SC", sans-serif` | `-apple-system, "PingFang SC", sans-serif` |

- **分工**：西文、代码、数字（含计时器、token 计数）一律 `font-mono`；中文正文 `font-ui`。混排时 mono 作为第一优先（西文字符命 mono，中文字形回退到雅黑/PingFang，视觉尺寸自然对齐）。
- **尺寸**：UI 基准 13px / 消息正文 13.5px·行高 1.65 / 代码 12.5px·行高 1.6 / 标题 14.5–16px / 状态栏 11.5px。
- 子集化：JetBrains Mono 仅保留 Latin + 标点 + 数字（woff2 ≈ 90KB），中文永远用系统字体（微软雅黑为系统随附，零包体成本；iOS PingFang 同理）。

### 2.3 间距 / 圆角 / 动效

| 类别 | Token |
|------|-------|
| 间距 | 4px 网格：`4 / 8 / 12 / 16 / 24 / 32` |
| 圆角 | 输入 `4px`、卡片 `6px`、面板 `10px`、胶囊 `999px` |
| **无边框悬浮容器** | 默认 `bg-surface + 1px border-soft`，**无投影**；仅模态/菜单浮层用 `0 8px 24px rgba(0,0,0,.4)` |
| 动效时长 | 微交互 `120ms`（hover/点击）/ 状态切换 `150ms`（阶段文字 crossfade）/ 面板 `220ms`；缓动统一 `cubic-bezier(.45,.05,.55,.95)` |
| 焦点环 | `outline: 1px solid accent; offset 2px`（键盘可达性，§6.4） |

---

## 3. 布局规格

```
┌ Toolbar · 36px ────────────────────────────────────────────────────────────┐
│ ◉ 鸣蛩   [智能体 ▾] [模型 ▾]   …spacer…   ⌘K 命令面板   ⚙ 设置   ⟳ 同步状态│
├──────────┬─────────────────────────────────────────────┬──────────────────┤
│ AgentRail│ SessionTabs · 28px（可滚动，中键关闭）          │ Inspector · 320px │
│ 240px    │ ┌─ MessageStream（虚拟滚动窗口）─────────────┐ │ ┌──────────────┐ │
│ 可折叠    │ │  user 消息（右对齐色调框）                 │ │ │ 上下文统计    │ │
│ ┈┈┈┈┈┈  │ │  ┌ reasoning ▸ 已深度思考 · 12.3s ──────┐  │ │ │ · 检索命中    │ │
│ ◈ 小说家 │ │  └──────────────────────────────────────┘ │ │ │ · token 记账 │ │
│   ├ ch.12│ │  ┌ assistant（流式渲染 + 工具卡内联）─────┐ │ │ │ · 模型/延迟  │ │
│   └ ch.13│ │  │   ▸ ⚙ kb_search ✓ 210ms              │ │ │ ├──────────────┤ │
│ ◈ 码农   │ │  └──────────────────────────────────────┘ │ │ │ Goal-DAG 视图 │ │
│          │ │  ┌ 等待三件套（生成中）──────────────────┐ │ │ │ / 图谱视图   │ │
│ ＋ 智能体 │ │  │ 检索知识库…  ●●●●  00:07.4           │ │ │ └──────────────┘ │
│ ＋ 会话  │ │  └───────────────────────────────────────┘ │ │                  │
│          │ └───────────────────────────────────────────┘ │                  │
│          │ ┌ Composer · min 44px 自适应 ────────────────┐ │                  │
│          │ │ ▸ 输入… ⏎发送 · ⇧⏎换行 · ⎋停止  [📎][口]  │ │                  │
│          │ └────────────────────────────────────────────┘ │                  │
├──────────┴───────────────────────────────────────────────┴──────────────────┤
│ StatusBar · 22px： ● 已连接 · relay 3ms · gpt-5 · 本轮 1,247 tok · ¥0.012     │
└──────────────────────────────────────────────────────────────────────────────┘
```

- 响应式：`< 1100px` Inspector 自动叠进右侧抽屉；`< 760px` AgentRail 折叠为图标列。
- 双端差异：iOS 布局同构，AgentRail 变为 `NavigationSplitView`，消息区 `List` 窗口化。

---

## 4. 核心组件规格

### 4.1 消息（无边框悬浮容器）
- 用户消息：右对齐胶囊（`bg-elevated`，圆角 10px，max-width 78%）；助手消息：全宽无框块，仅左侧 2px `border-soft` 竖线区隔角色。
- 消息操作（hover 显示）：`复制 · 重新生成 · 编辑分支 · 删除变体`；变体切换器 `‹ 2/3 ›`（§6.2）。

### 4.2 ReasoningAccordion（思考链折叠）

```
┌ 5px ┬ 2px 紫色竖轨 ┐
 ◍  已深度思考（12.3s · 3,182 tok）      ▸   ← 折叠态：reasoning 紫 + 汇总数据
──────────────────────────────────────
 流式思考文本… 12.5px mono·reasoning 色，行高 1.6
```
- 生成中：默认**展开**但高度限 40vh（自动跟随滚动）；`ReasoningDelta` 停止 1.5s 后自动折叠为摘要头。
- 头部信息由端侧从事件推导：耗时 = 首 ReasoningDelta → MessageEnd；tokens = `usage.reasoning_tokens`。

### 4.3 ToolCallCard（工具调用卡）

```
┌───────────────────────────────────────────────┐
│ ⚙ kb_search                       ✓ 210ms     │ ← 状态色: 进行中(accent)/成功(success)/失败(error)
│ ▸ 参数 {"query":"鸣蛩 世界观", "top_k":8}      │ ← 默认折叠，点击展开 JSON（mono 12px）
│ ▸ 结果 preview ≤ 3 行 · "命中 5 chunks [ch3,ch7…]"│
└───────────────────────────────────────────────┘
```

### 4.4 其他
- **CommandPalette（⌘K）**：模糊搜索"智能体/会话/工具/设置/操作"，`140ms` 淡入，`bg-elevated + 阴影`。
- **Inspector**：上下文统计（检索命中条目、ContextPack 预算占用条形图）、token/成本记账、按模式切换 Goal-DAG 视图（petgraph 布局为简化分层树）或世界观图谱视图（节点点击跳转编辑）。

---

## 5. 等待体验三件套（Wave Dots + 阶段文字 + 精确计时器）★

### 5.1 结构与布局

```
┌─────────────────────────────────────────────────────┐  高 24px，内边距 8px×12px
│  检索知识库…        ● ● ● ●         00:07.4          │  bg-surface + border-soft
│  └阶段文字          └波浪点          └精确计时        │
└─────────────────────────────────────────────────────┘
   13px·fg-secondary   4×5px 点·accent   12px·mono·fg-muted
   三组间 12px；整体在消息流中占位 = "正在生成的下一条消息"
```

### 5.2 状态机绑定（事件 → 视图）

| CoreEvent | 三件套行为 |
|---|---|
| `SessionStarted` | 隐藏 |
| `StageChanged{stage}` | 阶段文字更新（150ms crossfade）；文字映射表见下 |
| `MessageStart` | 计时器 **t0 = performance.now()**（以本地时钟为准，不依赖网络事件间隔） |
| `ReasoningDelta/TextDelta` | 阶段文字退场（"生成中…"保留），计时器继续 |
| `ToolResult{duration_ms}` | 卡片刻画该工具耗时 |
| `MessageEnd / Stage{Aborted,Failed,Done}` | 计时器停止；结果沉淀为 ReasoningAccordion 头部摘要；三件套移除 |

**阶段文案映射**（`stage` → 文案，文案可本地化）：

| stage | 文案 |
|---|---|
| `connecting` | 连接模型网关… |
| `planning` | 规划回复结构… |
| `retrieving_knowledge` | 检索知识库… |
| `reasoning` | 深度思考中… |
| `calling_tool{name}` | 调用工具 {name}… |
| `executing` | 执行任务… |
| `verifying` | 验证执行结果… |
| `streaming` | 生成中… |

### 5.3 Wave Dots —— 正弦波浪形跳动点

**规格**：4 个点，直径 5px，间距 5px；颜色 `accent`；**相邻点相位差 90°**（视觉呈现"波浪行进"）；振幅 5px；周期 1.2s。

**CSS 实现（Windows）**（正弦的 keyframe 近似，45° 错相 = 0.15s stagger）：

```css
.wave-dots { display: inline-flex; gap: 5px; align-items: center; height: 24px; }
.wave-dots span {
  width: 5px; height: 5px; border-radius: 50%;
  background: var(--accent);
  animation: cricket-wave 1.2s cubic-bezier(.45,.05,.55,.95) infinite;
}
.wave-dots span:nth-child(2) { animation-delay: .15s; }
.wave-dots span:nth-child(3) { animation-delay: .30s; }
.wave-dots span:nth-child(4) { animation-delay: .45s; }
@keyframes cricket-wave {          /* 0%→35%→70%→100% 采样正弦半波 */
  0%, 100% { transform: translateY(0);      opacity: .45; }
  35%      { transform: translateY(-5px);   opacity: 1;   }
  70%      { transform: translateY(0);      opacity: .55; }
}
@media (prefers-reduced-motion: reduce) {
  .wave-dots span { animation: cricket-breathe 1.6s ease-in-out infinite; animation-delay: 0s !important; }
  @keyframes cricket-breathe { 0%,100% { opacity: .3; } 50% { opacity: 1; } }
}
```

**SwiftUI 实现（iOS，精确正弦）**：

```swift
struct WaveDots: View {
    private let t0 = Date()
    var body: some View {
        TimelineView(.animation(minimumInterval: 1.0 / 60.0)) { ctx in
            HStack(spacing: 5) {
                ForEach(0..<4, id: \.self) { i in
                    Circle()
                        .frame(width: 5, height: 5)
                        .foregroundStyle(Color(hex: 0x4DA3FF))
                        .offset(y: offset(index: i, now: ctx.date))
                }
            }
        }
    }
    /// 精确正弦：y_i(t) = -5 · sin(2π t/T + i·π/2)，T=1.2s，相邻相位差 90°
    private func offset(index: Int, now: Date) -> CGFloat {
        let t = now.timeIntervalSince(t0)
        return CGFloat(-5 * sin(2 * .pi * t / 1.2 + Double(index) * .pi / 2))
    }
}
// reducedMotionAccessibility 时改用 0.6 透明度呼吸动画（iOS: accessibilityReduceMotion）
```

### 5.4 Precise Elapsed Timer —— 精确流逝计时器

**格式**：`mm:ss.d`（分:秒.十分位），≥ 1h 切 `h:mm:ss.d`；等宽 `JetBrains Mono` + `tabular-nums`，杜绝数字抖动。

**Windows（rAF + 100ms 节流）**：

```js
function startElapsed(el) {
  const t0 = performance.now(); let last = -1e9; let raf;
  const fmt = ms => {
    const s = ms / 1000;
    const m = Math.floor(s / 60), sec = Math.floor(s % 60), d = Math.floor((s * 10) % 10);
    return `${String(m).padStart(2,'0')}:${String(sec).padStart(2,'0')}.${d}`;
  };
  (function tick() {
    const ms = performance.now() - t0;
    if (ms - last >= 100) { last = ms; el.textContent = fmt(ms); }   // 100ms 节流
    raf = requestAnimationFrame(tick);
  })();
  return () => { cancelAnimationFrame(raf); return fmt(performance.now() - t0); }; // 停止并返回终值
}
```

iOS 用同一公式在 `TimelineView` 内计算（与 WaveDots 共用一个 tick，杜绝双定时器漂移；整体通过 `reduceMotion` 停掉波浪但保留计时）。

---

## 6. 关键交互细节

### 6.1 流式中断（abort）
- Composer 生成中变身为"停止"控件（`⎋` 或点击按钮）；点击 → invoke `abort`。
- 反馈链：按钮即时进入"停止中…"（禁用态）→ 收到 `Stage{Aborted}`/`StreamClosed` 后已生成文本保留并标注"已中断"标记（灰点角标）。

### 6.2 Regenerate / 变体
- 触发点：消息操作按钮（`↻`）或流式失败后的"重试"。
- 语义：在**该消息的父上下文**上重跑（02 文档 §1.3）；旧消息保留为变体，可 `‹ n/m ›` 横向切换；切换后新会话上下文以当前采纳变体为准。
- 服务端空转保护：同 session 并发 regenerate 仅允许 1 个（Session Actor 串行化，天然满足）。

### 6.3 自动滚动锁定
- 用户上滚离开底部 > 80px → 锚定解除，出现"↓ 回到底部"浮标；生成期间不强拉滚动。
- 折叠面板展开/收起不改变视口锚点（`scroll-anchor`：消息流以"锚定到最新消息"实现）。

### 6.4 快捷键

| 组合 | 动作 |
|---|---|
| `⌘/Ctrl + K` | 命令面板 |
| `⌘/Ctrl + N` | 当前智能体新建会话 |
| `⌘/Ctrl + 1..9` | 切换会话 Tab |
| `Enter` / `Shift+Enter` | 发送 / 换行 |
| `Esc` | 停止生成 / 关闭浮层 |
| `⌘/Ctrl + Shift + R` | 对最后回复 regenerate |
| `⌘/Ctrl + B` | 折叠 AgentRail |
| `⌘/Ctrl + I` | Inspector 开关 |

---

## 7. 性能预算（流式渲染路径）

| 项 | 预算 | 手段 |
|----|------|------|
| 消息流 DOM/视图节点 | ≤ 2,000 | 虚拟滚动：视口外 ±5 条窗口化， IntersectionObserver 驱动（Windows）；iOS 用 `List` + `identification` 惰性化 |
| 每帧主线程 | ≤ 2ms | Delta 合帧已在 Core（16ms）；端侧"文本增量 append + textContent patch"，**禁止**全量 diff 重渲染 |
| Markdown 渐进解析 | 流式期间只做**惰性标记**（换行/代码围栏检测），复杂高亮推迟到 `MessageEnd` | 流式期间 `white-space: pre-wrap` 纯文本渲染 |
| 图片/资源 | 运行时零外链 | 全部静态资产随包 |
| 内存 | WebView2 ≤ 70MB / SwiftUI ≤ 60MB | 节点窗口化 + 变体折叠态不入渲染树 |

---

## 8. 可访问性

- 全部交互控件可 Tab 聚焦，焦点环 `accent`（§2.3）；`aria-live="polite"` 播报阶段文字切换。
- `prefers-reduced-motion`：波浪点降级为呼吸式透明度动画，计时器保留（信息不损失）。
- 色板含 4 类语义色，同时对色弱提供图标冗余（✓/!/✗ + 状态文字，不以颜色为唯一通道）。
