//! A session loads one catalogue for both shell hooks and bubble selection.
use serde::Deserialize;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs, io,
    path::Path,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pack {
    version: u32,
    command_not_found: Option<Vec<String>>,
    failed: Option<Vec<String>>,
    interrupted: Option<Vec<String>>,
    #[serde(default)]
    families: Vec<Family>,
    #[serde(default)]
    groups: Vec<Group>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Family {
    commands: Vec<String>,
    editing: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Group {
    commands: Vec<String>,
    editing: Vec<String>,
    running: Vec<String>,
    done: Vec<String>,
}

enum Rule {
    Family(Family),
    Group(Group),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    Editing,
    Running,
    Done,
    Failed,
    Interrupted,
    Missing,
}

/// A recognized shell event shared by gestures and speech. The speech module
/// chooses a line only when it receives the event.
#[derive(Clone, Copy)]
pub struct CommandEvent<'a> {
    pub phase: Phase,
    pool: usize,
    lines: &'a [String],
}

#[derive(Default)]
pub struct Catalog {
    // One-based indices into rules. Zero clears a preview.
    pub commands: BTreeMap<String, usize>,
    pub families: BTreeMap<String, usize>,
    rules: Vec<Rule>,
    command_not_found: Vec<String>,
    failed: Vec<String>,
    interrupted: Vec<String>,
}

impl Catalog {
    pub fn load(path: Option<&Path>) -> io::Result<Self> {
        let mut catalog = Self::default();
        catalog
            .extend(include_str!("../assets/commands.yaml"))
            .map_err(io::Error::other)?;
        if let Some(path) = path {
            let source = fs::read_to_string(path)
                .map_err(|e| io::Error::new(e.kind(), format!("{}: {e}", path.display())))?;
            catalog.extend(&source).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{}: {e}", path.display()),
                )
            })?;
        }
        Ok(catalog)
    }

    fn extend(&mut self, source: &str) -> Result<(), String> {
        let pack: Pack = yaml_serde::from_str(source).map_err(|e| e.to_string())?;
        if pack.version != 1 {
            return Err(format!("不支持的规则版本 {}", pack.version));
        }
        for (name, lines, target) in [
            (
                "command_not_found",
                pack.command_not_found,
                &mut self.command_not_found,
            ),
            ("failed", pack.failed, &mut self.failed),
            ("interrupted", pack.interrupted, &mut self.interrupted),
        ] {
            if let Some(lines) = lines {
                validate_lines(name, &lines)?;
                *target = lines;
            }
        }
        self.add_rules(pack.families.into_iter().map(Rule::Family))?;
        self.add_rules(pack.groups.into_iter().map(Rule::Group))
    }

    fn add_rules(&mut self, rules: impl Iterator<Item = Rule>) -> Result<(), String> {
        let mut seen = HashSet::new();
        for rule in rules {
            let (names, index) = match &rule {
                Rule::Family(f) => {
                    validate_lines(&format!("{:?}.editing", f.commands), &f.editing)?;
                    if f.commands.iter().any(|name| name.contains(' ')) {
                        return Err(format!("families 只接受单词工具名：{:?}", f.commands));
                    }
                    (&f.commands, &mut self.families)
                }
                Rule::Group(g) => {
                    for (phase, lines) in [
                        ("editing", &g.editing),
                        ("running", &g.running),
                        ("done", &g.done),
                    ] {
                        validate_lines(&format!("{:?}.{phase}", g.commands), lines)?;
                    }
                    (&g.commands, &mut self.commands)
                }
            };
            if names.is_empty() {
                return Err("commands 至少需要一个匹配项".into());
            }
            for name in names {
                if !valid_pattern(name) {
                    return Err(format!("无效的命令匹配项 {name:?}"));
                }
                if !seen.insert(name.clone()) {
                    return Err(format!("规则文件中有重复匹配项 {name:?}"));
                }
                index.insert(name.clone(), self.rules.len() + 1);
            }
            self.rules.push(rule);
        }
        Ok(())
    }

    pub fn editing(&self, prefix: &str) -> Option<usize> {
        let mut words = prefix.trim_end();
        loop {
            if let Some(id) = self.commands.get(words) {
                return Some(*id);
            }
            match words.rsplit_once(' ') {
                Some((rest, _)) => words = rest,
                None => return self.families.get(words).copied(),
            }
        }
    }

    pub fn event(&self, phase: u8, id: usize) -> Option<CommandEvent<'_>> {
        let rule = id.checked_sub(1).and_then(|i| self.rules.get(i));
        let (phase, pool, lines) = match (rule, phase) {
            (Some(Rule::Family(f)), b'e') => (Phase::Editing, id, &f.editing),
            (Some(Rule::Group(g)), b'e') => (Phase::Editing, id, &g.editing),
            (Some(Rule::Group(g)), b'r') => (Phase::Running, id, &g.running),
            (Some(Rule::Group(g)), b'+') => (Phase::Done, id, &g.done),
            (Some(Rule::Group(_)), b'-') => (Phase::Failed, 0, &self.failed),
            (Some(Rule::Group(_)), b'i') => (Phase::Interrupted, 0, &self.interrupted),
            (None, b'n') if id == 0 => (Phase::Missing, 0, &self.command_not_found),
            _ => return None,
        };
        Some(CommandEvent { phase, pool, lines })
    }

    pub fn known_prefix(&self, prefix: &str) -> bool {
        [&self.commands, &self.families].into_iter().any(|index| {
            index
                .range::<str, _>((
                    std::ops::Bound::Included(prefix),
                    std::ops::Bound::Unbounded,
                ))
                .next()
                .is_some_and(|(name, _)| name.starts_with(prefix))
        })
    }
}

// Both shell hooks embed these literal words in case statements.
fn valid_pattern(name: &str) -> bool {
    name.split(' ').all(|word| {
        !word.is_empty()
            && word
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_./+=:@".contains(&b))
    })
}

fn validate_lines(field: &str, lines: &[String]) -> Result<(), String> {
    let mut seen = HashSet::new();
    if lines.is_empty()
        || lines.iter().any(|line| {
            line.trim().is_empty() || line.chars().any(char::is_control) || !seen.insert(line)
        })
    {
        return Err(format!(
            "{field} 需要至少一句台词，每句须非空、无控制字符且不重复"
        ));
    }
    Ok(())
}

#[derive(Default)]
pub struct Dialogue {
    last: HashMap<(Phase, usize), usize>,
}

impl Dialogue {
    pub fn choose<'a>(&mut self, event: CommandEvent<'a>) -> &'a str {
        let key = (event.phase, event.pool);
        let lines = event.lines;
        let index = match self.last.get(&key) {
            Some(&previous) if lines.len() > 1 => {
                (previous + fastrand::usize(1..lines.len())) % lines.len()
            }
            _ => fastrand::usize(..lines.len()),
        };
        self.last.insert(key, index);
        &lines[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_specific_prefixes_and_overrides_only_named_aliases() {
        let file = tempfile::NamedTempFile::new().unwrap();
        fs::write(
            file.path(),
            r#"
version: 1
families: [{commands: [task], editing: [开工]}]
groups:
  - commands: [git status, task check]
    editing: [准备]
    running: [检查]
    done: [完成]
  - commands: [git status -s]
    editing: [简报]
    running: [稍候]
    done: [就绪]
"#,
        )
        .unwrap();
        let catalog = Catalog::load(Some(file.path())).unwrap();
        let mut dialogue = Dialogue::default();
        for (line, phase, expected) in [
            ("task", b'e', "开工"),
            ("task check", b'r', "检查"),
            ("git status -s src", b'e', "简报"),
            ("git status -something", b'+', "完成"),
        ] {
            let id = catalog.editing(line).unwrap();
            for _ in 0..2 {
                assert_eq!(dialogue.choose(catalog.event(phase, id).unwrap()), expected);
            }
        }
        assert_ne!(catalog.commands["git diff"], catalog.commands["git status"]);
        assert!(catalog.event(b'+', catalog.families["task"]).is_none());
    }

    #[test]
    fn multi_line_pools_remember_their_last_reply() {
        let catalog = Catalog::load(None).unwrap();
        let mut dialogue = Dialogue::default();
        for phase in [b'e', b'r', b'+', b'-', b'i'] {
            let mut previous = None;
            for turn in 0..12 {
                let id = if matches!(phase, b'-' | b'i') && turn % 2 == 0 {
                    catalog.commands["ls"]
                } else {
                    catalog.commands["pwd"]
                };
                let line = dialogue.choose(catalog.event(phase, id).unwrap());
                assert_ne!(Some(line), previous);
                previous = Some(line);
            }
        }
        let missing = dialogue.choose(catalog.event(b'n', 0).unwrap());
        assert!(catalog.command_not_found.iter().any(|line| line == missing));
        assert_ne!(dialogue.choose(catalog.event(b'n', 0).unwrap()), missing);
        assert!(catalog.event(b'+', 0).is_none());
    }

    #[test]
    fn rejects_invalid_imports_with_the_source_path() {
        let file = tempfile::NamedTempFile::new().unwrap();
        for content in [
            "version: 2",
            "version: 1\ngropus: []",
            "version: 1\nfailed: []",
            "version: 1\nfailed: [same, same]",
            "version: 1\nfailed: [\"line\\nbreak\"]",
            "version: 1\nfamilies: [{commands: ['a*b'], editing: [hi]}]",
            "version: 1\nfamilies: [{commands: [git, git], editing: [hi]}]",
        ] {
            fs::write(file.path(), content).unwrap();
            let error = Catalog::load(Some(file.path())).err().expect(content);
            assert_eq!(error.kind(), io::ErrorKind::InvalidData);
            assert!(error.to_string().contains(file.path().to_str().unwrap()));
        }
    }
}
