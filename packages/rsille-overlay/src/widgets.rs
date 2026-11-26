//! Float a regular rsille-tui application over the child's output.
use crate::{Context, MouseKind, Overlay, Pointer, ShellEvent};
use crossterm::event::{Event, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use rsille_render::{
    area::{Area, Size},
    style::Style,
};
use rsille_tui::runtime::Runtime;
use std::{io, time::Instant};

/// Where a [`WidgetOverlay`] sits in the terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    /// A zero-based cell for the top-left corner, clamped to the viewport.
    At(u16, u16),
}

type ShellMapper<M> = Box<dyn FnMut(ShellEvent<'_>) -> Option<M>>;

/// A headless rsille-tui [`Runtime`] painted into a fixed-size box.
///
/// Mouse events inside the box reach the application's widgets, so buttons,
/// checkboxes and lists work as usual. The keyboard stays with the child.
/// Map [`ShellEvent`]s to application messages with [`Self::on_shell`].
///
/// The box is painted as text and hidden whenever the child writes, then
/// repainted, so it never corrupts the child's screen. Its cells are opaque.
pub struct WidgetOverlay<S, M: Send + 'static> {
    runtime: Runtime<S, M>,
    size: Size,
    anchor: Anchor,
    margin: (u16, u16),
    on_shell: Option<ShellMapper<M>>,
    events: Vec<Event>,
    painted: Option<Area>,
    /// The cells last painted, to skip repaints when a step changed nothing.
    shown: Vec<rsille_render::buffer::Cell>,
    stale: bool,
    poke: bool,
    hovered: bool,
}

impl<S, M: Send + 'static> WidgetOverlay<S, M> {
    /// Host `runtime` (from `App::...build()`) in a box of `(width, height)` cells,
    /// anchored at the bottom right by default.
    pub fn new(mut runtime: Runtime<S, M>, (width, height): (u16, u16)) -> Self {
        let size = Size { width, height };
        runtime.resize(size);
        Self {
            runtime,
            size,
            anchor: Anchor::BottomRight,
            margin: (1, 0),
            on_shell: None,
            events: Vec::new(),
            painted: None,
            shown: Vec::new(),
            stale: true,
            poke: true,
            hovered: false,
        }
    }

    pub fn anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
        self
    }

    /// Cells kept free between the box and the anchored edges (default `(1, 0)`).
    /// The right margin also keeps the box out of the last column, where
    /// writing would make some terminals wrap.
    pub fn margin(mut self, horizontal: u16, vertical: u16) -> Self {
        self.margin = (horizontal, vertical);
        self
    }

    /// Turn shell activity into application messages.
    pub fn on_shell(mut self, map: impl FnMut(ShellEvent<'_>) -> Option<M> + 'static) -> Self {
        self.on_shell = Some(Box::new(map));
        self
    }

    pub fn state(&self) -> &S {
        self.runtime.state()
    }

    pub fn runtime(&self) -> &Runtime<S, M> {
        &self.runtime
    }

    pub fn runtime_mut(&mut self) -> &mut Runtime<S, M> {
        self.poke = true;
        &mut self.runtime
    }

    /// The box in zero-based cells for a viewport, or `None` if it does not fit.
    pub fn place(&self, cols: u16, rows: u16) -> Option<Area> {
        let Size { width, height } = self.size;
        let (mx, my) = self.margin;
        if width == 0 || height == 0 {
            return None;
        }
        let right = cols.checked_sub(width.checked_add(mx)?)?;
        let bottom = rows.checked_sub(height.checked_add(my)?)?;
        let (x, y) = match self.anchor {
            Anchor::TopLeft => (mx.min(right), my.min(bottom)),
            Anchor::TopRight => (right, my.min(bottom)),
            Anchor::BottomLeft => (mx.min(right), bottom),
            Anchor::BottomRight => (right, bottom),
            Anchor::At(x, y) => (x.min(right), y.min(bottom)),
        };
        Some(Area::new((x, y).into(), self.size))
    }

    fn paint(&self, area: Area, out: &mut Vec<u8>) -> io::Result<()> {
        use std::io::Write;
        let frame = self.runtime.frame();
        let width = usize::from(frame.size().width);
        for (y, row) in frame.content().chunks(width.max(1)).enumerate() {
            write!(
                out,
                "\x1b[{};{}H\x1b[0m",
                area.y() + y as u16 + 1,
                area.x() + 1
            )?;
            let mut pen = Style::default();
            for cell in row {
                if cell.is_occupied {
                    continue; // Covered by the wide grapheme before it.
                }
                let content = &cell.content;
                if content.style != pen {
                    if pen != Style::default() {
                        out.extend_from_slice(b"\x1b[0m");
                    }
                    pen = content.style;
                }
                if content.is_empty() || content.width() == 0 {
                    out.push(b' ');
                } else {
                    content.queue(out)?;
                }
            }
            out.extend_from_slice(b"\x1b[0m");
        }
        Ok(())
    }

    fn local(&self, pointer: &Pointer) -> Option<(u16, u16)> {
        let area = self.painted?;
        let x = pointer.col.checked_sub(1)?.checked_sub(area.x())?;
        let y = pointer.row.checked_sub(1)?.checked_sub(area.y())?;
        area.is_inside(x, y).then_some((x, y))
    }
}

fn modifiers(button: u16) -> KeyModifiers {
    let mut modifiers = KeyModifiers::empty();
    for (bit, modifier) in [
        (4, KeyModifiers::SHIFT),
        (8, KeyModifiers::ALT),
        (16, KeyModifiers::CONTROL),
    ] {
        if button & bit != 0 {
            modifiers |= modifier;
        }
    }
    modifiers
}

impl<S, M: Send + 'static> Overlay for WidgetOverlay<S, M> {
    fn draw(&mut self, cx: &mut Context<'_>, out: &mut Vec<u8>) -> io::Result<()> {
        let events = std::mem::take(&mut self.events);
        let step = self.runtime.step(&events).map_err(io::Error::other)?;
        self.poke = false;
        let area = self.place(cx.cols, cx.rows);
        if area != self.painted {
            if let Some(old) = self.painted.take() {
                cx.erase(old, out);
            }
            self.stale = true;
        }
        if let Some(area) = area {
            let changed = step.rendered && self.runtime.frame().content() != self.shown;
            if self.stale || changed {
                self.shown.clear();
                self.shown.extend_from_slice(self.runtime.frame().content());
                self.paint(area, out)?;
                self.painted = Some(area);
                self.stale = false;
            }
        }
        Ok(())
    }

    fn hide(&mut self, cx: &mut Context<'_>, out: &mut Vec<u8>) {
        if let Some(area) = self.painted.take() {
            cx.erase(area, out);
        }
        self.stale = true;
    }

    fn invalidate(&mut self) {
        self.stale = true;
    }

    fn wake_at(&mut self, now: Instant) -> Option<Instant> {
        if self.poke || !self.events.is_empty() {
            return Some(now);
        }
        self.runtime.next_update_at()
    }

    fn hit(&self, pointer: &Pointer) -> bool {
        self.local(pointer).is_some()
    }

    fn pointer(&mut self, pointer: Pointer) {
        let Some(area) = self.painted else { return };
        let inside = self.local(&pointer);
        let kind = match pointer.kind {
            MouseKind::LeftPress => MouseEventKind::Down(MouseButton::Left),
            MouseKind::LeftDrag => MouseEventKind::Drag(MouseButton::Left),
            MouseKind::LeftRelease => MouseEventKind::Up(MouseButton::Left),
            MouseKind::RightPress => MouseEventKind::Down(MouseButton::Right),
            MouseKind::Motion => MouseEventKind::Moved,
            MouseKind::Other => match pointer.scroll() {
                Some(-1) => MouseEventKind::ScrollUp,
                Some(_) => MouseEventKind::ScrollDown,
                None => return,
            },
        };
        let hover = matches!(kind, MouseEventKind::Moved);
        let (column, row) = match inside {
            Some(cell) => cell,
            // Tell the widgets the pointer left; keep a drag pinned to the edge.
            None if hover && self.hovered => (area.width(), area.height()),
            None if !hover && !matches!(kind, MouseEventKind::Down(_)) => {
                let x =
                    (pointer.col.saturating_sub(1)).clamp(area.x(), area.x() + area.width() - 1);
                let y =
                    (pointer.row.saturating_sub(1)).clamp(area.y(), area.y() + area.height() - 1);
                (x - area.x(), y - area.y())
            }
            None => return,
        };
        if hover {
            self.hovered = inside.is_some();
        }
        self.events.push(Event::Mouse(MouseEvent {
            kind,
            column,
            row,
            modifiers: modifiers(pointer.button),
        }));
    }

    fn pointer_reset(&mut self) {
        if self.hovered {
            self.hovered = false;
            self.events.push(Event::Mouse(MouseEvent {
                kind: MouseEventKind::Moved,
                column: self.size.width,
                row: self.size.height,
                modifiers: KeyModifiers::empty(),
            }));
        }
    }

    fn shell(&mut self, event: ShellEvent<'_>) {
        if let Some(message) = self.on_shell.as_mut().and_then(|map| map(event)) {
            // The receiver lives in the runtime, so sending cannot fail while it exists.
            let _ = self.runtime.handle().send(message);
            self.poke = true;
        }
    }

    fn resized(&mut self, _cols: u16, _rows: u16) {
        self.stale = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsille_tui::prelude::{button, label, App};

    fn counter() -> WidgetOverlay<u32, ()> {
        let app = App::new(0u32)
            .update(|count: &mut u32, (): (), _ctx| *count += 1)
            .view(|count: &u32| button(format!("n={count}")).on_click(|| ()))
            .build();
        WidgetOverlay::new(app, (8, 1))
    }

    fn draw(overlay: &mut impl Overlay, screen: &mut Option<crate::Screen>) -> Vec<u8> {
        let mut out = Vec::new();
        let now = Instant::now();
        let mut cx = Context {
            now,
            epoch: now,
            cols: 20,
            rows: 5,
            cell: crate::CellPixels {
                width: 0,
                height: 0,
            },
            graphics: crate::Graphics::Text,
            alternate: false,
            screen,
        };
        overlay.draw(&mut cx, &mut out).unwrap();
        out
    }

    fn click(col: u16, row: u16) -> Pointer {
        Pointer {
            kind: MouseKind::LeftPress,
            col,
            row,
            pixel: None,
            button: 0,
        }
    }

    #[test]
    fn places_the_box_inside_the_margins_or_not_at_all() {
        let overlay = counter();
        assert_eq!(
            overlay.place(20, 5),
            Some(Area::new((11, 4).into(), (8, 1).into()))
        );
        let top = counter().anchor(Anchor::TopLeft).margin(2, 1);
        assert_eq!(
            top.place(20, 5),
            Some(Area::new((2, 1).into(), (8, 1).into()))
        );
        let at = counter().anchor(Anchor::At(100, 100));
        assert_eq!(at.place(20, 5).unwrap().pos(), (11, 4).into());
        assert_eq!(counter().place(8, 5), None);
    }

    #[test]
    fn clicks_inside_the_box_reach_the_widget_and_repaint_it() {
        let mut overlay = counter();
        let mut screen = crate::Screen::new(5, 20, (1, 1));
        let mut grid = crate::Screen::new(5, 20, (1, 1)).unwrap();
        grid.feed(&draw(&mut overlay, &mut screen));
        assert!(grid.grid().contents().contains("n=0"));
        assert!(!overlay.hit(&click(1, 1)));
        let inside = click(13, 5);
        assert!(overlay.hit(&inside));
        overlay.pointer(inside);
        assert!(overlay.wake_at(Instant::now()).is_some());
        grid.feed(&draw(&mut overlay, &mut screen));
        assert_eq!(*overlay.state(), 1);
        assert!(grid.grid().contents().contains("n=1"));
        // Nothing changed: no bytes.
        assert!(draw(&mut overlay, &mut screen).is_empty());
    }

    #[test]
    fn hiding_restores_the_child_text_under_the_box() {
        let mut overlay = counter();
        let mut screen = crate::Screen::new(5, 20, (1, 1));
        let shell = b"\x1b[5;1Hthe quick brown fox!";
        screen.as_mut().unwrap().feed(shell);
        let mut grid = crate::Screen::new(5, 20, (1, 1)).unwrap();
        grid.feed(shell);
        let before = grid.grid().contents();
        grid.feed(&draw(&mut overlay, &mut screen));
        assert_ne!(grid.grid().contents(), before);
        let mut out = Vec::new();
        let now = Instant::now();
        overlay.hide(
            &mut Context {
                now,
                epoch: now,
                cols: 20,
                rows: 5,
                cell: crate::CellPixels {
                    width: 0,
                    height: 0,
                },
                graphics: crate::Graphics::Text,
                alternate: false,
                screen: &mut screen,
            },
            &mut out,
        );
        grid.feed(&out);
        assert_eq!(grid.grid().contents(), before);
        // A hidden box repaints fully on the next draw.
        assert!(!draw(&mut overlay, &mut screen).is_empty());
    }

    #[test]
    fn shell_events_become_messages() {
        let app = App::new(Vec::<String>::new())
            .update(|log: &mut Vec<String>, line: String, _ctx| log.push(line))
            .view(|log: &Vec<String>| label(log.join(",")))
            .build();
        let mut overlay = WidgetOverlay::new(app, (10, 1)).on_shell(|event| match event {
            ShellEvent::Exec(line) => Some(line.to_owned()),
            _ => None,
        });
        overlay.shell(ShellEvent::Prompt);
        overlay.shell(ShellEvent::Exec("ls"));
        draw(&mut overlay, &mut None);
        assert_eq!(overlay.state(), &["ls"]);
    }
}
