//! Local interactive PTY session for an ANSI-compatible terminal.
use crate::deepseek_pet::Feature;
use rsille::render::raster_output::{CellPixels, InlineRasterSession};
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

fn cursor_position(reply: &[u8]) -> Option<(u16, u16)> {
    reply
        .windows(2)
        .enumerate()
        .filter(|(_, prefix)| *prefix == b"\x1b[")
        .find_map(|(i, _)| {
            let fields = reply[i + 2..]
                .split_inclusive(|&b| b == b'R')
                .next()?
                .strip_suffix(b"R")?;
            let (row, col) = std::str::from_utf8(fields).ok()?.split_once(';')?;
            let (row, col) = (row.parse().ok()?, col.parse().ok()?);
            (row > 0 && col > 0).then_some((col, row))
        })
}

fn probe_cursor(original: &libc::termios) -> io::Result<Option<(u16, u16)>> {
    let mut raw = *original;
    unsafe { libc::cfmakeraw(&mut raw) };
    check(unsafe { libc::tcsetattr(0, libc::TCSANOW, &raw) })?;
    let result = (|| {
        let mut out = io::stdout().lock();
        out.write_all(b"\x1b[6n")?;
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
            if let Some(cursor) = cursor_position(&reply[..used]) {
                return Ok(Some(cursor));
            }
        }
        Ok(None)
    })();
    check(unsafe { libc::tcsetattr(0, libc::TCSANOW, original) })?;
    result
}

struct Terminal<'a> {
    kitty: Option<&'a crate::deepseek_pet::pet::KittySprites>,
    original: libc::termios,
    alt_screen: bool,
    child_alternate: bool,
}
impl Drop for Terminal<'_> {
    fn drop(&mut self) {
        // Ctrl+] can also interrupt a child application that enabled mouse tracking.
        let _ =
            io::stdout().write_all(b"\x18\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[?1016l");
        let mut out = io::stdout().lock();
        if let Some(kitty) = self.kitty {
            let _ = kitty.whale.delete(&mut out);
            let _ = kitty.fish.delete(&mut out);
        }
        let _ = out.write_all(b"\x1b[0m");
        if self.alt_screen || self.child_alternate {
            let _ = out.write_all(b"\x1b[?1049l");
        }
        let _ = io::stdout().flush();
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

/// Requires exclusive use of stdin, stdout and the spawned child's wait status.
pub fn run(options: crate::deepseek_pet::Options) -> io::Result<i32> {
    let direct = !options.command.is_empty();
    let program = options
        .command
        .first()
        .cloned()
        .unwrap_or_else(|| std::env::var_os("SHELL").unwrap_or_else(|| "/bin/sh".into()));
    if program.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "程序路径为空"));
    }
    let features = options.enabled_features();
    let command_events = !direct
        && (features.contains(&Feature::CommandReactions) || features.contains(&Feature::Speech));
    let forage = features.contains(&Feature::Forage);
    let capture_mouse = features.contains(&Feature::Mouse);
    let catalog = if command_events || options.commands.is_some() {
        crate::deepseek_pet::dialogue::Catalog::load(options.commands.as_deref())?
    } else {
        Default::default()
    };
    let mut pet = crate::deepseek_pet::pet::Pet::new(&catalog, features);
    let hooks = if command_events {
        crate::deepseek_pet::commands::Hooks::prepare(program.as_ref(), &catalog)?
    } else {
        None
    };
    let environment = hooks
        .as_ref()
        .filter(|(hooks, _)| hooks.fish_init.is_none())
        .map(|(hooks, _)| {
            hooks
                .environment
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
        args.push(b"-i\0".as_ptr().cast());
    }
    if let Some(init) = hooks
        .as_ref()
        .and_then(|(hooks, _)| hooks.fish_init.as_ref())
    {
        args.extend([b"-C\0".as_ptr().cast(), init.as_ptr()]);
    }
    args.push(std::ptr::null());
    let session = options
        .backend
        .negotiate_inline(None, Duration::from_millis(500))
        .map_err(|e| {
            io::Error::new(
                e.kind(),
                format!("无法确认终端图形协议或字符格像素尺寸，未启动子进程：{e}"),
            )
        })?;
    let mut pixels = session.cell_pixels();
    let kitty = if matches!(session, InlineRasterSession::Kitty(_)) {
        use rsille::render::raster_output::KittyImage;
        Some(crate::deepseek_pet::pet::KittySprites {
            whale: KittyImage::new()?,
            fish: KittyImage::new()?,
        })
    } else {
        None
    };
    pet.kitty = kitty.as_ref();
    let mut s = size(1)?;
    let mut original = unsafe { std::mem::zeroed() };
    check(unsafe { libc::tcgetattr(0, &mut original) })?;
    let cursor = if (forage || !options.no_restore) && !options.alt_screen {
        probe_cursor(&original)?
    } else {
        None
    };
    let mut fd = -1;
    let pid =
        check(unsafe { libc::forkpty(&mut fd, std::ptr::null_mut(), &mut original, &mut s) })?;
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
            let error = b"rsille pet: cannot execute ";
            libc::write(2, error.as_ptr().cast(), error.len());
            libc::write(2, program.as_ptr().cast(), program.as_bytes().len());
            libc::write(2, b"\r\n".as_ptr().cast(), 2);
            libc::_exit(127);
        }
    }
    let mut child = Child(pid);
    let mut preview = hooks
        .as_ref()
        .filter(|(hooks, _)| hooks.fish_init.is_some())
        .map(|_| crate::deepseek_pet::commands::FishPreview::default());
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
        kitty: kitty.as_ref(),
        original,
        alt_screen: options.alt_screen,
        child_alternate: false,
    };
    let mut decoder = crate::deepseek_pet::mouse::Mouse::default();
    let mut buttons = crate::deepseek_pet::mouse::Buttons::default();
    let mut input_deadline = None;
    let cursor = if options.alt_screen {
        Some((1, 1))
    } else {
        cursor
    }
    .filter(|&(col, row)| col <= s.ws_col && row <= s.ws_row);
    if !options.no_restore {
        pet.screen = cursor
            .and_then(|pos| crate::deepseek_pet::screen::Screen::new(s.ws_row, s.ws_col, pos));
    }
    let mut food = forage.then(|| crate::deepseek_pet::food::Food::new(cursor));
    let mut food_deadline = None;
    let mut out = io::stdout().lock();
    if options.alt_screen {
        out.write_all(b"\x1b[?1049h\x1b[2J\x1b[H")?;
    }
    if capture_mouse {
        out.write_all(b"\x1b[?1006h\x1b[?1003h")?;
    }
    let mut pending = std::collections::VecDeque::<u8>::new();
    let mut input = [0u8; 4096];
    let mut output = [0u8; 8192];
    let mut forwarded = Vec::new();
    let mut update = Vec::new();
    let mut sprite = Vec::new();
    let mut frame = 0u64;
    let mut next = Instant::now();
    const CAP: usize = 64 * 1024;
    let mut batch = Vec::with_capacity(CAP);
    let mut due = None;
    let mut boundary = crate::deepseek_pet::boundary::Boundary::default();
    let mut child_eof = false;
    loop {
        let now = Instant::now();
        if input_deadline.is_some_and(|d| now >= d) {
            decoder.flush(&mut pending);
            if !pending.is_empty() {
                pet.notice_input();
            }
            input_deadline = None;
        }
        if food_deadline.is_some_and(|d| now >= d) {
            pet.forget_food();
            food_deadline = None;
        }
        if let Some(hooks) = &hooks {
            let mut event = [0u8; 32];
            for _ in 0..64 {
                match hooks.events.recv(&mut event) {
                    Ok(2) if &event[..2] == b"p0" => {
                        if let Some(preview) = &mut preview {
                            preview.prompt();
                        }
                    }
                    Ok(n) if (2..event.len()).contains(&n) => {
                        let Some(id) = std::str::from_utf8(&event[1..n])
                            .ok()
                            .and_then(|s| s.parse::<usize>().ok())
                        else {
                            continue;
                        };
                        if event[0] == b'r' && preview.as_mut().is_some_and(|p| p.end()) {
                            pet.notice_command(b'e', 0);
                        }
                        pet.notice_command(event[0], id);
                    }
                    Ok(_) => continue,
                    Err(e) if retry(&e) => break,
                    Err(e) => return Err(e),
                }
            }
        }
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
                if let Some(food) = &mut food {
                    food.resize();
                }
                if let Some(screen) = &mut pet.screen {
                    if !screen.resize(s.ws_row, s.ws_col) {
                        pet.screen = None;
                    }
                }
                pet.forget_food();
                food_deadline = None;
                pet.invalidate();
            }
            frame = frame.wrapping_add(1);
            next = now + Duration::from_millis(100);
        }
        // A pointer update may wake us between animation ticks, but never split shell output.
        let mut fish_due = None;
        if batch.is_empty() && boundary.can_draw() && !boundary.alternate {
            fish_due = pet.fish_refresh_at(now);
        }
        if tick
            || due.is_some_and(|d| now >= d)
            || batch.len() == CAP
            || fish_due.is_some_and(|d| now >= d)
        {
            fish_due = None;
            let start = boundary.can_draw();
            let was_alternate = boundary.alternate;
            let was_child_mouse = boundary.child_mouse();
            forwarded.clear();
            if relay_modes(
                &mut boundary,
                &batch,
                capture_mouse,
                kitty.is_some(),
                &mut forwarded,
            ) {
                pet.notice_bell();
            }
            if boundary.screen_selected
                || (!boundary.alternate && was_child_mouse != boundary.child_mouse())
            {
                buttons = Default::default();
                decoder.flush(&mut pending);
                input_deadline = None;
                pet.cancel_mouse_interaction();
            }
            if boundary.screen_selected {
                pet.forget_food();
                food_deadline = None;
                if let Some(food) = &mut food {
                    *food = crate::deepseek_pet::food::Food::new(None);
                }
            } else if let Some(cell) = food
                .as_mut()
                .filter(|_| !boundary.alternate)
                .and_then(|food| food.feed(&batch, s.ws_col, s.ws_row))
            {
                if let Some((col, row)) = cell {
                    pet.notice_food(col, row);
                    food_deadline = Some(now + Duration::from_secs(3));
                } else {
                    pet.forget_food();
                    food_deadline = None;
                }
            }
            let end = boundary.can_draw();
            update.clear();
            if !batch.is_empty() {
                if start && (!was_alternate || boundary.screen_selected) {
                    pet.hide(s.ws_col, s.ws_row, &mut update);
                }
                if was_alternate {
                    // Scrolling, clearing or plugin deletes may remove or move a placement.
                    pet.invalidate();
                }
                if let Some(screen) = &mut pet.screen {
                    screen.feed(&batch);
                }
            }
            sprite.clear();
            terminal.child_alternate = boundary.alternate;
            pet.image_only = boundary.alternate;
            // Forward the screen switch first. Resume drawing on a later batch.
            // A full batch can be followed immediately by more shell output.
            // During a flood, cap sprite repaints at the animation tick rate.
            if end
                && (!boundary.alternate || kitty.is_some())
                && !boundary.screen_selected
                && (tick || (!boundary.alternate && batch.len() < CAP))
            {
                pet.draw(
                    frame.wrapping_sub(1),
                    s.ws_col,
                    s.ws_row,
                    pixels.width,
                    pixels.height,
                    &mut sprite,
                )?;
                fish_due = pet.fish_refresh_at(now);
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
            .min(fish_due.unwrap_or(next))
            .min(input_deadline.unwrap_or(next))
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
                    let written = pending.drain(..n);
                    if let Some(preview) = &mut preview {
                        for byte in written {
                            if let Some(kind) = preview.input(&catalog, byte) {
                                pet.notice_command(b'e', kind);
                            }
                        }
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
            if bytes.contains(&0x1d) {
                break;
            }
            let shared = boundary.alternate && kitty.is_some();
            if capture_mouse && (shared || !boundary.child_mouse()) {
                decoder.feed(bytes, &mut pending, |report, out| {
                    if shared {
                        let modes = boundary.mouse;
                        let cell = report.cell(modes.encoding, (pixels.width, pixels.height));
                        let pixel = (modes.encoding == 1016).then_some((report.x, report.y));
                        let kind = report.kind();
                        if buttons.pet_owns(report, pet.covers(cell, pixel)) {
                            pet.mouse_event(kind, cell.0, cell.1);
                        } else {
                            report.forward(modes, out);
                        }
                    } else if s.ws_col > 1 && s.ws_row > 0 {
                        pet.mouse_event(
                            report.kind(),
                            report.x.min(s.ws_col - 1),
                            report.y.min(s.ws_row),
                        );
                    }
                });
                input_deadline = decoder
                    .has_partial_report()
                    .then(|| Instant::now() + Duration::from_millis(100));
            } else {
                pending.extend(bytes);
            }
            if !pending.is_empty() {
                pet.notice_input();
            }
        }
    }
    update.clear();
    let start = boundary.can_draw();
    if start {
        pet.hide(s.ws_col, s.ws_row, &mut update);
    }
    boundary.feed(&batch);
    terminal.child_alternate = boundary.alternate;
    render_batch(
        &mut out,
        &update,
        &batch,
        &[],
        start && boundary.can_draw() && !boundary.screen_selected,
    )?;
    if !boundary.ready() {
        out.write_all(b"\x18")?; // Cancel unfinished shell output before terminal teardown.
    }
    out.flush()?;
    drop(terminal);
    if direct && child_eof {
        child.wait()
    } else {
        Ok(0)
    }
}

// Insert mode changes at the CSI final byte, so later Vim mode requests in
// the same read take effect normally (also when the CSI spans two reads).
fn relay_modes(
    boundary: &mut crate::deepseek_pet::boundary::Boundary,
    bytes: &[u8],
    capture: bool,
    kitty: bool,
    out: &mut Vec<u8>,
) -> bool {
    let mut start = 0;
    let mut was_shared = kitty && boundary.alternate;
    let mut was_child = boundary.child_mouse();
    let bell = boundary.feed_with_mouse(bytes, |end, state| {
        let shared = kitty && state.alternate;
        if capture && (shared || was_shared || !state.child_mouse() || !was_child) {
            let (tracking, encoding) = if shared {
                let tracking = if state.mouse.tracking == 1003 {
                    1003
                } else {
                    1002
                };
                let encoding = if state.mouse.encoding == 1016 {
                    1016
                } else {
                    1006
                };
                (tracking, encoding)
            } else if state.child_mouse() {
                (state.mouse.tracking, state.mouse.encoding)
            } else {
                (1003, 1006)
            };
            out.extend_from_slice(&bytes[start..end]);
            start = end;
            if was_shared && !shared {
                out.extend_from_slice(b"\x1b[?1002l\x1b[?1016l");
            }
            out.extend_from_slice(b"\x1b[?1003l\x1b[?1006l");
            for mode in [encoding, tracking] {
                if mode != 0 {
                    write!(out, "\x1b[?{mode}h").expect("Vec write");
                }
            }
        }
        was_shared = shared;
        was_child = state.child_mouse();
    });
    out.extend_from_slice(&bytes[start..]);
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
    fn kitty_requests_drags_even_without_child_tracking_and_restores_legacy_mode() {
        let mut b = crate::deepseek_pet::boundary::Boundary::default();
        let mut out = Vec::new();
        relay_modes(&mut b, b"\x1b[?1049h", true, true, &mut out);
        assert!(out.ends_with(b"\x1b[?1006h\x1b[?1002h"));
        assert_eq!(b.mouse.tracking, 0);
        relay_modes(&mut b, b"\x1b[?1000h\x1b[?1016h", true, true, &mut out);
        assert!(out.ends_with(b"\x1b[?1016h\x1b[?1002h"));
        relay_modes(&mut b, b"\x1b[?1016l\x1b[?1049l", true, true, &mut out);
        assert!(out.ends_with(b"\x1b[?1006l\x1b[?1000h"));
    }

    #[test]
    fn mouse_handoff_survives_split_switches_and_vims_teardown_order() {
        let mut b = crate::deepseek_pet::boundary::Boundary::default();
        let mut out = Vec::new();
        for byte in b"\x1b[?1049h\x1b[?1002h\x1b[?1006h" {
            relay_modes(&mut b, &[*byte], true, false, &mut out);
        }
        assert!(b.child_mouse());
        assert_eq!(
            out,
            b"\x1b[?1049h\x1b[?1003l\x1b[?1006l\x1b[?1002h\x1b[?1006h"
        );
        out.clear();
        relay_modes(
            &mut b,
            b"\x1b[?1049l\x1b[?1002l\x1b[?1006l",
            true,
            false,
            &mut out,
        );
        assert!(!b.child_mouse());
        assert!(out.ends_with(b"\x1b[?1006h\x1b[?1003h"));
    }

    #[test]
    fn reads_one_based_cursor_position_alongside_device_attributes() {
        assert_eq!(cursor_position(b"\x1b[?62;4c\x1b[21;7R"), Some((7, 21)));
        assert_eq!(cursor_position(b"\x1b[21;7R\x1b[?62;4c"), Some((7, 21)));
        assert_eq!(cursor_position(b"\x1b[0;7R"), None);
        assert_eq!(cursor_position(b"\x1b[21;7"), None);
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
