//! Drive the production view and routing with a deterministic clock/executor.
use super::*;
use rsille::tui::runtime::{ManualClock, ManualExecutor};
use std::fmt::Write;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::empty()))
}
fn send(rt: &mut Runtime<State, Msg>, msg: Msg) -> Result {
    rt.handle().send(msg).expect("runtime alive");
    rt.step(&[])?;
    Ok(())
}
fn focus(rt: &mut Runtime<State, Msg>, target: &str) -> Result {
    for _ in 0..45 {
        if rt.focused_key() == Some(target) {
            return Ok(());
        }
        rt.step(&[key(KeyCode::Tab)])?;
    }
    panic!("Could not focus {target}, focus is {:?}", rt.focused_key());
}
fn text(rt: &Runtime<State, Msg>) -> String {
    rt.frame()
        .content()
        .chunks(usize::from(rt.frame().size().width))
        .map(|row| {
            row.iter()
                .filter(|c| c.owner.is_none())
                .map(|c| c.content.symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
fn advance(rt: &mut Runtime<State, Msg>, clock: &ManualClock, frames: usize) -> Result {
    for _ in 0..frames {
        clock.advance(STEP);
        rt.step(&[])?;
    }
    Ok(())
}

pub(super) fn run() -> Result {
    let clock = ManualClock::default();
    let executor = ManualExecutor::default();
    let mut rt = app(State::default())
        .with_clock(clock.clone())
        .with_executor(executor.clone())
        .update(update)
        .view(ui::view)
        .build()
        .size(120, 44);
    rt.step(&[])?;
    advance(&mut rt, &clock, 40)?;
    assert!(
        text(&rt).contains("Less noise. More possibility."),
        "{}",
        text(&rt)
    );
    let before = rt.frame().content().to_vec();
    advance(&mut rt, &clock, 2)?;
    assert_ne!(
        rt.frame().content(),
        before,
        "The canvas must actually animate"
    );

    // Hover the actual hero content: its parent border brightens, focus stays put.
    let focused = rt.focused_key().map(str::to_owned);
    let border = 4 * 120 + 26;
    let original = rt.frame().content()[border]
        .content
        .style
        .colors
        .unwrap()
        .foreground;
    rt.step(&[Event::Mouse(MouseEvent {
        kind: MouseEventKind::Moved,
        column: 40,
        row: 8,
        modifiers: KeyModifiers::empty(),
    })])?;
    advance(&mut rt, &clock, 6)?;
    assert_eq!(rt.focused_key(), focused.as_deref());
    assert_eq!(
        rt.frame().content()[border]
            .content
            .style
            .colors
            .unwrap()
            .foreground,
        Some(ui::palette(0).mint.into())
    );
    rt.step(&[Event::Mouse(MouseEvent {
        kind: MouseEventKind::Moved,
        column: 119,
        row: 43,
        modifiers: KeyModifiers::empty(),
    })])?;
    advance(&mut rt, &clock, 6)?;
    assert_eq!(
        rt.frame().content()[border]
            .content
            .style
            .colors
            .unwrap()
            .foreground,
        original
    );

    // Click the actual orbit area and ensure the canvas transforms input.
    rt.step(&[Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 95,
        row: 10,
        modifiers: KeyModifiers::empty(),
    })])?;
    assert!(
        rt.state().pointer.is_some(),
        "The hero orbit must receive pointer input"
    );
    assert_eq!(rt.state().ripple_age, 0.0);

    focus(&mut rt, "run")?;
    rt.step(&[key(KeyCode::Enter)])?;
    assert_eq!(rt.state().running.as_deref(), Some("orbit"));
    // It continues while a different page owns focus.
    rt.step(&[key(KeyCode::F(4))])?;
    assert_eq!(rt.state().page, "flows");
    advance(&mut rt, &clock, 185)?;
    assert!(rt.state().running.is_none());
    assert_eq!(rt.state().completed, 1);
    assert_eq!(rt.state().flows[0].runs, 129);
    assert!(rt.state().toast.is_some());
    let stale = rt.state().serial;
    send(&mut rt, Msg::Notify("A newer notification".into()))?;
    send(&mut rt, Msg::Dismiss(stale))?;
    assert!(rt.state().toast.as_ref().unwrap().1.contains("newer"));

    focus(&mut rt, "flow-filter")?;
    rt.step(&[Event::Paste("Prism".into())])?;
    assert!(text(&rt).contains("Prism / design sync"));
    assert!(!text(&rt).contains("Tide / inbox sweep"));
    focus(&mut rt, "flows-table")?;
    rt.step(&[key(KeyCode::Enter)])?;
    assert_eq!(rt.state().inspected, "prism");
    focus(&mut rt, "sort")?;
    rt.step(&[key(KeyCode::Enter)])?;
    assert!(rt.state().sort_desc);

    // A real modal form, with background clicks blocked and values controlled.
    focus(&mut rt, "new")?;
    rt.step(&[key(KeyCode::Enter)])?;
    advance(&mut rt, &clock, 30)?;
    assert_eq!(rt.state().modal, Some(Modal::NewFlow));
    rt.step(&[Event::Mouse(MouseEvent {
        kind: MouseEventKind::Moved,
        column: 30,
        row: 6,
        modifiers: KeyModifiers::empty(),
    })])?;
    advance(&mut rt, &clock, 6)?;
    assert!(
        rt.hovered_path().is_some_and(|path| path
            .as_slice()
            .iter()
            .any(|key| matches!(key, WidgetKey::Named(name) if name=="lumen-modal"))),
        "The modal layer owns backdrop hit testing"
    );
    let theme = rt.state().theme;
    rt.step(&[Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 95,
        row: 1,
        modifiers: KeyModifiers::empty(),
    })])?;
    assert_eq!(rt.state().theme, theme);
    focus(&mut rt, "flow-name")?;
    rt.step(&[Event::Paste("云端之外 / local first".into())])?;
    for _ in 0..8 {
        assert!(matches!(
            rt.focused_key(),
            Some("flow-name" | "cancel" | "create")
        ));
        rt.step(&[key(KeyCode::Tab)])?;
    }
    focus(&mut rt, "create")?;
    rt.step(&[key(KeyCode::Enter)])?;
    assert_eq!(rt.state().flows.len(), 5);
    assert!(rt.state().modal.is_none());
    assert!(rt.state().filter.is_empty());

    // Fuzzy command search uses normal keystrokes, including Ctrl+K routing.
    rt.step(&[Event::Key(KeyEvent::new(
        KeyCode::Char('k'),
        KeyModifiers::CONTROL,
    ))])?;
    advance(&mut rt, &clock, 30)?;
    assert_eq!(rt.focused_key(), Some("palette"));
    for c in "Notebook".chars() {
        rt.step(&[key(KeyCode::Char(c))])?;
    }
    rt.step(&[key(KeyCode::Enter)])?;
    assert_eq!(rt.state().page, "notes");
    advance(&mut rt, &clock, 30)?;
    focus(&mut rt, "notes-editor")?;
    rt.step(&[Event::Paste("\n中文 👩🏽‍💻 e\u{301}".into())])?;
    let rev = rt.state().revision;
    advance(&mut rt, &clock, 7)?;
    assert_eq!(executor.pending(), 0);
    rt.step(&[Event::Paste(" / another thought".into())])?;
    advance(&mut rt, &clock, 7)?;
    assert_eq!(executor.pending(), 0, "Second edit must reset debounce");
    send(&mut rt, Msg::Page("overview".into()))?;
    advance(&mut rt, &clock, 7)?;
    assert!(executor.run_next());
    rt.step(&[])?;
    assert_eq!(rt.state().saved_notes, rt.state().notes);
    send(&mut rt, Msg::Saved(rev, "stale".into()))?;
    assert_eq!(rt.state().saved_notes, rt.state().notes);

    send(&mut rt, Msg::Page("motion".into()))?;
    advance(&mut rt, &clock, 30)?;
    focus(&mut rt, "effect-1")?;
    rt.step(&[key(KeyCode::Enter)])?;
    assert_eq!(rt.state().effect, 1);
    focus(&mut rt, "expand")?;
    rt.step(&[key(KeyCode::Enter)])?;
    assert!(rt.state().expanded);
    advance(&mut rt, &clock, 20)?;
    focus(&mut rt, "quiet")?;
    rt.step(&[key(KeyCode::Char(' '))])?;
    assert!(rt.state().quiet);
    let t = rt.state().time;
    advance(&mut rt, &clock, 30)?;
    assert_eq!(rt.state().time, t);
    focus(&mut rt, "replay")?;
    rt.step(&[key(KeyCode::Enter)])?;
    assert_eq!(rt.state().effect_age, 0.0);
    assert!(text(&rt).contains("Something") || text(&rt).contains("S o m e t h i n g"));

    // With decorative motion paused, scrolling alone changes the small viewport.
    rt.step(&[Event::Resize(80, 24)])?;
    focus(&mut rt, "viewport-motion")?;
    let before = rt.frame().content().to_vec();
    rt.step(&[key(KeyCode::PageDown)])?;
    assert_ne!(
        rt.frame().content(),
        before,
        "The compact viewport must scroll"
    );
    assert!(text(&rt).contains("FIND YOUR RHYTHM"));

    // Every page renders at small, normal and large terminal sizes in each palette.
    for theme in 0..3 {
        send(&mut rt, Msg::Theme)?;
        assert_eq!(rt.state().theme, (theme + 1) % 3);
        for (page, _) in PAGES {
            send(&mut rt, Msg::Page(page.into()))?;
            for (w, h) in [(60, 20), (80, 24), (100, 30), (120, 44), (160, 50)] {
                rt.step(&[Event::Resize(w, h)])?;
                assert_eq!((rt.state().width, rt.state().height), (w, h));
                assert_eq!(rt.frame().content().len(), usize::from(w) * usize::from(h));
            }
        }
    }
    rt.step(&[key(KeyCode::F(1))])?;
    assert!(!rt.step(&[key(KeyCode::Esc)])?.should_quit);
    assert!(rt.state().modal.is_none());
    assert!(rt.step(&[key(KeyCode::Esc)])?.should_quit);
    desktop_interactions()?;
    Ok(())
}

fn mouse(kind: MouseEventKind, x: u16, y: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column: x,
        row: y,
        modifiers: KeyModifiers::empty(),
    })
}
fn focused_bounds(rt: &Runtime<State, Msg>) -> rsille::render::area::Area {
    rt.bounds(rt.focused_path().unwrap()).unwrap()
}

fn desktop_interactions() -> Result {
    use desktop::Action;
    let clock = ManualClock::default();
    let mut rt = app(State::default())
        .with_clock(clock.clone())
        .update(update)
        .view(ui::view)
        .build()
        .size(120, 44);
    rt.step(&[])?;
    advance(&mut rt, &clock, 20)?;
    // Launch through a real button; the workspace remains usable underneath.
    focus(&mut rt, "float")?;
    rt.step(&[key(KeyCode::Enter)])?;
    assert!(rt.state().desk.enabled);
    advance(&mut rt, &clock, 12)?;
    focus(&mut rt, "lens-title")?;
    let title = focused_bounds(&rt);
    let original = rt.state().desk.windows[0].rect;
    let (x, y) = (title.x() + 5, title.y());
    rt.step(&[mouse(MouseEventKind::Down(MouseButton::Left), x, y)])?;
    rt.step(&[mouse(MouseEventKind::Drag(MouseButton::Left), x + 9, y + 3)])?;
    assert_eq!(rt.state().desk.windows[0].rect.x, original.x + 9);
    assert_eq!(rt.state().desk.windows[0].rect.y, original.y + 3);
    let moved = focused_bounds(&rt);
    assert_eq!(
        (moved.x(), moved.y()),
        (title.x() + 9, title.y() + 3),
        "Captured movement must paint without lag"
    );
    // Capture survives leaving the title and clamps safely at the screen edge.
    rt.step(&[mouse(MouseEventKind::Drag(MouseButton::Left), 0, 0)])?;
    rt.step(&[mouse(MouseEventKind::Up(MouseButton::Left), 0, 0)])?;
    assert_eq!(
        (
            rt.state().desk.windows[0].rect.x,
            rt.state().desk.windows[0].rect.y
        ),
        (0, 0)
    );
    let corner = rt.frame().content()[0].content.symbol();
    assert_eq!(
        corner, "╭",
        "The actual window border must follow its state"
    );
    // A released pointer cannot keep moving a window.
    let released = rt.state().desk.windows[0];
    rt.step(&[mouse(MouseEventKind::Drag(MouseButton::Left), 50, 30)])?;
    assert_eq!(rt.state().desk.windows[0], released);

    focus(&mut rt, "arrange-desk")?;
    rt.step(&[key(KeyCode::Enter)])?;
    advance(&mut rt, &clock, 12)?;
    focus(&mut rt, "lens-title")?;
    let title = focused_bounds(&rt);
    rt.step(&[mouse(
        MouseEventKind::Down(MouseButton::Left),
        title.x() + 5,
        title.y(),
    )])?;
    rt.step(&[mouse(
        MouseEventKind::Up(MouseButton::Left),
        title.x() + 5,
        title.y(),
    )])?;
    focus(&mut rt, "lens-resize")?;
    let grip = focused_bounds(&rt);
    let original = rt.state().desk.windows[0].rect;
    let (x, y) = (grip.x() + grip.width() - 2, grip.y());
    rt.step(&[mouse(MouseEventKind::Down(MouseButton::Left), x, y)])?;
    rt.step(&[mouse(MouseEventKind::Drag(MouseButton::Left), x + 7, y + 3)])?;
    assert_eq!(
        (
            rt.state().desk.windows[0].rect.width,
            rt.state().desk.windows[0].rect.height
        ),
        (original.width + 7, original.height + 3)
    );
    rt.step(&[mouse(MouseEventKind::Up(MouseButton::Left), x + 7, y + 3)])?;
    // Keyboard movement and resizing are available on the title too.
    focus(&mut rt, "lens-title")?;
    let before = rt.state().desk.windows[0].rect;
    rt.step(&[key(KeyCode::Left)])?;
    rt.step(&[Event::Key(KeyEvent::new(
        KeyCode::Right,
        KeyModifiers::SHIFT,
    ))])?;
    assert_eq!(rt.state().desk.windows[0].rect.x, before.x - 2);
    assert_eq!(rt.state().desk.windows[0].rect.width, before.width + 2);
    rt.step(&[key(KeyCode::Enter)])?;
    assert_eq!(
        rt.state().desk.windows[0].visibility,
        WindowVisibility::Collapsed
    );
    for _ in 0..35 {
        rt.step(&[key(KeyCode::Tab)])?;
        assert!(
            !matches!(
                rt.focused_key(),
                Some("desk-pause" | "desk-scene" | "lens-resize")
            ),
            "Folded content must leave the focus order"
        );
    }
    focus(&mut rt, "show-lens")?;
    rt.step(&[key(KeyCode::Enter)])?;
    assert_eq!(
        rt.state().desk.windows[0].visibility,
        WindowVisibility::Expanded
    );
    advance(&mut rt, &clock, 12)?;

    // Two overlapping windows must route the shared area to the front one.
    send(&mut rt, Msg::Desk(Action::Arrange))?;
    let signal = rt.state().desk.windows[1]
        .rect
        .translated(CellPoint::new(-26, -7));
    send(&mut rt, Msg::Desk(Action::Rect(1, signal)))?;
    send(
        &mut rt,
        Msg::Desk(Action::Active(Some("floating-0".into()))),
    )?;
    advance(&mut rt, &clock, 12)?;
    let signal = rt.state().desk.windows[1].rect;
    rt.step(&[mouse(
        MouseEventKind::Down(MouseButton::Left),
        signal.x as u16 + 5,
        signal.y as u16 + 1,
    )])?;
    assert_eq!(
        rt.state().desk.active.as_deref(),
        Some("floating-0"),
        "Covered signal title must not receive a click"
    );
    rt.step(&[mouse(
        MouseEventKind::Up(MouseButton::Left),
        signal.x as u16 + 5,
        signal.y as u16 + 1,
    )])?;
    // Click the exposed right border: the signal comes forward.
    rt.step(&[mouse(
        MouseEventKind::Down(MouseButton::Left),
        signal.x as u16 + signal.width - 1,
        signal.y as u16 + 3,
    )])?;
    assert_eq!(rt.state().desk.active.as_deref(), Some("floating-1"));
    rt.step(&[mouse(
        MouseEventKind::Up(MouseButton::Left),
        signal.x as u16 + signal.width - 1,
        signal.y as u16 + 3,
    )])?;
    let cell = signal.y as usize * 120 + signal.x as usize;
    assert_eq!(
        rt.frame().content()[cell]
            .content
            .style
            .colors
            .unwrap()
            .foreground,
        Some(ui::palette(0).mint.into())
    );
    focus(&mut rt, "signal-title")?;
    advance(&mut rt, &clock, 12)?;
    let title = focused_bounds(&rt);
    rt.step(&[mouse(
        MouseEventKind::Down(MouseButton::Left),
        title.x() + title.width() - 2,
        title.y(),
    )])?;
    assert!(!rt.state().desk.windows[1].open);
    focus(&mut rt, "show-signal")?;
    rt.step(&[key(KeyCode::Enter)])?;
    assert!(rt.state().desk.windows[1].open);

    // A modal covers the complete desk, including floating controls and F5.
    rt.step(&[key(KeyCode::F(1))])?;
    advance(&mut rt, &clock, 12)?;
    let windows = rt.state().desk.windows;
    rt.step(&[
        mouse(MouseEventKind::Down(MouseButton::Left), 35, 12),
        key(KeyCode::F(5)),
    ])?;
    assert_eq!(rt.state().desk.windows, windows);
    assert!(rt.state().desk.enabled);
    assert_eq!(rt.state().modal, Some(Modal::Commands));
    rt.step(&[key(KeyCode::Esc)])?;
    // Rearranging only affects placement, preserving the scene and its clock.
    send(&mut rt, Msg::Desk(Action::Arrange))?;
    advance(&mut rt, &clock, 12)?;
    focus(&mut rt, "desk-scene")?;
    rt.step(&[key(KeyCode::Enter)])?;
    advance(&mut rt, &clock, 20)?;
    assert_eq!(rt.state().desk.scene, 1);
    assert!(rt.state().desk.morph > 0.9);
    focus(&mut rt, "desk-pause")?;
    rt.step(&[key(KeyCode::Enter)])?;
    let time = rt.state().desk.time;
    advance(&mut rt, &clock, 10)?;
    assert_eq!(rt.state().desk.time, time);
    rt.step(&[key(KeyCode::Enter)])?;
    rt.step(&[key(KeyCode::F(3))])?;
    advance(&mut rt, &clock, 10)?;
    assert_eq!(
        rt.state().desk.time,
        time,
        "Reduced motion freezes both canvases"
    );
    assert_eq!(rt.state().desk.morph, 1.0);
    focus(&mut rt, "desk-run")?;
    rt.step(&[key(KeyCode::Enter)])?;
    send(&mut rt, Msg::Page("notes".into()))?;
    advance(&mut rt, &clock, 185)?;
    assert_eq!(
        rt.state().completed,
        1,
        "Floating flow controls share application state"
    );
    assert!(rt.state().desk.enabled, "Windows persist across pages");
    rt.step(&[key(KeyCode::F(5))])?;
    assert!(!rt.state().desk.enabled);
    rt.step(&[key(KeyCode::F(5))])?;
    assert!(rt.state().desk.enabled);
    // Resizing cancels capture and keeps both windows reachable, even in tiny TTYs.
    for (w, h) in [(80, 24), (40, 16), (20, 10), (120, 44)] {
        rt.step(&[Event::Resize(w, h)])?;
        for window in rt.state().desk.windows.map(|w| w.rect) {
            assert!(window.x + i32::from(window.width) <= i32::from(w));
            assert!(window.y + i32::from(window.height) <= i32::from(h));
        }
    }
    Ok(())
}

pub(super) fn snapshot(path: &str, mut state: State) -> Result {
    state.time = 3.0;
    state.age = 3.0;
    state.effect_age = 3.0;
    state.desk.time = 3.0;
    let clock = ManualClock::default();
    let mut rt = app(state)
        .with_clock(clock.clone())
        .update(update)
        .view(ui::view)
        .build()
        .size(120, 44);
    rt.step(&[])?;
    clock.advance(Duration::from_secs(2));
    rt.step(&[])?;
    std::fs::write(path, svg(rt.frame()))?;
    println!("Wrote {path}");
    Ok(())
}

fn svg(buffer: &rsille::render::buffer::Buffer) -> String {
    fn color(value: Option<crossterm::style::Color>, fallback: &str) -> String {
        match value {
            Some(crossterm::style::Color::Rgb { r, g, b }) => format!("#{r:02x}{g:02x}{b:02x}"),
            _ => fallback.to_owned(),
        }
    }
    let width = usize::from(buffer.size().width);
    let w = width * 10 + 32;
    let h = usize::from(buffer.size().height) * 20 + 32;
    let mut output = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\"><rect width=\"100%\" height=\"100%\" fill=\"#0f161d\"/><g font-family=\"DejaVu Sans Mono,monospace\" font-size=\"16\">");
    // Paint all backgrounds first: wide graphemes may cover multiple cells.
    for (i, cell) in buffer.content().iter().enumerate() {
        let (x, y) = (i % width * 10 + 16, i / width * 20 + 16);
        let bg = color(
            cell.content.style.colors.and_then(|c| c.background),
            "#0f161d",
        );
        let _ = write!(
            output,
            "<rect x=\"{x}\" y=\"{y}\" width=\"10\" height=\"20\" fill=\"{bg}\"/>"
        );
    }
    for (i, cell) in buffer
        .content()
        .iter()
        .enumerate()
        .filter(|(_, c)| c.owner.is_none())
    {
        let (x, y) = (i % width * 10 + 16, i / width * 20 + 16);
        let fg = color(
            cell.content.style.colors.and_then(|c| c.foreground),
            "#e2ebeb",
        );
        let symbol = cell.content.symbol();
        if let Some(bits) = symbol
            .chars()
            .next()
            .map(u32::from)
            .filter(|c| (0x2800..=0x28ff).contains(c))
        {
            // Explicit dots make exported braille independent of installed fonts.
            for (bit, (dx, dy)) in [
                (0, 0),
                (0, 1),
                (0, 2),
                (1, 0),
                (1, 1),
                (1, 2),
                (0, 3),
                (1, 3),
            ]
            .into_iter()
            .enumerate()
            {
                if (bits - 0x2800) & (1 << bit) != 0 {
                    let _ = write!(
                        output,
                        "<circle cx=\"{}\" cy=\"{}\" r=\"0.95\" fill=\"{fg}\"/>",
                        x as f64 + 2.5 + f64::from(dx) * 5.0,
                        y as f64 + 2.5 + f64::from(dy) * 5.0
                    );
                }
            }
            continue;
        }
        let glyph = symbol
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        let bold = cell
            .content
            .style
            .attr
            .is_some_and(|a| a.has(crossterm::style::Attribute::Bold));
        let font = if symbol
            .chars()
            .any(|c| ('\u{2500}'..='\u{257f}').contains(&c))
        {
            "DejaVu Sans"
        } else {
            "DejaVu Sans Mono"
        };
        let _ = write!(output, "<text x=\"{x}\" y=\"{}\" fill=\"{fg}\" font-family=\"{font}\" font-weight=\"{}\" xml:space=\"preserve\">{glyph}</text>", y + 16, if bold { "bold" } else { "normal" });
    }
    output.push_str("</g></svg>");
    output
}

#[cfg(test)]
mod tests {
    #[test]
    fn workspace_interactions() {
        super::run().unwrap();
    }
}
