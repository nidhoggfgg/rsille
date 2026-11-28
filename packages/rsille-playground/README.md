# rsille Playground

在交互终端中直接运行 `rsille` 启动内联交互式展台（实时 30 FPS 预览、终端图形协议自动探测并可一键进入体验）；在非 TTY / 管道环境中运行则输出 `--help`。也可使用子命令直接启动对应模式。

| 命令 | 功能 | 终端要求 |
| --- | --- | --- |
| `rsille voxel`（别名 `craft`） | 第一人称 3D 方块沙盒：探索地形、木屋与湖泊，挖掘搭建、飞行、昼夜切换。 | 任意真彩色终端 |
| `rsille clockwork` | 132 个零件拆解重组为步行机、扑翼机与机械花，支持磁力震散、爆炸视图与拖拽视角。 | 任意真彩色终端 |
| `rsille desk`（别名 `lumen`） | 桌面式 TUI 工作台：可拖拽缩放浮窗、Dock、动态曲线、视觉特效、命令面板与多主题。 | 任意真彩色终端 |
| `rsille hud` | 浮在真实 shell 上方的实用工具栏：可拖拽目录树（一键 `cd here`）、命令耗时徽章、待办清单与快捷宏。 | 任意终端（PTY / ConPTY） |
| [`rsille pet`](deepseek-pet/README.md) | 浮在真实 shell（或 `vim` 等程序）上方的 DeepSeek 小鲸鱼宠物，响应命令输入与鼠标互动。 | Kitty / Sixel |
| [`rsille draw`](function-plotter/README_cn.md) | 内联 2D/3D 函数绘图 REPL，支持极坐标、参数方程、旋转 3D 曲面与 `:grid` 多图布局。`draw` 是 `function-plotter` 的别名。 | Kitty / Sixel |
| [`rsille jellyfish`](jellyfish/README_cn.md) | 深海发光水母水族箱，使用 Unicode 半块字符流畅播放。 | 任意真彩色终端 |
| `rsille doctor` | 探测终端 Kitty / Sixel / 单元格像素能力，并同屏四路对比 Braille、HalfBlock、Quadrant 与原生像素渲染（支持 `--plain` 纯文本输出）。 | 任意终端 |

*原 `ai-babies` 演示已移至仓库独立示例 [`examples/ai-babies`](../../examples/ai-babies/README_cn.md)，以保持 Playground 安装包更轻量。*

## 安装

### 下载可执行文件（无需 Rust 或 Cargo）

可直接前往 **[rsille 下载页](https://rsille.vercel.app/download)** 选择对应系统下载（所有文件均经 SHA-256 校验镜像），或在 [Playground 发布列表](https://github.com/nidhoggfgg/rsille/releases?q=rsille-playground-v) 打开最新的 `rsille-playground-v…` 版本，在 **Assets** 中按下表选择下载文件。可执行文件从 `0.1.2` 之后的版本开始提供。

| 系统 | 处理器 | 下载文件 |
| --- | --- | --- |
| Linux（含 WSL） | Intel / AMD 64 位 | `rsille-linux-x64` |
| Linux（含 WSL） | ARM64 | `rsille-linux-arm64` |
| macOS | Apple 芯片 | `rsille-macos-apple-silicon` |
| macOS | Intel 芯片 | `rsille-macos-intel` |
| Windows | Intel / AMD 64 位 | `rsille.exe` |

**Linux / macOS** 可以用 `uname -m` 查看处理器架构。在下载目录运行以下命令。第一行以 Linux x64 为例，其他平台请换成上表对应的文件名。

```sh
mv rsille-linux-x64 rsille
chmod +x rsille
./rsille --help
```

要在任意目录运行 `rsille`，可以把它安装到自己的命令目录：

```sh
mkdir -p "$HOME/.local/bin"
install -m 755 rsille "$HOME/.local/bin/rsille"
```

如果该目录还未加入 `PATH`，在 shell 配置文件中加入 `export PATH="$HOME/.local/bin:$PATH"`，然后重新打开终端。

**Windows** 在下载目录打开 PowerShell，运行 `.\rsille.exe --help`。要在任意目录运行，把 `rsille.exe` 所在目录加入用户的 `Path`，重新打开终端即可。在 Windows Terminal 1.22 或更新版本中运行 `.\rsille.exe pet` 即可启动宠物。

### 通过 Cargo 安装

```sh
cargo install rsille-playground --locked
rsille --help
```

## 从源码运行

在仓库根目录运行以下命令。

```sh
cargo run -p rsille-playground
cargo run -p rsille-playground -- draw
cargo run -p rsille-playground -- pet --help
```

从当前工作区安装可使用 `cargo install --path packages/rsille-playground`。

## 发布

Playground 独立发版。确认依赖的 `rsille` 版本已发布到 crates.io，将本包的版本号和 `releases/rsille-playground/<版本号>.md` 合入 `main`，然后在该提交上打 `rsille-playground-v<版本号>` tag 并推送。工作流从已发布的 crate 构建各平台的可执行文件，运行 `rsille --help` 和 `rsille draw --headless` 后，将可执行文件、许可证和 `SHA256SUMS-rsille-playground` 校验文件附到该 tag 的 GitHub Release。

### Vercel 项目主页与下载页

将本仓库导入 Vercel，项目根目录选仓库根目录，生产分支选 `main`。仓库中的 `vercel.json` 已配置零框架静态构建。部署后，根路径 `/` 展示 `rsille` 项目主页（含中英双语切换与下载入口），`/download` 提供独立的 Playground 直达下载页。可以把主页或下载页固定地址放在仓库的 Website 栏或分享给用户。

在 Vercel 项目的 **Settings → Git → Deploy Hooks** 为 `main` 创建一个 hook，将完整 URL 存入 GitHub 仓库的 Actions secret `PLAYGROUND_VERCEL_DEPLOY_HOOK`。每次 Playground 正式发版完成后，GitHub Actions 会触发重新构建；建议同时在 Vercel 环境变量里配置只读 `GITHUB_TOKEN`，避免 GitHub API 匿名限流。

构建会从 GitHub 读取最新稳定版 Playground 的五个可执行文件和许可证，按该 Release 的 SHA-256 清单核对后写入静态站点。用户从 Vercel 直接下载。首个含二进制的版本发布前，页面显示发布状态；下载或校验失败会中止构建。

在仓库根目录运行 `node web/build.mjs`（Node.js 22 或更新版本；本地离线调试样式可加 `--offline`），再运行 `python3 -m http.server --directory target/site`，可在本机打开 `http://localhost:8000/` 和 `http://localhost:8000/download/` 预览。
