//! Run the pet over a real fish session through a PTY and read its speech.
#![cfg(unix)]

use rsille_overlay::vt100;
use std::{
    ffi::CString,
    fs::File,
    io::{Read, Write},
    os::fd::FromRawFd,
    time::{Duration, Instant},
};

const ROWS: u16 = 30;
const COLS: u16 = 100;

struct Terminal {
    master: File,
    grid: vt100::Parser,
    pid: i32,
    seen: Vec<u8>,
}

impl Terminal {
    fn spawn(args: &[&str], shell: &str, home: &std::path::Path) -> Self {
        let exe = CString::new(env!("CARGO_BIN_EXE_rsille")).unwrap();
        let args: Vec<_> = std::iter::once(exe.clone())
            .chain(args.iter().map(|a| CString::new(*a).unwrap()))
            .collect();
        let mut argv: Vec<_> = args.iter().map(|a| a.as_ptr()).collect();
        argv.push(std::ptr::null());
        let env = [
            format!("SHELL={shell}"),
            format!("HOME={}", home.display()),
            format!("XDG_CONFIG_HOME={}", home.display()),
            format!("XDG_DATA_HOME={}", home.display()),
            format!("PATH={}", std::env::var("PATH").unwrap()),
            "TERM=xterm-256color".into(),
        ]
        .map(|v| CString::new(v).unwrap());
        let mut envp: Vec<_> = env.iter().map(|v| v.as_ptr()).collect();
        envp.push(std::ptr::null());
        let mut size = libc::winsize {
            ws_row: ROWS,
            ws_col: COLS,
            ws_xpixel: COLS * 10,
            ws_ypixel: ROWS * 20,
        };
        let mut fd = -1;
        let pid = unsafe {
            libc::forkpty(
                &mut fd,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &raw mut size,
            )
        };
        assert!(pid >= 0);
        if pid == 0 {
            unsafe {
                libc::chdir(c"/".as_ptr());
                libc::execve(exe.as_ptr(), argv.as_ptr(), envp.as_ptr());
                libc::_exit(127);
            }
        }
        unsafe {
            let flags = libc::fcntl(fd, libc::F_GETFL);
            libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
        }
        Self {
            master: unsafe { File::from_raw_fd(fd) },
            grid: vt100::Parser::new(ROWS.try_into().unwrap(), COLS.try_into().unwrap(), 0),
            pid,
            seen: Vec::new(),
        }
    }

    fn pump(&mut self) {
        let mut buffer = [0; 8192];
        while let Ok(n @ 1..) = self.master.read(&mut buffer) {
            self.seen.extend_from_slice(&buffer[..n]);
            self.grid.process(&buffer[..n]);
        }
        // Answer the startup graphics probe like a Sixel terminal (DA1 with 4).
        if let Some(at) = self.seen.windows(3).position(|w| w == b"\x1b[c") {
            self.seen.drain(..at + 3);
            self.master.write_all(b"\x1b[?62;4c").unwrap();
        }
        let keep = self.seen.len().saturating_sub(2);
        self.seen.drain(..keep);
    }

    /// Visible text without line breaks or spaces, so wrapped bubbles still match.
    fn text(&self) -> String {
        self.grid.screen().contents().replace(['\n', ' '], "")
    }

    fn wait_for_any(&mut self, what: &str, lines: &[&str]) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            self.pump();
            let text = self.text();
            if lines
                .iter()
                .any(|line| text.contains(&line.replace(' ', "")))
            {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!(
            "timed out waiting for {what}; screen:\n{}",
            self.grid.screen().contents()
        );
    }

    fn type_slowly(&mut self, text: &[u8]) {
        for byte in text {
            self.master.write_all(std::slice::from_ref(byte)).unwrap();
            std::thread::sleep(Duration::from_millis(30));
            self.pump();
        }
    }

    fn exit_status(&mut self) -> i32 {
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut status = 0;
        while Instant::now() < deadline {
            self.pump();
            if unsafe { libc::waitpid(self.pid, &mut status, libc::WNOHANG) } == self.pid {
                return libc::WEXITSTATUS(status);
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        unsafe { libc::kill(self.pid, libc::SIGKILL) };
        panic!("pet did not exit");
    }
}

fn find(name: &str) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|p| p.is_file())
        .map(|p| p.display().to_string())
}

/// Type a known command and a missing one, and read the pet's replies.
fn talk(name: &str, configure: impl FnOnce(&std::path::Path)) {
    let Some(shell) = find(name) else {
        eprintln!("{name} not installed; skipping");
        return;
    };
    let home = tempfile::tempdir().unwrap();
    configure(home.path());
    let mut term = Terminal::spawn(
        &["pet", "--backend", "sixel", "--features", "speech"],
        &shell,
        home.path(),
    );
    term.wait_for_any("a prompt", &["$"]);
    term.type_slowly(b"pwd");
    term.wait_for_any(
        "the editing line",
        &["游到哪儿了？", "看看咱们的位置？", "让我认认路"],
    );
    term.type_slowly(b"\r");
    term.wait_for_any(
        "the result line",
        &["原来在这里", "记住这个路口啦", "位置找到了"],
    );
    term.type_slowly(b"lxq\r");
    term.wait_for_any(
        "the missing-command line",
        &[
            "咦，这条命令游哪儿去了？",
            "是不是少敲了一个字母？",
            "我也没找到这条命令",
            "再瞅瞅它叫什么？",
            "这名字我没认出来",
        ],
    );
    term.type_slowly(b"exit\r");
    assert_eq!(term.exit_status(), 0);
}

#[test]
fn the_pet_talks_about_fish_commands_from_typing_to_their_result() {
    talk("fish", |home| {
        std::fs::create_dir_all(home.join("fish")).unwrap();
        std::fs::write(
            home.join("fish/config.fish"),
            "set -g fish_greeting\nfunction fish_prompt; echo -n '$ '; end\n",
        )
        .unwrap();
    });
}

#[test]
fn the_pet_talks_about_bash_commands_from_typing_to_their_result() {
    talk("bash", |home| {
        std::fs::write(home.join(".bashrc"), "PS1='$ '\n").unwrap();
    });
}

#[test]
fn the_pet_talks_about_zsh_commands_from_editing_to_their_result() {
    talk("zsh", |home| {
        std::fs::write(home.join(".zshrc"), "PROMPT='$ '\n").unwrap();
    });
}
