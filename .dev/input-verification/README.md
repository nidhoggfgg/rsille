# PTY 输入验证记录

日期：2026-10-23。平台：macOS 27.0.1 arm64。输入库：crossterm 0.28.1。
这是 Unix PTY 协议模拟验证，不是真实终端的物理按键验收。

复现：`python3 scripts/check-terminal-input.py --output-dir /tmp/rsille-input-pty`。
脚本默认构建 debug Minecraft，验证增强协议、传统协议及查询无响应三种环境。
这里仅保留解码后的事件日志；脚本还会在指定输出目录生成 ANSI 输出。

- `enhanced.log`：W / 右方向键的 Press、Repeat、Release，焦点、粘贴及 Q 松开 / 按下。
- `legacy.log`：能力检测不支持时的传统按键路径、焦点、粘贴及退出。
- `timeout.log`：能力查询无响应时降级，随后正常处理传统输入并退出。

三种环境均通过退出码、焦点与粘贴模式关闭、光标恢复、备用屏幕退出和原始 termios 恢复检查。
增强模式还检查键盘模式在离开同一备用屏幕之前弹出。
