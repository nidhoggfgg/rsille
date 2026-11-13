# rsille

[English](README.md) · 中文

<img align="right" src="examples/ai-babies/preview-deepseek.gif" width="96" alt="">

**让终端，惊艳起来。**

**rsille 是 Rust 终端应用与图形框架**：
把组件布局、彩色图形、动画特效和键鼠交互放进同一套 API。
用 Unicode Canvas 绘制字符图形，用 Sixel 输出像素图像；
画布、编辑器、表格和浮窗可以组合成同一个应用，
从轻量 inline 工具到全屏交互工作台，
不必分别搭建渲染与事件循环。

## 一个终端，可以有多少种模样？

| 像素级绘图 · Sixel | 桌面式布局 · TUI |
| --- | --- |
| [![函数绘图的离线 RGB 预览：曲面、螺旋线和三叶结](examples/function-plotter/preview.gif)](examples/function-plotter/README_cn.md) | [![Lumen 的三维画布、实时曲线和可拖动浮窗](examples/lumen/preview-desk.png)](examples/lumen/README_cn.md) |
| **图形不再受字符形状限制。** RGB 像素画布通过 Sixel 输出，支持布局、裁剪、滚动与不透明遮挡；在兼容终端中，将精细曲线、图像和自定义可视化嵌入界面。 | **终端界面不必只有固定面板。** Flex/Grid 布局组合编辑器、表格和画布；浮窗支持拖动、调整大小、收起和恢复，配合焦点管理与命令面板，构建多区域工作台。 |

| 连续动画 · Unicode Canvas | 图形与组件 · 同一套交互 |
| --- | --- |
| [![132 个零件拆解、组装并变形成机械生物](examples/clockwork/preview.gif)](examples/clockwork/README_cn.md) | [![三维战场中的皮卡丘、水箭龟与招式按钮](examples/pokemon-battle/preview.png)](examples/pokemon-battle/README_cn.md) |
| **字符格也能承载动态场景。** 盲文、半块和四分块编码支持彩色采样图形，配合逐帧更新与输入处理，可呈现程序化动画和自行投影的三维场景，而不只是静态图表。 | **画布不是界面里的孤岛。** 图形与按钮、状态条共用布局和事件系统；世界坐标与指针转换保持一致，再组合渐变、扫光、毛玻璃与过渡，让可视化既能看，也能操作。 |

Function Plotter 为离线 RGB 预览，非终端录屏，
不包含 Sixel 量化与刷新行为；
其余预览由渲染后的终端字符格导出，Clockwork 的 GIF 做了加速。
场景和模拟逻辑由示例提供，rsille 负责画布、编码、组件、渲染与事件循环。

## 效果惊艳，写起来也顺手

<img align="right" src="examples/ai-babies/preview-pet.gif" width="96" alt="">

- **声明界面，不手写事件循环。** `view` 描述组件，`update` 处理消息与状态；
  运行时统一处理焦点、输入、后台任务和退出清理。
- **组合特效，不重写组件。** `visual(...)` 为字符组件叠加渐变、扫光、溶解和波浪，
  保留原有交互；内置视觉动画由运行时驱动，无需手写逐帧循环。
- **按应用选择呈现方式。** 全屏构建完整工作台，inline 留在普通终端屏幕；
  不需要组件树时，也能直接使用 Canvas 和渲染器。
- **交互和异步逻辑也能自动化测试。** 无需真实终端即可回放输入、检查渲染帧，
  用手动时钟和执行器控制计时器与后台任务。

**更多示例**：
[发光水母](examples/jellyfish/README_cn.md)、
[方块沙盒](examples/minecraft/README_cn.md)、
[拖放文件管理器](examples/orbit-files/README_cn.md)、
[七页组件展厅](examples/tui-showcase/README_cn.md)。

## 配合 AI 编程助手开发

[rsille skill](https://github.com/nidhoggfgg/rsille.skills/blob/main/rsille/SKILL.md)
帮助编程助手使用 v3 API 开发终端应用，
涵盖组件与布局、后台任务、输入处理、Canvas 图形、动画和无终端测试，
并提供
[组件索引](https://github.com/nidhoggfgg/rsille.skills/blob/main/rsille/references/components.md)
及可独立运行的应用模板，
可通过 [Skills CLI](https://github.com/vercel-labs/skills) 安装：

```bash
npx skills add nidhoggfgg/rsille.skills --skill rsille
```

## 在应用中引入 rsille

```toml
[dependencies]
rsille = "3.1.0"
```

默认启用 `sixel` feature，
可选 `syntax-highlighting` 为代码查看器提供 Syntect 语法高亮。

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

## 绘图：Unicode Canvas 与 Sixel

**一个字符格，也能画出更多细节。**
`Encoding::Quadrant` 将每个终端字符格细分为 **2 × 2 个采样位置**，
通过四分块字符与前景、背景颜色组合，
在有限的行列里表现更细的轮廓。
120 × 20 个字符格，对应 240 × 40 的采样网格。

[![120 × 20 字符格中使用 2 × 2 四分块编码渲染的像素角色](imgs/readme-canvas-quadrant.png)](examples/ai-babies/README_cn.md)

上图是 [AI Babies 静态模式](examples/ai-babies/README_cn.md) 的实际 Canvas 输出，
仅占 **20 行**；预览按字符格比例放大，便于查看细节。

| 编码 | 每个字符格的采样数 | 用途 |
| --- | --- | --- |
| `Encoding::Braille`（默认） | 2 × 4 | 密集线条、曲线和彩色场景 |
| `Encoding::HalfBlock` | 1 × 2 | 上下两个可独立着色的采样点 |
| `Encoding::Quadrant` | 2 × 2 | 四分块形状，采样颜色拟合为终端颜色 |

**支持 Sixel，让终端图形不再受字符形状限制。**
在支持 Sixel 的终端中呈现精细曲线、彩色图像与丰富的可视化，
并与文本、按钮和其他组件融为一体。

## 扩展：Typst 数学公式

独立扩展 [`rsille-typst`](packages/rsille-typst/README.md)
将 Typst 数学语法渲染为 `RasterCanvas`，在兼容的 Sixel 终端中展示公式，
不向核心 crate 添加 Typst 依赖。


## API 与资料入口

| 公开模块 | 职责 | 源码 |
| --- | --- | --- |
| `rsille::tui` | 应用、组件、布局、输入和特效 | [src/tui](src/tui) |
| `rsille::render` | Buffer、带样式的 Unicode 文本、绘制与终端循环 | [src/render](src/render) |
| `rsille::canvas` | 采样图形与终端编码 | [src/canvas](src/canvas) |
| `rsille::raster` | RGB 栅格画布、Sixel 输出与能力配置 | [src/render/raster](src/render/raster) |

- [示例目录](examples/README_cn.md)
- [TUI 小示例](packages/tui/examples/README_cn.md)
- [Sixel 与 Canvas 对比示例](examples/sixel_windows.rs)
- [MIT 许可](LICENCE)
