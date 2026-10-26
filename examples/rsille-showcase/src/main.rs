use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rsille::canvas::Canvas;
use rsille::tui::prelude::*;

const SHOWOFF_NOTES: &str = r#"# Showoff mode

This page is intentionally theatrical: a particle tunnel, rotating wireframe,
wave ribbons, and a live signal field are all drawn with the braille canvas.

- The same canvas widget scales from a tiny chart to a full-screen scene.
- Controls mutate real app state and immediately alter the drawing callback.
- Row-based color keeps dense terminal graphics readable.
"#;

const WORKBENCH_NOTES: &str = r#"# Real app surface

This tab behaves like a compact code review cockpit:

1. Browse repository files.
2. Focus or open a file to refresh the preview.
3. Inspect a generated patch beside the real source text.
4. Watch app events and command palette state update live.
"#;

#[derive(Debug, Clone)]
enum Msg {
    Frame(FrameInfo),
    Tick,
    TabChanged(String),
    FilterChanged(String),
    ModeChanged(String),
    ToggleLive(bool),
    FileFocused(String),
    FileOpened(String),
    FileLoad(String),
    ProcessFocused(String),
    ProcessOpened(String),
    ProcessCellFocused(String, String),
    SelectedProcesses(Vec<String>),
    CalcChanged(String),
    CalcSubmitted(String),
    CalcButton(String),
    PaletteHighlighted(String),
    RunCommand(String),
    TogglePalette,
    ClosePalette,
    OpenDialog,
    CloseDialog,
}

#[derive(Debug)]
struct State {
    frame: u64,
    elapsed_secs: f32,
    pulse: usize,
    tab: String,
    filter: String,
    mode: String,
    live: bool,
    file: String,
    opened_file: String,
    load_request: String,
    file_preview: String,
    file_diff: String,
    process: String,
    opened_process: String,
    process_cell: String,
    selected_processes: Vec<String>,
    calc_expr: String,
    calc_result: String,
    calc_history: Vec<String>,
    command: String,
    highlighted_command: String,
    show_palette: bool,
    show_dialog: bool,
}

impl Default for State {
    fn default() -> Self {
        let initial_file = "Cargo.toml".to_owned();
        let preview = read_preview(&initial_file);
        let result = calculator_result("((13 + 21) * 3) / 8");

        Self {
            frame: 0,
            elapsed_secs: 0.0,
            pulse: 0,
            tab: "workbench".to_owned(),
            filter: String::new(),
            mode: "tunnel".to_owned(),
            live: true,
            file: initial_file.clone(),
            opened_file: initial_file.clone(),
            load_request: String::new(),
            file_diff: diff_for_file(&initial_file, &preview),
            file_preview: preview,
            process: "rsille-tui".to_owned(),
            opened_process: String::new(),
            process_cell: String::new(),
            selected_processes: Vec::new(),
            calc_expr: "((13 + 21) * 3) / 8".to_owned(),
            calc_result: result,
            calc_history: vec![
                "144 / 12 = 12".to_owned(),
                "2 ^ 8 + 17 = 273".to_owned(),
                "sqrt is intentionally not builtin".to_owned(),
            ],
            command: "boot".to_owned(),
            highlighted_command: String::new(),
            show_palette: false,
            show_dialog: false,
        }
    }
}

fn main() -> WidgetResult<()> {
    App::new(State::default())
        .on_frame(Msg::Frame)
        .on_tick(Duration::from_millis(900), || Msg::Tick)
        .mode(Mode::Inline)
        .update(|state, msg, _ctx| update(state, msg))
        .view(view)
        .run()
        .map(|_| ())
}

fn update(state: &mut State, msg: Msg) {
    match msg {
        Msg::Frame(info) => {
            state.frame = info.frame;
            state.elapsed_secs = info.since_start.as_secs_f32();
        }
        Msg::Tick => state.pulse = state.pulse.wrapping_add(1),
        Msg::TabChanged(value) => state.tab = value,
        Msg::FilterChanged(value) => state.filter = value,
        Msg::ModeChanged(value) => state.mode = value,
        Msg::ToggleLive(value) => state.live = value,
        Msg::FileFocused(path) => set_file(state, path, false),
        Msg::FileOpened(path) => set_file(state, path, true),
        Msg::FileLoad(path) => state.load_request = path,
        Msg::ProcessFocused(id) => state.process = id,
        Msg::ProcessOpened(id) => state.opened_process = id,
        Msg::ProcessCellFocused(row, column) => state.process_cell = format!("{row}/{column}"),
        Msg::SelectedProcesses(ids) => state.selected_processes = ids,
        Msg::CalcChanged(expr) => {
            state.calc_result = calculator_result(&expr);
            state.calc_expr = expr;
        }
        Msg::CalcSubmitted(expr) => submit_calculation(state, expr),
        Msg::CalcButton(value) => press_calculator_button(state, &value),
        Msg::PaletteHighlighted(id) => state.highlighted_command = id,
        Msg::TogglePalette => state.show_palette = !state.show_palette,
        Msg::ClosePalette => state.show_palette = false,
        Msg::OpenDialog => state.show_dialog = true,
        Msg::CloseDialog => state.show_dialog = false,
        Msg::RunCommand(id) => run_command(state, id),
    }
}

fn set_file(state: &mut State, path: String, opened: bool) {
    state.file = path.clone();
    if opened {
        state.opened_file = path.clone();
    }

    let preview = read_preview(&path);
    state.file_diff = diff_for_file(&path, &preview);
    state.file_preview = preview;
}

fn press_calculator_button(state: &mut State, value: &str) {
    match value {
        "clear" => {
            state.calc_expr.clear();
            state.calc_result = calculator_result(&state.calc_expr);
        }
        "back" => {
            state.calc_expr.pop();
            state.calc_result = calculator_result(&state.calc_expr);
        }
        "eval" => submit_calculation(state, state.calc_expr.clone()),
        "ans" => {
            if !state.calc_result.starts_with("expected")
                && !state.calc_result.starts_with("unexpected")
                && !state.calc_result.starts_with("missing")
                && !state.calc_result.starts_with("division")
                && !state.calc_result.starts_with("invalid")
                && state.calc_result != "empty expression"
                && state.calc_result != "not finite"
            {
                state.calc_expr.push_str(&state.calc_result);
                state.calc_result = calculator_result(&state.calc_expr);
            }
        }
        token => {
            state.calc_expr.push_str(token);
            state.calc_result = calculator_result(&state.calc_expr);
        }
    }
}

fn submit_calculation(state: &mut State, expr: String) {
    state.calc_result = calculator_result(&expr);
    state.calc_expr = expr.clone();
    if !expr.trim().is_empty() {
        state
            .calc_history
            .insert(0, format!("{expr} = {}", state.calc_result));
        state.calc_history.truncate(8);
    }
}

fn run_command(state: &mut State, id: String) {
    state.command = id.clone();
    state.show_palette = false;

    match id.as_str() {
        "workbench" | "top" | "calc" | "showoff" => state.tab = id,
        "toggle-live" => state.live = !state.live,
        "mode-tunnel" => {
            state.tab = "showoff".to_owned();
            state.mode = "tunnel".to_owned();
        }
        "mode-cube" => {
            state.tab = "showoff".to_owned();
            state.mode = "cube".to_owned();
        }
        "mode-field" => {
            state.tab = "showoff".to_owned();
            state.mode = "field".to_owned();
        }
        "open-dialog" => state.show_dialog = true,
        _ => {}
    }
}

fn view(state: &State) -> impl Widget<Msg> {
    let app = col::<Msg>()
        .style(Style::default().bg(Color::Rgb(7, 10, 16)))
        .padding(Padding::uniform(1))
        .gap(1)
        .child(header(state))
        .child(nav(state))
        .child(page(state))
        .child(status_bar(state));

    let ui = overlay(app);
    let ui = if state.show_palette {
        ui.layer(
            OverlayLayer::new(command_palette_view())
                .floating(OverlayAnchor::Center)
                .size(62, 14)
                .z_index(20),
        )
        .trap_focus()
    } else {
        ui
    };

    if state.show_dialog {
        ui.layer(
            OverlayLayer::new(about_dialog())
                .floating(OverlayAnchor::Center)
                .size(54, 11)
                .z_index(30),
        )
        .trap_focus()
    } else {
        ui
    }
}

fn header(state: &State) -> impl Widget<Msg> {
    row::<Msg>()
        .style(Style::default().bg(Color::Rgb(13, 18, 28)))
        .padding(Padding::new(0, 1, 1, 1))
        .gap(2)
        .child(label("RSILLE STUDIO").bold().fg(Color::Rgb(111, 224, 255)))
        .child(label(format!("frame {:<5}", state.frame)).fg(Color::Rgb(163, 177, 198)))
        .child(label(format!("mode {}", state.mode)).fg(Color::Rgb(186, 170, 255)))
        .child(
            loading_indicator::<Msg>()
                .frame(state.frame as usize)
                .label(if state.live { "live" } else { "hold" }),
        )
        .child(
            button("Cmd")
                .variant(ButtonVariant::Secondary)
                .on_click(|| Msg::TogglePalette),
        )
        .child(button("About").on_click(|| Msg::OpenDialog))
}

fn nav(state: &State) -> impl Widget<Msg> {
    tabs::<Msg>()
        .key("tabs")
        .selected(state.tab.clone())
        .tabs([
            TabItem::new("workbench", "Workbench"),
            TabItem::new("top", "Top"),
            TabItem::new("calc", "Calculator"),
            TabItem::new("showoff", "Showoff"),
        ])
        .on_change(Msg::TabChanged)
}

fn page(state: &State) -> Box<dyn Widget<Msg>> {
    match state.tab.as_str() {
        "top" => Box::new(top_page(state)),
        "calc" => Box::new(calculator_page(state)),
        "showoff" => Box::new(showoff_page(state)),
        _ => Box::new(workbench_page(state)),
    }
}

fn workbench_page(state: &State) -> impl Widget<Msg> {
    col::<Msg>()
        .gap(1)
        .child(
            row::<Msg>()
                .gap(1)
                .child(project_tree_card(state))
                .child(file_preview_card(state)),
        )
        .child(
            row::<Msg>()
                .gap(1)
                .child(diff_card(state))
                .child(workbench_activity_card(state)),
        )
}

fn top_page(state: &State) -> impl Widget<Msg> {
    col::<Msg>()
        .gap(1)
        .child(
            row::<Msg>()
                .gap(1)
                .child(process_table_card(state))
                .child(process_inspector_card(state)),
        )
        .child(resource_canvas_card(state))
}

fn calculator_page(state: &State) -> impl Widget<Msg> {
    row::<Msg>()
        .gap(1)
        .child(calculator_card(state))
        .child(calculator_history_card(state))
}

fn showoff_page(state: &State) -> impl Widget<Msg> {
    col::<Msg>().gap(1).child(showoff_canvas_card(state)).child(
        row::<Msg>()
            .gap(1)
            .child(showoff_controls_card(state))
            .child(
                panel::<Msg>()
                    .title("Canvas notes")
                    .border(BorderStyle::Rounded)
                    .padding(Padding::uniform(1))
                    .child(
                        markdown_viewer::<Msg>(SHOWOFF_NOTES)
                            .key("showoff-notes")
                            .height(8),
                    ),
            ),
    )
}

fn project_tree_card(_state: &State) -> impl Widget<Msg> {
    panel::<Msg>()
        .title("Repository browser")
        .border(BorderStyle::Rounded)
        .padding(Padding::horizontal(1))
        .gap(1)
        .style(Style::default().bg(Color::Rgb(10, 14, 22)))
        .child(
            file_explorer::<Msg>()
                .key("repo-files")
                .height(18)
                .border(BorderStyle::Rounded)
                .multi_select(true)
                .items(project_file_tree())
                .on_change(Msg::FileFocused)
                .on_open(Msg::FileOpened)
                .on_load_children(Msg::FileLoad),
        )
}

fn file_preview_card(state: &State) -> impl Widget<Msg> {
    panel::<Msg>()
        .title(format!("Preview: {}", compact_path(&state.file)))
        .border(BorderStyle::Rounded)
        .padding(Padding::horizontal(1))
        .gap(1)
        .style(Style::default().bg(Color::Rgb(9, 13, 20)))
        .child(
            code_viewer::<Msg>(&state.file_preview)
                .key("file-preview")
                .language(language_for(&state.file))
                .height(18)
                .border(BorderStyle::Rounded),
        )
}

fn diff_card(state: &State) -> impl Widget<Msg> {
    panel::<Msg>()
        .title(format!(
            "Patch sketch: {}",
            compact_path(&state.opened_file)
        ))
        .border(BorderStyle::Rounded)
        .padding(Padding::horizontal(1))
        .style(Style::default().bg(Color::Rgb(9, 13, 20)))
        .child(
            diff_viewer::<Msg>(&state.file_diff)
                .key("file-diff")
                .height(14)
                .border(BorderStyle::Rounded),
        )
}

fn workbench_activity_card(state: &State) -> impl Widget<Msg> {
    panel::<Msg>()
        .title("Activity")
        .border(BorderStyle::Rounded)
        .padding(Padding::horizontal(1))
        .gap(1)
        .style(Style::default().bg(Color::Rgb(12, 16, 24)))
        .child(
            markdown_viewer::<Msg>(WORKBENCH_NOTES)
                .key("workbench-notes")
                .height(7),
        )
        .child(
            log_viewer::<Msg>()
                .key("workbench-log")
                .height(6)
                .border(BorderStyle::Rounded)
                .lines(workbench_logs(state)),
        )
}

fn process_table_card(state: &State) -> impl Widget<Msg> {
    panel::<Msg>()
        .title("Enhanced top")
        .border(BorderStyle::Rounded)
        .padding(Padding::uniform(1))
        .gap(1)
        .style(Style::default().bg(Color::Rgb(10, 14, 22)))
        .child(
            text_input::<Msg>()
                .key("process-filter")
                .value(state.filter.clone())
                .placeholder("filter process")
                .on_change(Msg::FilterChanged),
        )
        .child(
            data_table::<Msg>()
                .key("processes")
                .height(14)
                .navigation_mode(DataTableNavigationMode::Cell)
                .multi_select(true)
                .filter_query_opt((!state.filter.trim().is_empty()).then(|| state.filter.clone()))
                .sort(DataTableSort::new("cpu", DataTableSortDirection::Desc))
                .columns([
                    DataTableColumn::new("Process")
                        .id("proc")
                        .width(18)
                        .filterable(true),
                    DataTableColumn::new("CPU")
                        .id("cpu")
                        .width(6)
                        .align(TableAlign::Right)
                        .sortable(true),
                    DataTableColumn::new("Mem")
                        .id("mem")
                        .width(6)
                        .align(TableAlign::Right)
                        .sortable(true),
                    DataTableColumn::new("IO")
                        .id("io")
                        .width(6)
                        .align(TableAlign::Right),
                    DataTableColumn::new("State")
                        .id("state")
                        .width(9)
                        .filterable(true),
                ])
                .rows(process_rows(state))
                .on_change(Msg::ProcessFocused)
                .on_submit(Msg::ProcessOpened)
                .on_cell_change(Msg::ProcessCellFocused)
                .on_selection_change(Msg::SelectedProcesses),
        )
}

fn process_inspector_card(state: &State) -> impl Widget<Msg> {
    let stats = process_stats(state, &state.process);

    panel::<Msg>()
        .title("Inspector")
        .border(BorderStyle::Rounded)
        .padding(Padding::uniform(1))
        .gap(1)
        .style(Style::default().bg(Color::Rgb(12, 16, 24)))
        .child(label(format!("Focus: {}", compact(&state.process))).bold())
        .child(label(format!("Open: {}", compact(&state.opened_process))))
        .child(label(format!("Cell: {}", compact(&state.process_cell))))
        .child(label(format!(
            "Selected rows: {}",
            state.selected_processes.len()
        )))
        .child(divider().text("load"))
        .child(metric_row(
            "cpu",
            stats.cpu / 100.0,
            Color::Rgb(111, 224, 255),
        ))
        .child(metric_row(
            "mem",
            stats.mem / 100.0,
            Color::Rgb(159, 255, 190),
        ))
        .child(metric_row(
            "io",
            stats.io / 100.0,
            Color::Rgb(255, 204, 102),
        ))
        .child(divider().text("operators"))
        .child(
            row::<Msg>()
                .gap(1)
                .child(
                    button("Freeze")
                        .variant(ButtonVariant::Secondary)
                        .on_click(|| Msg::ToggleLive(false)),
                )
                .child(button("Live").on_click(|| Msg::ToggleLive(true))),
        )
}

fn resource_canvas_card(state: &State) -> impl Widget<Msg> {
    panel::<Msg>()
        .title("Resource timeline")
        .border(BorderStyle::Rounded)
        .padding(Padding::uniform(1))
        .style(Style::default().bg(Color::Rgb(5, 10, 18)))
        .child(resource_canvas(state, 72, 10))
}

fn calculator_card(state: &State) -> impl Widget<Msg> {
    panel::<Msg>()
        .title("Calculator")
        .border(BorderStyle::Double)
        .padding(Padding::uniform(1))
        .gap(1)
        .style(Style::default().bg(Color::Rgb(10, 14, 22)))
        .child(
            text_input::<Msg>()
                .key("calculator-input")
                .value(state.calc_expr.clone())
                .placeholder("type arithmetic, press Enter")
                .on_change(Msg::CalcChanged)
                .on_submit(Msg::CalcSubmitted),
        )
        .child(
            label(format!("= {}", state.calc_result))
                .bold()
                .fg(Color::Rgb(159, 255, 190)),
        )
        .child(divider().text("supported"))
        .child(label("+  -  *  /  ^  parentheses"))
        .child(label("Example: (8 + 5) * 3 ^ 2 / 9"))
        .child(divider().text("keypad"))
        .child(calculator_keypad())
}

fn calculator_keypad() -> impl Widget<Msg> {
    col::<Msg>()
        .gap(1)
        .child(
            row::<Msg>()
                .gap(1)
                .child(calc_action_button(" C ", "clear"))
                .child(calc_action_button("DEL", "back"))
                .child(calc_button(" ( ", "("))
                .child(calc_button(" ) ", ")")),
        )
        .child(
            row::<Msg>()
                .gap(1)
                .child(calc_button(" 7 ", "7"))
                .child(calc_button(" 8 ", "8"))
                .child(calc_button(" 9 ", "9"))
                .child(calc_button(" / ", " / ")),
        )
        .child(
            row::<Msg>()
                .gap(1)
                .child(calc_button(" 4 ", "4"))
                .child(calc_button(" 5 ", "5"))
                .child(calc_button(" 6 ", "6"))
                .child(calc_button(" * ", " * ")),
        )
        .child(
            row::<Msg>()
                .gap(1)
                .child(calc_button(" 1 ", "1"))
                .child(calc_button(" 2 ", "2"))
                .child(calc_button(" 3 ", "3"))
                .child(calc_button(" - ", " - ")),
        )
        .child(
            row::<Msg>()
                .gap(1)
                .child(calc_button(" 0 ", "0"))
                .child(calc_button(" . ", "."))
                .child(calc_button(" ^ ", " ^ "))
                .child(calc_button(" + ", " + ")),
        )
        .child(
            row::<Msg>()
                .gap(1)
                .child(calc_button("00 ", "00"))
                .child(calc_button(" % ", " / 100"))
                .child(calc_action_button("ANS", "ans"))
                .child(calc_action_button(" = ", "eval")),
        )
}

fn calc_button(label: &str, value: &str) -> impl Widget<Msg> {
    let value = value.to_owned();
    button(label)
        .variant(ButtonVariant::Ghost)
        .on_click(move || Msg::CalcButton(value.clone()))
}

fn calc_action_button(label: &str, value: &str) -> impl Widget<Msg> {
    let value = value.to_owned();
    button(label)
        .variant(ButtonVariant::Ghost)
        .on_click(move || Msg::CalcButton(value.clone()))
}

fn calculator_history_card(state: &State) -> impl Widget<Msg> {
    let items = state
        .calc_history
        .iter()
        .enumerate()
        .map(|(index, value)| ListItem::new(format!("h{index}"), value.clone()))
        .collect::<Vec<_>>();

    panel::<Msg>()
        .title("History")
        .border(BorderStyle::Rounded)
        .padding(Padding::uniform(1))
        .gap(1)
        .style(Style::default().bg(Color::Rgb(12, 16, 24)))
        .child(list::<Msg>().key("calc-history").height(10).items(items))
        .child(
            log_viewer::<Msg>()
                .key("calc-log")
                .height(7)
                .border(BorderStyle::Rounded)
                .lines(calculator_logs(state)),
        )
}

fn showoff_canvas_card(state: &State) -> impl Widget<Msg> {
    panel::<Msg>()
        .title("Braille hypercanvas")
        .border(BorderStyle::Double)
        .padding(Padding::uniform(1))
        .style(Style::default().bg(Color::Rgb(3, 6, 12)))
        .child(showoff_canvas(state, 90, 24))
}

fn showoff_controls_card(state: &State) -> impl Widget<Msg> {
    panel::<Msg>()
        .title("Scene controls")
        .border(BorderStyle::Rounded)
        .padding(Padding::uniform(1))
        .gap(1)
        .style(Style::default().bg(Color::Rgb(12, 16, 24)))
        .child(
            switch("Live motion")
                .checked(state.live)
                .on_change(Msg::ToggleLive),
        )
        .child(
            radio_group::<Msg>()
                .key("showoff-mode")
                .selected(state.mode.clone())
                .options([
                    RadioOption::new("tunnel", "particle tunnel"),
                    RadioOption::new("cube", "wireframe cube"),
                    RadioOption::new("field", "signal field"),
                ])
                .on_change(Msg::ModeChanged),
        )
        .child(divider().text("jump"))
        .child(
            row::<Msg>()
                .gap(1)
                .child(
                    button("Top")
                        .variant(ButtonVariant::Secondary)
                        .on_click(|| Msg::RunCommand("top".to_owned())),
                )
                .child(
                    button("Calc")
                        .variant(ButtonVariant::Secondary)
                        .on_click(|| Msg::RunCommand("calc".to_owned())),
                )
                .child(button("Files").on_click(|| Msg::RunCommand("workbench".to_owned()))),
        )
}

fn status_bar(state: &State) -> impl Widget<Msg> {
    row::<Msg>()
        .gap(2)
        .style(Style::default().bg(Color::Rgb(13, 18, 28)))
        .padding(Padding::new(0, 1, 1, 1))
        .child(label("Tab focus").fg(Color::Rgb(163, 177, 198)))
        .child(label("Arrows navigate").fg(Color::Rgb(163, 177, 198)))
        .child(label("Enter activate").fg(Color::Rgb(163, 177, 198)))
        .child(label("Esc quit").fg(Color::Rgb(163, 177, 198)))
        .child(label(format!("cmd {}", state.command)).fg(Color::Rgb(111, 224, 255)))
        .child(
            label(format!("hint {}", compact(&state.highlighted_command)))
                .fg(Color::Rgb(186, 170, 255)),
        )
}

fn command_palette_view() -> impl Widget<Msg> {
    command_palette::<Msg>()
        .key("commands")
        .height(12)
        .title("Commands")
        .prompt(">")
        .items([
            CommandItem::new("workbench", "Open repo workbench")
                .keywords(["files", "diff", "review"]),
            CommandItem::new("top", "Open enhanced top").keywords(["process", "monitor", "cpu"]),
            CommandItem::new("calc", "Open calculator").keywords(["math", "evaluate"]),
            CommandItem::new("showoff", "Open showoff canvas")
                .keywords(["canvas", "braille", "cool"]),
            CommandItem::new("toggle-live", "Toggle live motion").keywords(["pause", "resume"]),
            CommandItem::new("mode-tunnel", "Show particle tunnel").keywords(["scene", "showoff"]),
            CommandItem::new("mode-cube", "Show wireframe cube").keywords(["scene", "3d"]),
            CommandItem::new("mode-field", "Show signal field").keywords(["scene", "field"]),
            CommandItem::new("open-dialog", "About showcase").keywords(["help", "info"]),
        ])
        .on_change(Msg::PaletteHighlighted)
        .on_submit(Msg::RunCommand)
        .on_close(|| Msg::ClosePalette)
}

fn about_dialog() -> impl Widget<Msg> {
    dialog::<Msg>()
        .title("rsille showcase")
        .border(BorderStyle::Double)
        .padding(Padding::uniform(1))
        .gap(1)
        .child(label("A practical terminal studio plus a canvas showoff mode.").bold())
        .child(label("Browse files, inspect diffs, monitor processes, calculate values, and draw dense braille scenes."))
        .child(
            row::<Msg>()
                .gap(2)
                .child(button("Close").on_click(|| Msg::CloseDialog))
                .child(button("Showoff").variant(ButtonVariant::Secondary).on_click(|| Msg::RunCommand("showoff".to_owned()))),
        )
}

fn metric_row(name: &str, value: f64, color: Color) -> impl Widget<Msg> {
    row::<Msg>()
        .gap(1)
        .child(label(format!("{name:<4}")).fg(Color::Rgb(163, 177, 198)))
        .child(
            progress_bar::<Msg>(value)
                .width(26)
                .variant(ProgressBarVariant::Block)
                .label(format!("{:>3}%", (value * 100.0).round() as u8))
                .fill_style(Style::default().fg(color)),
        )
}

fn project_file_tree() -> Vec<FileExplorerItem> {
    ["Cargo.toml", "README.md", "src", "packages", "examples"]
        .into_iter()
        .filter_map(|path| build_file_item(Path::new(path), 0))
        .collect()
}

fn build_file_item(path: &Path, depth: usize) -> Option<FileExplorerItem> {
    let label = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(".");
    let id = path.to_string_lossy().to_string();
    let metadata = fs::metadata(path).ok()?;

    if metadata.is_dir() {
        let mut children = Vec::new();
        if depth < 3 {
            let mut entries = fs::read_dir(path)
                .ok()?
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|entry| !is_hidden_or_heavy(entry))
                .collect::<Vec<_>>();
            entries.sort_by_key(|entry| (!entry.is_dir(), entry.to_string_lossy().to_string()));
            for entry in entries.into_iter().take(18) {
                if let Some(child) = build_file_item(&entry, depth + 1) {
                    children.push(child);
                }
            }
        }
        Some(FileExplorerItem::directory(id, label).children(children))
    } else {
        Some(FileExplorerItem::file(id, label))
    }
}

fn is_hidden_or_heavy(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    name.starts_with('.')
        || matches!(
            name,
            "target" | "Cargo.lock" | "node_modules" | "dist" | "build"
        )
}

fn read_preview(path: &str) -> String {
    let path = PathBuf::from(path);
    if path.is_dir() {
        return format!(
            "{}\n\nDirectory selected. Open child files to preview source and generate a patch sketch.",
            path.display()
        );
    }

    match fs::read_to_string(&path) {
        Ok(content) => content.lines().take(120).collect::<Vec<_>>().join("\n"),
        Err(err) => format!("Could not read {}: {err}", path.display()),
    }
}

fn diff_for_file(path: &str, preview: &str) -> String {
    let title = compact_path(path);
    let mut lines = preview.lines().take(8).collect::<Vec<_>>();
    if lines.is_empty() {
        lines.push("");
    }

    let first = lines.first().copied().unwrap_or("");
    let second = lines.get(1).copied().unwrap_or(first);

    format!(
        "diff --git a/{title} b/{title}\n@@\n-{first}\n+{first}\n+// reviewed in rsille-showcase\n {second}\n@@\n-// plain terminal surface\n+// focusable widgets + braille canvas + command routing\n"
    )
}

fn language_for(path: &str) -> &'static str {
    match Path::new(path).extension().and_then(|ext| ext.to_str()) {
        Some("rs") => "rs",
        Some("toml") => "toml",
        Some("md") => "md",
        Some("json") => "json",
        Some("lock") => "toml",
        _ => "txt",
    }
}

#[derive(Debug, Clone, Copy)]
struct ProcessStats {
    cpu: f64,
    mem: f64,
    io: f64,
}

fn process_rows(state: &State) -> Vec<DataTableRow> {
    process_names()
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let stats = process_stats_by_index(state, index);
            let status = if stats.cpu > 72.0 {
                "hot"
            } else if stats.io > 60.0 {
                "io wait"
            } else if index % 4 == 0 {
                "sleep"
            } else {
                "running"
            };
            DataTableRow::new(
                *name,
                [
                    (*name).to_owned(),
                    format!("{:>4.1}", stats.cpu),
                    format!("{:>4.1}", stats.mem),
                    format!("{:>4.1}", stats.io),
                    status.to_owned(),
                ],
            )
        })
        .collect()
}

fn process_stats(state: &State, process: &str) -> ProcessStats {
    let index = process_names()
        .iter()
        .position(|name| *name == process)
        .unwrap_or(0);
    process_stats_by_index(state, index)
}

fn process_stats_by_index(state: &State, index: usize) -> ProcessStats {
    let phase = if state.live {
        state.elapsed_secs as f64
    } else {
        state.pulse as f64 * 0.37
    };
    let base = index as f64 + 1.0;
    ProcessStats {
        cpu: (42.0 + (phase * 1.4 + base).sin() * 26.0 + base * 3.4).clamp(1.0, 99.0),
        mem: (36.0 + (phase * 0.9 + base * 0.7).cos() * 21.0 + base * 2.1).clamp(1.0, 96.0),
        io: (18.0 + (phase * 1.9 + base * 1.3).sin().abs() * 64.0).clamp(0.0, 100.0),
    }
}

fn process_names() -> [&'static str; 9] {
    [
        "rsille-tui",
        "canvas-raster",
        "syntect-worker",
        "event-loop",
        "layout-engine",
        "diff-viewer",
        "file-indexer",
        "input-router",
        "render-flush",
    ]
}

fn workbench_logs(state: &State) -> Vec<LogLine> {
    vec![
        LogLine::new(
            LogLevel::Info,
            format!("INFO focused={}", compact_path(&state.file)),
        ),
        LogLine::new(
            LogLevel::Debug,
            format!("DEBUG opened={}", compact_path(&state.opened_file)),
        ),
        LogLine::new(
            LogLevel::Trace,
            format!("TRACE load_request={}", compact(&state.load_request)),
        ),
        LogLine::new(LogLevel::Info, format!("INFO command={}", state.command)),
        LogLine::new(LogLevel::Warn, "WARN generated patch is illustrative"),
    ]
}

fn calculator_logs(state: &State) -> Vec<LogLine> {
    vec![
        LogLine::new(LogLevel::Info, format!("INFO expr={}", state.calc_expr)),
        LogLine::new(LogLevel::Info, format!("INFO result={}", state.calc_result)),
        LogLine::new(LogLevel::Debug, "DEBUG parser=recursive-descent"),
        LogLine::new(LogLevel::Trace, "TRACE operators=+,-,*,/,^"),
    ]
}

fn calculator_result(expr: &str) -> String {
    match Parser::new(expr).parse() {
        Ok(value) if value.is_finite() => trim_float(value),
        Ok(_) => "not finite".to_owned(),
        Err(err) => err,
    }
}

fn trim_float(value: f64) -> String {
    if (value - value.round()).abs() < 1e-9 {
        format!("{}", value.round() as i64)
    } else {
        format!("{value:.6}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned()
    }
}

struct Parser<'a> {
    chars: Vec<char>,
    pos: usize,
    source: &'a str,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            chars: source.chars().collect(),
            pos: 0,
            source,
        }
    }

    fn parse(&mut self) -> Result<f64, String> {
        let value = self.parse_expr()?;
        self.skip_ws();
        if self.pos < self.chars.len() {
            Err(format!("unexpected '{}'", self.chars[self.pos]))
        } else if self.source.trim().is_empty() {
            Err("empty expression".to_owned())
        } else {
            Ok(value)
        }
    }

    fn parse_expr(&mut self) -> Result<f64, String> {
        let mut value = self.parse_term()?;
        loop {
            self.skip_ws();
            if self.consume('+') {
                value += self.parse_term()?;
            } else if self.consume('-') {
                value -= self.parse_term()?;
            } else {
                return Ok(value);
            }
        }
    }

    fn parse_term(&mut self) -> Result<f64, String> {
        let mut value = self.parse_power()?;
        loop {
            self.skip_ws();
            if self.consume('*') {
                value *= self.parse_power()?;
            } else if self.consume('/') {
                let rhs = self.parse_power()?;
                if rhs.abs() < f64::EPSILON {
                    return Err("division by zero".to_owned());
                }
                value /= rhs;
            } else {
                return Ok(value);
            }
        }
    }

    fn parse_power(&mut self) -> Result<f64, String> {
        let mut value = self.parse_factor()?;
        self.skip_ws();
        if self.consume('^') {
            value = value.powf(self.parse_power()?);
        }
        Ok(value)
    }

    fn parse_factor(&mut self) -> Result<f64, String> {
        self.skip_ws();
        if self.consume('-') {
            return Ok(-self.parse_factor()?);
        }
        if self.consume('+') {
            return self.parse_factor();
        }
        if self.consume('(') {
            let value = self.parse_expr()?;
            self.skip_ws();
            if !self.consume(')') {
                return Err("missing ')'".to_owned());
            }
            return Ok(value);
        }
        self.parse_number()
    }

    fn parse_number(&mut self) -> Result<f64, String> {
        self.skip_ws();
        let start = self.pos;
        while self.pos < self.chars.len()
            && (self.chars[self.pos].is_ascii_digit() || self.chars[self.pos] == '.')
        {
            self.pos += 1;
        }
        if start == self.pos {
            return Err("expected number".to_owned());
        }
        self.chars[start..self.pos]
            .iter()
            .collect::<String>()
            .parse::<f64>()
            .map_err(|_| "invalid number".to_owned())
    }

    fn skip_ws(&mut self) {
        while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
            self.pos += 1;
        }
    }

    fn consume(&mut self, ch: char) -> bool {
        if self.pos < self.chars.len() && self.chars[self.pos] == ch {
            self.pos += 1;
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
struct SceneState {
    frame: u64,
    elapsed_secs: f32,
    pulse: usize,
    mode: String,
    live: bool,
}

impl SceneState {
    fn new(state: &State) -> Self {
        Self {
            frame: state.frame,
            elapsed_secs: state.elapsed_secs,
            pulse: state.pulse,
            mode: state.mode.clone(),
            live: state.live,
        }
    }

    fn phase(&self) -> f64 {
        if self.live {
            self.elapsed_secs as f64
        } else {
            self.pulse as f64 * 0.2
        }
    }
}

fn showoff_canvas(state: &State, min_width: u16, min_height: u16) -> impl Widget<Msg> {
    let scene = SceneState::new(state);
    canvas::<Msg, _>(move |surface, ctx| draw_showoff_scene(&scene, surface, ctx))
        .key("showoff-canvas")
        .min_size(min_width, min_height)
}

fn resource_canvas(state: &State, min_width: u16, min_height: u16) -> impl Widget<Msg> {
    let scene = SceneState::new(state);
    canvas::<Msg, _>(move |surface, ctx| draw_resource_scene(&scene, surface, ctx))
        .key("resource-canvas")
        .min_size(min_width, min_height)
}

fn draw_showoff_scene(state: &SceneState, surface: &mut Canvas, ctx: CanvasContext) {
    let w = ctx.cell_width() as i32 * 2;
    let h = ctx.cell_height() as i32 * 4;
    if w <= 0 || h <= 0 {
        return;
    }

    let mut canvas = SceneCanvas::new(surface, ctx.cell_height(), 0, 0, ctx.cell_height());
    match state.mode.as_str() {
        "cube" => draw_cube_scene(state, &mut canvas, w, h),
        "field" => draw_field_scene(state, &mut canvas, w, h),
        _ => draw_tunnel_scene(state, &mut canvas, w, h),
    }
}

fn draw_resource_scene(state: &SceneState, surface: &mut Canvas, ctx: CanvasContext) {
    let w = ctx.cell_width() as i32 * 2;
    let h = ctx.cell_height() as i32 * 4;
    if w <= 0 || h <= 0 {
        return;
    }

    let mut canvas = SceneCanvas::new(surface, ctx.cell_height(), 0, 0, ctx.cell_height());
    let phase = state.phase();
    for lane in 0..3 {
        let base = h as f64 * (0.22 + lane as f64 * 0.25);
        let amp = h as f64 * (0.08 + lane as f64 * 0.03);
        for x in 0..w {
            let xf = x as f64;
            let y = base + (xf * 0.08 + phase * (1.0 + lane as f64 * 0.3)).sin() * amp;
            canvas.set(xf, y);
            if x % 7 == 0 {
                canvas.line((xf, 0.0), (xf, y));
            }
        }
    }
}

fn draw_tunnel_scene(state: &SceneState, canvas: &mut SceneCanvas<'_>, w: i32, h: i32) {
    let phase = state.phase() * 1.8;
    let cx = w as f64 / 2.0;
    let cy = h as f64 / 2.0;
    let max_r = (w.min(h * 2) as f64 * 0.48).max(8.0);

    for ring in 0..14 {
        let p = ring as f64 / 14.0;
        let radius = ((p * max_r + phase * 9.0) % max_r).max(2.0);
        let twist = phase * 0.4 + p * std::f64::consts::TAU;
        let steps = (radius as usize * 5).clamp(34, 220);
        for i in 0..steps {
            if (i + ring + state.frame as usize) % 5 == 0 {
                continue;
            }
            let t = i as f64 / steps as f64 * std::f64::consts::TAU + twist;
            let warp = 1.0 + (t * 3.0 + phase).sin() * 0.12;
            canvas.set(
                cx + t.cos() * radius * 1.85 * warp,
                cy + t.sin() * radius * warp,
            );
        }
    }

    for seed in 0..180 {
        let depth = ((seed as f64 * 0.071 + phase * 0.09) % 1.0).max(0.03);
        let angle = seed as f64 * 2.399 + phase * 0.15;
        let radius = max_r * depth;
        let x = cx + angle.cos() * radius * 1.9;
        let y = cy + angle.sin() * radius;
        canvas.set(x, y);
        if seed % 11 == 0 {
            canvas.line((cx, cy), (x, y));
        }
    }
}

fn draw_cube_scene(state: &SceneState, canvas: &mut SceneCanvas<'_>, w: i32, h: i32) {
    let phase = state.phase();
    let cx = w as f64 / 2.0;
    let cy = h as f64 / 2.0;
    let scale = h.min(w / 2) as f64 * 0.72;
    let verts = [
        (-1.0, -1.0, -1.0),
        (1.0, -1.0, -1.0),
        (1.0, 1.0, -1.0),
        (-1.0, 1.0, -1.0),
        (-1.0, -1.0, 1.0),
        (1.0, -1.0, 1.0),
        (1.0, 1.0, 1.0),
        (-1.0, 1.0, 1.0),
    ];
    let projected = verts.map(|v| project_vertex(v, phase, cx, cy, scale));
    for (a, b) in [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0),
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 4),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ] {
        canvas.line(projected[a], projected[b]);
    }
    for &(x, y) in &projected {
        for dx in -1..=1 {
            for dy in -1..=1 {
                canvas.set(x + dx as f64, y + dy as f64);
            }
        }
    }
    for i in 0..64 {
        let t = i as f64 / 64.0 * std::f64::consts::TAU + phase;
        canvas.set(cx + t.cos() * scale * 1.7, cy + t.sin() * scale * 0.34);
    }
}

fn project_vertex(vertex: (f64, f64, f64), phase: f64, cx: f64, cy: f64, scale: f64) -> (f64, f64) {
    let (mut x, mut y, mut z) = vertex;
    let sin_y = (phase * 0.8).sin();
    let cos_y = (phase * 0.8).cos();
    let nx = x * cos_y + z * sin_y;
    let nz = -x * sin_y + z * cos_y;
    x = nx;
    z = nz;

    let sin_x = (phase * 0.55).sin();
    let cos_x = (phase * 0.55).cos();
    let ny = y * cos_x - z * sin_x;
    let nz = y * sin_x + z * cos_x;
    y = ny;
    z = nz;

    let perspective = 2.8 / (z + 4.2);
    (
        cx + x * scale * perspective * 1.8,
        cy + y * scale * perspective,
    )
}

fn draw_field_scene(state: &SceneState, canvas: &mut SceneCanvas<'_>, w: i32, h: i32) {
    let phase = state.phase();
    let step_x = 5;
    let step_y = 4;
    for y in (0..h).step_by(step_y) {
        for x in (0..w).step_by(step_x) {
            let xf = x as f64;
            let yf = y as f64;
            let angle = (xf * 0.035 + phase).sin() + (yf * 0.05 - phase * 0.7).cos();
            let len = 2.0 + (angle * 2.0).sin().abs() * 5.0;
            canvas.line(
                (xf, yf),
                (xf + angle.cos() * len * 1.8, yf + angle.sin() * len),
            );
        }
    }

    for x in 0..w {
        let xf = x as f64;
        let y = h as f64 * 0.5
            + (xf * 0.09 + phase * 1.7).sin() * h as f64 * 0.22
            + (xf * 0.025 - phase).cos() * h as f64 * 0.11;
        canvas.set(xf, y);
        canvas.set(xf, h as f64 - y);
    }
}

fn canvas_row_color(index: usize) -> Color {
    match index % 6 {
        0 => Color::Rgb(111, 224, 255),
        1 => Color::Rgb(159, 255, 190),
        2 => Color::Rgb(186, 170, 255),
        3 => Color::Rgb(255, 204, 102),
        4 => Color::Rgb(255, 118, 128),
        _ => Color::Rgb(147, 213, 255),
    }
}

struct SceneCanvas<'a> {
    surface: &'a mut Canvas,
    offset_x: i32,
    offset_y: i32,
    total_cell_height: u16,
}

impl<'a> SceneCanvas<'a> {
    fn new(
        surface: &'a mut Canvas,
        total_cell_height: u16,
        cell_x: u16,
        cell_y: u16,
        cell_height: u16,
    ) -> Self {
        let bottom_cell = total_cell_height
            .saturating_sub(cell_y)
            .saturating_sub(cell_height);
        Self {
            surface,
            offset_x: cell_x as i32 * 2,
            offset_y: bottom_cell as i32 * 4,
            total_cell_height,
        }
    }

    fn set<T>(&mut self, x: T, y: T) -> &mut Self
    where
        T: Into<f64> + Copy,
    {
        let x = x.into() + self.offset_x as f64;
        let y = y.into() + self.offset_y as f64;
        let row = self.row_for_dot_y(y);
        self.surface.set_color(x, y, canvas_row_color(row));
        self
    }

    fn line(&mut self, xy1: (f64, f64), xy2: (f64, f64)) -> &mut Self {
        let (x1, y1) = (xy1.0.round() as i32, xy1.1.round() as i32);
        let (x2, y2) = (xy2.0.round() as i32, xy2.1.round() as i32);
        let d = |v1, v2| {
            if v1 <= v2 {
                (v2 - v1, 1.0)
            } else {
                (v1 - v2, -1.0)
            }
        };

        let (xdiff, xdir) = d(x1, x2);
        let (ydiff, ydir) = d(y1, y2);
        let r = std::cmp::max(xdiff, ydiff);
        if r == 0 {
            return self.set(x1, y1);
        }

        for i in 0..=r {
            let r = r as f64;
            let i = i as f64;
            let x = x1 as f64 + i * xdiff as f64 / r * xdir;
            let y = y1 as f64 + i * ydiff as f64 / r * ydir;
            self.set(x, y);
        }

        self
    }

    fn row_for_dot_y(&self, y: f64) -> usize {
        let tile_y = (y.round() as i32).div_euclid(4);
        let row = self.total_cell_height as i32 - 1 - tile_y;
        row.max(0) as usize
    }
}

fn compact(value: &str) -> &str {
    if value.is_empty() {
        "none"
    } else {
        value
    }
}

fn compact_path(path: &str) -> String {
    if path.is_empty() {
        return "none".to_owned();
    }
    let mut parts = path.rsplit('/').take(3).collect::<Vec<_>>();
    parts.reverse();
    parts.join("/")
}
