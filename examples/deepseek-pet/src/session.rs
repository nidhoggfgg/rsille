//! Local interactive PTY session for an ANSI-compatible terminal.
use rsille::render::raster_output::CellPixels;
use std::{
    ffi::CString,
    fs::File,
    io::{self, Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::ffi::OsStrExt,
    },
    time::{Duration, Instant},
};

fn check(n: libc::c_int) -> io::Result<libc::c_int> {
    if n < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(n)
    }
}

fn size(fd: i32) -> io::Result<libc::winsize> {
    unsafe {
        let mut s = std::mem::zeroed::<libc::winsize>();
        check(libc::ioctl(fd, libc::TIOCGWINSZ, &mut s))?;
        Ok(s)
    }
}

// Primary device attributes advertise Sixel with a standalone parameter 4.
fn advertises_sixel(reply: &[u8]) -> bool {
    let Some(start) = reply.windows(3).rposition(|w| w == b"\x1b[?") else {
        return false;
    };
    let Some(params) = reply[start + 3..]
        .split_inclusive(|&b| b == b'c')
        .next()
        .and_then(|s| s.strip_suffix(b"c"))
    else {
        return false;
    };
    params.split(|&b| b == b';').skip(1).any(|p| p == b"4")
}

fn check_sixel(original: &libc::termios) -> io::Result<()> {
    let mut raw = *original;
    unsafe { libc::cfmakeraw(&mut raw) };
    check(unsafe { libc::tcsetattr(0, libc::TCSANOW, &raw) })?;
    let result = (|| {
        let mut out = io::stdout().lock();
        out.write_all(b"\x1b[c")?;
        out.flush()?;
        let deadline = Instant::now() + Duration::from_millis(500);
        let mut reply = [0u8; 256];
        let mut used = 0;
        let mut poll = libc::pollfd {
            fd: 0,
            events: libc::POLLIN,
            revents: 0,
        };
        while used < reply.len() {
            let ms = deadline
                .saturating_duration_since(Instant::now())
                .as_millis() as i32;
            if ms == 0 || check(unsafe { libc::poll(&mut poll, 1, ms) })? == 0 {
                break;
            }
            let buf = &mut reply[used..];
            let n = unsafe { libc::read(0, buf.as_mut_ptr().cast(), buf.len()) };
            if n < 0 {
                return Err(io::Error::last_os_error());
            }
            if n == 0 {
                break;
            }
            used += n as usize;
            if advertises_sixel(&reply[..used]) {
                return Ok(());
            }
        }
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "未收到声明支持 Sixel 的终端响应",
        ))
    })();
    check(unsafe { libc::tcsetattr(0, libc::TCSANOW, original) })?;
    result
}

struct Terminal(libc::termios);
impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = io::stdout().write_all(b"\x1b[0m\x1b[?1049l");
        let _ = io::stdout().flush();
        unsafe {
            libc::tcsetattr(0, libc::TCSANOW, &self.0);
        }
    }
}

struct Child(i32);
impl Drop for Child {
    fn drop(&mut self) {
        // The master is dropped first (hangup). Don't block on a shell that ignores it.
        unsafe {
            libc::kill(self.0, libc::SIGKILL);
            while libc::waitpid(self.0, std::ptr::null_mut(), 0) < 0
                && io::Error::last_os_error().kind() == io::ErrorKind::Interrupted
            {}
        }
    }
}

/// Requires exclusive use of stdin, stdout and the spawned child's wait status.
pub fn run(mut pet: crate::pet::Pet) -> io::Result<()> {
    let shell = std::env::var_os("SHELL")
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "/bin/sh".into());
    let shell = CString::new(shell.as_bytes())?;
    let args = [shell.as_ptr(), b"-i\0".as_ptr().cast(), std::ptr::null()];
    let mut pixels = CellPixels::resolve(None).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("无法测量终端字符单元的像素尺寸，未启动 shell：{e}"),
        )
    })?;
    let mut s = size(1)?;
    let mut original = unsafe { std::mem::zeroed() };
    check(unsafe { libc::tcgetattr(0, &mut original) })?;
    check_sixel(&original).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("无法确认终端支持 Sixel，未启动 shell：{e}"),
        )
    })?;
    let mut fd = -1;
    let pid =
        check(unsafe { libc::forkpty(&mut fd, std::ptr::null_mut(), &mut original, &mut s) })?;
    if pid == 0 {
        unsafe {
            libc::execv(shell.as_ptr(), args.as_ptr());
            libc::_exit(127);
        }
    }
    let _child = Child(pid);
    let mut master = unsafe { File::from_raw_fd(fd) };
    let flags = check(unsafe { libc::fcntl(fd, libc::F_GETFL) })?;
    check(unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) })?;
    let mut raw = original;
    unsafe {
        libc::cfmakeraw(&mut raw);
    }
    check(unsafe { libc::tcsetattr(0, libc::TCSANOW, &raw) })?;
    let _terminal = Terminal(original);
    let mut out = io::stdout().lock();
    out.write_all(b"\x1b[?1049h\x1b[2J\x1b[H")?;
    let mut pending = std::collections::VecDeque::<u8>::new();
    let mut input = [0u8; 4096];
    let mut output = [0u8; 8192];
    let mut update = Vec::new();
    let mut sprite = Vec::new();
    let mut frame = 0u64;
    let mut next = Instant::now();
    const CAP: usize = 64 * 1024;
    let mut batch = Vec::with_capacity(CAP);
    let mut due = None;
    let mut boundary = crate::boundary::Boundary::default();
    loop {
        let now = Instant::now();
        let tick = now >= next;
        if tick {
            let new = size(1)?;
            if (s.ws_col, s.ws_row, s.ws_xpixel, s.ws_ypixel)
                != (new.ws_col, new.ws_row, new.ws_xpixel, new.ws_ypixel)
            {
                check(unsafe { libc::ioctl(master.as_raw_fd(), libc::TIOCSWINSZ, &new) })?;
                s = new;
                if let Some(measured) =
                    CellPixels::from_terminal().and_then(|p| CellPixels::resolve(Some(p)).ok())
                {
                    pixels = measured;
                }
                pet.invalidate();
            }
            frame = frame.wrapping_add(1);
            next = now + Duration::from_millis(100);
        }
        if tick || due.is_some_and(|d| now >= d) || batch.len() == CAP {
            let start = boundary.ready();
            boundary.feed(&batch);
            let end = boundary.ready();
            update.clear();
            if start && !batch.is_empty() {
                pet.hide(&mut update);
            }
            sprite.clear();
            if end {
                pet.draw(
                    frame.wrapping_sub(1),
                    s.ws_col,
                    s.ws_row,
                    pixels.width,
                    pixels.height,
                    &mut sprite,
                );
            }
            render_batch(&mut out, &update, &batch, &sprite, start && end)?;
            batch.clear();
            due = None;
        }
        out.flush()?;
        let reading = pending.is_empty();
        let mut fds = [
            libc::pollfd {
                fd: if reading { 0 } else { -1 },
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd,
                events: libc::POLLIN | if reading { 0 } else { libc::POLLOUT },
                revents: 0,
            },
        ];
        let ms = due
            .unwrap_or(next)
            .min(next)
            .saturating_duration_since(Instant::now())
            .as_nanos()
            .div_ceil(1_000_000) as i32;
        if let Err(e) = check(unsafe { libc::poll(fds.as_mut_ptr(), 2, ms) }) {
            if e.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(e);
        }
        if fds[1].revents & (libc::POLLIN | libc::POLLHUP | libc::POLLERR) != 0 {
            let room = output.len().min(CAP - batch.len());
            match master.read(&mut output[..room]) {
                Ok(0) => break,
                Ok(n) => {
                    batch.extend_from_slice(&output[..n]);
                    due.get_or_insert_with(|| Instant::now() + Duration::from_millis(20));
                }
                Err(e) if e.raw_os_error() == Some(libc::EIO) => break,
                Err(e) if retry(&e) => {}
                Err(e) => return Err(e),
            }
        }
        if fds[1].revents & libc::POLLOUT != 0 {
            match master.write(pending.as_slices().0) {
                Ok(n) => {
                    pending.drain(..n);
                }
                Err(e) if retry(&e) => {}
                Err(e) => return Err(e),
            }
        }
        if fds[0].revents != 0 {
            let n = unsafe { libc::read(0, input.as_mut_ptr().cast(), input.len()) };
            if n == 0 {
                break;
            }
            if n < 0 {
                let e = io::Error::last_os_error();
                if retry(&e) {
                    continue;
                }
                return Err(e);
            }
            let bytes = &input[..n as usize];
            if bytes.contains(&0x1d) {
                break;
            }
            pending.extend(bytes);
        }
    }
    update.clear();
    if !batch.is_empty() && boundary.ready() {
        pet.hide(&mut update);
    }
    boundary.feed(&batch);
    render_batch(&mut out, &update, &batch, &[], boundary.ready())?;
    if !boundary.ready() {
        out.write_all(b"\x18")?; // Cancel unfinished shell output before terminal teardown.
    }
    out.flush()
}

fn render_batch(
    out: &mut impl Write,
    erase: &[u8],
    bytes: &[u8],
    sprite: &[u8],
    complete: bool,
) -> io::Result<()> {
    if !complete {
        write_update(out, erase)?;
        out.write_all(bytes)?;
        return write_update(out, sprite);
    }
    if erase.is_empty() && sprite.is_empty() {
        return out.write_all(bytes);
    }
    out.write_all(b"\x1b[?2026h")?;
    cursor_update(out, erase)?;
    out.write_all(bytes)?;
    cursor_update(out, sprite)?;
    out.write_all(b"\x1b[?2026l")
}

fn retry(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock
    )
}

fn write_update(out: &mut impl Write, update: &[u8]) -> io::Result<()> {
    if !update.is_empty() {
        out.write_all(b"\x1b[?2026h")?;
        cursor_update(out, update)?;
        out.write_all(b"\x1b[?2026l")?;
    }
    Ok(())
}

fn cursor_update(out: &mut impl Write, update: &[u8]) -> io::Result<()> {
    if !update.is_empty() {
        out.write_all(b"\x1b7")?;
        out.write_all(update)?;
        out.write_all(b"\x1b8")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_sixel_in_a_complete_device_attributes_reply() {
        assert!(advertises_sixel(b"\x1b[?62;4;22c"));
        assert!(!advertises_sixel(b"\x1b[?62;4"));
        assert!(!advertises_sixel(b"\x1b[?4;42c"));
    }

    #[test]
    fn editing_and_scrolling_share_one_transaction() {
        let mut out = Vec::new();
        render_batch(&mut out, b"erase", b"\x1b[K\r\n", b"draw", true).unwrap();
        assert_eq!(
            out,
            b"\x1b[?2026h\x1b7erase\x1b8\x1b[K\r\n\x1b7draw\x1b8\x1b[?2026l"
        );
    }

    #[test]
    fn split_sequences_are_contiguous_between_updates() {
        for (a, b) in [
            (b"\x1b[".as_slice(), b"K".as_slice()),
            (&[0xe9][..], &[0xb2, 0xb8][..]),
        ] {
            let mut out = Vec::new();
            render_batch(&mut out, b"erase", a, b"", false).unwrap();
            render_batch(&mut out, b"", b, b"draw", false).unwrap();
            let raw = [b"\x1b[?2026l".as_slice(), a, b, b"\x1b[?2026h"].concat();
            assert!(out.windows(raw.len()).any(|w| w == raw));
        }
    }

    #[test]
    fn wraps_raw_update_but_skips_empty_frame() {
        let mut out = Vec::new();
        write_update(&mut out, b"").unwrap();
        assert!(out.is_empty());
        write_update(&mut out, b"\x1bP\xff\x1b\\").unwrap();
        assert_eq!(out, b"\x1b[?2026h\x1b7\x1bP\xff\x1b\\\x1b8\x1b[?2026l");
    }
}
