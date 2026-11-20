//! A private datagram socket carries only numeric rule IDs from the interactive shell.
use crate::dialogue::Catalog;
use std::{
    ffi::CString,
    fs, io,
    os::{
        fd::AsRawFd,
        unix::{ffi::OsStrExt, fs::PermissionsExt, net::UnixDatagram},
    },
    path::Path,
};

pub struct Hooks {
    pub events: UnixDatagram,
    _startup: tempfile::TempDir,
    pub fish_init: Option<CString>,
    pub environment: Vec<CString>,
}

impl Hooks {
    pub fn prepare(shell: &Path, catalog: &Catalog) -> io::Result<Option<(Self, UnixDatagram)>> {
        let fish = shell.file_name().is_some_and(|name| name == "fish");
        if !fish && shell.file_name().is_none_or(|name| name != "zsh") {
            return Ok(None);
        }
        let (events, sender) = UnixDatagram::pair()?;
        events.set_nonblocking(true)?;
        sender.set_nonblocking(true)?;
        use std::fmt::Write;
        let mut cases = String::new();
        // A longer literal prefix wins over a general command in either shell.
        for (command, code) in catalog.commands.iter().rev() {
            if fish {
                writeln!(cases, "            case '{command}' '{command} *'").unwrap();
                writeln!(cases, "                set -g __rsille_pet_kind {code}").unwrap();
            } else {
                writeln!(cases, "    '{command}'|'{command} '*) reply={code} ;;").unwrap();
            }
        }
        let startup = tempfile::Builder::new()
            .prefix("rsille-pet-")
            .permissions(fs::Permissions::from_mode(0o700))
            .tempdir()?;
        if fish {
            let init = include_str!("commands.fish")
                .replace("@{COMMAND_CASES}@", &cases)
                .replace("@{EVENT_FD}@", &sender.as_raw_fd().to_string());
            let script = startup.path().join("commands.fish");
            fs::write(&script, init)?;
            // Single-quoted fish strings escape only backslash and single quote.
            let mut source = b"source '".to_vec();
            for &byte in script.as_os_str().as_bytes() {
                if matches!(byte, b'\\' | b'\'') {
                    source.push(b'\\');
                }
                source.push(byte);
            }
            source.push(b'\'');
            return Ok(Some((
                Self {
                    events,
                    fish_init: Some(CString::new(source)?),
                    _startup: startup,
                    environment: vec![],
                },
                sender,
            )));
        }
        let mut families = String::new();
        for (name, kind) in &catalog.families {
            writeln!(families, "      '{name}') reply={kind} ;;").unwrap();
        }
        let script = include_str!("commands.zsh")
            .replace("@{COMMAND_CASES}@", &cases)
            .replace("@{FAMILY_CASES}@", &families)
            .replace("@{EVENT_FD}@", &sender.as_raw_fd().to_string());
        fs::write(
            startup.path().join(".zshenv"),
            include_str!("startup.zshenv"),
        )?;
        fs::write(startup.path().join(".zshrc"), script)?;
        let mut environment = Vec::new();
        for (mut name, value) in std::env::vars_os() {
            if name == "_RSILLE_ORIG_ZDOTDIR" {
                continue;
            }
            if name == "ZDOTDIR" {
                name = "_RSILLE_ORIG_ZDOTDIR".into();
            }
            name.push("=");
            name.push(value);
            environment.push(CString::new(name.as_bytes())?);
        }
        let mut setting = std::ffi::OsString::from("ZDOTDIR=");
        setting.push(startup.path());
        environment.push(CString::new(setting.as_bytes())?);
        Ok(Some((
            Self {
                events,
                _startup: startup,
                fish_init: None,
                environment,
            },
            sender,
        )))
    }
}

// Retain known prefixes only. An unknown argument is represented by its length.
#[derive(Default)]
pub struct FishPreview {
    active: bool,
    prefix: String,
    // Extra spaces at each known word boundary, for backspace.
    spaces: Vec<usize>,
    tail: usize,
    shown: Option<usize>,
}

impl FishPreview {
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
    use super::FishPreview;

    #[test]
    fn preview_recovers_a_more_specific_option_after_backspace() {
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), "version: 1\ngroups: [{commands: [git status -s], editing: [short], running: [run], done: [done]}]").unwrap();
        let catalog = super::Catalog::load(Some(file.path())).unwrap();
        let mut preview = FishPreview::default();
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
    fn preview_only_matches_at_a_fresh_fish_prompt() {
        let catalog = super::Catalog::load(None).unwrap();
        let add = catalog.editing("git add").unwrap();
        let install = catalog.editing("npm install").unwrap();
        let packages = catalog.editing("npm").unwrap();
        let push = catalog.editing("git push").unwrap();
        let test = catalog.editing("cargo test").unwrap();
        let project = catalog.editing("cargo").unwrap();
        let status = catalog.editing("git status").unwrap();
        let git = catalog.editing("git").unwrap();
        let mut preview = FishPreview::default();
        let events = |p: &mut FishPreview, bytes: &[u8]| {
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
