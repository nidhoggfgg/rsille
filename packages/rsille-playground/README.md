# rsille Playground

在交互终端运行 `rsille` 打开菜单，也可以用子命令直接启动内置模式。

| 命令 | 功能 |
| --- | --- |
| [`rsille draw`](function-plotter/README_cn.md) | 打开函数绘图器，支持 Kitty 和 Sixel。`draw` 是 `function-plotter` 的别名。 |
| [`rsille pet`](deepseek-pet/README.md) | 在 Unix 交互终端启动 DeepSeek 宠物。 |
| [`rsille ai-babies`](ai-babies/README_cn.md) | 显示三位像素角色，加 `--animate` 播放动画。 |
| [`rsille jellyfish`](jellyfish/README_cn.md) | 在终端观看水母游动。 |

## 安装

发布到 crates.io 后可按包名安装，可执行文件名为 `rsille`。

```sh
cargo install rsille-playground
rsille --help
```

## 从源码运行

在仓库根目录运行以下命令。省略子命令时打开菜单。

```sh
cargo run -p rsille-playground
cargo run -p rsille-playground -- draw
cargo run -p rsille-playground -- pet --help
```

从当前工作区安装可使用 `cargo install --path packages/rsille-playground`。

## 发布

Playground 独立发版。确认依赖的 `rsille` 和 `rsille-typst` 版本已发布到 crates.io，将本包的版本号和 `releases/rsille-playground/<版本号>.md` 合入 `main`，然后在该提交上打 `rsille-playground-v<版本号>` tag 并推送。例如，`0.1.0` 对应 `rsille-playground-v0.1.0`。
