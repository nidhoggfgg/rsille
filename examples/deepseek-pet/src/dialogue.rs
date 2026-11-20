//! The shell hooks send a category and phase; only the pet chooses the dialogue.
use std::collections::HashMap;

type Lines = [&'static str; 3];

// These categories describe the tool while its subcommand is still being edited.
pub const FAMILIES: &[(&[&str], u8, Lines)] = &[
    (
        &["git"],
        b'G',
        ["要看看仓库吗？", "这回想对仓库做什么？", "我也来逛逛仓库"],
    ),
    (
        &["cargo", "go"],
        b'B',
        ["今天要捣鼓哪个项目？", "让我看看你的工程", "我来给你打下手"],
    ),
    (
        &["npm", "pnpm", "yarn", "pip"],
        b'P',
        ["要用包管理器啦？", "工具箱打开啦", "这回要用哪个工具？"],
    ),
];

pub struct CommandGroup {
    pub code: u8,
    pub commands: &'static [&'static str],
    editing: Lines,
    running: Lines,
    done: Lines,
}

pub const GROUPS: &[CommandGroup] = &[
    CommandGroup {
        code: b'l',
        commands: &["ls", "tree", "eza"],
        editing: ["这儿藏着什么？", "让我也瞧一眼", "来数数文件？"],
        running: ["我挨个儿瞧瞧", "尾巴收好，别挡字", "让我探个头"],
        done: ["都在这儿啦", "这一眼看清啦", "有你要找的吗？"],
    },
    CommandGroup {
        code: b'p',
        commands: &["pwd"],
        editing: ["游到哪儿了？", "看看咱们的位置？", "让我认认路"],
        running: ["找找路标", "我看看地址", "地图摊开啦"],
        done: ["原来在这里", "记住这个路口啦", "位置找到了"],
    },
    CommandGroup {
        code: b'd',
        commands: &["cd", "pushd", "popd"],
        editing: ["要游去哪儿？", "这回往哪走？", "我跟你换个地方"],
        running: ["跟上你啦", "带上我！", "等等我的小尾巴"],
        done: ["到啦！", "新位置记住啦", "我也游过来了"],
    },
    CommandGroup {
        code: b's',
        commands: &["find", "fd", "rg", "grep"],
        editing: ["在找什么？", "我来当你的眼睛", "要找点线索？"],
        running: ["让我仔细瞅瞅", "我往这边找找", "水底也看看"],
        done: ["找完这一趟啦", "看看有没有线索？", "这片翻过啦"],
    },
    CommandGroup {
        code: b'f',
        commands: &["cat", "less", "head", "tail", "bat"],
        editing: ["想读哪一段？", "我也来旁听", "给我留个位置"],
        running: ["借我瞄一眼", "我陪你慢慢读", "我趴在边上看"],
        done: ["这段看完啦", "读到这儿啦", "我把位置让给你"],
    },
    CommandGroup {
        code: b'g',
        commands: &["git status", "git diff", "git log", "git show"],
        editing: ["看看改了什么？", "要翻翻仓库记录？", "我也看看仓库"],
        running: ["一起看看", "我在旁边对一眼", "让我凑近一点"],
        done: ["看完啦！", "仓库看过一遍啦", "有你关心的那条吗？"],
    },
    CommandGroup {
        code: b'v',
        commands: &["git add", "git commit", "git stash"],
        editing: ["要收好这些改动？", "这份工作要存起来？", "我看着这些改动"],
        running: ["我陪你收拾", "看着呢", "我在这儿守着"],
        done: ["这一步办好啦", "收拾妥当", "好，接着往下游"],
    },
    CommandGroup {
        code: b'n',
        commands: &["git fetch", "git pull", "git push"],
        editing: ["去远端看看？", "对岸有个仓库", "要和远端碰头？"],
        running: ["我去码头张望", "等对岸的消息", "看看远端怎么说"],
        done: ["这趟来回顺利", "和对岸聊完啦", "远端的事办好啦"],
    },
    CommandGroup {
        code: b't',
        commands: &[
            "cargo test",
            "npm test",
            "pnpm test",
            "yarn test",
            "go test",
            "pytest",
        ],
        editing: ["要跑测试啦？", "这轮能全绿吗？", "我来给测试加油"],
        running: ["我陪你等测试", "我来盯着结果", "一项一项来"],
        done: ["测试通过啦！", "绿灯亮啦！", "这轮过关了！"],
    },
    CommandGroup {
        code: b'b',
        commands: &["cargo build", "cargo check", "go build", "make", "cmake"],
        editing: ["准备开工？", "我来当监工", "工具都带齐了？"],
        running: ["我来帮你盯着", "这会儿我值班", "等它忙完"],
        done: ["顺利完成！", "收工，挥个鳍！", "这一关过啦"],
    },
    CommandGroup {
        code: b'i',
        commands: &[
            "npm install",
            "pnpm install",
            "yarn install",
            "pip install",
            "cargo fetch",
        ],
        editing: ["要补点依赖？", "缺什么零件啦？", "去取工具包？"],
        running: ["我在这儿等包裹", "让我守着工具箱", "给新零件腾个位置"],
        done: ["工具备好啦", "这批依赖齐啦", "包裹收到啦"],
    },
    CommandGroup {
        code: b'c',
        commands: &["mkdir", "touch", "cp", "mv"],
        editing: ["要整理文件啦？", "让我挪挪尾巴", "这边腾个位置"],
        running: ["我帮你看着", "我让条路", "小心放稳哦"],
        done: ["安顿好啦", "文件的事办妥啦", "收拾好了！"],
    },
];

const FAILED: Lines = ["咦，没顺利结束？", "我陪你看看输出", "要再瞧一眼吗？"];
const INTERRUPTED: Lines = ["好，歇一会儿", "收到，先停下", "我就在这儿等你"];

#[derive(Default)]
pub struct Dialogue {
    last: HashMap<(u8, u8), usize>,
}

impl Dialogue {
    pub fn choose(&mut self, phase: u8, code: u8) -> Option<&'static str> {
        // Shared result lines have a shared history, even when the command changes.
        let (key, lines) = if let Some(group) = GROUPS.iter().find(|group| group.code == code) {
            match phase {
                b'e' => ((phase, code), &group.editing),
                b'r' => ((phase, code), &group.running),
                b'+' => ((phase, code), &group.done),
                b'-' => ((phase, 0), &FAILED),
                b'i' => ((phase, 0), &INTERRUPTED),
                _ => return None,
            }
        } else {
            let (_, _, lines) = FAMILIES
                .iter()
                .find(|(_, kind, _)| *kind == code && phase == b'e')?;
            ((phase, code), lines)
        };
        let index = match self.last.get(&key) {
            Some(&previous) => (previous + fastrand::usize(1..lines.len())) % lines.len(),
            None => fastrand::usize(..lines.len()),
        };
        self.last.insert(key, index);
        Some(lines[index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_command_has_distinct_replies_and_remembers_each_stage() {
        let mut dialogue = Dialogue::default();
        let mut names = HashSet::new();
        let mut codes = HashSet::new();
        for group in GROUPS {
            assert!(codes.insert(group.code));
            for name in group.commands {
                assert!(names.insert(name));
            }
            for pool in [
                &group.editing,
                &group.running,
                &group.done,
                &FAILED,
                &INTERRUPTED,
            ] {
                assert_eq!(pool.iter().collect::<HashSet<_>>().len(), pool.len());
            }
            let mut previous = [None; 5];
            for _ in 0..12 {
                for ((phase, pool), last) in [
                    (b'e', &group.editing),
                    (b'r', &group.running),
                    (b'+', &group.done),
                    (b'-', &FAILED),
                    (b'i', &INTERRUPTED),
                ]
                .into_iter()
                .zip(previous.iter_mut())
                {
                    let line = dialogue.choose(phase, group.code).unwrap();
                    assert!(pool.contains(&line));
                    assert_ne!(*last, Some(line));
                    *last = Some(line);
                }
            }
        }
        assert!(dialogue.choose(b'+', b'0').is_none());
    }
}
