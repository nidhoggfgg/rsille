# rsille Playground

`rsille` 是一个交互式终端游乐场。不带参数运行即可打开菜单，启动仓库 `examples/` 下的示例：`function-plotter`（别名 `draw`）、`ai-babies`（别名 `pet`）或 `jellyfish`。绘图需要支持 Sixel 的终端。

## 从源码运行

在仓库根目录的交互终端运行：

```sh
cargo run -p rsille-playground
```

也可以跳过菜单，见[直接启动示例](examples/README.md)。

## 安装并查看帮助

```sh
cargo install --path packages/rsille-playground
rsille --help
```

安装后的可执行文件名为 `rsille`。运行 `rsille` 打开菜单，或运行 `rsille draw`、`rsille pet` 直接进入对应模式。使用 `rsille draw --help` 或 `rsille pet --help` 查看各命令的用法。
