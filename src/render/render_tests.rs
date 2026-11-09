//! Check terminal SGR state at each printed cell, not just the frame buffer.
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
