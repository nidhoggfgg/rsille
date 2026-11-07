# rsille showcase

[English](README.md) · 中文

一个应用中的四种终端界面：仓库工作台、模拟进程监视器、计算器和动态盲文图形。

## 运行

在仓库根目录执行，以便工作台找到预期文件：

```bash
cargo run --release --manifest-path examples/rsille-showcase/Cargo.toml
```

本项目依赖本地 `rsille`，并已启用语法高亮。

| 标签页 | 实际功能与模拟内容 |
| --- | --- |
| Workbench | 读取本地项目文件，展示源码预览、生成的演示 diff、笔记和日志 |
| Top | 筛选和选择模拟进程行；指标与时间曲线均为合成数据 |
| Calculator | 通过输入框或可点击键盘计算算术表达式，保留会话历史 |
| Showoff | 用盲文画布绘制粒子隧道、线框和信号场 |

工作台从当前目录下的 `Cargo.toml`、`README.md`、`src`、`packages`、`examples`
构建文件树。树的深度和条目数量有限，源码预览最多展示 120 行。
展示的 diff 不是 Git 工作区差异。

## 操作

Tab 切换焦点，方向键导航标签页、树、列表和表格，Enter 或空格激活控件。
点击 Cmd 打开命令面板，Esc 退出。通过界面控件切换场景、筛选数据和暂停动态显示。

## 检查与源码

```bash
cargo check --manifest-path examples/rsille-showcase/Cargo.toml --all-targets
```

本示例没有 `--headless` 或快照命令。状态、组件组合、计算器与绘图逻辑集中在
[src/main.rs](src/main.rs)。需要参考自动交互回放时，可查看
[Lumen](../lumen/README_cn.md) 或 [TUI showcase](../tui-showcase/README_cn.md)。

[全部示例](../README_cn.md)
