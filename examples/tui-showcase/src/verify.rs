//! Deterministic integration checks using the real app, widgets and input routing.
use super::*;

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::empty()))
}

fn send(runtime: &mut Runtime<State, Msg>, message: Msg) -> Result<(), Box<dyn std::error::Error>> {
    runtime.handle().send(message).expect("live runtime");
    runtime.step(&[])?;
    Ok(())
}

fn focus(
    runtime: &mut Runtime<State, Msg>,
    target: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    for _ in 0..50 {
        if runtime.focused_key() == Some(target) {
            return Ok(());
        }
        runtime.step(&[key(KeyCode::Tab)])?;
    }
    panic!(
        "Could not focus {target}; current focus: {:?}",
        runtime.focused_key()
    );
}

fn frame_text(runtime: &Runtime<State, Msg>) -> String {
    runtime
        .frame()
        .content()
        .chunks(runtime.frame().size().width as usize)
        .map(|row| {
            row.iter()
                .filter(|cell| cell.owner.is_none())
                .map(|cell| cell.content.symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// The same app, update and view run with a manual scheduler and worker.
pub(super) fn run() -> Result<(), Box<dyn std::error::Error>> {
    use rsille::tui::runtime::{ManualClock, ManualExecutor};
    let clock = ManualClock::default();
    let executor = ManualExecutor::default();
    let mut runtime = app()
        .with_clock(clock.clone())
        .with_executor(executor.clone())
        .update(update)
        .view(view)
        .build()
        .size(120, 45);
    runtime.step(&[])?;
    for (page, _) in PAGES {
        send(&mut runtime, Msg::Page(page.into()))?;
        assert_eq!(runtime.state().page, page);
        let expected = match page {
            "overview" => "Start with controls",
            "controls" => "Hello, Terminal explorer",
            "collections" => "api-gateway",
            "content" => "use rsille::tui::prelude::*;",
            "layout" => "Spans three columns",
            "motion" => "Play animation",
            "themes" => "Current: Tokyo Night",
            _ => unreachable!(),
        };
        let text = frame_text(&runtime);
        assert!(
            text.contains(expected),
            "Missing {expected} on {page}:\n{text}"
        );
        // Keep every page usable in both compact and large terminals.
        for (width, height) in [(80, 24), (120, 45)] {
            runtime.step(&[Event::Resize(width, height)])?;
        }
    }
    focus(&mut runtime, "navigation")?;
    runtime.step(&[key(KeyCode::Left)])?;
    assert_eq!(runtime.state().page, "motion");
    for index in 0..THEMES.len() {
        runtime.step(&[key(KeyCode::F(2))])?;
        assert_eq!(runtime.state().theme, index);
    }
    send(&mut runtime, Msg::Page("controls".into()))?;
    focus(&mut runtime, "remember")?;
    runtime.step(&[key(KeyCode::Char(' '))])?;
    assert!(!runtime.state().remember);
    focus(&mut runtime, "calendar")?;
    runtime.step(&[key(KeyCode::Right), key(KeyCode::Enter)])?;
    assert_eq!(runtime.state().date.day, 24);
    focus(&mut runtime, "notes")?;
    runtime.step(&[Event::Paste("\n中文👩🏽‍💻".into())])?;
    assert!(runtime.state().notes.contains("中文👩🏽‍💻"));
    clock.advance(Duration::from_millis(200));
    runtime.step(&[])?;
    assert_eq!(executor.pending(), 0);
    runtime.step(&[Event::Paste(" / second edit".into())])?;
    clock.advance(Duration::from_millis(200));
    runtime.step(&[])?;
    assert_eq!(
        executor.pending(),
        0,
        "The second edit must restart debounce"
    );
    clock.advance(Duration::from_millis(150));
    runtime.step(&[])?;
    assert_eq!(executor.pending(), 1);
    assert!(executor.run_next());
    runtime.step(&[])?;
    assert_eq!(runtime.state().saved, runtime.state().notes);
    send(&mut runtime, Msg::Saved(1, "outdated result".into()))?;
    assert_eq!(runtime.state().saved, runtime.state().notes);
    assert!(runtime
        .tasks()
        .iter()
        .any(|task| task.state == TaskState::Completed));

    runtime.step(&[key(KeyCode::F(3))])?;
    assert_eq!(runtime.state().modal, Some(Modal::Confirm));
    assert!(frame_text(&runtime).contains("Hello, Terminal explorer"));
    let active_theme = runtime.state().theme;
    runtime.step(&[Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 30,
        row: 2,
        modifiers: KeyModifiers::empty(),
    })])?;
    assert_eq!(
        runtime.state().theme,
        active_theme,
        "Modal background must ignore clicks"
    );
    for _ in 0..6 {
        assert!(matches!(runtime.focused_key(), Some("cancel" | "confirm")));
        runtime.step(&[key(KeyCode::Tab)])?;
    }
    runtime.step(&[key(KeyCode::Esc)])?;
    assert_eq!(runtime.state().modal, None);
    assert_eq!(runtime.focused_key(), Some("theme"));
    assert_eq!(runtime.state().saved, runtime.state().notes);
    focus(&mut runtime, "priority")?;
    runtime.step(&[key(KeyCode::Enter)])?;
    assert!(!runtime.step(&[key(KeyCode::Esc)])?.should_quit);

    send(&mut runtime, Msg::Page("collections".into()))?;
    focus(&mut runtime, "services")?;
    runtime.step(&[key(KeyCode::Char(' '))])?;
    assert_eq!(runtime.state().selected.len(), 1);
    focus(&mut runtime, "sort")?;
    runtime.step(&[key(KeyCode::Enter)])?;
    assert!(runtime.state().sort_desc);
    focus(&mut runtime, "filter")?;
    runtime.step(&[Event::Paste("auth".into())])?;
    assert_eq!(runtime.state().filter, "auth");
    let text = frame_text(&runtime);
    assert!(text.contains("identity"));
    assert!(!text.contains("billing"));
    runtime.step(&[key(KeyCode::F(1))])?;
    assert_eq!(runtime.focused_key(), Some("palette"));
    for c in "Motion".chars() {
        runtime.step(&[key(KeyCode::Char(c))])?;
    }
    runtime.step(&[key(KeyCode::Enter)])?;
    assert_eq!(runtime.state().page, "motion");
    assert_eq!(runtime.state().modal, None);
    let ticks = runtime.state().ticks;
    send(&mut runtime, Msg::Pause(false))?;
    clock.advance(Duration::from_secs(1));
    runtime.step(&[])?;
    assert_eq!(runtime.state().ticks, ticks);
    assert!(runtime.step(&[key(KeyCode::Esc)])?.should_quit);
    assert_eq!(runtime.into_exit().reason, ExitReason::Requested);
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn gallery_interaction_flow() {
        super::run().unwrap();
    }
}
