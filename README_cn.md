# rsille

[English](README.md) · 中文

面向 Rust 的终端应用与图形框架。既可以用组件、布局和后台任务构建交互工具，也可以通过
渲染器和采样画布直接绘制终端字符。

当前仓库使用 **v3 API**（`Cargo.toml` 中版本为 `3.0.0`）。从 v2 升级请阅读
[迁移指南](MIGRATION_V3.md)。

![使用 rsille 构建的 Lumen 工作台](examples/lumen/preview.png)

## 可以做什么

- **终端应用**：表单、Unicode 文本编辑、表格、树、文件浏览器、命令面板和内容查看器。
- **交互布局**：Flex、Grid、滚动、可调分栏、模态 Portal、可拖动浮窗、指针捕获和键盘焦点。
- **动画与图形**：过渡、视觉特效、毛玻璃背景，以及支持盲文、半块和四分块编码的彩色 Canvas。
- **可测试的流程**：消息、后台任务、防抖、带作用域的订阅，以及无需终端即可驱动的运行时。

[示例目录](examples/README_cn.md) 包含工具、游戏和动画。完整应用可以从
[Lumen](examples/lumen/README_cn.md) 开始，组件展示看
[TUI showcase](examples/tui-showcase/README_cn.md)，单项 API 用法看
[TUI 小示例](packages/tui/examples/README_cn.md)。

## 先运行看看

使用当前稳定版 Rust 工具链，在仓库根目录执行：

```bash
cargo run -p tui --example hello
cargo run --release --manifest-path examples/lumen/Cargo.toml
cargo run --release --manifest-path examples/pokemon-battle/Cargo.toml
```

交互示例需要终端。图形示例建议使用 Unicode 等宽字体与 True Color；各示例文档列出了
推荐窗口尺寸和操作方式。首次运行会编译依赖，图形示例建议加 `--release`。

## 在应用中引入 rsille

如果要在另一个项目中使用当前源码，添加路径依赖，并替换为本地仓库的位置：

```toml
[dependencies]
rsille = { path = "../rsille" }
```

对外的包名是 `rsille`。仓库中的 `tui`、`render`、`canvas` 都是内部开发用 crate，
均设置了 `publish = false`。

默认没有启用任何 feature。代码查看器需要 Syntect 语法高亮时，启用 `syntax-highlighting`：

```toml
[dependencies]
rsille = { path = "../rsille", features = ["syntax-highlighting"] }
```

未启用时，代码查看器仍支持纯文本显示和导航。

## 第一个应用

```rust,no_run
use rsille::tui::prelude::*;

fn main() -> WidgetResult<()> {
    let exit = App::new(0_u32)
        .mode(Mode::Inline)
        .update(|count, (), _ctx| *count += 1)
        .view(|count| button(format!("计数：{count}")).on_click(|| ()))
        .run()?;

    println!("最终计数：{}", exit.state);
    Ok(())
}
```

Tab 切换焦点，Enter 或空格激活按钮，默认按 Esc 退出。`view` 根据状态描述组件，
组件产生的消息交给 `update`。更新函数的第三个参数 `UpdateCtx` 可调度任务、发送消息，
也可以调用 `ctx.exit()` 退出。

主题、快捷键、订阅和时间相关配置放在 `.update()` 之前。默认全屏；`Mode::Inline`
使用普通终端屏幕。`run()` 在清理后返回 `AppExit`，其中包含最终状态和退出原因。
在已有的、启用了 I/O 与时间驱动的 Tokio runtime 中使用 `.run_async().await`。

## 无需终端的测试

同一个 builder 也能创建内存中的运行时：

```rust
use rsille::tui::prelude::*;

let mut runtime = App::new(0_u32)
    .update(|count, (), _ctx| *count += 1)
    .view(|count| button(format!("计数：{count}")).on_click(|| ()))
    .build()
    .size(40, 3);

runtime.handle().send(()).unwrap();
runtime.step(&[]).unwrap();
assert_eq!(*runtime.state(), 1);
assert_eq!(runtime.frame().size().width, 40);
```

通过 `step(&events)` 回放输入，使用 `frame()` 检查渲染后的字符格。
从 `rsille::tui::runtime` 导入 `ManualClock` 和 `ManualExecutor`，即可手动推进计时器
和队列中的任务。[workflow 示例](packages/tui/examples/basics/workflow.rs) 用同一个应用
验证 Unicode 编辑、防抖、后台保存和退出：

```bash
cargo run -p tui --example workflow -- --headless
```

## 使用 Canvas 绘图

不需要组件树时，可以直接使用渲染器：

```rust,no_run
use rsille::{canvas::Canvas, render};

fn main() -> std::io::Result<()> {
    let mut canvas = Canvas::new();
    for x in 0..160 {
        let x = x as f64;
        canvas.set(x, 15.0 + (x / 10.0).sin() * 10.0);
    }
    render::Builder::new()
        .size((80, 10))
        .append_newline(true)
        .build_render(canvas, std::io::stdout())
        .render()
}
```

| 编码 | 每个字符格的采样数 | 用途 |
| --- | --- | --- |
| `Encoding::Braille`（默认） | 2 × 4 | 密集线条、曲线和彩色场景 |
| `Encoding::HalfBlock` | 1 × 2 | 上下两个可独立着色的采样点 |
| `Encoding::Quadrant` | 2 × 2 | 四分块形状，采样颜色拟合为终端颜色 |

通过 `Canvas::with_encoding(...)` 选择编码。绘图坐标以采样点为单位，y 轴向上；
边界与输出尺寸以终端字符格为单位。`set_color` 设置采样点颜色，`set_background`
填充未覆盖区域。每个字符格最多使用一个前景色和一个背景色，因此盲文和四分块编码可能合并采样颜色。

TUI 的 `canvas(...)` 组件把同一套绘图能力嵌入布局。通过 `.viewport(...)` 和
`ctx.world(surface)` 使用世界坐标，并让指针转换与画面一致。
运行 `cargo run -p tui --example canvas` 可对比三种编码。

## API 与源码入口

| 公开模块 | 职责 | 共用实现 |
| --- | --- | --- |
| `rsille::tui` | 应用、组件、布局、输入和特效 | [src/tui](src/tui) |
| `rsille::render` | Buffer、带样式的 Unicode 文本、绘制与终端循环 | [src/render](src/render) |
| `rsille::canvas` | 采样图形与终端编码 | [src/canvas](src/canvas) |

`packages/` 中的内部 crate 复用这些源码，提供独立测试和示例入口，不维护重复实现。
可在本地生成 API 文档：

```bash
cargo doc -p rsille --all-features --no-deps --open
```

文本编辑以字素簇为单位，实际字宽与显示效果仍受终端和字体影响。需要按住键操作时，
可启用 `KeyboardMode::Auto` 并检查 `InputEvent::Capabilities`；准确的松键事件和独立
多键按住需要终端支持，详见 [Minecraft 输入说明](examples/minecraft/README_cn.md)。

## 开发与验证

```bash
cargo test --workspace
cargo test --workspace --all-features
cargo check --workspace --all-targets --all-features
```

`examples/` 下的独立项目不属于根 workspace，需要用 `--manifest-path` 单独检查，
具体命令见各示例 README。

`python3 scripts/check-release.py --offline` 验证自包含的打包产物和独立消费应用，
不会上传。需要 Python 3.12+ 和已缓存的依赖；允许下载依赖时去掉 `--offline`。

[示例目录](examples/README_cn.md) · [TUI 小示例](packages/tui/examples/README_cn.md) ·
[迁移指南](MIGRATION_V3.md) · [MIT 许可](LICENCE)
