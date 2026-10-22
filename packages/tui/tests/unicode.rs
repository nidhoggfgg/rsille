use tui::prelude::*;
use tui::text::{floor_boundary, UnicodeSegmentation};
use tui::widgets::text_input::TextInputState;

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::empty()))
}
fn shift(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::SHIFT))
}
fn paste(text: &str) -> Event {
    Event::Paste(text.to_owned())
}
fn input(multiline: bool, value: &str) -> Box<dyn Widget<String>> {
    if multiline {
        Box::new(textarea().value(value).on_change(|s| s))
    } else {
        Box::new(text_input().value(value).on_change(|s| s))
    }
}
fn cursor(app: &Runtime<String, String>, multiline: bool) -> (usize, Option<usize>) {
    if multiline {
        let s = app
            .widget_state::<TextAreaState>(&WidgetId::root())
            .unwrap();
        (s.cursor_position, s.selection_anchor)
    } else {
        let s = app
            .widget_state::<TextInputState>(&WidgetId::root())
            .unwrap();
        (s.cursor_position, s.selection_anchor)
    }
}
fn row<S, M: Clone + std::fmt::Debug + Send + 'static>(app: &Runtime<S, M>, y: u16) -> String {
    let start = y as usize * app.frame().size().width as usize;
    app.frame().content()[start..start + app.frame().size().width as usize]
        .iter()
        .filter(|c| !c.is_occupied)
        .map(|c| c.content.symbol().into_owned())
        .collect()
}
fn symbol<S, M: Clone + std::fmt::Debug + Send + 'static>(
    app: &Runtime<S, M>,
    x: u16,
    y: u16,
) -> String {
    app.frame()
        .get((x, y).into())
        .unwrap()
        .symbol()
        .into_owned()
}

#[test]
fn both_editors_move_delete_select_and_paste_whole_graphemes_in_one_batch() {
    for multiline in [false, true] {
        let mut app = App::new(String::new())
            .build(|s, value| *s = value, move |s| input(multiline, s))
            .size(30, 5);
        app.step(&[paste("e\u{301}👍🏽👨‍👩‍👧‍👦🇨🇳")]).unwrap();
        assert_eq!(cursor(&app, multiline).0, app.state().len());
        app.step(&[
            key(KeyCode::Left),
            key(KeyCode::Backspace),
            key(KeyCode::Delete),
        ])
        .unwrap();
        assert_eq!(app.state(), "e\u{301}👍🏽");
        app.step(&[shift(KeyCode::Left)]).unwrap();
        assert_eq!(
            cursor(&app, multiline),
            ("e\u{301}".len(), Some("e\u{301}👍🏽".len()))
        );
        app.step(&[paste("👩🏽‍💻")]).unwrap();
        assert_eq!(app.state(), "e\u{301}👩🏽‍💻");
        app.step(&[key(KeyCode::Home), key(KeyCode::Delete)])
            .unwrap();
        assert_eq!(app.state(), "👩🏽‍💻");
        app.step(&[key(KeyCode::End), key(KeyCode::Backspace)])
            .unwrap();
        assert_eq!(app.state(), "");
    }
}

#[test]
fn separately_typed_combining_marks_and_zwj_sequences_join_without_invalid_cursors() {
    for multiline in [false, true] {
        let mut app = App::new(String::new())
            .build(|s, value| *s = value, move |s| input(multiline, s))
            .size(30, 5);
        let events: Vec<_> = "e\u{301}👩🏽‍💻"
            .chars()
            .map(|c| key(KeyCode::Char(c)))
            .collect();
        app.step(&events).unwrap();
        assert_eq!(app.state(), "e\u{301}👩🏽‍💻");
        assert_eq!(cursor(&app, multiline).0, app.state().len());
        app.step(&[key(KeyCode::Backspace)]).unwrap();
        assert_eq!(app.state(), "e\u{301}");
        app.step(&[key(KeyCode::Backspace)]).unwrap();
        assert!(app.state().is_empty());
    }
    let mut editor = EditBuffer::new("👩💻".into(), "👩".len(), None);
    editor.handle(&paste("\u{200d}"), TextMetrics::default(), false);
    assert_eq!(editor.value, "👩‍💻");
    assert_eq!(editor.cursor, editor.value.len());
}

#[test]
fn external_replacement_repairs_cursor_and_selection_before_drawing_or_input() {
    for multiline in [false, true] {
        let mut app = App::new("ab".to_owned())
            .build(|s, value| *s = value, move |s| input(multiline, s))
            .size(30, 5);
        app.step(&[key(KeyCode::End), shift(KeyCode::Left)])
            .unwrap();
        assert_eq!(cursor(&app, multiline), (1, Some(2)));
        app.handle().send("e\u{301}👩🏽‍💻".into()).unwrap();
        app.step(&[]).unwrap();
        assert_eq!(cursor(&app, multiline), (0, Some(0)));
        app.step(&[key(KeyCode::Delete)]).unwrap();
        assert_eq!(app.state(), "👩🏽‍💻");
        app.handle().send(String::new()).unwrap();
        app.step(&[]).unwrap();
        assert_eq!(cursor(&app, multiline), (0, None));
    }
}

#[test]
fn multiline_navigation_uses_terminal_columns_and_clicks_snap_to_grapheme_leads() {
    let mut app = App::new("a中z\nabcde".to_owned())
        .build(|s, value| *s = value, |s| input(true, s))
        .size(30, 5);
    app.step(&[key(KeyCode::Right), key(KeyCode::Right), key(KeyCode::Down)])
        .unwrap();
    assert_eq!(cursor(&app, true).0, "a中z\nabc".len());
    app.step(&[key(KeyCode::Up)]).unwrap();
    assert_eq!(cursor(&app, true).0, "a中".len());
    app.step(&[Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 3,
        row: 1,
        modifiers: KeyModifiers::empty(),
    })])
    .unwrap();
    // Border at x=0, 'a' at x=1, '中' spans x=2..4.
    assert_eq!(cursor(&app, true).0, 1);
    assert_eq!(symbol(&app, 2, 1), "中");
    assert!(app.frame().is_occupied((3, 1).into()));
}

#[test]
fn bracketed_paste_normalizes_line_endings_and_does_not_submit_or_inject_controls() {
    for multiline in [false, true] {
        let mut app = App::new(String::new())
            .build(|s, value| *s = value, move |s| input(multiline, s))
            .size(30, 5);
        app.step(&[paste("中\r\n👩🏽‍💻\rZ\u{1b}[31m")]).unwrap();
        let expected = if multiline {
            "中\n👩🏽‍💻\nZ[31m"
        } else {
            "中 👩🏽‍💻 Z[31m"
        };
        assert_eq!(app.state(), expected);
    }
}

#[test]
fn input_horizontal_scrolling_and_password_masks_use_graphemes_and_active_widths() {
    let value = "e\u{301}👩🏽‍💻中abcdef";
    let mut app = App::new(value.to_owned())
        .build(|s, value| *s = value, |s| input(false, s))
        .size(10, 3);
    app.step(&[key(KeyCode::End)]).unwrap();
    assert!(row(&app, 1).contains("bcdef"));
    app.step(&[key(KeyCode::Home)]).unwrap();
    assert_eq!(symbol(&app, 2, 1), "e\u{301}");
    assert_eq!(symbol(&app, 3, 1), "👩🏽‍💻");
    let mut password = App::new(value.to_owned())
        .build(
            |s, value| *s = value,
            |s| {
                text_input()
                    .value(s)
                    .variant(TextInputVariant::Password)
                    .on_change(|s| s)
            },
        )
        .size(24, 3);
    password.step(&[key(KeyCode::End)]).unwrap();
    let rendered = row(&password, 1);
    assert_eq!(rendered.matches('•').count(), value.graphemes(true).count());
    assert!(!rendered.contains('👩'));
}

#[test]
fn selection_background_covers_a_complete_cluster_and_replacement_is_atomic() {
    let theme = Theme::dark();
    let selection_bg = theme
        .styles
        .selection_range
        .to_render_style()
        .colors
        .unwrap()
        .background;
    let mut app = App::new("👩🏽‍💻x".to_owned())
        .with_theme(theme)
        .build(|s, value| *s = value, |s| input(false, s))
        .size(24, 3);
    app.step(&[shift(KeyCode::Right)]).unwrap();
    assert_eq!(symbol(&app, 2, 1), "👩🏽‍💻");
    assert_eq!(
        app.frame().content()[26]
            .content
            .style
            .colors
            .unwrap()
            .background,
        selection_bg
    );
    assert_eq!(app.frame().content()[27].owner, Some(26));
    app.step(&[paste("e\u{301}")]).unwrap();
    assert_eq!(app.state(), "e\u{301}x");
    assert!(!app.frame().is_occupied((3, 1).into()));
}

#[test]
fn label_layout_and_mapped_scope_share_theme_width_policy_with_the_buffer() {
    let metrics = TextMetrics {
        ambiguous_width: AmbiguousWidth::Wide,
        tab_width: 8,
    };
    let mut app = App::new(())
        .with_theme(Theme::dark().with_text_metrics(metrics))
        .build(
            |_, _: ()| {},
            |_| {
                tui::layout::row()
                    .child(label::<()>("·👩🏽‍💻").map(|_| ()).scope("label"))
                    .child(label("x").key("tail"))
            },
        )
        .size(20, 1);
    app.step(&[]).unwrap();
    assert_eq!(app.frame().text_metrics(), metrics);
    assert_eq!(
        app.bounds(&WidgetPath::root().child("tail")).unwrap().x(),
        4
    );
    assert_eq!(symbol(&app, 0, 0), "·");
    assert_eq!(symbol(&app, 2, 0), "👩🏽‍💻");
    assert_eq!(symbol(&app, 4, 0), "x");
}

#[test]
fn labels_lists_and_tables_keep_the_same_graphemes_when_clipping() {
    let value = "e\u{301}👩🏽‍💻中x";
    let mut label_app = App::new(())
        .build(|_, _: ()| {}, move |_| label(value))
        .size(4, 1);
    label_app.step(&[]).unwrap();
    assert_eq!(row(&label_app, 0), "e\u{301}👩🏽‍💻 ");
    let mut list_app = App::new(())
        .build(
            |_, _: ()| {},
            move |_| list().borderless().height(1).item(value),
        )
        .size(7, 1);
    list_app.step(&[]).unwrap();
    assert_eq!(symbol(&list_app, 3, 0), "e\u{301}");
    assert_eq!(symbol(&list_app, 4, 0), "👩🏽‍💻");
    assert_eq!(symbol(&list_app, 6, 0), " ");
    let mut table_app = App::new(())
        .build(
            |_, _: ()| {},
            move |_| {
                data_table()
                    .borderless()
                    .column(DataTableColumn::new("A").width(4))
                    .row(DataTableRow::new("row", [value]))
            },
        )
        .size(4, 4);
    table_app.step(&[]).unwrap();
    assert_eq!(symbol(&table_app, 0, 2), "e\u{301}");
    assert_eq!(symbol(&table_app, 1, 2), "👩🏽‍💻");
    assert_eq!(symbol(&table_app, 3, 2), " ");
}

#[test]
fn editor_repairs_arbitrary_public_positions_without_slicing_invalid_utf8() {
    let value = "e\u{301}👩🏽‍💻🇨🇳";
    for byte in 0..value.len() + 2 {
        let editor = EditBuffer::new(value.to_owned(), byte, Some(byte));
        assert_eq!(editor.cursor, floor_boundary(value, byte));
        assert_eq!(editor.anchor, Some(editor.cursor));
    }
}

#[test]
fn changing_width_policy_rebuilds_layout_and_occupancy_and_resets_diff() {
    let mut app = App::new(false)
        .with_theme_from(|wide| {
            Theme::dark().with_text_metrics(TextMetrics {
                ambiguous_width: if *wide {
                    AmbiguousWidth::Wide
                } else {
                    AmbiguousWidth::Narrow
                },
                tab_width: 4,
            })
        })
        .build(
            |state, next| *state = next,
            |_| {
                tui::layout::row()
                    .child(animate(label::<bool>("·")))
                    .child(label("x").key("tail"))
            },
        )
        .size(10, 1);
    app.step(&[]).unwrap();
    assert_eq!(
        app.bounds(&WidgetPath::root().child("tail")).unwrap().x(),
        1
    );
    app.handle().send(true).unwrap();
    app.step(&[]).unwrap();
    assert!(app.frame().previous().is_none());
    assert!(app.frame().is_occupied((1, 0).into()));
    assert_eq!(symbol(&app, 2, 0), "x");
    app.handle().send(false).unwrap();
    app.step(&[]).unwrap();
    assert!(!app.frame().is_occupied((1, 0).into()));
    assert_eq!(symbol(&app, 1, 0), "x");
    assert_eq!(symbol(&app, 2, 0), " ");
}

#[test]
fn wide_ambiguous_text_has_matching_editor_columns_and_single_column_borders() {
    let metrics = TextMetrics {
        ambiguous_width: AmbiguousWidth::Wide,
        tab_width: 4,
    };
    let mut app = App::new("·中x".to_owned())
        .with_theme(Theme::dark().with_text_metrics(metrics))
        .build(|state, next| *state = next, |value| input(false, value))
        .size(20, 3);
    app.step(&[key(KeyCode::Right)]).unwrap();
    assert_eq!(cursor(&app, false).0, "·".len());
    assert_eq!(symbol(&app, 2, 1), "·");
    assert!(app.frame().is_occupied((3, 1).into()));
    assert_eq!(symbol(&app, 4, 1), "中");
    assert_eq!(symbol(&app, 0, 0), "+");
    assert_eq!(symbol(&app, 19, 0), "+");
}
