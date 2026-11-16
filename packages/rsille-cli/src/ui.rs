//! Terminal-independent CLI adapter. The host owns terminal setup and presentation.
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use rsille::render::{buffer::Buffer, chunk::Chunk};
use rsille::tui::prelude::*;
use serde_json::{json, Map, Value};

use crate::config::{display_text, Action, Config, Node};

struct State {
    config: Config,
    values: Map<String, Value>,
    outcome: Option<Result<Value, ()>>,
}

enum Message {
    Value(String, Value),
    Finish(Action),
}

/// An in-memory rsille runtime; never reads stdin or writes stdout/stderr.
/// Pass a validated configuration with table sources already resolved.
/// A single control at the layout root is a quick prompt (Enter submits).
/// Compound layouts submit through their configured buttons.
pub struct Ui {
    runtime: Runtime<State, Message>,
    quick: bool,
}

pub fn build(config: Config) -> Ui {
    Ui::new(config)
}

impl Ui {
    pub fn new(config: Config) -> Self {
        let quick = matches!(
            config.layout,
            Node::Input(_) | Node::Select(_) | Node::Checkbox(_) | Node::Table(_)
        );
        let mut values = Map::new();
        initial_values(&config.layout, &mut values);
        let runtime = App::new(State {
            config,
            values,
            outcome: None,
        })
        .disable_quit_key()
        .with_motion_policy(MotionPolicy::disabled())
        .build(update, move |state| {
            let content = view(&state.config.layout, state, quick);
            if state.config.title.is_empty() {
                content
            } else {
                Box::new(
                    col()
                        .child(label(display_text(&state.config.title)))
                        .child(content),
                ) as Box<dyn Widget<Message>>
            }
        });
        Self { runtime, quick }
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.runtime.resize((width.max(1), height.max(1)).into());
    }

    pub fn event(&mut self, event: Event) -> Result<(), String> {
        if self.runtime.state().outcome.is_some() {
            return Ok(());
        }
        if let Event::Resize(width, height) = event {
            self.resize(width, height);
            return self.step(&[]);
        }
        let mut submit = false;
        if let Event::Key(key) = &event {
            if key.kind == KeyEventKind::Release {
                return Ok(());
            }
            if key.code == KeyCode::Esc
                || (matches!(key.code, KeyCode::Char('c' | 'C'))
                    && key.modifiers.contains(KeyModifiers::CONTROL))
            {
                return self.finish(Action::Cancel);
            }
            submit = self.quick && key.code == KeyCode::Enter;
        }
        // Mount/focus before routing the first input, even if the host did not draw.
        self.step(&[])?;
        self.step(&[event])?;
        if submit {
            self.finish(Action::Submit)?;
        }
        Ok(())
    }

    pub fn draw(&mut self) -> Result<&Buffer, String> {
        self.step(&[])?;
        Ok(self.runtime.frame())
    }

    /// None while active, Some(Err(())) on cancellation, or the submit envelope.
    pub fn outcome(&self) -> Option<Result<Value, ()>> {
        self.runtime.state().outcome.clone()
    }

    fn step(&mut self, events: &[Event]) -> Result<(), String> {
        self.runtime
            .step(events)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    fn finish(&mut self, action: Action) -> Result<(), String> {
        self.runtime
            .handle()
            .send(Message::Finish(action))
            .map_err(|e| e.to_string())?;
        self.step(&[])
    }
}

fn update(state: &mut State, message: Message) {
    if state.outcome.is_some() {
        return;
    }
    match message {
        Message::Value(id, value) => {
            state.values.insert(id, value);
        }
        Message::Finish(Action::Submit) => {
            state.outcome = Some(Ok(json!({"action": "submit", "values": state.values})));
        }
        Message::Finish(Action::Cancel) => state.outcome = Some(Err(())),
    }
}

fn initial_values(node: &Node, values: &mut Map<String, Value>) {
    match node {
        Node::Row(nodes) | Node::Column(nodes) => {
            for node in nodes {
                initial_values(node, values);
            }
        }
        Node::Input(spec) => {
            values.insert(spec.id.clone(), json!(spec.value));
        }
        Node::Select(spec) => {
            values.insert(spec.id.clone(), json!(spec.options.get(spec.selected)));
        }
        Node::Checkbox(spec) => {
            values.insert(spec.id.clone(), json!(spec.checked));
        }
        Node::Table(spec) if spec.selectable => {
            if let Some(id) = &spec.id {
                values.insert(id.clone(), json!([]));
            }
        }
        _ => {}
    }
}

fn view(node: &Node, state: &State, quick: bool) -> Box<dyn Widget<Message>> {
    match node {
        Node::Row(nodes) | Node::Column(nodes) => {
            let children = nodes.iter().map(|n| view(n, state, false)).collect();
            Box::new(if matches!(node, Node::Row(_)) {
                Flex::horizontal(children).gap(1)
            } else {
                Flex::vertical(children)
            })
        }
        Node::Text(text) => Box::new(label(display_text(text))),
        Node::Input(spec) => {
            let id = spec.id.clone();
            let value = state.values.get(&id).and_then(Value::as_str).unwrap_or("");
            Box::new(
                col().child(label(display_text(&spec.label))).child(
                    text_input()
                        .key(&id)
                        .value(display_text(value))
                        .on_change(move |text| {
                            Message::Value(id.clone(), json!(display_text(&text)))
                        }),
                ),
            )
        }
        Node::Select(spec) => {
            let id = spec.id.clone();
            let options = spec.options.clone();
            let widget = select()
                .key(&id)
                .options(
                    spec.options
                        .iter()
                        .enumerate()
                        .map(|(i, text)| SelectOption::new(i.to_string(), display_text(text))),
                )
                .on_change(move |index| {
                    let value = index
                        .parse::<usize>()
                        .ok()
                        .and_then(|i| options.get(i))
                        .cloned();
                    Message::Value(id.clone(), json!(value))
                });
            Box::new(
                col()
                    .child(label(display_text(&spec.label)))
                    .child(SeededSelect {
                        inner: widget,
                        selected: spec.selected.to_string(),
                        quick,
                    }),
            )
        }
        Node::Checkbox(spec) => {
            let id = spec.id.clone();
            Box::new(
                checkbox(display_text(&spec.label))
                    .key(&id)
                    .checked(
                        state
                            .values
                            .get(&id)
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                    )
                    .on_change(move |checked| Message::Value(id.clone(), json!(checked))),
            )
        }
        Node::Table(spec) => {
            let mut table =
                data_table()
                    .height(10)
                    .columns(spec.columns.iter().enumerate().map(|(i, name)| {
                        DataTableColumn::new(display_text(name)).id(i.to_string())
                    }))
                    .rows(spec.rows.iter().enumerate().map(|(i, row)| {
                        DataTableRow::new(
                            i.to_string(),
                            spec.columns.iter().map(|column| match row.get(column) {
                                Some(Value::String(text)) => display_text(text),
                                Some(value) => display_text(&value.to_string()),
                                None => String::new(),
                            }),
                        )
                    }));
            if let Some(id) = &spec.id {
                table = table.key(id);
            }
            if spec.selectable {
                if let Some(id) = spec.id.clone() {
                    let rows = spec.rows.clone();
                    table = table.multi_select(false).on_selection_change(move |ids| {
                        let selected: Vec<Value> = ids
                            .iter()
                            .filter_map(|id| {
                                id.parse::<usize>().ok().and_then(|i| rows.get(i)).cloned()
                            })
                            .collect();
                        Message::Value(id.clone(), json!(selected))
                    });
                }
            }
            Box::new(table)
        }
        Node::Button(spec) => {
            let action = spec.action;
            Box::new(button(display_text(&spec.label)).on_click(move || Message::Finish(action)))
        }
    }
}

/// Select has no default-selection builder. Seed its native state on mount;
/// navigation, selection, rendering and focus remain entirely framework-owned.
struct SeededSelect {
    inner: Select<Message>,
    selected: String,
    quick: bool,
}

impl Widget<Message> for SeededSelect {
    fn lifecycle(&self, ctx: &mut StateCtx) {
        self.inner.lifecycle(ctx);
        if ctx.lifecycle() == Lifecycle::Mount {
            let state = ctx.state_mut::<SelectState>();
            state.selected_option = Some(self.selected.clone());
            state.active_option = Some(self.selected.clone());
            state.selection.replace_selection(self.selected.clone());
            state.selection.cursor = Some(self.selected.clone());
            state.is_open = self.quick;
        }
    }
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        self.inner.render(chunk, ctx);
    }
    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<Message>) {
        self.inner.handle_event(event, ctx);
    }
    fn constraints(&self) -> Constraints {
        self.inner.constraints()
    }
    fn focus_config(&self) -> FocusConfig {
        self.inner.focus_config()
    }
    fn key(&self) -> Option<&str> {
        Widget::key(&self.inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::parse;
    use crossterm::event::KeyEvent;

    fn ui(layout: &str) -> Ui {
        Ui::new(parse(&format!("version: 1\nlayout: {layout}"), &Value::Null).unwrap())
    }
    fn key(ui: &mut Ui, code: KeyCode) {
        ui.event(Event::Key(KeyEvent::new(code, KeyModifiers::NONE)))
            .unwrap();
    }

    #[test]
    fn quick_input_and_resize_without_terminal() {
        let mut ui = ui("{input: {id: name, label: Name}}");
        ui.resize(50, 12);
        ui.draw().unwrap();
        key(&mut ui, KeyCode::Char('中'));
        key(&mut ui, KeyCode::Enter);
        assert_eq!(
            ui.outcome(),
            Some(Ok(json!({"action":"submit", "values":{"name":"中"}})))
        );
    }

    #[test]
    fn quick_select_uses_native_cursor_and_raw_values() {
        let mut ui = ui(
            r#"{select: {id: env, label: Env, options: [first, second, "third\u001b"], selected: 1}}"#,
        );
        key(&mut ui, KeyCode::Down);
        key(&mut ui, KeyCode::Enter);
        assert_eq!(
            ui.outcome().unwrap().unwrap()["values"]["env"],
            "third\u{001b}"
        );
    }

    #[test]
    fn table_enter_submits_current_original_object() {
        for (keys, expected) in [
            (vec![], json!([{"name":"a"}])),
            (
                vec![KeyCode::Down],
                json!([{"name":"b\u{001b}", "nested":{"n":1}}]),
            ),
            (vec![KeyCode::Down, KeyCode::Up], json!([{"name":"a"}])),
            (
                vec![KeyCode::Char(' '), KeyCode::Down],
                json!([{"name":"b\u{001b}", "nested":{"n":1}}]),
            ),
        ] {
            let mut ui = ui(
                r#"{table: {id: rows, selectable: true, rows: [{name: a}, {name: "b\u001b", nested: {n: 1}}]}}"#,
            );
            for code in keys {
                key(&mut ui, code);
            }
            assert!(ui.outcome().is_none());
            key(&mut ui, KeyCode::Enter);
            assert_eq!(
                ui.outcome(),
                Some(Ok(json!({"action":"submit", "values":{"rows":expected}})))
            );
        }
    }

    #[test]
    fn empty_table_submits_empty_selection() {
        let mut ui = ui("{table: {id: rows, selectable: true, rows: []}}");
        key(&mut ui, KeyCode::Down);
        key(&mut ui, KeyCode::Enter);
        assert_eq!(
            ui.outcome(),
            Some(Ok(json!({"action":"submit", "values":{"rows":[]}})))
        );
    }

    #[test]
    fn compound_layout_focus_checkbox_and_submit_button() {
        let mut ui = ui("{column: [{text: Hello}, {row: [{checkbox: {id: yes, label: Yes}}, {button: {label: OK, action: submit}}]}, {table: {id: display, rows: [{x: 1}]}}]}");
        ui.draw().unwrap();
        key(&mut ui, KeyCode::Char(' '));
        key(&mut ui, KeyCode::Tab);
        key(&mut ui, KeyCode::Enter);
        assert_eq!(
            ui.outcome(),
            Some(Ok(json!({"action":"submit", "values":{"yes":true}})))
        );
    }

    #[test]
    fn cancellation_and_release_do_not_submit() {
        let mut prompt = ui("{input: {id: x, label: X}}");
        prompt
            .event(Event::Key(KeyEvent::new_with_kind(
                KeyCode::Enter,
                KeyModifiers::NONE,
                KeyEventKind::Release,
            )))
            .unwrap();
        assert!(prompt.outcome().is_none());
        key(&mut prompt, KeyCode::Esc);
        key(&mut prompt, KeyCode::Enter);
        assert_eq!(prompt.outcome(), Some(Err(())));
        let mut prompt = ui("{text: Hello}");
        prompt
            .event(Event::Key(KeyEvent::new(
                KeyCode::Char('c'),
                KeyModifiers::CONTROL,
            )))
            .unwrap();
        assert_eq!(prompt.outcome(), Some(Err(())));
        let mut prompt = ui("{button: {label: Cancel, action: cancel}}");
        key(&mut prompt, KeyCode::Enter);
        assert_eq!(prompt.outcome(), Some(Err(())));
    }

    #[test]
    fn untouched_input_preserves_original_value() {
        let mut ui = ui(r#"{input: {id: raw, label: "Label\u001b", value: "a\u001bb"}}"#);
        key(&mut ui, KeyCode::Enter);
        assert_eq!(
            ui.outcome().unwrap().unwrap()["values"]["raw"],
            "a\u{001b}b"
        );
    }
}
