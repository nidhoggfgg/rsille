# Folded Harbor / 折叠星港

[English](README.md) · 中文

把视角变成机关的空间解谜：旋转悬浮建筑，让不同深度的两个通道口在屏幕中重合，
再折叠空间跨越间隙。共三章、九次折叠。

## 运行

在仓库根目录执行：

```bash
cargo run --release --manifest-path examples/folded-harbor/Cargo.toml
cargo run --release --manifest-path examples/folded-harbor/Cargo.toml -- --demo
```

默认全屏，`--inline` 使用普通终端屏幕。建议使用 True Color 和盲文字符支持，
推荐窗口 144 × 44，80 × 24 也可游玩。达到 116 × 34 时显示观察面板。
所有场景在运行时生成。

## 玩法

找到标有 A / DEPARTURE 和 B / ARRIVAL 的发光圆环，旋转视角让它们重合，保持约 0.4 秒。
圆环变白后按空格或点击 FOLD 按钮跨越。每完成三次折叠，进入下一章。

| 输入 | 操作 |
| --- | --- |
| 方向键 / w、a、s、d / 左键拖动 | 转动相机 |
| 滚轮 / + / - | 缩放 |
| 空格 / Enter | 折叠、进入下一章或通关后重开 |
| h / t | 辅助对齐 / 开关自动演示 |
| p / Esc | 暂停或继续，空格也可继续 |
| ? | 帮助，空格、Esc 或 ? 关闭 |
| r / q / Ctrl+C | 从第一章重开 / 退出 / 退出 |

手动旋转会接管自动演示。折叠期间锁定相机旋转，但可以缩放。暂停和帮助会冻结模拟。
对齐容差不受窗口大小与缩放影响，自动演示通关后循环播放。

## 验证与导出

```bash
cargo test --manifest-path examples/folded-harbor/Cargo.toml
cargo run --release --manifest-path examples/folded-harbor/Cargo.toml -- --headless
cargo run --release --manifest-path examples/folded-harbor/Cargo.toml -- --snapshot /tmp/harbor.svg --at 4.5
cargo run --release --manifest-path examples/folded-harbor/Cargo.toml -- --benchmark
```

`--snapshot FILE.svg --at SECONDS` 截取自动旅程的指定时刻（0–3600 秒）。
无终端检查覆盖完整通关、投影按钮、相机输入、焦点和窗口调整。
性能测量统计 144 × 44 下的 CPU 更新、布局和绘制时间，不包含终端 I/O。

[world.rs](src/world.rs) 实现投影和解谜规则，[scene.rs](src/scene.rs) 与
[raster.rs](src/raster.rs) 绘制世界，[main.rs](src/main.rs) 把普通 TUI 标签和按钮放到
三维投影位置，[verify.rs](src/verify.rs) 驱动验证。

[全部示例](../README_cn.md)
