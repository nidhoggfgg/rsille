# Examples / 示例

[English](README.md) · 中文

使用本地 `rsille` 构建的应用、游戏和图形示例。单个组件和运行时特性的用法见
[TUI 小示例指南](../packages/tui/examples/README_cn.md)。

## 运行示例

这些文档中的命令均在仓库根目录执行：

```bash
cargo run --example basic_canvas
cargo run --release --manifest-path examples/lumen/Cargo.toml
```

`basic_canvas.rs` 和 `raster.rs` 是根包的单文件示例，使用 `--example` 运行，
不是独立项目。下面每个子目录都是通过路径依赖引用 `rsille` 的独立 Cargo 项目，
不属于根 workspace。用表中的目录名配合 `--manifest-path` 运行；
动画和三维场景建议使用 release 构建。

| 示例 | 文件 / 目录 | 展示内容 |
| --- | --- | --- |
| [Raster 浮窗兼容性测试](raster.rs) | `raster.rs`（根包示例） | 在同一场景中检查 Kitty 和 Sixel 的窗口移动、遮挡、置前、缩放与重绘。 |
| [函数绘图](function-plotter/README_cn.md) | `function-plotter` | Mathematica 风格输入/输出单元、紧凑 Sixel 函数图像、重新执行与滚动历史。 |
| [TUI showcase](tui-showcase/README_cn.md) | `tui-showcase` | 七页组件展厅：表单、数据、内容、布局、动画和主题。 |
| [Lumen](lumen/README_cn.md) | `lumen` | 包含流程、笔记和浮动桌面的交互工作台。 |
| [Orbit Files](orbit-files/README_cn.md) | `orbit-files` | 支持框选、拖放、预览和撤销的文件管理器。 |
| [rsille showcase](rsille-showcase/README_cn.md) | `rsille-showcase` | 仓库浏览、模拟进程监视器、计算器和图形演示。 |
| [Neon Observatory](neon-observatory/README_cn.md) | `neon-observatory` | 以 inline 模式播放六种程序化盲文场景。 |
| [Folded Harbor](folded-harbor/README_cn.md) | `folded-harbor` | 通过视角对齐解谜，包含三章九次折叠。 |
| [Minecraft](minecraft/README_cn.md) | `minecraft` | 支持挖掘、搭建和飞行的第一人称方块沙盒。 |
| [Clockwork](clockwork/README_cn.md) | `clockwork` | 132 个零件自动组装成三种机械生物。 |
| [Pokémon Battle](pokemon-battle/README_cn.md) | `pokemon-battle` | 皮卡丘对战水箭龟的 3D 回合制游戏。 |
| [Jellyfish](jellyfish/README_cn.md) | `jellyfish` | 两只发光水母自动游动的 inline 水族箱。 |
| [AI Babies](ai-babies/README_cn.md) | `ai-babies` | 三个可切换的像素角色，每个角色有六组动作。 |

交互演示需要 TTY。建议使用支持 True Color 的终端和 Unicode 等宽字体；鼠标交互需要
终端支持鼠标事件。窗口尺寸、按键、数据持久化和导出选项见各示例文档。
Orbit Files 的完整文件操作需要 Linux；不传 `--path` 时使用一次性演示目录。

## 函数绘图

```sh
cargo run --release --manifest-path examples/function-plotter/Cargo.toml
cargo run --manifest-path examples/function-plotter/Cargo.toml -- --help
```

不带参数启动交互式 Sixel REPL，`--help` 显示帮助。
终端及复用器必须支持 Sixel、ED 2 和局部 EL 2 图形擦除、图形随文字滚动、
稳定调色板及无滚屏的定位输出。这些能力不会自动检测。
操作方式、实际字符格像素覆盖值和 headless 检查见[函数绘图指南](function-plotter/README_cn.md)。

## Raster 浮窗兼容性测试

```sh
cargo run --release --features kitty --example raster
cargo run --release --features kitty --example raster -- --backend kitty
cargo run --release --example raster -- --backend sixel
```

两个浮窗用原生 RasterCanvas 和字符 Canvas 绘制同一颗行星。默认 `auto` 从已编译的后端中优先选 Kitty，再选 Sixel。顶部显示实际协议和每格像素尺寸。可在同一终端分别指定两个后端，对照以下操作。

1. 把字符窗口拖到行星中央，检查前景的文字、空格和边框是否完全遮住后方图像。
2. 把窗口移开，再点行星窗口的标题置前。检查露出的图像是否完整，桌面是否还有残影。
3. 拖动底部手柄调整窗口大小，再把窗口移到屏幕底边。检查行星是否按比例缩放，底行有无错位或意外滚屏。
4. 按 `e` 切换字符编码，`r` 恢复窗口位置和尺寸，`q` 或 Esc 退出。也可用 Tab 聚焦标题、方向键移动、Shift+方向键缩放。

建议使用至少 100×32 字符格。在 Unix（含 macOS、WSL）上显式指定协议也会探测支持情况；`--cell-pixels WxH` 可提供实测的单格物理像素尺寸。原生 Windows 需同时指定已验证的后端和实测尺寸。全屏 Sixel 要求 ED 2 能清除图形、调色板稳定、补齐透明，且定位输出不会滚屏。

```sh
cargo run --example raster -- --headless
cargo test --features kitty --example raster
python3 scripts/check-sixel-query.py --both-backends
```

`--headless` 在内存中验证鼠标操作与遮挡恢复。PTY 脚本在 Unix 上检查协议协商、输出和键盘输入。实际显示效果可按上述步骤在目标终端观察。

## 无终端验证

除 `rsille-showcase` 外，所有独立项目均提供 `--headless`：

```bash
cargo run --release --manifest-path examples/lumen/Cargo.toml -- --headless
cargo test --manifest-path examples/lumen/Cargo.toml
cargo check --manifest-path examples/rsille-showcase/Cargo.toml --all-targets
```

根目录的 `cargo test --workspace` 不会测试这些独立项目。快照和性能测量的参数因项目
而异，以对应 README 为准。快照由渲染后的终端 Buffer 还原；CPU 性能测量不包含
终端 I/O，也不代表最终显示帧率。

## 添加示例

单项功能放在 `packages/tui/examples/`，并在 `packages/tui/Cargo.toml` 注册。
完整应用放在这里，设置 `publish = false`，通过路径依赖引用 `rsille`。
同时提供中英文说明、启动命令、操作方式和可重复执行的验证命令。

[项目首页](../README_cn.md)
