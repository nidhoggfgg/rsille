use super::*;

fn options(args: &[&str]) -> Result<Options, String> {
    Options::parse(args.iter().map(|arg| arg.to_string()))
}

#[test]
fn native_is_default_and_obsolete_flags_are_rejected() {
    let defaults = options(&[]).unwrap();
    assert_eq!(defaults.backend, RasterBackend::Auto);
    assert!(!defaults.help);
    assert_eq!(defaults.scene, Scene::Windows);
    assert_eq!(defaults.pixels, None);
    assert!(options(&["--text-only"]).is_err());
    assert_eq!(
        options(&["--sixel"]).unwrap_err(),
        "unknown argument: --sixel; use --help"
    );
    assert!(options(&["--cell-pixels", "8x18"]).is_ok());
    assert!(options(&["--cell-pixels", "8"]).is_err());
    assert!(options(&["--cell-pixels"]).is_err());
    assert!(options(&["--unknown"]).is_err());
}

#[test]
fn help_headless_and_backend_choices_are_available() {
    assert!(options(&["--help"]).unwrap().help);
    assert!(options(&["-h"]).unwrap().help);
    assert_eq!(options(&["--headless"]).unwrap().scene, Scene::Headless);
    assert_eq!(options(&["--basic"]).unwrap().scene, Scene::Basic);
    assert!(options(&["--basic", "--headless"]).is_err());
    assert!(HELP.contains("--backend auto|kitty|sixel"));
    for (arg, backend) in [
        ("auto", RasterBackend::Auto),
        ("kitty", RasterBackend::Kitty),
        ("sixel", RasterBackend::Sixel),
    ] {
        assert_eq!(options(&["--backend", arg]).unwrap().backend, backend);
    }
    assert!(options(&["--backend"]).is_err());
    assert!(options(&["--backend", "text"]).is_err());
}

#[test]
fn inline_argument_is_rejected() {
    for args in [
        vec!["--inline"],
        vec!["--cell-pixels", "8x18", "--inline"],
        vec!["--headless", "--inline"],
    ] {
        assert_eq!(
            options(&args).unwrap_err(),
            "unknown argument: --inline; use --help"
        );
    }
}

#[test]
fn forcing_grid_sized_cells_requires_an_explicit_override() {
    assert_eq!(
        options(&["--force-cell-pixels"]).unwrap_err(),
        "--force-cell-pixels requires --cell-pixels WxH"
    );
    for args in [
        vec!["--cell-pixels", "120x36", "--force-cell-pixels"],
        vec!["--force-cell-pixels", "--cell-pixels", "120x36"],
    ] {
        assert!(options(&args).unwrap().force_cell_pixels);
    }
}

#[test]
fn terminal_grid_is_not_a_cell_pixel_measurement() {
    let opts = options(&["--cell-pixels", "120x36"]).unwrap();
    let error = opts.measured_pixels((120, 36)).unwrap_err();
    assert!(error.contains("120 columns x 36 rows"));
    assert!(error.contains("physical pixels per character cell"));
    assert!(error.contains("Omit --cell-pixels"));
    assert!(error.contains("--force-cell-pixels"));
    // Detection is based on the current grid, not a hardcoded 120x36 ban.
    let error = options(&["--cell-pixels", "80x24"])
        .unwrap()
        .measured_pixels((80, 24))
        .unwrap_err();
    assert!(error.contains("80 columns x 24 rows"));
    assert!(opts.measured_pixels((100, 32)).is_ok());
}

#[test]
fn unusual_grid_sized_cells_can_be_confirmed() {
    let pixels = CellPixels {
        width: 120,
        height: 36,
    };
    let forced = options(&["--cell-pixels", "120x36", "--force-cell-pixels"]).unwrap();
    assert_eq!(forced.measured_pixels((120, 36)).unwrap(), Some(pixels));
    // Confirmation bypasses only the grid ambiguity, never backend limits.
    for (value, grid) in [
        ("0x18", (0, 18)),
        ("8x0", (8, 0)),
        ("1025x18", (1025, 18)),
        ("8x1025", (8, 1025)),
        ("257x256", (257, 256)),
        ("1024x1020", (1024, 1020)),
    ] {
        assert!(options(&["--cell-pixels", value, "--force-cell-pixels"])
            .unwrap()
            .measured_pixels(grid)
            .is_err());
    }
}

#[test]
fn explicit_metrics_stay_authoritative_without_aspect_ratio_rules() {
    for value in [
        "8x16", "8x18", "9x18", "10x20", "18x18", "36x6", "120x36", "1x1024", "1024x1", "256x256",
        "1024x64",
    ] {
        let opts = options(&["--cell-pixels", value]).unwrap();
        assert_eq!(opts.measured_pixels((110, 36)).unwrap(), opts.pixels);
    }
}

#[test]
fn two_windows_really_drag_and_reuse_source() {
    headless().unwrap();
}

#[test]
fn resize_and_encoding_changes_keep_both_renderers_visible() {
    let mut runtime = application(110, 36, None).unwrap();
    for size in [(80, 24), (48, 16), (110, 36)] {
        runtime.resize(size.into());
        runtime.step(&[]).unwrap();
        assert!(runtime.bounds(&window_path("native")).is_some());
        assert!(runtime.bounds(&window_path("characters")).is_some());
        assert!(runtime.frame().has_raster());
        assert!(!runtime.frame().has_unsupported_raster_composition());
        let identity = runtime.state().image.id();
        for encoding in [Encoding::Braille, Encoding::Quadrant, Encoding::HalfBlock] {
            runtime
                .step(&[Event::Key(KeyCode::Char('e').into())])
                .unwrap();
            assert_eq!(runtime.state().encoding, encoding);
            assert_eq!(runtime.state().image.id(), identity);
        }
    }
}

#[test]
fn both_documented_quit_keys_exit() {
    for key in [KeyCode::Esc, KeyCode::Char('q')] {
        let mut runtime = application(80, 24, None).unwrap();
        runtime.step(&[]).unwrap();
        runtime.step(&[Event::Key(key.into())]).unwrap();
        assert!(runtime.exit_reason().is_some());
    }
}

#[cfg(feature = "kitty")]
#[test]
fn dragging_the_fullscreen_scene_reuses_kitty_uploads() {
    use rsille::render::{Builder, RasterSession};
    let mut runtime = application(110, 36, None).unwrap();
    runtime.step(&[]).unwrap();
    let mut output = Vec::new();
    let mut renderer = Builder::new()
        .size((110, 36))
        .enable_alt_screen()
        .raster_session(RasterSession::KittyFullscreen(CellPixels {
            width: 10,
            height: 20,
        }))
        .build_render(runtime.frame().clone(), &mut output);
    renderer.render().unwrap();
    // Raise the native window before moving it so all its surfaces are resident.
    let title = runtime
        .bounds(&window_path("native").child("native-title"))
        .unwrap();
    for kind in [
        MouseEventKind::Down(MouseButton::Left),
        MouseEventKind::Up(MouseButton::Left),
    ] {
        runtime
            .step(&[mouse(kind, title.x() + 3, title.y())])
            .unwrap();
    }
    *renderer.thing_mut() = runtime.frame().clone();
    renderer.render().unwrap();
    drag(&mut runtime, "native", "native-title", 5, 2).unwrap();
    *renderer.thing_mut() = runtime.frame().clone();
    renderer.render().unwrap();
    let trace = std::str::from_utf8(&output).unwrap();
    assert!(trace.contains("\x1b_Ga=t,"));
    let moved = trace.rsplit_once("\x1b[?2026h").unwrap().1;
    assert!(moved.contains("\x1b_Ga=p,"));
    assert!(!moved.contains("\x1b_Ga=t,"));
}

#[cfg(feature = "sixel")]
#[test]
fn native_window_emits_sixel_while_canvas_remains_character_output() {
    use rsille::render::Builder;
    let session = RasterSession::SixelFullscreen(CellPixels {
        width: 10,
        height: 20,
    });
    let mut runtime = application(110, 36, Some(session)).unwrap();
    runtime.step(&[]).unwrap();
    let native_cells = runtime.frame().raster_cells().count();
    assert!(native_cells > 0);
    assert!(!runtime.frame().has_unsupported_raster_composition());
    let mut output = Vec::new();
    {
        let mut renderer = Builder::new()
            .size((110, 36))
            .enable_alt_screen()
            .raster_session(session)
            .build_render(runtime.frame().clone(), &mut output);
        renderer.render().unwrap();
        assert_eq!(renderer.raster_stats().tiles_sent, native_cells as u64);
        let stats = renderer.raster_stats();
        renderer.render().unwrap();
        assert_eq!(renderer.raster_stats(), stats);
    }
    let header = b"\x1bP7;1;0q\"1;1;10;20";
    assert_eq!(
        output
            .windows(header.len())
            .filter(|bytes| *bytes == header)
            .count(),
        native_cells
    );
    assert!(String::from_utf8(output).unwrap().contains('▀'));
}
