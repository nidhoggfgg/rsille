# Internal canvas crate / 内部 canvas crate

[English](README.md) · 中文

彩色采样图形，支持 Braille（2 × 4）、Half-block（1 × 2）和 Quadrant（2 × 2）编码。
绘图坐标以采样点为单位，y 轴向上；边界与输出尺寸以终端字符格为单位。

这是内部 workspace 目标（`publish = false`）。应用应依赖 `rsille` 并导入
`rsille::canvas`。[src/lib.rs](src/lib.rs) 中的适配入口编译
[src/canvas](../../src/canvas) 下的共用实现，不维护重复源码。

在仓库根目录执行：

```bash
cargo check -p canvas
cargo test -p canvas
```

[项目首页](../../README_cn.md)
