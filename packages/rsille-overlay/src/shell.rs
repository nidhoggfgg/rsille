//! bash, fish and zsh hooks that report shell activity over a private datagram socket.
use crate::{Shell, ShellEvent};
use std::{
    ffi::CString,
    fs, io,
    os::{
        fd::AsRawFd,
        unix::{ffi::OsStrExt, fs::PermissionsExt, net::UnixDatagram},
    },
    path::Path,
};

/// Large enough for a 4097-character line of four-byte characters.
pub const MAX_EVENT: usize = 4097 * 4 + 1;

pub struct Hooks {
    pub events: UnixDatagram,
    pub shell: Shell,
    _startup: tempfile::TempDir,
    /// Arguments placed before `-i`.
    pub args: Vec<CString>,
    /// A replacement environment, if the shell needs one.
    pub environment: Option<Vec<CString>>,
}

impl Hooks {
    /// Returns the hooks and the socket end the child must inherit, or `None`
    /// for shells without hooks.
    pub fn prepare(shell: &Path) -> io::Result<Option<(Self, UnixDatagram)>> {
        let kind = match shell.file_name().and_then(|name| name.to_str()) {
            Some("bash") => Shell::Bash,
            Some("fish") => Shell::Fish,
            Some("zsh") => Shell::Zsh,
            _ => return Ok(None),
        };
        let (events, sender) = UnixDatagram::pair()?;
        events.set_nonblocking(true)?;
        sender.set_nonblocking(true)?;
        let fd = sender.as_raw_fd().to_string();
        let startup = tempfile::Builder::new()
            .prefix("rsille-overlay-")
            .permissions(fs::Permissions::from_mode(0o700))
            .tempdir()?;
        if kind == Shell::Bash {
            let rc = startup.path().join("bashrc");
            fs::write(
                &rc,
                include_str!("shell/hooks.bash").replace("@{EVENT_FD}@", &fd),
            )?;
            return Ok(Some((
                Self {
                    events,
                    shell: kind,
                    _startup: startup,
                    // Long options must precede bash's single-letter ones.
                    args: vec![c"--rcfile".into(), CString::new(rc.as_os_str().as_bytes())?],
                    environment: None,
                },
                sender,
            )));
        }
        if kind == Shell::Fish {
            let init = include_str!("shell/hooks.fish").replace("@{EVENT_FD}@", &fd);
            let script = startup.path().join("hooks.fish");
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
                    shell: Shell::Fish,
                    args: vec![c"-C".into(), CString::new(source)?],
                    _startup: startup,
                    environment: None,
                },
                sender,
            )));
        }
        let script = include_str!("shell/hooks.zsh").replace("@{EVENT_FD}@", &fd);
        fs::write(
            startup.path().join(".zshenv"),
            include_str!("shell/startup.zshenv"),
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
                shell: Shell::Zsh,
                _startup: startup,
                args: vec![],
                environment: Some(environment),
            },
            sender,
        )))
    }
}

/// Decode one datagram. Unknown or malformed messages are ignored.
pub fn parse(message: &[u8]) -> Option<ShellEvent<'_>> {
    let (&tag, rest) = message.split_first()?;
    let text = || std::str::from_utf8(rest).ok();
    match tag {
        b'p' if rest.is_empty() => Some(ShellEvent::Prompt),
        b'r' if rest.is_empty() => Some(ShellEvent::Read),
        b'e' => text().map(ShellEvent::Edit),
        b'x' => text().map(ShellEvent::Exec),
        b'd' => text()?.parse().ok().map(ShellEvent::Done),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_each_event_and_rejects_malformed_messages() {
        assert_eq!(parse(b"p"), Some(ShellEvent::Prompt));
        assert_eq!(parse(b"r"), Some(ShellEvent::Read));
        assert_eq!(parse(b"e"), Some(ShellEvent::Edit("")));
        assert_eq!(parse(b"egit st"), Some(ShellEvent::Edit("git st")));
        assert_eq!(
            parse("xls 文件".as_bytes()),
            Some(ShellEvent::Exec("ls 文件"))
        );
        assert_eq!(parse(b"d127"), Some(ShellEvent::Done(127)));
        for bad in [&b""[..], b"px", b"d", b"dx", b"q", b"x\xff"] {
            assert_eq!(parse(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn fish_hooks_report_commands_and_statuses() {
        let Ok(fish) = which("fish") else { return };
        let (hooks, sender) = Hooks::prepare(&fish).unwrap().unwrap();
        assert_eq!(hooks.shell, Shell::Fish);
        let init = hooks.args[1].to_str().unwrap();
        // Run the hooks in a non-interactive fish with the events emitted by hand.
        let script = format!(
            "{init}; emit fish_preexec 'echo  hi | cat'; false; emit fish_postexec; emit fish_prompt"
        );
        let status = unsafe {
            let fd = sender.as_raw_fd();
            assert_eq!(libc::fcntl(fd, libc::F_SETFD, 0), 0);
            std::process::Command::new(&fish)
                .args(["--no-config", "-c", &script])
                .status()
                .unwrap()
        };
        assert!(status.success());
        let mut events = Vec::new();
        let mut buffer = [0; MAX_EVENT];
        while let Ok(n) = hooks.events.recv(&mut buffer) {
            events.push(buffer[..n].to_vec());
        }
        assert_eq!(events, [&b"xecho  hi | cat"[..], b"d1", b"p"]);
    }

    fn which(name: &str) -> io::Result<std::path::PathBuf> {
        let path = std::env::var_os("PATH").unwrap_or_default();
        std::env::split_paths(&path)
            .map(|dir| dir.join(name))
            .find(|p| p.is_file())
            .ok_or_else(|| io::ErrorKind::NotFound.into())
    }
}
