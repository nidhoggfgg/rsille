# 多键按住输入：实现与验收记录

记录日期：2026-10-23。状态：已实现，自动化验证通过；真实终端按键验收待补。

## 现象与复现

在 Minecraft 示例中，长按 WASD 移动时再按住方向键，无法可靠地同时持续移动与转头。
用户报告后按住的键看起来没有效果。具体哪一个键持续收到重复事件，需要捕获实际输入
才能确定；尚未记录用户的终端类型和原始事件日志。

```bash
cargo run --release --manifest-path examples/minecraft/Cargo.toml
```

1. 持续按住 `W`，再同时按住 `→`，观察能否一边前进一边转头。
2. 交换按键顺序，再尝试 `W` + `D`、`W` + `↑`。
3. 松开其中一个键，检查另一个键能否继续；全部松开后应停止。
4. 按住键时打开手册、暂停或切换窗口，再恢复，检查有无卡键或残留运动。

期望：各键的按住状态独立维护，移动与转向由每帧时间统一计算；重复事件频率不影响速度，
松开后停止对应动作，失焦和进入菜单时清空状态。

## 修复前实现与问题归属

| 层级 | 当前情况 |
| --- | --- |
| 终端输入 | 传统终端输入通常没有可靠的松开事件。支持增强键盘协议的终端可以报告按下、重复和松开；须协商启用，并确认当前平台实际支持。 |
| rsille 终端生命周期 | `src/render/event_loop.rs` 的 `TerminalGuard` 启用了 raw mode、粘贴和鼠标，但没有协商、启用或恢复增强键盘模式。 |
| rsille 事件与快捷键 | 底层 crossterm 0.28.1 有键盘增强和 `KeyEventKind` 能力，原始事件可进入组件；`App::on_key` 只接收键码。`src/tui/runtime/core.rs` 的全局快捷键、Tab 导航与退出处理没有统一排除 Release，直接启用松开上报可能导致松手再次触发操作。还需审查命令路由、快捷键注册表及各组件。 |
| Minecraft 示例 | `examples/minecraft/src/main.rs` 使用 `on_key` 发送移动与转向消息，没有记录按下/松开集合。`world.rs` 的 `KeyboardMotion` 只平滑每次事件产生的有限位移，不代表键仍被按住。 |

这不是只在示例里打开一个现有开关即可完成的功能。框架需要接入终端能力与事件语义，
示例再维护按键状态；不支持松开事件的终端需要明确的降级行为。

### 前期缓解

仅在 demo 内将 WASD、方向键和飞行升降改为逐帧消耗有上限的待执行动作，
减轻按键重复造成的位置、角度跳变。暂停、手册、切换飞行和回到出生点会清空剩余动作；
鼠标中键拖动仍直接响应，并清除尚未完成的键盘转向。

已有测试覆盖 30 / 60 / 120 FPS 下的平滑行为、重复事件间持续运动、积压上限、
鼠标接管及暂停后清空。**这些测试不表示多键按住已修复。**
传统按键重复的首次延迟、重复频率影响速度、多键持续状态和准确松手停止仍待解决。

## 修复范围

1. 在框架配置中提供可选的增强键盘模式，检测终端能力后启用。
   检查 crossterm 的 `PushKeyboardEnhancementFlags` / `PopKeyboardEnhancementFlags`，
   包括事件类型上报，以及普通字符键完整上报所需的标志组合。
   在终端 guard 中处理正常退出、初始化中途失败和异常展开时的恢复。
2. 提供可保留 `KeyEvent.kind` 的输入入口，明确 Press / Repeat / Release 的语义。
   全面审查快捷键、按钮、Tab、退出等现有路径，避免 Release 再触发点击、切换或退出。
   原始事件不应因焦点、事件冒泡或全局快捷键处理而遗漏必要的松开状态更新。
3. demo 维护按住的键集合：Press 加入，Release 移除，Repeat 不叠加速度。
   每帧按秒计算移动和旋转，同时组合 WASD 与方向键；斜向移动归一化，反向键相互抵消。
   暂停、手册、失焦、返回出生点等状态切换需清空按键；确认终端焦点事件也已启用。
4. 保留传统终端的兼容路径，并说明能力限制。不要仅凭重复事件超时声称能准确识别松手，
   也不要假定所有终端、平台或 tmux / SSH 组合都支持增强协议。

鼠标回中、系统级鼠标控制不在本问题范围内；用户已决定暂不处理。

## 已完成的实现

- 框架默认保持 `KeyboardMode::Legacy`；`App` / `render::Builder` 可选 `Auto`。
  终端 guard 已独立到 `src/render/terminal.rs`，在 raw mode 后、创建 `EventStream` 前
  使用 crossterm 检测协议支持，成功时启用
  `DISAMBIGUATE_ESCAPE_CODES | REPORT_EVENT_TYPES | REPORT_ALL_KEYS_AS_ESCAPE_CODES`。
  检测失败或超时保留传统模式；Windows 使用 crossterm 原生按键松开事件。
  `focus_events(true)` 独立请求焦点上报。guard 在退出、初始化失败和异常展开时恢复状态，
  在离开备用屏幕之前弹出同屏幕的键盘模式。
- `App::on_input` 接收 `InputEvent::Capabilities` 和完整的 `InputEvent::Terminal`，
  在组件处理之前按顺序入队，即使事件随后被消费也不丢失按键松开和焦点消息。
  `on_key` 保留 Press / Repeat，Release 不激活快捷键、Tab、退出或内置组件；
  按钮、checkbox、switch、collapsible 只在 Press 时触发。
  自定义组件仍可收到原始 Release，自己决定如何处理。
- Minecraft 在 `src/input.rs` 维护物理按下集合和当前生效集合。
  Press 加入、Release 移除，Repeat 不新增动作；原生重复 Press 也会去重。
  WASD / 方向键 / 飞行升降按秒组合，平面斜向归一化，反向抵消。
  按键字符统一大小写，松开时不依赖原来的修饰键组合；Ctrl / Alt 等组合不会误移动。
- 暂停、手册、失焦、切换飞行、回到出生点均清空动作，松开和焦点消息在非活动状态仍处理。
  恢复后需要松开再按下相关键，避免重复事件重新启动运动。
  鼠标转向取消当前键盘转向，仍可保持键盘移动。
- 传统模式保留原有有上限的动作平滑，启动提示和手册说明其多键、重复和松手限制。
  `--input-log FILE` 可记录能力和原始事件；日志写入失败会正常退出并返回错误。

## 自动化验证记录

- Minecraft 共 16 个测试，包括新增 8 个增强输入测试和原有 headless 流程。
  覆盖组合与顺序、部分 / 全部松开、反向键、不同重复频率、30 / 60 / 120 FPS、
  修饰键变化、菜单 / 失焦恢复、鼠标接管及传统降级。
- 框架新增 4 个输入集成测试、4 个终端生命周期测试和 1 个底层退出测试。
  生命周期测试注入初始化和恢复失败，并验证异常展开及 pop / 离屏顺序。
- `scripts/check-terminal-input.py` 在 macOS 27.0.1 arm64 的 Unix PTY 中，
  模拟增强协议、传统协议及查询无响应三种环境，均通过协商、crossterm 解码、
  焦点、粘贴、退出和原始 termios 恢复检查。
  本机事件日志在 `.dev/input-verification/`。这不是支持协议的真实终端实测记录。
- 工作区全量测试通过；Minecraft 严格 Clippy 通过。工作区严格 Clippy 仍被既有告警阻止；
  普通 Clippy 已完成，与独立 HEAD 基线同为 140 条诊断，无新增告警。

## 真实终端验收入口

```bash
cargo run --release --manifest-path examples/minecraft/Cargo.toml -- --input-log /tmp/rsille-input.log
python3 scripts/check-terminal-input.py --output-dir /tmp/rsille-input-pty
```

真实终端仍需记录终端名称 / 版本、平台、是否经过 tmux / SSH，再执行下列验收。
能力检测和模式启用不能替代对终端实际按键事件的验证；不保证协议不完整或有缺陷的终端行为。

## 验收

- 注入 Press / Repeat / Release 的无 TTY 测试：`W` + `→`、`W` + `D`、交换按键顺序、
  部分松开、全部松开、反向键、不同帧率与不同重复频率。
- 检查 Release 不会激活按钮、再次触发开关、移动焦点或退出应用。
- 暂停、手册和失焦后不残留按键，恢复时不自动移动。
- 在支持协议的真实终端中验证边走边转头和松手停止；记录终端、版本、平台及事件日志。
- 在不支持协议的终端中验证可正常运行、退出后终端状态恢复，降级行为与说明一致。
- 保持现有 Minecraft 测试、headless 流程与 Clippy 通过，并补充框架输入相关回归测试。

参考：[Kitty 键盘协议](https://sw.kovidgoyal.net/kitty/keyboard-protocol/)。
