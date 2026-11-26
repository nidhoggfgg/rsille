//! Draw over a live shell instead of replacing it.
//!
//! An overlay session runs the user's shell (or any program) in a PTY and
//! relays its output to the real terminal unchanged. Between output batches,
//! at points where no escape sequence, image upload or synchronized update is
//! open, the session asks an [`Overlay`] to draw. Before the next output that
//! could move or overwrite those cells, it asks the overlay to hide again.
//!
//! The session takes care of the parts that are easy to get wrong:
//!
//! - finding safe redraw points in the child's byte stream, even when a
//!   sequence is split across reads;
//! - keeping a shadow copy of the child's screen so text covered by the
//!   overlay can be put back ([`Context::erase`]);
//! - sharing the mouse with the child: presses the overlay [`Overlay::hit`]s
//!   stay with the overlay through drag and release, everything else is
//!   re-encoded in the mode the child asked for;
//! - reporting what the shell is doing ([`ShellEvent`]) for bash, fish and zsh;
//! - restoring the terminal on exit, including after errors.
//!
//! Use [`WidgetOverlay`] to float a regular rsille-tui application (buttons,
//! labels, progress bars…) over the shell, or implement [`Overlay`] directly
//! for full control over the bytes, including Kitty or Sixel images.
//!
//! ```no_run
//! use rsille_overlay::{Anchor, Session, WidgetOverlay};
//! use rsille_tui::prelude::*;
//!
//! let app = App::new(0u32)
//!     .update(|count: &mut u32, (): (), _ctx| *count += 1)
//!     .view(|count| button(format!("clicked {count}")).on_click(|| ()))
//!     .build();
//! let mut overlay = WidgetOverlay::new(app, (16, 1)).anchor(Anchor::TopRight);
//! Session::new().mouse(true).run(&mut overlay)?;
//! # Ok::<(), std::io::Error>(())
//! ```
//!
//! Sessions require Unix PTYs. On other platforms [`Session::run`] returns
//! [`std::io::ErrorKind::Unsupported`].

pub mod boundary;
pub(crate) mod mouse;
pub mod screen;
#[cfg(unix)]
mod session;
#[cfg(unix)]
mod shell;
mod stack;
mod widgets;

pub use mouse::MouseKind;
pub use rsille_render::raster_output::{CellPixels, InlineRasterSession, RasterBackend};
pub use screen::Screen;
pub use stack::Stack;
pub use vt100;
pub use widgets::{Anchor, WidgetOverlay};

use rsille_render::area::Area;
use std::{
    ffi::OsString,
    io,
    time::{Duration, Instant},
};

/// How the overlay can draw on this terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Graphics {
    /// Only character cells. Overlays hide while the child uses the alternate screen.
    Text,
    /// Kitty image placements float above the text grid, so they never cover
    /// the child's characters and can stay visible on the alternate screen.
    Kitty,
    /// Sixel images are written into the grid like text and hide on the
    /// alternate screen.
    Sixel,
}

/// The interactive shell that reports [`ShellEvent`]s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shell {
    Bash,
    Fish,
    Zsh,
}

/// What the interactive shell is doing. bash, fish and zsh report events.
///
/// bash reports the line from its history, so lines history skips (for
/// example with `HISTCONTROL=ignorespace`) arrive as the previous entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellEvent<'a> {
    /// The shell is about to show a new prompt (also after Ctrl+C at the prompt).
    Prompt,
    /// The command line being edited changed (zsh only). The line is cut
    /// after 4097 characters.
    Edit(&'a str),
    /// A command line is about to run. The line is cut after 4097 characters.
    Exec(&'a str),
    /// The command started by the last [`ShellEvent::Exec`] finished with this
    /// status. 127 usually means "command not found", 130 an interrupt.
    Done(i32),
    /// A running command is reading a line through the shell (fish `read`).
    Read,
}

/// Facts about the session, available once before the child starts.
#[derive(Debug, Clone, Copy)]
pub struct Attach {
    /// The session clock's origin. [`Context::now`] is measured from it.
    pub epoch: Instant,
    pub graphics: Graphics,
    /// Physical cell size. Zero when the terminal does not report it.
    pub cell: CellPixels,
    pub cols: u16,
    pub rows: u16,
    /// The shell reporting [`ShellEvent`]s, if any.
    pub shell: Option<Shell>,
}

/// The terminal state while drawing, hiding or reading output.
pub struct Context<'a> {
    pub now: Instant,
    pub epoch: Instant,
    pub cols: u16,
    pub rows: u16,
    /// Physical cell size. Zero when the terminal does not report it.
    pub cell: CellPixels,
    pub graphics: Graphics,
    /// The child is showing its alternate screen (a fullscreen program).
    pub alternate: bool,
    /// A copy of the child's visible text, or `None` when restoration is
    /// disabled or the grid is too large. An overlay that keeps its own
    /// handle may move it out temporarily but must put it back.
    pub screen: &'a mut Option<Screen>,
}

impl Context<'_> {
    /// Time since the session started.
    pub fn elapsed(&self) -> Duration {
        self.now.saturating_duration_since(self.epoch)
    }

    /// Clear `area` (zero-based cells) and put back the child's text under it.
    /// Cells outside the viewport are ignored.
    pub fn erase(&self, area: Area, out: &mut Vec<u8>) {
        screen::erase_area(area, self.cols, self.rows, self.screen.as_ref(), out);
    }
}

/// A pointer event in one-based terminal cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pointer {
    pub kind: MouseKind,
    pub col: u16,
    pub row: u16,
    /// One-based pixel position when the terminal reports pixels (Kitty sessions).
    pub pixel: Option<(u16, u16)>,
    /// The raw SGR button code, including modifier, motion and wheel bits.
    pub button: u16,
}

impl Pointer {
    /// Motion with no button held.
    pub fn is_hover(&self) -> bool {
        self.kind == MouseKind::Motion && self.button & 3 == 3
    }

    /// `Some(-1)` for wheel up, `Some(1)` for wheel down.
    pub fn scroll(&self) -> Option<i8> {
        match self.button & !(4 | 8 | 16) {
            64 => Some(-1),
            65 => Some(1),
            _ => None,
        }
    }
}

/// Something drawn over the child's output.
///
/// All byte output goes to `out`, which the session writes inside a
/// synchronized update with the cursor saved and restored. Use absolute cursor
/// positioning (`ESC [ row ; col H`) and reset any SGR state you change.
///
/// The session calls [`Overlay::hide`] before child output that could
/// disturb what was drawn, then [`Overlay::draw`] again once the output is
/// complete. Text overlays must therefore repaint on every `draw` after a
/// `hide`. Kitty placements survive output and only need hiding when they move.
pub trait Overlay {
    /// Called once before the child starts.
    fn attach(&mut self, _attach: &Attach) -> io::Result<()> {
        Ok(())
    }

    /// Bytes that remove everything this overlay could have put on screen
    /// (for example Kitty image deletions). Written when the session ends,
    /// even after an error. Called right after [`Overlay::attach`].
    fn cleanup(&self, _out: &mut Vec<u8>) {}

    /// Draw the current frame. Called at least ten times a second while the
    /// overlay is visible, and at the times returned by [`Overlay::wake_at`].
    fn draw(&mut self, cx: &mut Context<'_>, out: &mut Vec<u8>) -> io::Result<()>;

    /// Remove everything drawn so far while its coordinates are still valid.
    fn hide(&mut self, cx: &mut Context<'_>, out: &mut Vec<u8>);

    /// What was drawn may be gone (the child scrolled or cleared the screen).
    /// Repaint fully on the next draw.
    fn invalidate(&mut self) {}

    /// When the overlay next wants to draw, if before the next 100 ms tick.
    fn wake_at(&mut self, _now: Instant) -> Option<Instant> {
        None
    }

    /// Whether a press at this pointer position belongs to the overlay. A
    /// claimed button keeps going to the overlay until it is released.
    fn hit(&self, _pointer: &Pointer) -> bool {
        false
    }

    /// A pointer event for the overlay: claimed presses with their drags and
    /// releases, and hover motion anywhere (also forwarded to the child).
    fn pointer(&mut self, _pointer: Pointer) {}

    /// A press, release or scroll that went to the child instead. Useful to
    /// dismiss menus when the user clicks elsewhere.
    fn pointer_passed(&mut self, _pointer: Pointer) {}

    /// The child took over the mouse or switched screens. Drop drags and grabs.
    fn pointer_reset(&mut self) {}

    /// Input bytes just delivered to the child (keys, paste, unclaimed mouse).
    fn input(&mut self, _bytes: &[u8]) {}

    /// A batch of child output, after it reached the terminal and the screen copy.
    fn output(&mut self, _bytes: &[u8], _cx: &mut Context<'_>) {}

    /// The child rang the terminal bell.
    fn bell(&mut self) {}

    /// The interactive shell reported an event.
    fn shell(&mut self, _event: ShellEvent<'_>) {}

    /// The terminal changed size. A full redraw follows.
    fn resized(&mut self, _cols: u16, _rows: u16) {}
}

impl<O: Overlay + ?Sized> Overlay for &mut O {
    fn attach(&mut self, attach: &Attach) -> io::Result<()> {
        (**self).attach(attach)
    }
    fn cleanup(&self, out: &mut Vec<u8>) {
        (**self).cleanup(out)
    }
    fn draw(&mut self, cx: &mut Context<'_>, out: &mut Vec<u8>) -> io::Result<()> {
        (**self).draw(cx, out)
    }
    fn hide(&mut self, cx: &mut Context<'_>, out: &mut Vec<u8>) {
        (**self).hide(cx, out)
    }
    fn invalidate(&mut self) {
        (**self).invalidate()
    }
    fn wake_at(&mut self, now: Instant) -> Option<Instant> {
        (**self).wake_at(now)
    }
    fn hit(&self, pointer: &Pointer) -> bool {
        (**self).hit(pointer)
    }
    fn pointer(&mut self, pointer: Pointer) {
        (**self).pointer(pointer)
    }
    fn pointer_passed(&mut self, pointer: Pointer) {
        (**self).pointer_passed(pointer)
    }
    fn pointer_reset(&mut self) {
        (**self).pointer_reset()
    }
    fn input(&mut self, bytes: &[u8]) {
        (**self).input(bytes)
    }
    fn output(&mut self, bytes: &[u8], cx: &mut Context<'_>) {
        (**self).output(bytes, cx)
    }
    fn bell(&mut self) {
        (**self).bell()
    }
    fn shell(&mut self, event: ShellEvent<'_>) {
        (**self).shell(event)
    }
    fn resized(&mut self, cols: u16, rows: u16) {
        (**self).resized(cols, rows)
    }
}

/// A PTY session that runs a program under an [`Overlay`].
#[derive(Debug, Clone)]
pub struct Session {
    pub(crate) command: Vec<OsString>,
    pub(crate) backend: Option<RasterBackend>,
    pub(crate) raster: Option<InlineRasterSession>,
    pub(crate) alt_screen: bool,
    pub(crate) restore_text: bool,
    pub(crate) mouse: bool,
    pub(crate) shell_events: bool,
    pub(crate) detach_key: Option<u8>,
    pub(crate) name: String,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Session {
    /// Run `$SHELL -i` (or `/bin/sh`), with text-only graphics and shell events.
    pub fn new() -> Self {
        Self {
            command: Vec::new(),
            backend: None,
            raster: None,
            alt_screen: false,
            restore_text: true,
            mouse: false,
            shell_events: true,
            detach_key: Some(0x1d),
            name: "rsille-overlay".into(),
        }
    }

    /// Run this program and arguments (looked up in `PATH`) instead of a shell.
    /// [`Session::run`] then returns its exit status. Shell events are not reported.
    pub fn command<I, S>(mut self, command: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.command = command.into_iter().map(Into::into).collect();
        self
    }

    /// Negotiate Kitty or Sixel graphics at startup. Fails if the terminal
    /// supports neither of the requested protocols.
    pub fn graphics(mut self, backend: RasterBackend) -> Self {
        self.backend = Some(backend);
        self
    }

    /// Use graphics negotiated earlier, for example to prepare images first.
    pub fn raster_session(mut self, session: InlineRasterSession) -> Self {
        self.raster = Some(session);
        self
    }

    /// Run on the terminal's alternate screen and leave the scrollback untouched.
    pub fn alt_screen(mut self, enabled: bool) -> Self {
        self.alt_screen = enabled;
        self
    }

    /// Keep a copy of the child's text to restore what the overlay covered
    /// (default on). Without it, [`Context::erase`] only blanks cells.
    pub fn restore_text(mut self, enabled: bool) -> Self {
        self.restore_text = enabled;
        self
    }

    /// Request mouse reports for the overlay (default off).
    ///
    /// With Kitty graphics the mouse is shared: claimed presses go to the
    /// overlay and everything else is forwarded in the child's own mode. With
    /// text or Sixel the overlay receives the mouse only while the child has
    /// not enabled mouse tracking itself.
    pub fn mouse(mut self, enabled: bool) -> Self {
        self.mouse = enabled;
        self
    }

    /// Install bash, fish or zsh hooks reporting [`ShellEvent`]s (default on).
    /// Has no effect for other shells or with [`Session::command`].
    pub fn shell_events(mut self, enabled: bool) -> Self {
        self.shell_events = enabled;
        self
    }

    /// The input byte that ends the session even while a program is running
    /// (default `0x1d`, Ctrl+]). `None` disables it.
    pub fn detach_key(mut self, key: Option<u8>) -> Self {
        self.detach_key = key;
        self
    }

    /// The prefix of error messages printed by the child process.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Run until the child exits or the detach key is pressed. Returns the exit
    /// status of a [`Session::command`] that exited on its own, otherwise 0.
    ///
    /// Requires exclusive use of stdin and stdout.
    pub fn run(self, mut overlay: impl Overlay) -> io::Result<i32> {
        #[cfg(unix)]
        {
            session::run(self, &mut overlay)
        }
        #[cfg(not(unix))]
        {
            let _ = &mut overlay;
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "overlay sessions require a Unix terminal with PTY support",
            ))
        }
    }
}

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
