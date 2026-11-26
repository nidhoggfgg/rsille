//! Local interactive PTY session for an ANSI-compatible terminal.
use crate::{
    boundary::Boundary,
    mouse::{Buttons, Mouse},
    sys::{self, Pty, TerminalMode},
    Attach, CellPixels, Context, Graphics, InlineRasterSession, Overlay, Pointer, Screen, Session,
};
use std::{
    io::{self, Write},
    time::{Duration, Instant},
};

const CLEAR_VIEWPORT: &[u8] = b"\x1b[2J\x1b[H";
/// Overlays redraw at least this often while visible. Resizes are noticed per tick.
const TICK: Duration = Duration::from_millis(100);

struct Terminal {
    cleanup: Vec<u8>,
    _mode: TerminalMode,
    alt_screen: bool,
    child_alternate: bool,
}
impl Drop for Terminal {
    fn drop(&mut self) {
        // The detach key can also interrupt a child application that enabled mouse tracking.
        let mut out = io::stdout().lock();
        let _ = out.write_all(b"\x18\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[?1016l");
        #[cfg(windows)]
        let _ = out.write_all(b"\x1b[?9001l\x1b[?1004l");
        let _ = out.write_all(&self.cleanup);
        let _ = out.write_all(b"\x1b[0m");
        if self.alt_screen || self.child_alternate {
            let _ = out.write_all(b"\x1b[?1049l");
        }
        let _ = out.flush();
    }
}

fn measured_pixels() -> Option<CellPixels> {
    CellPixels::from_terminal().and_then(|p| CellPixels::resolve(Some(p)).ok())
}

/// Requires exclusive use of stdin, stdout and the spawned child's wait status.
pub fn run<O: Overlay + ?Sized>(
    options: Session,
    overlay: &mut O,
    widget_input: Option<std::sync::mpsc::Receiver<Vec<u8>>>,
) -> io::Result<i32> {
    let direct = !options.command.is_empty();
    let program = options
        .command
        .first()
        .cloned()
        .unwrap_or_else(sys::default_shell);
    if program.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "empty program path",
        ));
    }
    #[cfg(unix)]
    let mut hooks = if !direct && options.shell_events {
        sys::Hooks::prepare(program.as_ref())?
    } else {
        None
    };
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
    let mut s = sys::size()?;
    let epoch = Instant::now();
    overlay.attach(&Attach {
        epoch,
        graphics,
        cell: pixels,
        cols: s.cols,
        rows: s.rows,
        #[cfg(unix)]
        shell: hooks.as_ref().map(sys::Hooks::shell),
        #[cfg(windows)]
        shell: None,
    })?;
    let mut cleanup = Vec::new();
    overlay.cleanup(&mut cleanup);

    let mode = TerminalMode::save()?;
    let spawn = sys::Spawn {
        program: &program,
        args: options.command.get(1..).unwrap_or_default(),
        size: s,
        #[cfg(unix)]
        direct,
        #[cfg(unix)]
        name: &options.name,
    };
    #[cfg(unix)]
    let mut pty = Pty::spawn(&spawn, hooks.as_mut(), &mode)?;
    #[cfg(windows)]
    let mut pty = Pty::spawn(&spawn)?;
    mode.enter_raw()?;
    let mut terminal = Terminal {
        cleanup,
        _mode: mode,
        alt_screen: options.alt_screen,
        child_alternate: false,
    };
    let capture_mouse = options.mouse;
    let mut decoder = Mouse::default();
    let mut buttons = Buttons::default();
    let mut input_deadline = None;
    let mut screen = if options.restore_text {
        Screen::new(s.rows, s.cols, (1, 1))
    } else {
        None
    };
    macro_rules! cx {
        ($now:expr, $alternate:expr) => {
            Context {
                now: $now,
                epoch,
                cols: s.cols,
                rows: s.rows,
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
    #[cfg(unix)]
    let mut event = vec![0u8; crate::shell::MAX_EVENT];
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
        #[cfg(unix)]
        if let Some(hooks) = &hooks {
            for _ in 0..64 {
                match hooks.recv(&mut event) {
                    Ok(n) => {
                        if let Some(event) = crate::shell::parse(&event[..n]) {
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
            let new = sys::size()?;
            if s != new {
                pty.resize(new)?;
                s = new;
                if let Some(measured) = measured_pixels() {
                    pixels = measured;
                }
                boundary.resized();
                if screen
                    .as_mut()
                    .is_some_and(|screen| !screen.resize(s.rows, s.cols))
                {
                    screen = None;
                }
                overlay.resized(s.cols, s.rows);
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
        if pending.is_empty() {
            if let Some(bytes) = widget_input.as_ref().and_then(|rx| rx.try_recv().ok()) {
                pending.extend(bytes);
            }
        }
        let reading = pending.is_empty();
        let next_tick = epoch + TICK * (tick_index as u32 + 1);
        let timeout = [due, wake, input_deadline]
            .into_iter()
            .flatten()
            .fold(next_tick, Instant::min)
            .saturating_duration_since(Instant::now());
        let ready = pty.poll(reading, !reading, timeout)?;
        if ready.output {
            let room = output.len().min(CAP - batch.len());
            match pty.read_output(&mut output[..room]) {
                Ok(0) => {
                    child_eof = true;
                    break;
                }
                Ok(n) => {
                    batch.extend_from_slice(&output[..n]);
                    due.get_or_insert_with(|| Instant::now() + Duration::from_millis(20));
                }
                Err(e) if retry(&e) => {}
                Err(e) => return Err(e),
            }
        }
        if ready.writable {
            match pty.write_input(pending.as_slices().0) {
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
        if ready.input {
            #[cfg(windows)]
            let read = pty.read_input(&mut input, options.detach_key);
            #[cfg(unix)]
            let read = pty.read_input(&mut input);
            let n = match read {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) if retry(&e) => continue,
                Err(e) => return Err(e),
            };
            let bytes = &input[..n];
            #[cfg(unix)]
            if options.detach_key.is_some_and(|key| bytes.contains(&key)) {
                break;
            }
            if capture_mouse && (shared || !boundary.child_mouse()) {
                let (cols, rows) = (s.cols, s.rows);
                decoder.feed(bytes, &mut pending, |report, out| {
                    let pointer = |col, row, pixel| Pointer {
                        kind: report.kind(),
                        col,
                        row,
                        pixel,
                        button: report.button,
                    };
                    if shared || cfg!(windows) {
                        // ConPTY handles the child's mouse subscription internally.
                        // Offer visible overlay hits first and send other reports
                        // in SGR cell coordinates for ConPTY to translate.
                        let modes = if cfg!(windows) {
                            crate::mouse::MouseModes {
                                tracking: 1003,
                                encoding: 1006,
                            }
                        } else {
                            boundary.mouse
                        };
                        let (col, row) = report.cell(modes.encoding, (pixels.width, pixels.height));
                        let pixel = (modes.encoding == 1016).then_some((report.x, report.y));
                        let p = pointer(col, row, pixel);
                        let hit = overlay.hit(&p);
                        // Text widgets scroll in cells on both PTY backends.
                        // Shared Kitty images leave the wheel with the child.
                        let widget_scroll = cfg!(windows) && !shared && hit && p.scroll().is_some();
                        if widget_scroll || buttons.overlay_owns(report, hit) {
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
        pty.wait()
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
