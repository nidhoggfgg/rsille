//! Exercise the actual runtime, input routing and filesystem, with no TTY.
use super::*;
use rsille::tui::runtime::{ManualClock, ManualExecutor};
use std::fmt::Write;

fn key(code: KeyCode, modifiers: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(code, modifiers))
}
fn mouse(kind: MouseEventKind, x: u16, y: u16, modifiers: KeyModifiers) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column: x,
        row: y,
        modifiers,
    })
}
fn send(rt: &mut Runtime<State, Msg>, msg: Msg) -> Result {
    rt.handle().send(msg)?;
    rt.step(&[])?;
    Ok(())
}
fn focus(rt: &mut Runtime<State, Msg>, target: &str) -> Result {
    for _ in 0..50 {
        if rt.focused_key() == Some(target) {
            return Ok(());
        }
        rt.step(&[key(KeyCode::Tab, KeyModifiers::empty())])?;
    }
    panic!("Cannot focus {target}; focused {:?}", rt.focused_key());
}
fn text(rt: &Runtime<State, Msg>) -> String {
    rt.frame()
        .content()
        .chunks(rt.frame().size().width as usize)
        .map(|row| {
            row.iter()
                .filter(|c| c.owner.is_none())
                .map(|c| c.content.symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
fn settle(rt: &mut Runtime<State, Msg>, executor: &ManualExecutor) -> Result {
    while executor.run_next() {}
    rt.step(&[])?;
    assert!(rt.state().busy.is_none());
    Ok(())
}
fn tile_point(bounds: rsille::render::area::Area, index: usize, list: bool) -> (u16, u16) {
    let cols = if list {
        1
    } else {
        ((bounds.width().saturating_sub(4)) / 18).max(1) as usize
    };
    let step = bounds.width().saturating_sub(4) / cols as u16;
    (
        bounds.x() + 2 + (index % cols) as u16 * step + 3,
        bounds.y() + 2 + (index / cols) as u16 * if list { 2 } else { 6 } + u16::from(!list),
    )
}
fn begin_drag(rt: &mut Runtime<State, Msg>, ctrl: bool) -> Result<rsille::render::area::Area> {
    let paths: Vec<_> = ["main.rs", "Notes.txt", "Roadmap.md"]
        .iter()
        .map(|name| rt.state().cwd.join(name))
        .collect();
    let lead = paths[0].clone();
    send(rt, Msg::Select(paths, Some(lead.clone())))?;
    focus(rt, "files")?;
    let bounds = rt
        .layout_geometry(rt.focused_path().unwrap())
        .unwrap()
        .display;
    let index = rt
        .state()
        .visible()
        .iter()
        .position(|e| e.path == lead)
        .unwrap();
    let (x, y) = tile_point(bounds, index, rt.state().list);
    rt.step(&[mouse(
        MouseEventKind::Down(MouseButton::Left),
        x,
        y,
        if ctrl {
            KeyModifiers::CONTROL
        } else {
            KeyModifiers::empty()
        },
    )])?;
    Ok(bounds)
}
fn drag_feedback() -> Result {
    use rsille::tui::event::ModifierKeyCode;
    let demo = files::Demo::new()?;
    let executor = ManualExecutor::default();
    let mut rt = app(State::new(demo.0.clone(), true))
        .with_executor(executor.clone())
        .update(update)
        .view(ui::view)
        .build()
        .size(120, 42);
    rt.step(&[])?;
    let no = KeyModifiers::empty();
    let bounds = begin_drag(&mut rt, false)?;
    assert!(
        rt.state().drag_preview.is_none(),
        "a click must not show a drag image"
    );
    let target = rt
        .state()
        .visible()
        .iter()
        .position(|e| e.name == "Design")
        .unwrap();
    let (tx, ty) = tile_point(bounds, target, false);
    let source = rt
        .state()
        .visible()
        .iter()
        .position(|e| e.name == "main.rs")
        .unwrap();
    let (sx, sy) = tile_point(bounds, source, false);
    let source_cell = usize::from(sy) * 120 + usize::from(sx - 3);
    let source_color = rt.frame().content()[source_cell]
        .content
        .style
        .colors
        .unwrap()
        .background;
    rt.step(&[mouse(MouseEventKind::Drag(MouseButton::Left), tx, ty, no)])?;
    let visual = rt.state().drag_preview.as_ref().unwrap();
    assert_eq!(visual.total, 3);
    assert_eq!(visual.entries[0].name, "main.rs");
    assert_eq!(visual.pointer, CellPoint::new(tx.into(), ty.into()));
    assert_eq!(visual.target.path(), Some(&demo.0.join("Design")));
    assert!(
        text(&rt).contains("Move to Design"),
        "paint the operation alongside the moving files"
    );
    assert_ne!(
        rt.frame().content()[source_cell]
            .content
            .style
            .colors
            .unwrap()
            .background,
        source_color,
        "lifted source cards must fade"
    );
    let before = rt.frame().content().to_vec();
    rt.step(&[mouse(MouseEventKind::Drag(MouseButton::Left), 70, 37, no)])?;
    assert_ne!(
        rt.frame().content(),
        before,
        "the image must follow the pointer"
    );
    assert_eq!(
        rt.state().drag_preview.as_ref().unwrap().target,
        drag::Target::Blocked("Choose a folder")
    );
    rt.step(&[mouse(MouseEventKind::Drag(MouseButton::Left), tx, ty, no)])?;
    rt.step(&[key(
        KeyCode::Modifier(ModifierKeyCode::LeftControl),
        KeyModifiers::CONTROL,
    )])?;
    assert!(rt.state().drag_preview.as_ref().unwrap().copy);
    assert!(text(&rt).contains("Copy to Design"));
    rt.step(&[Event::Key(KeyEvent::new_with_kind(
        KeyCode::Modifier(ModifierKeyCode::LeftControl),
        no,
        KeyEventKind::Release,
    ))])?;
    assert!(!rt.state().drag_preview.as_ref().unwrap().copy);
    let file = rt
        .state()
        .visible()
        .iter()
        .position(|e| e.name == "theme.json")
        .unwrap();
    let (fx, fy) = tile_point(bounds, file, false);
    rt.step(&[mouse(MouseEventKind::Drag(MouseButton::Left), fx, fy, no)])?;
    assert_eq!(
        rt.state().drag_preview.as_ref().unwrap().target,
        drag::Target::Blocked("Not a folder")
    );
    rt.step(&[mouse(MouseEventKind::Drag(MouseButton::Left), sx, sy, no)])?;
    assert_eq!(
        rt.state().drag_preview.as_ref().unwrap().target,
        drag::Target::Blocked("Same selection")
    );
    // The paint layer must follow capture over navigation without stealing the release.
    rt.step(&[mouse(MouseEventKind::Drag(MouseButton::Left), 118, 1, no)])?;
    assert!(text(&rt).contains("Outside file area"));
    rt.step(&[mouse(MouseEventKind::Up(MouseButton::Left), 118, 1, no)])?;
    assert!(rt.state().drag_preview.is_none());
    assert!(rt.state().busy.is_none());
    // Ctrl held from pickup must preserve the entire group, then copy on release.
    let bounds = begin_drag(&mut rt, true)?;
    let (tx, ty) = tile_point(bounds, target, false);
    rt.step(&[mouse(
        MouseEventKind::Drag(MouseButton::Left),
        tx,
        ty,
        KeyModifiers::CONTROL,
    )])?;
    assert_eq!(rt.state().drag_preview.as_ref().unwrap().total, 3);
    assert_eq!(rt.state().selected.len(), 3);
    // The payload being dragged remains fixed even if application selection changes.
    send(
        &mut rt,
        Msg::Select(
            vec![demo.0.join("Welcome.md")],
            Some(demo.0.join("Welcome.md")),
        ),
    )?;
    rt.step(&[mouse(
        MouseEventKind::Up(MouseButton::Left),
        tx,
        ty,
        KeyModifiers::CONTROL,
    )])?;
    assert!(rt.state().drag_preview.is_none());
    settle(&mut rt, &executor)?;
    for name in ["main.rs", "Notes.txt", "Roadmap.md"] {
        assert_eq!(
            std::fs::read(demo.0.join(name))?,
            std::fs::read(demo.0.join("Design").join(name))?
        );
    }
    send(&mut rt, Msg::View)?;
    let bounds = begin_drag(&mut rt, false)?;
    let (tx, ty) = tile_point(bounds, target, true);
    rt.step(&[mouse(MouseEventKind::Drag(MouseButton::Left), tx, ty, no)])?;
    assert!(
        text(&rt).contains("Move to Design"),
        "list mode needs the same drag image"
    );
    rt.step(&[key(KeyCode::Esc, no)])?;
    assert!(rt.state().drag_preview.is_none());
    assert!(rt.state().busy.is_none());
    for event in [
        Event::FocusLost,
        Event::Resize(80, 24),
        key(KeyCode::F(1), no),
    ] {
        rt.step(&[Event::Resize(120, 42)])?;
        let bounds = begin_drag(&mut rt, false)?;
        let (tx, ty) = tile_point(bounds, target, true);
        rt.step(&[mouse(MouseEventKind::Drag(MouseButton::Left), tx, ty, no)])?;
        assert!(rt.state().drag_preview.is_some());
        rt.step(&[event])?;
        assert!(
            rt.state().drag_preview.is_none(),
            "cancellation must remove the ghost immediately"
        );
        if rt.state().modal.is_some() {
            rt.step(&[key(KeyCode::Esc, no)])?;
        }
    }
    // Keep the paint-only image within smaller windows, including pointer edge flips.
    for (w, h) in [(80, 24), (40, 16), (20, 10)] {
        rt.step(&[Event::Resize(w, h)])?;
        send(&mut rt, Msg::Filter("main".into()))?;
        let path = demo.0.join("main.rs");
        send(&mut rt, Msg::Select(vec![path.clone()], Some(path)))?;
        focus(&mut rt, "files")?;
        let bounds = rt
            .layout_geometry(rt.focused_path().unwrap())
            .unwrap()
            .display;
        let (x, y) = tile_point(bounds, 0, true);
        rt.step(&[mouse(MouseEventKind::Down(MouseButton::Left), x, y, no)])?;
        rt.step(&[mouse(
            MouseEventKind::Drag(MouseButton::Left),
            w - 1,
            h - 2,
            no,
        )])?;
        assert!(rt.state().drag_preview.is_some());
        assert_eq!(rt.frame().size().width, w);
        rt.step(&[key(KeyCode::Esc, no)])?;
        assert!(rt.state().drag_preview.is_none());
    }
    Ok(())
}
pub fn run() -> Result {
    let demo = files::Demo::new()?;
    let executor = ManualExecutor::default();
    let mut rt = app(State::new(demo.0.clone(), true))
        .with_executor(executor.clone())
        .update(update)
        .view(ui::view)
        .build()
        .size(120, 42);
    rt.step(&[])?;
    assert!(!text(&rt).contains("WORKSPACE"));
    assert!(text(&rt).contains("Projects"));
    focus(&mut rt, "files")?;
    let bounds = rt
        .layout_geometry(rt.focused_path().unwrap())
        .unwrap()
        .display;
    assert!(
        bounds.width() >= 110 && bounds.height() >= 36,
        "the file pane should occupy most of the window"
    );
    let (x, y) = (bounds.x(), bounds.y());
    let no = KeyModifiers::empty();
    let ctrl = KeyModifiers::CONTROL;
    // Begin on the gutter, cross two cards, and inspect real controlled selection.
    rt.step(&[mouse(MouseEventKind::Down(MouseButton::Left), x, y + 1, no)])?;
    rt.step(&[mouse(
        MouseEventKind::Drag(MouseButton::Left),
        x + 36,
        y + 6,
        no,
    )])?;
    assert!(
        rt.state().selected.len() >= 2,
        "marquee must select multiple cards"
    );
    let masked = rt.frame().content().to_vec();
    rt.step(&[mouse(
        MouseEventKind::Up(MouseButton::Left),
        x + 36,
        y + 6,
        no,
    )])?;
    for row in 0..bounds.height() {
        for col in 0..bounds.width() {
            let index = usize::from(y + row) * 120 + usize::from(x + col);
            let during = &masked[index].content;
            let after = &rt.frame().content()[index].content;
            assert_eq!(during.symbol(), after.symbol(), "mask must preserve glyphs");
            let during_colors = during.style.colors.unwrap();
            let after_colors = after.style.colors.unwrap();
            assert_eq!(during_colors.foreground, after_colors.foreground);
            if col <= 36 && (1..=6).contains(&row) {
                assert_ne!(
                    during_colors.background, after_colors.background,
                    "the whole selection rectangle must be tinted until release"
                );
            } else {
                assert_eq!(
                    during_colors.background, after_colors.background,
                    "mask must stay within the selection rectangle"
                );
            }
        }
    }
    // Capture must release even outside the widget; cancellation restores prior selection.
    let before = rt.state().selected.clone();
    rt.step(&[mouse(
        MouseEventKind::Down(MouseButton::Left),
        x,
        y + 1,
        ctrl,
    )])?;
    rt.step(&[mouse(
        MouseEventKind::Drag(MouseButton::Left),
        119,
        41,
        ctrl,
    )])?;
    rt.step(&[Event::FocusLost])?;
    assert_eq!(rt.state().selected, before);
    focus(&mut rt, "files")?;
    rt.step(&[
        key(KeyCode::Home, no),
        key(KeyCode::Right, KeyModifiers::SHIFT),
    ])?;
    assert_eq!(rt.state().selected.len(), 2);
    rt.step(&[key(KeyCode::Char('a'), ctrl)])?;
    assert_eq!(rt.state().selected.len(), rt.state().visible().len());
    send(&mut rt, Msg::Escape)?;
    // A whole down/drag/up batch must keep the selection and reach a real folder.
    let drag_sources: Vec<_> = ["Roadmap.md", "main.rs"]
        .iter()
        .map(|n| demo.0.join(n))
        .collect();
    send(
        &mut rt,
        Msg::Select(drag_sources.clone(), Some(drag_sources[0].clone())),
    )?;
    let entries = rt.state().visible();
    let source_i = entries
        .iter()
        .position(|e| e.path == drag_sources[0])
        .unwrap();
    let target_i = entries.iter().position(|e| e.name == "Design").unwrap();
    let cols = ((bounds.width().saturating_sub(4)) / 18).max(1) as usize;
    let step = bounds.width().saturating_sub(4) / cols as u16;
    let point = |i: usize| {
        (
            x + 2 + (i % cols) as u16 * step + 3,
            y + 2 + (i / cols) as u16 * 6 + 1,
        )
    };
    let (sx, sy) = point(source_i);
    let (tx, ty) = point(target_i);
    rt.step(&[
        mouse(MouseEventKind::Down(MouseButton::Left), sx, sy, no),
        mouse(MouseEventKind::Drag(MouseButton::Left), tx, ty, ctrl),
        mouse(MouseEventKind::Up(MouseButton::Left), tx, ty, ctrl),
    ])?;
    assert!(
        rt.state().busy.is_some(),
        "a captured drag must start a filesystem job"
    );
    settle(&mut rt, &executor)?;
    for source in &drag_sources {
        assert!(source.exists());
        assert_eq!(
            std::fs::read(source)?,
            std::fs::read(demo.0.join("Design").join(source.file_name().unwrap()))?
        );
    }
    // The same batch with no modifier moves every selected file, then undo restores it.
    rt.step(&[
        mouse(MouseEventKind::Down(MouseButton::Left), sx, sy, no),
        mouse(MouseEventKind::Drag(MouseButton::Left), tx, ty, no),
        mouse(MouseEventKind::Up(MouseButton::Left), tx, ty, no),
    ])?;
    settle(&mut rt, &executor)?;
    assert!(
        rt.state().status.contains("failed"),
        "existing destination files must not be overwritten"
    );
    for source in &drag_sources {
        assert!(source.exists());
    }
    // Move into another target with no conflicts.
    let target_i = rt
        .state()
        .visible()
        .iter()
        .position(|e| e.name == "Music")
        .unwrap();
    let (tx, ty) = point(target_i);
    rt.step(&[
        mouse(MouseEventKind::Down(MouseButton::Left), sx, sy, no),
        mouse(MouseEventKind::Drag(MouseButton::Left), tx, ty, no),
        mouse(MouseEventKind::Up(MouseButton::Left), tx, ty, no),
    ])?;
    settle(&mut rt, &executor)?;
    for source in &drag_sources {
        assert!(!source.exists());
    }
    send(&mut rt, Msg::Undo)?;
    settle(&mut rt, &executor)?;
    for source in &drag_sources {
        assert!(source.exists());
    }
    // Mouse Ctrl toggles, while a plain click collapses a group on release.
    send(&mut rt, Msg::Escape)?;
    rt.step(&[
        mouse(MouseEventKind::Down(MouseButton::Left), sx, sy, ctrl),
        mouse(MouseEventKind::Up(MouseButton::Left), sx, sy, ctrl),
    ])?;
    assert_eq!(rt.state().selected.len(), 1);
    rt.step(&[
        mouse(MouseEventKind::Down(MouseButton::Left), sx, sy, ctrl),
        mouse(MouseEventKind::Up(MouseButton::Left), sx, sy, ctrl),
    ])?;
    assert!(rt.state().selected.is_empty());
    // Double-click a folder, then return through actual history.
    let (dx, dy) = point(0);
    let click = [
        mouse(MouseEventKind::Down(MouseButton::Left), dx, dy, no),
        mouse(MouseEventKind::Up(MouseButton::Left), dx, dy, no),
    ];
    rt.step(&click)?;
    rt.step(&click)?;
    assert_eq!(rt.state().cwd, demo.0.join("Archive"));
    send(&mut rt, Msg::Back)?;
    let original = rt.state().cwd.clone();
    send(&mut rt, Msg::Up)?;
    assert_eq!(
        rt.state().cwd,
        original,
        "demo navigation must stay inside its workspace"
    );
    // Right click selects its target and opens rsille's native roving menu.
    rt.step(&[mouse(MouseEventKind::Down(MouseButton::Right), sx, sy, no)])?;
    assert_eq!(rt.state().modal, Some(Modal::Actions));
    assert_eq!(rt.focused_key(), Some("file-actions"));
    rt.step(&[key(KeyCode::Down, no), key(KeyCode::Enter, no)])?;
    assert!(rt.state().modal.is_none());
    assert_eq!(rt.state().clipboard.as_ref().unwrap().1.len(), 1);
    // Actual clipboard → navigate → background copy → bytes on disk.
    let source = demo.0.join("Welcome.md");
    let destination = demo.0.join("Projects");
    send(
        &mut rt,
        Msg::Select(vec![source.clone()], Some(source.clone())),
    )?;
    focus(&mut rt, "files")?;
    rt.step(&[key(KeyCode::Char('c'), ctrl)])?;
    assert_eq!(
        rt.state().clipboard.as_ref().unwrap().1,
        vec![source.clone()]
    );
    send(&mut rt, Msg::Navigate(destination.clone()))?;
    focus(&mut rt, "files")?;
    rt.step(&[key(KeyCode::Char('v'), ctrl)])?;
    assert!(rt.state().busy.is_some());
    settle(&mut rt, &executor)?;
    assert_eq!(
        std::fs::read(&source)?,
        std::fs::read(destination.join("Welcome.md"))?
    );
    send(&mut rt, Msg::Paste)?;
    settle(&mut rt, &executor)?;
    assert!(destination.join("Welcome (copy 1).md").is_file());
    // Move, undo, rename, trash, undo, all through production update paths.
    send(&mut rt, Msg::Back)?;
    let note = demo.0.join("Notes.txt");
    send(&mut rt, Msg::Select(vec![note.clone()], Some(note.clone())))?;
    send(&mut rt, Msg::Clipboard(true))?;
    send(&mut rt, Msg::Navigate(destination.clone()))?;
    send(&mut rt, Msg::Paste)?;
    settle(&mut rt, &executor)?;
    assert!(!note.exists());
    assert!(destination.join("Notes.txt").exists());
    send(&mut rt, Msg::Undo)?;
    settle(&mut rt, &executor)?;
    assert!(note.exists());
    send(&mut rt, Msg::Navigate(demo.0.clone()))?;
    send(&mut rt, Msg::Select(vec![note.clone()], Some(note.clone())))?;
    send(&mut rt, Msg::Modal(Modal::Rename))?;
    assert_eq!(rt.focused_key(), Some("name-input"));
    assert!(
        text(&rt).contains("Save"),
        "form actions must fit inside the modal"
    );
    send(&mut rt, Msg::Draft("../escape".into()))?;
    send(&mut rt, Msg::Submit)?;
    assert!(!rt.state().error.is_empty());
    assert!(note.exists());
    send(&mut rt, Msg::Draft("灵感.txt".into()))?;
    send(&mut rt, Msg::Submit)?;
    let renamed = demo.0.join("灵感.txt");
    assert!(renamed.exists());
    send(
        &mut rt,
        Msg::Select(vec![renamed.clone()], Some(renamed.clone())),
    )?;
    send(&mut rt, Msg::Modal(Modal::Trash))?;
    let history = rt.state().history.clone();
    rt.step(&[mouse(MouseEventKind::Down(MouseButton::Left), 3, 5, no)])?;
    assert_eq!(
        rt.state().history,
        history,
        "modal must block background navigation"
    );
    send(&mut rt, Msg::Submit)?;
    settle(&mut rt, &executor)?;
    assert!(!renamed.exists());
    send(&mut rt, Msg::Undo)?;
    settle(&mut rt, &executor)?;
    assert!(renamed.exists());
    send(&mut rt, Msg::Undo)?;
    settle(&mut rt, &executor)?;
    assert!(note.exists());
    // Create a folder via the actual Unicode text input and Enter submission.
    send(&mut rt, Msg::Modal(Modal::NewFolder))?;
    rt.step(&[Event::Paste("新文件夹".into())])?;
    rt.step(&[key(KeyCode::Enter, no)])?;
    assert!(demo.0.join("新文件夹").is_dir());
    // Search is an actual controlled text widget.
    focus(&mut rt, "search")?;
    rt.step(&[Event::Paste("Welcome".into())])?;
    assert_eq!(rt.state().visible().len(), 1);
    send(&mut rt, Msg::Filter(String::new()))?;
    send(&mut rt, Msg::Hidden)?;
    assert!(rt
        .state()
        .visible()
        .iter()
        .any(|e| e.name == ".hidden-note"));
    send(&mut rt, Msg::View)?;
    assert!(text(&rt).contains("Type"));
    focus(&mut rt, "files")?;
    rt.step(&[key(KeyCode::Home, no)])?;
    rt.step(&[
        key(KeyCode::Down, no),
        key(KeyCode::Down, no),
        key(KeyCode::Down, no),
    ])?;
    assert_eq!(
        rt.state().active,
        Some(rt.state().visible()[3].path.clone()),
        "batched keys must use the latest cursor"
    );
    rt.step(&[key(KeyCode::End, no)])?;
    assert!(
        text(&rt).contains("Welcome.md"),
        "End must scroll the last file into view"
    );
    send(&mut rt, Msg::View)?;
    focus(&mut rt, "files")?;
    let b = rt
        .layout_geometry(rt.focused_path().unwrap())
        .unwrap()
        .display;
    rt.step(&[mouse(MouseEventKind::ScrollDown, b.x() + 4, b.y() + 5, no)])?;
    rt.step(&[mouse(
        MouseEventKind::Down(MouseButton::Left),
        b.x(),
        b.y() + 1,
        no,
    )])?;
    rt.step(&[mouse(
        MouseEventKind::Drag(MouseButton::Left),
        b.x() + 25,
        b.y() + 10,
        no,
    )])?;
    rt.step(&[key(KeyCode::Esc, no)])?;
    assert!(
        rt.state().selected.len() <= 1,
        "Escape cancels a scrolled marquee"
    );
    send(&mut rt, Msg::Modal(Modal::Help))?;
    assert!(text(&rt).contains("Ctrl+Q"));
    assert!(
        text(&rt).contains("Got it"),
        "all help content must fit inside its dialog"
    );
    rt.step(&[key(KeyCode::Esc, no)])?;
    // Directory refresh during a pointer sequence must invalidate stale item indices.
    focus(&mut rt, "files")?;
    let b = rt
        .layout_geometry(rt.focused_path().unwrap())
        .unwrap()
        .display;
    rt.step(&[mouse(
        MouseEventKind::Down(MouseButton::Left),
        b.x() + 4,
        b.y() + 3,
        no,
    )])?;
    std::fs::write(
        demo.0.join("000-new.txt"),
        "refresh changes the sorted order",
    )?;
    send(&mut rt, Msg::Refresh)?;
    rt.step(&[mouse(
        MouseEventKind::Up(MouseButton::Left),
        b.x() + 4,
        b.y() + 3,
        no,
    )])?;
    assert!(rt.state().modal.is_none());
    rt.step(&[key(KeyCode::Char('f'), ctrl)])?;
    assert_eq!(rt.state().modal, Some(Modal::Search));
    rt.step(&[Event::Paste("Welcome".into())])?;
    rt.step(&[key(KeyCode::Enter, no)])?;
    assert_eq!(rt.state().visible().len(), 1);
    send(&mut rt, Msg::Filter(String::new()))?;
    // File previews are available on demand without a permanent inspector pane.
    send(&mut rt, Msg::Open(demo.0.join("Welcome.md")))?;
    assert_eq!(rt.state().modal, Some(Modal::Preview));
    assert!(text(&rt).contains("# Welcome to Orbit"));
    focus(&mut rt, "close-preview")?;
    rt.step(&[key(KeyCode::Enter, no)])?;
    assert!(rt.state().modal.is_none());
    focus(&mut rt, "more")?;
    rt.step(&[key(KeyCode::Enter, no)])?;
    assert_eq!(rt.focused_key(), Some("file-actions"));
    send(&mut rt, Msg::Action("view".into()))?;
    assert!(rt.state().list);
    // Repeat all layouts at small and large terminal sizes.
    for (w, h) in [
        (160, 48),
        (100, 30),
        (70, 22),
        (40, 16),
        (20, 10),
        (120, 42),
    ] {
        rt.step(&[Event::Resize(w, h)])?;
        assert_eq!(rt.frame().size().width, w);
        send(&mut rt, Msg::Modal(Modal::Help))?;
        rt.step(&[key(KeyCode::Esc, no)])?;
        assert!(rt.state().modal.is_none());
    }
    filesystem_checks()?;
    drag_feedback()?;
    println!("Orbit verified: marquee, floating drag previews, drop feedback, capture cancellation, keyboard ranges, clipboard, copies, moves, undo, Unicode rename/create, trash, modal isolation, search, hidden files and resize.");
    Ok(())
}
fn filesystem_checks() -> Result {
    let demo = files::Demo::new()?;
    let project = demo.0.join("Projects");
    let before = std::fs::read(demo.0.join("Welcome.md"))?;
    let conflict = Job {
        sources: vec![demo.0.join("Welcome.md")],
        destination: demo.0.clone(),
        operation: Operation::Move,
    }
    .run(|_, _| {});
    assert_eq!(conflict.completed, 0);
    assert_eq!(std::fs::read(demo.0.join("Welcome.md"))?, before);
    let recursive = Job {
        sources: vec![demo.0.clone()],
        destination: project.clone(),
        operation: Operation::Copy,
    }
    .run(|_, _| {});
    assert_eq!(recursive.completed, 0);
    assert!(!recursive.errors.is_empty());
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&demo.0, project.join("cycle"))?;
        files::copy_new(&project, &demo.0.join("Project-copy"))?;
        assert!(std::fs::symlink_metadata(demo.0.join("Project-copy/cycle"))?.is_symlink());
        std::os::unix::fs::symlink("missing", project.join("broken"))?;
        let conflict = files::move_new(&demo.0.join("Notes.txt"), &project.join("broken"));
        assert!(conflict.is_err());
        assert!(demo.0.join("Notes.txt").exists());
    }
    Ok(())
}
pub fn snapshot_drag(state: State, path: &str) -> Result {
    let mut rt = app(state)
        .update(update)
        .view(ui::view)
        .build()
        .size(120, 42);
    rt.step(&[])?;
    let bounds = begin_drag(&mut rt, false)?;
    let index = rt
        .state()
        .visible()
        .iter()
        .position(|e| e.name == "Design")
        .ok_or("drag snapshot needs the demo Design folder")?;
    let (x, y) = tile_point(bounds, index, false);
    rt.step(&[mouse(
        MouseEventKind::Drag(MouseButton::Left),
        x,
        y,
        KeyModifiers::CONTROL,
    )])?;
    std::fs::write(path, svg(rt.frame()))?;
    println!("Wrote {path}");
    Ok(())
}
pub fn snapshot(state: State, path: &str) -> Result {
    let clock = ManualClock::default();
    let mut rt = app(state)
        .with_clock(clock.clone())
        .update(update)
        .view(ui::view)
        .build()
        .size(120, 42);
    rt.step(&[])?;
    let selected: Vec<_> = rt
        .state()
        .visible()
        .into_iter()
        .filter(|e| matches!(e.name.as_str(), "main.rs" | "Roadmap.md" | "Notes.txt"))
        .map(|e| e.path)
        .collect();
    let active = selected
        .iter()
        .find(|p| p.file_name().is_some_and(|n| n == "Roadmap.md"))
        .cloned();
    send(&mut rt, Msg::Select(selected, active))?;
    clock.advance(Duration::from_secs(1));
    rt.step(&[])?;
    std::fs::write(path, svg(rt.frame()))?;
    println!("Wrote {path}");
    Ok(())
}
fn svg(frame: &rsille::render::buffer::Buffer) -> String {
    fn color(c: Option<crossterm::style::Color>, fallback: &str) -> String {
        if let Some(crossterm::style::Color::Rgb { r, g, b }) = c {
            format!("#{r:02x}{g:02x}{b:02x}")
        } else {
            fallback.to_owned()
        }
    }
    let width = frame.size().width as usize;
    let mut out = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\"><rect width=\"100%\" height=\"100%\" fill=\"#101622\"/><g font-family=\"DejaVu Sans Mono,monospace\" font-size=\"15\">",width*10+32,usize::from(frame.size().height)*20+32);
    for (i, c) in frame.content().iter().enumerate() {
        let bg = color(c.content.style.colors.and_then(|c| c.background), "#101622");
        let _ = write!(
            out,
            "<rect x=\"{}\" y=\"{}\" width=\"10\" height=\"20\" fill=\"{bg}\"/>",
            i % width * 10 + 16,
            i / width * 20 + 16
        );
    }
    for (i, c) in frame
        .content()
        .iter()
        .enumerate()
        .filter(|(_, c)| c.owner.is_none())
    {
        let fg = color(c.content.style.colors.and_then(|c| c.foreground), "#dfe8f5");
        // Terminal box drawing joins at cell boundaries. SVG text has font side
        // bearings, so export those strokes as paths to preserve the joins.
        let path = match c.content.symbol().as_ref() {
            "─" => Some("M0 10H10"),
            "│" => Some("M5 0V20"),
            "╭" => Some("M5 20V15Q5 10 10 10"),
            "╮" => Some("M0 10Q5 10 5 15V20"),
            "╰" => Some("M5 0V5Q5 10 10 10"),
            "╯" => Some("M0 10Q5 10 5 5V0"),
            "┌" => Some("M5 20V10H10"),
            "┐" => Some("M0 10H5V20"),
            "└" => Some("M5 0V10H10"),
            "┘" => Some("M0 10H5V0"),
            "┬" => Some("M0 10H10M5 10V20"),
            "┴" => Some("M0 10H10M5 0V10"),
            "├" => Some("M5 0V20M5 10H10"),
            "┤" => Some("M5 0V20M0 10H5"),
            "┼" => Some("M0 10H10M5 0V20"),
            _ => None,
        };
        if let Some(path) = path {
            let _ = write!(
                out,
                "<path transform=\"translate({} {})\" d=\"{path}\" fill=\"none\" stroke=\"{fg}\" stroke-width=\"1\"/>",
                i % width * 10 + 16,
                i / width * 20 + 16
            );
            continue;
        }
        let glyph = c
            .content
            .symbol()
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        if glyph.trim().is_empty() {
            continue;
        }
        let _ = write!(
            out,
            "<text x=\"{}\" y=\"{}\" fill=\"{fg}\">{glyph}</text>",
            i % width * 10 + 16,
            i / width * 20 + 32
        );
    }
    out.push_str("</g></svg>");
    out
}
#[cfg(test)]
mod tests {
    #[test]
    fn end_to_end() {
        super::run().unwrap();
    }
}
