//! Drive the real session through an outer ConPTY, without a desktop or stdin.
use super::*;
use crate::{Context, MouseKind, Overlay, Pointer, Session};
use std::{ffi::OsStr, time::Instant};

struct Badge(bool);
impl Overlay for Badge {
    fn draw(&mut self, cx: &mut Context<'_>, out: &mut Vec<u8>) -> io::Result<()> {
        write!(out, "\x1b[1;1Hbadge-{}-{}", cx.cols, u8::from(self.0))
    }
    fn hide(&mut self, _: &mut Context<'_>, _: &mut Vec<u8>) {}
    fn hit(&self, p: &Pointer) -> bool {
        p.row == 1 && p.col == 1
    }
    fn pointer(&mut self, p: Pointer) {
        self.0 |= p.kind == MouseKind::LeftPress;
    }
}

#[test]
#[ignore = "child fixture launched inside the test pseudoconsole"]
fn session_child() {
    let status = Session::new()
        .command(["cmd.exe", "/D", "/Q"])
        .mouse(true)
        .run(Badge(false))
        .unwrap();
    std::process::exit(status);
}

#[test]
fn nested_session_click_resize_and_exit() {
    let exe = std::env::current_exe().unwrap();
    let args = [
        "--exact",
        "sys::windows::tests::session_child",
        "--ignored",
        "--nocapture",
    ]
    .map(OsString::from);
    let mut pty = Pty::spawn(&Spawn {
        program: OsStr::new(&exe),
        args: &args,
        size: Size { cols: 80, rows: 24 },
    })
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut screen = vt100::Parser::default();
    let mut step = 0;
    let mut pending: &[u8] = &[];
    let mut buf = [0; 8192];
    loop {
        assert!(
            Instant::now() < deadline,
            "stalled at step {step}: {}",
            screen.screen().contents()
        );
        let ready = pty
            .poll(false, !pending.is_empty(), Duration::from_millis(100))
            .unwrap();
        if ready.writable {
            match pty.write_input(pending) {
                Ok(n) => pending = &pending[n..],
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(e) => panic!("{e}"),
            }
        }
        if ready.output {
            let n = pty.read_output(&mut buf).unwrap();
            if n == 0 {
                break;
            }
            screen.process(&buf[..n]);
            let contents = screen.screen().contents();
            match step {
                0 if contents.contains("badge-80-0") => {
                    pending = b"\x1b[<0;1;1M\x1b[<0;1;1m";
                    step += 1;
                }
                1 if contents.contains("badge-80-1") => {
                    pty.resize(Size { cols: 78, rows: 20 }).unwrap();
                    step += 1;
                }
                2 if contents.contains("badge-78-1") => {
                    pending = b"exit 37\r";
                    step += 1;
                }
                _ => {}
            }
        }
    }
    assert_eq!(step, 3);
    assert_eq!(pty.wait().unwrap(), 37);
}
