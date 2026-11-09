# TUI examples / TUI 小示例

[English](README.md) · 中文

针对单个组件和框架特性的可运行小示例。完整工具、游戏和图形演示见
[应用示例目录](../../../examples/README_cn.md)。这里的源码导入内部 workspace crate `tui`，
仓库外的应用应改为导入 `rsille::tui`。

## 从这里开始

在仓库根目录执行。虽然源码按目录分类，Cargo 示例名称仍是平铺的：

```bash
cargo run -p tui --example hello
cargo run -p tui --example workflow
cargo run -p tui --example workflow -- --async
cargo run -p tui --example workflow -- --headless
cargo run -p tui --example windows
cargo run -p tui --features syntax-highlighting --example code_viewer
```

| 示例 | 学习内容 |
| --- | --- |
| `hello` | 最小 inline 应用，Esc 退出 |
| `workflow` | Unicode 编辑 → 防抖 → 后台保存 → 退出，支持交互和无终端模式 |
| `headless` | 无 TTY 输入回放、帧检查和手动时间 |
| `background` | 通过 `AppHandle` 接收外部消息 |
| `composition` | 独立组件实例、消息映射和作用域资源 |
| `extensions` | 虚拟日志、锚点命令面板和世界坐标画布 |
| `windows` | 浮窗、移动／缩放手势和窗口状态保留 |

## 全部示例

| 目录 | 示例目标与源码 |
| --- | --- |
| `basics/` | [workflow](basics/workflow.rs), [hello](basics/hello.rs), [headless](basics/headless.rs), [background](basics/background.rs), [composition](basics/composition.rs), [extensions](basics/extensions.rs) |
| `layout/` | [flex](layout/flex.rs), [grid](layout/grid.rs), [overlay](layout/overlay.rs), [scroll](layout/scroll.rs), [split](layout/split.rs), [stack](layout/stack.rs), [windows](layout/windows.rs) |
| `widgets/` · Controls / 输入控件 | [button](widgets/button.rs), [calendar](widgets/calendar.rs), [checkbox](widgets/checkbox.rs), [radio_group](widgets/radio_group.rs), [select](widgets/select.rs), [switch](widgets/switch.rs), [text_input](widgets/text_input.rs), [textarea](widgets/textarea.rs) |
| `widgets/` · Collections / 数据组件 | [data_table](widgets/data_table.rs), [file_explorer](widgets/file_explorer.rs), [list](widgets/list.rs), [selection](widgets/selection.rs), [tree](widgets/tree.rs) |
| `widgets/` · Content / 内容展示 | [canvas](widgets/canvas.rs), [code_viewer](widgets/code_viewer.rs), [diff_viewer](widgets/diff_viewer.rs), [label](widgets/label.rs), [loading_indicator](widgets/loading_indicator.rs), [log_viewer](widgets/log_viewer.rs), [markdown_viewer](widgets/markdown_viewer.rs), [progress](widgets/progress.rs) |
| `widgets/` · Navigation / 导航 | [command_palette](widgets/command_palette.rs), [menu](widgets/menu.rs), [tabs](widgets/tabs.rs) |
| `widgets/` · Containers / 容器与装饰 | [collapsible](widgets/collapsible.rs), [dialog](widgets/dialog.rs), [divider](widgets/divider.rs), [panel](widgets/panel.rs), [spacer](widgets/spacer.rs) |
| `motion/` | [visual](motion/visual.rs) |
| `themes/` | [theme](themes/theme.rs) |

## 应用与测试模式

使用 `App::new(state).update(update).view(view).run()`。在 `.update()` 之前配置模式、
主题、快捷键、订阅、时钟和执行器。每次更新都接收 `UpdateCtx`，调用 `ctx.exit()` 请求退出。
默认全屏，`.mode(Mode::Inline)` 使用普通屏幕。在已有的、启用了 I/O 和时间驱动的
Tokio runtime 中使用 `.run_async().await`。

`.build().size(width, height)` 创建无需终端的运行时。`step(&events)` 处理输入、消息、
计时器和绘制，通过 `state()`、`frame()`、`focused_key()`、组件状态和边界检查行为。
需要确定性时间时，从 `rsille::tui::runtime`（这些内部示例中为 `tui::runtime`）导入
`ManualClock` 和 `ManualExecutor`。推进时钟或执行队列任务后，再用 `step(&[])` 收集变化。

```bash
cargo run -p tui --example headless
cargo run -p tui --example extensions -- --headless
cargo check -p tui --examples --all-features
cargo test -p tui
```

## 可复用到应用中的模式

- **稳定身份**：需要保留的编辑器和会重新排序的条目应设置稳定的 `.key(...)`。
  key 在最近的有 key 祖先内唯一，没有 key 的子项按位置匹配。
- **组件组合**：`.map(...)` 转换子消息，视图、effect 和订阅使用相同的 `.scope(...)`，
  实例卸载时取消其资源。
- **状态保留**：保留的折叠子树保存局部状态，但不参与布局和输入；移除描述后结束生命周期。
- **受控输入**：文本和表格变化由应用状态接收。Unicode 编辑使用字素簇，布局和指针位置共用字宽策略。
- **指针输入**：命中遵循裁剪、浮层和模态隔离，悬停不移动键盘焦点；捕获手势可用 `drag_region`。
- **后台工作**：取消会拒绝过期结果，但已运行的阻塞任务需要主动配合取消。
  手动时间控制调度器，不控制任务内部任意阻塞代码。
- **图形**：Canvas 采样尺寸随编码变化。切换编码而保持世界范围时，使用显式 viewport 和
  `ctx.world(surface)`。
- **键盘能力**：按住键应用使用 `.keyboard_mode(KeyboardMode::Auto)`、`.focus_events(true)`
  和 `.on_input(...)`，仅在能力报告支持松键时维护按住状态，失焦时清空。默认仍为传统输入。

详细契约见 [app](../../../src/tui/app.rs)、[runtime](../../../src/tui/runtime.rs)、
[widget](../../../src/tui/widget.rs)、[effect](../../../src/tui/effect.rs)、
[subscription](../../../src/tui/subscription.rs) 的源码文档。

## 添加小示例

在对应分类中添加一个源码文件，并在 [Cargo.toml](../Cargo.toml) 注册 `[[example]]`。
围绕一个组件或特性组织内容；包含多页面或业务流程的完整应用放到根目录的 `examples/`。

[项目首页](../../../README_cn.md)
