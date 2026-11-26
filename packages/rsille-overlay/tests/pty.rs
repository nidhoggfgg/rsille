//! Drive a real overlay session through a PTY, the way a terminal would.
//!
//! The test binary re-runs itself as the session process (see `session_child`)
//! and reads its output into a terminal emulator.
#![cfg(unix)]

use rsille_overlay::{Anchor, Session, ShellEvent, WidgetOverlay};
use rsille_tui::prelude::{button, label, row, App, Widget};
use std::{
    ffi::CString,
    fs::File,
    io::{Read, Write},
    os::{fd::FromRawFd, unix::ffi::OsStrExt},
    time::{Duration, Instant},
};

const CHILD: &str = "RSILLE_OVERLAY_PTY_CHILD";

#[derive(Debug)]
enum Msg {
    Shell(String),
    Clear,
}

fn badge_view(log: &String) -> impl Widget<Msg> {
    row::<Msg>()
        .child(label(format!("[{log}]")).width(20))
        .child(button("x").on_click(|| Msg::Clear))
}

/// The session process. Does nothing unless started by `Terminal::spawn`.
#[test]
fn session_child() {
    let Some(mode) = std::env::var_os(CHILD) else {
        return;
    };
    if mode == "kitty-screen-switch" {
        use rsille_overlay::{CellPixels, InlineRasterSession, Stack};

        let counter = App::new(0)
            .update(|n, (), _| *n += 1)
            .view(|n| button(format!("count={n}")).on_click(|| ()))
            .build();
        let marker = App::new(())
            .update(|_, (), _| {})
            .view(|_| label("marker"))
            .build();
        let layers = Stack::new()
            .push(WidgetOverlay::new(counter, (12, 1)).primary_screen_only())
            .push(WidgetOverlay::new(marker, (6, 1)).anchor(Anchor::TopLeft));
        let child = r#"
            stty -echo
            read -r start
            printf '\033[?1049h\033[?1000h\033[?1006h\033[3;1Halt-child'
            IFS= read -r click
            [ "$click" = "$(printf '\033[<0;56;12M\033[<0;56;12m')" ] || exit 3
            printf '\033[?1000l\033[?1006l\033[?1049l\r\nmouse-delivered\r\n'
            read -r finish
        "#;
        let status = Session::new()
            .raster_session(InlineRasterSession::Kitty(CellPixels {
                width: 8,
                height: 16,
            }))
            .command(["/bin/sh", "-c", child])
            .mouse(true)
            .run(layers)
            .unwrap();
        std::process::exit(status);
    }
    if mode == "widget-input" {
        let (tx, rx) = std::sync::mpsc::channel();
        let app = App::new(())
            .update(move |_, (), _| tx.send(b"yes, continue".to_vec()).unwrap())
            .view(|_| button("Yes").on_click(|| ()))
            .build();
        let status = Session::new()
            .command([
                "/bin/sh",
                "-c",
                "read -r line && printf '<%s>\\n' \"$line\"",
            ])
            .mouse(true)
            .run_with_input(WidgetOverlay::new(app, (7, 1)), rx)
            .unwrap();
        std::process::exit(status);
    }
    // Shows "<status> <command line>" for the last command.
    let app = App::new(String::new())
        .update(|log: &mut String, msg, _ctx| match msg {
            Msg::Shell(event) => *log = event,
            Msg::Clear => log.clear(),
        })
        .view(badge_view)
        .build();
    let mut line = String::new();
    let overlay = WidgetOverlay::new(app, (24, 1))
        .anchor(Anchor::TopRight)
        .on_shell(move |event| match event {
            ShellEvent::Exec(exec) => {
                line = exec.to_owned();
                None
            }
            ShellEvent::Done(status) => Some(Msg::Shell(format!("{status} {line}"))),
            _ => None,
        });
    let mut session = Session::new().mouse(true);
    if mode == "cat" {
        session = session.command(["cat"]);
    }
    let status = session.run(overlay).unwrap();
    std::process::exit(status);
}

struct Terminal {
    master: File,
    grid: vt100::Parser,
    pid: i32,
}

impl Terminal {
    fn spawn(mode: &str, shell: &str, home: &std::path::Path) -> Self {
        let exe = CString::new(std::env::current_exe().unwrap().as_os_str().as_bytes()).unwrap();
        let args = [
            exe.clone(),
            CString::new("--exact").unwrap(),
            CString::new("session_child").unwrap(),
            CString::new("--nocapture").unwrap(),
        ];
        let mut argv: Vec<_> = args.iter().map(|a| a.as_ptr()).collect();
        argv.push(std::ptr::null());
        let env = [
            format!("{CHILD}={mode}"),
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
            ws_row: 12,
            ws_col: 60,
            ws_xpixel: 0,
            ws_ypixel: 0,
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
            grid: vt100::Parser::new(12.try_into().unwrap(), 60.try_into().unwrap(), 0),
            pid,
        }
    }

    fn pump(&mut self) {
        let mut buffer = [0; 8192];
        loop {
            match self.master.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(n) => self.grid.process(&buffer[..n]),
            }
        }
    }

    fn contents(&self) -> String {
        self.grid.screen().contents()
    }

    fn wait_for(&mut self, what: &str, test: impl Fn(&str) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            self.pump();
            if test(&self.contents()) {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("timed out waiting for {what}; screen:\n{}", self.contents());
    }

    fn send(&mut self, bytes: &[u8]) {
        self.master.write_all(bytes).unwrap();
    }

    fn row(&self, row: u16) -> String {
        self.grid.screen().contents_between(row, 0, row, 60)
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
        panic!("session did not exit; screen:\n{}", self.contents());
    }
}

fn find(name: &str) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|p| p.is_file())
        .map(|p| p.display().to_string())
}

/// Run a shell under the badge overlay and check its events, clicks and output.
fn check_shell(
    name: &str,
    status: &str,
    configure: impl FnOnce(&std::path::Path),
) -> Option<tempfile::TempDir> {
    let Some(shell) = find(name) else {
        eprintln!("{name} not installed; skipping");
        return None;
    };
    let home = tempfile::tempdir().unwrap();
    configure(home.path());
    let mut term = Terminal::spawn("shell", &shell, home.path());
    term.wait_for("the badge", |s| s.contains("[]"));
    term.wait_for("a prompt", |s| s.contains("$ "));

    term.send(b"echo  hi\r");
    term.wait_for("the command result", |s| s.contains("[0 echo  hi]"));
    // An empty line runs nothing and reports nothing.
    term.send(b"\r");
    term.send(b"false\r");
    term.wait_for("the failure", |s| s.contains("[1 false]"));

    // The badge sits on row 1, right-aligned with one column of margin. Click its `x`.
    let col = term.row(0).find(" x ").expect("button") as u16 + 2;
    term.send(format!("\x1b[<0;{col};1M\x1b[<0;{col};1m").as_bytes());
    term.wait_for("the badge to clear", |s| s.contains("[]"));

    // The click was not typed into the shell, and the shell output is intact.
    term.send(format!("echo still-{status}\r").as_bytes());
    term.wait_for("the echo", |s| s.contains("\nstill-1"));
    assert!(term.contents().contains("\nhi\n"), "{}", term.contents());
    term.send(b"exit\r");
    assert_eq!(term.exit_status(), 0);
    Some(home)
}

#[test]
fn fish_commands_reach_widgets_and_clicks_stay_out_of_the_shell() {
    check_shell("fish", "$status", |home| {
        std::fs::create_dir_all(home.join("fish")).unwrap();
        std::fs::write(
            home.join("fish/config.fish"),
            "set -g fish_greeting\nfunction fish_prompt; echo -n '$ '; end\n",
        )
        .unwrap();
    });
}

#[test]
fn bash_commands_reach_widgets_and_clicks_stay_out_of_the_shell() {
    let home = check_shell("bash", "$?", |home| {
        // The user's own PROMPT_COMMAND still runs, and still sees the last status.
        std::fs::write(
            home.join(".bashrc"),
            "PS1='$ '\nPROMPT_COMMAND='echo \"$?\" > ~/status'\n",
        )
        .unwrap();
    });
    if let Some(home) = home {
        // The user's PROMPT_COMMAND kept running and saw the last command's status.
        let status = std::fs::read_to_string(home.path().join("status")).unwrap();
        assert_eq!(status.trim(), "0");
    }
}

#[test]
fn zsh_commands_reach_widgets_and_clicks_stay_out_of_the_shell() {
    check_shell("zsh", "$?", |home| {
        std::fs::write(home.join(".zshrc"), "PROMPT='$ '\n").unwrap();
    });
}

#[test]
fn widget_input_reaches_the_child_without_submitting_the_line() {
    let home = tempfile::tempdir().unwrap();
    let mut term = Terminal::spawn("widget-input", "/bin/sh", home.path());
    term.wait_for("the button", |s| s.contains("Yes"));
    term.send(b"prefix-\x1b[<0;56;12M\x1b[<0;56;12m");
    term.wait_for("the typed text", |s| s.contains("prefix-yes, continue"));
    term.send(b"-suffix\r");
    term.wait_for("the submitted line", |s| {
        s.contains("<prefix-yes, continue-suffix>")
    });
    assert_eq!(term.exit_status(), 0);
}

#[test]
fn primary_screen_only_widget_hides_and_relinquishes_clicks_in_a_kitty_session() {
    let home = tempfile::tempdir().unwrap();
    let mut term = Terminal::spawn("kitty-screen-switch", "/bin/sh", home.path());
    let click = b"\x1b[<0;56;12M\x1b[<0;56;12m";
    term.wait_for("the primary widget", |s| s.contains("count=0"));
    term.send(click);
    term.wait_for("the changed state", |s| s.contains("count=1"));
    term.send(b"\r");
    // The other layer repainting proves the Kitty alternate-screen draw ran.
    term.wait_for("only the unrestricted layer", |s| {
        s.contains("alt-child") && s.contains("marker") && !s.contains("count=")
    });
    term.send(click);
    term.send(b"\r");
    term.wait_for("the restored primary widget", |s| {
        s.contains("mouse-delivered") && s.contains("count=1")
    });
    term.send(click);
    term.wait_for("the restored hit target", |s| s.contains("count=2"));
    term.send(b"\r");
    assert_eq!(term.exit_status(), 0);
}

#[test]
fn direct_commands_keep_their_output_and_exit_status() {
    let home = tempfile::tempdir().unwrap();
    let mut term = Terminal::spawn("cat", "/bin/sh", home.path());
    term.wait_for("the badge", |s| s.contains("[]"));
    // Text written under the badge's row is put back when the badge moves off it.
    term.send(b"0123456789012345678901234567890123456789012345678901234567\r");
    term.wait_for("the echo", |s| s.matches("0123456789").count() >= 2);
    let first = term.row(0);
    assert!(first.contains("[]"), "{first:?}");
    // End of input: cat exits 0, and the session reports it.
    term.send(b"\x04");
    assert_eq!(term.exit_status(), 0);
}
