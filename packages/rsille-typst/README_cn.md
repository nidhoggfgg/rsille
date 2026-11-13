# rsille-typst

[English](README.md) | [简体中文](README_cn.md)

[API 文档](https://docs.rs/rsille-typst) · [crates.io](https://crates.io/crates/rsille-typst) · [代码仓库](https://github.com/nidhoggfgg/rsille)

将 **Typst 数学公式渲染为不透明 RGB `RasterCanvas`**，直接接入 rsille 的 raster widget。使用真正的 Typst 编译器和 `typst-render`，在进程内完成，无需命令行子进程，也不做 LaTeX 转换。

输入**数学模式内部语法**，例如 `x^2`、`frac(1, 2)`、`sqrt(x)` 或 `sum_(k=1)^n k`。不要添加外围 `$ ... $`；不支持 `\frac{1}{2}` 等 LaTeX 语法。

## 快速开始

在应用的 `Cargo.toml` 中添加：

```toml
[dependencies]
rsille-typst = "0.1.0"
# 下方示例使用的可选字体来源；也可以提供自己的字体。
typst-assets = { version = "=0.13.1", features = ["fonts"] }
```

```rust
use rsille_typst::{MathRenderer, RenderOptions, Rgb};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let renderer = MathRenderer::new(
        typst_assets::fonts(),
        "New Computer Modern Math",
    )?;
    let options = RenderOptions {
        font_size: 20.0,
        foreground: Rgb(230, 230, 230),
        background: Rgb(24, 24, 24),
        max_pixels: 4 * 1024 * 1024,
        ..RenderOptions::default()
    };
    let result = renderer.render_with_diagnostics(
        r"integral_0^1 x^2 dif x = frac(1, 3)",
        &options,
    )?;
    for warning in result.warnings {
        eprintln!("Typst warning: {}", warning.message);
    }
    let canvas = result.canvas;
    println!("{} × {} pixels", canvas.width(), canvas.height());
    Ok(())
}
```

这段代码生成像素，并不向终端输出图像；无需 TTY。复用 renderer 中已加载的字体，并保留生成的 canvas，避免在每个 UI 帧中重新编译。

### Feature 与版本

0.1.0 版本依赖 `rsille` **3.1.0**，并启用其默认 features。本扩展不定义任何 Cargo feature；Sixel 编码始终可用。核心不依赖 Typst。

`typst` 和 `typst-render` 固定为 **0.13.1**；若使用字体资产包，请选择匹配的 `typst-assets = "=0.13.1"`。应用应保留自己的 lockfile，以固定传递依赖。

## 接入 rsille

需要终端输出时，直接依赖核心：

```toml
[dependencies]
rsille-typst = "0.1.0"
rsille = "3.1.0"
```

以下函数接收已渲染的 canvas，以及实际后端使用的、已验证的字符格像素尺寸：

```rust
use rsille::tui::prelude::*;

fn formula_widget(
    canvas: RasterCanvas,
    cell_pixels: CellPixels,
) -> Result<impl Widget<()>, Box<dyn std::error::Error>> {
    let size = cell_pixels
        .size_for_pixels(canvas.width(), canvas.height())
        .ok_or("invalid cell metrics or terminal size overflow")?;
    Ok(raster_canvas::<()>(canvas)
        .scale(RasterScale::Original)
        .fixed(size.width, size.height))
}
```

将 widget 加入现有 `App`，并显式配置 `RasterOutput` 能力；完整流程见[示例](examples/formula.rs)。底层绘制也可以使用 `Chunk::raster` 和 `RasterOptions`。

像素尺寸不等于终端列数和行数。`Original` 保留图像原始像素尺寸并允许裁剪；需要缩放时选择 `Fit`。生成 RGB 像素和检查 headless raster 元数据不执行 Sixel 编码。真实终端的 raster 输出**仅支持 Sixel**：缺少能力或指标无效时会报错，不会降级为文本或字符预览。

## 运行演示

在仓库根目录执行：

```sh
cargo run -p rsille-typst --example formula -- --help
cargo run -p rsille-typst --example formula
cargo run -p rsille-typst --example formula -- 'sum_(k=1)^n k = frac(n(n+1), 2)'
# 无需 TTY：仅检查布局和元数据，不做 Sixel 编码。
cargo run -p rsille-typst --example formula -- --headless
# 无需 TTY：额外执行 Sixel 内存编码检查。
cargo run -p rsille-typst --example formula -- --headless --cell-pixels 10x20
```

交互演示只编译一次，在普通屏幕上 inline 展示块级公式，不进入备用屏幕、不捕获鼠标；按 **q / Esc** 退出。公式应作为一个带引号的参数传入；以 `-` 开头的公式放在 `--` 之后。

示例使用 `RasterScale::Original`，预留公式所需行数和一行标题，受框架 inline 内容最多 50 行的限制，并额外预留底部保护行。终端过小时只裁剪，不缩放或重排；扩大终端后重新显示，无需重新编译。超大公式可能无法完整显示。

### 终端兼容性与字符格尺寸

演示要求**终端及任何复用器**均支持 Sixel、局部 EL 2 图形清除、图像随文本滚动、稳定调色板、不滚屏的定位输出，以及最后不足六像素带的透明补齐。配置使用 `reliable_inline_erase: true`，而非全屏 `reliable_erase`。**不会自动检测这些能力。** 具体约定见 [`RasterCapabilities`](https://docs.rs/rsille/3.1.0/rsille/render/raster_output/struct.RasterCapabilities.html)。

未提供 override 时，`CellPixels::resolve(None)` 在输入 reader 启动前探测：先读取 OS 指标，再尝试 Unix（含 WSL）CSI 16 t 查询，超时为 500 ms。查询要求 stdin/stdout 为控制终端上的 TTY；原生 Windows 没有查询 fallback。探测失败时，通过 `--cell-pixels WxH` 提供**实测的单个字符格物理像素尺寸**，不是列数×行数、窗口像素或字体点数。例如，960×648 像素的文本区域有 120 列、36 行，则每格为 8×18 像素。

- 显式 override 跳过全部探测。每轴必须在 `1..=1024` 内，面积不超过 65536。与终端列行数相同的 override 还需 `--force-cell-pixels` 确认。
- 指标仅在启动时保存；更改字体或 DPI 后需重启。查询不是 Sixel 能力检测，可能消耗同时输入的按键，迟到的响应也可能进入输入 reader。
- Inline Sixel 补齐可能需要空白保护行，从而减少可用高度：10×20 字符格额外需要一行，8×18 则不需要。

`--headless` 不启动终端 I/O 或输入 reader，也不探测指标。单独使用时，以合成的 10×20 布局指标检查 raster 元数据。添加 `--cell-pixels` 后改为真实的 Sixel 内存编码检查，该功能始终可用，无需 feature 标志；指标无效时直接失败，不退回元数据检查。合成指标仅适用于此类检查。

两种 headless 模式均检查同一 Runtime 的布局、resize 和退出键。Sixel 模式还检查 DCS 输出、tile 数量、不变帧复用，以及不使用备用屏幕、鼠标或全屏清除序列。**两种模式都不能验证真实终端/复用器上的视觉兼容性。** 仓库中的 `python3 scripts/check-sixel-query.py --formula` 可检查 PTY 启动探测、输入及终端状态恢复，同样不验证视觉效果。

## 字体与许可证

公共库**不捆绑隐式字体**，不搜索系统目录，也不在运行时下载资源。`MathRenderer::new` 接受字体字节迭代器（`AsRef<[u8]>`），复制并解析一次，注册 TTF/OTF/TTC 集合中的所有可读 face。选定 family 至少需要一个带 OpenType **MATH** 表的 face。

空字体集合、不可读输入、不存在或不支持数学的 family 会返回错误。可额外提供文字字体作为 fallback；缺少字形等情况可能产生 warning，因此建议使用 `render_with_diagnostics`。

快速开始和演示显式使用 `typst-assets` 的 `fonts` feature，其中包含 New Computer Modern Math。未启用该 feature 时，其字体迭代器为空。也可以在应用代码中读取自行部署的字体，或通过 `include_bytes!` 嵌入；这不会赋予 Typst 文件系统访问权限。

**rsille-typst 使用 [MIT 许可证](LICENCE)，字体则有各自的许可证。** 请保留所选字体要求的许可声明和署名，并遵守其分发条款；本 crate 的 MIT 许可证不会替代字体许可证。字体资产来源见 [typst-assets 0.13.1](https://github.com/typst/typst-assets/tree/v0.13.1)。

## API 与渲染行为

| API | 用途 |
| --- | --- |
| `MathRenderer::new(font_bytes, math_family)` | 加载可复用的字体、font book 和模板源码。 |
| `render(formula, &options)` | 返回 `RasterCanvas`，忽略非致命 warning。 |
| `render_with_diagnostics(formula, &options)` | 返回 `RenderedMath { canvas, warnings }`。 |
| `RenderOptions` | 字号、像素比例、RGB 颜色和像素预算；实现 `Default`。 |
| `Error` | 字体/配置错误、`Compile(Vec<SourceDiagnostic>)`、页数错误、raster 限制或 `Raster(RasterError)`；实现 `Display` 和 `std::error::Error`。 |

重新导出 `RasterCanvas`、`RasterError`、`Rgb` 和 `SourceDiagnostic`。这是同步的单公式渲染器，不是完整文档 API、专用 widget、异步任务系统或公式结果缓存。

模板使用自动页面尺寸、零页边距和块级 equation。保留 Typst 自身的数学布局与行度量，不做逐像素 ink 裁剪。空公式按 Typst 的布局结果处理，每个输出轴至少为一个像素。

默认配置为 18 pt、2 pixels/pt（144 dpi）、黑色前景、白色背景，以及 `16 × 1024 × 1024` 像素预算。1 pt 为 1/72 英寸。颜色为不透明的 8-bit sRGB；公式内部样式可以覆盖前景色。透明页面的预乘 RGBA 按 `premultiplied_rgb + background * (1 - alpha)` 舍入合成，不重复乘 alpha，也不先反预乘。输出不含 alpha；背景同时作为 canvas 元数据。

### 诊断

编译错误保留 Typst 的原始消息、hint、span 和 trace。`Display` 展示消息和 hints；完整诊断可从 `Error::Compile` 获取。由于输入经 `eval(..., mode: "math")` 传入，span 指向模板调用，**不提供原始公式的精确行列映射**。

`warnings` 保留 Typst 实际返回的 warning，但 Typst 0.13.1 的立即 `eval` 使用独立 sink，会丢弃其中的 warning（例如直接调用 `text` 时的未知字体提示）。布局/延迟 context 阶段的 warning 可以保留。本 API 无法恢复上游丢弃的 warning。

## 资源与安全限制

- 字号必须有限且在 `0.1..=4096` 内；pixels/pt 必须有限且在 `0.01..=16` 内。`max_pixels` 必须在 `1..=16 * 1024 * 1024` 内。非法配置在编译前返回错误。
- 编译结果必须恰好一页。光栅化前，按照固定版本 renderer 的 f32 乘法和 `round().max(1)` 规则检查两轴：每轴最多 16384 像素，总像素数同时受 `max_pixels` 和核心上限约束。可降低预算，不可提高核心上限。
- RGB 分配使用 `try_reserve_exact`，随后由 `RasterCanvas::from_pixels` 验证。像素预算**不是进程内存上限**：RGBA 与 RGB 缓冲区同时存在，另有字体、编译和布局数据。上游分配失败或内部 panic 不保证能转换为 `Error`。

Typst World 仅暴露自身内存主源码和调用方提供的字体。其它所有 source 请求（包括包）及全部 file 请求均返回 `AccessDenied`。没有路径解析、文件读取、包下载、系统字体搜索或当前日期。输入作为 `sys.inputs` 字符串传入，而非拼接进模板，因此无法通过闭合 `$` 分隔符逃逸模板。但合法的 Typst `#` 表达式仍可执行。

**禁用文件访问不等于强沙箱。** 循环、递归、大型表达式、内嵌数据/图像及复杂布局仍可能在尺寸检查前或渲染期间耗尽 CPU 或内存。没有超时、取消、执行步数预算或进程隔离。处理不可信公式时，请使用独立 worker 进程，由宿主实施时间/内存限制并在必要时终止进程。不要在 UI 帧循环中反复编译。

本 crate 不缓存公式结果，但 Typst/comemo 和 interner 使用进程级缓存/存储。丢弃 renderer 不会清空全部上游缓存；长期处理无限多种不同公式时，不承诺固定的内存上界。有严格限制的服务应轮换 worker 进程。

## 开发检查

```sh
cargo test -p rsille-typst
cargo test -p rsille-typst --all-targets
cargo build -p rsille-typst --examples
cargo fmt -p rsille-typst -- --check
```

测试覆盖真实渲染、颜色/alpha、字体、诊断、资源访问拒绝、尺寸限制和 rsille widget 集成。Typst 的编译依赖较重；内存紧张时，可尝试单任务构建并关闭调试信息（POSIX shell）：

```sh
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p rsille-typst -j 1
```

### 发布（维护者）

本 crate 独立于核心发布。在干净的工作区中，从仓库根目录运行
`python3 scripts/check-release-typst.py`（需要 Python 3.12+）。脚本会打包，
在 workspace 外使用 registry 依赖而非本地 rsille，运行归档中的测试和两种
headless 示例。仅当所需 registry 依赖均已缓存时使用 `--offline`。

发布前更新本 crate 的版本号和文档，新增 `releases/rsille-typst/<版本号>.md`，
并将修改合入 `main`。确认所需核心版本已经发布后，创建并推送
`rsille-typst-v<版本号>`（例如 `rsille-typst-v0.1.0`）。
`Release rsille-typst` workflow 会校验 tag、执行检查，使用仓库的
`CARGO_REGISTRY_TOKEN` secret 发布，并将 crate 和校验文件附加到 GitHub Release，
不会替换核心的 Latest release。推送发布 tag 会真正发布到 crates.io；本地检查
脚本不会上传任何内容。
