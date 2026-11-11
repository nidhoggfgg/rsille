# rsille-typst

独立的 **Typst 数学公式 → `rsille::raster::RasterCanvas`** 扩展。
直接输入 Typst 数学模式内部语法，不接受 LaTeX，也不做 LaTeX 转换。
采用真正的 Typst 编译器和 `typst-render`，不是占位图或命令行子进程。

本 crate 在 rsille workspace 中维护，有独立 manifest 和公开 API；依赖方向为
`rsille-typst → rsille`，核心没有 Typst 依赖。不实现完整文档、专用 Widget、
异步任务或公式结果缓存。

## 直接在终端展示公式

在仓库根目录运行：

```sh
cargo run -p rsille-typst --example formula -- --help
cargo run -p rsille-typst --features sixel --example formula -- --sixel
cargo run -p rsille-typst --features sixel --example formula -- --sixel 'sum_(k=1)^n k = frac(n(n+1), 2)'
# 仅字符预览，不保证公式可读性：
cargo run -p rsille-typst --example formula -- --text-only 'sqrt(x)'
# 无需 TTY 的真实渲染及 Widget/Runtime 自检：
cargo run -p rsille-typst --example formula -- --headless
cargo run -p rsille-typst --features sixel --example formula -- --sixel --cell-pixels 10x20 --headless
cargo test -p rsille-typst
cargo test -p rsille-typst --features sixel --all-targets
cargo build -p rsille-typst --features sixel --examples
cargo fmt -p rsille-typst -- --check
```

示例将 `MathRenderer → RasterCanvas` 接到现有 `App` / `raster_canvas`，默认使用
`Mode::Inline` 在普通屏幕展示单个块级公式，不需要 `--inline`，不进入备用屏幕、
不捕获鼠标，`q` / `Esc` 退出。公式只在启动时编译一次。
使用 `RasterScale::Original` 而非会放大的 `Fit`：按公式像素尺寸除以 cell pixels
向上取整确定 widget 宽高，内容只占公式所需行数加一行标题（框架最多 50 行），
原生模式另由现有 renderer 预留底部 guard。窄/矮终端或 resize 变小时裁剪，
变大后重新显示，不重排、不缩放、不重新编译；超长公式可能无法完整显示。
字符预览按 HalfBlock 的 1×2 采样换算，可能占更多行，同样裁剪。无参数显示用法；公式参数为一个带引号的 Typst 数学字符串，省略时
使用求和公式。以 `-` 开头的公式放在 `--` 之后。

`--sixel` 是用户对终端**及复用器**能力的显式断言：支持 Sixel、局部 EL 2 图形清除、
图像随文本滚动、稳定调色板、不滚屏的定位输出，以及最后不足六像素带的透明补齐。
配置使用 `reliable_inline_erase: true`，不使用全屏 `reliable_erase`。不会自动检测能力。
使用 `RequireSixel`，缺少 feature 或能力/指标错误时失败，绝不静默降级。
新包 `sixel` feature 只转发 `rsille/sixel`，核心默认依赖与 feature 不变。
`--text-only` 显式选择 HalfBlock 预览，不探测像素指标，**不保证公式可读性**。

原生模式默认在启动输入 reader **之前**通过 `CellPixels::resolve(None)` 调用
`CellPixels::probe`：先读 OS 指标，再尝试 Unix（含 WSL）CSI 16 t 查询，超时 500ms。
查询需要控制终端上的 stdin/stdout TTY；原生 Windows 没有查询 fallback。
探测失败时必须手动提供 `--cell-pixels WxH`，它指**单个字符格的物理像素**，不是
终端列数×行数、窗口像素或字体点数。例如文本区域 960×648 像素、120 列×36 行，
对应 8×18 像素/格。请测量而非照抄示例值。显式 override 跳过所有 OS/终端查询；
范围 1..=1024，面积不超过 65536。与终端列行数相同的 override 需额外
`--force-cell-pixels` 确认。指标仅启动时保存，改变字体或 DPI 后需重启。
查询不是 Sixel 能力检测，可能消耗同时输入的按键，超时后的迟到响应可能进入 reader。

`--headless` 不启动终端或输入 reader，默认检查 text-only；原生配置自检需显式
`--sixel --cell-pixels WxH`（此时允许合成测试指标）。检查同一 Runtime 的 raster
布局、resize、字符 fallback 和两种退出键，并将帧交给真实终端编码器写入内存，
检查完整 DCS 输出、tile 数量、不变帧复用和不使用备用屏幕/鼠标/全屏清除；
不证明真实终端兼容性。

Inline 沿用现有底部 guard/no-scroll 规则，不依赖全屏底行的锚点上移修复。
cell 高度为 h 时，guard 行数为 `ceil(((6 - h % 6) % 6) / h)`：10×20 时
为 1 行、8×18 时为 0 行、h=1 时为 5 行。renderer 从终端可用高度扣除 guard，
先预留行再绘制，使用保存的底部锚点相对定位；透明补齐本身不足以防止滚屏。
成功帧光标停在最后一行内容，退出换行可进入 guard。不修改实际 cell pixels。
参见[协议与回归说明](../../docs/raster-design.md#encoder-and-budgets)。
本次仅验证内存编码及 PTY 协议，尚未在真实图形终端/复用器上验证 inline 显示。

Unix/WSL 可运行完整启动/输出的 PTY 回归（不是实际图形终端）：

```sh
python3 scripts/check-sixel-query.py --formula
python3 scripts/check-sixel-query.py --formula 'integral_0^1 x^2 dif x = frac(1, 3)'
```

这会检查无 override 的 OS/CSI 查询路径、普通屏幕且无鼠标捕获、完整相对定位/DCS
边界、紧凑内容行数及安全底部余量、12×6 窄矮窗口 resize、q/Esc 退出及光标/termios
恢复，以及探测失败路径。启动光标模拟在底行，确保覆盖绘制前的行预留。PTY 的 10×20 是合成测试指标，不是建议用户设置的 override。

示例显式使用 dev-dependency `typst-assets = 0.13.1` 的 `fonts` feature，其中包含
New Computer Modern Math；不扫描系统目录，不在运行时联网。不要省略该 feature：
未开启时字体迭代器为空。

Typst 的编译依赖较重。内存紧张时可在 POSIX shell 使用：

```sh
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test -p rsille-typst -j 1
CARGO_PROFILE_DEV_DEBUG=0 cargo build -p rsille-typst --all-targets -j 1
```

## 在应用中使用

本扩展尚未声称已发布到 crates.io。使用本地 checkout，例如在仓库根目录下的
独立应用中配置（路径应相对于你的应用 manifest 调整）：

```toml
[dependencies]
rsille-typst = { path = "../packages/rsille-typst" }
# 仅在选用下面的可部署字体方案时需要：
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
        pixels_per_point: 2.0,
        foreground: Rgb(230, 230, 230),
        background: Rgb(24, 24, 24),
        max_pixels: 4 * 1024 * 1024,
    };
    let result = renderer.render_with_diagnostics(
        r"integral_0^1 x^2 dif x = frac(1, 3)",
        &options,
    )?;
    for warning in result.warnings {
        eprintln!("{}", warning.message);
    }
    let canvas = result.canvas;
    println!("{} × {}", canvas.width(), canvas.height());
    Ok(())
}
```

### 字体部署

公共库**不捆绑任何隐式字体**。`MathRenderer::new` 接受可迭代的字体字节
（`AsRef<[u8]>`），复制并解析一次，支持 TTF、OTF、TTC，注册集合中的所有可读 face。
选定 family 至少需要一个带 OpenType MATH 表的 face；字体损坏、空字体集合、
不存在或不支持数学的 family 会返回错误。所需字形缺失等情况可能产生 Typst warning，
请用 `render_with_diagnostics` 检查。可以额外传入文字字体作为 fallback。

除 `typst-assets` 外，也可以由调用方自行部署字体并显式读取：

```rust,no_run
use rsille_typst::MathRenderer;
# fn main() -> Result<(), Box<dyn std::error::Error>> {
let font = std::fs::read("/your/deployed/fonts/NewCMMath-Regular.otf")?;
let renderer = MathRenderer::new([font], "New Computer Modern Math")?;
# Ok(()) }
```

这里文件读取发生在调用方，不是 Typst World 中。也可以用 `include_bytes!`
把自己的字体编入应用。请保留所选字体的许可证/署名，遵循字体本身的分发条款；
本 crate 的 MIT 许可证不会替代字体许可证。字体资产来源：
[typst-assets v0.13.1](https://github.com/typst/typst-assets/tree/v0.13.1)。

### 接入现有 rsille 展示路径

无需另造 Widget。应用另行依赖同一份 `rsille` 后：

```rust,ignore
use rsille::tui::prelude::*;

// canvas 来自 renderer.render(...)；cell_pixels 应取自实际后端/终端配置。
let size = cell_pixels.size_for_pixels(canvas.width(), canvas.height())
    .ok_or("invalid cell metrics or terminal size overflow")?;
let widget = raster_canvas::<()>(canvas)
    .scale(RasterScale::Original)
    .fixed(size.width, size.height);
```

也可以 `chunk.raster(&canvas, RasterOptions { scale: RasterScale::Original,
..Default::default() })?`。原始像素尺寸不等于字符格尺寸；不要硬编码字体像素与
终端格子的换算。`Original` 保留原始比例/尺寸并允许裁剪；需要缩放时选择 `Fit`。
Sixel 仍由 rsille 后端配置决定，字符 fallback 可继续使用。

## 公开接口

- `MathRenderer::new(font_bytes, math_family) -> Result<MathRenderer, Error>`：复用字体、
  FontBook 和模板源码；不依赖系统资源。
- `render(&self, formula: &str, &RenderOptions) -> Result<RasterCanvas, Error>`：简便接口，
  忽略非致命 warning。
- `render_with_diagnostics(...) -> Result<RenderedMath, Error>`：返回 `canvas` 和原始
  `warnings: Vec<SourceDiagnostic>`。
- `RenderOptions`：字号、像素比例、RGB 前景/背景、最大像素数，支持 `Default`。
- `Error`：字体/配置错误、`Compile(Vec<SourceDiagnostic>)`、页数错误、尺寸限制、
  `Raster(RasterError)`；实现 `Display` 和 `std::error::Error`。
- 重新导出 `RasterCanvas`、`RasterError`、`Rgb` 和 `SourceDiagnostic`，便于调用。

公式示例：`x^2`、`frac(1, 2)`、`sqrt(x)`、`sum_(k=1)^n k`。
**不要添加外围 `$ ... $`**；`\frac{1}{2}` 不是本 API 的语法。

模板自动页面宽高、零页边距、块级 equation。字号单位为 pt，1pt = 1/72 英寸；
像素比例为 pixels/pt，默认 2（144dpi）。Typst 自身的数学布局/行度量仍会保留，
这里不做逐像素 ink 裁剪。空公式按 Typst 的布局结果处理，渲染轴至少为 1 像素。

## 颜色、错误与资源边界

- 前景/背景是 8-bit 不透明 sRGB。公式内部的 Typst 样式可以覆盖默认前景。
- 页面初始透明；`typst-render` 返回 tiny-skia 的**预乘 alpha** RGBA。
  输出按 `premultiplied_rgb + background * (1-alpha)` 舍入合成，不重复乘 alpha，
  不先反预乘。结果是无 alpha 的 RGB，背景同时用作 `RasterCanvas` 元数据。
- 字号必须有限且在 `0.1..=4096`，比例必须有限且在 `0.01..=16`；非法配置在
  编译前返回 `InvalidOptions`，不进入上游 renderer。
- 编译结果必须恰好一页。渲染前按照固定版本上游的 f32 乘法和 `round().max(1)`
  检查两轴，每轴最多 16384 像素，总像素数不超过 `max_pixels` 和核心
  `MAX_RASTER_PIXELS`（16 × 1024 × 1024）。可降低 `max_pixels`，不可提高核心上限。
- 预检后才调用真实 raster renderer。输出 RGB 分配使用 `try_reserve_exact`，
  再交给 `RasterCanvas::from_pixels` 做最终长度/尺寸检查。
- 总像素上限不是整个进程内存上限：RGBA 和 RGB 输出缓冲区会同时存在，还需要
  字体、编译树和布局资源。上游的分配失败/内部 panic 不保证能转换为 `Error`。
- Typst 编译错误保留原始消息、hint、span、trace，不替换为泛化的“渲染失败”。
  因为公式通过 `eval(..., mode: "math")` 传入，错误 span 指向模板调用，
  **不提供原始公式的精确行列映射**。`Display` 展示消息和 hints，完整信息在诊断中。
- `warnings` 保留编译器实际返回的 warning。但 Typst 0.13.1 的立即 `eval`
  使用独立 sink，不向外传播其中的 warning（例如直接调用 `text` 时的未知字体提示）；
  布局/延迟 context 阶段返回的 warning 可正常保留。本接口不能恢复上游丢弃的 warning。

## 安全与缓存限制

World 只接受自身内存主源码和构造时的字体。任意其它 source 请求（包括包）以及
所有 file 请求均返回 `AccessDenied`；无路径解析、文件读取、包下载或系统字体搜索。
当前日期不可用。公式作为 `sys.inputs` 字符串传入数学模式 evaluator，不能通过
闭合 `$` 把字符串拼接进页面模板；不过合法的 Typst `#` 表达式仍可执行。

**禁用文件访问不等于强沙箱。** 循环、递归、大型表达式、内嵌数据/图像或复杂布局
仍可能在页面尺寸检查前或渲染期间消耗大量 CPU/内存。没有超时、取消、执行步数上限，
也没有进程隔离。不要在 UI 帧循环中重复编译；对不可信输入使用独立 worker 进程，
由宿主实施时间/内存限制，必要时终止进程。

此 crate 不缓存公式结果。Typst/comemo 和内部 interner 自身存在进程级缓存/存储，
本库不改变全局缓存策略，也不承诺长时间处理无限不同公式时内存有固定上界。
丢弃 renderer 不等于清空所有上游缓存；有严格上限的服务应轮换 worker 进程。

## 版本与验证

`typst`、`typst-render` 和测试/示例字体资产固定为匹配的 **0.13.1**，使用该版本
已核对的 World、数学模式 eval 和像素舍入 API，不自动跨 Typst 版本升级。
这不是“最新 Typst”声明。上游 Typst 声明 Rust 1.80，但 workspace 及当前传递依赖
可能要求更高版本；本包未承诺 1.80 MSRV，实际验证工具链为 Rust/Cargo **1.97.1**。

依赖的本地 rsille 包版本为 3.0.4，需要提交 `57c9ad5` 中的公开 raster 集成 API；
不要假定同版本旧的已发布核心具有这些接口。发布扩展前应先确认核心的已发布版本。
workspace 按现有惯例忽略 `Cargo.lock`；本次验证使用本地解析后的共享 lock，
生产应用应提交自己的 lockfile 来固定传递依赖。

测试包含真实编译/渲染、重复性、字号与比例、背景与抗锯齿、半透明内容、编译错误、
文件/包/日期拒绝、模板逃逸拒绝、字体校验、非法配置、渲染前资源限制，以及已有
rsille Widget 对接；另外有预乘 alpha/尺寸舍入单元测试及公开 API doctest。
