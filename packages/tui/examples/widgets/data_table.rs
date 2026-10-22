//! DataTable widget with internal row navigation.
//!
//! Run with: `cargo run -p tui --example data_table`
//! Use Up/Down/Home/End/PageUp/PageDown to move between rows. Press Enter to submit.

use tui::prelude::*;

#[derive(Debug, Clone)]
enum Msg {
    FilterChanged(String),
    ToggleSort,
    ToggleOwner,
    Highlighted(String),
    CellFocused(String, String),
    Opened(String),
    Selected(Vec<String>),
}

#[derive(Debug)]
struct State {
    filter: String,
    sort_desc: bool,
    show_owner: bool,
    active: String,
    cell: String,
    opened: String,
    selected: Vec<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            filter: String::new(),
            sort_desc: false,
            show_owner: true,
            active: String::new(),
            cell: String::new(),
            opened: String::new(),
            selected: Vec::new(),
        }
    }
}

fn main() -> WidgetResult<()> {
    App::new(State::default())
        .mode(Mode::Inline)
        .update(|state, msg, _ctx| update(state, msg))
        .view(view)
        .run()
        .map(|_| ())
}

fn update(state: &mut State, msg: Msg) {
    match msg {
        Msg::FilterChanged(query) => state.filter = query,
        Msg::ToggleSort => state.sort_desc = !state.sort_desc,
        Msg::ToggleOwner => state.show_owner = !state.show_owner,
        Msg::Highlighted(id) => state.active = id,
        Msg::CellFocused(row, column) => state.cell = format!("{row}/{column}"),
        Msg::Opened(id) => state.opened = id,
        Msg::Selected(ids) => state.selected = ids,
    }
}

fn view(state: &State) -> impl Widget<Msg> {
    let sort = DataTableSort::new(
        "service",
        if state.sort_desc {
            DataTableSortDirection::Desc
        } else {
            DataTableSortDirection::Asc
        },
    );

    row::<Msg>()
        .padding(Padding::uniform(1))
        .gap(2)
        .child(
            col::<Msg>()
                .gap(1)
                .child(
                    text_input::<Msg>()
                        .key("table-filter")
                        .value(state.filter.clone())
                        .placeholder("Filter by service name")
                        .on_change(Msg::FilterChanged),
                )
                .child(
                    data_table::<Msg>()
                        .key("deployments")
                        .height(10)
                        .navigation_mode(DataTableNavigationMode::Cell)
                        .multi_select(true)
                        .hidden_columns((!state.show_owner).then_some("owner"))
                        .filter_query_opt(
                            (!state.filter.trim().is_empty()).then(|| state.filter.clone()),
                        )
                        .sort(sort)
                        .columns([
                            DataTableColumn::new("Service")
                                .id("service")
                                .width(18)
                                .sortable(true)
                                .filterable(true),
                            DataTableColumn::new("Stage").id("stage").width(10),
                            DataTableColumn::new("Latency")
                                .id("latency")
                                .width(10)
                                .align(TableAlign::Right),
                            DataTableColumn::new("Owner").id("owner").width(12),
                        ])
                        .rows([
                            DataTableRow::new(
                                "api-prod",
                                ["api-gateway", "prod", "34 ms", "platform"],
                            ),
                            DataTableRow::new(
                                "worker-prod",
                                ["job-worker", "prod", "51 ms", "ops"],
                            ),
                            DataTableRow::new(
                                "search-staging",
                                ["search", "staging", "89 ms", "search"],
                            ),
                            DataTableRow::new(
                                "billing-prod",
                                ["billing", "prod", "41 ms", "finance"],
                            ),
                            DataTableRow::new(
                                "auth-canary",
                                ["auth", "canary", "27 ms", "identity"],
                            ),
                            DataTableRow::new(
                                "email-queue",
                                ["mailer", "queue", "73 ms", "growth"],
                            ),
                        ])
                        .on_change(Msg::Highlighted)
                        .on_cell_change(Msg::CellFocused)
                        .on_selection_change(Msg::Selected)
                        .on_submit(Msg::Opened),
                ),
        )
        .child(
            col::<Msg>()
                .border(BorderStyle::Rounded)
                .padding(Padding::uniform(1))
                .gap(1)
                .child(label("DataTable Example").bold())
                .child(label("Tab can move focus between filter input and table."))
                .child(label("The filter uses filterable columns only."))
                .child(label("Use the button to flip service sorting."))
                .child(
                    button(if state.sort_desc {
                        "Sort service ascending"
                    } else {
                        "Sort service descending"
                    })
                    .variant(ButtonVariant::Secondary)
                    .on_click(|| Msg::ToggleSort),
                )
                .child(
                    button(if state.show_owner {
                        "Hide owner column"
                    } else {
                        "Show owner column"
                    })
                    .variant(ButtonVariant::Secondary)
                    .on_click(|| Msg::ToggleOwner),
                )
                .child(divider().text("Selection"))
                .child(label(format!(
                    "Active row: {}",
                    if state.active.is_empty() {
                        "none"
                    } else {
                        &state.active
                    }
                )))
                .child(label(format!(
                    "Active cell: {}",
                    if state.cell.is_empty() {
                        "none"
                    } else {
                        &state.cell
                    }
                )))
                .child(label(format!(
                    "Submitted row: {}",
                    if state.opened.is_empty() {
                        "none"
                    } else {
                        &state.opened
                    }
                )))
                .child(label(format!("Selected rows: {}", state.selected.len())))
                .child(
                    button("Inspect selection")
                        .variant(ButtonVariant::Secondary)
                        .on_click(|| Msg::Opened("manual-inspect".to_owned())),
                ),
        )
}
