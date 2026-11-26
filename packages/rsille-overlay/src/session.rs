//! Local interactive PTY session for an ANSI-compatible terminal.
use crate::{
    boundary::Boundary,
    mouse::{Buttons, Mouse},
    shell::{self, Hooks},
    Attach, CellPixels, Context, Graphics, InlineRasterSession, Overlay, Pointer, Screen, Session,
};
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

const CLEAR_VIEWPORT: &[u8] = b"\x1b[2J\x1b[H";
/// Overlays redraw at least this often while visible. Resizes are noticed per tick.
const TICK: Duration = Duration::from_millis(100);

struct Terminal {
    cleanup: Vec<u8>,
    original: libc::termios,
    alt_screen: bool,
    child_alternate: bool,
}
impl Drop for Terminal {
    fn drop(&mut self) {
        // The detach key can also interrupt a child application that enabled mouse tracking.
        let _ =
            io::stdout().write_all(b"\x18\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[?1016l");
        let mut out = io::stdout().lock();
        let _ = out.write_all(&self.cleanup);
        let _ = out.write_all(b"\x1b[0m");
        if self.alt_screen || self.child_alternate {
            let _ = out.write_all(b"\x1b[?1049l");
        }
        let _ = out.flush();
        unsafe {
            libc::tcsetattr(0, libc::TCSANOW, &self.original);
        }
    }
}

struct Child(i32);

impl Child {
    fn wait(&mut self) -> io::Result<i32> {
        let mut status = 0;
        loop {
            if unsafe { libc::waitpid(self.0, &mut status, 0) } >= 0 {
                self.0 = -1;
                return Ok(if libc::WIFEXITED(status) {
                    libc::WEXITSTATUS(status)
                } else {
                    128 + libc::WTERMSIG(status)
                });
            }
            let error = io::Error::last_os_error();
            if error.kind() != io::ErrorKind::Interrupted {
                return Err(error);
            }
        }
    }
}
impl Drop for Child {
    fn drop(&mut self) {
        if self.0 < 0 {
            return;
        }
        // The master is dropped first (hangup). Don't block on a shell that ignores it.
        unsafe {
            libc::kill(self.0, libc::SIGKILL);
            while libc::waitpid(self.0, std::ptr::null_mut(), 0) < 0
                && io::Error::last_os_error().kind() == io::ErrorKind::Interrupted
            {}
        }
    }
}

fn measured_pixels() -> Option<CellPixels> {
    CellPixels::from_terminal().and_then(|p| CellPixels::resolve(Some(p)).ok())
}

/// Requires exclusive use of stdin, stdout and the spawned child's wait status.
pub fn run<O: Overlay + ?Sized>(options: Session, overlay: &mut O) -> io::Result<i32> {
    let direct = !options.command.is_empty();
    let program = options
        .command
        .first()
        .cloned()
        .unwrap_or_else(|| std::env::var_os("SHELL").unwrap_or_else(|| "/bin/sh".into()));
    if program.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "empty program path",
        ));
    }
    let hooks = if !direct && options.shell_events {
        Hooks::prepare(program.as_ref())?
    } else {
        None
    };
    let environment = hooks
        .as_ref()
        .and_then(|(hooks, _)| hooks.environment.as_ref())
        .map(|environment| {
            environment
                .iter()
                .map(|v| v.as_ptr())
                .chain(std::iter::once(std::ptr::null()))
                .collect::<Vec<_>>()
        });
    let program = CString::new(program.as_bytes())?;
    let arguments = options
        .command
        .iter()
        .skip(1)
        .map(|arg| CString::new(arg.as_bytes()))
        .collect::<Result<Vec<_>, _>>()?;
    let mut args = vec![program.as_ptr()];
    if direct {
        args.extend(arguments.iter().map(|arg| arg.as_ptr()));
    } else {
        if let Some((hooks, _)) = &hooks {
            args.extend(hooks.args.iter().map(|arg| arg.as_ptr()));
        }
        args.push(c"-i".as_ptr());
    }
    args.push(std::ptr::null());
    // Prepared before forking: the child only calls async-signal-safe functions.
    let mut exec_error = format!("{}: cannot execute ", options.name).into_bytes();
    exec_error.extend_from_slice(program.as_bytes());
    exec_error.extend_from_slice(b"\r\n");

    let raster = match (options.raster, options.backend) {
        (Some(session), _) => Some(session),
        (None, Some(backend)) => Some(
            backend
                .negotiate_inline(None, Duration::from_millis(500))
                .map_err(|e| {
                    io::Error::new(e.kind(), format!("cannot start terminal graphics: {e}"))
                })?,
        ),
        (None, None) => None,
    };
    let (graphics, mut pixels) = match raster {
        None => (
            Graphics::Text,
            measured_pixels().unwrap_or(CellPixels {
                width: 0,
                height: 0,
            }),
        ),
        Some(InlineRasterSession::Kitty(p)) => (Graphics::Kitty, p),
        Some(InlineRasterSession::Sixel(p)) => (Graphics::Sixel, p),
        Some(_) => {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "unsupported inline graphics session",
            ))
        }
    };
    // Kitty placements float above the text, so the pointer can be shared and
    // the overlay can stay on the child's alternate screen.
    let shared = graphics == Graphics::Kitty;
    let mut s = size(1)?;
    let epoch = Instant::now();
    overlay.attach(&Attach {
        epoch,
        graphics,
        cell: pixels,
        cols: s.ws_col,
        rows: s.ws_row,
        shell: hooks.as_ref().map(|(hooks, _)| hooks.shell),
    })?;
    let mut cleanup = Vec::new();
    overlay.cleanup(&mut cleanup);

    let mut original = unsafe { std::mem::zeroed() };
    check(unsafe { libc::tcgetattr(0, &mut original) })?;
    let mut fd = -1;
    let pid = check(unsafe { libc::forkpty(&mut fd, std::ptr::null_mut(), &original, &s) })?;
    if pid == 0 {
        unsafe {
            if let Some((_, sender)) = &hooks {
                if libc::fcntl(sender.as_raw_fd(), libc::F_SETFD, 0) < 0 {
                    libc::_exit(127);
                }
            }
            if let Some(env) = &environment {
                libc::execve(program.as_ptr(), args.as_ptr(), env.as_ptr());
            } else if direct {
                libc::execvp(program.as_ptr(), args.as_ptr());
            } else {
                libc::execv(program.as_ptr(), args.as_ptr());
            }
            libc::write(2, exec_error.as_ptr().cast(), exec_error.len());
            libc::_exit(127);
        }
    }
    let mut child = Child(pid);
    // Drop the parent's copy of the sending socket.
    let hooks = hooks.map(|(hooks, _)| hooks);
    let mut master = unsafe { File::from_raw_fd(fd) };
    let flags = check(unsafe { libc::fcntl(fd, libc::F_GETFL) })?;
    check(unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) })?;
    let mut raw = original;
    unsafe {
        libc::cfmakeraw(&mut raw);
    }
    check(unsafe { libc::tcsetattr(0, libc::TCSANOW, &raw) })?;
    let mut terminal = Terminal {
        cleanup,
        original,
        alt_screen: options.alt_screen,
        child_alternate: false,
    };
    let capture_mouse = options.mouse;
    let mut decoder = Mouse::default();
    let mut buttons = Buttons::default();
    let mut input_deadline = None;
    let mut screen = if options.restore_text {
        Screen::new(s.ws_row, s.ws_col, (1, 1))
    } else {
        None
    };
    macro_rules! cx {
        ($now:expr, $alternate:expr) => {
            Context {
                now: $now,
                epoch,
                cols: s.ws_col,
                rows: s.ws_row,
                cell: pixels,
                graphics,
                alternate: $alternate,
                screen: &mut screen,
            }
        };
    }
    let mut out = io::stdout().lock();
    if options.alt_screen {
        out.write_all(b"\x1b[?1049h")?;
    }
    out.write_all(CLEAR_VIEWPORT)?;
    if capture_mouse {
        out.write_all(b"\x1b[?1006h\x1b[?1003h")?;
    }
    let mut pending = std::collections::VecDeque::<u8>::new();
    let mut delivered = Vec::new();
    let mut input = [0u8; 4096];
    let mut output = [0u8; 8192];
    let mut event = vec![0u8; shell::MAX_EVENT];
    let mut forwarded = Vec::new();
    let mut update = Vec::new();
    let mut sprite = Vec::new();
    let mut last_tick = None;
    const CAP: usize = 64 * 1024;
    let mut batch = Vec::with_capacity(CAP);
    let mut due = None;
    let mut boundary = Boundary::default();
    let mut child_eof = false;
    loop {
        let now = Instant::now();
        if input_deadline.is_some_and(|d| now >= d) {
            decoder.flush(&mut pending);
            input_deadline = None;
        }
        if let Some(hooks) = &hooks {
            for _ in 0..64 {
                match hooks.events.recv(&mut event) {
                    Ok(n) => {
                        if let Some(event) = shell::parse(&event[..n]) {
                            overlay.shell(event);
                        }
                    }
                    Err(e) if retry(&e) => break,
                    Err(e) => return Err(e),
                }
            }
        }
        let tick_index = now.duration_since(epoch).as_nanos() / TICK.as_nanos();
        let tick = last_tick != Some(tick_index);
        if tick {
            let new = size(1)?;
            if (s.ws_col, s.ws_row, s.ws_xpixel, s.ws_ypixel)
                != (new.ws_col, new.ws_row, new.ws_xpixel, new.ws_ypixel)
            {
                check(unsafe { libc::ioctl(master.as_raw_fd(), libc::TIOCSWINSZ, &new) })?;
                s = new;
                if let Some(measured) = measured_pixels() {
                    pixels = measured;
                }
                boundary.resized();
                if screen
                    .as_mut()
                    .is_some_and(|screen| !screen.resize(s.ws_row, s.ws_col))
                {
                    screen = None;
                }
                overlay.resized(s.ws_col, s.ws_row);
                overlay.invalidate();
            }
            last_tick = Some(tick_index);
        }
        // A wake-up may arrive between ticks, but never splits child output.
        let mut wake = None;
        if batch.is_empty() && boundary.can_draw() && (!boundary.alternate || shared) {
            wake = overlay.wake_at(now);
        }
        let woken = wake.is_some_and(|w| now >= w);
        if tick || woken || due.is_some_and(|d| now >= d) || batch.len() == CAP {
            let start = boundary.can_draw();
            let was_alternate = boundary.alternate;
            let was_child_mouse = boundary.child_mouse();
            forwarded.clear();
            if relay_modes(
                &mut boundary,
                &mut batch,
                capture_mouse,
                shared,
                &mut forwarded,
            ) {
                overlay.bell();
            }
            if !shared
                && (boundary.screen_selected
                    || (!boundary.alternate && was_child_mouse != boundary.child_mouse()))
            {
                buttons = Default::default();
                decoder.flush(&mut pending);
                input_deadline = None;
                overlay.pointer_reset();
            }
            let end = boundary.can_draw();
            update.clear();
            if !batch.is_empty() {
                if start && (!was_alternate || boundary.screen_selected) {
                    overlay.hide(&mut cx!(now, was_alternate), &mut update);
                }
                if was_alternate {
                    // Scrolling, clearing or plugin deletes may remove or move a placement.
                    overlay.invalidate();
                }
                if let Some(screen) = &mut screen {
                    screen.feed(&batch);
                }
                overlay.output(&batch, &mut cx!(now, boundary.alternate));
            }
            sprite.clear();
            terminal.child_alternate = boundary.alternate;
            // Forward the screen switch first. Resume drawing on a later batch.
            // A full batch can be followed immediately by more child output.
            // During a flood, cap repaints at the tick rate.
            if end
                && (!boundary.alternate || shared)
                && !boundary.screen_selected
                && (tick
                    || (woken && batch.is_empty())
                    || (!boundary.alternate && batch.len() < CAP))
            {
                overlay.draw(&mut cx!(now, boundary.alternate), &mut sprite)?;
            }
            render_batch(
                &mut out,
                &update,
                &forwarded,
                &sprite,
                start && end && !boundary.screen_selected,
            )?;
            batch.clear();
            due = None;
            wake = None;
            if boundary.can_draw() && (!boundary.alternate || shared) {
                wake = overlay.wake_at(Instant::now());
            }
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
        let next_tick = epoch + TICK * (tick_index as u32 + 1);
        let ms = [due, wake, input_deadline]
            .into_iter()
            .flatten()
            .fold(next_tick, Instant::min)
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
                Ok(0) => {
                    child_eof = true;
                    break;
                }
                Ok(n) => {
                    batch.extend_from_slice(&output[..n]);
                    due.get_or_insert_with(|| Instant::now() + Duration::from_millis(20));
                }
                Err(e) if e.raw_os_error() == Some(libc::EIO) => {
                    child_eof = true;
                    break;
                }
                Err(e) if retry(&e) => {}
                Err(e) => return Err(e),
            }
        }
        if fds[1].revents & libc::POLLOUT != 0 {
            match master.write(pending.as_slices().0) {
                Ok(n) => {
                    delivered.clear();
                    delivered.extend(pending.drain(..n));
                    if !delivered.is_empty() {
                        overlay.input(&delivered);
                    }
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
            if options.detach_key.is_some_and(|key| bytes.contains(&key)) {
                break;
            }
            if capture_mouse && (shared || !boundary.child_mouse()) {
                let (cols, rows) = (s.ws_col, s.ws_row);
                decoder.feed(bytes, &mut pending, |report, out| {
                    let pointer = |col, row, pixel| Pointer {
                        kind: report.kind(),
                        col,
                        row,
                        pixel,
                        button: report.button,
                    };
                    if shared {
                        let modes = boundary.mouse;
                        let (col, row) = report.cell(modes.encoding, (pixels.width, pixels.height));
                        let pixel = (modes.encoding == 1016).then_some((report.x, report.y));
                        let p = pointer(col, row, pixel);
                        if buttons.overlay_owns(report, overlay.hit(&p)) {
                            overlay.pointer(p);
                        } else {
                            if report.is_hover() {
                                overlay.pointer(p);
                            } else {
                                overlay.pointer_passed(p);
                            }
                            report.forward(modes, out);
                        }
                    } else if report.is_hover() && (report.x > cols || report.y > rows) {
                        overlay.pointer(pointer(report.x, report.y, None));
                    } else if cols > 1 && rows > 0 {
                        overlay.pointer(pointer(report.x.min(cols - 1), report.y.min(rows), None));
                    }
                });
                input_deadline = decoder
                    .has_partial_report()
                    .then(|| Instant::now() + Duration::from_millis(100));
            } else {
                pending.extend(bytes);
            }
        }
    }
    update.clear();
    let start = boundary.can_draw();
    if start {
        overlay.hide(&mut cx!(Instant::now(), boundary.alternate), &mut update);
    }
    forwarded.clear();
    relay_modes(&mut boundary, &mut batch, false, shared, &mut forwarded);
    terminal.child_alternate = boundary.alternate;
    render_batch(
        &mut out,
        &update,
        &forwarded,
        &[],
        start && boundary.can_draw() && !boundary.screen_selected,
    )?;
    if !boundary.ready() {
        out.write_all(b"\x18")?; // Cancel unfinished child output before terminal teardown.
    }
    out.flush()?;
    drop(terminal);
    if direct && child_eof {
        child.wait()
    } else {
        Ok(0)
    }
}

// Relay the child output, inserting mouse handoffs and resize clears at their
// exact terminal sequence boundaries, even when a sequence spans two reads.
fn relay_modes(
    boundary: &mut crate::boundary::Boundary,
    bytes: &mut Vec<u8>,
    capture: bool,
    kitty: bool,
    out: &mut Vec<u8>,
) -> bool {
    let mut start = 0;
    let mut was_child = boundary.child_mouse();
    let mut clears = Vec::new();
    let bell = boundary.feed_with(bytes, |end, state, event| {
        if matches!(event, crate::boundary::Event::Clear) {
            out.extend_from_slice(&bytes[start..end]);
            out.extend_from_slice(CLEAR_VIEWPORT);
            start = end;
            clears.push(end);
            return;
        }
        if capture && (kitty || !state.child_mouse() || !was_child) {
            let (tracking, encoding) = if kitty {
                let encoding = if state.mouse.encoding == 1016 {
                    1016
                } else {
                    1006
                };
                (1003, encoding)
            } else if state.child_mouse() {
                (state.mouse.tracking, state.mouse.encoding)
            } else {
                (1003, 1006)
            };
            out.extend_from_slice(&bytes[start..end]);
            start = end;

            out.extend_from_slice(b"\x1b[?1003l\x1b[?1006l");
            for mode in [encoding, tracking] {
                if mode != 0 {
                    write!(out, "\x1b[?{mode}h").expect("Vec write");
                }
            }
        }
        was_child = state.child_mouse();
    });
    out.extend_from_slice(&bytes[start..]);
    // Replay the same clears in the text and forage models, in PTY output order.
    for offset in clears.into_iter().rev() {
        bytes.splice(offset..offset, CLEAR_VIEWPORT.iter().copied());
    }
    bell
}

fn render_batch(
    out: &mut impl Write,
    erase: &[u8],
    bytes: &[u8],
    sprite: &[u8],
    group_update: bool,
) -> io::Result<()> {
    if !group_update {
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
    fn kitty_tracks_hover_across_screen_switches_and_preserves_child_encoding() {
        let mut b = crate::boundary::Boundary::default();
        let mut out = Vec::new();
        relay_modes(&mut b, &mut b"\x1b[?1049h".to_vec(), true, true, &mut out);
        assert!(out.ends_with(b"\x1b[?1006h\x1b[?1003h"));
        assert_eq!(b.mouse.tracking, 0);
        relay_modes(
            &mut b,
            &mut b"\x1b[?1000h\x1b[?1016h".to_vec(),
            true,
            true,
            &mut out,
        );
        assert!(out.ends_with(b"\x1b[?1016h\x1b[?1003h"));
        relay_modes(
            &mut b,
            &mut b"\x1b[?1016l\x1b[?1049l".to_vec(),
            true,
            true,
            &mut out,
        );
        assert!(out.ends_with(b"\x1b[?1006h\x1b[?1003h"));
        assert_eq!(b.mouse.tracking, 1000);
    }

    #[test]
    fn mouse_handoff_survives_split_switches_and_vims_teardown_order() {
        let mut b = crate::boundary::Boundary::default();
        let mut out = Vec::new();
        for byte in b"\x1b[?1049h\x1b[?1002h\x1b[?1006h" {
            relay_modes(&mut b, &mut vec![*byte], true, false, &mut out);
        }
        assert!(b.child_mouse());
        assert_eq!(
            out,
            b"\x1b[?1049h\x1b[?1003l\x1b[?1006l\x1b[?1002h\x1b[?1006h"
        );
        out.clear();
        relay_modes(
            &mut b,
            &mut b"\x1b[?1049l\x1b[?1002l\x1b[?1006l".to_vec(),
            true,
            false,
            &mut out,
        );
        assert!(!b.child_mouse());
        assert!(out.ends_with(b"\x1b[?1006h\x1b[?1003h"));
    }

    #[test]
    fn resize_clears_after_split_output_and_before_new_text_on_each_screen() {
        let mut boundary = crate::boundary::Boundary::default();
        boundary.feed(b"primary\x1b[?1049halt\x1b[3");
        boundary.resized();
        let mut batch = b"1mnew alt\x1b[?1049lnew primary".to_vec();
        let mut out = Vec::new();
        relay_modes(&mut boundary, &mut batch, false, false, &mut out);
        assert_eq!(
            out,
            b"1m\x1b[2J\x1b[Hnew alt\x1b[?1049l\x1b[2J\x1b[Hnew primary"
        );
        assert_eq!(batch, out);
    }

    #[test]
    fn idle_resize_clears_once_even_without_child_output() {
        let mut boundary = crate::boundary::Boundary::default();
        boundary.resized();
        let mut out = Vec::new();
        for _ in 0..2 {
            relay_modes(&mut boundary, &mut Vec::new(), false, false, &mut out);
        }
        assert_eq!(out, CLEAR_VIEWPORT);
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
