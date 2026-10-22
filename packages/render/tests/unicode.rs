use render::{
    area::Area,
    buffer::{Buffer, LineState},
    chunk::Chunk,
    style::{Style, Stylized},
    text::*,
};

fn symbol(buffer: &Buffer, x: u16, y: u16) -> String {
    buffer
        .get((x, y).into())
        .map(|c| c.symbol().into_owned())
        .unwrap_or_default()
}

fn assert_owners(buffer: &Buffer) {
    let columns = buffer.size().width as usize;
    for (i, cell) in buffer.content().iter().enumerate() {
        if cell.is_occupied {
            let owner = cell.owner.expect("continuations have owners");
            assert!(owner < i);
            assert_eq!(owner / columns, i / columns);
            assert!(!buffer.content()[owner].is_occupied);
            assert!(owner + buffer.content()[owner].width() > i);
        } else {
            assert!(cell.owner.is_none());
            for tail in i + 1..i + cell.width() {
                assert_eq!(buffer.content()[tail].owner, Some(i));
            }
        }
    }
}

#[test]
fn metrics_share_grapheme_boundaries_columns_tabs_and_cjk_policy() {
    let metrics = TextMetrics::default();
    let text = "e\u{301}👩🏽‍💻中\tz";
    let graphemes: Vec<_> = text.grapheme_indices(true).collect();
    assert_eq!(graphemes.len(), 5);
    assert_eq!(metrics.width(text), 9);
    for (byte, g) in &graphemes {
        assert_eq!(floor_boundary(text, byte + g.len() - 1), *byte);
        assert_eq!(ceil_boundary(text, byte + 1), byte + g.len());
    }
    assert_eq!(metrics.byte_to_column(text, graphemes[2].0), 3);
    assert_eq!(metrics.column_to_byte(text, 2), graphemes[1].0);
    assert_eq!(metrics.column_to_byte(text, 6), graphemes[3].0);
    assert_eq!(metrics.truncate(text, 2), "e\u{301}");
    assert_eq!(metrics.truncate(text, 3), "e\u{301}👩🏽‍💻");
    assert_eq!(metrics.clip(text, 2, 4), " 中 ");
    assert_eq!(metrics.expand("中\tx"), "中  x");
    assert_eq!(metrics.width("·"), 1);
    let cjk = TextMetrics {
        ambiguous_width: AmbiguousWidth::Wide,
        tab_width: 8,
    };
    assert_eq!(cjk.width("·"), 2);
    assert_eq!(cjk.width("👩🏽‍💻"), 2);
    assert_eq!(cjk.width("·\tx"), 9);
    assert_eq!(
        TextMetrics {
            tab_width: 0,
            ..metrics
        }
        .width("\t"),
        1
    );
}

#[test]
fn wrapping_and_position_conversions_preserve_clusters_and_empty_lines() {
    let metrics = TextMetrics::default();
    let text = "e\u{301}👩🏽‍💻中\n\nabc";
    let lines = metrics.wrap(text, 3);
    let actual: Vec<_> = lines
        .iter()
        .map(|line| (&text[line.bytes.clone()], line.width))
        .collect();
    assert_eq!(
        actual,
        vec![("e\u{301}👩🏽‍💻", 3), ("中", 2), ("", 0), ("abc", 3)]
    );
    let byte = text.find('中').unwrap();
    assert_eq!(metrics.byte_to_line_column(text, byte), (0, 3));
    assert_eq!(metrics.line_column_to_byte(text, 0, 2), "e\u{301}".len());
    assert_eq!(metrics.line_column_to_byte(text, 2, 2), text.len() - 1);
    assert_eq!(metrics.wrap("中", 1).len(), 1);
    assert!(metrics.wrap(text, 0).is_empty());
}

#[test]
fn buffer_replaces_entire_owners_and_emits_old_tails_in_both_diff_modes() {
    for use_set in [false, true] {
        let mut buffer = Buffer::new((6, 2).into());
        buffer
            .set((0, 0).into(), Stylized::grapheme("👩🏽‍💻", Style::default()))
            .unwrap();
        buffer.set((2, 0).into(), Stylized::plain('中')).unwrap();
        assert_owners(&buffer);
        assert_eq!(symbol(&buffer, 0, 0), "👩🏽‍💻");
        buffer.clear();
        buffer
            .set((0, 0).into(), Stylized::grapheme("👩🏽‍💻", Style::default()))
            .unwrap();
        buffer.set((2, 0).into(), Stylized::plain('中')).unwrap();
        if use_set {
            buffer.set((0, 0).into(), Stylized::plain('x')).unwrap();
        } else {
            buffer
                .overwrite((0, 0).into(), Stylized::plain('x'))
                .unwrap();
        }
        buffer
            .overwrite((3, 0).into(), Stylized::plain('y'))
            .unwrap();
        assert_owners(&buffer);
        let diff: Vec<_> = buffer
            .diff()
            .unwrap()
            .map(|(x, y, c)| (x, y, c.content.symbol().into_owned()))
            .collect();
        assert_eq!(
            diff,
            vec![
                (0, 0, "x".into()),
                (1, 0, " ".into()),
                (2, 0, " ".into()),
                (3, 0, "y".into())
            ]
        );
        let first = buffer.diff_lines().next().unwrap();
        match first.state {
            LineState::Changed {
                current_len,
                previous_len,
                cells,
            } => {
                assert_eq!((current_len, previous_len), (6, 6));
                assert_eq!(
                    cells
                        .map(|c| c.content.symbol().into_owned())
                        .collect::<String>(),
                    "x  y  "
                );
            }
            _ => panic!("changed line"),
        }
    }
}

#[test]
fn overwriting_overlapping_wide_glyphs_never_leaves_orphan_continuations() {
    let glyphs = ["a", "中", "e\u{301}", "👩🏽‍💻", "🇨🇳", "\u{301}"];
    for first in glyphs {
        for second in glyphs {
            for x in 0..6 {
                let mut buffer = Buffer::new((6, 2).into());
                for col in [0, 2, 4] {
                    buffer
                        .overwrite((col, 0).into(), Stylized::grapheme(first, Style::default()))
                        .unwrap();
                }
                let _ =
                    buffer.overwrite((x, 0).into(), Stylized::grapheme(second, Style::default()));
                assert_owners(&buffer);
                assert!(buffer.content()[6..]
                    .iter()
                    .all(|c| c.content.as_char() == Some(' ')));
            }
        }
    }
}

#[test]
fn chunk_drawing_clips_whole_graphemes_and_expands_tabs_relative_to_text_origin() {
    let mut buffer = Buffer::new((8, 2).into());
    let mut chunk = Chunk::new(&mut buffer, Area::new((2, 0).into(), (5, 2).into())).unwrap();
    chunk
        .set_string(0, 0, "e\u{301}👩🏽‍💻中x", Style::default())
        .unwrap();
    chunk.set_string(0, 1, "a\tz", Style::default()).unwrap();
    assert_eq!(symbol(&buffer, 2, 0), "e\u{301}");
    assert_eq!(symbol(&buffer, 3, 0), "👩🏽‍💻");
    assert_eq!(symbol(&buffer, 5, 0), "中");
    assert_eq!(symbol(&buffer, 6, 1), "z");
    assert_eq!(symbol(&buffer, 7, 0), " ");
    assert!(buffer
        .overwrite((7, 0).into(), Stylized::plain('中'))
        .is_err());
    assert!(buffer.set((8, 0).into(), Stylized::plain('x')).is_err());
    assert_owners(&buffer);
}

#[test]
fn rich_text_merges_clusters_across_spans_and_uses_leading_style() {
    use crossterm::style::{Color, Colors};
    let first = Style::with_colors(Colors::new(Color::Red, Color::Black));
    let second = Style::with_colors(Colors::new(Color::Blue, Color::Black));
    let line = Line::from_texts(vec![
        Span::new("e", first),
        Span::new("\u{301}👩🏽‍", second),
        Span::plain("💻"),
    ]);
    assert_eq!(line.width(), 3);
    assert_eq!(line.cells().len(), 2);
    let text = Text::new([line]);
    let mut buffer = Buffer::new((3, 1).into());
    let mut chunk = Chunk::new(&mut buffer, Area::new((0, 0).into(), (3, 1).into())).unwrap();
    chunk.set_text(0, 0, &text).unwrap();
    assert_eq!(symbol(&buffer, 0, 0), "e\u{301}");
    assert_eq!(buffer.content()[0].content.style, first);
    assert_eq!(symbol(&buffer, 1, 0), "👩🏽‍💻");
    assert_eq!(buffer.content()[1].content.style, second);
}

#[test]
fn terminal_output_contains_complete_clusters_and_diff_detects_combining_changes() {
    let mut output = Vec::new();
    Stylized::grapheme("👩🏽‍💻", Style::default())
        .queue(&mut output)
        .unwrap();
    assert_eq!(output, "👩🏽‍💻".as_bytes());
    let mut buffer = Buffer::new((2, 1).into());
    buffer
        .set(
            (0, 0).into(),
            Stylized::grapheme("e\u{301}", Style::default()),
        )
        .unwrap();
    buffer.clear();
    buffer
        .set(
            (0, 0).into(),
            Stylized::grapheme("e\u{300}", Style::default()),
        )
        .unwrap();
    assert_eq!(buffer.diff().unwrap().count(), 1);
    let metrics = TextMetrics {
        ambiguous_width: AmbiguousWidth::Wide,
        ..Default::default()
    };
    buffer.set_text_metrics(metrics);
    buffer.set((0, 0).into(), Stylized::plain('·')).unwrap();
    assert_eq!(buffer.content()[0].width(), 2);
    assert!(buffer.content()[1].is_occupied);
    buffer.resize((1, 1).into());
    assert!(!buffer.content()[0].is_occupied);
    assert!(buffer.previous().is_none());
}

#[test]
fn diff_coordinates_do_not_truncate_large_buffer_indices() {
    let mut buffer = Buffer::new((400, 200).into());
    buffer.clear();
    buffer.set((399, 199).into(), Stylized::plain('x')).unwrap();
    let (x, y, _) = buffer.diff().unwrap().next().unwrap();
    assert_eq!((x, y), (399, 199));
}
