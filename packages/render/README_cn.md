# Internal render crate / 内部 render crate

[English](README.md) · 中文

终端 Buffer、带样式的 Unicode 文本、局部区域绘制和终端事件循环。
坐标和尺寸以终端列、行为单位。

这是内部 workspace 目标（`publish = false`）。应用应依赖 `rsille` 并导入
`rsille::render`。[src/lib.rs](src/lib.rs) 中的适配入口编译
[src/render](../../src/render) 下的共用实现，不维护重复源码。

在仓库根目录执行：

```bash
cargo check -p render
cargo test -p render
```

[项目首页](../../README_cn.md)
