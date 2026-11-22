# Canvas 调试

[English](README.md) · 中文

以下命令从仓库根目录运行。示例默认编入 Kitty 和 Sixel 两个后端。

```sh
cargo run --manifest-path examples/canvas/Cargo.toml -- --basic
cargo run --release --manifest-path examples/canvas/Cargo.toml
cargo run --release --manifest-path examples/canvas/Cargo.toml -- --backend kitty
cargo run --release --manifest-path examples/canvas/Cargo.toml -- --backend sixel
```

`--basic` 用字符 Canvas 输出正弦曲线。默认进入全屏场景，两个浮窗用原生 RasterCanvas 和字符 Canvas 绘制同一颗行星。默认 `auto` 从已编译的后端中优先选 Kitty，再选 Sixel。顶部显示实际协议和每格像素尺寸。可在同一终端分别指定两个后端，对照以下操作。

1. 把字符窗口拖到行星中央，检查前景的文字、空格和边框是否完全遮住后方图像。
2. 把窗口移开，再点行星窗口的标题置前。检查露出的图像是否完整，桌面是否还有残影。
3. 拖动底部手柄调整窗口大小，再把窗口移到屏幕底边。检查行星是否按比例缩放，底行有无错位或意外滚屏。
4. 按 `e` 切换字符编码，`r` 恢复窗口位置和尺寸，`q` 或 Esc 退出。也可用 Tab 聚焦标题、方向键移动、Shift+方向键缩放。

建议使用至少 100×32 字符格。在 Unix（含 macOS、WSL）上显式指定协议也会探测支持情况；`--cell-pixels WxH` 可提供实测的单格物理像素尺寸。原生 Windows 需同时指定已验证的后端和实测尺寸。全屏 Sixel 要求 ED 2 能清除图形、调色板稳定、补齐透明，且定位输出不会滚屏。

```sh
cargo run --manifest-path examples/canvas/Cargo.toml -- --headless
cargo test --manifest-path examples/canvas/Cargo.toml
python3 scripts/check-sixel-query.py --both-backends
```

`--headless` 在内存中验证鼠标操作与遮挡恢复。PTY 脚本在 Unix 上检查协议协商、输出和键盘输入。实际显示效果可按上述步骤在目标终端观察。
