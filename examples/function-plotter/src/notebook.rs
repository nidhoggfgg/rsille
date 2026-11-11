use crate::Plot;
use rsille::render::{area::Area, chunk::Chunk};
use rsille::tui::prelude::*;
use std::{rc::Rc, time::Duration};

const PREVIEW_DELAY: Duration = Duration::from_millis(500);
const PREVIEW_TIMER: &str = "preview";

const MAX_HISTORY: usize = 24;
const MAX_INPUT: usize = 4096;
const GRAPH_ROWS: u16 = 14;

pub(crate) struct State {
    number: u64,
    initial_expression: bool,
    x_range: [f64; 2],
    bounds: [f64; 4],
    draft: String,
    output: Option<Rc<Plot>>,
    cell_pixels: (u16, u16),
    error: Option<String>,
    validation_error: Option<String>,
    revision: u64,
    finished: bool,
    history: Vec<String>,
    history_index: Option<usize>,
    saved_draft: String,
    saved_validation: Option<String>,
}

impl State {
    pub(crate) fn new(source: Option<String>, x_range: [f64; 2]) -> Result<Self, String> {
        let initial_expression = source.is_some();
        let source = source.unwrap_or_default();
        if source.len() > MAX_INPUT {
            return Err(format!("Expression exceeds {MAX_INPUT} bytes"));
        }
        Ok(Self {
            number: 1,
            initial_expression,
            x_range,
            bounds: [x_range[0], x_range[1], -1.0, 1.0],
            draft: source,
            output: None,
            cell_pixels: (1, 2),
            error: None,
            validation_error: None,
            revision: 0,
            finished: false,
            history: Vec::new(),
            history_index: None,
            saved_draft: String::new(),
            saved_validation: None,
        })
    }

    fn cancel_preview(&mut self, ctx: &mut UpdateCtx<Msg>) {
        self.revision += 1;
        ctx.cancel_debounce(PREVIEW_TIMER);
    }

    fn schedule_preview(&mut self, ctx: &mut UpdateCtx<Msg>) {
        self.cancel_preview(ctx);
        // Clear both together so an old graph never gets the new draft's legend.
        self.output = None;
        self.error = None;
        if !self.draft.trim().is_empty() && self.validation_error.is_none() {
            ctx.debounce(PREVIEW_TIMER, PREVIEW_DELAY, Msg::Preview(self.revision));
        }
    }

    fn render_draft(&mut self) {
        // Edits already invalidate the source. Submitting a successful preview
        // reuses its parsed functions and, when layout agrees, its raster.
        if self.output.is_some() {
            return;
        }
        self.error = self.validation_error.clone();
        if self.error.is_none() {
            match Plot::parse(&self.draft) {
                Ok(plot) => {
                    self.bounds = plot.auto_bounds(self.x_range);
                    self.output = Some(Rc::new(plot));
                }
                Err(error) => self.error = Some(error),
            }
        }
    }

    fn update(&mut self, msg: Msg, ctx: &mut UpdateCtx<Msg>) {
        match msg {
            Msg::Evaluate => {
                self.cancel_preview(ctx);
                // Preview errors never block immediate evaluation of the current source.
                self.render_draft();
                if self.history.len() == MAX_HISTORY {
                    self.history.remove(0);
                }
                self.history.push(self.draft.clone());
                self.finished = true;
                ctx.commit_frame(Msg::NextPrompt);
            }
            Msg::Preview(revision) => {
                if revision == self.revision && !self.finished {
                    self.render_draft();
                }
            }
            Msg::NextPrompt => {
                self.cancel_preview(ctx);
                self.validation_error = None;
                self.number += 1;
                self.draft.clear();
                self.output = None;
                self.error = None;
                self.finished = false;
                self.history_index = None;
                self.saved_draft.clear();
                self.saved_validation = None;
            }
            Msg::Edit(mut value) => {
                self.validation_error = None;
                if value.len() > MAX_INPUT {
                    let mut end = MAX_INPUT;
                    while !value.is_char_boundary(end) {
                        end -= 1;
                    }
                    value.truncate(end);
                    self.validation_error = Some(format!("Input limited to {MAX_INPUT} bytes"));
                }
                self.draft = value;
                self.history_index = None;
                self.schedule_preview(ctx);
            }
            Msg::Previous => {
                if !self.history.is_empty() {
                    let index = match self.history_index {
                        Some(index) => index.saturating_sub(1),
                        None => {
                            self.saved_draft = self.draft.clone();
                            self.saved_validation = self.validation_error.clone();
                            self.history.len() - 1
                        }
                    };
                    self.history_index = Some(index);
                    self.draft = self.history[index].clone();
                    self.validation_error = None;
                    self.schedule_preview(ctx);
                }
            }
            Msg::Next => {
                if let Some(index) = self.history_index {
                    if index + 1 < self.history.len() {
                        self.history_index = Some(index + 1);
                        self.draft = self.history[index + 1].clone();
                        self.validation_error = None;
                    } else {
                        self.history_index = None;
                        self.draft = self.saved_draft.clone();
                        self.validation_error = self.saved_validation.clone();
                    }
                    self.schedule_preview(ctx);
                }
            }
        }
    }
}

#[derive(Clone)]
pub(crate) enum Msg {
    Edit(String),
    Evaluate,
    Preview(u64),
    NextPrompt,
    Previous,
    Next,
}

pub(crate) fn view(state: &State) -> impl Widget<Msg> {
    let mut input = row()
        .key("input")
        .child(label(format!("In[{}]:=", state.number)).height(1));
    if state.finished {
        input = input.child(label(state.draft.clone()).height(1));
    } else {
        input = input.child(InputLine(
            textarea()
                .variant(TextAreaVariant::Borderless)
                .key("editor")
                .value(&state.draft)
                .placeholder("sin(x); cos(x)")
                .on_change(Msg::Edit),
        ));
    }
    let mut body = col().padding(Padding::horizontal(1)).child(input);
    if let Some(error) = state.validation_error.as_ref().or(state.error.as_ref()) {
        body = body.child(
            label(format!(
                "{}: {error}",
                if state.finished || state.validation_error.is_some() {
                    "Error"
                } else {
                    "Preview error"
                }
            ))
            .height(1)
            .style(Style::default().fg(Color::Rgb(255, 179, 92))),
        );
    }
    if let Some(image) = &state.output {
        body = body.child(
            label(if state.finished {
                format!("Out[{}]=", state.number)
            } else {
                "Preview (not submitted)".into()
            })
            .height(1),
        );
        body = body.child(FormulaOutput {
            source: Rc::clone(image),
            cell_pixels: state.cell_pixels,
        });
        body = body.child(Graph {
            source: Rc::clone(image),
            bounds: state.bounds,
            cell_pixels: state.cell_pixels,
        });
    }
    if !state.finished {
        body = body.child(
            label("Preview: 500ms idle · Enter/F5: submit now · Up/Down: history · Esc: quit")
                .height(1),
        );
    }
    Prompt {
        body: Box::new(body),
    }
}

// TextArea's height setter clamps to three rows. Override only its layout and
// single-line event policy; keep the library's editor state and rendering.
struct InputLine(TextArea<Msg>);

impl Widget<Msg> for InputLine {
    fn key(&self) -> Option<&str> {
        Widget::key(&self.0)
    }

    fn constraints(&self) -> Constraints {
        Constraints {
            min_width: 0,
            max_width: None,
            min_height: 1,
            max_height: Some(1),
            flex: Some(1.0),
        }
    }

    fn lifecycle(&self, ctx: &mut StateCtx) {
        self.0.lifecycle(ctx);
    }

    fn focus_config(&self) -> FocusConfig {
        self.0.focus_config()
    }

    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        self.0.render(chunk, ctx);
    }

    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<Msg>) {
        match event {
            Event::Paste(value) => {
                // Match TextInput: CRLF is one space, lone CR/LF are spaces.
                // TextArea still handles control filtering and caret insertion.
                let value = value.replace("\r\n", "\n").replace(['\r', '\n'], " ");
                self.0.handle_event(&Event::Paste(value), ctx);
            }
            // Plain Enter is captured by Prompt; modified Enter must not add a line.
            Event::Key(key) if key.code == KeyCode::Enter => ctx.set_handled(),
            _ => self.0.handle_event(event, ctx),
        }
    }
}

// Capture submission/history before the single-line editor consumes those keys.
struct Prompt {
    body: Box<dyn Widget<Msg>>,
}
impl Widget<Msg> for Prompt {
    fn layout(&self, area: Area, ctx: &MeasureCtx) -> Layout {
        self.body.layout(area, ctx)
    }
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        self.body.render(chunk, ctx);
    }
    fn constraints(&self) -> Constraints {
        self.body.constraints()
    }
    fn children(&self) -> &[Box<dyn Widget<Msg>>] {
        self.body.children()
    }
    fn handle_event(&self, event: &Event, ctx: &mut EventCtx<Msg>) {
        if !matches!(ctx.phase(), EventPhase::Capture | EventPhase::Target) {
            return;
        }
        if let Event::Key(key) = event {
            if key.modifiers != KeyModifiers::NONE {
                return;
            }
            let message = match key.code {
                KeyCode::Enter | KeyCode::F(5) => Msg::Evaluate,
                KeyCode::Up => Msg::Previous,
                KeyCode::Down => Msg::Next,
                _ => return,
            };
            if key.kind == KeyEventKind::Press {
                ctx.emit(message);
            }
            ctx.stop_propagation();
        }
    }
}

// Layout selects the final pixel grid before adaptive curve sampling.
// The separate bounds survey runs once when the source is parsed.
struct FormulaOutput {
    source: Rc<Plot>,
    cell_pixels: (u16, u16),
}
impl Widget<Msg> for FormulaOutput {
    fn key(&self) -> Option<&str> {
        Some("formula")
    }
    fn constraints(&self) -> Constraints {
        let height = if self.cell_pixels == (1, 2) {
            self.source.formula.text_rows().count() as u16
        } else {
            self.source.formula.height(self.cell_pixels.1)
        };
        Constraints {
            min_width: 1,
            max_width: Some(self.source.formula.width(self.cell_pixels)),
            min_height: height,
            max_height: Some(height),
            flex: None,
        }
    }
    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        let area = ctx.layout_target().unwrap_or_else(|| chunk.area());
        if area.width() == 0 || area.height() == 0 {
            return;
        }
        if self.cell_pixels != (1, 2) {
            let size = (
                u32::from(area.width()) * u32::from(self.cell_pixels.0),
                u32::from(area.height()) * u32::from(self.cell_pixels.1),
            );
            if let Some(image) = self.source.formula.image(size, self.cell_pixels.1) {
                raster_canvas::<Msg>(image)
                    .align_x(RasterAlign::Start)
                    .render(chunk, ctx);
                return;
            }
        }
        for (y, (text, index)) in self.source.formula.text_rows().enumerate() {
            let rsille::raster::Rgb(r, g, b) = crate::plot::COLORS[index];
            let _ = chunk.set_string(
                0,
                y as u16,
                text,
                Style::default().fg(Color::Rgb(r, g, b)).to_render_style(),
            );
        }
    }
}

struct Graph {
    source: Rc<Plot>,
    bounds: [f64; 4],
    cell_pixels: (u16, u16),
}

impl Widget<Msg> for Graph {
    fn key(&self) -> Option<&str> {
        Some("output")
    }

    fn render(&self, chunk: &mut Chunk, ctx: &RenderCtx) {
        let area = ctx.layout_target().unwrap_or_else(|| chunk.area());
        if area.width() == 0 || area.height() == 0 {
            return;
        }
        let image = self.source.image(
            self.bounds,
            (
                u32::from(area.width()) * u32::from(self.cell_pixels.0),
                u32::from(area.height()) * u32::from(self.cell_pixels.1),
            ),
        );
        raster_canvas::<Msg>(image)
            .align_x(RasterAlign::Start)
            .render(chunk, ctx);
    }

    fn constraints(&self) -> Constraints {
        Constraints {
            min_width: 1,
            max_width: Some(self.source.formula.width(self.cell_pixels)),
            min_height: GRAPH_ROWS,
            max_height: Some(GRAPH_ROWS),
            flex: None,
        }
    }
}

pub(crate) fn application(state: State, output: RasterOutput) -> Runtime<State, Msg> {
    start(
        application_builder(state, output)
            .update(State::update)
            .view(view)
            .build(),
    )
}

fn application_builder(mut state: State, output: RasterOutput) -> App<State, Msg> {
    if output.policy == RasterPolicy::RequireSixel {
        if let Some(pixels) = output.capabilities.cell_pixels {
            state.cell_pixels = (pixels.width, pixels.height);
        }
    }
    // Unset widget styles inherit theme backgrounds, so remove the fills at
    // their source. Keep caret/selection visible without a contrasting block.
    let mut theme = Theme::dark();
    theme.styles.surface.bg_color = None;
    theme.styles.interactive.bg_color = None;
    theme.styles.interactive_focused.bg_color = None;
    theme.styles.cursor = theme.styles.text.underlined();
    theme.styles.selection_range = Style::default().underlined();
    App::new(state)
        .with_theme(theme)
        .mode(Mode::Inline)
        .disable_mouse_capture()
        .with_motion_policy(MotionPolicy::disabled())
        .keyboard_mode(KeyboardMode::Auto)
        .raster_output(output)
        .with_quit_key(KeyCode::Esc)
}

fn start(runtime: Runtime<State, Msg>) -> Runtime<State, Msg> {
    if runtime.state().initial_expression {
        runtime
            .handle()
            .send(Msg::Evaluate)
            .expect("runtime is alive");
    }
    runtime
}

pub(crate) fn smoke_test(state: State) -> Result<(), Box<dyn std::error::Error>> {
    let initial_expression = state.initial_expression;
    let mut runtime = application(state, RasterOutput::text_only()).size(100, 32);
    assert_eq!(runtime.step(&[])?.committed, initial_expression);
    if initial_expression {
        runtime.step(&[])?;
    } else {
        assert_eq!(runtime.state().number, 1);
        assert!(runtime.state().history.is_empty());
        assert!(!runtime.frame().has_raster());
    }
    assert_eq!(runtime.focused_key(), Some("editor"));
    for source in [
        "sin(x); x^2/8",
        "polar(1+cos(theta), 0, 2*pi)",
        "param(sin(3*t), sin(2*t), 0, 2*pi)",
        "sin(x); polar(2, 0, 2*pi); param(t, t^2, -1, 1)",
        "unknown(x)",
        "cos(x)",
    ] {
        let result = runtime.step(&[
            Event::Paste(source.into()),
            Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        ])?;
        assert!(result.committed);
        assert_eq!(runtime.frame().has_raster(), source != "unknown(x)");
        runtime.step(&[])?;
        assert!(runtime.state().draft.is_empty());
        assert!(!runtime.frame().has_raster());
        assert_eq!(runtime.focused_key(), Some("editor"));
    }
    println!("Inline REPL passed: repeated commits, error, next prompt, typing, focus and raster snapshots");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }
    fn runtime() -> Runtime<State, Msg> {
        application(
            State::new(Some("sin(x)".into()), crate::X_RANGE).unwrap(),
            RasterOutput::text_only(),
        )
        .size(100, 32)
    }

    fn timed() -> (Runtime<State, Msg>, rsille::tui::runtime::ManualClock) {
        timed_with_pixels((1, 2))
    }

    fn timed_with_pixels(
        cell: (u16, u16),
    ) -> (Runtime<State, Msg>, rsille::tui::runtime::ManualClock) {
        let clock = rsille::tui::runtime::ManualClock::default();
        let mut state = State::new(Some("sin(x)".into()), crate::X_RANGE).unwrap();
        state.cell_pixels = cell;
        let mut rt = start(
            application_builder(state, RasterOutput::text_only())
                .with_clock(clock.clone())
                .update(State::update)
                .view(view)
                .build(),
        )
        .size(100, 32);
        assert!(rt.step(&[]).unwrap().committed);
        rt.step(&[]).unwrap();
        (rt, clock)
    }

    fn send(rt: &mut Runtime<State, Msg>, msg: Msg) {
        rt.handle().send(msg).unwrap();
        assert!(!rt.step(&[]).unwrap().committed);
    }

    #[test]
    fn preview_deadline_coalescing_and_empty_clear() {
        let (mut rt, clock) = timed();
        send(&mut rt, Msg::Edit("cos(x)".into()));
        clock.advance(Duration::from_millis(499));
        assert!(!rt.step(&[]).unwrap().committed);
        assert!(!rt.frame().has_raster());
        clock.advance(Duration::from_millis(1));
        assert!(!rt.step(&[]).unwrap().committed);
        assert!(rt.frame().has_raster());
        assert_eq!(rt.state().number, 2);
        assert_eq!(rt.state().history, ["sin(x)"]);
        send(&mut rt, Msg::Edit("bad(x)".into()));
        assert!(!rt.frame().has_raster());
        let old_revision = rt.state().revision;
        clock.advance(Duration::from_millis(400));
        send(&mut rt, Msg::Edit("x".into()));
        send(&mut rt, Msg::Preview(old_revision));
        clock.advance(Duration::from_millis(100));
        rt.step(&[]).unwrap();
        assert!(rt.state().error.is_none());
        assert!(!rt.frame().has_raster());
        clock.advance(Duration::from_millis(400));
        rt.step(&[]).unwrap();
        assert!(rt.frame().has_raster());
        send(&mut rt, Msg::Edit(String::new()));
        clock.advance(PREVIEW_DELAY);
        rt.step(&[]).unwrap();
        assert!(!rt.frame().has_raster());
        assert!(rt.state().error.is_none());
    }

    #[test]
    fn preview_error_recovery_and_immediate_submission_cancel_pending_work() {
        let (mut rt, clock) = timed();
        send(&mut rt, Msg::Edit("unknown(x)".into()));
        clock.advance(PREVIEW_DELAY);
        rt.step(&[]).unwrap();
        assert!(rt.state().error.is_some());
        assert_eq!(rt.state().history.len(), 1);
        send(&mut rt, Msg::Edit("x".into()));
        clock.advance(PREVIEW_DELAY);
        rt.step(&[]).unwrap();
        assert!(rt.state().error.is_none());
        assert!(rt.frame().has_raster());
        send(&mut rt, Msg::Edit("cos(x)".into()));
        let revision = rt.state().revision;
        assert!(rt.step(&[key(KeyCode::Enter)]).unwrap().committed);
        assert_eq!(rt.state().history, ["sin(x)", "cos(x)"]);
        assert!(rt.frame().has_raster());
        // Even an already queued timer message must not leak across the commit.
        rt.handle().send(Msg::Preview(revision)).unwrap();
        rt.step(&[]).unwrap();
        clock.advance(PREVIEW_DELAY);
        rt.step(&[]).unwrap();
        assert_eq!(rt.state().number, 3);
        assert!(rt.state().draft.is_empty());
        assert!(!rt.frame().has_raster());
        assert!(rt.state().error.is_none());
        send(&mut rt, Msg::Edit("unknown(x)".into()));
        clock.advance(PREVIEW_DELAY);
        rt.step(&[]).unwrap();
        send(&mut rt, Msg::Edit("sin(x)".into()));
        assert!(rt.step(&[key(KeyCode::Enter)]).unwrap().committed);
        assert!(rt.frame().has_raster());
        assert!(rt.state().error.is_none());
    }

    #[test]
    fn history_recall_and_draft_restore_schedule_fresh_previews() {
        let (mut rt, clock) = timed();
        send(&mut rt, Msg::Edit("cos(x)".into()));
        let revision = rt.state().revision;
        rt.step(&[key(KeyCode::Up)]).unwrap();
        send(&mut rt, Msg::Preview(revision));
        assert_eq!(rt.state().draft, "sin(x)");
        assert!(!rt.frame().has_raster());
        clock.advance(PREVIEW_DELAY);
        rt.step(&[]).unwrap();
        assert!(rt.frame().has_raster());
        rt.step(&[key(KeyCode::Down)]).unwrap();
        assert_eq!(rt.state().draft, "cos(x)");
        assert!(!rt.frame().has_raster());
        clock.advance(PREVIEW_DELAY);
        rt.step(&[]).unwrap();
        assert!(rt.frame().has_raster());
        assert_eq!(rt.state().history, ["sin(x)"]);
        send(&mut rt, Msg::Edit("x".repeat(MAX_INPUT + 1)));
        rt.step(&[key(KeyCode::Up), key(KeyCode::Down)]).unwrap();
        clock.advance(PREVIEW_DELAY);
        rt.step(&[]).unwrap();
        assert!(rt.state().validation_error.is_some());
        assert!(!rt.frame().has_raster());
    }

    #[test]
    fn repeated_commits_keep_result_until_continuation_and_release_raster() {
        let mut rt = runtime();
        assert!(rt.step(&[]).unwrap().committed);
        assert!(rt.frame().has_raster());
        let old = Rc::downgrade(rt.state().output.as_ref().unwrap());
        rt.step(&[]).unwrap();
        assert!(old.upgrade().is_none());
        for number in 2..6 {
            assert_eq!(rt.focused_key(), Some("editor"));
            assert!(!rt.frame().has_raster());
            let result = rt
                .step(&[
                    key(KeyCode::Char('x')),
                    key(KeyCode::F(5)),
                    key(KeyCode::Char('2')),
                ])
                .unwrap();
            assert!(result.committed);
            assert_eq!(rt.state().number, number);
            assert_eq!(rt.state().draft, "x");
            assert!(rt.frame().has_raster());
            rt.step(&[]).unwrap();
            assert_eq!(rt.state().draft, "2");
            assert_eq!(rt.focused_key(), Some("editor"));
            rt.step(&[key(KeyCode::Backspace)]).unwrap();
        }
    }

    #[test]
    fn errors_and_empty_inputs_commit_then_recover() {
        let mut rt = runtime();
        rt.step(&[]).unwrap();
        rt.step(&[]).unwrap();
        for source in ["unknown(x)", "", "sin(x)"] {
            assert!(
                rt.step(&[Event::Paste(source.into()), key(KeyCode::Enter)])
                    .unwrap()
                    .committed
            );
            assert_eq!(rt.state().error.is_some(), source != "sin(x)");
            assert_eq!(rt.frame().has_raster(), source == "sin(x)");
            rt.step(&[]).unwrap();
            assert!(rt.state().error.is_none());
            assert!(rt.state().draft.is_empty());
        }
    }

    #[test]
    fn bounded_history_restores_draft_and_can_be_edited() {
        let mut rt = runtime();
        rt.step(&[]).unwrap();
        for i in 0..MAX_HISTORY + 3 {
            rt.step(&[]).unwrap();
            assert!(
                rt.step(&[Event::Paste(format!("bad{i}")), key(KeyCode::Enter)])
                    .unwrap()
                    .committed
            );
        }
        rt.step(&[]).unwrap();
        assert_eq!(rt.state().history.len(), MAX_HISTORY);
        assert_eq!(rt.state().history[0], "bad3");
        rt.step(&[Event::Paste("draft".into()), key(KeyCode::Up)])
            .unwrap();
        assert_eq!(rt.state().draft, "bad26");
        rt.step(&[key(KeyCode::Down)]).unwrap();
        assert_eq!(rt.state().draft, "draft");
        for _ in 0..MAX_HISTORY + 2 {
            rt.step(&[key(KeyCode::Up)]).unwrap();
        }
        assert_eq!(rt.state().draft, "bad3");
        rt.step(&[key(KeyCode::End), Event::Paste("+x".into())])
            .unwrap();
        assert_eq!(rt.state().draft, "bad3+x");
        assert_eq!(rt.state().history[0], "bad3");
        assert_eq!(rt.focused_key(), Some("editor"));
    }

    #[test]
    fn unicode_limits_and_nonpress_submission() {
        assert!(State::new(Some("x".repeat(MAX_INPUT + 1)), crate::X_RANGE).is_err());
        let mut rt = runtime();
        rt.step(&[]).unwrap();
        rt.step(&[]).unwrap();
        rt.step(&[Event::Paste("界".repeat(MAX_INPUT))]).unwrap();
        assert!(rt.state().draft.len() <= MAX_INPUT);
        assert!(rt.state().validation_error.is_some());
        for kind in [KeyEventKind::Repeat, KeyEventKind::Release] {
            let mut event = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
            event.kind = kind;
            assert!(!rt.step(&[Event::Key(event)]).unwrap().committed);
        }
        assert!(rt.step(&[key(KeyCode::Enter)]).unwrap().committed);
        assert!(rt.state().error.is_some());
        assert!(!rt.frame().has_raster());
    }

    fn assert_no_prompt_border(rt: &Runtime<State, Msg>) {
        for cell in rt.frame().content() {
            assert!(
                !cell
                    .content
                    .symbol()
                    .chars()
                    .any(|ch| ('\u{2500}'..='\u{257f}').contains(&ch)),
                "unexpected box-drawing glyph: {:?}",
                cell.content.symbol()
            );
        }
    }

    fn frame_row(rt: &Runtime<State, Msg>, y: usize) -> String {
        rt.frame().content()[y * 100..(y + 1) * 100]
            .iter()
            .map(|cell| cell.content.symbol())
            .collect()
    }

    fn assert_inline_input(rt: &Runtime<State, Msg>, expected: &str) {
        assert_eq!(frame_row(rt, 0).trim(), expected);
        for cell in &rt.frame().content()[..100] {
            assert_eq!(
                cell.content
                    .style
                    .colors
                    .and_then(|colors| colors.background),
                None,
                "input row must not override the terminal background"
            );
        }
    }

    #[test]
    fn inline_input_stays_linear_with_readable_formula_rows() {
        let (mut rt, clock) = timed();
        assert_inline_input(&rt, "In[2]:=");
        rt.step(&[Event::Paste("sin(x); cos(x)".into())]).unwrap();
        assert_inline_input(&rt, "In[2]:=sin(x); cos(x)");
        let editor = WidgetPath::root()
            .child(WidgetKey::Named("input".into()))
            .child(WidgetKey::Named("editor".into()));
        let area = rt.layout_geometry(&editor).unwrap().target;
        assert_eq!((area.x(), area.y(), area.height()), (8, 0, 1));
        clock.advance(PREVIEW_DELAY);
        rt.step(&[]).unwrap();
        assert_inline_input(&rt, "In[2]:=sin(x); cos(x)");
        assert_eq!(frame_row(&rt, 1).trim(), "Preview (not submitted)");
        assert_eq!(frame_row(&rt, 2).trim(), "sin(x)");
        assert_eq!(frame_row(&rt, 3).trim(), "cos(x)");
        let graph = WidgetPath::root().child(WidgetKey::Named("output".into()));
        assert_eq!(rt.layout_geometry(&graph).unwrap().target.y(), 4);
        assert!(rt.step(&[key(KeyCode::Enter)]).unwrap().committed);
        assert_inline_input(&rt, "In[2]:=sin(x); cos(x)");
        assert_eq!(frame_row(&rt, 1).trim(), "Out[2]=");
        assert_eq!(rt.layout_geometry(&graph).unwrap().target.y(), 4);
        rt.step(&[]).unwrap();
        assert_inline_input(&rt, "In[3]:=");
        assert_eq!(rt.focused_key(), Some("editor"));
        rt.step(&[key(KeyCode::Up)]).unwrap();
        assert_inline_input(&rt, "In[3]:=sin(x); cos(x)");
    }

    const HEART: &str = "param(16*sin(t)^3,13*cos(t)-5*cos(2*t)-2*cos(3*t)-cos(4*t),0,2*pi)";

    #[test]
    fn long_paste_fits_and_delete_repaste_keeps_preview_source() {
        let (mut rt, clock) = timed();
        rt.step(&[Event::Paste(HEART.into())]).unwrap();
        assert_eq!(rt.state().draft, HEART);
        assert_inline_input(&rt, &format!("In[2]:={HEART}"));
        clock.advance(PREVIEW_DELAY);
        rt.step(&[]).unwrap();
        assert_eq!(
            rt.state().bounds,
            Plot::parse(HEART).unwrap().auto_bounds(crate::X_RANGE)
        );
        let expected = Plot::parse(HEART).unwrap();
        assert_eq!(
            rt.state()
                .output
                .as_ref()
                .unwrap()
                .formula
                .text_rows()
                .collect::<Vec<_>>(),
            expected.formula.text_rows().collect::<Vec<_>>()
        );
        rt.step(&[
            key(KeyCode::Home),
            Event::Key(KeyEvent::new(KeyCode::End, KeyModifiers::SHIFT)),
            key(KeyCode::Delete),
        ])
        .unwrap();
        assert!(rt.state().draft.is_empty());
        assert!(rt.state().output.is_none());
        assert_inline_input(&rt, "In[2]:=");
        rt.step(&[Event::Paste(HEART.into())]).unwrap();
        assert_eq!(rt.state().draft, HEART);
        assert_inline_input(&rt, &format!("In[2]:={HEART}"));
        clock.advance(PREVIEW_DELAY);
        rt.step(&[]).unwrap();
        assert_eq!(
            rt.state().bounds,
            Plot::parse(HEART).unwrap().auto_bounds(crate::X_RANGE)
        );
        assert_eq!(
            rt.state()
                .output
                .as_ref()
                .unwrap()
                .formula
                .text_rows()
                .collect::<Vec<_>>(),
            expected.formula.text_rows().collect::<Vec<_>>()
        );
        assert!(rt.step(&[key(KeyCode::Enter)]).unwrap().committed);
        assert_eq!(rt.state().history.last().unwrap(), HEART);
    }

    #[test]
    fn overflowing_input_navigation_resize_and_controlled_replacement() {
        let (mut rt, _) = timed();
        rt.step(&[Event::Resize(40, 32), Event::Paste(HEART.into())])
            .unwrap();
        let row = |rt: &Runtime<State, Msg>| -> String {
            rt.frame().content()[..40]
                .iter()
                .map(|c| c.content.symbol())
                .collect()
        };
        assert_eq!(rt.state().draft, HEART);
        assert!(row(&rt).contains("2*pi)"));
        assert!(!row(&rt).contains("param("));
        rt.step(&[key(KeyCode::Home)]).unwrap();
        assert!(row(&rt).contains("In[2]:=param("));
        rt.step(&[key(KeyCode::End), key(KeyCode::Left)]).unwrap();
        assert_eq!(rt.state().draft, HEART);
        rt.step(&[Event::Resize(100, 32)]).unwrap();
        assert_inline_input(&rt, &format!("In[2]:={HEART}"));
        // Resizing must not only repair painting: editing still uses the full buffer.
        rt.step(&[key(KeyCode::Char(' '))]).unwrap();
        assert_eq!(rt.state().draft, format!("{} )", &HEART[..HEART.len() - 1]));
        send(&mut rt, Msg::Edit("界e\u{301}".into()));
        assert_inline_input(&rt, "In[2]:=界e\u{301}");
        rt.step(&[key(KeyCode::End), key(KeyCode::Backspace)])
            .unwrap();
        assert_eq!(rt.state().draft, "界");
        rt.step(&[key(KeyCode::Backspace), Event::Paste(HEART.into())])
            .unwrap();
        assert_eq!(rt.state().draft, HEART);
        assert_inline_input(&rt, &format!("In[2]:={HEART}"));
    }

    #[test]
    fn one_row_editor_normalizes_paste_and_preserves_editing_and_history() {
        let mut rt = application(
            State::new(None, crate::X_RANGE).unwrap(),
            RasterOutput::text_only(),
        )
        .size(100, 32);
        rt.step(&[]).unwrap();
        assert!(
            !rt.step(&[
                Event::Paste("x\r\n+\r2\n+\t3\u{7}".into()),
                key(KeyCode::End),
                key(KeyCode::Backspace),
                key(KeyCode::Char('4')),
                Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT)),
            ])
            .unwrap()
            .committed
        );
        assert_eq!(rt.state().draft, "x + 2 +\t4");
        assert_no_prompt_border(&rt);
        assert!(rt.step(&[key(KeyCode::Enter)]).unwrap().committed);
        assert_eq!(rt.state().history, ["x + 2 +\t4"]);
        assert!(rt.state().error.is_none());
        // The input shares the In row; Out follows immediately below it.
        let width = 100;
        let row: String = rt.frame().content()[width..2 * width]
            .iter()
            .map(|c| c.content.symbol())
            .collect();
        assert!(row.contains("Out[1]="), "{row:?}");
        rt.step(&[]).unwrap();
        assert_no_prompt_border(&rt);
        assert_eq!(rt.focused_key(), Some("editor"));
        rt.step(&[key(KeyCode::Up)]).unwrap();
        assert_eq!(rt.state().draft, "x + 2 +\t4");
        rt.step(&[
            key(KeyCode::End),
            key(KeyCode::Backspace),
            key(KeyCode::Char('5')),
        ])
        .unwrap();
        assert_eq!(rt.state().draft, "x + 2 +\t5");
        assert!(rt.step(&[key(KeyCode::F(5))]).unwrap().committed);
        assert_eq!(rt.state().history, ["x + 2 +\t4", "x + 2 +\t5"]);
    }

    #[test]
    fn blank_start_has_no_commit_history_or_graph_and_focuses_editor() {
        let mut rt = application(
            State::new(None, [-2.0, 4.0]).unwrap(),
            RasterOutput::text_only(),
        )
        .size(100, 32);
        assert!(!rt.step(&[]).unwrap().committed);
        assert_eq!(rt.state().number, 1);
        assert!(rt.state().history.is_empty());
        assert!(rt.state().draft.is_empty());
        assert!(!rt.frame().has_raster());
        assert_eq!(rt.focused_key(), Some("editor"));
        let path = WidgetPath::root()
            .child(WidgetKey::Named("input".into()))
            .child(WidgetKey::Named("editor".into()));
        assert_eq!(rt.layout_geometry(&path).unwrap().target.height(), 1);
        assert_no_prompt_border(&rt);
        assert!(
            rt.step(&[Event::Paste("0.01*x".into()), key(KeyCode::Enter)])
                .unwrap()
                .committed
        );
        assert_eq!(rt.state().number, 1);
        assert_eq!(&rt.state().bounds[..2], &[-2.0, 4.0]);
        assert!(rt.state().bounds[2] < -0.02 && rt.state().bounds[3] > 0.04);
        rt.step(&[]).unwrap();
        assert_eq!(rt.state().number, 2);
        assert_eq!(rt.state().history, ["0.01*x"]);
    }

    #[test]
    fn explicit_empty_initial_expression_still_commits_error() {
        let mut rt = application(
            State::new(Some(String::new()), crate::X_RANGE).unwrap(),
            RasterOutput::text_only(),
        );
        assert!(rt.step(&[]).unwrap().committed);
        assert!(rt.state().error.is_some());
    }

    #[test]
    fn initial_invalid_expression_is_a_committed_error() {
        let mut rt = application(
            State::new(Some("bad(x)".into()), crate::X_RANGE).unwrap(),
            RasterOutput::text_only(),
        );
        assert!(rt.step(&[]).unwrap().committed);
        assert!(rt.state().error.is_some());
        rt.step(&[]).unwrap();
        assert_eq!(rt.state().number, 2);
        assert_eq!(rt.focused_key(), Some("editor"));
    }

    #[test]
    fn preview_resize_and_submission_share_source_and_target_cache() {
        for cell in [(1, 2), (8, 16), (10, 20)] {
            // Headless frames retain raster metadata even without native output.
            let (mut rt, clock) = timed_with_pixels(cell);
            send(&mut rt, Msg::Edit("sin(x)".into()));
            clock.advance(PREVIEW_DELAY);
            rt.step(&[]).unwrap();
            let source = Rc::clone(rt.state().output.as_ref().unwrap());
            let bounds = rt.state().bounds;
            let columns = source.formula.width(cell);
            let image = source.image(
                bounds,
                (
                    u32::from(columns) * u32::from(cell.0),
                    14 * u32::from(cell.1),
                ),
            );
            for (_, _, raster) in rt.frame().raster_cells() {
                assert_eq!(
                    raster.canvas.width(),
                    u32::from(raster.target.width) * u32::from(cell.0)
                );
                assert_eq!(
                    raster.canvas.height(),
                    u32::from(raster.target.height) * u32::from(cell.1)
                );
                if raster.target.height == GRAPH_ROWS {
                    assert_eq!(raster.canvas.pixels(), image.pixels());
                }
            }
            rt.step(&[Event::Resize(32, 32)]).unwrap();
            assert!(Rc::ptr_eq(&source, rt.state().output.as_ref().unwrap()));
            let resized = source.image(
                bounds,
                (
                    u32::from(columns.min(30)) * u32::from(cell.0),
                    14 * u32::from(cell.1),
                ),
            );
            for (_, _, raster) in rt.frame().raster_cells() {
                if raster.target.height != GRAPH_ROWS {
                    continue;
                }
                assert_eq!(raster.canvas.width(), resized.width());
                assert_eq!(raster.canvas.height(), resized.height());
                assert_eq!(raster.canvas.pixels(), resized.pixels());
            }
            rt.step(&[]).unwrap();
            assert!(std::sync::Arc::ptr_eq(
                &resized,
                &source.image(bounds, (resized.width(), resized.height()))
            ));
            assert!(rt.step(&[key(KeyCode::Enter)]).unwrap().committed);
            assert!(Rc::ptr_eq(&source, rt.state().output.as_ref().unwrap()));
        }
    }

    #[test]
    fn formula_preview_commit_cache_and_text_fallback() {
        let (mut rt, clock) = timed_with_pixels((8, 16));
        send(&mut rt, Msg::Edit("sqrt(1/(1+x^2))".into()));
        clock.advance(PREVIEW_DELAY);
        rt.step(&[]).unwrap();
        let source = Rc::clone(rt.state().output.as_ref().unwrap());
        let path = WidgetPath::root().child(WidgetKey::Named("formula".into()));
        let area = rt.layout_geometry(&path).unwrap().target;
        assert_eq!(area.y(), 2);
        let size = (u32::from(area.width()) * 8, u32::from(area.height()) * 16);
        let image = source.formula.image(size, 16).unwrap();
        assert!(rt
            .frame()
            .raster_cells()
            .any(|(_, _, r)| r.canvas.pixels() == image.pixels()));
        assert!(rt.step(&[key(KeyCode::Enter)]).unwrap().committed);
        assert!(std::sync::Arc::ptr_eq(
            &image,
            &source.formula.image(size, 16).unwrap()
        ));
        rt.step(&[]).unwrap();
        assert!(!rt.frame().has_raster());
        assert!(rt.state().output.is_none());

        for pixels in [(1, 2), (1, 16)] {
            let (mut rt, clock) = timed_with_pixels(pixels);
            send(&mut rt, Msg::Edit("param(cos(t),sin(t),0,2*pi)".into()));
            clock.advance(PREVIEW_DELAY);
            rt.step(&[]).unwrap();
            assert_eq!(frame_row(&rt, 2).trim(), "⎰ x(t) = cos(t)  t in [0, 2*pi]");
            assert_eq!(frame_row(&rt, 3).trim(), "⎱ y(t) = sin(t)");
            let path = WidgetPath::root().child(WidgetKey::Named("formula".into()));
            if pixels == (1, 2) {
                assert_eq!(rt.layout_geometry(&path).unwrap().target.height(), 2);
            }
            assert!(rt
                .frame()
                .raster_cells()
                .all(|(_, _, r)| r.target.height == GRAPH_ROWS));
        }
    }

    #[test]
    fn formulas_use_terminal_width_instead_of_graph_width() {
        let (rt, clock) = timed_with_pixels((8, 16));
        let mut rt = rt.size(160, 32);
        send(
            &mut rt,
            Msg::Edit("param(16*sin(t)^3,13*cos(t)-5*cos(2*t)-2*cos(3*t)-cos(4*t),0,2*pi)".into()),
        );
        clock.advance(PREVIEW_DELAY);
        rt.step(&[]).unwrap();
        let path = WidgetPath::root().child(WidgetKey::Named("formula".into()));
        let area = rt.layout_geometry(&path).unwrap().target;
        assert_eq!(
            area.width(),
            rt.state().output.as_ref().unwrap().formula.width((8, 16))
        );
        assert!(area.width() < 158);
        let image = rt
            .state()
            .output
            .as_ref()
            .unwrap()
            .formula
            .image(
                (u32::from(area.width()) * 8, u32::from(area.height()) * 16),
                16,
            )
            .expect("the heart equations should fit a wide terminal");
        assert!(rt
            .frame()
            .raster_cells()
            .any(|(_, _, r)| r.canvas.pixels() == image.pixels()));
        let graph = WidgetPath::root().child(WidgetKey::Named("output".into()));
        assert_eq!(
            rt.layout_geometry(&graph).unwrap().target.width(),
            area.width()
        );
    }

    #[test]
    fn native_formula_and_graph_share_content_width_and_leave_blank_space() {
        for source in ["sin(x)", "param(cos(t),sin(t),0,2*pi)"] {
            let (rt, clock) = timed_with_pixels((8, 16));
            let mut rt = rt.size(160, 32);
            send(&mut rt, Msg::Edit(source.into()));
            clock.advance(PREVIEW_DELAY);
            rt.step(&[]).unwrap();
            let columns = rt.state().output.as_ref().unwrap().formula.width((8, 16));
            assert!(columns < 158);
            for terminal_width in [160, 200, 12, 160] {
                rt.step(&[Event::Resize(terminal_width, 32)]).unwrap();
                let formula = WidgetPath::root().child(WidgetKey::Named("formula".into()));
                let graph = WidgetPath::root().child(WidgetKey::Named("output".into()));
                let f = rt.layout_geometry(&formula).unwrap().target;
                let g = rt.layout_geometry(&graph).unwrap().target;
                assert_eq!(f.width(), columns.min(terminal_width - 2));
                assert_eq!((f.x(), f.width()), (g.x(), g.width()));
                for (x, _, raster) in rt.frame().raster_cells() {
                    assert!(x < f.x() + f.width());
                    assert_eq!(raster.canvas.width(), u32::from(f.width()) * 8);
                }
            }
        }
    }

    #[test]
    fn raster_geometry_is_compact_at_different_sizes() {
        for (width, height) in [(160, 50), (80, 24), (40, 24), (100, 32)] {
            let mut rt = runtime().size(width, height);
            assert!(rt.step(&[]).unwrap().committed);
            let path = WidgetPath::root().child(WidgetKey::Named("output".into()));
            let geometry = rt.layout_geometry(&path).unwrap();
            assert_eq!(geometry.target.width(), 70.min(width - 2));
            assert_eq!(geometry.target.height(), GRAPH_ROWS);
            assert!(rt.frame().has_raster());
        }
    }
}
