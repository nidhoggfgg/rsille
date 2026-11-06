# Jellyfish / 深海水母

一个打开就可以静静看的终端水族箱。两只放大的发光水母上下游动、左右漂移，伞体有节奏地明显收缩，
触须随水流大幅摆动，半透明伞体里透出发光的内部结构。青蓝与珊瑚粉的水母之间，
有缓慢移动的水下光束、浮游微粒和远处的小鱼群。

```sh
cargo run --release --manifest-path examples/jellyfish/Cargo.toml
```

![实际终端输出，12 秒动画片段](preview.gif)

启动即自动播放，没有任务、按钮或操作面板。按 `q`、Esc 或 Ctrl+C 退出并恢复终端。
默认使用 **inline 模式**，在当前终端输出位置播放，最多占 **28 行**，不切换全屏。
推荐支持 True Color 和 Unicode 半块字符的终端，画面 **112×28**；**80×24** 也可运行，
宽度跟随窗口，高度不足 28 行时自动适应。通过减少水母数量、放大主体和缩短触须，
为更明显的上下游动留出空间，尽量保留伞体与触须细节。播放限制为 30 FPS，GIF 是实际终端帧以 10 FPS 采样的 12 秒片段，
放大后便于查看；预览回到开头时会跳转，实际播放持续推进。

## 实现

- `src/sea.rs`：程序化海水背景、柔和发光线段、半透明伞体、触须和鱼群；无需图片或模型素材。
- 水母各自拥有不同的呼吸相位，触须使用延迟波动；位置直接由时间计算，不依赖帧数或随机状态。
- `src/main.rs`：2×2 超采样，再交给公开的 `Canvas` Half-block 编码；
  `render::Builder` 管理帧循环、窗口变化和终端恢复。inline 画面无文字覆盖，不捕获鼠标。
- `src/export.rs`：从真实 `Draw` 输出的字符 Buffer 还原半块像素，供预览和性能检查使用。

这是为观赏设计的运动与光照效果，不是流体或生物物理模拟。

## 验证与导出

```sh
cargo test --manifest-path examples/jellyfish/Cargo.toml
cargo clippy --manifest-path examples/jellyfish/Cargo.toml --all-targets -- -D warnings
cargo run --release --manifest-path examples/jellyfish/Cargo.toml -- --headless
cargo run --release --manifest-path examples/jellyfish/Cargo.toml -- --snapshot /tmp/jellyfish.ppm 8
cargo run --release --manifest-path examples/jellyfish/Cargo.toml -- --frames /tmp/jellyfish-frames
```

测试覆盖无输入动画、同一时刻的确定性输出、长时间播放、窄屏和窗口尺寸变化，以及退出键。
`--headless` 检查 90 个时间点，并报告绘制与 Canvas 编码的平均耗时，不含终端 I/O。
`--snapshot FILE.ppm [SECONDS]` 导出 112×28 字符对应的 112×56 像素图，默认第 8 秒；
`--frames DIRECTORY` 导出从第 0 秒开始的 120 帧，采样率 10 FPS。PPM 文件可用常见图像工具转换。
