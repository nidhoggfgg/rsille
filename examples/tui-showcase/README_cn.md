# TUI showcase

[English](README.md) · 中文

公开 `rsille::tui` API 的七页组件展厅。在同一个应用中体验表单、数据组件、内容查看器、
布局、动画和主题。

## 运行

在仓库根目录执行：

```bash
cargo run --manifest-path examples/tui-showcase/Cargo.toml
cargo run --manifest-path examples/tui-showcase/Cargo.toml -- --inline
```

默认全屏，建议使用 104 × 44 的终端；小窗口可滚动。本项目已在依赖声明中开启语法高亮。

| 页面 | 内容 |
| --- | --- |
| Overview | 导航、进度条变体、加载指示器与弹窗 |
| Controls | Unicode 编辑器、复选框、开关、切换按钮、单选组、按钮、选择器和日历 |
| Data | 表格筛选与排序、列显隐、多选、树、列表和文件浏览器 |
| Content | Rust 高亮、Markdown、diff 和交互日志 |
| Layout | Flex、Grid、可调分栏、滚动、折叠区、Stack 和 Overlay |
| Motion | 动态盲文画布、视觉效果、布局过渡和播放控制 |
| Themes | 五套内置主题、语义颜色、排版和边框 |

## 操作

| 按键 | 操作 |
| --- | --- |
| Tab / Shift+Tab | 切换焦点；导航获得焦点时用方向键切页 |
| F1 / F2 / F3 | 命令面板 / 下一套主题 / 对话框 |
| Esc | 先关闭弹出控件，在主页面退出 |
| Ctrl+C | 退出 |
| 表格中的方向键 / 空格 / Enter | 导航 / 选择 / 提交 |
| 日历中的方向键 / PageUp / PageDown / Enter | 移动日期 / 上月 / 下月 / 提交 |

拖动分隔条或聚焦后用方向键调整分栏。使用滚轮滚动，或聚焦页面视口后按方向键。
Controls 页的笔记演示 350 ms 防抖、后台保存和过期结果保护。保存只写内存；
文件浏览器和表格初始数据均为预设内容，不读取磁盘。

## 验证

```bash
cargo run --manifest-path examples/tui-showcase/Cargo.toml -- --headless
cargo test --manifest-path examples/tui-showcase/Cargo.toml
cargo clippy --manifest-path examples/tui-showcase/Cargo.toml --all-targets -- -D warnings
```

无终端检查使用手动时钟和执行器验证页面、主题、编辑、防抖、表格、日历、模态焦点和窗口调整。
应用控制的表单值在弹窗切换后保留；重新挂载的组件会初始化局部光标和滚动状态。

[main.rs](src/main.rs) 定义应用，[pages.rs](src/pages.rs) 包含组件组合，
[verify.rs](src/verify.rs) 演示无 TTY 输入回放。

[全部示例](../README_cn.md)
