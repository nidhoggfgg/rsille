# Neon Observatory / 霓虹观测站

[English](README.md) · 中文

通过 rsille 彩色盲文画布绘制六种程序化场景：环形行星、光子隧道、霓虹山谷、螺旋星系、
金属环面和湖畔极光。所有画面在 CPU 上实时计算，无需图片素材或网络服务。

## 运行

在仓库根目录执行：

```bash
cargo run --release --manifest-path examples/neon-observatory/Cargo.toml
```

应用以 inline 模式运行，最多占用 140 列、40 行，窗口更小时按可用尺寸缩小。
建议使用 True Color 和包含盲文字符的字体。保留之前的终端输出，退出后回到命令行。

## 操作

| 输入 | 操作 |
| --- | --- |
| 1–6 | 切换行星、隧道、地形、星系、环面、极光 |
| p / c / s | 暂停 / 下一套配色 / 切换 1×、2×、0.5× 速度 |
| ? | 开关帮助 |
| Tab / Shift+Tab、Enter / 空格 | 聚焦和激活播放控件 |
| 鼠标点击 | 激活场景或播放按钮 |
| q / Esc | 退出 |

六种场景共用模拟时间。场景按钮支持点击，但不进入键盘焦点顺序，可用数字键直接切换。
小窗口可能裁切控件，快捷键仍可使用。

## 验证与导出

```bash
cargo test --manifest-path examples/neon-observatory/Cargo.toml
cargo run --release --manifest-path examples/neon-observatory/Cargo.toml -- --headless
cargo run --release --manifest-path examples/neon-observatory/Cargo.toml -- --snapshot /tmp/neon.svg 4 12
cargo run --release --manifest-path examples/neon-observatory/Cargo.toml -- --benchmark
```

`--snapshot FILE.svg [1-6] [SECONDS]` 导出实际的 140 × 40 Buffer，默认是第 12 秒的场景 1。
`--headless` 检查播放、场景、控件、焦点、窗口调整和退出。
性能测量包含更新、布局与绘制，不包含终端 I/O。

[scenes.rs](src/scenes.rs) 定义场景几何与公式，[raster.rs](src/raster.rs) 处理光能、光晕和曝光，
[main.rs](src/main.rs) 连接 TUI 与验证逻辑。三维场景由示例实现，rsille 提供画布编码、布局和输入。
颜色与画面比例受终端和字体影响。

[全部示例](../README_cn.md)
