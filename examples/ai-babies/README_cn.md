# AI Babies

[English](README.md) · 中文

以 ChatGPT、DeepSeek 和 Claude 为灵感的三个像素动画角色，每次显示一个。
每个角色有六组动作、24 个预制帧槽位：待机眨眼、挥手、原地走路、跳跃、思考和睡觉。
默认选择 ChatGPT，自动播放列表约 17 秒循环一次。

![ChatGPT 角色](preview-pet.gif)

| DeepSeek | Claude |
| --- | --- |
| ![DeepSeek 动作](preview-deepseek.gif) | ![Claude 动作](preview-claude.gif) |

## 运行

在仓库根目录执行：

```bash
cargo run --release --manifest-path examples/ai-babies/Cargo.toml
cargo run --release --manifest-path examples/ai-babies/Cargo.toml -- --character deepseek
cargo run --release --manifest-path examples/ai-babies/Cargo.toml -- --character claude --height 24
```

默认动画使用 inline 模式、True Color 和 2 × 2 Quadrant 编码。默认高度上限为 24 行，
`--height ROWS` 仅接受 24 或 32，播放时受实际终端高度限制。
推荐 80 × 26 的终端，为 shell 提示符留出空间；透明像素沿用终端默认背景。

24 行使用 96 × 48 帧，32 行使用 128 × 64 帧，均从原图直接采样。
横向采样翻倍，实际显示比例不变；小窗口会缩小显示，更高的区域只增加留白，
角色不会超过 128 × 64 个采样。每格归并为终端支持的两种颜色；部分透明的格子
使用一种前景色并保留终端背景。旧路径内嵌原始像素数据，无需图片解码器。
所有播放模式在运行时均无需 Python 或网络。上方 GIF 是现有的旧版 Quadrant 预览，
不是新的 raster/Sixel 输出。

## Raster 与 Sixel

以下可选动画模式不改变默认的透明 Quadrant 播放、静态模式及现有导出行为：

```bash
cargo run --release --manifest-path examples/ai-babies/Cargo.toml -- --sixel --character claude
cargo run --release --manifest-path examples/ai-babies/Cargo.toml -- --sixel --cell-pixels 8x16
```

- `--sixel` 使用 inline 动画，不会静默回退。无需额外的 `--inline` 参数。
  所有动画模式都不进入备用屏幕，也不捕获鼠标事件。渲染模式择一使用。
- `--background RRGGBB` 可用于任一模式，默认黑色；允许前导 `#`（在 shell 命令中需加引号）。
  RGBA 会合成到此不透明背景上，两种模式均不保留终端透明背景。
- `--cell-pixels WxH` 仅用于 Sixel，指定**实际物理字符格的像素尺寸**，不是终端的列数和行数。
  省略时通过 `CellPixels::resolve` 在启动时探测。探测先于输入处理，尺寸解析失败会报错。
  `--force-cell-pixels` 只能与 `--sixel --cell-pixels` 同用，允许使用与终端网格一致的实测尺寸覆盖。
  字体或 DPI 改变后需重启。

选择 `--sixel` 表示你确认**整个终端/复用器链路**支持 Sixel、稳定调色板、EL 2 局部清行时
擦除图形，并且图像随文字滚动；Sixel 定位输出必须安全，末尾不完整条带的透明 padding 不得擦除相邻字符格。
配置使用 `reliable_inline_erase: true` 和 `reliable_erase: false`，不要求备用屏幕的 ED 2 图形擦除。
`no_scroll` 要求 Sixel 输出本身不滚屏；inline 预留底部保护行容纳末带 padding，而非仅依赖透明性。
预留行时允许受控的文字滚动。inline 独占其占用行的完整宽度，而不仅是图片所占的列。
不自动检测这些能力；它们是兼容性要求，
无终端检查也不能验证这些能力。当前 sixel 编码器使用固定 RGB332 256 色调色板，
因此原生输出提升的是空间分辨率，并非无损 24 位真彩色。

两种模式均使用三个角色的内嵌 PNG 图集，每帧 320 × 320。现有打包脚本直接从原始
256 × 256 源图单元生成：这是放大采样，并非新增原生 320 像素细节，也不是放大旧的
128 × 64 帧。新增的 `png` 0.18 解码器在启动时一次性解码并缓存帧。
`--height` 仍仅接受 24/32（默认 24）并限制内容高度。Sixel 靠左显示，最多约占 320 × 320 个物理像素，
内容行数还受图片所需行数及终端扣除空白保护行后的剩余高度限制。空间足够时，10×20 字符格使用
**16 行图片内容 + 1 行空白保护行**，而非总共 16 行；8×18 无需额外保护行。
字符格高度为 `h` 时，保护行数为 `ceil(((6 - h % 6) % 6) / h)`，因此极小的 `h = 1` 需要 5 行。
成功绘帧后光标仍回到最后内容行，退出换行可落在保护行。较小终端会缩小显示以适应空间。
Raster 不再提供字符预览；默认 Canvas 模式保持不变。

## 操作

| 按键 | 操作 |
| --- | --- |
| Tab / g / d / c | 下一角色 / ChatGPT / DeepSeek / Claude |
| 1 / 2 / 3 / 4 / 5 / 6 | 待机 / 挥手 / 走路 / 跳跃 / 思考 / 睡觉 |
| 0 | 从头播放自动列表 |
| 空格 | 暂停或继续 |
| q / Esc / Ctrl+C | 退出 |

选择动作会从头播放，切换角色保留所选动作并从第一帧重新播放。
这两种操作都会保留当前暂停状态。

## 动画验证与导出

```bash
cargo test --manifest-path examples/ai-babies/Cargo.toml
cargo run --release --manifest-path examples/ai-babies/Cargo.toml -- --headless
cargo run --release --manifest-path examples/ai-babies/Cargo.toml -- --snapshot /tmp/ai-babies.ppm 0
cargo run --release --manifest-path examples/ai-babies/Cargo.toml -- --character deepseek --height 24 --frames /tmp/deepseek-frames
cargo run --release --manifest-path examples/ai-babies/Cargo.toml -- --character claude --action wave /tmp/claude-wave
cargo run --release --manifest-path examples/ai-babies/Cargo.toml -- --sixel --headless
```

`--character`、`--height` 及新的渲染参数必须放在导出选项之前，彼此顺序不限。
导出选项择一使用：

| 选项 | 输出 |
| --- | --- |
| `--headless` | 无终端检查 90 个采样帧 |
| `--snapshot FILE.ppm [SECONDS [COLUMNS ROWS]]` | 单帧，默认第 0 秒；显式尺寸覆盖 `--height` |
| `--frames DIRECTORY` | 按 24 FPS 导出完整播放列表 |
| `--action idle\|wave\|walk\|hop\|think\|sleep DIRECTORY` | 按 24 FPS 导出单个动作循环 |

快照时间范围为 0–86400 秒，显式尺寸范围为 1–512 列、1–128 行。
每张 PPM 配套同名 `.pgm` 透明度掩码。默认旧模式下，单独打开 PPM 时透明区域显示为黑色，
该黑色不会用于终端绘制。

`--sixel` 仅允许 `--headless`；拒绝 Raster 的 `--snapshot`、`--frames`、`--action` 图片导出，
避免把占位字符当作图像。默认 Canvas 模式继续支持图片导出。
无终端模式测试绘制，但**不能**证明终端/复用器兼容。

请使用新的输出目录，避免混入旧帧。安装 Pillow 后，可从两种文件制作预览
（仅旧路径保留透明度）：

```bash
python3 examples/ai-babies/tools/preview_actions.py /tmp/deepseek-frames --name deepseek
```

预览脚本会在本示例目录写入 `preview-<name>.png`，并替换 `preview-<name>.gif`。

测试覆盖动作时序、角色切换、暂停、窗口调整、透明边缘、编码颜色，以及待机和挥手的静止像素一致性。

## 静态三角色模式

![静态三角色预览](preview-static.png)

```bash
cargo run --release --manifest-path examples/ai-babies/Cargo.toml -- --static
cargo run --release --manifest-path examples/ai-babies/Cargo.toml -- --static --height 20 --snapshot /tmp/ai-babies-static.ppm
```

`--static` 必须放在最前面。此模式输出一次后退出，使用 Quadrant 编码并保留参考图白底。
默认 144 列 × 24 行，`--height` 接受 1–40，宽度为高度的六倍。
使用独立的 240 × 80 RGB 源图，不使用动画图集。

## 素材与源码

[scene.rs](src/scene.rs) 选择动作并采样内嵌 RGBA 帧，[main.rs](src/main.rs) 管理播放与输入，
[raster.rs](src/raster.rs) 解码并缓存高分辨率画布，[options.rs](src/options.rs) 校验渲染选项。
[export.rs](src/export.rs) 还原编码后的终端字符格，[static_image.rs](src/static_image.rs)
实现静态模式。预览通过字符块还原，接缝和比例可能与实际终端字体不同。

| 文件 | 用途 |
| --- | --- |
| [chibi-trio-source.png](assets/chibi-trio-source.png) | 三角色原始参考图 |
| [chatgpt-actions-source.png](assets/chatgpt-actions-source.png) | ChatGPT 动作源图 |
| `assets/{deepseek,claude}-actions-clean-source.png` | 另外两个角色的动作源图 |
| `assets/*-actions-*.png` 和 `.rgba`（96x48/128x64） | 旧版预览图集与内嵌动画像素 |
| `assets/*-actions*-320x320.png` | raster/Sixel 内嵌高分辨率 PNG 图集，无 320x320 `.rgba` 文件 |
| `assets/chibi-trio-240x80.png` 和 `.rgb` | 静态源图与内嵌像素 |
| `assets/*-prompt.txt` | 生成与清理提示词 |

这些是非官方角色演绎。打包使用固定调色板和二值透明度，待机与挥手共用身体像素。
需要重建像素数据时，安装 Pillow 后运行对应脚本：

```bash
python3 examples/ai-babies/tools/pack_actions.py
python3 examples/ai-babies/tools/pack_companions.py
python3 examples/ai-babies/tools/pack_static.py
```

[全部示例](../README_cn.md)
