use super::*;
use crate::render::raster::{RasterCanvas, RasterCell, RasterOptions, RasterScale, Rgb};
use crate::render::raster_output::{
    CellPixels, RasterCapabilities, RasterOutput, RasterPolicy, RasterStats,
};
use crate::render::Update;
use std::io::{self, Write};

#[derive(Clone)]
struct Scene {
    canvas: RasterCanvas,
    origin: Position,
    target: Size,
    visible: bool,
    text: Vec<(Position, char)>,
}

impl Default for Scene {
    fn default() -> Self {
        let mut canvas = RasterCanvas::new(4, 3, Rgb(255, 0, 0)).unwrap();
        canvas.set_pixel(3, 2, Rgb(0, 255, 255));
        Self {
            canvas,
            origin: (0, 0).into(),
            target: (2, 1).into(),
            visible: true,
            text: Vec::new(),
        }
    }
}

impl Draw for Scene {
    fn draw(&mut self, mut chunk: Chunk) -> Result<Size, DrawErr> {
        let size = chunk.area().size();
        // An explicit underlay makes erased/moved/hidden raster coverage observable.
        for y in 0..size.height {
            for x in 0..size.width {
                chunk.set(x, y, Stylized::plain('.'))?;
            }
        }
        if self.visible {
            for y in 0..self.target.height {
                for x in 0..self.target.width {
                    let position = (self.origin.x + x, self.origin.y + y);
                    if position.0 < size.width && position.1 < size.height {
                        chunk.set_raster_cell(
                            position.0,
                            position.1,
                            RasterCell {
                                canvas: self.canvas.clone(),
                                target: self.target,
                                x,
                                y,
                                options: RasterOptions {
                                    scale: RasterScale::Stretch,
                                    ..RasterOptions::default()
                                },
                            },
                        )?;
                    }
                }
            }
        }
        for &(position, character) in &self.text {
            chunk.set(position.x, position.y, Stylized::plain(character))?;
        }
        Ok(size)
    }
}

impl Update for Scene {
    fn on_events(&mut self, _: &[crossterm::event::Event]) -> Result<(), DrawErr> {
        Ok(())
    }

    fn update(&mut self) -> Result<bool, DrawErr> {
        Ok(false)
    }
}

fn verified(policy: RasterPolicy) -> RasterOutput {
    RasterOutput {
        policy,
        capabilities: RasterCapabilities {
            sixel: true,
            reliable_erase: true,
            no_scroll: true,
            cell_pixels: Some(CellPixels {
                width: 8,
                height: 18,
            }),
        },
    }
}

fn renderer<W: Write>(config: RasterOutput, writer: W) -> Render<W, Scene> {
    Builder::new()
        .size((4, 2))
        .clear(false)
        .enable_alt_screen()
        .raster_output(config)
        .build_render(Scene::default(), writer)
}

fn contains(bytes: &[u8], needle: &[u8]) -> bool {
    bytes.windows(needle.len()).any(|window| window == needle)
}

fn frame(render: &mut Render<Vec<u8>, Scene>) -> Vec<u8> {
    render.out.clear();
    render.render().unwrap();
    std::mem::take(&mut render.out)
}

#[test]
fn unknown_auto_and_explicit_text_only_emit_fallback_without_dcs() {
    for config in [RasterOutput::default(), verified(RasterPolicy::TextOnly)] {
        for inline in [false, true] {
            let mut render = renderer(config, Vec::new());
            render.inline_mode = inline;
            let output = frame(&mut render);
            assert!(!contains(&output, b"\x1bP"));
            assert!(!contains(&output, b"\x1b[?2026"));
            assert!(contains(&output, "▀".as_bytes()));
            assert_eq!(render.raster_stats(), RasterStats::default());
            assert!(render.buffer.previous().is_some());
            assert!(render.buffer.previous_raster_at((0, 0).into()).is_some());
        }
    }
}

#[test]
fn strict_unsupported_frames_write_nothing_and_do_not_commit() {
    let mut unsupported = vec![RasterOutput {
        policy: RasterPolicy::RequireSixel,
        ..RasterOutput::default()
    }];
    for missing in 0..5 {
        let mut config = verified(RasterPolicy::RequireSixel);
        match missing {
            0 => config.capabilities.sixel = false,
            1 => config.capabilities.reliable_erase = false,
            2 => config.capabilities.no_scroll = false,
            3 => {
                config.capabilities.cell_pixels = Some(CellPixels {
                    width: 0,
                    height: 18,
                })
            }
            _ => {
                config.capabilities.cell_pixels = Some(CellPixels {
                    width: 8,
                    height: 1025,
                })
            }
        }
        unsupported.push(config);
    }
    for config in unsupported {
        let mut render = renderer(config, Vec::new());
        render.clear = true;
        assert_eq!(
            render.render().unwrap_err().kind(),
            io::ErrorKind::Unsupported
        );
        assert!(
            render.out.is_empty(),
            "strict rejection must precede even reset/clear bytes"
        );
        assert!(render.buffer.previous().is_none());
        assert!(!render.buffer.has_raster());
        assert_eq!(render.raster_stats(), RasterStats::default());
    }
}

#[test]
fn alternate_screen_and_non_inline_mode_are_both_required() {
    for inline in [false, true] {
        for policy in [RasterPolicy::Auto, RasterPolicy::RequireSixel] {
            let mut render = renderer(verified(policy), Vec::new());
            render.inline_mode = inline;
            render.set_alternate_screen(inline);
            if policy == RasterPolicy::RequireSixel {
                assert_eq!(
                    render.render().unwrap_err().kind(),
                    io::ErrorKind::Unsupported
                );
                assert!(render.out.is_empty());
                assert!(render.buffer.previous().is_none());
            } else {
                let output = frame(&mut render);
                assert!(!contains(&output, b"\x1bP"));
                assert!(contains(&output, "▀".as_bytes()));
            }
            assert_eq!(render.raster_stats(), RasterStats::default());
        }
    }
}

fn newline_renderer(policy: RasterPolicy) -> Render<Vec<u8>, Scene> {
    Builder::new()
        .size((4, 2))
        .clear(true)
        .enable_alt_screen()
        .append_newline(true)
        .raster_output(verified(policy))
        .build_render(Scene::default(), Vec::new())
}

#[test]
fn append_newline_auto_uses_character_fallback_without_dcs() {
    let mut render = newline_renderer(RasterPolicy::Auto);
    let output = frame(&mut render);
    assert!(!contains(&output, b"\x1bP"));
    assert!(contains(&output, "▀".as_bytes()));
    assert!(output.ends_with(b"\n"));
    assert!(render.buffer.previous().is_some());
    assert!(render.buffer.previous_raster_at((0, 0).into()).is_some());
    assert_eq!(render.raster_stats(), RasterStats::default());
}

#[test]
fn append_newline_strict_rejects_before_any_output_or_commit() {
    let mut render = newline_renderer(RasterPolicy::RequireSixel);
    assert_eq!(
        render.render().unwrap_err().kind(),
        io::ErrorKind::Unsupported
    );
    assert!(
        render.out.is_empty(),
        "reject before reset, clear, DCS, or newline bytes"
    );
    assert!(render.buffer.previous().is_none());
    assert!(!render.buffer.has_raster());
    assert_eq!(render.raster_stats(), RasterStats::default());
}

#[cfg(not(feature = "sixel"))]
#[test]
fn disabled_feature_falls_back_in_auto_and_rejects_strict_even_when_verified() {
    let mut render = renderer(verified(RasterPolicy::Auto), Vec::new());
    let output = frame(&mut render);
    assert!(!contains(&output, b"\x1bP"));
    assert!(contains(&output, "▀".as_bytes()));
    assert_eq!(render.raster_stats(), RasterStats::default());

    let mut render = renderer(verified(RasterPolicy::RequireSixel), Vec::new());
    assert_eq!(
        render.render().unwrap_err().kind(),
        io::ErrorKind::Unsupported
    );
    assert!(render.out.is_empty());
    assert!(render.buffer.previous().is_none());
    assert_eq!(render.raster_stats(), RasterStats::default());
}

#[cfg(feature = "sixel")]
mod native {
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    enum Event {
        Clear,
        Print(Position, char),
        Tile(Position, Vec<u8>),
        Save(Position),
        Restore(Position),
    }

    // Parse terminal framing separately from Sixel data: palette digits and
    // printable sixel bytes must never be mistaken for ordinary text output.
    fn events(bytes: &[u8]) -> Vec<Event> {
        let mut events = Vec::new();
        let mut cursor = Position::default();
        let mut saved = None;
        let mut index = 0;
        while index < bytes.len() {
            // CAN/SUB cancel protocol state without painting or moving the cursor.
            if matches!(bytes[index], 0x18 | 0x1a) {
                index += 1;
                continue;
            }
            if bytes[index] != 0x1b {
                let character = std::str::from_utf8(&bytes[index..])
                    .unwrap()
                    .chars()
                    .next()
                    .unwrap();
                assert!(
                    !character.is_control(),
                    "unexpected control character {character:?}"
                );
                events.push(Event::Print(cursor, character));
                cursor.x += 1;
                index += character.len_utf8();
                continue;
            }
            let start = index;
            index += 2;
            match bytes[start + 1] {
                b'P' => {
                    loop {
                        assert!(index < bytes.len(), "terminated or cancelled DCS");
                        if matches!(bytes[index], 0x18 | 0x1a) {
                            // A cancelled partial tile is not a complete Tile event.
                            index += 1;
                            break;
                        }
                        if bytes[index..].starts_with(b"\x1b\\") {
                            index += 2;
                            events.push(Event::Tile(cursor, bytes[start..index].to_vec()));
                            break;
                        }
                        index += 1;
                    }
                }
                // Recovery can send ST after CAN or after an already terminated DCS.
                b'\\' => {}
                b'7' => {
                    saved = Some(cursor);
                    events.push(Event::Save(cursor));
                }
                b'8' => {
                    cursor = saved.take().expect("cursor save before restore");
                    events.push(Event::Restore(cursor));
                }
                b'[' => {
                    let parameters = index;
                    while !(0x40..=0x7e).contains(&bytes[index]) {
                        index += 1;
                    }
                    let command = bytes[index];
                    if matches!(command, b'h' | b'l') {
                        assert_eq!(&bytes[parameters..index], b"?2026");
                        index += 1;
                        continue;
                    }
                    let values: Vec<u16> = std::str::from_utf8(&bytes[parameters..index])
                        .unwrap()
                        .split(';')
                        .map(|value| value.parse().unwrap_or(0))
                        .collect();
                    index += 1;
                    match command {
                        b'm' => {}
                        b'H' | b'f' => {
                            cursor = (
                                values.get(1).copied().unwrap_or(1).max(1) - 1,
                                values[0].max(1) - 1,
                            )
                                .into()
                        }
                        b'J' => {
                            assert_eq!(values, [2], "only verified ED 2 erases graphics");
                            events.push(Event::Clear);
                        }
                        b's' => {
                            saved = Some(cursor);
                            events.push(Event::Save(cursor));
                        }
                        b'u' => {
                            cursor = saved.take().expect("cursor save before restore");
                            events.push(Event::Restore(cursor));
                        }
                        _ => panic!("unexpected CSI command {}", char::from(command)),
                    }
                }
                command => panic!("unexpected escape command {}", char::from(command)),
            }
        }
        assert!(saved.is_none(), "unrestored cursor save");
        events
    }

    fn tiles(events: &[Event]) -> Vec<(Position, &[u8])> {
        events
            .iter()
            .filter_map(|event| match event {
                Event::Tile(position, bytes) => Some((*position, bytes.as_slice())),
                _ => None,
            })
            .collect()
    }

    fn assert_clear_before_paint(events: &[Event]) {
        assert_eq!(
            events.first(),
            Some(&Event::Clear),
            "erase graphics before redrawing either text or raster"
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| **event == Event::Clear)
                .count(),
            1
        );
    }

    fn assert_print(events: &[Event], position: (u16, u16), character: char) {
        assert!(
            events.contains(&Event::Print(position.into(), character)),
            "missing {character:?} at {position:?}: {events:?}"
        );
    }

    fn assert_no_raster_commit<W: Write>(render: &Render<W, Scene>, before: RasterStats) {
        let after = render.raster_stats();
        assert_eq!(after.tiles_sent, before.tiles_sent);
        assert_eq!(after.bytes_sent, before.bytes_sent);
        assert!(
            render.buffer.previous().is_none(),
            "failed frame cannot become diff history"
        );
        assert!(
            !render.buffer.has_raster(),
            "failed working frame must be discarded"
        );
    }

    #[test]
    fn recovery_controls_do_not_hide_erase_or_paint_events() {
        for prefix in [
            b"\x18\x1b\\".as_slice(),
            b"\x1b\\".as_slice(),
            b"\x1bP0;1;0q\"1;1;8;18#1\x18\x1b\\".as_slice(),
        ] {
            let mut bytes = prefix.to_vec();
            bytes.extend_from_slice(b"\x1b[2J\x1b[2;3H.");
            let trace = events(&bytes);
            assert_clear_before_paint(&trace);
            assert_eq!(trace, [Event::Clear, Event::Print((2, 1).into(), '.')]);

            let mut paint_before_clear = prefix.to_vec();
            paint_before_clear.extend_from_slice(b".\x1b[2J");
            assert_eq!(
                events(&paint_before_clear),
                [Event::Print((0, 0).into(), '.'), Event::Clear]
            );
        }
    }

    fn assert_synchronized(bytes: &[u8]) {
        let begin = b"\x1b[?2026h";
        let end = b"\x1b[?2026l";
        assert!(
            bytes.starts_with(begin),
            "synchronize before any clear or paint"
        );
        assert!(
            bytes.ends_with(end),
            "release only after text, tiles and cursor restoration"
        );
        assert_eq!(
            bytes.windows(begin.len()).filter(|s| *s == begin).count(),
            1
        );
        assert_eq!(bytes.windows(end.len()).filter(|s| *s == end).count(), 1);
    }

    #[test]
    fn native_frames_and_cleanup_are_one_synchronized_update() {
        let mut render = renderer(verified(RasterPolicy::RequireSixel), Vec::new());
        assert_synchronized(&frame(&mut render));
        for origin in [(1, 0), (1, 1), (0, 1), (0, 0)] {
            render.thing.origin = origin.into();
            let moved = frame(&mut render);
            assert_synchronized(&moved);
            let trace = events(&moved);
            assert_clear_before_paint(&trace);
            assert_eq!(tiles(&trace).len(), 2);
        }
        // The cleanup frame must be synchronized even without new native tiles.
        render.thing.visible = false;
        let hidden = frame(&mut render);
        assert_synchronized(&hidden);
        assert_clear_before_paint(&events(&hidden));
        assert!(!contains(&frame(&mut render), b"\x1b[?2026"));

        render.thing.visible = true;
        assert_synchronized(&frame(&mut render));
        render.set_raster_output(RasterOutput::text_only());
        let fallback = frame(&mut render);
        assert_synchronized(&fallback);
        assert_clear_before_paint(&events(&fallback));
        assert!(contains(&fallback, "▀".as_bytes()));
        assert!(!contains(&frame(&mut render), b"\x1b[?2026"));
    }

    #[test]
    fn verified_builder_allows_native_and_identical_frames_do_no_raster_work() {
        for policy in [RasterPolicy::Auto, RasterPolicy::RequireSixel] {
            let mut render = renderer(verified(policy), Vec::new());
            let first = events(&frame(&mut render));
            let encoded = tiles(&first);
            assert_eq!(
                encoded
                    .iter()
                    .map(|(position, _)| *position)
                    .collect::<Vec<_>>(),
                [(0, 0).into(), (1, 0).into()]
            );
            assert!(!first
                .iter()
                .any(|event| matches!(event, Event::Print(_, '▀'))));
            let stats = render.raster_stats();
            assert_eq!(
                (
                    stats.encodes,
                    stats.cache_hits,
                    stats.cache_entries,
                    stats.tiles_sent
                ),
                (2, 0, 2, 2)
            );
            assert_eq!(
                stats.bytes_sent,
                encoded
                    .iter()
                    .map(|(_, bytes)| bytes.len() as u64)
                    .sum::<u64>()
            );
            assert_eq!(stats.cache_bytes as u64, stats.bytes_sent);
            for _ in 0..3 {
                assert!(events(&frame(&mut render)).is_empty());
                assert_eq!(render.raster_stats(), stats);
            }
        }
    }

    #[test]
    fn static_1200_cell_frames_do_no_raster_work_despite_cache_eviction() {
        let scene = Scene {
            target: (40, 30).into(),
            ..Scene::default()
        };
        let mut render = Builder::new()
            .size((42, 31))
            .clear(false)
            .enable_alt_screen()
            .raster_output(verified(RasterPolicy::RequireSixel))
            .build_render(scene, Vec::new());
        let initial = events(&frame(&mut render));
        let initial_tiles = tiles(&initial);
        assert_eq!(initial_tiles.len(), 1200);
        let initial_stats = render.raster_stats();
        assert_eq!(
            (initial_stats.encodes, initial_stats.tiles_sent),
            (1200, 1200)
        );
        assert!(initial_stats.cache_entries <= 1024);
        assert!(initial_stats.cache_bytes <= 4 * 1024 * 1024);
        assert_eq!(
            initial_stats.bytes_sent,
            initial_tiles
                .iter()
                .map(|(_, bytes)| bytes.len() as u64)
                .sum::<u64>()
        );

        // Display history, not cache residency, determines whether a static
        // tile needs work. A row-major cache lookup here would thrash the LRU.
        for _ in 0..3 {
            assert!(events(&frame(&mut render)).is_empty());
            assert_eq!(render.raster_stats(), initial_stats);
        }
        render.thing.text.push(((41, 30).into(), '!'));
        assert_eq!(
            events(&frame(&mut render)),
            [Event::Print((41, 30).into(), '!')]
        );
        assert_eq!(render.raster_stats(), initial_stats);

        render.thing.origin = (1, 0).into();
        let moved = events(&frame(&mut render));
        assert_clear_before_paint(&moved);
        let moved_tiles = tiles(&moved);
        assert_eq!(moved_tiles.len(), 1200);
        for (index, ((position, bytes), (_, original))) in
            moved_tiles.iter().zip(&initial_tiles).enumerate()
        {
            assert_eq!(
                *position,
                Position::from(((index % 40 + 1) as u16, (index / 40) as u16))
            );
            assert_eq!(
                bytes, original,
                "moving changes destination, not the sampled tile"
            );
        }
        for y in 0..30 {
            assert_print(&moved, (0, y), '.');
            assert!(render.buffer.previous_raster_at((0, y).into()).is_none());
        }
        assert_print(&moved, (41, 30), '!');
        let moved_stats = render.raster_stats();
        assert_eq!(moved_stats.tiles_sent - initial_stats.tiles_sent, 1200);
        assert_eq!(
            moved_stats.bytes_sent - initial_stats.bytes_sent,
            initial_stats.bytes_sent
        );
        // Movement may churn every entry when the working set exceeds 1024.
        // Require bounded work, not an impossible all-hit move or a specific LRU order.
        assert!(moved_stats.encodes - initial_stats.encodes <= 1200);
        assert!(moved_stats.cache_hits - initial_stats.cache_hits <= 1200);
        assert!(moved_stats.cache_entries <= 1024);
        assert!(moved_stats.cache_bytes <= 4 * 1024 * 1024);
        for _ in 0..3 {
            assert!(events(&frame(&mut render)).is_empty());
            assert_eq!(render.raster_stats(), moved_stats);
        }
    }

    #[test]
    fn unrelated_text_diff_does_not_clear_resend_or_reencode_native_tiles() {
        let mut render = renderer(verified(RasterPolicy::Auto), Vec::new());
        frame(&mut render);
        let before = render.raster_stats();
        render.thing.text.push(((3, 1).into(), '!'));
        assert_eq!(
            events(&frame(&mut render)),
            [Event::Print((3, 1).into(), '!')]
        );
        assert_eq!(render.raster_stats(), before);
        render.thing.text.clear();
        assert_eq!(
            events(&frame(&mut render)),
            [Event::Print((3, 1).into(), '.')]
        );
        assert_eq!(render.raster_stats(), before);
    }

    #[test]
    fn moving_source_or_renderer_origin_reuses_tiles_and_erases_old_coverage() {
        let mut render = renderer(verified(RasterPolicy::Auto), Vec::new());
        let initial = events(&frame(&mut render));
        let initial_tiles = tiles(&initial);
        render.thing.origin = (1, 1).into();
        let moved = events(&frame(&mut render));
        assert_clear_before_paint(&moved);
        assert_print(&moved, (0, 0), '.');
        assert_print(&moved, (1, 0), '.');
        let moved_tiles = tiles(&moved);
        assert_eq!(
            moved_tiles,
            [
                ((1, 1).into(), initial_tiles[0].1),
                ((2, 1).into(), initial_tiles[1].1)
            ]
        );
        assert!(render.buffer.previous_raster_at((0, 0).into()).is_none());
        assert!(render.buffer.previous_raster_at((1, 1).into()).is_some());
        let stats = render.raster_stats();
        assert_eq!(
            (stats.encodes, stats.cache_hits, stats.tiles_sent),
            (2, 2, 4)
        );

        render.pos = (3, 4).into();
        let moved_origin = events(&frame(&mut render));
        assert_clear_before_paint(&moved_origin);
        assert_eq!(
            tiles(&moved_origin),
            [
                ((4, 5).into(), initial_tiles[0].1),
                ((5, 5).into(), initial_tiles[1].1)
            ]
        );
        let stats = render.raster_stats();
        assert_eq!(
            (stats.encodes, stats.cache_hits, stats.tiles_sent),
            (2, 4, 6)
        );
    }

    #[test]
    fn opaque_text_occludes_exact_cells_and_removal_restores_cached_tile() {
        let mut render = renderer(verified(RasterPolicy::Auto), Vec::new());
        let initial = events(&frame(&mut render));
        render.thing.text.push(((0, 0).into(), '#'));
        let covered = events(&frame(&mut render));
        assert_clear_before_paint(&covered);
        assert_print(&covered, (0, 0), '#');
        assert_eq!(tiles(&covered), [((1, 0).into(), tiles(&initial)[1].1)]);
        assert!(render.buffer.previous_raster_at((0, 0).into()).is_none());
        assert_eq!(render.raster_stats().encodes, 2);

        render.thing.text.clear();
        let restored = events(&frame(&mut render));
        assert!(!restored.contains(&Event::Clear));
        assert_eq!(tiles(&restored), [((0, 0).into(), tiles(&initial)[0].1)]);
        assert!(!restored.iter().any(|event| matches!(event, Event::Print(position, _) if *position == Position::from((0, 0)))));
        assert!(render.buffer.previous_raster_at((0, 0).into()).is_some());
        let stats = render.raster_stats();
        assert_eq!(
            (stats.encodes, stats.cache_hits, stats.tiles_sent),
            (2, 2, 4)
        );
    }

    #[test]
    fn hiding_and_showing_restores_underlay_then_reuses_encodings() {
        let mut render = renderer(verified(RasterPolicy::Auto), Vec::new());
        frame(&mut render);
        render.thing.visible = false;
        let hidden = events(&frame(&mut render));
        assert_clear_before_paint(&hidden);
        assert!(tiles(&hidden).is_empty());
        assert_print(&hidden, (0, 0), '.');
        assert_print(&hidden, (1, 0), '.');
        assert!(render.buffer.previous_raster_at((0, 0).into()).is_none());
        let before = render.raster_stats();
        assert!(events(&frame(&mut render)).is_empty());
        assert_eq!(render.raster_stats(), before);
        render.thing.visible = true;
        let shown = events(&frame(&mut render));
        assert_eq!(tiles(&shown).len(), 2);
        assert_eq!(render.raster_stats().encodes, 2);
        assert_eq!(render.raster_stats().cache_hits, 2);
    }

    #[test]
    fn shrinking_target_erases_old_cells_and_changes_sampling_cache_key() {
        let mut render = renderer(verified(RasterPolicy::Auto), Vec::new());
        frame(&mut render);
        render.thing.target = (1, 1).into();
        let shrunk = events(&frame(&mut render));
        assert_clear_before_paint(&shrunk);
        assert_print(&shrunk, (1, 0), '.');
        assert_eq!(tiles(&shrunk).len(), 1);
        assert_eq!(tiles(&shrunk)[0].0, (0, 0).into());
        assert_eq!(render.raster_stats().encodes, 3);
        assert_eq!(render.raster_stats().tiles_sent, 3);
        assert!(render.buffer.previous_raster_at((1, 0).into()).is_none());
        assert_eq!(
            render
                .buffer
                .previous_raster_at((0, 0).into())
                .unwrap()
                .target,
            (1, 1).into()
        );
    }

    #[test]
    fn resize_crops_original_target_and_restores_cached_cells_on_growth() {
        let mut render = renderer(verified(RasterPolicy::Auto), Vec::new());
        let initial = events(&frame(&mut render));
        render.resize((1, 1).into());
        let cropped = events(&frame(&mut render));
        assert_clear_before_paint(&cropped);
        assert_eq!(tiles(&cropped), [((0, 0).into(), tiles(&initial)[0].1)]);
        assert_eq!(
            render
                .buffer
                .previous_raster_at((0, 0).into())
                .unwrap()
                .target,
            (2, 1).into()
        );
        render.resize((4, 2).into());
        let grown = events(&frame(&mut render));
        assert_clear_before_paint(&grown);
        assert_eq!(tiles(&grown), tiles(&initial));
        let stats = render.raster_stats();
        assert_eq!(
            (stats.encodes, stats.cache_hits, stats.tiles_sent),
            (2, 3, 5)
        );
    }

    #[test]
    fn physical_metric_changes_reencode_and_invalid_dimensions_fall_back() {
        let mut render = renderer(verified(RasterPolicy::Auto), Vec::new());
        frame(&mut render);
        let mut config = verified(RasterPolicy::Auto);
        config.capabilities.cell_pixels = Some(CellPixels {
            width: 10,
            height: 20,
        });
        render.set_raster_output(config);
        let resized = events(&frame(&mut render));
        assert_clear_before_paint(&resized);
        assert_eq!(tiles(&resized).len(), 2);
        for (_, bytes) in tiles(&resized) {
            assert_bounded_tile(bytes, 10, 20);
        }
        assert_eq!(render.raster_stats().encodes, 4);
        let before = render.raster_stats();
        assert!(events(&frame(&mut render)).is_empty());
        assert_eq!(render.raster_stats(), before);

        config.capabilities.cell_pixels = Some(CellPixels {
            width: 10,
            height: 0,
        });
        render.set_raster_output(config);
        let fallback = events(&frame(&mut render));
        assert_clear_before_paint(&fallback);
        assert!(tiles(&fallback).is_empty());
        assert_print(&fallback, (0, 0), '▀');
        assert_eq!(render.raster_stats(), before);
    }

    #[test]
    fn switching_sixel_text_only_sixel_clears_graphics_and_reuses_cache() {
        let mut render = renderer(verified(RasterPolicy::Auto), Vec::new());
        let initial = events(&frame(&mut render));
        render.set_raster_output(RasterOutput::text_only());
        let text = events(&frame(&mut render));
        assert_clear_before_paint(&text);
        assert!(tiles(&text).is_empty());
        assert_print(&text, (0, 0), '▀');
        assert_print(&text, (1, 0), '▀');
        assert_eq!(render.raster_stats().tiles_sent, 2);
        render.set_raster_output(verified(RasterPolicy::Auto));
        let native = events(&frame(&mut render));
        assert_eq!(tiles(&native), tiles(&initial));
        assert!(!native
            .iter()
            .any(|event| matches!(event, Event::Print(_, '▀'))));
        let stats = render.raster_stats();
        assert_eq!(
            (stats.encodes, stats.cache_hits, stats.tiles_sent),
            (2, 2, 4)
        );
    }

    #[test]
    fn strict_rejection_after_native_frame_does_not_forget_old_graphics() {
        let mut render = renderer(verified(RasterPolicy::Auto), Vec::new());
        frame(&mut render);
        let before = render.raster_stats();
        render.set_raster_output(RasterOutput {
            policy: RasterPolicy::RequireSixel,
            ..Default::default()
        });
        assert_eq!(
            render.render().unwrap_err().kind(),
            io::ErrorKind::Unsupported
        );
        assert!(render.out.is_empty());
        assert_no_raster_commit(&render, before);
        assert_eq!(render.raster_stats(), before);
        render.set_raster_output(RasterOutput::text_only());
        let fallback = events(&frame(&mut render));
        assert_clear_before_paint(&fallback);
        assert!(tiles(&fallback).is_empty());
        assert_print(&fallback, (0, 0), '▀');
    }

    fn assert_bounded_tile(bytes: &[u8], width: usize, height: usize) {
        let header = format!("\x1bP0;1;0q\"1;1;{width};{height}");
        assert!(bytes.starts_with(header.as_bytes()));
        assert!(bytes.ends_with(b"\x1b\\"));
        assert!(!bytes.ends_with(b"-\x1b\\"));
        assert!(bytes.len() <= 1024 * 1024);
        let body = &bytes[header.len()..bytes.len() - 2];
        let (mut index, mut x, mut y) = (0, 0, 0);
        let mut painted = vec![false; width * height];
        while index < body.len() {
            let command = body[index];
            index += 1;
            match command {
                b'#' => {
                    while index < body.len()
                        && (body[index].is_ascii_digit() || body[index] == b';')
                    {
                        index += 1;
                    }
                }
                b'$' => x = 0,
                b'-' => {
                    x = 0;
                    y += 6;
                    assert!(y < height);
                }
                b'!' | b'?'..=b'~' => {
                    let (count, character) = if command == b'!' {
                        let start = index;
                        while body[index].is_ascii_digit() {
                            index += 1;
                        }
                        let count = std::str::from_utf8(&body[start..index])
                            .unwrap()
                            .parse::<usize>()
                            .unwrap();
                        let character = body[index];
                        index += 1;
                        (count, character)
                    } else {
                        (1, command)
                    };
                    assert!((b'?'..=b'~').contains(&character));
                    assert!(count > 0 && x + count <= width);
                    for column in x..x + count {
                        for bit in 0..6 {
                            if (character - b'?') & (1 << bit) != 0 {
                                assert!(y + bit < height, "Sixel paints outside its cell");
                                painted[(y + bit) * width + column] = true;
                            }
                        }
                    }
                    x += count;
                }
                _ => panic!("unexpected tile command {command}"),
            }
        }
        assert!(painted.into_iter().all(|pixel| pixel));
        assert_eq!(y, (height - 1) / 6 * 6);
    }

    #[test]
    fn bottom_row_tiles_are_cell_bounded_and_restore_the_saved_cursor() {
        for height in 1..=25 {
            let mut config = verified(RasterPolicy::RequireSixel);
            config.capabilities.cell_pixels = Some(CellPixels { width: 10, height });
            let mut render = renderer(config, Vec::new());
            render.pos = (4, 3).into();
            render.thing.origin = (2, 1).into();
            render.thing.target = (2, 1).into();
            let output = frame(&mut render);
            assert!(!output.contains(&b'\n'));
            let trace = events(&output);
            let encoded = tiles(&trace);
            assert_eq!(
                encoded
                    .iter()
                    .map(|(position, _)| *position)
                    .collect::<Vec<_>>(),
                [(6, 4).into(), (7, 4).into()]
            );
            for (_, bytes) in encoded {
                assert_bounded_tile(bytes, 10, usize::from(height));
            }
            let saved = trace
                .iter()
                .find_map(|event| match event {
                    Event::Save(position) => Some(*position),
                    _ => None,
                })
                .unwrap();
            assert_eq!(trace.last(), Some(&Event::Restore(saved)));
            assert_eq!(
                trace
                    .iter()
                    .filter(|event| matches!(event, Event::Save(_)))
                    .count(),
                1
            );
            assert_eq!(
                trace
                    .iter()
                    .filter(|event| matches!(event, Event::Restore(_)))
                    .count(),
                1
            );
        }
    }

    #[test]
    fn ten_by_twenty_cells_remain_bounded_when_moving_and_occluding() {
        let mut config = verified(RasterPolicy::RequireSixel);
        config.capabilities.cell_pixels = Some(CellPixels {
            width: 10,
            height: 20,
        });
        let mut render = renderer(config, Vec::new());
        render.thing.target = (2, 2).into();
        let initial = events(&frame(&mut render));
        let initial_tiles = tiles(&initial);
        assert_eq!(initial_tiles.len(), 4);
        for (index, (position, bytes)) in initial_tiles.iter().enumerate() {
            assert_eq!(
                *position,
                Position::from(((index % 2) as u16, (index / 2) as u16))
            );
            assert_bounded_tile(bytes, 10, 20);
        }
        render.thing.origin = (1, 0).into();
        let moved = frame(&mut render);
        assert_synchronized(&moved);
        let trace = events(&moved);
        assert_clear_before_paint(&trace);
        for ((position, bytes), (old_position, old_bytes)) in
            tiles(&trace).iter().zip(&initial_tiles)
        {
            assert_eq!(
                *position,
                Position::from((old_position.x + 1, old_position.y))
            );
            assert_eq!(bytes, old_bytes);
        }
        assert_eq!(tiles(&trace).len(), 4);
        assert_eq!(render.raster_stats().encodes, 4);
        assert_eq!(render.raster_stats().cache_hits, 4);

        // A lower-row text cell must not be painted by the upper tile's padding.
        render.thing.text.push(((1, 1).into(), '!'));
        let covered = events(&frame(&mut render));
        assert_clear_before_paint(&covered);
        assert_print(&covered, (1, 1), '!');
        assert_eq!(tiles(&covered).len(), 3);
        for (position, bytes) in tiles(&covered) {
            assert_ne!(position, Position::from((1, 1)));
            assert_bounded_tile(bytes, 10, 20);
        }
        render.thing.visible = false;
        let hidden = events(&frame(&mut render));
        assert_clear_before_paint(&hidden);
        assert!(tiles(&hidden).is_empty());
        assert_print(&hidden, (1, 0), '.');
        assert_print(&hidden, (1, 1), '!');
    }

    #[derive(Clone, Default)]
    struct FaultWriter {
        bytes: Vec<u8>,
        remaining: Option<usize>,
        persistent: bool,
        fail_flush: bool,
    }

    impl Write for FaultWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if let Some(remaining) = self.remaining {
                if remaining == 0 {
                    if !self.persistent {
                        self.remaining = None;
                    }
                    return Err(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        "injected write failure",
                    ));
                }
                let count = remaining.min(bytes.len());
                self.bytes.extend_from_slice(&bytes[..count]);
                self.remaining = Some(remaining - count);
                return Ok(count);
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            if std::mem::take(&mut self.fail_flush) {
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "injected flush failure",
                ));
            }
            Ok(())
        }
    }

    #[test]
    fn every_partial_native_write_releases_sync_and_retry_repaints() {
        let mut probe = renderer(verified(RasterPolicy::Auto), FaultWriter::default());
        probe.render().unwrap();
        for offset in 0..probe.out.bytes.len() {
            let mut render = renderer(verified(RasterPolicy::Auto), FaultWriter::default());
            render.out.remaining = Some(offset);
            assert_eq!(
                render.render().unwrap_err().kind(),
                io::ErrorKind::BrokenPipe
            );
            assert_no_raster_commit(&render, RasterStats::default());
            assert!(
                contains(&render.out.bytes, b"\x18\x1b\\\x1b[?2026l"),
                "release sync after any recoverable write failure, offset {offset}"
            );
            render.out.bytes.clear();
            render.render().unwrap();
            assert!(render.out.bytes.starts_with(b"\x18\x1b\\\x1b[?2026l"));
            assert!(render.out.bytes.ends_with(b"\x1b[?2026l"));
            let trace = events(&render.out.bytes);
            assert_clear_before_paint(&trace);
            assert_eq!(tiles(&trace).len(), 2);
        }
    }

    #[test]
    fn write_and_flush_failures_do_not_commit_and_retry_clears_and_reuses_cache() {
        for previous_frame in [false, true] {
            for failure in 0..3 {
                let mut render = renderer(verified(RasterPolicy::Auto), FaultWriter::default());
                if previous_frame {
                    render.render().unwrap();
                    render.out.bytes.clear();
                    render.thing.canvas.set_pixel(0, 0, Rgb(0, 255, 0));
                }
                let before = render.raster_stats();
                let mut probe = render.clone();
                probe.render().unwrap();
                let dcs = probe
                    .out
                    .bytes
                    .windows(2)
                    .position(|bytes| bytes == b"\x1bP")
                    .unwrap();
                match failure {
                    0 => render.out.remaining = Some(0),
                    1 => render.out.remaining = Some(dcs + 25),
                    _ => render.out.fail_flush = true,
                }
                assert_eq!(
                    render.render().unwrap_err().kind(),
                    io::ErrorKind::BrokenPipe
                );
                assert_no_raster_commit(&render, before);
                assert!(contains(&render.out.bytes, b"\x18\x1b\\\x1b[?2026l"));
                let failed_stats = render.raster_stats();
                assert_eq!(
                    failed_stats.encodes,
                    before.encodes + 2,
                    "encoding is cache work, not a successful display commit"
                );
                render.out.remaining = None;
                render.out.bytes.clear();
                render.render().unwrap();
                let retry = events(&render.out.bytes);
                assert_clear_before_paint(&retry);
                assert_eq!(tiles(&retry).len(), 2);
                let after = render.raster_stats();
                assert_eq!(after.encodes, failed_stats.encodes);
                assert_eq!(after.cache_hits, failed_stats.cache_hits + 2);
                assert_eq!(after.tiles_sent, before.tiles_sent + 2);
                assert!(render.buffer.previous().is_some());
                render.out.bytes.clear();
                render.render().unwrap();
                assert!(events(&render.out.bytes).is_empty());
                assert_eq!(render.raster_stats(), after);
            }
        }
    }

    #[test]
    fn persistent_mid_dcs_failure_is_terminated_before_retry_clear_commands() {
        let mut render = renderer(verified(RasterPolicy::Auto), FaultWriter::default());
        let mut probe = render.clone();
        probe.render().unwrap();
        let dcs = probe
            .out
            .bytes
            .windows(2)
            .position(|bytes| bytes == b"\x1bP")
            .unwrap();
        render.out.remaining = Some(dcs + 25);
        render.out.persistent = true;
        assert_eq!(
            render.render().unwrap_err().kind(),
            io::ErrorKind::BrokenPipe
        );
        assert_no_raster_commit(&render, RasterStats::default());
        assert!(contains(&render.out.bytes, b"\x1bP"));
        assert!(!contains(&render.out.bytes[dcs..], b"\x1b\\"));
        let retry_start = render.out.bytes.len();
        render.out.remaining = None;
        render.render().unwrap();
        let retry = &render.out.bytes[retry_start..];
        assert!(retry.starts_with(b"\x18\x1b\\\x1b[?2026l\x1b[?2026h"));
        assert!(retry.ends_with(b"\x1b[?2026l"));
        let clear = retry
            .windows(4)
            .position(|bytes| bytes == b"\x1b[2J")
            .expect("retry must erase possible graphics");
        let terminator = retry.windows(2).position(|bytes| bytes == b"\x1b\\");
        let cancel = retry.iter().position(|byte| matches!(byte, 0x18 | 0x1a));
        assert!(terminator.is_some_and(|position| position < clear) || cancel.is_some_and(|position| position < clear),
            "an unterminated failed DCS must be terminated/cancelled before retry CSI commands; otherwise erase is not reliable");
        let trace = events(retry);
        assert_clear_before_paint(&trace);
        assert_eq!(tiles(&trace).len(), 2);
        assert_eq!(render.raster_stats().tiles_sent, 2);
    }
}
