//! Terminal-independent CLI adapter. The host owns terminal setup and presentation.
use std::{collections::HashMap, sync::Arc};

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use rsille::render::{buffer::Buffer, chunk::Chunk};
use rsille::tui::prelude::*;
use rsille_typst::{MathRenderer, RenderOptions};
use serde_json::{json, Map, Value};

use crate::config::{display_text, Action, Config, Node};
use crate::style::StylePreset;

struct State {
    config: Config,
    values: Map<String, Value>,
    outcome: Option<Result<Value, ()>>,
    formulas: HashMap<FormulaKey, FormulaImage>,
}

type FormulaKey = (String, u64, u32);

struct FormulaImage {
    canvas: Arc<RasterCanvas>,
    width: u16,
    height: u16,
}

fn formula_key(spec: &crate::config::FormulaSpec) -> FormulaKey {
    (
        spec.source.clone(),
        spec.font_size.to_bits(),
        spec.pixels_per_point.to_bits(),
    )
}

fn cache_formulas(
    node: &Node,
    pixels: CellPixels,
    theme: &Theme,
    renderer: &mut Option<MathRenderer>,
    images: &mut HashMap<FormulaKey, FormulaImage>,
) -> Result<(), String> {
    match node {
        Node::Row(nodes) | Node::Column(nodes) => {
            for node in nodes {
                cache_formulas(node, pixels, theme, renderer, images)?;
            }
        }
        Node::Formula(spec) => {
            let key = formula_key(spec);
            if images.contains_key(&key) {
                return Ok(());
            }
            if renderer.is_none() {
                *renderer = Some(
                    MathRenderer::new(typst_assets::fonts(), "New Computer Modern Math")
                        .map_err(|e| format!("initialize formula fonts: {e}"))?,
                );
            }
            let mut options = RenderOptions {
                font_size: spec.font_size,
                pixels_per_point: spec.pixels_per_point,
                ..RenderOptions::default()
            };
            if let Some(Color::Rgb(r, g, b)) = theme.styles.text.fg_color {
                options.foreground = Rgb(r, g, b);
            }
            if let Some(Color::Rgb(r, g, b)) = theme.styles.surface.bg_color {
                options.background = Rgb(r, g, b);
            }
            let canvas = renderer
                .as_ref()
                .unwrap()
                .render(&spec.source, &options)
                .map_err(|e| format!("render formula: {e}"))?;
            let size = pixels
                .size_for_pixels(canvas.width(), canvas.height())
                .ok_or_else(|| "formula dimensions exceed terminal cell limits".to_owned())?;
            images.insert(
                key,
                FormulaImage {
                    canvas: Arc::new(canvas),
                    width: size.width.max(1),
                    height: size.height.max(1),
                },
            );
        }
        _ => {}
    }
    Ok(())
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
    /// Text-only compatibility constructor; formula hosts use `new_with_cell_pixels`.
    pub fn new(config: Config) -> Self {
        Self::with_images(config, HashMap::new(), RasterOutput::default())
    }

    /// Compile formulas once using host-resolved physical cell metrics; never probe here.
    pub fn new_with_cell_pixels(
        config: Config,
        size: (u16, u16),
        pixels: CellPixels,
    ) -> Result<Self, String> {
        let pixels = CellPixels::resolve(Some(pixels)).map_err(|e| format!("cell pixels: {e}"))?;
        let mut images = HashMap::new();
        cache_formulas(
            &config.layout,
            pixels,
            &config.theme.build(),
            &mut None,
            &mut images,
        )?;
        let inline = config.mode == "inline";
        let output = RasterOutput {
            capabilities: RasterCapabilities {
                sixel: true,
                reliable_erase: !inline,
                reliable_inline_erase: inline,
                no_scroll: true,
                cell_pixels: Some(pixels),
            },
        };
        let mut ui = Self::with_images(config, images, output);
        ui.resize(size.0, size.1);
        Ok(ui)
    }

    fn with_images(
        config: Config,
        formulas: HashMap<FormulaKey, FormulaImage>,
        output: RasterOutput,
    ) -> Self {
        let quick = matches!(
            config.layout,
            Node::Input(_)
                | Node::Select(_)
                | Node::Checkbox(_)
                | Node::Table(_)
                | Node::Formula(_)
        );
        let mut values = Map::new();
        initial_values(&config.layout, &mut values);
        let theme = config.theme.build();
        let runtime = App::new(State {
            config,
            values,
            outcome: None,
            formulas,
        })
        .raster_output(output)
        .disable_quit_key()
        .with_theme(theme)
        .with_motion_policy(MotionPolicy::disabled())
        .build(update, move |state| {
            let content = view(&state.config.layout, state, quick);
            let content = if state.config.title.is_empty() {
                content
            } else {
                Box::new(
                    col()
                        .child(label(display_text(&state.config.title)))
                        .child(content),
                ) as Box<dyn Widget<Message>>
            };
            Box::new(Flex::vertical(vec![content]).padding(state.config.style.padding()))
                as Box<dyn Widget<Message>>
        });
        Self { runtime, quick }
    }

    /// Reserve the layout's preferred rows, bounded later by the host terminal.
    pub fn inline_height(&self) -> u16 {
        fn height(node: &Node, state: &State) -> u16 {
            match node {
                Node::Column(nodes) => nodes
                    .iter()
                    .fold(0u16, |sum, node| sum.saturating_add(height(node, state)))
                    .saturating_add(
                        (nodes.len().saturating_sub(1) as u16)
                            .saturating_mul(state.config.style.gap()),
                    ),
                Node::Row(nodes) => nodes
                    .iter()
                    .map(|node| height(node, state))
                    .max()
                    .unwrap_or(0),
                _ => view(node, state, false).constraints().min_height,
            }
        }
        let state = self.runtime.state();
        height(&state.config.layout, state)
            .saturating_add(u16::from(!state.config.title.is_empty()))
            .saturating_add(state.config.style.padding().vertical_total())
            .max(1)
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
    let style = state.config.style;
    match node {
        Node::Row(nodes) | Node::Column(nodes) => {
            let children = nodes.iter().map(|n| view(n, state, false)).collect();
            Box::new(if matches!(node, Node::Row(_)) {
                Flex::horizontal(children).gap(1)
            } else {
                Flex::vertical(children).gap(style.gap())
            })
        }
        Node::Text(text) => Box::new(label(display_text(text))),
        Node::Formula(spec) => match state.formulas.get(&formula_key(spec)) {
            Some(image) => Box::new(
                raster_canvas(image.canvas.clone())
                    .background(image.canvas.background())
                    .scale(RasterScale::Original)
                    .align_x(RasterAlign::Start)
                    .align_y(RasterAlign::Start)
                    .fixed(image.width, image.height),
            ),
            // `new` remains infallible and text-only; do not compile during view rebuilds.
            None => Box::new(label(display_text(&spec.source))),
        },
        Node::Input(spec) => {
            let id = spec.id.clone();
            let value = state.values.get(&id).and_then(Value::as_str).unwrap_or("");
            Box::new(
                col().child(label(display_text(&spec.label))).child(
                    text_input()
                        .border(style.border())
                        .variant(if style == StylePreset::Minimal {
                            TextInputVariant::Borderless
                        } else {
                            TextInputVariant::Default
                        })
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
            let widget = if style == StylePreset::Minimal {
                widget.borderless()
            } else {
                widget.border(style.border())
            };
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
            table = if style == StylePreset::Minimal {
                table.borderless()
            } else {
                table.border(style.border())
            };
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
            Box::new(
                button(display_text(&spec.label))
                    .variant(style.button())
                    .on_click(move || Message::Finish(action)),
            )
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

    fn formula_config(source: &str) -> Config {
        let mut config = parse("version: 1\nlayout: {text: placeholder}", &Value::Null).unwrap();
        config.sixel = true;
        config.layout = Node::Formula(crate::config::FormulaSpec {
            source: source.to_owned(),
            font_size: 18.0,
            pixels_per_point: 2.0,
        });
        config
    }

    fn formula_ui(config: Config) -> Result<Ui, String> {
        Ui::new_with_cell_pixels(
            config,
            (80, 24),
            CellPixels {
                width: 8,
                height: 16,
            },
        )
    }

    #[test]
    fn formula_raster_has_natural_layout_and_theme_colors() {
        let config = formula_config("frac(1, 2)");
        let theme = config.theme.build();
        let mut prompt = formula_ui(config).unwrap();
        let image = prompt.runtime.state().formulas.values().next().unwrap();
        let canvas = image.canvas.clone();
        let (width, height) = (image.width, image.height);
        assert_eq!(u32::from(width), canvas.width().div_ceil(8));
        assert_eq!(u32::from(height), canvas.height().div_ceil(16));
        assert_eq!(prompt.inline_height(), height);
        if let Some(Color::Rgb(r, g, b)) = theme.styles.surface.bg_color {
            assert_eq!(canvas.background(), Rgb(r, g, b));
        }
        if let Some(Color::Rgb(r, g, b)) = theme.styles.text.fg_color {
            assert!(canvas.pixels().contains(&Rgb(r, g, b)));
        }
        prompt.resize(80, height);
        let frame = prompt.draw().unwrap();
        assert!(frame.has_raster());
        let raster = frame.raster_at((0, 0).into()).unwrap();
        assert_eq!(raster.target, (width, height).into());
        assert_eq!(raster.options.scale, RasterScale::Original);
        assert_eq!(raster.options.background, canvas.background());
    }

    #[test]
    fn formula_resize_and_redraw_reuse_cached_source() {
        let mut prompt = formula_ui(formula_config("x^2 + y^2")).unwrap();
        let canvas = prompt
            .runtime
            .state()
            .formulas
            .values()
            .next()
            .unwrap()
            .canvas
            .clone();
        let natural_height = prompt.inline_height();
        for (width, height) in [(80, 24), (3, 2), (120, 40)] {
            prompt.event(Event::Resize(width, height)).unwrap();
            let frame = prompt.draw().unwrap();
            assert_eq!(frame.size(), (width, height).into());
            assert!(frame.has_raster());
            assert_eq!(prompt.inline_height(), natural_height);
            assert!(Arc::ptr_eq(
                &canvas,
                &prompt
                    .runtime
                    .state()
                    .formulas
                    .values()
                    .next()
                    .unwrap()
                    .canvas
            ));
        }
    }

    #[test]
    fn formula_errors_are_returned_before_drawing() {
        let error = formula_ui(formula_config("frac(")).err().unwrap();
        assert!(error.contains("render formula"), "{error}");
        assert!(Ui::new_with_cell_pixels(
            formula_config("x"),
            (80, 24),
            CellPixels {
                width: 0,
                height: 16
            },
        )
        .is_err());
    }

    #[test]
    fn root_formula_enter_submits_empty_values_without_initial_draw() {
        let mut prompt = formula_ui(formula_config("x")).unwrap();
        key(&mut prompt, KeyCode::Enter);
        assert_eq!(
            prompt.outcome(),
            Some(Ok(json!({"action": "submit", "values": {}})))
        );
    }

    #[test]
    fn yaml_formula_is_a_non_value_component() {
        let config = parse(
            "version: 1\nlayout:\n  column:\n    - formula: {source: 'x^2', font_size: 18, pixels_per_point: 2}\n    - button: {label: Submit, action: submit}",
            &Value::Null,
        )
        .unwrap();
        let mut prompt = formula_ui(config).unwrap();
        assert!(prompt.draw().unwrap().has_raster());
        key(&mut prompt, KeyCode::Enter);
        assert_eq!(
            prompt.outcome(),
            Some(Ok(json!({"action": "submit", "values": {}})))
        );
    }

    #[test]
    fn compound_formulas_are_non_values_and_share_cached_images() {
        let mut config = formula_config("x");
        let formula = config.layout.clone();
        config.layout = Node::Column(vec![
            formula.clone(),
            formula,
            Node::Input(crate::config::InputSpec {
                id: "name".into(),
                label: "Name".into(),
                value: "Ada".into(),
            }),
            Node::Button(crate::config::ButtonSpec {
                label: "Submit".into(),
                action: Action::Submit,
            }),
        ]);
        let mut prompt = formula_ui(config).unwrap();
        assert_eq!(prompt.runtime.state().formulas.len(), 1);
        key(&mut prompt, KeyCode::Enter);
        assert_eq!(prompt.outcome(), None);
        key(&mut prompt, KeyCode::Tab);
        key(&mut prompt, KeyCode::Enter);
        assert_eq!(
            prompt.outcome(),
            Some(Ok(json!({"action": "submit", "values": {"name": "Ada"}})))
        );
    }

    #[test]
    fn presets_change_geometry_and_keep_input_submission() {
        for (style, corner) in [
            (StylePreset::Classic, Some('┌')),
            (StylePreset::Minimal, None),
            (StylePreset::Rounded, Some('╭')),
            (StylePreset::Retro, Some('╔')),
            (StylePreset::Bold, Some('┏')),
        ] {
            for layout in [
                "{input: {id: value, label: Name, value: hello}}",
                "{select: {id: value, label: Name, options: [hello, world]}}",
                "{table: {columns: [name], rows: [{name: hello}]}}",
            ] {
                let mut config =
                    parse(&format!("version: 1\nlayout: {layout}"), &Value::Null).unwrap();
                config.style = style;
                let mut prompt = Ui::new(config);
                prompt.resize(50, prompt.inline_height());
                let frame = prompt.draw().unwrap();
                let chars: String = frame
                    .content()
                    .iter()
                    .filter_map(|cell| cell.content.as_char())
                    .collect();
                if let Some(corner) = corner {
                    assert!(chars.contains(corner), "{style:?}: {chars}");
                } else {
                    assert!(!chars.contains('┌'), "{chars}");
                }
                key(&mut prompt, KeyCode::Enter);
                assert!(prompt.outcome().is_some(), "{style:?}");
            }
        }
    }

    #[test]
    fn styled_inline_height_counts_nested_gaps_and_padding() {
        let mut config = parse(
            "version: 1\nlayout: {column: [{text: A}, {row: [{text: B}, {text: C}]}, {text: D}]}",
            &Value::Null,
        )
        .unwrap();
        assert_eq!(Ui::new(config.clone()).inline_height(), 3);
        config.style = StylePreset::Rounded;
        assert_eq!(Ui::new(config).inline_height(), 7);
    }

    #[test]
    fn heading_and_select_share_background_without_losing_highlight() {
        use clap::ValueEnum;
        for preset in crate::theme::ThemePreset::value_variants() {
            let mut config = parse(
            "version: 1\ntitle: Heading\nlayout: {select: {id: env, label: Environment, options: [dev, prod]}}",
            &Value::Null,
        )
        .unwrap();
            config.theme = *preset;
            let mut prompt = Ui::new(config);
            prompt.resize(40, 12);
            let frame = prompt.draw().unwrap();
            let background = preset
                .build()
                .styles
                .surface_elevated
                .to_render_style()
                .colors
                .unwrap()
                .background;
            let selected = preset
                .build()
                .styles
                .selected_focused
                .to_render_style()
                .colors
                .unwrap()
                .background;
            for row in [0, 1, 2] {
                assert_eq!(
                    frame.content()[row * 40]
                        .content
                        .style
                        .colors
                        .unwrap()
                        .background,
                    background
                );
            }
            assert_ne!(background, selected);
            assert!(frame.content().iter().any(|cell| {
                cell.content
                    .style
                    .colors
                    .is_some_and(|colors| colors.background == selected)
            }));
        }
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
