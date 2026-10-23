# Folded Harbor / 折叠星港

**视角就是机关：看起来相接的地方，就能跨越。**

旋转一座悬浮在星空中的立体建筑群，让相隔不同深度的两个通道口在屏幕上重合。
两道环变白并锁定后，按空格折叠空间，带着光点穿过原本无法抵达的间隙。
共有三个章节、九次折叠：潮汐档案馆、失落信号花园、最后的灯塔。

从仓库根目录运行，推荐使用 **release**：

```bash
cargo run --release --manifest-path examples/folded-harbor/Cargo.toml
```

先看约一分钟的完整自动演示：

```bash
cargo run --release --manifest-path examples/folded-harbor/Cargo.toml -- --demo
```

支持真彩色和 Unicode 盲文的终端会得到最好的效果，推荐 **144 × 44**。
**80 × 24** 也能玩；达到 **116 × 34** 时显示侧边观察面板，小窗口隐藏它。
`--inline` 可在普通终端屏幕中运行；默认全屏。画面全部实时生成，不读取图片或模型资源。

## 怎么玩

1. 找到 `A / DEPARTURE` 和 `B / ARRIVAL` 标记旁的两道发光圆环。
2. 用方向键、WASD 或鼠标左键拖动改变观察方向，使两环重合。
3. 保持视角约 0.4 秒。圆环变白时，`FOLD [Space]` 按钮会出现在通道旁。
4. 按 `Space` / `Enter`，或点击这个按钮。光点会穿过折叠通道抵达下一座岛。
5. 每章完成三次折叠，继续进入下个章节。

第一道机关可以尝试向右转、降低仰角。拿不准时按 `H` 平滑辅助对齐，再按空格跨越。
可以按 `T` 观看自动解谜，再按任意旋转键接管。自动演示通关后会重新开始。

| 操作 | 按键 / 鼠标 |
| --- | --- |
| 水平旋转 | `←` / `→` 或 `A` / `D` |
| 改变仰角 | `↑` / `↓` 或 `W` / `S` |
| 自由观察 | 鼠标左键拖动场景 |
| 缩放 | 滚轮、`+` / `-` |
| 折叠 / 下一章 / 重新开始 | `Space` / `Enter` 或按钮 |
| 辅助对齐 | `H`；计入本次游玩的辅助次数 |
| 开关自动演示 | `T` |
| 暂停 / 继续 | `P` / `Esc`；`Space` 也可继续 |
| 手册 | `?`；`Space` / `Esc` / `?` 关闭 |
| 从第一章重新开始 | `R` |
| 退出 | `Q` / `Ctrl+C` |

折叠期间相机方向锁定；可以缩放。暂停与手册会冻结模拟。
缩放和窗口尺寸只改变画面大小，对齐判定始终使用固定的投影空间容差。
手册的 Tab 焦点限制在弹窗中，后台操作不会重置游戏。
较小窗口可能裁切部分说明和工具栏，所有快捷键仍然有效。

## 框架能力如何进入玩法

这个示例把画布、组件和交互放在同一空间里：

- **实时盲文场景**：带光照的实体建筑、阶梯、悬浮环、星云和体积感光晕。
  绘制先在 CPU 光照缓冲中完成，再为每个盲文单元拟合前景 / 背景两种颜色。
- **三维位置上的普通 TUI 组件**：A/B 标签和可点击通道按钮随三维投影移动。
  自定义 `Scene::layout` 给它们分配真实布局位置，命中测试由框架处理。
- **组件树的事件冒泡**：画布接收鼠标目标事件，父组件把拖动和滚轮变成应用消息。
  拖动起点通过 `WidgetStore` 保留，视图逐帧重建时仍连续。
- **状态驱动的视觉效果**：章节切换使用 `visual` 溶解揭幕，标题渐变随时间流动，
  手册与章节弹窗淡入；进入不同章节时组件主题与场景色彩一起变化。
- **统一的应用循环**：相机阻尼、锁定计时、光点跨越、界面进度和自动演示
  通过 `FrameInfo`、同一个 `update` 和 `view` 同步推进。
- **响应布局和模态焦点**：窄窗口收起观察面板，手册和章节完成弹窗限制焦点。
- **无终端验证**：相同应用可注入手动时钟，验证真实鼠标拖动、三维按钮点击和章节交互。

三维投影、几何光照和空间解谜规则由示例实现；rsille 提供组件、布局、渲染、事件、
运行时和视觉效果接口。没有引入游戏引擎或图形窗口。

## 源码与验证

| 文件 | 用途 |
| --- | --- |
| `world.rs` | 三维向量、相机投影、关卡、对齐判定、跨越状态、自动演示 |
| `scene.rs` | 几何体、可见面光照、星云、圆环、粒子和光点 |
| `raster.rs` | CPU 光照缓冲、泛光、盲文双色采样；沿用霓虹观测站的绘图方法 |
| `main.rs` | 应用、界面、三维组件布局、键鼠消息路由和弹窗 |
| `verify.rs` | 真实应用流程检查、SVG 帧导出、帧耗时测量 |

```bash
cargo test --manifest-path examples/folded-harbor/Cargo.toml
cargo clippy --manifest-path examples/folded-harbor/Cargo.toml --all-targets -- -D warnings
cargo run --release --manifest-path examples/folded-harbor/Cargo.toml -- --headless
cargo run --release --manifest-path examples/folded-harbor/Cargo.toml -- --benchmark

# 导出实际终端 buffer，不依赖 TTY；--at 指定自动演示的时间
cargo run --release --manifest-path examples/folded-harbor/Cargo.toml -- --snapshot /tmp/harbor.svg --at 1.25
cargo run --release --manifest-path examples/folded-harbor/Cargo.toml -- --snapshot /tmp/fold.svg --at 4.5
```

测试覆盖九道机关都有空间间隙与可达投影解、对齐保持判定、缩放不改变容差、
暂停 / 手册冻结、跨越期间的视角锁定、完整自动通关、实际三维按钮点击、
拖动 / 滚轮、弹窗焦点、窗口缩放、辅助次数、重开和章节切换。

`--benchmark` 测量 **144 × 44** 下状态更新、组件布局和场景绘制的 CPU 耗时，
不包含终端 I/O。实际帧率也取决于终端处理真彩色输出的速度。
