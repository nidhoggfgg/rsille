//! Theme showcase with runtime switching.
//!
//! Run with: `cargo run -p tui --example theme`
//! Use Left/Right or 1-5 to switch themes. Tab focuses the interactive controls. Esc to quit.

use tui::prelude::*;

const THEME_COUNT: usize = 5;

#[derive(Debug, Default)]
struct State {
    active_theme: usize,
    query: String,
    submitted: String,
}

#[derive(Debug, Clone)]
enum Msg {
    PrevTheme,
    NextTheme,
    SetTheme(usize),
    QueryChanged(String),
    QuerySubmitted(String),
    LoadSample,
    ClearQuery,
}

fn main() -> WidgetResult<()> {
    App::new(State::default())
        .with_theme_from(|state| theme_for(state.active_theme))
        .on_key(KeyCode::Left, || Msg::PrevTheme)
        .on_key(KeyCode::Right, || Msg::NextTheme)
        .on_key(KeyCode::Char('1'), || Msg::SetTheme(0))
        .on_key(KeyCode::Char('2'), || Msg::SetTheme(1))
        .on_key(KeyCode::Char('3'), || Msg::SetTheme(2))
        .on_key(KeyCode::Char('4'), || Msg::SetTheme(3))
        .on_key(KeyCode::Char('5'), || Msg::SetTheme(4))
        .mode(Mode::Inline)
        .update(|state, msg, _ctx| update(state, msg))
        .view(view)
        .run()
        .map(|_| ())
}

fn update(state: &mut State, msg: Msg) {
    match msg {
        Msg::PrevTheme => state.active_theme = cycle_theme(state.active_theme, -1),
        Msg::NextTheme => state.active_theme = cycle_theme(state.active_theme, 1),
        Msg::SetTheme(index) => state.active_theme = index.min(THEME_COUNT - 1),
        Msg::QueryChanged(value) => state.query = value,
        Msg::QuerySubmitted(value) => state.submitted = value,
        Msg::LoadSample => {
            state.query = format!("{} theme tokens", theme_title(state.active_theme));
        }
        Msg::ClearQuery => {
            state.query.clear();
            state.submitted.clear();
        }
    }
}

fn view(state: &State) -> impl Widget<Msg> {
    let theme = theme_for(state.active_theme);
    let card_style = theme.styles.border.merge(theme.styles.surface_elevated);

    col::<Msg>()
        .padding(Padding::uniform(1))
        .gap(1)
        .child(
            col::<Msg>()
                .border(BorderStyle::Rounded)
                .padding(Padding::uniform(1))
                .style(card_style)
                .gap(1)
                .child(label("Theme Showcase").style(theme.styles.text_heading))
                .child(label(format!(
                    "Current theme: {} ({})",
                    theme_title(state.active_theme),
                    theme.name
                )))
                .child(
                    label("Press Left/Right or 1-5 to switch themes. Tab moves into the input and action buttons.")
                        .style(theme.styles.text_muted),
                ),
        )
        .child(theme_picker_row(state.active_theme))
        .child(
            col::<Msg>()
                .border(BorderStyle::Rounded)
                .padding(Padding::uniform(1))
                .style(card_style)
                .gap(1)
                .child(label("Semantic Styles").style(theme.styles.text_heading))
                .child(
                    label(theme_description(state.active_theme)).style(theme.styles.text_muted),
                )
                .child(
                    row::<Msg>()
                        .gap(1)
                        .child(swatch(" primary ", theme.styles.primary_action, card_style))
                        .child(swatch(" secondary ", theme.styles.secondary_action, card_style))
                        .child(swatch(" danger ", theme.styles.destructive_action, card_style))
                        .child(swatch(" selected ", theme.styles.selected, card_style))
                        .child(swatch(" focused ", theme.styles.selected_focused, card_style)),
                )
                .child(
                    row::<Msg>()
                        .gap(1)
                        .child(swatch(" interactive ", theme.styles.interactive, card_style))
                        .child(swatch(" active ", theme.styles.list_active, card_style))
                        .child(swatch(" header ", theme.styles.surface_header, card_style))
                        .child(swatch(" border ", theme.styles.border, card_style))
                        .child(swatch(" focus ring ", theme.styles.border_focused, card_style)),
                )
                .child(
                    row::<Msg>()
                        .gap(1)
                        .child(swatch(" info ", theme.styles.status_info, card_style))
                        .child(swatch(" success ", theme.styles.status_success, card_style))
                        .child(swatch(" warning ", theme.styles.status_warning, card_style))
                        .child(swatch(" error ", theme.styles.status_error, card_style))
                        .child(swatch(" invalid ", theme.styles.validation_error, card_style)),
                )
                .child(
                    row::<Msg>()
                        .gap(1)
                        .child(swatch(" modal ", theme.styles.surface_modal, card_style))
                        .child(swatch(" popup ", theme.styles.surface_popup, card_style))
                        .child(swatch(" tooltip ", theme.styles.surface_tooltip, card_style))
                        .child(swatch(" menu active ", theme.styles.menu_item_active_focused, card_style))
                        .child(swatch(" scrollbar ", theme.styles.scrollbar_thumb, card_style)),
                ),
        )
        .child(
            col::<Msg>()
                .border(BorderStyle::Rounded)
                .padding(Padding::uniform(1))
                .style(card_style)
                .gap(1)
                .child(label("Interactive Preview").style(theme.styles.text_heading))
                .child(
                    text_input::<Msg>()
                        .key("theme-query")
                        .value(state.query.as_str())
                        .placeholder("Type and submit to see the active theme on inputs...")
                        .on_change(Msg::QueryChanged)
                        .on_submit(Msg::QuerySubmitted),
                )
                .child(
                    row::<Msg>()
                        .gap(1)
                        .child(button("Previous").variant(ButtonVariant::Ghost).on_click(|| Msg::PrevTheme))
                        .child(button("Next").on_click(|| Msg::NextTheme))
                        .child(
                            button("Load Sample")
                                .variant(ButtonVariant::Secondary)
                                .on_click(|| Msg::LoadSample),
                        )
                        .child(
                            button("Clear")
                                .variant(ButtonVariant::Destructive)
                                .on_click(|| Msg::ClearQuery),
                        ),
                )
                .child(label(format!(
                    "Submitted text: {}",
                    if state.submitted.is_empty() {
                        "nothing yet"
                    } else {
                        state.submitted.as_str()
                    }
                ))),
        )
}

fn theme_picker_row(active_theme: usize) -> Flex<Msg> {
    row::<Msg>()
        .gap(1)
        .child(theme_button("1 Dark", 0, active_theme))
        .child(theme_button("2 Light", 1, active_theme))
        .child(theme_button("3 One Dark", 2, active_theme))
        .child(theme_button("4 Dracula", 3, active_theme))
        .child(theme_button("5 Tokyo Night", 4, active_theme))
}

fn theme_button(label_text: &'static str, index: usize, active_theme: usize) -> Button<Msg> {
    let variant = if index == active_theme {
        ButtonVariant::Primary
    } else {
        ButtonVariant::Secondary
    };

    button(label_text)
        .variant(variant)
        .on_click(move || Msg::SetTheme(index))
}

fn swatch(label_text: &'static str, style: Style, base: Style) -> Label<Msg> {
    // Some semantic styles only define part of the final appearance, such as
    // a border/focus color with no background. Merge them over a stable card
    // surface so the preview chip always renders as a complete block.
    label(label_text).style(style.merge(base).bold())
}

fn cycle_theme(active_theme: usize, delta: isize) -> usize {
    match delta {
        -1 => {
            if active_theme == 0 {
                THEME_COUNT - 1
            } else {
                active_theme - 1
            }
        }
        1 => (active_theme + 1) % THEME_COUNT,
        _ => active_theme,
    }
}

fn theme_title(index: usize) -> &'static str {
    match index.min(THEME_COUNT - 1) {
        0 => "Dark",
        1 => "Light",
        2 => "One Dark",
        3 => "Dracula",
        _ => "Tokyo Night",
    }
}

fn theme_description(index: usize) -> &'static str {
    match index.min(THEME_COUNT - 1) {
        0 => "Built-in dark palette with stronger selected and focused states.",
        1 => "Built-in light palette for bright terminal sessions.",
        2 => "Atom's One Dark palette adapted for terminal contrast.",
        3 => "Dracula's high-chroma palette with clear focus affordances.",
        _ => "Tokyo Night colors tuned for calm, readable terminal UIs.",
    }
}

fn theme_for(index: usize) -> Theme {
    match index.min(THEME_COUNT - 1) {
        0 => Theme::dark(),
        1 => Theme::light(),
        2 => Theme::one_dark(),
        3 => Theme::dracula(),
        _ => Theme::tokyo_night(),
    }
}
