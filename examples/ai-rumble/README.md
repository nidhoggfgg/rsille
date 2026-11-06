# AI Rumble / AI 娘大乱斗

DeepSeek 鲸鱼娘、Claude 抱书少女、ChatGPT 白龙娘的 Q 版自动战斗动画。
圆脸、大眼睛、眨眼和笑脸搭配各自的服装与配饰；蹲下蓄力、弹跳扑击、绕场飞行、
互相追逐和夸张回弹连续串成一段打闹。

```sh
cargo run --release --manifest-path examples/ai-rumble/Cargo.toml
```

![实际 rsille 终端帧，完整 24 秒循环](preview.gif)

默认 **inline 模式，最多 32 行**，宽度随终端调整。建议 **128×32** 或更大窗口，
**80×24** 也可播放。使用 True Color 与 Unicode Half-block；启动即播放，无需操作，
按 `q`、Esc 或 Ctrl+C 退出。退出后保留最后一帧，光标回到下一行。

## 一轮动画

每轮 24 秒，动作彼此衔接，自动循环：

- **DeepSeek**：深蓝女仆裙、荷叶头箍、白围裙小鲸鱼、蓝色侧鳍与宽尾鳍。先蹲扁蓄力，
  再拉长弹出，甩出一只小鲸鱼；被反击时身体先歪过去，长发和尾巴随后跟上。
- **Claude**：橙色长卷发、放射花饰、黑领结和腰封、米色外袍及棕色书本。
  被撞后回弹举书，摆臂甩出沿弧线飞行的书页，袖口、书本与手臂一起运动。
- **ChatGPT**：白色长发、弯曲分节龙角、浅紫龙翼、弯尾及结形饰物。
  拍翼腾空绕过场地，用弯曲吐息回击，再转身飞回。
- **追逐与收尾**：三人沿错开的弧线穿插换位，三色招式汇聚后一起弹开；
  缓过神后眨眼、笑着挥手，冒出爱心，再自然接回开头。

身体会压扁、拉长、回弹；头部轻微反向摆动，头发、裙摆、尾巴使用延迟运动。
眼睛会眨动，受击时眯眼，开心时露齿笑或眨一只眼。名字固定在底部，上方留给动作。
没有操作、胜负判定或模型能力排名，是一段不追求真实物理的卡通动画。

## 角色参考与署名

造型参考 [ZipZipPipe 的《所有的 AI 们，和 OpenCode》原帖](https://www.bilibili.com/opus/1233069202138464289)，
并通过 [AI 娘档案](https://wulusai.net/AI/AI%E5%A8%98%E6%A1%A3%E6%A1%88)
及 [角色对照页](https://genderswapai.com/zh-CN/ai-gijinka/brand-to-character) 查看立绘。

- **DeepSeek**：原角色「溟月」由**上善无形**创作，鲸鱼女仆版由 **ZipZipPipe** 二次设计。
- **Claude / ChatGPT**：橙发抱书少女、白发龙娘参考 **ZipZipPipe** 的同人设计。

本示例以程序绘制简化 Q 版，不包含网上立绘原文件；动作、舞台和特效为本示例编写。
角色均为社区同人形象，并非厂商官方形象。DeepSeek 角色改编与对应预览遵循参考来源标注的
[CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/)；
其余参考角色的权利仍归原作者，本示例按署名、非商业同人展示处理。
仓库代码许可不代表对参考角色或商标授予新的授权。

## 实现与验证

`motion.rs` 使用首尾周期衔接的三次 Hermite 曲线驱动位置、倾斜、压缩、双臂和表情；
`characters.rs` 分别绘制三套角色，头发与尾巴采样稍早的姿态获得跟随效果。
`scene.rs` 组合场景、景深排序和招式；`raster.rs` 提供曲线轮廓、贝塞尔描边、椭圆、星芒和像素字。图形经过 2×2 超采样，再由公开的 rsille Canvas 编码为 Half-block。
帧循环、inline 输出、窗口调整和终端恢复由 `render::Builder` 管理。

```sh
cargo test --manifest-path examples/ai-rumble/Cargo.toml
cargo clippy --manifest-path examples/ai-rumble/Cargo.toml --all-targets -- -D warnings
cargo run --release --manifest-path examples/ai-rumble/Cargo.toml -- --headless
cargo run --release --manifest-path examples/ai-rumble/Cargo.toml -- --snapshot /tmp/ai-rumble.ppm 6
cargo run --release --manifest-path examples/ai-rumble/Cargo.toml -- --frames /tmp/ai-rumble-frames
```

测试覆盖三人主动出招与受击、逐帧姿态连续性、循环处速度衔接、帧确定性、窗口尺寸变化、长时间运行和退出。
`--headless` 渲染 90 个时间点，报告绘制及编码耗时（不含终端 I/O）。
导出读取实际终端 Buffer；`--snapshot` 生成 128×64 像素的 PPM，
`--frames` 以 25 FPS 导出完整一轮的 600 帧。GIF 使用这些帧制作，实际播放上限为 60 FPS。
