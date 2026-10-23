use tui::prelude::*;

fn key(code: KeyCode, kind: KeyEventKind) -> Event {
    Event::Key(KeyEvent::new_with_kind(code, KeyModifiers::empty(), kind))
}

#[test]
fn raw_input_observes_capabilities_and_consumed_events_in_order() {
    #[derive(Debug, PartialEq)]
    enum Msg {
        Input(InputEvent),
        Changed(String),
    }
    let mut runtime = App::new(Vec::new())
        .keyboard_mode(KeyboardMode::Auto)
        .on_input(|input| Some(Msg::Input(input)))
        .update(|state, message, _| state.push(message))
        .view(|_| text_input().on_change(Msg::Changed))
        .build();
    let capabilities = InputCapabilities {
        key_releases: true,
        focus_events: true,
    };
    runtime.set_input_capabilities(capabilities);
    let press = key(KeyCode::Char('w'), KeyEventKind::Press);
    let repeat = key(KeyCode::Char('w'), KeyEventKind::Repeat);
    let release = key(KeyCode::Char('w'), KeyEventKind::Release);
    runtime
        .step(&[
            press.clone(),
            repeat.clone(),
            release.clone(),
            Event::FocusLost,
        ])
        .unwrap();
    let observed: Vec<_> = runtime
        .state()
        .iter()
        .filter_map(|message| match message {
            Msg::Input(input) => Some(input.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        observed,
        [
            InputEvent::Capabilities(capabilities),
            InputEvent::Terminal(press),
            InputEvent::Terminal(repeat),
            InputEvent::Terminal(release),
            InputEvent::Terminal(Event::FocusLost),
        ]
    );
    assert_eq!(
        runtime
            .state()
            .iter()
            .filter(|m| matches!(m, Msg::Changed(_)))
            .count(),
        2
    );
}

#[test]
fn releases_do_not_activate_actions_change_focus_or_quit() {
    let mut commands = CommandRouter::new();
    commands
        .register("command", "Command", || "command")
        .hotkey(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::empty()));
    let mut runtime = App::new(Vec::new())
        .on_key(KeyCode::Char('g'), || "global")
        .on_hotkey(
            Hotkey::simple("hotkey", KeyCode::Char('h'), "Hotkey"),
            || "hotkey",
        )
        .with_command_router(commands)
        .update(|state, message, _| state.push(message))
        .view(|_| {
            col()
                .child(button("First").key("first").on_click(|| "button"))
                .child(button("Second").key("second").on_click(|| "second"))
        })
        .build();
    runtime.step(&[]).unwrap();
    assert_eq!(runtime.focused_key(), Some("first"));
    for code in [
        KeyCode::Char('g'),
        KeyCode::Char('h'),
        KeyCode::Char('c'),
        KeyCode::Enter,
        KeyCode::Char(' '),
        KeyCode::Tab,
        KeyCode::Esc,
    ] {
        let result = runtime.step(&[key(code, KeyEventKind::Release)]).unwrap();
        assert!(!result.should_quit);
        assert!(runtime.state().is_empty());
        assert_eq!(runtime.focused_key(), Some("first"));
    }
    for code in [
        KeyCode::Char('g'),
        KeyCode::Char('h'),
        KeyCode::Char('c'),
        KeyCode::Enter,
    ] {
        runtime.step(&[key(code, KeyEventKind::Press)]).unwrap();
    }
    assert_eq!(runtime.state(), &["global", "hotkey", "command", "button"]);
    runtime
        .step(&[key(KeyCode::Tab, KeyEventKind::Press)])
        .unwrap();
    assert_eq!(runtime.focused_key(), Some("second"));
    assert!(
        runtime
            .step(&[key(KeyCode::Esc, KeyEventKind::Press)])
            .unwrap()
            .should_quit
    );
}

#[test]
fn toggles_and_buttons_only_activate_on_press() {
    for variant in 0..4 {
        let mut runtime = App::new(0)
            .update(|state, _, _| *state += 1)
            .view(move |_| -> Box<dyn Widget<bool>> {
                match variant {
                    0 => Box::new(button("Button").on_click(|| true)),
                    1 => Box::new(checkbox("Checkbox").on_change(|value| value)),
                    2 => Box::new(switch("Switch").on_change(|value| value)),
                    _ => Box::new(collapsible("Collapsible").on_toggle(|value| value)),
                }
            })
            .build();
        runtime
            .step(&[key(KeyCode::Char(' '), KeyEventKind::Press)])
            .unwrap();
        assert_eq!(*runtime.state(), 1);
        runtime
            .step(&[
                key(KeyCode::Char(' '), KeyEventKind::Repeat),
                key(KeyCode::Char(' '), KeyEventKind::Release),
            ])
            .unwrap();
        assert_eq!(*runtime.state(), 1);
    }
}

#[test]
fn registries_reject_release_even_when_called_directly() {
    let mut hotkeys = HotkeyRegistry::new();
    hotkeys.bind_key(KeyCode::Char('x'), || 1);
    let mut commands = CommandRouter::new();
    commands
        .register("x", "X", || 1)
        .hotkey(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::empty()));
    for kind in [
        KeyEventKind::Press,
        KeyEventKind::Repeat,
        KeyEventKind::Release,
    ] {
        let event = KeyEvent::new_with_kind(KeyCode::Char('x'), KeyModifiers::empty(), kind);
        let expected = (kind != KeyEventKind::Release).then_some(1);
        assert_eq!(hotkeys.resolve(&event), expected);
        assert_eq!(commands.dispatch_hotkey(&event), expected);
    }
}
