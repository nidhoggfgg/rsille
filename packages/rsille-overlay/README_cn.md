# rsille-overlay

[English](README.md) · 中文

在正在使用的 shell 上作画，而不是替换它。

`rsille-overlay` 在 PTY 中运行你的 shell（或任意程序），原样转发它的输出。两批输出之间，只要没有未结束的控制序列、图像上传或同步更新，就让叠加层在上面绘制。下一批可能干扰它的输出到来之前，叠加层先隐藏，被遮住的文字随即补回。

shell 的活动会以事件形式报告，叠加层可以据此做出反应：

| 事件 | 时机 |
| --- | --- |
| `Prompt` | 显示新的提示符 |
| `Edit(line)` | 正在编辑的命令行有变化（zsh） |
| `Exec(line)` | 命令即将运行 |
| `Done(status)` | 该命令结束 |
| `Read` | 运行中的命令通过 shell 读取一行输入（fish） |

事件来自只为本次会话安装的 bash、fish 和 zsh 钩子，用户自己的启动文件仍会先运行。bash 从历史记录读取命令行，因此被历史记录略过的行（例如设置了 `HISTCONTROL=ignorespace`）会报告成上一条记录。其他 shell 也能在会话中运行，只是不报告事件。

会话启动时和每次调整窗口大小后都会清屏。会话无法读取终端上原有的内容，终端在调整大小时重排文字的方式也与会话保存的副本不同，清屏才能保证补回的文字正确。想保留回滚记录，可以使用 `alt_screen(true)`。

## 在 shell 上放一个按钮

`WidgetOverlay` 把普通的 rsille-tui 应用放进一个小方框。方框内的鼠标点击会交给其中的组件，键盘始终留给 shell。

```rust,no_run
use rsille_overlay::{Anchor, Session, ShellEvent, WidgetOverlay};
use rsille_tui::prelude::*;

let app = App::new(String::new())
    .update(|last: &mut String, msg: Option<String>, _ctx| *last = msg.unwrap_or_default())
    .view(|last: &String| {
        row::<Option<String>>()
            .child(label(last.clone()).width(20))
            .child(button("×").on_click(|| None))
    })
    .build();
let overlay = WidgetOverlay::new(app, (24, 1))
    .anchor(Anchor::TopRight)
    .on_shell(|event| match event {
        ShellEvent::Exec(line) => Some(Some(line.to_owned())),
        _ => None,
    });
Session::new().mouse(true).run(overlay)?;
# Ok::<(), std::io::Error>(())
```

在 bash、fish 或 zsh 中运行示例：

```sh
cargo run -p rsille-overlay --example status_badge     # 上一条命令的结果
cargo run -p rsille-overlay --example click_counter    # shell 上的两个方框
cargo run -p rsille-overlay --example click_counter -- top
cargo run -p rsille-overlay --example yes_engineer     # 点击输入 "yes, continue" 并回车
cargo run -p rsille-overlay --example todolist
cargo run -p rsille-overlay --example folder_tree
```

待办清单内置三条英文任务。点击任务可勾选或取消勾选，点击 **Clear done** 可清除本次会话中已完成的任务。

目录树从启动示例时的工作目录开始，点击文件夹即可展开或收起，条目较多时可用滚轮浏览。在空白 shell 提示符下，选中目录并点击 **cd here**，当前 shell 就会切换到该目录。拖动标题可以移动面板。

目录树通过 `WidgetOverlay::primary_screen_only()` 只在主屏幕显示。子程序进入备用屏幕时，目录树隐藏并停止接收点击。回到主屏幕后恢复原来的位置和展开状态。这个设置在 Kitty 图形会话中同样生效。

随时按 **Ctrl+]** 退出会话，也可以在 shell 中输入 `exit`。

## 自定义叠加层

实现 `Overlay` 即可自己输出字节，包括 Kitty 或 Sixel 图像。这个 trait 只有 `draw` 和 `hide` 两个必需方法，其余（指针命中、输入、输出、铃声、shell 事件、窗口大小变化）都可选。`Stack` 可以组合多个叠加层，按下操作交给最上层认领它的那一个。

```rust,no_run
use rsille_overlay::{Context, Overlay, Session};
use rsille_render::area::Area;
use std::io::{self, Write};

struct Clock(Option<Area>);

impl Overlay for Clock {
    fn draw(&mut self, cx: &mut Context<'_>, out: &mut Vec<u8>) -> io::Result<()> {
        let col = cx.cols.saturating_sub(10);
        write!(out, "\x1b[1;{}H\x1b[7m {:>6.1}s \x1b[0m", col + 1, cx.elapsed().as_secs_f32())?;
        self.0 = Some(Area::new((col, 0).into(), (9, 1).into()));
        Ok(())
    }
    fn hide(&mut self, cx: &mut Context<'_>, out: &mut Vec<u8>) {
        if let Some(area) = self.0.take() {
            cx.erase(area, out); // 补回时钟下面的 shell 文字。
        }
    }
}

Session::new().run(Clock(None))?;
# Ok::<(), std::io::Error>(())
```

## 图形与鼠标

| | 文本（默认） | Sixel | Kitty |
| --- | --- | --- | --- |
| 启用方式 | — | `.graphics(RasterBackend::Sixel)` | `.graphics(RasterBackend::Kitty)` |
| 子程序全屏时（vim、top） | 隐藏 | 隐藏 | 保持可见 |
| `.mouse(true)` 时的鼠标 | 子程序未开启鼠标跟踪时交给叠加层 | 同文本 | 共享：叠加层认领的按下归叠加层，其余按子程序自己的鼠标模式转发 |

rsille-playground 中的 `rsille pet` 就构建在这个 crate 之上。

Windows 10 1809 及以上版本使用 ConPTY。在 Windows Terminal 中可以这样给原生 PowerShell 加上计数挂件：

```powershell
cargo run -p rsille-overlay --example click_counter -- powershell.exe -NoLogo
```

Windows 默认启动 `%COMSPEC%`（通常是 `cmd.exe`）。命令执行和退出状态等 shell 事件由 Unix 上的 bash、fish、zsh 钩子提供。在 Windows 上，可见挂件通过 `Overlay::hit` 认领按下事件，其余点击按字符格坐标交给 ConPTY，再由子程序的输入模式决定如何处理。Windows Terminal 1.22 及以上版本支持 Sixel。更改字体或 DPI 后，重新启动图形会话即可重新测量字符格尺寸。
