# TUI showcase

一个使用公开 `rsille::tui` API 编写的综合组件展厅。所有页面共用同一个
`App → update → view`，可以直接参考源码中的组件组合方式。

从仓库根目录运行：

```bash
cargo run --manifest-path examples/tui-showcase/Cargo.toml
```

默认全屏；加 `--inline` 可在普通终端屏幕中运行。建议终端至少为 **104 × 44**。
较小窗口会显示页面滚动条：Tab 聚焦页面视口后用方向键滚动，也可以使用鼠标滚轮。
页面内容区至少为 100 × 35，宽窗口中内容随可用空间展开。

| 页面 | 展示内容 |
| --- | --- |
| Overview | 导航菜单、四种进度条、加载指示器、弹窗入口和操作提示 |
| Controls | Unicode 单行/多行编辑、复选框、开关、切换按钮、单选组、六种按钮状态/样式、可搜索选择器、日历 |
| Data | 表格筛选、排序、列显隐、单元格导航、多选，以及树、列表、文件浏览器 |
| Content | Rust 代码高亮、Markdown、diff、包含真实交互记录的日志 |
| Layout | Flex、Grid 跨列、可调整分栏、嵌套滚动、折叠区、Stack 和 Overlay |
| Motion | 动态盲文画布、淡入、渐变、擦入、布局尺寸动画、播放/暂停 |
| Themes | 五套内置主题、语义状态色、排版、Unicode、边框和不同层级的表面 |

操作方式：

- `Tab` / `Shift+Tab`：移动焦点；导航标签获得焦点时，用左右键切换页面。
- `F1`：搜索命令或页面，`Enter` 执行；也可点击顶部 Commands 按钮。
- `F2`：循环切换五套主题；顶部主题按钮提供相同操作。
- `F3`：打开对话框。焦点限制在弹窗中，关闭后返回顶部主题按钮；表单值保留。
- `Esc`：先关闭当前下拉框、命令面板或对话框；主页面中退出。`Ctrl+C` 也可退出。
- 数据表格：方向键移动，`Space` 多选，`Enter` 提交；筛选框、排序按钮和 Owner 复选框控制表格。
- 日历：方向键按天/周移动，`PageUp` / `PageDown` 切换月份，`Enter` 提交日期。
- 分栏：拖拽分隔条，或聚焦分隔条后使用方向键调整。

Controls 页的笔记编辑演示 **350 ms 防抖 → 后台任务 → 结果消息**，仅保存到内存；
离开页面后任务仍能完成，返回时保留文本。版本号用于忽略过期保存结果。
表格、文件浏览器、进度和日志中的初始内容均为确定性的示例数据，文件浏览器不访问磁盘。
Motion 和 Overview 的播放开关共用状态。

无需 TTY 的交互验证：

```bash
cargo run --manifest-path examples/tui-showcase/Cargo.toml -- --headless
cargo test --manifest-path examples/tui-showcase/Cargo.toml
cargo clippy --manifest-path examples/tui-showcase/Cargo.toml --all-targets -- -D warnings
```

验证复用正式的状态、更新函数和视图，通过手动时钟/执行器驱动页面和主题切换、
Unicode 粘贴、防抖重置、过期保存结果、表格筛选/排序/多选、日历、弹窗焦点约束、
命令搜索、播放暂停和窗口缩放。弹窗背景只绘制预览，不参与输入；关闭后组件局部
状态（如光标和滚动位置）重新初始化，应用控制的表单值保持不变。

源码入口：[main.rs](src/main.rs) 管理应用、消息、任务和弹窗；
[pages.rs](src/pages.rs) 按页面组织所有组件组合；
[verify.rs](src/verify.rs) 演示如何在无终端环境中验证真实交互。
