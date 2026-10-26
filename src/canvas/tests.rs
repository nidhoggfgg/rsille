use super::{
    braille::{Pixel, PixelOp},
    Canvas, Encoding, Paint, PaintErr,
};
use crate::render::{area::Area, buffer::Buffer, chunk::Chunk, style::Stylized, Draw};
use crossterm::style::{Color, Colors};

fn render(canvas: &mut Canvas, width: u16, height: u16) -> Buffer {
    let size = (width, height).into();
    let mut buffer = Buffer::new(size);
    canvas
        .draw(Chunk::new(&mut buffer, Area::new((0, 0).into(), size)).unwrap())
        .unwrap();
    buffer
}

fn first(canvas: &mut Canvas) -> Stylized {
    render(canvas, 1, 1).content()[0].content.clone()
}

#[test]
fn default_braille_preserves_every_coverage_pattern() {
    // Compare all 256 patterns to the existing low-level Braille primitive.
    for mask in 1u16..256 {
        let mut canvas = Canvas::new();
        let mut pixel = Pixel::new();
        for i in 0..8 {
            if mask & (1 << i) != 0 {
                let (x, y) = (i % 2, i / 2);
                canvas.set(x, y);
                pixel.set(x, y);
            }
        }
        assert_eq!(first(&mut canvas).as_char(), Some(pixel.to_char()));
    }
}

#[test]
fn half_block_preserves_top_bottom_colors_in_either_drawing_order() {
    for reverse in [false, true] {
        let mut canvas = Canvas::with_encoding(Encoding::HalfBlock);
        let mut samples = [(0, Color::Blue), (1, Color::Red)];
        if reverse {
            samples.reverse();
        }
        for (y, color) in samples {
            canvas.set_color(0, y, color);
        }
        let cell = first(&mut canvas);
        assert_eq!(cell.as_char(), Some('▀'));
        assert_eq!(
            cell.style.colors,
            Some(Colors {
                foreground: Some(Color::Red),
                background: Some(Color::Blue),
            })
        );
    }
}

#[test]
fn half_block_handles_partial_uniform_and_default_colored_samples() {
    let mut canvas = Canvas::with_encoding(Encoding::HalfBlock);
    canvas.set_color(0, 0, Color::Green);
    assert_eq!(first(&mut canvas).as_char(), Some('▄'));
    canvas.clear();
    canvas.set_color(0, 1, Color::Green);
    assert_eq!(first(&mut canvas).as_char(), Some('▀'));
    canvas.set_color(0, 0, Color::Green);
    assert_eq!(first(&mut canvas).as_char(), Some('█'));
    canvas.set(0, 0);
    let cell = first(&mut canvas);
    assert_eq!(cell.as_char(), Some('▄'));
    assert_eq!(
        cell.style.colors.unwrap(),
        Colors {
            foreground: None,
            background: Some(Color::Green),
        }
    );
    canvas.set(0, 1).set_color(0, 0, Color::Red);
    let cell = first(&mut canvas);
    assert_eq!(cell.as_char(), Some('▀'));
    assert_eq!(
        cell.style.colors.unwrap(),
        Colors {
            foreground: None,
            background: Some(Color::Red),
        }
    );
}

#[test]
fn braille_partial_coverage_mixes_colors_without_last_writer_winning() {
    let red = Color::Rgb { r: 255, g: 0, b: 0 };
    let blue = Color::Rgb { r: 0, g: 0, b: 255 };
    let mut a = Canvas::new();
    let mut b = Canvas::new();
    a.set_color(0, 0, red).set_color(1, 3, blue);
    b.set_color(1, 3, blue).set_color(0, 0, red);
    assert_eq!(first(&mut a), first(&mut b));
    let cell = first(&mut a);
    assert_eq!(cell.as_char(), Some('\u{2848}'));
    assert_eq!(
        cell.style.colors.unwrap().foreground,
        Some(Color::Rgb {
            r: 128,
            g: 0,
            b: 128
        })
    );
    a.unset(1, 3);
    assert_eq!(first(&mut a).style.colors.unwrap().foreground, Some(red));
}

#[test]
fn braille_full_coverage_fits_two_colors_and_uniform_cells_fill_background() {
    let black = Color::Rgb { r: 0, g: 0, b: 0 };
    let red = Color::Rgb { r: 255, g: 0, b: 0 };
    let mut canvas = Canvas::new();
    for y in 0..4 {
        for x in 0..2 {
            canvas.set_color(x, y, black);
        }
    }
    let cell = first(&mut canvas);
    assert_eq!(
        cell.style.colors.unwrap(),
        Colors {
            foreground: Some(black),
            background: Some(black)
        }
    );
    canvas.set_color(0, 3, red);
    let cell = first(&mut canvas);
    assert_eq!(cell.as_char(), Some('\u{2801}'));
    assert_eq!(
        cell.style.colors.unwrap(),
        Colors {
            foreground: Some(red),
            background: Some(black)
        }
    );
}

#[test]
fn negative_samples_use_euclidean_cells_for_both_encodings() {
    for encoding in [Encoding::Braille, Encoding::HalfBlock] {
        let mut canvas = Canvas::with_encoding(encoding);
        canvas.set(-1, -1).set(0, 0);
        assert_eq!(canvas.get_size(), (2, 2));
        let buffer = render(&mut canvas, 2, 2);
        let chars: Vec<_> = buffer
            .content()
            .iter()
            .map(|c| c.content.as_char())
            .collect();
        let expected = match encoding {
            Encoding::Braille => [Some(' '), Some('\u{2840}'), Some('\u{2808}'), Some(' ')],
            Encoding::HalfBlock => [Some(' '), Some('▄'), Some('▀'), Some(' ')],
        };
        assert_eq!(chars, expected);
    }
}

#[test]
fn fixed_bounds_reject_samples_before_storage_and_lines_clip() {
    for encoding in [Encoding::Braille, Encoding::HalfBlock] {
        let mut canvas = Canvas::with_encoding(encoding);
        canvas.set_bound((0, 0), (0, 0));
        canvas.fixed_bound(true);
        canvas.set_color(2, 0, Color::Red).set(-1, -1);
        canvas.set(f64::NAN, 0.0).toggle(f64::INFINITY, 0.0);
        canvas.line((-1e9, 0.0), (1e9, 0.0));
        canvas.line((0.0, f64::NAN), (0.0, 0.0));
        assert_eq!(canvas.get_size(), (1, 1));
        canvas.set_bound((-1, 1), (-1, 1));
        let buffer = render(&mut canvas, 3, 3);
        assert_eq!(
            buffer
                .content()
                .iter()
                .filter(|c| c.content.as_char() != Some(' '))
                .count(),
            1
        );
        canvas.clear();
        canvas.line((-0.1, 0.0), (-0.1, 0.0));
        assert_eq!(
            render(&mut canvas, 3, 3)
                .content()
                .iter()
                .filter(|c| c.content.as_char() != Some(' '))
                .count(),
            1,
        );
        canvas.clear();
        canvas.line((0, 0), (0, 0));
        assert_eq!(
            render(&mut canvas, 3, 3)
                .content()
                .iter()
                .filter(|c| c.content.as_char() != Some(' '))
                .count(),
            1
        );
    }
}

#[test]
fn background_paints_empty_cells_and_clear_retains_configuration() {
    let mut canvas = Canvas::with_encoding(Encoding::HalfBlock);
    canvas.set_bound((0, 1), (0, 1));
    canvas
        .set_background(Color::Blue)
        .set_color(0, 0, Color::Red);
    canvas.clear();
    let buffer = render(&mut canvas, 2, 2);
    assert!(buffer
        .content()
        .iter()
        .all(|cell| cell.content.style.colors.unwrap().background == Some(Color::Blue)));
    canvas.clear_background();
    assert!(render(&mut canvas, 2, 2).content().iter().all(|cell| cell
        .content
        .style
        .colors
        .is_none()));
    canvas.reset();
    assert_eq!(canvas.encoding(), Encoding::HalfBlock);
    assert_eq!(canvas.get_size(), (1, 1));
}

#[test]
fn toggle_removes_sample_color_and_restores_default_ink() {
    let mut canvas = Canvas::with_encoding(Encoding::HalfBlock);
    canvas.set_color(0, 0, Color::Red).toggle(0, 0);
    assert_eq!(first(&mut canvas).as_char(), Some(' '));
    canvas.toggle(0, 0);
    let cell = first(&mut canvas);
    assert_eq!(cell.as_char(), Some('▄'));
    assert!(cell.style.colors.is_none());
}

#[test]
fn writer_and_draw_use_the_same_encoding_and_reset_colors() {
    for encoding in [Encoding::Braille, Encoding::HalfBlock] {
        let mut canvas = Canvas::with_encoding(encoding);
        canvas.set_bound((0, 1), (0, 0));
        canvas.set(0, 0);
        let expected: String = render(&mut canvas, 2, 1)
            .content()
            .iter()
            .map(|c| c.content.as_char().unwrap())
            .chain(['\n'])
            .collect();
        let mut bytes = Vec::new();
        canvas.print_on(&mut bytes, false).unwrap();
        assert_eq!(String::from_utf8(bytes).unwrap(), expected);
        canvas.set_color(0, 0, Color::Red);
        let mut bytes = Vec::new();
        canvas.print_on(&mut bytes, true).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("\x1b[0m"));
        assert!(text.ends_with("\x1b[1E"));
    }
}

#[test]
fn boxed_painters_dispatch_to_the_inner_implementation() {
    struct Point;
    impl Paint for Point {
        fn paint<T: Into<f64>>(&self, canvas: &mut Canvas, x: T, y: T) -> Result<(), PaintErr> {
            canvas.set(x.into(), y.into());
            Ok(())
        }
    }
    let mut canvas = Canvas::with_encoding(Encoding::HalfBlock);
    canvas.paint(&Box::new(Point), 0, 0).unwrap();
    assert_eq!(first(&mut canvas).as_char(), Some('▄'));
}

#[test]
fn half_block_uses_single_column_fallback_under_wide_ambiguous_policy() {
    use crate::render::text::{AmbiguousWidth, TextMetrics};
    let mut canvas = Canvas::with_encoding(Encoding::HalfBlock);
    canvas
        .set_color(0, 1, Color::Red)
        .set_color(0, 0, Color::Blue);
    canvas.set(1, 0);
    canvas.set(2, 0).set(2, 1);
    let size = (3, 1).into();
    let mut buffer = Buffer::new(size);
    let metrics = TextMetrics {
        ambiguous_width: AmbiguousWidth::Wide,
        ..TextMetrics::default()
    };
    buffer.set_text_metrics(metrics);
    canvas
        .draw(Chunk::new(&mut buffer, Area::new((0, 0).into(), size)).unwrap())
        .unwrap();
    for (index, cell) in buffer.content().iter().enumerate() {
        assert_eq!(cell.content.width_with(metrics, index), 1);
        assert_ne!(cell.content.as_char(), Some(' '));
    }
    assert_eq!(
        buffer.content()[0].content.style.colors.unwrap(),
        Colors {
            foreground: Some(Color::Red),
            background: Some(Color::Blue),
        }
    );
}
