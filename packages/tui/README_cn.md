# Internal tui crate / 内部 tui crate

[English](README.md) · 中文

应用、可组合组件、布局、焦点、指针手势、动画和后台任务。
同一个运行时支持终端交互与无终端逐步执行。

这是内部 workspace 目标（`publish = false`）。应用应依赖 `rsille` 并导入
`rsille::tui`。[src/lib.rs](src/lib.rs) 中的适配入口编译
[src/tui](../../src/tui) 下的共用实现，不维护重复源码。

在仓库根目录执行：

```bash
cargo check -p tui
cargo test -p tui
```

可运行的组件和运行时示例见[小示例指南](examples/README_cn.md)。

[项目首页](../../README_cn.md) · [迁移指南](../../MIGRATION_V3.md)
