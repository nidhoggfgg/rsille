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

`basic_canvas.rs` 和 `sixel_windows.rs` 是根包的单文件示例，使用 `--example` 运行，
不是独立项目。下面每个子目录都是通过路径依赖引用 `rsille` 的独立 Cargo 项目，
不属于根 workspace。用表中的目录名配合 `--manifest-path` 运行；
动画和三维场景建议使用 release 构建。

| 示例 | 文件 / 目录 | 展示内容 |
| --- | --- | --- |
| [Sixel 与 Canvas 浮窗](../docs/raster.md#run-the-showcase) | `sixel_windows.rs`（根包示例） | 两个可拖动、调整大小的浮窗，以原生 RGB 栅格和 Canvas 字符采样展示同一颗静态带环行星。 |
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

## Sixel 与 Canvas 浮窗

```sh
cargo run --release --features sixel --example sixel_windows -- --sixel
cargo run --release --example sixel_windows -- --text-only
cargo run --example sixel_windows -- --help
```

在同一个 `window_host` 中用两个 `floating_window` 展示同一颗程序化带环行星，
没有实时动画。拖动标题移动窗口，拖动底部手柄调整大小，点击置前。
`e` 循环切换 Canvas 的 Braille/HalfBlock/Quadrant；`r` 恢复初始窗口位置和大小；
`q`/Esc 退出。建议使用 100×32 字符格，也支持更小终端及终端缩放。

`--sixel` 表示用户明确确认整个终端/复用器链路支持 Sixel、ED 2 图形擦除、
无滚屏及稳定调色板。示例使用 `RequireSixel`，不会静默降级。每格像素尺寸优先来自
操作系统窗口度量；不可用时在 Unix（包括 WSL）回退到 `CSI 16 t` 查询，要求
stdin/stdout 均为连接到控制终端的 TTY，绝对超时为 500 ms。原生 Windows 尚无查询回退，
仍需显式尺寸。可选的 `--cell-pixels 8x18` 跳过上述两种来源，必须匹配**实际字体度量**，
不能任意填写。查询成功与失败均按终端字符网格缓存，列数/行数变化时刷新；字体改变但
网格未变时，请使用显式覆盖或重启。此查询不协商 Sixel 能力，`--text-only` 不发送每格
像素查询。进入托管终端会话前会检查 feature 和后端限制
（每个维度为 1..=1024，乘积不超过 65,536）。
支持实际为 10×20 像素的字符格，可使用 `--cell-pixels 10x20`，不要将高度取整为 18 或 24。
终端仍须正确处理末尾不完整六行带的透明填充，并满足图形擦除及无滚屏断言；
接受这些尺寸不代表已通过真实终端认证。
不带参数显示帮助，而不是默认字符降级演示；`--text-only` 显式选择字符预览。
完整安全约定见[栅格指南](../docs/raster.md)。

有限的、无需终端的鼠标拖动与遮挡检查，以及示例测试：

```sh
cargo run --example sixel_windows -- --headless
cargo test --example sixel_windows --features sixel
cargo test --example sixel_windows --no-default-features
```

这些检查不能认证真实终端的图形行为，也不是性能测量。

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
