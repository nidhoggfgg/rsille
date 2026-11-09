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

动画使用 inline 模式、True Color 和 2 × 2 Quadrant 编码。默认高度上限为 24 行，
`--height ROWS` 仅接受 24 或 32，播放时受实际终端高度限制。
推荐 80 × 26 的终端，为 shell 提示符留出空间；透明像素沿用终端默认背景。

24 行使用 96 × 48 帧，32 行使用 128 × 64 帧，均从原图直接采样。
横向采样翻倍，实际显示比例不变；小窗口会缩小显示，更高的区域只增加留白，
角色不会超过 128 × 64 个采样。每格归并为终端支持的两种颜色；部分透明的格子
使用一种前景色并保留终端背景。像素数据内嵌在二进制中，
播放无需网络、Python 或图片解码器。

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
```

`--character` 和 `--height` 必须放在导出选项之前，两者顺序不限。导出选项择一使用：

| 选项 | 输出 |
| --- | --- |
| `--headless` | 无终端检查 90 个采样帧 |
| `--snapshot FILE.ppm [SECONDS [COLUMNS ROWS]]` | 单帧，默认第 0 秒；显式尺寸覆盖 `--height` |
| `--frames DIRECTORY` | 按 24 FPS 导出完整播放列表 |
| `--action idle\|wave\|walk\|hop\|think\|sleep DIRECTORY` | 按 24 FPS 导出单个动作循环 |

快照时间范围为 0–86400 秒，显式尺寸范围为 1–512 列、1–128 行。
每张 PPM 配套同名 `.pgm` 透明度掩码。单独打开 PPM 时透明区域显示为黑色，该黑色不会用于终端绘制。
请使用新的输出目录，避免混入旧帧。安装 Pillow 后，可从两种文件制作透明预览：

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
[export.rs](src/export.rs) 还原编码后的终端字符格，[static_image.rs](src/static_image.rs)
实现静态模式。预览通过字符块还原，接缝和比例可能与实际终端字体不同。

| 文件 | 用途 |
| --- | --- |
| [chibi-trio-source.png](assets/chibi-trio-source.png) | 三角色原始参考图 |
| [chatgpt-actions-source.png](assets/chatgpt-actions-source.png) | ChatGPT 动作源图 |
| `assets/{deepseek,claude}-actions-clean-source.png` | 另外两个角色的动作源图 |
| `assets/*-actions-*.png` 和 `.rgba`（96x48/128x64） | 当前预览图集与内嵌动画像素 |
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
