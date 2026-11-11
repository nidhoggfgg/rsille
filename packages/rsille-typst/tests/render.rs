use std::sync::OnceLock;

use rsille_typst::{Error, MathRenderer, RenderOptions, Rgb};

fn renderer() -> &'static MathRenderer {
    static RENDERER: OnceLock<MathRenderer> = OnceLock::new();
    RENDERER.get_or_init(|| {
        MathRenderer::new(typst_assets::fonts(), "New Computer Modern Math").unwrap()
    })
}

#[test]
fn real_formula_has_ink_and_is_repeatable() {
    let options = RenderOptions::default();
    let formula = r"sum_(k=1)^n k = frac(n(n+1), 2)";
    let result = renderer()
        .render_with_diagnostics(formula, &options)
        .unwrap();
    assert!(result.warnings.is_empty(), "{:?}", result.warnings);
    let canvas = result.canvas;
    assert!(canvas.width() > 50 && canvas.height() > 20);
    assert!(canvas.pixels().contains(&options.background));
    assert!(canvas.pixels().iter().any(|p| *p != options.background));
    let again = renderer().render(formula, &options).unwrap();
    assert_eq!(canvas.pixels(), again.pixels());
    assert_eq!(
        (canvas.width(), canvas.height()),
        (again.width(), again.height())
    );
}

#[test]
fn nonfatal_typst_warnings_are_retained() {
    let result = renderer()
        .render_with_diagnostics(
            r#"#context text(font: "rsille-nonexistent-font", [x])"#,
            &RenderOptions::default(),
        )
        .unwrap();
    assert!(
        result
            .warnings
            .iter()
            .any(|warning| warning.message.contains("unknown font family")),
        "{:?}",
        result.warnings
    );
    assert!(!result.canvas.pixels().is_empty());
}

#[test]
fn scale_and_font_size_change_resolution() {
    let mut options = RenderOptions {
        pixels_per_point: 1.0,
        ..Default::default()
    };
    let small = renderer().render("sqrt(x) + alpha", &options).unwrap();
    options.pixels_per_point = 2.0;
    let scaled = renderer().render("sqrt(x) + alpha", &options).unwrap();
    assert!(scaled.width().abs_diff(small.width() * 2) <= 1);
    assert!(scaled.height().abs_diff(small.height() * 2) <= 1);
    options.font_size *= 2.0;
    let large = renderer().render("sqrt(x) + alpha", &options).unwrap();
    assert!(large.width() > scaled.width());
    assert!(large.height() > scaled.height());
}

#[test]
fn custom_colors_and_antialiasing() {
    let options = RenderOptions {
        foreground: Rgb(220, 100, 40),
        background: Rgb(20, 30, 10),
        // Thick strokes guarantee fully covered pixels as well as AA edges.
        font_size: 72.0,
        ..Default::default()
    };
    let canvas = renderer().render("x + y", &options).unwrap();
    assert_eq!(canvas.background(), options.background);
    assert!(canvas.pixels().contains(&options.background));
    assert!(canvas.pixels().contains(&options.foreground));
    assert!(canvas
        .pixels()
        .iter()
        .any(|p| *p != options.background && *p != options.foreground));
    for &Rgb(r, g, b) in canvas.pixels() {
        assert!((20..=220).contains(&r));
        assert!((30..=100).contains(&g));
        assert!((10..=40).contains(&b));
    }
}

#[test]
fn transparent_content_is_composited_once() {
    let options = RenderOptions {
        background: Rgb(20, 40, 60),
        ..Default::default()
    };
    let canvas = renderer()
        .render(
            r##"#box(width: 10pt, height: 10pt, fill: rgb("#c8643280"))"##,
            &options,
        )
        .unwrap();
    // Premultiplied (100, 50, 25, 128) over the chosen background.
    assert!(canvas.pixels().contains(&Rgb(110, 70, 55)));
    assert!(!canvas.pixels().contains(&Rgb(200, 100, 50)));
}

#[test]
fn original_compile_errors_and_hints_are_available() {
    let error = renderer()
        .render("frac(", &RenderOptions::default())
        .unwrap_err();
    let Error::Compile(diagnostics) = &error else {
        panic!("{error:?}")
    };
    assert!(!diagnostics.is_empty());
    assert!(error.to_string().contains(diagnostics[0].message.as_str()));
    let error = renderer()
        .render("unknownfunction(1)", &RenderOptions::default())
        .unwrap_err();
    assert!(matches!(error, Error::Compile(_)));
}

#[test]
fn files_packages_and_dates_are_unavailable() {
    for formula in [
        r#"#read("/etc/passwd")"#,
        r#"#read("../../Cargo.toml")"#,
        r#"#include "/etc/passwd""#,
        r#"#{ import "@preview/cetz:0.3.4": *; [x] }"#,
        "#datetime.today()",
    ] {
        let error = renderer()
            .render(formula, &RenderOptions::default())
            .unwrap_err();
        assert!(matches!(error, Error::Compile(_)), "{formula}: {error:?}");
        let message = error.to_string();
        assert!(
            message.contains("denied")
                || message.contains("cannot access file system")
                || message.contains("current date"),
            "{formula}: {message}"
        );
    }
}

#[test]
fn math_input_cannot_escape_into_document_template() {
    let error = renderer()
        .render("$ #set page(width: 100000pt) $", &RenderOptions::default())
        .unwrap_err();
    assert!(matches!(error, Error::Compile(_)));
}

#[test]
fn invalid_fonts_and_missing_math_family_are_errors() {
    assert!(matches!(
        MathRenderer::new([b"invalid"], "missing"),
        Err(Error::InvalidFont(0))
    ));
    assert!(matches!(
        MathRenderer::new(std::iter::empty::<&[u8]>(), "missing"),
        Err(Error::MissingMathFont(_))
    ));
    assert!(matches!(
        MathRenderer::new(typst_assets::fonts(), "missing"),
        Err(Error::MissingMathFont(_))
    ));
    assert!(matches!(
        MathRenderer::new(typst_assets::fonts(), "DejaVu Sans Mono"),
        Err(Error::MissingMathFont(_))
    ));
}

#[test]
fn invalid_options_are_rejected_before_compilation() {
    for value in [f64::NAN, f64::INFINITY, -1.0, 0.0, 5000.0] {
        let options = RenderOptions {
            font_size: value,
            ..Default::default()
        };
        assert!(matches!(
            renderer().render("frac(", &options),
            Err(Error::InvalidOptions(_))
        ));
    }
    for value in [f32::NAN, f32::INFINITY, -1.0, 0.0, 17.0] {
        let options = RenderOptions {
            pixels_per_point: value,
            ..Default::default()
        };
        assert!(matches!(
            renderer().render("x", &options),
            Err(Error::InvalidOptions(_))
        ));
    }
    for max_pixels in [0, usize::MAX] {
        let options = RenderOptions {
            max_pixels,
            ..Default::default()
        };
        assert!(matches!(
            renderer().render("x", &options),
            Err(Error::InvalidOptions(_))
        ));
    }
}

#[test]
fn real_layout_is_rejected_before_oversized_raster_allocation() {
    let options = RenderOptions {
        max_pixels: 1,
        ..Default::default()
    };
    assert!(matches!(
        renderer().render("x", &options),
        Err(Error::RasterLimit)
    ));
    assert!(matches!(
        renderer().render(
            "#box(width: 100000pt, height: 1pt)",
            &RenderOptions::default()
        ),
        Err(Error::RasterLimit)
    ));
}

#[test]
fn existing_rsille_widget_accepts_result() {
    use rsille::tui::prelude::*;
    let canvas = renderer().render("x^2", &RenderOptions::default()).unwrap();
    let size = CellPixels {
        width: 8,
        height: 16,
    }
    .size_for_pixels(canvas.width(), canvas.height())
    .unwrap();
    let _widget = raster_canvas::<()>(canvas)
        .scale(RasterScale::Original)
        .fixed(size.width, size.height);
}
