use tui::prelude::*;

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::empty()))
}
#[derive(Debug, Clone)]
enum Msg {
    Active(Option<String>),
    Selection(Vec<String>),
    Submit(String),
    Configure(Option<Option<String>>, Option<Vec<String>>),
    Filter(bool),
    Delete,
    Accept(bool),
}
struct State {
    active: Option<Option<String>>,
    selection: Option<Vec<String>>,
    filter: bool,
    deleted: bool,
    accept: bool,
    events: Vec<Msg>,
}
fn app(
    active: Option<Option<&str>>,
    selection: Option<Vec<&str>>,
    multiple: bool,
) -> Runtime<State, Msg> {
    App::new(State {
        active: active.map(|v| v.map(str::to_owned)),
        selection: selection.map(|v| v.into_iter().map(str::to_owned).collect()),
        filter: false,
        deleted: false,
        accept: true,
        events: vec![],
    })
    .with_motion_policy(MotionPolicy::disabled())
    .build(
        |s, m| {
            match &m {
                Msg::Active(v) if s.accept => {
                    if s.active.is_some() {
                        s.active = Some(v.clone());
                    }
                }
                Msg::Selection(v) if s.accept => {
                    if s.selection.is_some() {
                        s.selection = Some(v.clone());
                    }
                }
                Msg::Configure(a, b) => {
                    s.active = a.clone();
                    s.selection = b.clone();
                }
                Msg::Filter(v) => s.filter = *v,
                Msg::Delete => s.deleted = true,
                Msg::Accept(v) => s.accept = *v,
                _ => {}
            }
            s.events.push(m);
        },
        move |s| {
            let mut table = data_table()
                .key("table")
                .height(8)
                .column(DataTableColumn::new("Name").filterable(true))
                .rows(
                    [
                        DataTableRow::new("a", ["Alpha"]),
                        DataTableRow::new("disabled", ["Disabled"]).disabled(true),
                        DataTableRow::new("b", ["Beta"]),
                        DataTableRow::new("c", ["Gamma"]),
                    ]
                    .into_iter()
                    .filter(|r| !s.deleted || r.id != "b"),
                )
                .multi_select(multiple)
                .default_active_row(Some("a".into()))
                .default_selected_ids(["a".into()])
                .on_active_row_change(Msg::Active)
                .on_selection_change(Msg::Selection)
                .on_submit(Msg::Submit);
            if let Some(value) = &s.active {
                table = table.active_row(value.clone());
            }
            if let Some(value) = &s.selection {
                table = table.selected_ids(value.clone());
            }
            if s.filter {
                table = table.filter_query("Gamma");
            }
            table
        },
    )
    .size(32, 10)
}
fn table(rt: &Runtime<State, Msg>) -> &DataTableState {
    rt.widget_state(&WidgetId::root()).unwrap()
}
fn send(rt: &mut Runtime<State, Msg>, m: Msg) {
    rt.handle().send(m).unwrap();
    rt.step(&[]).unwrap();
}
#[test]
fn controlled_batch_accumulates_and_rejection_restores_authority() {
    let mut rt = app(Some(Some("a")), Some(vec![]), true);
    rt.step(&[]).unwrap();
    rt.step(&[
        key(KeyCode::Down),
        key(KeyCode::Down),
        key(KeyCode::Char(' ')),
    ])
    .unwrap();
    assert_eq!(rt.state().active, Some(Some("c".into())));
    assert_eq!(rt.state().selection, Some(vec!["c".into()]));
    assert_eq!(
        rt.state()
            .events
            .iter()
            .filter(|e| matches!(e, Msg::Active(_)))
            .count(),
        2
    );
    assert!(rt
        .state()
        .events
        .iter()
        .all(|e| !matches!(e, Msg::Submit(_))));
    send(&mut rt, Msg::Accept(false));
    rt.step(&[key(KeyCode::Up), key(KeyCode::Char(' '))])
        .unwrap();
    assert_eq!(table(&rt).active_row.as_deref(), Some("c"));
    assert_eq!(table(&rt).selection.selected, vec!["c"]);
    let count = rt.state().events.len();
    rt.step(&[]).unwrap();
    assert_eq!(
        count,
        rt.state().events.len(),
        "Sync never emits user changes"
    );
}
#[test]
fn explicit_empty_cursor_does_not_submit_fallback_and_first_navigation_is_defined() {
    let mut rt = app(Some(None), Some(vec![]), false);
    rt.step(&[]).unwrap();
    rt.step(&[key(KeyCode::Enter)]).unwrap();
    assert!(rt.state().events.is_empty());
    rt.step(&[key(KeyCode::Up)]).unwrap();
    assert_eq!(table(&rt).active_row.as_deref(), Some("c"));
    send(&mut rt, Msg::Configure(Some(None), Some(vec![])));
    rt.step(&[key(KeyCode::Down)]).unwrap();
    assert_eq!(table(&rt).active_row.as_deref(), Some("a"));
    rt.step(&[key(KeyCode::Enter)]).unwrap();
    assert!(rt
        .state()
        .events
        .iter()
        .any(|e| matches!(e,Msg::Submit(id) if id=="a")));
    assert_eq!(table(&rt).selection.selected, vec!["a"]);
}
#[test]
fn hidden_unknown_selection_survives_filter_and_visible_edits() {
    let mut rt = app(Some(Some("b")), Some(vec!["a", "missing", "a"]), true);
    rt.step(&[]).unwrap();
    assert_eq!(table(&rt).selection.selected, vec!["a", "missing"]);
    send(&mut rt, Msg::Filter(true));
    assert_eq!(table(&rt).active_row.as_deref(), Some("b"));
    rt.step(&[key(KeyCode::Enter)]).unwrap();
    assert!(rt
        .state()
        .events
        .iter()
        .any(|e| matches!(e, Msg::Submit(id) if id=="c")));
    rt.step(&[key(KeyCode::Char(' '))]).unwrap();
    assert_eq!(table(&rt).selection.selected, vec!["a", "missing", "c"]);
    send(&mut rt, Msg::Delete);
    assert_eq!(table(&rt).active_row.as_deref(), Some("b"));
}
#[test]
fn defaults_only_mount_and_releasing_control_uses_last_authority() {
    let mut rt = app(None, None, false);
    rt.step(&[]).unwrap();
    rt.step(&[key(KeyCode::Down)]).unwrap();
    assert_eq!(table(&rt).active_row.as_deref(), Some("b"));
    send(&mut rt, Msg::Filter(false));
    assert_eq!(table(&rt).active_row.as_deref(), Some("b"));
    send(
        &mut rt,
        Msg::Configure(Some(Some("c".into())), Some(vec!["b".into(), "a".into()])),
    );
    assert_eq!(table(&rt).selection.selected, vec!["b"]);
    send(&mut rt, Msg::Configure(None, None));
    assert_eq!(table(&rt).active_row.as_deref(), Some("c"));
    assert_eq!(table(&rt).selection.selected, vec!["b"]);
    send(&mut rt, Msg::Delete);
    assert!(table(&rt).selection.selected.is_empty());
    let frame = rt
        .frame()
        .content()
        .iter()
        .map(|c| c.content.symbol())
        .collect::<String>();
    assert!(frame.contains("Disabled"), "disabled rows remain visible");
}

#[test]
fn releasing_an_explicitly_empty_cursor_keeps_it_empty() {
    let mut rt = app(Some(None), Some(vec![]), false);
    rt.step(&[]).unwrap();
    send(&mut rt, Msg::Configure(None, None));
    let count = rt.state().events.len();
    rt.step(&[key(KeyCode::Enter)]).unwrap();
    assert_eq!(rt.state().events.len(), count);
    rt.step(&[key(KeyCode::Down)]).unwrap();
    assert_eq!(table(&rt).active_row.as_deref(), Some("a"));
}
