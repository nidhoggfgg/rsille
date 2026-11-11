use super::*;
use rsille::{
    raster::Rgb,
    render::raster_output::{RasterOutput, RasterPolicy},
};

fn parse(args: &[&str]) -> Result<Options, Box<dyn Error>> {
    Ok(animation_args(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())?.0)
}

#[test]
fn rendering_options_validate_modes_colors_and_physical_metrics() {
    for args in [
        vec![
            "--raster",
            "--background",
            "#18203a",
            "--character",
            "claude",
            "--height",
            "32",
        ],
        vec![
            "--height",
            "32",
            "--background",
            "18203A",
            "--character",
            "claude",
            "--raster",
        ],
    ] {
        let options = parse(&args).unwrap();
        assert_eq!(options.output, Output::Raster);
        assert_eq!(options.background, Rgb(24, 32, 58));
        assert_eq!(options.character, scene::Character::Claude);
        assert_eq!(options.height, 32);
    }
    for args in [
        vec!["--raster", "--sixel"],
        vec!["--sixel", "--raster"],
        vec!["--background", "000000"],
        vec!["--raster", "--background"],
        vec!["--raster", "--background", "00000g"],
        vec!["--raster", "--background", "00000"],
        vec!["--cell-pixels", "10x20"],
        vec!["--raster", "--cell-pixels", "10x20"],
        vec!["--sixel", "--cell-pixels"],
        vec!["--sixel", "--cell-pixels", "0x20"],
        vec!["--sixel", "--cell-pixels", "1025x20"],
        vec!["--sixel", "--cell-pixels", "1024x1024"],
        vec!["--sixel", "--cell-pixels", "10x20x30"],
        vec!["--sixel", "--force-cell-pixels"],
    ] {
        assert!(parse(&args).is_err(), "{args:?}");
    }
    let args = ["--sixel", "--cell-pixels", "10x20", "--headless"].map(str::to_string);
    let (options, rest) = animation_args(&args).unwrap();
    assert_eq!(
        options.pixels,
        Some(CellPixels {
            width: 10,
            height: 20
        })
    );
    assert_eq!(rest, &["--headless"]);
    assert!(export::command(&["--snapshot".into(), "unused.ppm".into()], options).is_err());
}

#[test]
fn text_output_never_probes_and_native_output_never_silently_falls_back() {
    for output in [Output::Quadrant, Output::Raster] {
        let options = Options {
            output,
            ..Options::default()
        };
        assert_eq!(
            options
                .native_config_with((80, 24), |_| panic!("text must not probe"))
                .unwrap(),
            RasterOutput::text_only()
        );
    }
    let options = parse(&["--sixel", "--cell-pixels", "10x20"]).unwrap();
    if !cfg!(feature = "sixel") {
        assert!(options
            .native_config_with((80, 24), |_| panic!("feature check must precede probe"))
            .is_err());
        return;
    }
    let config = options.native_config((80, 24)).unwrap();
    assert_eq!(config.policy, RasterPolicy::RequireSixel);
    assert_eq!(config.capabilities.cell_pixels, options.pixels);
    assert!(
        config.capabilities.sixel
            && config.capabilities.reliable_inline_erase
            && !config.capabilities.reliable_erase
            && config.capabilities.no_scroll
    );
    assert!(options.native_config((10, 20)).is_err());
    let forced = Options {
        force_cell_pixels: true,
        ..options
    };
    assert!(forced.native_config((10, 20)).is_ok());
    let automatic = parse(&["--sixel"]).unwrap();
    assert!(automatic
        .native_config_with((80, 24), |pixels| {
            assert!(pixels.is_none());
            Err(std::io::Error::other("no measured cell pixels"))
        })
        .is_err());
    assert_eq!(
        automatic
            .native_config_with((80, 24), |pixels| {
                assert!(pixels.is_none());
                Ok(CellPixels {
                    width: 10,
                    height: 20,
                })
            })
            .unwrap()
            .capabilities
            .cell_pixels,
        options.pixels
    );
}

#[test]
fn raster_playback_preserves_sources_across_holds_switches_and_resize() {
    use crossterm::event::KeyEvent;
    let options = parse(&[
        "--sixel",
        "--cell-pixels",
        "10x20",
        "--background",
        "18203a",
    ])
    .unwrap();
    let mut nursery = Nursery::new(options).unwrap();
    let size = (80, 24).into();
    let first = export::frame(&mut nursery, size).unwrap();
    let cell = first.raster_at((0, 4).into()).unwrap();
    assert!(first.raster_at((31, 4).into()).is_some());
    assert!(first.raster_at((32, 4).into()).is_none());
    assert_eq!(cell.target, (32, 16).into());
    assert_eq!((cell.canvas.width(), cell.canvas.height()), (320, 320));
    assert_eq!(cell.options.background, options.background);
    assert_eq!(first.raster_cells().count(), 32 * 16);
    let identity = (cell.canvas.id(), cell.canvas.version());
    nursery.time = 0.5;
    let held = export::frame(&mut nursery, size).unwrap();
    let cell = held.raster_at((0, 4).into()).unwrap();
    assert_eq!((cell.canvas.id(), cell.canvas.version()), identity);

    nursery.paused = true;
    nursery.action = Some(scene::Action::Wave);
    for (key, character) in [
        ('d', scene::Character::DeepSeek),
        ('c', scene::Character::Claude),
        ('g', scene::Character::ChatGpt),
    ] {
        nursery
            .on_events(&[Event::Key(KeyEvent::new(
                KeyCode::Char(key),
                KeyModifiers::NONE,
            ))])
            .unwrap();
        assert_eq!(nursery.character, character);
        assert!(nursery.paused);
        assert_eq!(nursery.time, 0.0);
        assert_eq!(nursery.action, Some(scene::Action::Wave));
        for dimensions in [(1, 1), (17, 5), (80, 24), (120, 40), (0, 0)] {
            let buffer = export::frame(&mut nursery, dimensions.into()).unwrap();
            assert_eq!(buffer.has_raster(), dimensions != (0, 0));
            for (x, _, cell) in buffer.raster_cells() {
                assert_eq!(x, cell.x, "raster must start at the left edge after resize");
                assert_eq!(cell.options.align_x, RasterAlign::Start);
                assert_eq!((cell.canvas.width(), cell.canvas.height()), (320, 320));
                assert!(cell.x < cell.target.width && cell.y < cell.target.height);
                assert_eq!(cell.options.background, options.background);
            }
        }
    }
    nursery.action = None;
    let returned = export::frame(&mut nursery, size).unwrap();
    assert_eq!(
        returned.raster_at((0, 4).into()).unwrap().canvas.id(),
        identity.0
    );
}

#[test]
fn raster_exports_reconstruct_opaque_halfblock_fallbacks() {
    let mut nursery =
        Nursery::new(parse(&["--raster", "--background", "18203a"]).unwrap()).unwrap();
    let buffer = export::frame(&mut nursery, (80, 24).into()).unwrap();
    let rgba = export::rgba(&buffer);
    assert_eq!(rgba.len(), 160 * 96);
    assert!(rgba.iter().all(|pixel| pixel[3] == 255));
    for y in 0..96usize {
        for x in 0..160usize {
            let cell = buffer
                .raster_at(((x / 2) as u16, (y / 4) as u16).into())
                .unwrap();
            let rgb = cell.sample(0, (y % 4 / 2) as u32, 1, 2);
            assert_eq!(rgba[y * 160 + x], [rgb.0, rgb.1, rgb.2, 255]);
        }
    }
}

#[cfg(feature = "sixel")]
#[test]
fn native_inline_playback_is_compact_and_reuses_unchanged_frames() {
    // Synthetic safety assertions apply to the memory writer, not a real terminal.
    let options = parse(&["--sixel", "--cell-pixels", "10x20"]).unwrap();
    let nursery = Nursery::new(options).unwrap();
    let mut bytes = Vec::new();
    {
        let mut renderer =
            playback_builder(options, (80, 40), options.native_config((80, 40)).unwrap())
                .build_render(nursery, &mut bytes);
        assert_eq!(renderer.size(), (80, 16).into());
        renderer.render().unwrap();
        let stats = renderer.raster_stats();
        assert_eq!(stats.tiles_sent, 32 * 16);
        renderer.render().unwrap();
        assert_eq!(renderer.raster_stats(), stats);
        renderer.thing_mut().time = 1.6;
        renderer.render().unwrap();
        assert!(renderer.raster_stats().tiles_sent > stats.tiles_sent);
    }
    let header = b"\x1bP0;1;0q\"1;1;10;20";
    assert!(bytes.windows(header.len()).any(|part| part == header));
    assert!(bytes.windows(4).any(|part| part == b"\x1b[2K"));
    assert!(!bytes.windows(4).any(|part| part == b"\x1b[2J"));
    assert!(!bytes.windows(8).any(|part| part == b"\x1b[?1049h"));
}

#[test]
fn playback_height_is_bounded_for_native_and_character_modes() {
    for output in [Output::Quadrant, Output::Raster, Output::Sixel] {
        for height in [24, 32] {
            let options = Options {
                output,
                height,
                pixels: Some(CellPixels {
                    width: 10,
                    height: 20,
                }),
                ..Options::default()
            };
            // Geometry is independent of the encoder feature and terminal queries.
            let mut config = RasterOutput::text_only();
            config.capabilities.cell_pixels = options.pixels;
            for grid in [(80, 40), (80, 12), (1, 1)] {
                let renderer = playback_builder(options, grid, config)
                    .build_render(Nursery::default(), Vec::new());
                let expected = if output == Output::Sixel { 16 } else { height };
                assert_eq!(renderer.size(), (grid.0, grid.1.min(expected)).into());
            }
        }
    }
}
