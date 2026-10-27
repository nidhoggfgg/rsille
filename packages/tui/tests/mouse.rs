use std::fmt::Debug;
use tui::prelude::*;
use tui::widgets::{ContentViewerState, TextInputState};

fn mouse(kind: MouseEventKind, x: u16, y: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column: x,
        row: y,
        modifiers: KeyModifiers::empty(),
    })
}
fn click(x: u16, y: u16) -> Event {
    mouse(MouseEventKind::Down(MouseButton::Left), x, y)
}
fn drag(x: u16, y: u16) -> Event {
    mouse(MouseEventKind::Drag(MouseButton::Left), x, y)
}
fn up(x: u16, y: u16) -> Event {
    mouse(MouseEventKind::Up(MouseButton::Left), x, y)
}
fn wheel(down: bool, x: u16, y: u16) -> Event {
    mouse(
        if down {
            MouseEventKind::ScrollDown
        } else {
            MouseEventKind::ScrollUp
        },
        x,
        y,
    )
}
fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::empty()))
}
fn runtime<M, W>(
    view: impl Fn(&Vec<M>) -> W + 'static,
    width: u16,
    height: u16,
) -> Runtime<Vec<M>, M>
where
    M: Clone + Debug + Send + 'static,
    W: Widget<M> + 'static,
{
    App::new(Vec::new())
        .with_motion_policy(MotionPolicy::disabled())
        .build(|messages, message| messages.push(message), view)
        .size(width, height)
}
fn row<S, M: Clone + Debug + Send + 'static>(app: &Runtime<S, M>, y: u16) -> String {
    let width = app.frame().size().width as usize;
    app.frame().content()[y as usize * width..(y as usize + 1) * width]
        .iter()
        .filter(|cell| !cell.is_occupied)
        .map(|cell| cell.content.symbol().into_owned())
        .collect()
}

#[test]
fn collapsible_only_toggles_from_its_title_and_respects_disabled() {
    for disabled in [false, true] {
        let mut app = runtime(
            move |_| {
                collapsible("Section")
                    .disabled(disabled)
                    .expanded(true)
                    .child(button("child").on_click(|| false))
                    .on_toggle(|value| value)
            },
            25,
            6,
        );
        app.step(&[click(1, 0)]).unwrap();
        assert_eq!(app.state().len(), usize::from(!disabled));
        app.step(&[click(1, 5)]).unwrap();
        assert_eq!(app.state().len(), usize::from(!disabled));
    }
}

#[test]
fn calendar_clicks_grid_cells_and_wheel_clamps_month_end() {
    // February 1, 2026 is Sunday; the border puts the first date at (1,3).
    let mut app = runtime(
        |_| calendar(CalendarDate::new(2026, 2, 28).unwrap()).on_change(|date| date),
        24,
        10,
    );
    app.step(&[click(4, 3)]).unwrap();
    assert_eq!(app.state(), &[CalendarDate::new(2026, 2, 2).unwrap()]);
    app.step(&[click(3, 3), click(0, 3), click(4, 2)]).unwrap();
    assert_eq!(
        app.state().len(),
        1,
        "spacing, border and weekday headings are inert"
    );
    app.step(&[wheel(true, 5, 5)]).unwrap();
    assert_eq!(
        app.state().last(),
        Some(&CalendarDate::new(2026, 3, 2).unwrap())
    );
    let mut app = runtime(
        |_| calendar(CalendarDate::new(2026, 1, 31).unwrap()).on_change(|date| date),
        24,
        10,
    );
    app.step(&[wheel(true, 5, 5), wheel(false, 5, 5)]).unwrap();
    assert_eq!(
        app.state(),
        &[
            CalendarDate::new(2026, 2, 28).unwrap(),
            CalendarDate::new(2026, 1, 28).unwrap()
        ]
    );
}

#[test]
fn tree_clicks_expand_and_open_leaves_and_skip_disabled_rows() {
    let mut app = runtime(
        |_| {
            tree()
                .height(6)
                .items([
                    TreeItem::new("branch", "Branch").child(TreeItem::new("leaf", "Leaf")),
                    TreeItem::new("disabled", "Disabled").disabled(true),
                ])
                .on_submit(|id| id)
        },
        24,
        6,
    );
    app.step(&[click(1, 1)]).unwrap();
    assert!(app
        .widget_state::<TreeState>(&WidgetId::root())
        .unwrap()
        .expanded_items
        .contains("branch"));
    app.step(&[click(5, 2)]).unwrap();
    assert_eq!(app.state(), &["leaf"]);
    app.step(&[click(5, 3), click(0, 2)]).unwrap();
    assert_eq!(app.state(), &["leaf"]);
    assert_eq!(
        app.widget_state::<TreeState>(&WidgetId::root())
            .unwrap()
            .active_item
            .as_deref(),
        Some("leaf")
    );
}

#[test]
fn multiselect_tree_expansion_handle_is_separate_from_selection() {
    let mut app = runtime(
        |_| {
            tree()
                .height(6)
                .multi_select(true)
                .item(TreeItem::new("branch", "Branch").child(TreeItem::new("leaf", "Leaf")))
                .on_selection_change(|ids| ids)
        },
        24,
        6,
    );
    app.step(&[click(1, 1)]).unwrap();
    let state = app.widget_state::<TreeState>(&WidgetId::root()).unwrap();
    assert!(state.expanded_items.contains("branch"));
    assert!(state.selection.selected.is_empty());
    app.step(&[click(6, 2)]).unwrap();
    assert_eq!(app.state(), &[vec!["leaf".to_owned()]]);
}

#[test]
fn explorer_clicks_open_files_and_load_lazy_directories_once_per_expansion() {
    let mut app = runtime(
        |_| {
            file_explorer()
                .height(7)
                .items([
                    FileExplorerItem::lazy_directory("lazy", "Lazy"),
                    FileExplorerItem::directory("dir", "Dir")
                        .child(FileExplorerItem::file("nested", "Nested")),
                    FileExplorerItem::file("file", "File"),
                    FileExplorerItem::file("disabled", "Disabled").disabled(true),
                ])
                .on_load_children(|id| format!("load:{id}"))
                .on_open(|id| format!("open:{id}"))
        },
        24,
        7,
    );
    app.step(&[click(1, 1), click(1, 1)]).unwrap();
    assert_eq!(app.state(), &["load:lazy"]);
    assert!(!app
        .widget_state::<FileExplorerState>(&WidgetId::root())
        .unwrap()
        .expanded_items
        .contains("lazy"));
    app.step(&[click(1, 2)]).unwrap();
    app.step(&[click(6, 3)]).unwrap();
    app.step(&[click(6, 4), click(6, 5)]).unwrap();
    assert_eq!(app.state(), &["load:lazy", "open:nested", "open:file"]);
}

#[test]
fn collection_wheel_scrolls_actual_window_and_click_uses_new_offset() {
    for kind in 0..4 {
        let mut app = runtime(
            move |_| -> Box<dyn Widget<String>> {
                match kind {
                    0 => Box::new(
                        list()
                            .height(6)
                            .items((0..20).map(|i| ListItem::new(i.to_string(), format!("row{i}"))))
                            .on_change(|id| id),
                    ),
                    1 => Box::new(
                        tree()
                            .height(6)
                            .items((0..20).map(|i| TreeItem::new(i.to_string(), format!("row{i}"))))
                            .on_change(|id| id),
                    ),
                    2 => {
                        Box::new(
                            file_explorer()
                                .height(6)
                                .items((0..20).map(|i| {
                                    FileExplorerItem::file(i.to_string(), format!("row{i}"))
                                }))
                                .on_change(|id| id),
                        )
                    }
                    _ => Box::new(
                        menu()
                            .height(6)
                            .items((0..20).map(|i| MenuItem::new(i.to_string(), format!("row{i}"))))
                            .on_select(|id| id),
                    ),
                }
            },
            24,
            6,
        );
        app.step(&[wheel(true, 5, 2)]).unwrap();
        assert!(
            row(&app, 1).contains("row3"),
            "kind={kind}: {}",
            row(&app, 1)
        );
        if kind == 3 {
            assert!(app.state().is_empty(), "wheel must not activate menu items");
        }
        app.step(&[click(6, 2)]).unwrap();
        assert_eq!(
            app.state().last().map(String::as_str),
            Some("4"),
            "kind={kind}"
        );
        app.step(&[wheel(false, 5, 2)]).unwrap();
        assert!(row(&app, 1).contains("row0"));
    }
}

#[test]
fn select_wheel_keeps_dropdown_open_and_commits_only_on_click() {
    let mut app = runtime(
        |_| {
            select()
                .height(8)
                .options((0..20).map(|i| SelectOption::new(i.to_string(), format!("option{i}"))))
                .on_change(|id| id)
        },
        30,
        8,
    );
    app.step(&[wheel(true, 5, 1)]).unwrap();
    assert!(
        !app.widget_state::<SelectState>(&WidgetId::root())
            .unwrap()
            .is_open
    );
    app.step(&[click(5, 1)]).unwrap();
    app.step(&[wheel(true, 5, 4)]).unwrap();
    let state = app.widget_state::<SelectState>(&WidgetId::root()).unwrap();
    assert!(state.is_open);
    assert_eq!(state.scroll_offset, 3);
    assert!(app.state().is_empty());
    app.step(&[click(5, 4)]).unwrap();
    assert_eq!(app.state(), &["4"]);
    assert!(
        !app.widget_state::<SelectState>(&WidgetId::root())
            .unwrap()
            .is_open
    );
}

#[test]
fn palette_click_respects_title_search_filter_and_scroll_offset() {
    let mut app = runtime(
        |_| {
            command_palette()
                .title("Commands")
                .height(9)
                .items((0..20).map(|i| CommandItem::new(i.to_string(), format!("run{i}"))))
                .on_submit(|id| id)
        },
        30,
        9,
    );
    app.step(&[click(5, 1), click(5, 2), click(0, 4)]).unwrap();
    assert!(app.state().is_empty());
    app.step(&[wheel(true, 5, 5)]).unwrap();
    assert!(row(&app, 4).contains("run3"));
    app.step(&[click(5, 5)]).unwrap();
    assert_eq!(app.state(), &["4"]);
    app.step(&[key(KeyCode::Char('1'))]).unwrap();
    app.step(&[click(5, 4)]).unwrap();
    assert_eq!(app.state().last().map(String::as_str), Some("1"));
}

#[test]
fn table_clicks_filtered_sorted_cells_and_wheel_preserves_selection() {
    let mut app = runtime(
        |_| {
            data_table()
                .height(8)
                .navigation_mode(DataTableNavigationMode::Cell)
                .columns([
                    DataTableColumn::new("Name").width(8).filterable(true),
                    DataTableColumn::new("Value").width(8),
                ])
                .rows((0..20).map(|i| {
                    DataTableRow::new(i.to_string(), [format!("row{i:02}"), format!("value{i}")])
                }))
                .sort(DataTableSort::new("name", DataTableSortDirection::Desc))
                .on_cell_change(|row, column| format!("{row}:{column}"))
        },
        30,
        8,
    );
    app.step(&[click(12, 4)]).unwrap();
    assert_eq!(app.state(), &["18:value"]);
    app.step(&[wheel(true, 5, 4)]).unwrap();
    let state = app
        .widget_state::<DataTableState>(&WidgetId::root())
        .unwrap();
    assert_eq!(state.scroll_offset, 3);
    assert_eq!(state.selection.selected, ["18"]);
    assert!(row(&app, 3).contains("row16"));
    app.step(&[click(12, 4)]).unwrap();
    assert_eq!(app.state().last().map(String::as_str), Some("15:value"));
}

#[test]
fn table_column_resize_captures_pointer_and_stops_on_release() {
    let mut app = runtime(
        |_| {
            data_table::<()>()
                .height(6)
                .columns([
                    DataTableColumn::new("First").width(8),
                    DataTableColumn::new("Second").width(8),
                ])
                .row(DataTableRow::new("row", ["first", "second"]))
        },
        40,
        6,
    );
    app.step(&[click(9, 1), drag(13, 10), up(13, 10)]).unwrap();
    let state = app
        .widget_state::<DataTableState>(&WidgetId::root())
        .unwrap();
    assert_eq!(state.column_widths, [("first".to_owned(), 12)]);
    assert!(state.resizing_column.is_none());
    app.step(&[drag(20, 1)]).unwrap();
    assert_eq!(
        app.widget_state::<DataTableState>(&WidgetId::root())
            .unwrap()
            .column_widths[0]
            .1,
        12
    );
}

#[test]
fn all_content_viewers_wheel_and_end_reach_the_last_line() {
    for kind in 0..4 {
        let mut app = runtime(
            move |_| -> Box<dyn Widget<()>> {
                let text = (0..20)
                    .map(|i| format!("line{i}"))
                    .collect::<Vec<_>>()
                    .join("\n");
                match kind {
                    0 => Box::new(code_viewer(text).height(6)),
                    1 => Box::new(log_viewer().lines(text.lines()).height(6)),
                    2 => Box::new(diff_viewer(text).height(6)),
                    _ => Box::new(markdown_viewer(text).height(6)),
                }
            },
            30,
            6,
        );
        app.step(&[wheel(true, 5, 2)]).unwrap();
        assert_eq!(
            app.widget_state::<ContentViewerState>(&WidgetId::root())
                .unwrap()
                .scroll_offset,
            3
        );
        assert!(row(&app, 1).contains("line3"));
        app.step(&[key(KeyCode::End)]).unwrap();
        assert_eq!(
            app.widget_state::<ContentViewerState>(&WidgetId::root())
                .unwrap()
                .scroll_offset,
            16
        );
        assert!(row(&app, 4).contains("line19"), "kind={kind}");
        app.step(&[wheel(true, 5, 2), wheel(false, 5, 2)]).unwrap();
        assert_eq!(
            app.widget_state::<ContentViewerState>(&WidgetId::root())
                .unwrap()
                .scroll_offset,
            13
        );
    }
}

#[test]
fn text_editors_drag_whole_graphemes_and_replace_selection() {
    for multiline in [false, true] {
        let mut app = App::new("a中👩🏽‍💻z".to_owned())
            .with_motion_policy(MotionPolicy::disabled())
            .build(
                |s, text| *s = text,
                move |s| -> Box<dyn Widget<String>> {
                    if multiline {
                        Box::new(textarea().value(s).on_change(|s| s))
                    } else {
                        Box::new(text_input().value(s).on_change(|s| s))
                    }
                },
            )
            .size(25, 5);
        let start = if multiline { 2 } else { 3 };
        app.step(&[click(start, 1), drag(start + 4, 1), up(start + 4, 1)])
            .unwrap();
        let (cursor, anchor, dragging) = if multiline {
            let s = app
                .widget_state::<TextAreaState>(&WidgetId::root())
                .unwrap();
            (s.cursor_position, s.selection_anchor, s.dragging)
        } else {
            let s = app
                .widget_state::<TextInputState>(&WidgetId::root())
                .unwrap();
            (s.cursor_position, s.selection_anchor, s.dragging)
        };
        assert_eq!(cursor, "a中👩🏽‍💻".len());
        assert_eq!(anchor, Some(1));
        assert!(!dragging);
        app.step(&[key(KeyCode::Char('X'))]).unwrap();
        assert_eq!(app.state(), "aXz");
    }
}

#[test]
fn textarea_wheel_preserves_caret_and_selection_then_clicks_visible_line() {
    let text = (0..20)
        .map(|i| format!("line{i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut app = App::new(text)
        .with_motion_policy(MotionPolicy::disabled())
        .build(
            |s, text| *s = text,
            |s| textarea().height(6).value(s).on_change(|s| s),
        )
        .size(25, 6);
    app.step(&[wheel(true, 5, 2)]).unwrap();
    let state = app
        .widget_state::<TextAreaState>(&WidgetId::root())
        .unwrap();
    assert_eq!(state.cursor_position, 0);
    assert_eq!(state.scroll_offset, 3);
    assert!(row(&app, 1).contains("line3"));
    app.step(&[click(1, 2)]).unwrap();
    assert_eq!(
        app.widget_state::<TextAreaState>(&WidgetId::root())
            .unwrap()
            .cursor_position,
        "line0\nline1\nline2\nline3\n".len()
    );
}

#[test]
fn editor_drag_outside_bounds_autoscrolls_and_shift_click_extends_selection() {
    let mut app = App::new("abcdefghijklmnopqrstuvwx".to_owned())
        .with_motion_policy(MotionPolicy::disabled())
        .build(
            |s, text| *s = text,
            |s| text_input().value(s).on_change(|s| s),
        )
        .size(12, 3);
    app.step(&[click(3, 1), drag(20, 8), up(20, 8)]).unwrap();
    let state = app
        .widget_state::<TextInputState>(&WidgetId::root())
        .unwrap();
    assert_eq!(state.selection_anchor, Some(1));
    assert!(state.horizontal_offset > 0);
    let mut event = click(5, 1);
    if let Event::Mouse(mouse) = &mut event {
        mouse.modifiers = KeyModifiers::SHIFT;
    }
    app.step(&[event]).unwrap();
    assert_eq!(
        app.widget_state::<TextInputState>(&WidgetId::root())
            .unwrap()
            .selection_anchor,
        Some(1)
    );
}

#[test]
fn scrollbars_page_on_track_and_drag_both_axes_outside_bounds() {
    for horizontal in [false, true] {
        let mut app = runtime(
            move |_| {
                scroll_view(label::<()>("content"))
                    .both()
                    .content_width(100)
                    .content_height(100)
                    .scrollbars(ScrollbarVisibility::Always)
            },
            12,
            7,
        );
        app.step(&[]).unwrap();
        let (x, y) = if horizontal { (10, 6) } else { (11, 5) };
        app.step(&[click(x, y), up(x, y)]).unwrap();
        let state = app.widget_state::<ScrollState>(&WidgetId::root()).unwrap();
        assert_eq!(
            if horizontal {
                state.offset_x
            } else {
                state.offset_y
            },
            if horizontal { 11 } else { 6 }
        );
        // Return to the start, grab the thumb, then move outside the allocation.
        app.step(&[key(KeyCode::Home)]).unwrap();
        let (x, y) = if horizontal { (0, 6) } else { (11, 0) };
        app.step(&[click(x, y), drag(30, 20), up(30, 20)]).unwrap();
        let state = app.widget_state::<ScrollState>(&WidgetId::root()).unwrap();
        assert_eq!(
            if horizontal {
                state.offset_x
            } else {
                state.offset_y
            },
            if horizontal { 89 } else { 94 }
        );
        assert!(state.dragging.is_none());
    }
}

#[test]
fn inner_viewer_at_wheel_boundary_scrolls_outer_viewport() {
    let mut app = runtime(
        |_| {
            scroll_view(
                col()
                    .child(code_viewer::<()>("a\nb").height(4))
                    .child(label("tail\ntail\ntail\ntail")),
            )
            .content_height(8)
            .scrollbars(ScrollbarVisibility::Hidden)
        },
        30,
        5,
    );
    app.step(&[wheel(true, 5, 2)]).unwrap();
    assert!(
        app.widget_state::<ScrollState>(&WidgetId::root())
            .unwrap()
            .offset_y
            > 0
    );
}

#[test]
fn disabled_widgets_ignore_click_wheel_and_drag_actions() {
    for kind in 0..8 {
        let mut app = runtime(
            move |_| -> Box<dyn Widget<String>> {
                match kind {
                    0 => Box::new(
                        tree()
                            .disabled(true)
                            .item(TreeItem::new("a", "A"))
                            .on_change(|s| s)
                            .on_submit(|s| s),
                    ),
                    1 => Box::new(
                        file_explorer()
                            .disabled(true)
                            .item(FileExplorerItem::file("a", "A"))
                            .on_open(|s| s),
                    ),
                    2 => Box::new(
                        data_table()
                            .disabled(true)
                            .column(DataTableColumn::new("Name"))
                            .row(DataTableRow::new("a", ["A"]))
                            .on_change(|s| s),
                    ),
                    3 => Box::new(
                        command_palette()
                            .disabled(true)
                            .item(CommandItem::new("a", "A"))
                            .on_submit(|s| s),
                    ),
                    4 => Box::new(
                        calendar(CalendarDate::new(2026, 1, 1).unwrap())
                            .disabled(true)
                            .on_change(|_| "changed".to_owned()),
                    ),
                    5 => Box::new(
                        list()
                            .disabled(true)
                            .item(ListItem::new("a", "A"))
                            .on_change(|s| s),
                    ),
                    6 => Box::new(
                        menu()
                            .disabled(true)
                            .item(MenuItem::new("a", "A"))
                            .on_select(|s| s),
                    ),
                    _ => Box::new(
                        select()
                            .disabled(true)
                            .option(SelectOption::new("a", "A"))
                            .on_change(|s| s),
                    ),
                }
            },
            30,
            10,
        );
        app.step(&[
            click(1, 1),
            click(4, 4),
            wheel(true, 5, 3),
            drag(10, 2),
            up(10, 2),
        ])
        .unwrap();
        assert!(app.state().is_empty(), "kind={kind}");
    }
}

#[test]
fn empty_tables_still_resize_and_invisible_calendars_ignore_clicks() {
    let mut app = runtime(
        |_| {
            data_table::<()>().columns([
                DataTableColumn::new("First").width(8),
                DataTableColumn::new("Second").width(8),
            ])
        },
        30,
        8,
    );
    app.step(&[click(9, 1), drag(11, 1), up(11, 1)]).unwrap();
    assert_eq!(
        app.widget_state::<DataTableState>(&WidgetId::root())
            .unwrap()
            .column_widths,
        [("first".to_owned(), 10)]
    );
    let mut app = runtime(
        |_| calendar(CalendarDate::new(2026, 1, 1).unwrap()).on_change(|date| date),
        8,
        4,
    );
    app.step(&[click(1, 3), wheel(true, 1, 1)]).unwrap();
    assert!(app.state().is_empty());
}

#[test]
fn tree_click_coordinates_follow_outer_scroll_clipping() {
    let mut app = runtime(
        |_| {
            scroll_view(
                col().child(label("header").height(2)).child(
                    tree()
                        .height(6)
                        .items([
                            TreeItem::new("a", "A"),
                            TreeItem::new("b", "B"),
                            TreeItem::new("c", "C"),
                        ])
                        .on_submit(|id| id),
                ),
            )
            .scrollbars(ScrollbarVisibility::Hidden)
            .scroll_step(2)
        },
        25,
        4,
    );
    app.step(&[wheel(true, 1, 0)]).unwrap();
    app.step(&[click(4, 2)]).unwrap();
    assert_eq!(app.state(), &["b"]);
}

#[test]
fn textarea_drag_selection_spans_lines_and_preserves_emoji_boundaries() {
    let mut app = App::new("a中x\n👩🏽‍💻abc".to_owned())
        .with_motion_policy(MotionPolicy::disabled())
        .build(
            |s, text| *s = text,
            |s| textarea().value(s).on_change(|s| s),
        )
        .size(25, 5);
    app.step(&[click(2, 1), drag(3, 2), up(3, 2)]).unwrap();
    let state = app
        .widget_state::<TextAreaState>(&WidgetId::root())
        .unwrap();
    assert_eq!(state.selection_anchor, Some(1));
    assert_eq!(state.cursor_position, "a中x\n👩🏽‍💻".len());
    app.step(&[Event::Paste("X".to_owned())]).unwrap();
    assert_eq!(app.state(), "aXabc");
}

#[test]
fn password_drag_uses_mask_positions_instead_of_original_text_widths() {
    let mut app = App::new("a中👩🏽‍💻z".to_owned())
        .with_motion_policy(MotionPolicy::disabled())
        .build(
            |s, text| *s = text,
            |s| {
                text_input()
                    .variant(TextInputVariant::Password)
                    .value(s)
                    .on_change(|s| s)
            },
        )
        .size(25, 3);
    app.step(&[click(2, 1), drag(4, 1), up(4, 1)]).unwrap();
    let state = app
        .widget_state::<TextInputState>(&WidgetId::root())
        .unwrap();
    assert_eq!(state.selection_anchor, Some(0));
    assert_eq!(state.cursor_position, "a中".len());
    app.step(&[Event::Paste("X".to_owned())]).unwrap();
    assert_eq!(app.state(), "X👩🏽‍💻z");
}

#[test]
fn borderless_editor_drag_above_nested_viewport_keeps_negative_coordinates() {
    let text = (0..20)
        .map(|i| format!("line{i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut app = App::new(text)
        .with_motion_policy(MotionPolicy::disabled())
        .build(
            |s, text| *s = text,
            |s| {
                col().child(label("header").height(2)).child(
                    scroll_view(
                        textarea()
                            .key("editor")
                            .variant(TextAreaVariant::Borderless)
                            .value(s)
                            .on_change(|s| s),
                    )
                    .content_height(6)
                    .scrollbars(ScrollbarVisibility::Hidden),
                )
            },
        )
        .size(20, 8);
    app.step(&[wheel(true, 2, 4)]).unwrap();
    app.step(&[click(2, 4), drag(2, 1)]).unwrap();
    let id = WidgetId::root().child("editor");
    let state = app.widget_state::<TextAreaState>(&id).unwrap();
    assert_eq!(state.scroll_offset, 2);
    assert!(state.selection_anchor.is_some());
    app.step(&[up(2, 1)]).unwrap();
    assert!(!app.widget_state::<TextAreaState>(&id).unwrap().dragging);
}
