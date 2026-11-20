//! A private datagram socket carries only fixed command categories from the interactive shell.
use crate::dialogue::GROUPS;
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
    _startup: Option<tempfile::TempDir>,
    pub fish_init: Option<CString>,
    pub environment: Vec<CString>,
}

impl Hooks {
    pub fn prepare(shell: &Path) -> io::Result<Option<(Self, UnixDatagram)>> {
        let fish = shell.file_name().is_some_and(|name| name == "fish");
        if !fish && shell.file_name().is_none_or(|name| name != "zsh") {
            return Ok(None);
        }
        let (events, sender) = UnixDatagram::pair()?;
        events.set_nonblocking(true)?;
        sender.set_nonblocking(true)?;
        use std::fmt::Write;
        let mut cases = String::new();
        for (command, kind) in commands() {
            let code = char::from(kind);
            if fish {
                writeln!(cases, "            case '{command}' '{command} *'").unwrap();
                writeln!(cases, "                set -g __rsille_pet_kind {code}").unwrap();
            } else {
                writeln!(cases, "    '{command}'|'{command} '*) reply={code} ;;").unwrap();
            }
        }
        if fish {
            let init = include_str!("commands.fish")
                .replace("@COMMAND_CASES@", &cases)
                .replace("@EVENT_FD@", &sender.as_raw_fd().to_string());
            return Ok(Some((
                Self {
                    events,
                    fish_init: Some(CString::new(init)?),
                    _startup: None,
                    environment: vec![],
                },
                sender,
            )));
        }
        let startup = tempfile::Builder::new()
            .prefix("rsille-pet-")
            .permissions(fs::Permissions::from_mode(0o700))
            .tempdir()?;
        let script = include_str!("commands.zsh")
            .replace("@COMMAND_CASES@", &cases)
            .replace("@EVENT_FD@", &sender.as_raw_fd().to_string());
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
                _startup: Some(startup),
                fish_init: None,
                environment,
            },
            sender,
        )))
    }
}

fn commands() -> impl Iterator<Item = (&'static str, u8)> {
    GROUPS
        .iter()
        .flat_map(|group| group.commands.iter().map(|&name| (name, group.code)))
}

// Only keep a prefix of a known command. Complex line edits defer to fish_preexec.
#[derive(Default)]
pub struct FishPreview {
    prefix: Option<&'static str>,
    shown: Option<u8>,
}

impl FishPreview {
    pub fn prompt(&mut self) {
        self.prefix = Some("");
        self.shown = None;
    }

    pub fn end(&mut self) -> bool {
        self.prefix = None;
        self.shown.take().is_some()
    }

    pub fn input(&mut self, byte: u8) -> Option<u8> {
        if byte.is_ascii_control() && !matches!(byte, 8 | 127)
            || matches!(byte, b';' | b'|' | b'&' | b'>' | b'<' | b'(' | b')')
        {
            return self.end().then_some(b'0');
        }
        let prefix = self.prefix?;
        if byte == b' ' && (prefix.is_empty() || self.shown.is_some()) {
            if self.shown.is_some() {
                self.prefix = None;
            }
            return None;
        }
        self.prefix = match byte {
            8 | 127 => Some(&prefix[..prefix.len().saturating_sub(1)]),
            _ => commands()
                .map(|(name, _)| name)
                .find(|name| {
                    name.starts_with(prefix) && name.as_bytes().get(prefix.len()) == Some(&byte)
                })
                .map(|name| &name[..prefix.len() + 1]),
        };
        let kind = commands()
            .find(|&(name, _)| Some(name) == self.prefix)
            .map(|(_, kind)| kind);
        if kind == self.shown {
            return None;
        }
        self.shown = kind;
        Some(kind.unwrap_or(b'0'))
    }
}

#[cfg(test)]
mod tests {
    use super::FishPreview;

    #[test]
    fn preview_only_matches_at_a_fresh_fish_prompt() {
        let mut preview = FishPreview::default();
        let events = |p: &mut FishPreview, bytes: &[u8]| {
            bytes.iter().filter_map(|&b| p.input(b)).collect::<Vec<_>>()
        };
        assert!(events(&mut preview, b"git status").is_empty());
        preview.prompt();
        assert_eq!(events(&mut preview, b"git status"), b"g");
        assert_eq!(events(&mut preview, b"\x7fs"), b"0g");
        assert_eq!(events(&mut preview, b" -s | cat"), b"0");
        preview.prompt();
        assert!(events(&mut preview, b"echo cargo test").is_empty());
        preview.prompt();
        assert!(events(&mut preview, b"git sta\tcargo test").is_empty());
        preview.prompt();
        assert_eq!(events(&mut preview, b"cargo test\rgit status"), b"t0");
    }
}
