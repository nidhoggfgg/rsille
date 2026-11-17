//! Check terminal output, style state, and inline region ownership.
use super::*;
use crate::render::style::{Style, Stylized};
use crossterm::style::{force_color_output, Attributes, Color, Colors};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct TerminalStyle {
    fg: Option<Color>,
    bg: Option<Color>,
    attrs: Attributes,
}

// A small SGR interpreter for the commands used in these fixtures. Cursor
// commands do not change style. Fail on unsupported SGR instead of ignoring it.
fn printed_styles(output: &[u8]) -> Vec<(char, TerminalStyle)> {
    let text = std::str::from_utf8(output).unwrap();
    let mut chars = text.chars();
    let mut style = TerminalStyle::default();
    let mut printed = Vec::new();
    while let Some(ch) = chars.next() {
        if ch != '\x1b' {
            if !ch.is_control() {
                printed.push((ch, style.clone()));
            }
            continue;
        }
        assert_eq!(chars.next(), Some('['));
        let mut params = String::new();
        let command = loop {
            let ch = chars.next().expect("complete CSI");
            if ('@'..='~').contains(&ch) {
                break ch;
            }
            params.push(ch);
        };
        if command != 'm' {
            continue;
        }
        let codes: Vec<u16> = params.split(';').map(|p| p.parse().unwrap_or(0)).collect();
        let mut codes = codes.into_iter();
        while let Some(code) = codes.next() {
            match code {
                0 => style = TerminalStyle::default(),
                1 => style.attrs.set(Attribute::Bold),
                3 => style.attrs.set(Attribute::Italic),
                4 => style.attrs.set(Attribute::Underlined),
                39 => style.fg = None,
                49 => style.bg = None,
                38 | 48 => {
                    let color = match codes.next().unwrap() {
                        5 => Color::AnsiValue(codes.next().unwrap() as u8),
                        2 => Color::Rgb {
                            r: codes.next().unwrap() as u8,
                            g: codes.next().unwrap() as u8,
                            b: codes.next().unwrap() as u8,
                        },
                        mode => panic!("unsupported color mode {mode}"),
                    };
                    if code == 38 {
                        style.fg = Some(color);
                    } else {
                        style.bg = Some(color);
                    }
                }
                code => panic!("unsupported SGR {code}"),
            }
        }
    }
    printed
}

struct Scene(Vec<Stylized>);
impl Draw for Scene {
    fn draw(&mut self, mut chunk: Chunk) -> Result<Size, DrawErr> {
        let size = chunk.area().size();
        for (i, cell) in self.0.iter().enumerate() {
            chunk.set(i as u16 % size.width, i as u16 / size.width, cell.clone())?;
        }
        Ok(size)
    }
}

#[derive(Default)]
struct FailingWriter {
    bytes: Vec<u8>,
    remaining: Option<usize>,
    fail_flush: bool,
}

impl std::io::Write for FailingWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let len = match self.remaining.as_mut() {
            Some(0) => return Err(std::io::Error::other("injected write failure")),
            Some(remaining) => {
                let len = bytes.len().min(*remaining);
                *remaining -= len;
                len
            }
            None => bytes.len(),
        };
        self.bytes.extend_from_slice(&bytes[..len]);
        Ok(len)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        if self.fail_flush {
            Err(std::io::Error::other("injected flush failure"))
        } else {
            Ok(())
        }
    }
}

fn colors(fg: Option<Color>, bg: Option<Color>) -> Style {
    Style::with_colors(Colors {
        foreground: fg,
        background: bg,
    })
}

fn expected(style: Style) -> TerminalStyle {
    TerminalStyle {
        fg: style
            .colors
            .and_then(|c| c.foreground)
            .filter(|c| *c != Color::Reset),
        bg: style
            .colors
            .and_then(|c| c.background)
            .filter(|c| *c != Color::Reset),
        attrs: style.attr.unwrap_or_default(),
    }
}

#[test]
fn clear_inline_erases_only_the_known_text_region_and_resets_diff() {
    let mut render = Builder::new()
        .size((1, 3))
        .inline_mode(true)
        .build_render(Scene(vec![Stylized::plain('a'); 3]), Vec::new());
    render.render().unwrap();
    assert_eq!(render.previous_inline_height, 3);
    render.out.clear();

    render.clear_inline().unwrap();
    assert_eq!(
        render.out,
        b"\x1b[0m\x1b[1G\x1b[2K\x1b[1F\x1b[2K\x1b[1F\x1b[2K"
    );
    assert_eq!(render.previous_inline_height, 0);
    assert!(render.buffer.previous().is_none());
    assert!(!render.output_uncertain);

    render.out.clear();
    render.clear_inline().unwrap();
    assert_eq!(render.out, b"\x1b[0m");
    render.out.clear();
    render.render().unwrap();
    assert_eq!(
        printed_styles(&render.out),
        vec![('a', TerminalStyle::default()); 3]
    );
}

#[test]
fn clear_inline_after_commit_preserves_history_and_next_region_is_fresh() {
    let mut render = Builder::new()
        .size((1, 2))
        .inline_mode(true)
        .build_render(Scene(vec![Stylized::plain('a'); 2]), Vec::new());
    render.render().unwrap();
    render.out.clear();
    render.commit_frame().unwrap();
    assert_eq!(render.out, b"\r\n");
    assert_eq!(render.previous_inline_height, 0);
    assert!(render.buffer.previous().is_none());
    assert!(!render.output_uncertain);

    for _ in 0..2 {
        render.out.clear();
        render.clear_inline().unwrap();
        assert_eq!(render.out, b"\x1b[0m");
    }
    render.out.clear();
    render.render().unwrap();
    assert_eq!(
        printed_styles(&render.out),
        vec![('a', TerminalStyle::default()); 2]
    );
    render.out.clear();
    render.clear_inline().unwrap();
    assert_eq!(render.out, b"\x1b[0m\x1b[1G\x1b[2K\x1b[1F\x1b[2K");
}

#[test]
fn inline_output_failures_cleanup_without_relative_moves_or_erasure() {
    for operation in ["render", "commit", "clear"] {
        for fail_flush in [false, true] {
            let mut render = Builder::new().size((1, 2)).inline_mode(true).build_render(
                Scene(vec![Stylized::plain('a'); 2]),
                FailingWriter::default(),
            );
            render.render().unwrap();
            render.out.bytes.clear();
            // Allow a partial command through before failing the next write.
            render.out.remaining = (!fail_flush).then_some(1);
            render.out.fail_flush = fail_flush;
            let result = match operation {
                "render" => render.render(),
                "commit" => render.commit_frame(),
                "clear" => render.clear_inline(),
                _ => unreachable!(),
            };
            assert!(result.is_err(), "{operation}, flush={fail_flush}");
            assert!(render.output_uncertain, "{operation}, flush={fail_flush}");
            assert!(!render.out.bytes.is_empty());

            render.out.remaining = None;
            render.out.fail_flush = false;
            render.out.bytes.clear();
            assert!(render.commit_frame().is_err());
            assert!(render.out.bytes.is_empty());
            assert!(render.output_uncertain);

            render.clear_inline().unwrap();
            // Only cancel protocol state, end synchronization, and reset style.
            // In particular, never move/erase relative to the unknown cursor.
            assert_eq!(render.out.bytes, b"\x18\x1b\\\x1b[?2026l\x1b[0m");
            assert!(!render.output_uncertain);
            assert_eq!(render.previous_inline_height, 0);
            assert!(render.buffer.previous().is_none());
            render.out.bytes.clear();
            render.clear_inline().unwrap();
            assert_eq!(render.out.bytes, b"\x1b[0m");

            render.out.bytes.clear();
            render.render().unwrap();
            assert_eq!(
                printed_styles(&render.out.bytes),
                vec![('a', TerminalStyle::default()); 2]
            );
        }
    }
}

#[test]
fn inline_draw_error_keeps_prior_owned_region_available_for_cleanup() {
    let mut render = Builder::new()
        .size((1, 2))
        .inline_mode(true)
        .build_render(Scene(vec![Stylized::plain('a'); 2]), Vec::new());
    render.render().unwrap();
    render.out.clear();
    // Scene propagates the out-of-bounds error before terminal output begins.
    render.thing.0.push(Stylized::plain('b'));
    assert!(render.render().is_err());
    assert!(render.out.is_empty());
    assert!(!render.output_uncertain);
    assert_eq!(render.previous_inline_height, 2);

    render.clear_inline().unwrap();
    assert_eq!(render.out, b"\x1b[0m\x1b[1G\x1b[2K\x1b[1F\x1b[2K");
    assert_eq!(render.previous_inline_height, 0);
    assert!(render.buffer.previous().is_none());
}

#[test]
fn inline_cleanup_and_commit_reject_fullscreen_without_output_or_state_changes() {
    let mut render = Builder::new()
        .size((1, 1))
        .inline_mode(false)
        .build_render(Scene(vec![Stylized::plain('a')]), Vec::new());
    render.render().unwrap();
    render.out.clear();
    let pos = render.pos;
    for commit in [false, true] {
        let result = if commit {
            render.commit_frame()
        } else {
            render.clear_inline()
        };
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::Unsupported);
        assert!(render.out.is_empty());
        assert!(!render.output_uncertain);
        assert_eq!(render.pos, pos);
        assert!(render.buffer.previous().is_some());
    }
}

#[test]
fn style_transitions_are_independent_in_full_and_differential_frames() {
    force_color_output(true);
    let red = Color::AnsiValue(1);
    let yellow = Color::AnsiValue(11);
    let styles = [
        Style::default(),
        colors(Some(red), Some(yellow)),
        colors(Some(red), None),
        colors(None, Some(yellow)),
        colors(None, None),
        colors(Some(Color::Reset), Some(Color::Reset)),
        colors(Some(red), None).attr(Attribute::Bold.into()),
        Style::with_attr(Attribute::Underlined.into()),
        Style::with_attr(Attributes::default()),
    ];
    for inline in [false, true] {
        for first in styles {
            for second in styles {
                let mut render = Builder::new()
                    .size((2, 1))
                    .clear(false)
                    .inline_mode(inline)
                    .build_render(Scene(vec![]), Vec::new());
                for symbols in [['a', 'b'], ['c', 'd']] {
                    render.thing.0 = vec![
                        Stylized::new(symbols[0], first),
                        Stylized::new(symbols[1], second),
                    ];
                    render.render().unwrap();
                    let actual = printed_styles(&render.out);
                    assert_eq!(
                        &actual[actual.len() - 2..],
                        &[
                            (symbols[0], expected(first)),
                            (symbols[1], expected(second))
                        ],
                        "inline={inline}, first={first:?}, second={second:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn default_cell_does_not_inherit_a_previous_frames_style() {
    force_color_output(true);
    for inline in [false, true] {
        let style = colors(Some(Color::AnsiValue(1)), Some(Color::AnsiValue(11)))
            .attr(Attribute::Bold.into());
        let mut render = Builder::new()
            .size((1, 1))
            .clear(false)
            .inline_mode(inline)
            .build_render(Scene(vec![Stylized::new('a', style)]), Vec::new());
        render.render().unwrap();
        // An unchanged frame must not break the synchronization either.
        render.render().unwrap();
        render.thing.0 = vec![Stylized::plain('b')];
        render.render().unwrap();
        assert_eq!(
            printed_styles(&render.out).last(),
            Some(&('b', TerminalStyle::default()))
        );
    }
}

#[test]
fn sparse_diff_does_not_rely_on_unchanged_cells_to_reset_style() {
    force_color_output(true);
    let bg = colors(None, Some(Color::AnsiValue(11)));
    let fg = colors(Some(Color::AnsiValue(1)), None);
    let mut render = Builder::new().size((3, 1)).clear(false).build_render(
        Scene(vec![
            Stylized::new('a', bg),
            Stylized::space(),
            Stylized::new('|', fg),
        ]),
        Vec::new(),
    );
    render.render().unwrap();
    render.thing.0[0].set_char('b');
    render.thing.0[2].set_char('!');
    let start = render.out.len();
    render.render().unwrap();
    let diff = printed_styles(&render.out[start..]);
    assert_eq!(diff, vec![('b', expected(bg)), ('!', expected(fg))]);
}

#[test]
fn explicit_attribute_reset_preserves_the_cells_colors() {
    force_color_output(true);
    for inline in [false, true] {
        let style = colors(Some(Color::AnsiValue(1)), Some(Color::AnsiValue(11)))
            .attr(Attribute::Reset.into());
        let mut render = Builder::new()
            .size((1, 1))
            .clear(false)
            .inline_mode(inline)
            .build_render(Scene(vec![Stylized::new('a', style)]), Vec::new());
        render.render().unwrap();
        let mut want = expected(style);
        want.attrs = Attributes::default();
        assert_eq!(printed_styles(&render.out), vec![('a', want)]);
    }
}

#[test]
fn clear_and_inline_height_shrink_use_default_background() {
    force_color_output(true);
    let bg = colors(None, Some(Color::AnsiValue(11)));
    let mut render = Builder::new().size((1, 2)).inline_mode(true).build_render(
        Scene(vec![Stylized::new('a', bg), Stylized::new('b', bg)]),
        Vec::new(),
    );
    render.render().unwrap();
    render.used_height = 1;
    render.thing.0.pop();
    render.render().unwrap();
    let output = String::from_utf8(render.out).unwrap();
    let clear = output.find("\x1b[2K").unwrap();
    // Probe the active style exactly where erase-line is executed.
    let mut prefix = output.as_bytes()[..clear].to_vec();
    prefix.push(b'?');
    assert_eq!(
        printed_styles(&prefix).last(),
        Some(&('?', TerminalStyle::default()))
    );

    let mut inherited = b"\x1b[48;5;11m".to_vec();
    let mut render = Builder::new()
        .size((1, 1))
        .clear(true)
        .build_render(Scene(vec![Stylized::plain('a')]), &mut inherited);
    render.render().unwrap();
    let clear = inherited.windows(4).position(|w| w == b"\x1b[2J").unwrap();
    let mut prefix = inherited[..clear].to_vec();
    prefix.push(b'?');
    assert_eq!(
        printed_styles(&prefix).last(),
        Some(&('?', TerminalStyle::default()))
    );
}
