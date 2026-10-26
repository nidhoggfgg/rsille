# rsille v3

面向 Rust 的终端应用框架，提供组件组合、Unicode 编辑、布局、动画、后台任务和盲文图形。
v3 仍在开发，本分支使用下列接口。[English](README.md) · [迁移说明](MIGRATION_V3.md)

```toml
[dependencies]
rsille = "3.0.0"
```

```rust,no_run
use rsille::tui::prelude::*;

fn main() -> WidgetResult<()> {
    let exit = App::new(0)
        .mode(Mode::Inline)
        .update(|count, (), _ctx| *count += 1)
        .view(|count| button(format!("计数：{count}")).on_click(|| ()))
        .run()?;
    println!("最终计数：{}；退出原因：{:?}", exit.state, exit.reason);
    Ok(())
}
```

在 `.update()` 前配置主题、订阅、快捷键、时钟和执行器。统一更新函数接收
`UpdateCtx`，可发送消息、调度任务、设置 debounce，以及通过 `ctx.exit()` 退出。
默认全屏；`Mode::Inline` 在普通终端屏幕内显示。默认 Esc 退出，组件先获得事件处理机会。
退出时卸载组件并取消资源，再返回包含最终状态和 `ExitReason` 的结果。

用 `.update(update).view(view).build()` 创建无 TTY 运行时，通过 `step(&events)`
输入事件，`handle().send(message)` 发送外部消息。可读取状态、帧、焦点、组件状态、任务和布局几何；
`into_exit()` 结束运行并取回状态。测试工具从 `rsille::tui::runtime` 显式导入
`ManualClock`、`ManualExecutor`，推进时间和执行任务后调用 `step(&[])`。

需要独立按住多个键时，在 `.update()` 前设置 `.keyboard_mode(KeyboardMode::Auto)`、
`.focus_events(true)` 和 `.on_input(...)`。输入回调先收到终端启动时的
`InputEvent::Capabilities`，再收到保留 Press / Repeat / Release 和焦点变化的
`InputEvent::Terminal`，不会因组件消费事件而漏报。只有 `key_releases` 为真时才维护
持续按住状态，并在失焦时清空，按 `FrameInfo.delta` 推进动作。传统终端自动降级，
无法保证准确松手和独立多键按住；框架默认仍为 Legacy 模式。
无 TTY 测试可用 `runtime.set_input_capabilities(...)` 注入能力。
`on_key` 保留 Press / Repeat 行为；内置组件、快捷键、焦点导航和退出不因 Release 激活，
按钮和布尔开关只在 Press 时激活。

嵌入已启用 I/O 和时间驱动的 Tokio 环境时使用 `.run_async().await`。同步 `.run()`
自行创建 runtime，在 Tokio 内调用会返回错误。UI 状态与回调可留在当前线程，无需
`Send`、`Clone` 或 `Debug`；消息因跨线程入口需要 `Send + 'static`，无需 `Clone` 或 `Debug`。
`build()` 存储回调，因此要求回调为 `'static`。

从仓库根目录运行：

```bash
cargo run -p tui --example hello
cargo run -p tui --example workflow
cargo run -p tui --example workflow -- --headless
cargo run -p tui --example composition
cargo run --manifest-path examples/tui-showcase/Cargo.toml
cargo run -p tui --example extensions -- --headless
cargo run --manifest-path examples/rsille-showcase/Cargo.toml
cargo run --release --manifest-path examples/neon-observatory/Cargo.toml
cargo run --release --manifest-path examples/folded-harbor/Cargo.toml
cargo run --release --manifest-path examples/minecraft/Cargo.toml
```

`workflow` 使用同一套应用逻辑验证 Unicode 粘贴、debounce、后台保存、键盘焦点和退出，
无 TTY 模式不使用真实睡眠。`extensions` 演示十万行虚拟日志、锚点模态命令面板，
以及支持缩放、平移和指针交互的世界坐标 Canvas。[完整示例和框架契约](packages/tui/examples/README.md)。

[TUI 综合组件展厅](examples/tui-showcase/README.md) 在七个页面中集中展示表单、数据组件、
内容查看器、布局、动画和五套主题，并提供命令面板与对话框交互。

[霓虹观测站](examples/neon-observatory/README.md) 以 inline 模式展示环形行星、光子隧道、霓虹山谷、星系、金属环面和极光，
包含渐变与揭幕特效、键鼠控制和按窗口尺寸调整的布局，并附框架能力评估与无 TTY 验证入口。

[折叠星港](examples/folded-harbor/README.md) 把视角变成解谜机关：旋转立体悬浮建筑，
让不同深度的通道口在投影中重合，再折叠跨越。包含三章九道机关、光照与光晕、
跟随三维位置的可点击组件、章节溶解、键鼠观察和完整自动演示。

[Minecraft 方块世界](examples/minecraft/README.md) 是一个第一人称终端沙盒：草地、橡树、湖泊和小木屋，
加上准星、快捷栏、挖掘放置、碰撞跳跃、飞行与昼夜切换，直接运行即可游玩。

对外只发布 **`rsille`**，使用以下三个公开模块：

| 模块 | 内容 |
| --- | --- |
| `rsille::tui` | 应用、组件、布局、effect |
| `rsille::render` | 终端 buffer、Unicode 文本、渲染 |
| `rsille::canvas` | 盲文图形，包括 `Canvas` |

仓库内保留 `tui`、`render`、`canvas` 三个独立 workspace crate，均设置 `publish = false`，
仍可分别编译和测试。其入口复用 `src/tui`、`src/render`、`src/canvas` 中的实现；
顶层 `rsille` 直接将同一份实现编译为模块。发布包自包含，不依赖内部 crate，也不生成或维护重复源码。
v2 的根路径类型已移入相应模块。绘图通过 `render::Builder` 输出，参见英文首页的示例。

默认 feature 集为空；启用 `syntax-highlighting` 后使用 Syntect 代码高亮。
未启用时代码查看器仍提供纯文本显示、布局与键盘导航。showcase 显式启用此功能。

验证命令：

```bash
cargo test --workspace
cargo test --workspace --all-features
cargo check --workspace --all-targets --all-features
python3 scripts/check-release.py --offline
```

发布检查仅生成一个 `rsille` 包，执行 Cargo 的打包验证，再对解压内容运行测试，
并用独立消费应用分别验证默认和全部 feature、完整 TUI 流程以及 Canvas 与渲染类型互通。
消费应用仅依赖解包后的 `rsille` 和第三方库。依赖未缓存时去掉 `--offline`；脚本不会上传。
[迁移与发布细节](MIGRATION_V3.md) · [MIT 许可](LICENCE)。
