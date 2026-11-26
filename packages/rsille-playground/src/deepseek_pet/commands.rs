//! Preview the command being typed in shells that report no edits (bash, fish).
use crate::deepseek_pet::dialogue::Catalog;

// Retain known prefixes only. An unknown argument is represented by its length.
#[derive(Default)]
pub struct TypedPreview {
    active: bool,
    prefix: String,
    // Extra spaces at each known word boundary, for backspace.
    spaces: Vec<usize>,
    tail: usize,
    shown: Option<usize>,
}

impl TypedPreview {
    pub fn prompt(&mut self) {
        *self = Self {
            active: true,
            ..Self::default()
        };
    }

    pub fn end(&mut self) -> bool {
        self.active = false;
        self.shown.take().is_some()
    }

    pub fn input(&mut self, catalog: &Catalog, byte: u8) -> Option<usize> {
        let backspace = matches!(byte, 8 | 127);
        if byte.is_ascii_control() && !backspace || b";|&><()`".contains(&byte) {
            return self.end().then_some(0);
        }
        if !self.active {
            return (backspace && self.end()).then_some(0);
        }
        if !byte.is_ascii() {
            self.active = false;
            if self.tail > 0 {
                return None;
            }
            let parent = self
                .prefix
                .rsplit_once(' ')
                .map_or("", |(parent, _)| parent);
            return self.change(catalog.editing(parent));
        }
        if self.tail > 0 {
            if backspace {
                self.tail -= 1;
            } else {
                self.tail += 1;
            }
            if self.tail > 0 {
                return None;
            }
        } else if backspace {
            if self.prefix.ends_with(' ') {
                let count = self.spaces.last_mut().unwrap();
                if *count > 1 {
                    *count -= 1;
                    return None;
                }
                self.spaces.pop();
            }
            self.prefix.pop();
        } else {
            if byte == b' ' {
                if self.prefix.is_empty() {
                    return None;
                }
                if self.prefix.ends_with(' ') {
                    *self.spaces.last_mut().unwrap() += 1;
                    return None;
                }
            }
            self.prefix.push(char::from(byte));
            if !catalog.known_prefix(&self.prefix) {
                let kind = self
                    .prefix
                    .rsplit_once(' ')
                    .and_then(|(parent, _)| catalog.editing(parent));
                self.prefix.pop();
                self.tail = 1;
                return self.change(kind);
            }
            if byte == b' ' {
                self.spaces.push(1);
            }
        }
        self.change(catalog.editing(&self.prefix))
    }

    fn change(&mut self, kind: Option<usize>) -> Option<usize> {
        if kind == self.shown {
            return None;
        }
        self.shown = kind;
        Some(kind.unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::TypedPreview;

    #[test]
    fn preview_recovers_a_more_specific_option_after_backspace() {
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), "version: 1\ngroups: [{commands: [git status -s], editing: [short], running: [run], done: [done]}]").unwrap();
        let catalog = super::Catalog::load(Some(file.path())).unwrap();
        let mut preview = TypedPreview::default();
        preview.prompt();
        let events: Vec<_> = b"git   status   -sx\x7f  src"
            .iter()
            .filter_map(|&b| preview.input(&catalog, b))
            .collect();
        let status = catalog.commands["git status"];
        let short = catalog.commands["git status -s"];
        assert_eq!(
            events,
            [catalog.families["git"], status, short, status, short]
        );
    }

    #[test]
    fn preview_only_matches_at_a_fresh_prompt() {
        let catalog = super::Catalog::load(None).unwrap();
        let add = catalog.editing("git add").unwrap();
        let install = catalog.editing("npm install").unwrap();
        let packages = catalog.editing("npm").unwrap();
        let push = catalog.editing("git push").unwrap();
        let test = catalog.editing("cargo test").unwrap();
        let project = catalog.editing("cargo").unwrap();
        let status = catalog.editing("git status").unwrap();
        let git = catalog.editing("git").unwrap();
        let mut preview = TypedPreview::default();
        let events = |p: &mut TypedPreview, bytes: &[u8]| {
            bytes
                .iter()
                .filter_map(|&b| p.input(&catalog, b))
                .collect::<Vec<_>>()
        };
        assert!(events(&mut preview, b"git status").is_empty());
        preview.prompt();
        assert!(events(&mut preview, b"gi").is_empty());
        assert_eq!(events(&mut preview, b"t"), [git]);
        assert!(events(&mut preview, b" sta").is_empty());
        assert_eq!(events(&mut preview, b"tus"), [status]);
        assert_eq!(events(&mut preview, b"\x7fs"), [git, status]);
        assert_eq!(events(&mut preview, b" -s | cat"), [0]);
        preview.prompt();
        assert!(events(&mut preview, b"echo cargo test").is_empty());
        preview.prompt();
        assert_eq!(events(&mut preview, b"git sta\tcargo test"), [git, 0]);
        preview.prompt();
        assert_eq!(
            events(&mut preview, b"cargo test\rgit status"),
            [project, test, 0]
        );
        preview.prompt();
        assert_eq!(
            events(&mut preview, b"gitx\x7f  ps\x7fush origin"),
            [git, 0, git, push]
        );
        preview.prompt();
        assert_eq!(events(&mut preview, b"npm install"), [packages, install]);
        preview.prompt();
        assert_eq!(events(&mut preview, "git add 文件".as_bytes()), [git, add]);
        assert_eq!(events(&mut preview, b" | cat"), [0]);
    }
}
