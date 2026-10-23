//! Each page assembles ordinary public widgets; no custom renderer is needed.
use super::{theme, Modal, Msg, State, THEMES};
use rsille::tui::prelude::*;

const CODE: &str = r#"use rsille::tui::prelude::*;

fn main() -> WidgetResult<()> {
    App::new(0)
        .update(|count, (), _ctx| *count += 1)
        .view(|count| {
            button(format!("Count: {count}"))
                .on_click(|| ())
        })
        .run()?;
    Ok(())
}"#;

const MARKDOWN: &str = r#"# One framework, many surfaces

Compose **widgets**, layout and state into a terminal app.

## Try the gallery

- Controls: edit Unicode text, choose a date.
- Data: filter, sort and select table rows.
- Layout: resize a split or scroll a viewport.
- Motion: braille waves and visual effects.

> F1 opens a searchable command palette.

Notes autosave to memory using debounce and a worker.
"#;

const DIFF: &str = r#"diff --git a/app.rs b/app.rs
--- a/app.rs
+++ b/app.rs
@@ -1,5 +1,6 @@
 fn view(count: &i32) -> impl Widget<()> {
-    label(format!("Count: {count}"))
+    button(format!("Count: {count}"))
+        .on_click(|| ())
 }
"#;

pub(super) fn view(state: &State) -> Box<dyn Widget<Msg>> {
    match state.page.as_str() {
        "controls" => Box::new(controls(state)),
        "collections" => Box::new(collections(state)),
        "content" => Box::new(content(state)),
        "layout" => Box::new(layout(state)),
        "motion" => Box::new(motion(state)),
        "themes" => Box::new(themes(state)),
        _ => Box::new(overview(state)),
    }
}

fn card(title: &str) -> Panel<Msg> {
    panel()
        .title(title)
        .border(BorderStyle::Rounded)
        .padding(Padding::new(0, 1, 0, 1))
        .gap(1)
}

fn two_by_two() -> Grid<Msg> {
    grid().columns("1fr 1fr").rows("18 16").gap(1)
}

fn overview(state: &State) -> impl Widget<Msg> {
    let styles = theme(state.theme).styles;
    two_by_two()
        .child(
            card("01 / Welcome")
                .gap(0)
                .child(label("Build a terminal app from small pieces.").bold())
                .child(label("An interactive tour of the rsille TUI API.").style(styles.text_muted))
                .child(divider().text("Explore"))
                .child(label("Controls   Inputs, toggles, choices, calendar"))
                .child(label("Data       Tables, lists, trees, explorer"))
                .child(label("Content    Code, diff, markdown, logs"))
                .child(label("Layout     Flex, grid, split, scroll, stack"))
                .child(label("Motion     Canvas, visual and layout effects"))
                .child(label("Themes     Five palettes and semantic styles"))
                .child(
                    button("Start with controls →")
                        .key("start")
                        .on_click(|| Msg::Page("controls".into())),
                ),
        )
        .child(
            card("02 / Live display widgets")
                .child(label("Animated sample progress (not a real job)").style(styles.text_muted))
                .child(
                    progress_bar((state.ticks % 101) as f64 / 100.0)
                        .label("Line")
                        .variant(ProgressBarVariant::Line),
                )
                .child(
                    progress_bar(0.42)
                        .label("Block / 42%")
                        .variant(ProgressBarVariant::Block),
                )
                .child(
                    progress_bar(0.66)
                        .label("Segmented / 66%")
                        .variant(ProgressBarVariant::Segmented),
                )
                .child(
                    progress_bar(0.9)
                        .label("Classic / 90%")
                        .variant(ProgressBarVariant::Classic),
                )
                .child(
                    loading_indicator()
                        .frame(state.ticks)
                        .label("Loading indicator"),
                )
                .child(
                    switch("Play animation")
                        .key("overview-play")
                        .checked(state.playing)
                        .on_change(Msg::Pause),
                ),
        )
        .child(
            card("03 / Navigation and overlays")
                .child(
                    menu()
                        .key("quick-menu")
                        .height(6)
                        .items([
                            MenuItem::new("content", "Read sample content"),
                            MenuItem::new("layout", "Explore layouts"),
                            MenuItem::new("motion", "Watch animations"),
                            MenuItem::new("unavailable", "Disabled menu item").disabled(true),
                        ])
                        .on_select(Msg::Page),
                )
                .child(
                    row()
                        .gap(1)
                        .child(button("Command palette").on_click(|| Msg::Open(Modal::Commands)))
                        .child(
                            button("Dialog")
                                .variant(ButtonVariant::Secondary)
                                .on_click(|| Msg::Open(Modal::Confirm)),
                        ),
                ),
        )
        .child(
            card("04 / Keyboard and mouse")
                .child(label("Tab / Shift+Tab    Move focus"))
                .child(label("Arrows            Navigate the focused widget"))
                .child(label("Enter / Space     Activate or select"))
                .child(label("F1 / F2 / F3      Commands / theme / dialog"))
                .child(label("Esc               Close popup, then exit"))
                .child(label("Mouse             Click, scroll, drag split"))
                .child(
                    label("Small terminal? Focus the page and scroll.").style(styles.text_muted),
                ),
        )
}

fn controls(state: &State) -> impl Widget<Msg> {
    two_by_two()
        .child(
            card("TextInput + TextArea / Unicode + autosave")
                .child(
                    text_input()
                        .key("name")
                        .value(&state.name)
                        .placeholder("Your name")
                        .on_change(Msg::Name),
                )
                .child(label(format!("Hello, {}", state.name)))
                .child(
                    textarea()
                        .key("notes")
                        .height(6)
                        .value(&state.notes)
                        .on_change(Msg::Notes),
                )
                .child(label(&state.save_status))
                .child(label("Debounce → background task → saved in memory")),
        )
        .child(
            card("Checkbox + Switch + Toggle + RadioGroup")
                .child(
                    checkbox("Remember workspace")
                        .key("remember")
                        .checked(state.remember)
                        .on_change(Msg::Remember),
                )
                .child(
                    switch("Notifications")
                        .key("notifications")
                        .checked(state.notifications)
                        .on_change(Msg::Notifications),
                )
                .child(
                    toggle("Compact mode")
                        .key("compact")
                        .checked(state.compact)
                        .on_change(Msg::Compact),
                )
                .child(checkbox("Disabled option").checked(true).disabled(true))
                .child(
                    radio_group()
                        .key("density")
                        .selected(&state.density)
                        .options([
                            RadioOption::new("compact", "Compact"),
                            RadioOption::new("comfortable", "Comfortable"),
                            RadioOption::new("spacious", "Spacious"),
                        ])
                        .on_change(Msg::Density),
                )
                .child(label(format!(
                    "{} / compact={}",
                    state.density, state.compact
                ))),
        )
        .child(
            card("Buttons + searchable Select")
                .gap(0)
                .child(
                    row()
                        .gap(1)
                        .child(button("+1").key("increment").on_click(|| Msg::Count(1)))
                        .child(
                            button("−1")
                                .variant(ButtonVariant::Secondary)
                                .on_click(|| Msg::Count(-1)),
                        )
                        .child(
                            button("Ghost")
                                .variant(ButtonVariant::Ghost)
                                .on_click(|| Msg::Count(1)),
                        )
                        .child(
                            button("Link")
                                .variant(ButtonVariant::Link)
                                .on_click(|| Msg::Open(Modal::Commands)),
                        ),
                )
                .child(
                    row()
                        .gap(1)
                        .child(
                            button("Confirm...")
                                .variant(ButtonVariant::Destructive)
                                .on_click(|| Msg::Open(Modal::Confirm)),
                        )
                        .child(button("Disabled").disabled(true))
                        .child(label(format!("Count: {}", state.count))),
                )
                .child(
                    select()
                        .key("priority")
                        .height(7)
                        .placeholder("Choose priority (type to filter)")
                        .searchable(true)
                        .search_mode(SelectSearchMode::Fuzzy)
                        .options([
                            SelectOption::new("low", "Low priority"),
                            SelectOption::new("normal", "Normal priority"),
                            SelectOption::new("high", "High priority"),
                            SelectOption::new("unavailable", "Unavailable").disabled(true),
                        ])
                        .on_change(Msg::Priority),
                )
                .child(label(format!("Priority: {}", state.priority))),
        )
        .child(
            card("Calendar / controlled date")
                .child(
                    calendar(state.date)
                        .key("calendar")
                        .on_change(Msg::Date)
                        .on_submit(Msg::DateSubmitted),
                )
                .child(label(format!(
                    "{:04}-{:02}-{:02}",
                    state.date.year, state.date.month, state.date.day
                )))
                .child(label("Arrows: day/week   PgUp/PgDn: month")),
        )
}

fn collections(state: &State) -> impl Widget<Msg> {
    grid()
        .columns("1fr 1fr 1fr")
        .rows("18 16")
        .gap(1)
        .child_at(
            card("DataTable / filtering, sorting, cells and multi-selection")
                .gap(0)
                .child(
                    text_input()
                        .key("filter")
                        .value(&state.filter)
                        .placeholder("Filter services...")
                        .on_change(Msg::Filter),
                )
                .child(
                    row()
                        .gap(2)
                        .child(
                            button(if state.sort_desc {
                                "Sort: Z → A"
                            } else {
                                "Sort: A → Z"
                            })
                            .key("sort")
                            .on_click(|| Msg::Sort),
                        )
                        .child(
                            checkbox("Owner column")
                                .checked(state.show_owner)
                                .on_change(Msg::Owner),
                        ),
                )
                .child(
                    data_table()
                        .key("services")
                        .height(8)
                        .navigation_mode(DataTableNavigationMode::Cell)
                        .multi_select(true)
                        .hidden_columns((!state.show_owner).then_some("owner"))
                        .filter_query_opt((!state.filter.is_empty()).then(|| state.filter.clone()))
                        .sort(DataTableSort::new(
                            "name",
                            if state.sort_desc {
                                DataTableSortDirection::Desc
                            } else {
                                DataTableSortDirection::Asc
                            },
                        ))
                        .columns([
                            DataTableColumn::new("Service")
                                .id("name")
                                .width(24)
                                .filterable(true)
                                .sortable(true),
                            DataTableColumn::new("Environment").id("env").width(18),
                            DataTableColumn::new("Latency")
                                .id("latency")
                                .width(12)
                                .align(TableAlign::Right),
                            DataTableColumn::new("Owner").id("owner").width(16),
                        ])
                        .rows([
                            DataTableRow::new(
                                "api",
                                ["api-gateway", "production", "24 ms", "platform"],
                            ),
                            DataTableRow::new("auth", ["auth", "canary", "12 ms", "identity"]),
                            DataTableRow::new(
                                "billing",
                                ["billing", "production", "41 ms", "finance"],
                            ),
                            DataTableRow::new(
                                "search",
                                ["search", "staging", "89 ms", "discovery"],
                            ),
                            DataTableRow::new("worker", ["worker", "production", "51 ms", "infra"]),
                            DataTableRow::new("mailer", ["mailer", "queue", "73 ms", "growth"]),
                        ])
                        .on_change(Msg::Inspect)
                        .on_submit(Msg::Inspect)
                        .on_selection_change(Msg::Selected),
                )
                .child(label(format!(
                    "{} / selected: {} (Space toggles a row)",
                    state.inspected,
                    state.selected.len()
                ))),
            GridPlacement::new().area_span(1, 1, 3, 1),
        )
        .child_at(
            card("Tree / expand with →").child(
                tree()
                    .key("component-tree")
                    .height(11)
                    .items([
                        TreeItem::new("controls", "Controls")
                            .child(TreeItem::new("button", "Button"))
                            .child(TreeItem::new("input", "TextInput")),
                        TreeItem::new("containers", "Containers")
                            .child(TreeItem::new("grid", "Grid"))
                            .child(TreeItem::new("split", "Split")),
                    ])
                    .on_change(Msg::Inspect)
                    .on_submit(Msg::Inspect),
            ),
            GridPlacement::new().area(1, 2),
        )
        .child_at(
            card("List / keyboard navigation").child(
                list()
                    .key("feature-list")
                    .height(11)
                    .items([
                        ListItem::new("unicode", "Unicode editing"),
                        ListItem::new("focus", "Focus management"),
                        ListItem::new("events", "Event routing"),
                        ListItem::new("tasks", "Background tasks"),
                        ListItem::new("effects", "Visual effects"),
                        ListItem::new("headless", "Headless runtime"),
                    ])
                    .on_change(Msg::Inspect)
                    .on_submit(Msg::Inspect),
            ),
            GridPlacement::new().area(2, 2),
        )
        .child_at(
            card("FileExplorer / sample files").child(
                file_explorer()
                    .key("sample-files")
                    .height(11)
                    .items([
                        FileExplorerItem::directory("src", "src")
                            .child(FileExplorerItem::file("src/main.rs", "main.rs"))
                            .child(FileExplorerItem::file("src/pages.rs", "pages.rs")),
                        FileExplorerItem::file("Cargo.toml", "Cargo.toml"),
                        FileExplorerItem::file("README.md", "README.md"),
                    ])
                    .on_change(Msg::Inspect)
                    .on_open(Msg::Inspect),
            ),
            GridPlacement::new().area(3, 2),
        )
}

fn content(state: &State) -> impl Widget<Msg> {
    two_by_two()
        .child(
            card("CodeViewer / Rust syntax highlighting").gap(0).child(
                code_viewer(CODE)
                    .key("source")
                    .language("rs")
                    .line_numbers(true)
                    .height(16)
                    .borderless(),
            ),
        )
        .child(
            card("MarkdownViewer / headings, lists and quotes")
                .gap(0)
                .child(
                    markdown_viewer(MARKDOWN)
                        .key("markdown")
                        .height(16)
                        .borderless(),
                ),
        )
        .child(
            card("DiffViewer / additions and removals")
                .gap(0)
                .child(diff_viewer(DIFF).key("diff").height(14).borderless()),
        )
        .child(
            card("LogViewer / live interaction history").gap(0).child(
                log_viewer()
                    .key("activity")
                    .height(14)
                    .borderless()
                    .lines(state.logs.clone()),
            ),
        )
}

fn layout(state: &State) -> impl Widget<Msg> {
    let styles = theme(state.theme).styles;
    two_by_two()
        .child(
            card("Grid / fractions and spanning cells").child(
                grid()
                    .columns("1fr 2fr 1fr")
                    .rows("3 3 3")
                    .gap(1)
                    .child_at(
                        label("1fr")
                            .bg(Color::Indexed(4))
                            .align(HorizontalAlign::Center),
                        GridPlacement::new().area(1, 1),
                    )
                    .child_at(
                        label("2fr")
                            .bg(Color::Indexed(5))
                            .align(HorizontalAlign::Center),
                        GridPlacement::new().area(2, 1),
                    )
                    .child_at(
                        label("1fr")
                            .bg(Color::Indexed(6))
                            .align(HorizontalAlign::Center),
                        GridPlacement::new().area(3, 1),
                    )
                    .child_at(
                        label("Spans three columns")
                            .style(styles.selected)
                            .align(HorizontalAlign::Center),
                        GridPlacement::new().area_span(1, 2, 3, 1),
                    )
                    .child_at(
                        row()
                            .gap(1)
                            .child(label("Flex row"))
                            .child(spacer())
                            .child(label("→")),
                        GridPlacement::new().area_span(1, 3, 3, 1),
                    ),
            ),
        )
        .child(
            card("Split / drag the divider or use arrows").child(
                split(
                    col()
                        .child(label("Sidebar").bold())
                        .child(label("Resizable")),
                    col()
                        .child(label("Main content").bold())
                        .child(label("Focus divider to resize.")),
                )
                .key("resizable-split")
                .sidebar(17)
                .min_first(10)
                .min_second(12)
                .resizable(true),
            ),
        )
        .child(
            card("ScrollView / nested viewport").child(
                scroll_lines((1..=50).map(|n| format!("{n:02}  Scroll me / 滚动内容 / line {n}")))
                    .key("nested-scroll")
                    .scrollbars(ScrollbarVisibility::Always),
            ),
        )
        .child(
            card("Collapsible + Stack")
                .child(
                    collapsible("Expandable details")
                        .key("details")
                        .expanded(state.expanded)
                        .on_toggle(Msg::Expanded)
                        .child(label("Focus skips hidden children."))
                        .child(button("Open dialog").on_click(|| Msg::Open(Modal::Confirm))),
                )
                .child(
                    stack()
                        .child(label(" Background layer ").style(styles.surface_header))
                        .child(
                            col()
                                .padding(Padding::new(1, 0, 0, 4))
                                .child(label(" Foreground layer ").style(styles.selected)),
                        ),
                )
                .child(divider().text("Overlay: press F3")),
        )
}

fn motion(state: &State) -> impl Widget<Msg> {
    let time = state.ticks as f64 * 0.08;
    let phase = (time % 3.0) / 3.0;
    two_by_two()
        .child(
            card("Canvas / animated braille signals")
                .child(
                    canvas(move |surface, ctx| {
                        let width = ctx.dot_width().max(1);
                        let height = ctx.dot_height().max(1) as f64;
                        for x in 0..width {
                            let x = x as f64;
                            surface.set(x, height * (0.5 + 0.32 * (x * 0.09 + time * 2.0).sin()));
                            surface.set(x, height * (0.5 + 0.22 * (x * 0.14 - time).cos()));
                        }
                    })
                    .key("wave-canvas")
                    .height(10),
                )
                .child(
                    switch("Play animation")
                        .key("motion-play")
                        .checked(state.playing)
                        .on_change(Msg::Pause),
                ),
        )
        .child(
            card("Visual / fade and moving color gradient")
                .child(
                    visual(label("A widget can fade into view.").bold())
                        .progress(phase)
                        .effect(VisualEffect::fade_in()),
                )
                .child(
                    visual(label("GRADIENT  /  RSILLE  /  TERMINAL").bold())
                        .progress(1.0)
                        .effect(
                            VisualEffect::gradient(
                                Color::Cyan,
                                Color::Magenta,
                                GradientDirection::Horizontal,
                            )
                            .phase(phase),
                        ),
                )
                .child(
                    visual(label("A reveal moves across this line."))
                        .progress(phase)
                        .effect(VisualEffect::reveal(WipeDirection::LeftToRight)),
                )
                .child(label("Effects wrap ordinary widget descriptions.")),
        )
        .child(
            card("Animated / layout transition")
                .child(
                    button("Resize the block")
                        .key("resize-block")
                        .on_click(|| Msg::Stretch),
                )
                .child(
                    animate(
                        label(" Animated width ")
                            .width(if state.stretched { 40 } else { 20 })
                            .height(3)
                            .bg(Color::Indexed(6)),
                    )
                    .key("animated-block")
                    .layout(AnimationSpec::normal()),
                )
                .child(label("Press repeatedly to retarget the transition.")),
        )
        .child(
            card("Progress + LoadingIndicator / timer-driven")
                .child(
                    loading_indicator()
                        .frame(state.ticks)
                        .label("Animated spinner"),
                )
                .child(
                    progress_bar(phase)
                        .label("Sample progress")
                        .variant(ProgressBarVariant::Segmented),
                )
                .child(label(format!("Animation ticks: {}", state.ticks)))
                .child(label("An 80 ms timer drives these sample visuals."))
                .child(label("The app and notes worker share one event loop.")),
        )
}

fn themes(state: &State) -> impl Widget<Msg> {
    let styles = theme(state.theme).styles;
    two_by_two()
        .child(
            card("Theme / five built-in palettes")
                .gap(0)
                .child(label(format!("Current: {}", THEMES[state.theme])).bold())
                .child(
                    button("Next theme [F2]")
                        .key("next-theme")
                        .on_click(|| Msg::Theme),
                )
                .child(label("Dark / Light / One Dark / Dracula / Tokyo Night"))
                .child(label("The same semantic styles apply across pages."))
                .child(label("Heading text").style(styles.text_heading))
                .child(label("Secondary text").style(styles.text_muted))
                .child(label("Selected item").style(styles.selected))
                .child(label("Focused selection").style(styles.selected_focused)),
        )
        .child(
            card("Status / semantic colors")
                .child(label(" INFO     Sample information ").style(styles.status_info))
                .child(label(" SUCCESS  Notes saved ").style(styles.status_success))
                .child(label(" WARNING  Sample warning ").style(styles.status_warning))
                .child(label(" ERROR    Sample error ").style(styles.status_error))
                .child(label("Validation feedback").style(styles.validation_error))
                .child(label("Disabled content").style(styles.disabled)),
        )
        .child(
            card("Typography / Unicode and wrapping")
                .child(label("Bold / emphasis").bold())
                .child(label("Italic / secondary emphasis").italic())
                .child(label("Underline / annotation").underline())
                .child(label("中文  日本語  e\u{301}  👩🏽‍💻  👨‍👩‍👧‍👦"))
                .child(
                    label(
                        "Word wrapping respects terminal columns and Unicode grapheme boundaries.",
                    )
                    .wrap_word(),
                ),
        )
        .child(
            card("Surfaces / borders and elevation")
                .child(
                    row()
                        .gap(1)
                        .child(
                            panel()
                                .title("Single")
                                .border(BorderStyle::Single)
                                .style(styles.surface)
                                .child(label(" Surface ")),
                        )
                        .child(
                            panel()
                                .title("Rounded")
                                .border(BorderStyle::Rounded)
                                .style(styles.surface_elevated)
                                .child(label(" Elevated ")),
                        ),
                )
                .child(
                    button("Preview a themed modal [F3]").on_click(|| Msg::Open(Modal::Confirm)),
                ),
        )
}
