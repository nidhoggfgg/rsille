//! A private datagram socket carries only fixed command categories from the interactive shell.
use crate::dialogue::{FAMILIES, GROUPS};
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
        let mut families = String::new();
        for (name, kind) in families_by_name() {
            writeln!(families, "      '{name}') reply={} ;;", char::from(kind)).unwrap();
        }
        let script = include_str!("commands.zsh")
            .replace("@COMMAND_CASES@", &cases)
            .replace("@FAMILY_CASES@", &families)
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

fn families_by_name() -> impl Iterator<Item = (&'static str, u8)> {
    FAMILIES
        .iter()
        .flat_map(|&(names, kind, _)| names.iter().map(move |&name| (name, kind)))
}

fn family(name: &str) -> Option<u8> {
    families_by_name()
        .find(|&(tool, _)| tool == name)
        .map(|(_, kind)| kind)
}

// Only keep a prefix of a known command. Complex line edits defer to fish_preexec.
#[derive(Default)]
pub struct FishPreview {
    prefix: Option<&'static str>,
    extra_spaces: usize,
    // Length of the unmatched ASCII suffix, so backspace can recover a known prefix.
    tail: usize,
    shown: Option<u8>,
}

impl FishPreview {
    pub fn prompt(&mut self) {
        *self = Self {
            prefix: Some(""),
            ..Self::default()
        };
    }

    pub fn end(&mut self) -> bool {
        self.prefix = None;
        self.shown.take().is_some()
    }

    pub fn input(&mut self, byte: u8) -> Option<u8> {
        if byte.is_ascii_control() && !matches!(byte, 8 | 127)
            || matches!(byte, b';' | b'|' | b'&' | b'>' | b'<' | b'(' | b')' | b'`')
        {
            return self.end().then_some(b'0');
        }
        let backspace = matches!(byte, 8 | 127);
        let Some(prefix) = self.prefix else {
            return (backspace && self.end()).then_some(b'0');
        };
        if !byte.is_ascii() {
            // Leave Unicode editing to fish. A filename can keep its command's bubble.
            self.prefix = None;
            return if self.tail > 0 {
                None
            } else {
                self.change(prefix.split_once(' ').and_then(|(tool, _)| family(tool)))
            };
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
        } else if byte == b' ' && (prefix.is_empty() || prefix.ends_with(' ')) {
            if !prefix.is_empty() {
                self.extra_spaces += 1;
            }
            return None;
        } else if backspace && prefix.ends_with(' ') && self.extra_spaces > 0 {
            self.extra_spaces -= 1;
            return None;
        } else if backspace {
            self.prefix = Some(&prefix[..prefix.len().saturating_sub(1)]);
        } else if let Some(name) = commands().map(|(name, _)| name).find(|name| {
            name.starts_with(prefix) && name.as_bytes().get(prefix.len()) == Some(&byte)
        }) {
            self.prefix = Some(&name[..prefix.len() + 1]);
        } else {
            self.tail = 1;
            // A space starts arguments. Any other byte extends the current word.
            let kind = if byte == b' ' {
                self.shown
            } else {
                prefix.split_once(' ').and_then(|(tool, _)| family(tool))
            };
            return self.change(kind);
        }
        let prefix = self.prefix?;
        let kind = commands()
            .find(|&(name, _)| name == prefix)
            .map(|(_, kind)| kind)
            .or_else(|| family(prefix.split(' ').next()?));
        self.change(kind)
    }

    fn change(&mut self, kind: Option<u8>) -> Option<u8> {
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
        assert_eq!(events(&mut preview, b"gi"), b"");
        assert_eq!(events(&mut preview, b"t"), b"G");
        assert_eq!(events(&mut preview, b" sta"), b"");
        assert_eq!(events(&mut preview, b"tus"), b"g");
        assert_eq!(events(&mut preview, b"\x7fs"), b"Gg");
        assert_eq!(events(&mut preview, b" -s | cat"), b"0");
        preview.prompt();
        assert!(events(&mut preview, b"echo cargo test").is_empty());
        preview.prompt();
        assert_eq!(events(&mut preview, b"git sta\tcargo test"), b"G0");
        preview.prompt();
        assert_eq!(events(&mut preview, b"cargo test\rgit status"), b"Bt0");
        preview.prompt();
        assert_eq!(events(&mut preview, b"gitx\x7f  ps\x7fush origin"), b"G0Gn");
        preview.prompt();
        assert_eq!(events(&mut preview, b"npm install"), b"Pi");
        preview.prompt();
        assert_eq!(events(&mut preview, "git add 文件".as_bytes()), b"Gv");
        assert_eq!(events(&mut preview, b" | cat"), b"0");
    }
}
