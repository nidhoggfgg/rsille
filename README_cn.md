# rsille

[English](README.md) · 中文

<img align="right" src="examples/ai-babies/preview-deepseek.gif" width="96" alt="">

**让终端，惊艳起来。**

画出发光的深海，让机械生物拆解重组，把可拖动的窗口和三维场景放进同一个终端。
**rsille 是 Rust 终端应用与图形框架**，把渲染、TUI、动画和交互放进同一套可组合的 API。

## 一个终端，可以有多少种模样？

点击预览，查看源码、运行命令和操作方式。

| Jellyfish · 把深海装进终端 | Lumen · 像桌面一样操作 |
| --- | --- |
| [![终端中发光的水母、光束与漂浮微粒](examples/jellyfish/preview.gif)](examples/jellyfish/README_cn.md) | [![Lumen 的三维画布、实时曲线和可拖动浮窗](examples/lumen/preview-desk.png)](examples/lumen/README_cn.md) |
| **渲染。** 全彩采样、Half-block 编码和连续重绘，呈现程序生成的水母、柔和光束与漂浮微粒。 | **TUI。** 画布与原生组件共处一个布局；窗口可拖动、缩放、收起和恢复，配合焦点管理与命令面板。 |

| Clockwork · 让零件自己组装 | Pokémon Battle · 亲手打一场 |
| --- | --- |
| [![132 个零件拆解、组装并变形成机械生物](examples/clockwork/preview.gif)](examples/clockwork/README_cn.md) | [![三维战场中的皮卡丘、水箭龟与招式按钮](examples/pokemon-battle/preview.png)](examples/pokemon-battle/README_cn.md) |
| **动画。** 示例用弹簧驱动 132 个零件，在三种形态间拆解重组；rsille 负责逐帧绘制和输入，还能边播放边转动相机。 | **图形 × TUI × 交互。** 三维场景叠加原生招式按钮、血条与毛玻璃背景；点击出招，拖动镜头，看攻击演出。 |

预览由渲染后的终端字符格导出；Clockwork 的 GIF 做了加速。
场景和模拟逻辑由示例提供，rsille 负责画布、编码、组件、渲染与事件循环。

## 效果惊艳，写起来也顺手

<img align="right" src="examples/ai-babies/preview-pet.gif" width="96" alt="">

- **给组件加上特效。** 用 `visual(...)` 包住按钮、面板或画布，链式叠加渐变、扫光、
  溶解与波浪，还能加上毛玻璃背景和入场、退场过渡。动画由运行时驱动。
- **把一个世界放进界面。** 用盲文、半块或四分块绘制彩色图形。把 `canvas(...)` 放入布局，
  使用世界坐标和配套的指针转换；也可以直接用渲染器绘制整个场景。
- **应用代码保持简单。** `view` 描述界面，`update` 修改状态。组合 Flex/Grid 布局、
  文本编辑器、表格和可拖动浮窗，焦点、输入、后台任务与退出清理由运行时处理。

继续逛 **[示例画廊](examples/README_cn.md)**：
[方块沙盒](examples/minecraft/README_cn.md)、
[拖放文件管理器](examples/orbit-files/README_cn.md)、
[七页组件展厅](examples/tui-showcase/README_cn.md)。
想一次学一个 API，可以从 [TUI 小示例](packages/tui/examples/README_cn.md) 开始。

## 在应用中引入 rsille

在应用中添加 rsille `3.0.2` 依赖：

```toml
[dependencies]
rsille = "3.0.2"
```

对外的包名是 `rsille`。仓库中的 `tui`、`render`、`canvas` 都是内部开发用 crate，
均设置了 `publish = false`。

默认没有启用任何 feature。代码查看器需要 Syntect 语法高亮时，启用 `syntax-highlighting`：

```toml
[dependencies]
rsille = { version = "3.0.2", features = ["syntax-highlighting"] }
```

未启用时，代码查看器仍支持纯文本显示和导航。

本地开发时，可改用 `rsille = { path = "../rsille" }`，将路径替换为本地仓库的位置。

## 配合 AI 编程助手开发

独立仓库提供 [rsille skill](https://github.com/nidhoggfgg/rsille.skills/blob/main/rsille/SKILL.md)，帮助编程助手使用 v3 API 开发终端应用，
涵盖组件与布局、后台任务、输入处理、Canvas 图形、动画和无终端测试。
[组件索引](https://github.com/nidhoggfgg/rsille.skills/blob/main/rsille/references/components.md) 按用途列出内置组件、关键用法和对应示例，
并附带可独立运行的应用模板。

在你的应用目录中，通过 [Skills CLI](https://github.com/vercel-labs/skills) 安装：

```bash
npx skills add nidhoggfgg/rsille.skills --skill rsille
```

按提示选择使用的编程助手；加上 `-g` 可跨项目使用。从本地仓库安装时，将
`nidhoggfgg/rsille.skills` 替换为 skills 仓库的绝对路径。本仓库通过 `skills/` Git 子模块
引用它，运行 `git submodule update --init skills` 即可拉取。也可以把整个 `skills/rsille`
目录复制到所用助手的 skills 目录。

安装后，可以这样提出需求：

> 使用 rsille skill 开发一个终端任务管理器，支持输入筛选、键盘导航，并提供无终端测试。

## 第一个应用，就带动画

```rust,no_run
use std::time::Duration;
use rsille::tui::prelude::*;

fn main() -> WidgetResult<()> {
    let exit = App::new(0_u32)
        .mode(Mode::Inline)
        .update(|count, (), _ctx| *count += 1)
        .view(|count| {
            visual(
                button(format!("计数：{count}"))
                    .variant(ButtonVariant::Ghost)
                    .on_click(|| ()),
            )
                .gradient(
                    Color::Rgb(70, 220, 255),
                    Color::Rgb(240, 110, 255),
                    GradientDirection::Horizontal,
                )
                .highlight_sweep()
                .animation(looping_visual_spec(Duration::from_secs(2)))
        })
        .run()?;

    println!("最终计数：{}", exit.state);
    Ok(())
}
```

<img src="imgs/readme-counter-cn.gif" width="424" alt="渐变计数按钮的循环扫光与按 Enter 后的计数变化">

预览由上述代码的 32 × 3 字符格运行时导出，回放 Enter 展示从 0 到 2 的计数变化。

<img align="right" src="examples/ai-babies/preview-claude.gif" width="96" alt="">

这是一个可点击的计数按钮，自带青色到品红的渐变和循环扫光。
三个链式调用就加上了颜色与动画，无需手写逐帧循环。
`visual(...)` 包裹交互组件，按钮仍保留原本的输入行为。

Tab 切换焦点，Enter 或空格激活按钮，默认按 Esc 退出。`view` 根据状态描述组件，
组件产生的消息交给 `update`。更新函数的第三个参数 `UpdateCtx` 可调度任务、发送消息，
也可以调用 `ctx.exit()` 退出。

主题、快捷键、订阅和时间相关配置放在 `.update()` 之前。默认全屏；`Mode::Inline`
使用普通终端屏幕。`run()` 在清理后返回 `AppExit`，其中包含最终状态和退出原因。
在已有的、启用了时间驱动的 Tokio runtime 中使用 `.run_async().await`。

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

**一个字符格，也能画出更多细节。** `Encoding::Quadrant` 将每个终端字符格细分为
**2 × 2 个采样位置**，通过四分块字符与前景、背景颜色组合，在有限的行列里表现更细的轮廓。
120 × 20 个字符格，对应 240 × 40 的采样网格。

[![120 × 20 字符格中使用 2 × 2 四分块编码渲染的像素角色](imgs/readme-canvas-quadrant.png)](examples/ai-babies/README_cn.md)

上图是 [AI Babies 静态模式](examples/ai-babies/README_cn.md) 的实际 Canvas 输出，
仅占 **20 行**；预览按字符格比例放大，便于查看细节。

```bash
cargo run --release --manifest-path examples/ai-babies/Cargo.toml -- --static --height 20
```

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

通过 `Canvas::with_encoding(Encoding::Quadrant)` 选择编码。绘图坐标以采样点为单位，y 轴向上；
边界与输出尺寸以终端字符格为单位。`set_color` 设置采样点颜色，`set_background`
填充未覆盖区域。每个字符格最多使用一个前景色和一个背景色，因此盲文和四分块编码可能合并采样颜色。

TUI 的 `canvas(...)` 组件把同一套绘图能力嵌入布局。通过 `.viewport(...)` 和
`ctx.world(surface)` 使用世界坐标，并让指针转换与画面一致。
运行 `cargo run -p tui --example canvas` 可对比三种编码。

## API 与源码入口

当前仓库使用 **v3 API**（`Cargo.toml` 中版本为 `3.0.2`）。

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
[MIT 许可](LICENCE)
