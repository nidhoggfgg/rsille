use super::*;
use crate::render::raster::{RasterCanvas, RasterCell, RasterOptions, RasterScale, Rgb};
use crate::render::raster_output::{CellPixels, RasterCapabilities, RasterOutput, RasterStats};
use crate::render::Update;
use std::io::{self, Write};

#[derive(Clone)]
struct Scene {
    canvas: RasterCanvas,
    origin: Position,
    target: Size,
    visible: bool,
    underlay: char,
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
            underlay: '.',
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
                chunk.set(x, y, Stylized::plain(self.underlay))?;
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

fn verified() -> RasterOutput {
    RasterOutput {
        capabilities: RasterCapabilities {
            sixel: true,
            reliable_erase: true,
            reliable_inline_erase: false,
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

fn inline_verified() -> RasterOutput {
    let mut config = verified();
    config.capabilities.reliable_inline_erase = true;
    config.capabilities.reliable_erase = false;
    config
}

fn inline_renderer<W: Write>(config: RasterOutput, writer: W) -> Render<W, Scene> {
    Builder::new()
        .inline_mode(true)
        .size((4, 2))
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
fn frames_without_raster_work_with_unknown_capabilities() {
    for inline in [false, true] {
        for append_newline in [false, true] {
            let mut render = renderer(RasterOutput::default(), Vec::new());
            render.inline_mode = inline;
            render.append_newline = append_newline;
            render.thing.visible = false;
            let output = frame(&mut render);
            assert!(!contains(&output, b"P"));
            assert!(!contains(&output, b"[16t"));
            assert!(render.buffer.previous().is_some());
            assert_eq!(render.raster_stats(), RasterStats::default());
        }
    }
}

#[test]
fn strict_unsupported_frames_write_nothing_and_do_not_commit() {
    let mut unsupported = vec![RasterOutput::default()];
    for missing in 0..6 {
        let mut config = verified();
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
            4 => {
                config.capabilities.cell_pixels = Some(CellPixels {
                    width: 8,
                    height: 1025,
                })
            }
            _ => config.capabilities.cell_pixels = None,
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
fn absolute_requires_alternate_screen_and_inline_requires_its_own_erase_assertion() {
    for inline in [false, true] {
        let mut render = renderer(verified(), Vec::new());
        render.inline_mode = inline;
        render.set_alternate_screen(inline);

        assert_eq!(
            render.render().unwrap_err().kind(),
            io::ErrorKind::Unsupported
        );
        assert!(render.out.is_empty());
        assert!(render.buffer.previous().is_none());

        assert_eq!(render.raster_stats(), RasterStats::default());
    }
}

fn newline_renderer() -> Render<Vec<u8>, Scene> {
    Builder::new()
        .size((4, 2))
        .clear(true)
        .enable_alt_screen()
        .append_newline(true)
        .raster_output(verified())
        .build_render(Scene::default(), Vec::new())
}

#[test]
fn unknown_capabilities_reject_inline_raster_without_output() {
    let mut render = inline_renderer(RasterOutput::default(), Vec::new());
    assert_eq!(
        render.render().unwrap_err().kind(),
        io::ErrorKind::Unsupported
    );
    assert!(render.out.is_empty());
    assert!(render.buffer.previous().is_none());
}

#[test]
fn append_newline_strict_rejects_before_any_output_or_commit() {
    let mut render = newline_renderer();
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
fn disabled_feature_rejects_even_when_verified() {
    let mut render = renderer(verified(), Vec::new());
    assert_eq!(
        render.render().unwrap_err().kind(),
        io::ErrorKind::Unsupported
    );
    assert!(render.out.is_empty());
    assert!(render.buffer.previous().is_none());
    assert_eq!(render.raster_stats(), RasterStats::default());
}

#[test]
fn inline_missing_erase_or_appended_newline_rejects() {
    for append_newline in [false, true] {
        let mut config = inline_verified();
        if !append_newline {
            // Even verified ED 2 cannot substitute for inline EL 2.
            config.capabilities.reliable_erase = true;
            config.capabilities.reliable_inline_erase = false;
        }
        let mut render = inline_renderer(config, Vec::new());
        render.append_newline = append_newline;

        assert_eq!(
            render.render().unwrap_err().kind(),
            io::ErrorKind::Unsupported
        );
        assert!(render.out.is_empty());
        assert!(render.buffer.previous().is_none());
        assert!(!render.buffer.has_raster());

        assert_eq!(render.raster_stats(), RasterStats::default());
        assert_eq!(render.inline_raster_checkpoint, None);
    }
}

#[cfg(not(feature = "sixel"))]
#[test]
fn inline_disabled_feature_rejects_even_with_inline_erase() {
    let mut render = inline_renderer(inline_verified(), Vec::new());

    assert_eq!(
        render.render().unwrap_err().kind(),
        io::ErrorKind::Unsupported
    );
    assert!(render.out.is_empty());
    assert!(render.buffer.previous().is_none());
    assert!(!render.buffer.has_raster());

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
        Newline(Position),
        ClearLine(Position),
    }

    // Parse terminal framing separately from Sixel data: palette digits and
    // printable sixel bytes must never be mistaken for ordinary text output.
    fn events(bytes: &[u8]) -> Vec<Event> {
        events_at(bytes, &mut Position::default(), &mut None, false)
    }

    fn events_at(
        bytes: &[u8],
        position: &mut Position,
        anchor: &mut Option<Position>,
        inline: bool,
    ) -> Vec<Event> {
        events_in(bytes, position, anchor, inline, None)
    }

    fn events_in(
        bytes: &[u8],
        position: &mut Position,
        anchor: &mut Option<Position>,
        inline: bool,
        mut terminal: Option<&mut BandTerminal>,
    ) -> Vec<Event> {
        let mut events = Vec::new();
        let mut cursor = *position;
        let mut saved = *anchor;
        let mut index = 0;
        while index < bytes.len() {
            // CAN/SUB cancel protocol state without painting or moving the cursor.
            if matches!(bytes[index], 0x18 | 0x1a) {
                index += 1;
                continue;
            }
            if inline && matches!(bytes[index], b'\r' | b'\n') {
                if bytes[index] == b'\r' {
                    cursor.x = 0;
                } else {
                    cursor.y += 1;
                    if let Some(terminal) = terminal.as_deref_mut() {
                        if cursor.y == terminal.rows {
                            terminal.scroll(1);
                            terminal.lf_scrolls += 1;
                            cursor.y -= 1;
                        }
                    }
                    events.push(Event::Newline(cursor));
                }
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
                if let Some(terminal) = terminal.as_deref_mut() {
                    terminal.text
                        [usize::from(cursor.y) * terminal.columns + usize::from(cursor.x)] =
                        character;
                }
                cursor.x += 1;
                index += character.len_utf8();
                continue;
            }
            let start = index;
            index += 2;
            match bytes[start + 1] {
                b'P' => {
                    let tile_position = cursor;
                    let body = index;
                    loop {
                        assert!(index < bytes.len(), "terminated or cancelled DCS");
                        if matches!(bytes[index], 0x18 | 0x1a) {
                            // Execute even a cancelled prefix: pixels/scrolling are immediate.
                            if let Some(terminal) = terminal.as_deref_mut() {
                                terminal.sixel(&bytes[body..index], &mut cursor);
                            }
                            // A cancelled partial tile is not a complete Tile event.
                            index += 1;
                            break;
                        }
                        if bytes[index..].starts_with(b"\x1b\\") {
                            if let Some(terminal) = terminal.as_deref_mut() {
                                terminal.sixel(&bytes[body..index], &mut cursor);
                            }
                            index += 2;
                            events.push(Event::Tile(tile_position, bytes[start..index].to_vec()));
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
                    cursor = saved.expect("cursor save before restore");
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
                            assert!(!inline, "inline output must not use absolute CUP/HVP");
                            cursor = (
                                values.get(1).copied().unwrap_or(1).max(1) - 1,
                                values[0].max(1) - 1,
                            )
                                .into()
                        }
                        b'J' => {
                            assert!(!inline, "inline output must not erase the screen");
                            assert_eq!(values, [2], "only verified ED 2 erases graphics");
                            if let Some(terminal) = terminal.as_deref_mut() {
                                terminal.pixels.fill(false);
                                terminal.text.fill(' ');
                            }
                            events.push(Event::Clear);
                        }
                        b'E' | b'F' => {
                            let count = values[0].max(1);
                            cursor.y = if command == b'E' {
                                cursor.y + count
                            } else {
                                cursor.y.saturating_sub(count)
                            };
                            if let Some(terminal) = terminal.as_deref() {
                                cursor.y = cursor.y.min(terminal.rows - 1);
                            }
                            cursor.x = 0;
                        }
                        b'G' => cursor.x = values[0].max(1) - 1,
                        b'K' => {
                            assert_eq!(values, [2], "inline graphics require EL 2");
                            if let Some(terminal) = terminal.as_deref_mut() {
                                let start =
                                    usize::from(cursor.y * terminal.cell.height) * terminal.columns;
                                let end =
                                    start + usize::from(terminal.cell.height) * terminal.columns;
                                terminal.pixels[start..end].fill(false);
                            }
                            events.push(Event::ClearLine(cursor));
                        }
                        b's' => {
                            saved = Some(cursor);
                            events.push(Event::Save(cursor));
                        }
                        b'u' => {
                            cursor = saved.expect("cursor save before restore");
                            events.push(Event::Restore(cursor));
                        }
                        _ => panic!("unexpected CSI command {}", char::from(command)),
                    }
                }
                command => panic!("unexpected escape command {}", char::from(command)),
            }
        }
        *position = cursor;
        *anchor = saved;
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
        let mut render = renderer(verified(), Vec::new());
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
        render.thing.visible = false;
        let cleanup = frame(&mut render);
        assert_synchronized(&cleanup);
        assert_clear_before_paint(&events(&cleanup));
        assert!(!contains(&cleanup, b"P"));
        assert!(!contains(&frame(&mut render), b"\x1b[?2026"));
    }

    #[test]
    fn verified_builder_allows_native_and_identical_frames_do_no_raster_work() {
        let mut render = renderer(verified(), Vec::new());
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
            .raster_output(verified())
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
        let mut render = renderer(verified(), Vec::new());
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
        let mut render = renderer(verified(), Vec::new());
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
        let mut render = renderer(verified(), Vec::new());
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
        let mut render = renderer(verified(), Vec::new());
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
        let mut render = renderer(verified(), Vec::new());
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
        let mut render = renderer(verified(), Vec::new());
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
    fn physical_metric_changes_reencode_and_invalid_dimensions_reject() {
        let mut render = renderer(verified(), Vec::new());
        frame(&mut render);
        let mut config = verified();
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
        assert_eq!(
            render.render().unwrap_err().kind(),
            io::ErrorKind::Unsupported
        );
        assert!(render.out.is_empty());
        assert_no_raster_commit(&render, before);
        assert_eq!(render.raster_stats(), before);
    }

    #[test]
    fn removing_and_restoring_raster_clears_graphics_and_reuses_cache() {
        let mut render = renderer(verified(), Vec::new());
        let initial = events(&frame(&mut render));
        render.thing.visible = false;
        let text = events(&frame(&mut render));
        assert_clear_before_paint(&text);
        assert!(tiles(&text).is_empty());
        assert_print(&text, (0, 0), '.');
        assert_print(&text, (1, 0), '.');
        assert_eq!(render.raster_stats().tiles_sent, 2);
        render.thing.visible = true;
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
        let mut render = renderer(verified(), Vec::new());
        frame(&mut render);
        let before = render.raster_stats();
        render.set_raster_output(RasterOutput::default());
        assert_eq!(
            render.render().unwrap_err().kind(),
            io::ErrorKind::Unsupported
        );
        assert!(render.out.is_empty());
        assert_no_raster_commit(&render, before);
        assert_eq!(render.raster_stats(), before);
        render.thing.visible = false;
        let cleanup = events(&frame(&mut render));
        assert_clear_before_paint(&cleanup);
        assert!(tiles(&cleanup).is_empty());
        assert_print(&cleanup, (0, 0), '.');
    }

    fn assert_bounded_tile(bytes: &[u8], width: usize, height: usize) {
        assert_padded_tile(bytes, width, height, 0);
    }

    fn assert_padded_tile(bytes: &[u8], width: usize, height: usize, padding: usize) {
        let height = height + padding;
        let header = format!("\x1bP7;1;0q\"1;1;{width};{height}");
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
        for (index, pixel) in painted.into_iter().enumerate() {
            assert_eq!(
                pixel,
                index / width >= padding,
                "transparent prefix or missing ink"
            );
        }
        assert_eq!(y, (height - 1) / 6 * 6);
    }

    #[test]
    fn bottom_row_tiles_are_cell_bounded_and_restore_the_saved_cursor() {
        for height in 1..=25 {
            let mut config = verified();
            config.capabilities.cell_pixels = Some(CellPixels { width: 10, height });
            let mut render = renderer(config, Vec::new());
            render.resize((4, 6).into());
            render.pos = (4, 3).into();
            render.thing.origin = (2, 5).into();
            render.thing.target = (2, 1).into();
            let output = frame(&mut render);
            assert!(!output.contains(&b'\n'));
            let trace = events(&output);
            let encoded = tiles(&trace);
            let shift = (0..6).find(|s| ((s + 1) * height) % 6 == 0).unwrap();
            assert_eq!(
                encoded
                    .iter()
                    .map(|(position, _)| *position)
                    .collect::<Vec<_>>(),
                [(6, 8 - shift).into(), (7, 8 - shift).into()]
            );
            for (_, bytes) in encoded {
                assert_padded_tile(bytes, 10, usize::from(height), usize::from(shift * height));
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
        let mut config = verified();
        config.capabilities.cell_pixels = Some(CellPixels {
            width: 10,
            height: 20,
        });
        let mut render = renderer(config, Vec::new());
        render.resize((4, 3).into());
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

    // A deliberately small WT-like model, not a general terminal emulator. Each
    // slot records ink in one tile column at one physical pixel row. LF scrolls
    // at the viewport bottom; CNL clamps without scrolling. Saved coordinates
    // stay viewport-relative when the screen scrolls. Sixel reserves all six
    // pixels of every band, including transparent bits, before painting it.
    struct BandTerminal {
        cursor: Position,
        saved: Option<Position>,
        rows: u16,
        columns: usize,
        cell: CellPixels,
        pixels: Vec<bool>,
        text: Vec<char>,
        lf_scrolls: usize,
        sixel_scrolls: usize,
    }

    impl BandTerminal {
        fn new(rows: u16, columns: usize, cell: CellPixels) -> Self {
            Self {
                cursor: (0, rows - 1).into(),
                saved: None,
                rows,
                columns,
                cell,
                pixels: vec![false; usize::from(rows) * usize::from(cell.height) * columns],
                text: vec![' '; usize::from(rows) * columns],
                lf_scrolls: 0,
                sixel_scrolls: 0,
            }
        }

        fn read(&mut self, bytes: &[u8]) -> Vec<Event> {
            self.read_mode(bytes, true)
        }

        fn read_mode(&mut self, bytes: &[u8], inline: bool) -> Vec<Event> {
            let (mut cursor, mut saved) = (self.cursor, self.saved);
            let trace = events_in(bytes, &mut cursor, &mut saved, inline, Some(self));
            self.cursor = cursor;
            self.saved = saved;
            assert!(cursor.y < self.rows);
            trace
        }

        fn scroll(&mut self, rows: usize) {
            let count = (rows * self.columns).min(self.text.len());
            self.text.rotate_left(count);
            let start = self.text.len() - count;
            self.text[start..].fill(' ');
            let count =
                (rows * usize::from(self.cell.height) * self.columns).min(self.pixels.len());
            self.pixels.rotate_left(count);
            let start = self.pixels.len() - count;
            self.pixels[start..].fill(false);
        }

        fn resize(&mut self, rows: u16, cell: CellPixels) {
            self.rows = rows;
            self.cell = cell;
            self.cursor.y = self.cursor.y.min(rows - 1);
            self.saved = self.saved.map(|p| (p.x, p.y.min(rows - 1)).into());
            // Resizing invalidates physical pixels; the next frame must repaint.
            self.pixels = vec![false; usize::from(rows) * usize::from(cell.height) * self.columns];
            self.text = vec![' '; usize::from(rows) * self.columns];
        }

        fn sixel(&mut self, data: &[u8], cursor: &mut Position) {
            let Some(q) = data.iter().position(|byte| *byte == b'q') else {
                return; // A writer can fail inside the DCS header.
            };
            let mut index = q + 1;
            let (mut x, mut band) = (0usize, 0i32);
            let mut origin = i32::from(cursor.y) * i32::from(self.cell.height);
            let mut segment = if data.starts_with(b"0;") { 12 } else { 6 };
            while index < data.len() {
                let command = data[index];
                index += 1;
                match command {
                    b'"' | b'#' => {
                        while index < data.len()
                            && (data[index].is_ascii_digit() || data[index] == b';')
                        {
                            index += 1;
                        }
                    }
                    b'$' => x = 0,
                    b'-' => {
                        self.reserve_band(&mut origin, band, segment);
                        segment = 6;
                        x = 0;
                        band += 6;
                    }
                    b'!' | b'?'..=b'~' => {
                        let (count, character) = if command == b'!' {
                            let start = index;
                            while index < data.len() && data[index].is_ascii_digit() {
                                index += 1;
                            }
                            if index == data.len() {
                                return; // Incomplete RLE in a cancelled write.
                            }
                            let count = std::str::from_utf8(&data[start..index])
                                .unwrap()
                                .parse::<usize>()
                                .unwrap();
                            let character = data[index];
                            index += 1;
                            (count, character)
                        } else {
                            (1, command)
                        };
                        assert!((b'?'..=b'~').contains(&character));
                        assert!(count > 0 && x + count <= usize::from(self.cell.width));
                        self.reserve_band(&mut origin, band, segment);
                        for bit in 0..6 {
                            let y = origin + band + bit;
                            if (character - b'?') & (1 << bit) != 0 && y >= 0 {
                                let column = usize::from(cursor.x);
                                assert!(column < self.columns);
                                self.pixels[y as usize * self.columns + column] = true;
                            }
                        }
                        x += count;
                    }
                    _ => panic!("unexpected Sixel command {command}"),
                }
            }
            self.reserve_band(&mut origin, band, segment);
            cursor.y = ((origin + band).max(0) / i32::from(self.cell.height)) as u16;
        }

        // Mirrors WT's _maybeFlushImageBuffer full-segment scroll test, not
        // raster attributes or the last set bit. P1=0 starts with 12px even
        // when DECGRA subsequently requests 1:1. This is a model, not WT itself.
        fn reserve_band(&mut self, origin: &mut i32, band: i32, segment: i32) {
            let page = i32::from(self.rows) * i32::from(self.cell.height);
            let mut available = page - *origin - band;
            if segment > available {
                while available < page && segment >= available {
                    self.scroll(1);
                    self.sixel_scrolls += 1;
                    available += i32::from(self.cell.height);
                    *origin -= i32::from(self.cell.height);
                }
            }
        }

        fn assert_image(&self, top: u16, width: u16, height: u16) {
            let top = usize::from(top) * usize::from(self.cell.height);
            let bottom = top + usize::from(height) * usize::from(self.cell.height);
            for (index, painted) in self.pixels.iter().enumerate() {
                let (y, x) = (index / self.columns, index % self.columns);
                assert_eq!(
                    *painted,
                    (top..bottom).contains(&y) && x < usize::from(width),
                    "incorrect ink at tile column {x}, pixel row {y}"
                );
            }
        }
    }

    fn band_config(width: u16, height: u16) -> RasterOutput {
        let mut config = inline_verified();
        config.capabilities.cell_pixels = Some(CellPixels { width, height });
        config
    }

    fn assert_band_tiles(trace: &[Event], top: u16, width: u16, height: u16, cell: CellPixels) {
        assert_eq!(
            tiles(trace).iter().map(|tile| tile.0).collect::<Vec<_>>(),
            (top..top + height)
                .flat_map(|y| (0..width).map(move |x| (x, y).into()))
                .collect::<Vec<Position>>()
        );
        for (_, bytes) in tiles(trace) {
            assert_bounded_tile(bytes, usize::from(cell.width), usize::from(cell.height));
        }
    }

    #[test]
    fn fullscreen_complete_stream_preserves_title_and_bottom_pixels() {
        for height in 1..=25 {
            let cell = CellPixels { width: 10, height };
            let mut config = verified();
            config.capabilities.cell_pixels = Some(cell);
            let mut render = renderer(config, Vec::new());
            render.thing.origin = (0, 1).into();
            render.thing.text = vec![((0, 0).into(), 'T')];
            let mut terminal = BandTerminal::new(10, 32, cell);
            for rows in [10, 7] {
                render.resize((32, rows).into());
                render.thing.target = (32, rows - 1).into();
                terminal.resize(rows, cell);
                for repaint in [false, true] {
                    if repaint {
                        render.thing.canvas.set_pixel(0, 0, Rgb(rows as u8, 0, 255));
                    }
                    let output = frame(&mut render);
                    let trace = terminal.read_mode(&output, false);
                    assert_eq!(tiles(&trace).len(), 32 * usize::from(rows - 1));
                    assert_eq!(terminal.sixel_scrolls, 0, "cell height {height}");
                    assert_eq!(terminal.lf_scrolls, 0);
                    assert_eq!(terminal.text[0], 'T');
                    terminal.assert_image(1, 32, rows - 1);
                    let saved = trace
                        .iter()
                        .find_map(|e| match e {
                            Event::Save(p) => Some(*p),
                            _ => None,
                        })
                        .unwrap();
                    assert_eq!(
                        terminal.cursor, saved,
                        "DCS cursor advance must be restored"
                    );
                    let warm = terminal.read_mode(&frame(&mut render), false);
                    assert!(tiles(&warm).is_empty());
                    terminal.assert_image(1, 32, rows - 1);
                }
            }
        }
    }

    #[test]
    fn fullscreen_legacy_complete_stream_scrolls_title_into_a_staircase() {
        let cell = CellPixels {
            width: 10,
            height: 20,
        };
        let mut config = verified();
        config.capabilities.cell_pixels = Some(cell);
        let mut render = renderer(config, Vec::new());
        render.resize((32, 11).into());
        render.thing.origin = (0, 1).into();
        render.thing.target = (32, 9).into();
        let trace = events(&frame(&mut render));
        // Recreate the former fullscreen loop with genuine, complete encoder
        // payloads. Absolute CUP and save/restore cannot undo Sixel scrolling.
        let mut legacy = b"\x1b[2J\x1b[1;1HT\x1b7".to_vec();
        for (position, tile) in tiles(&trace) {
            legacy.extend_from_slice(
                format!("\x1b[{};{}H", position.y + 1, position.x + 1).as_bytes(),
            );
            legacy.extend_from_slice(tile);
        }
        legacy.extend_from_slice(b"\x1b8");
        let mut terminal = BandTerminal::new(10, 32, cell);
        terminal.read_mode(&legacy, false);
        assert_eq!(terminal.sixel_scrolls, 32);
        assert!(!terminal.text.contains(&'T'));
        for x in 23..32 {
            let bottom = (0..200).rfind(|y| terminal.pixels[y * 32 + x]);
            assert_eq!(bottom, Some((x - 22) * 20 - 1));
        }
    }

    #[test]
    fn fullscreen_too_short_for_a_band_rejects_before_output() {
        let mut config = verified();
        config.capabilities.cell_pixels = Some(CellPixels {
            width: 10,
            height: 20,
        });
        let mut render = renderer(config, Vec::new());
        render.resize((4, 1).into());

        assert_eq!(
            render.render().unwrap_err().kind(),
            io::ErrorKind::Unsupported
        );
        assert!(render.out.is_empty());

        assert_eq!(render.raster_stats().encodes, 0);
    }

    #[test]
    fn band_bottom_legacy_sequence_reproduces_32_column_staircase() {
        for (width, height, padding) in [(10, 20, 1), (8, 18, 0), (10, 1, 6), (10, 2, 3)] {
            let config = band_config(width, height);
            let cell = config.capabilities.cell_pixels.unwrap();
            let mut render = inline_renderer(config, Vec::new());
            let trace = events_at(
                &frame(&mut render),
                &mut Position::default(),
                &mut None,
                true,
            );
            let tile = tiles(&trace)[0].1;
            assert_bounded_tile(tile, usize::from(width), usize::from(height));
            let mut terminal = BandTerminal::new(36, 32, cell);
            // The old renderer saved at the content bottom, then restored that
            // same physical row for every column. No guard row was reserved.
            let mut old = b"\r\x1b7".to_vec();
            for x in 1..=32 {
                old.extend_from_slice(format!("\x1b8\x1b[{x}G").as_bytes());
                old.extend_from_slice(tile);
            }
            terminal.read(&old);
            assert_eq!(terminal.lf_scrolls, 0);
            assert_eq!(terminal.sixel_scrolls, 32 * padding);
            if height == 20 {
                for x in 0..32 {
                    let bottom = (0..36 * 20).rfind(|y| terminal.pixels[y * 32 + x]);
                    assert_eq!(bottom, Some((4 + x) * 20 - 1), "staircase column {x}");
                }
            } else if height == 18 {
                terminal.assert_image(35, 32, 1);
            }
        }
        // Raster attributes and completely transparent bits do not suppress
        // allocation of the last band: 18 + 6 still exceeds a 20px bottom cell.
        let mut terminal = BandTerminal::new(
            3,
            1,
            CellPixels {
                width: 10,
                height: 20,
            },
        );
        terminal.read(b"\x1bP0;1;0q\"1;1;10;20?-?-?-?\x1b\\");
        assert_eq!(terminal.sixel_scrolls, 1);
        assert!(terminal.pixels.iter().all(|pixel| !pixel));
    }

    #[test]
    fn band_bottom_reserves_before_paint_and_animation_reuses_guard_rows() {
        for (width, height, padding) in [(10, 20, 1), (8, 18, 0), (10, 1, 5), (10, 2, 2)] {
            for known_viewport in [false, true] {
                let config = band_config(width, height);
                let cell = config.capabilities.cell_pixels.unwrap();
                let mut render = inline_renderer(config, Vec::new());
                render.resize((32, 2).into());
                render.thing.target = (32, 2).into();
                if known_viewport {
                    render.set_inline_viewport_height(10);
                }
                let mut terminal = BandTerminal::new(10, 32, cell);
                let first = terminal.read(&frame(&mut render));
                let top = 10 - padding - 2;
                assert_eq!(terminal.lf_scrolls, usize::from(1 + padding));
                assert_eq!(terminal.sixel_scrolls, 0);
                let save = first
                    .iter()
                    .position(|e| matches!(e, Event::Save(_)))
                    .unwrap();
                assert!(first[..save].iter().all(|e| matches!(e, Event::Newline(_))));
                assert_eq!(first[save], Event::Save((0, 9).into()));
                assert_band_tiles(&first, top, 32, 2, cell);
                terminal.assert_image(top, 32, 2);
                assert_eq!(terminal.cursor, (0, top + 1).into());
                assert_eq!(render.inline_raster_padding, padding);
                let reserved = terminal.lf_scrolls;
                for step in 0..4 {
                    if step % 2 == 1 {
                        render.thing.canvas = RasterCanvas::new(4, 3, Rgb(0, step, 255)).unwrap();
                    }
                    let trace = terminal.read(&frame(&mut render));
                    assert!(!trace.iter().any(|e| matches!(e, Event::Newline(_))));
                    if step % 2 == 1 {
                        assert_band_tiles(&trace, top, 32, 2, cell);
                    } else {
                        assert!(tiles(&trace).is_empty());
                    }
                    assert_eq!(terminal.lf_scrolls, reserved);
                    assert_eq!(terminal.sixel_scrolls, 0);
                    assert_eq!(terminal.cursor, (0, top + 1).into());
                    terminal.assert_image(top, 32, 2);
                }
            }
        }
    }

    #[test]
    fn band_bottom_resize_metrics_and_text_cleanup_keep_tiles_above_boundary() {
        let mut render = inline_renderer(band_config(10, 20), Vec::new());
        render.thing.target = (4, 2).into();
        render.set_inline_viewport_height(12);
        let mut terminal = BandTerminal::new(
            12,
            4,
            CellPixels {
                width: 10,
                height: 20,
            },
        );
        terminal.read(&frame(&mut render));
        terminal.assert_image(9, 4, 2);
        // UI shrink/growth and source scaling retain a common physical top.
        for height in [1, 3, 2] {
            render.resize((4, height).into());
            render.thing.target = (4, height).into();
            let trace = terminal.read(&frame(&mut render));
            let top = terminal.cursor.y + 1 - height;
            assert_band_tiles(&trace, top, 4, height, terminal.cell);
            terminal.assert_image(top, 4, height);
            assert_eq!(terminal.sixel_scrolls, 0);
        }
        // Change both pixel metrics and the number of guard rows in both directions.
        for (width, height, padding) in
            [(8, 18, 0), (10, 20, 1), (10, 1, 5), (10, 2, 2), (10, 20, 1)]
        {
            let config = band_config(width, height);
            let cell = config.capabilities.cell_pixels.unwrap();
            terminal.resize(12, cell);
            render.set_raster_output(config);
            let trace = terminal.read(&frame(&mut render));
            let top = terminal.cursor.y - 1;
            assert_band_tiles(&trace, top, 4, 2, cell);
            terminal.assert_image(top, 4, 2);
            assert_eq!(render.inline_raster_padding, padding);
            assert_eq!(terminal.sixel_scrolls, 0);
            let scrolls = terminal.lf_scrolls;
            terminal.read(&frame(&mut render));
            assert_eq!(terminal.lf_scrolls, scrolls);
        }
        let scrolls = terminal.lf_scrolls;
        render.thing.visible = false;
        let cleanup = terminal.read(&frame(&mut render));
        assert!(tiles(&cleanup).is_empty());
        assert!(cleanup.iter().any(|e| matches!(e, Event::ClearLine(_))));
        assert!(terminal.pixels.iter().all(|pixel| !pixel));
        assert_eq!(render.inline_raster_padding, 0);
        assert_eq!(terminal.lf_scrolls, scrolls);
        render.thing.visible = true;
        let restored = terminal.read(&frame(&mut render));
        assert_band_tiles(&restored, terminal.cursor.y - 1, 4, 2, terminal.cell);
        terminal.assert_image(terminal.cursor.y - 1, 4, 2);
        assert_eq!(terminal.sixel_scrolls, 0);
    }

    #[test]
    fn band_bottom_viewport_shrink_clamps_content_padding_and_failed_checkpoint() {
        for (height, padding) in [(20, 1), (18, 0), (1, 5), (2, 2)] {
            for failed in [false, true] {
                let config = band_config(10, height);
                let cell = config.capabilities.cell_pixels.unwrap();
                let mut render = inline_renderer(config, FaultWriter::default());
                render.resize((4, 4).into());
                render.thing.target = (4, 4).into();
                render.set_inline_viewport_height(12);
                let mut terminal = BandTerminal::new(12, 4, cell);
                render.render().unwrap();
                terminal.read(&std::mem::take(&mut render.out.bytes));
                if failed {
                    render.out.fail_flush = true;
                    assert_eq!(
                        render.render().unwrap_err().kind(),
                        io::ErrorKind::BrokenPipe
                    );
                    terminal.read(&std::mem::take(&mut render.out.bytes));
                    assert_eq!(render.inline_raster_checkpoint, Some(4 + padding));
                }
                let viewport = padding + 2;
                terminal.resize(viewport, cell);
                render.set_inline_viewport_height(viewport);
                assert_eq!(render.inline_raster_checkpoint, failed.then_some(viewport));
                assert!(render.previous_inline_height + render.inline_raster_padding <= viewport);
                render.set_used_height(u16::MAX);
                assert_eq!(render.used_height, 2);
                render.resize((4, 2).into());
                let scrolls = terminal.lf_scrolls;
                let reserve_rows = (render.used_height + padding)
                    .saturating_sub(render.previous_inline_height + render.inline_raster_padding);
                render.render().unwrap();
                let trace = terminal.read(&std::mem::take(&mut render.out.bytes));
                assert_band_tiles(&trace, 0, 4, 2, cell);
                terminal.assert_image(0, 4, 2);
                assert_eq!(terminal.cursor, (0, 1).into());
                assert_eq!(terminal.sixel_scrolls, 0);
                assert_eq!(terminal.lf_scrolls, scrolls + usize::from(reserve_rows));
                assert_eq!(render.inline_raster_checkpoint, None);
                assert_eq!(render.inline_raster_padding, padding);
                render.render().unwrap();
                let repeat = terminal.read(&std::mem::take(&mut render.out.bytes));
                assert!(tiles(&repeat).is_empty());
                assert!(!repeat.iter().any(|e| matches!(e, Event::Newline(_))));
            }
        }
    }

    #[test]
    fn band_bottom_clipped_guard_is_reserved_before_erasing_shell_rows() {
        let config = band_config(10, 20);
        let cell = config.capabilities.cell_pixels.unwrap();
        let mut render = inline_renderer(config, Vec::new());
        render.thing.target = (4, 2).into();
        render.set_inline_viewport_height(12);
        render.pos = (0, 11).into();
        let mut terminal = BandTerminal::new(12, 4, cell);
        let first = terminal.read(&frame(&mut render));
        assert_band_tiles(&first, 9, 4, 2, cell);
        terminal.assert_image(9, 4, 2);
        assert_eq!(terminal.saved, Some((0, 11).into()));
        assert_eq!(terminal.cursor, (0, 10).into());
        assert_eq!(render.pos.y, 9);
        assert_eq!(render.inline_raster_padding, 1);
        assert_eq!(terminal.lf_scrolls, 2);

        // Model shell glyph ink immediately above the owned rows. Unlike the
        // general resize helper, clipping only the guard preserves all other
        // pixels: otherwise an erroneous EL on shell row 8 would be invisible.
        let stride = usize::from(cell.height) * terminal.columns;
        let shell_ink: Vec<_> = (0..stride).map(|pixel| pixel % 3 == 0).collect();
        terminal.pixels[8 * stride..9 * stride].copy_from_slice(&shell_ink);
        terminal.rows = 11;
        terminal.pixels.truncate(11 * stride);
        terminal.saved = terminal.saved.map(|p| (p.x, p.y.min(10)).into());
        assert_eq!(terminal.cursor, (0, 10).into());
        render.set_inline_viewport_height(11);

        let scrolls = terminal.lf_scrolls;
        let trace = terminal.read(&frame(&mut render));

        assert_eq!(
            terminal.lf_scrolls,
            scrolls + 1,
            "a clipped guard needs LF, not clamped CNL"
        );
        assert_eq!(
            &trace[..2],
            &[Event::Newline((0, 10).into()), Event::Save((0, 10).into())],
            "scroll before saving the anchor or touching the old shell row"
        );
        assert_eq!(
            trace
                .iter()
                .filter_map(|event| match event {
                    Event::ClearLine(position) => Some(*position),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            [(0, 8).into(), (0, 9).into(), (0, 10).into()]
        );
        let last_clear = trace
            .iter()
            .rposition(|e| matches!(e, Event::ClearLine(_)))
            .unwrap();
        assert!(!trace[..last_clear]
            .iter()
            .any(|e| matches!(e, Event::Print(..) | Event::Tile(..))));
        assert_band_tiles(&trace, 8, 4, 2, cell);
        assert_eq!(
            &terminal.pixels[7 * stride..8 * stride],
            shell_ink.as_slice(),
            "LF must move the original shell row 8 to row 7 without erasing it"
        );
        assert!(trace.iter().all(|event| match event {
            Event::ClearLine(p) | Event::Print(p, _) | Event::Tile(p, _) => (8..=10).contains(&p.y),
            _ => true,
        }));
        for (index, painted) in terminal.pixels.iter().enumerate() {
            let row = index / stride;
            let expected = match row {
                7 => shell_ink[index % stride],
                8 | 9 => true,
                _ => false,
            };
            assert_eq!(
                *painted, expected,
                "incorrect ink after clipping at pixel {index}"
            );
        }
        assert_eq!(terminal.cursor, (0, 9).into());
        assert_eq!(terminal.sixel_scrolls, 0);
        assert_eq!(render.pos.y, 8);
        assert_eq!(render.inline_raster_padding, 1);
        let repeat = terminal.read(&frame(&mut render));
        assert!(!repeat
            .iter()
            .any(|e| matches!(e, Event::Newline(_) | Event::ClearLine(_) | Event::Tile(..))));
        assert_eq!(terminal.lf_scrolls, scrolls + 1);
        assert_eq!(
            &terminal.pixels[7 * stride..8 * stride],
            shell_ink.as_slice()
        );
    }

    #[test]
    fn band_bottom_failed_growth_keeps_scrolled_origin_on_smaller_retry() {
        let config = band_config(10, 20);
        let cell = config.capabilities.cell_pixels.unwrap();
        let mut render = inline_renderer(config, FaultWriter::default());
        render.resize((4, 6).into());
        render.set_used_height(2);
        render.thing.target = (4, 6).into();
        render.set_inline_viewport_height(10);
        render.pos = (0, 9).into();
        let mut terminal = BandTerminal::new(10, 4, cell);
        render.render().unwrap();
        let first = terminal.read(&std::mem::take(&mut render.out.bytes));
        assert_band_tiles(&first, 7, 4, 2, cell);
        terminal.assert_image(7, 4, 2);
        assert_eq!(terminal.cursor, (0, 8).into());
        assert_eq!(render.pos.y, 7);
        assert_eq!(render.inline_raster_padding, 1);
        let scrolls = terminal.lf_scrolls;

        render.set_used_height(6);
        render.out.fail_flush = true;
        assert_eq!(
            render.render().unwrap_err().kind(),
            io::ErrorKind::BrokenPipe
        );
        let failed = terminal.read(&std::mem::take(&mut render.out.bytes));

        assert_eq!(terminal.lf_scrolls, scrolls + 4);
        assert_eq!(
            failed
                .iter()
                .filter(|e| matches!(e, Event::Newline(_)))
                .count(),
            4
        );
        // The existing two rows scroll from 7/8 to 3/4; only the four newly
        // exposed rows need tile output when the source is unchanged.
        assert_band_tiles(&failed, 5, 4, 4, cell);
        terminal.assert_image(3, 4, 6);
        assert_eq!(
            terminal.cursor,
            (0, 9).into(),
            "failure restores the full-extent checkpoint"
        );
        assert_eq!(render.inline_raster_checkpoint, Some(7));
        assert_eq!(render.previous_inline_height, 7);
        assert_eq!(render.inline_raster_padding, 0);
        assert_eq!(
            render.pos.y, 3,
            "successful reservation LFs must update origin even when flush fails"
        );

        render.set_used_height(2);
        render.render().unwrap();
        let retry = terminal.read(&std::mem::take(&mut render.out.bytes));
        assert_band_tiles(&retry, 3, 4, 2, cell);
        terminal.assert_image(3, 4, 2);
        assert!(!retry.iter().any(|e| matches!(e, Event::Newline(_))));
        assert_eq!(terminal.lf_scrolls, scrolls + 4);
        assert_eq!(terminal.sixel_scrolls, 0);
        assert_eq!(terminal.cursor, (0, 4).into());
        assert_eq!(render.inline_raster_checkpoint, None);
        assert_eq!(render.previous_inline_height, 2);
        assert_eq!(render.inline_raster_padding, 1);
        assert_eq!(render.pos.y, 3);

        // Event-loop alignment only lowers origin to the viewport limit. It
        // cannot recover the four scrolled rows from the smaller retry's size.
        let max_origin = terminal.rows.saturating_sub(
            render
                .inline_used_height()
                .saturating_add(render.inline_bottom_padding()),
        );
        assert_eq!(max_origin, 7);
        render.set_inline_origin_y(render.inline_origin().y.min(max_origin));
        assert_eq!(
            render.inline_origin().y,
            3,
            "alignment must not revert origin to 7"
        );
        render.render().unwrap();
        let repeat = terminal.read(&std::mem::take(&mut render.out.bytes));
        assert!(tiles(&repeat).is_empty());
        assert!(!repeat.iter().any(|e| matches!(e, Event::Newline(_))));
        assert_eq!(terminal.lf_scrolls, scrolls + 4);
        assert_eq!(render.pos.y, 3);
        terminal.assert_image(3, 4, 2);
    }

    #[test]
    fn band_bottom_partial_dcs_and_flush_failure_retry_reuse_full_extent() {
        for (height, padding) in [(20, 1), (18, 0), (1, 5), (2, 2)] {
            for failure in 0..3 {
                let config = band_config(10, height);
                let cell = config.capabilities.cell_pixels.unwrap();
                let mut render = inline_renderer(config, FaultWriter::default());
                render.thing.target = (4, 2).into();
                render.set_inline_viewport_height(10);
                let mut probe = render.clone();
                probe.render().unwrap();
                // Fail near the end of a tile, after its last band has executed.
                let end = probe
                    .out
                    .bytes
                    .windows(2)
                    .rposition(|b| b == b"\x1b\\")
                    .unwrap();
                if failure < 2 {
                    render.out.remaining = Some(end);
                    render.out.persistent = failure == 1;
                } else {
                    render.out.fail_flush = true;
                }
                let mut terminal = BandTerminal::new(10, 4, cell);
                assert_eq!(
                    render.render().unwrap_err().kind(),
                    io::ErrorKind::BrokenPipe
                );
                assert_eq!(render.inline_raster_checkpoint, Some(2 + padding));
                assert_eq!(render.previous_inline_height, 2 + padding);
                assert_eq!(render.inline_raster_padding, 0);
                // Keep a persistent incomplete DCS together with the retry's CAN.
                let failed_len = render.out.bytes.len();
                render.out.remaining = None;
                render.render().unwrap();
                assert!(render.out.bytes[failed_len..].starts_with(b"\x18\x1b\\"));
                assert!(!render.out.bytes[failed_len..].contains(&b'\n'));
                let trace = terminal.read(&std::mem::take(&mut render.out.bytes));
                let save = trace
                    .iter()
                    .rposition(|e| matches!(e, Event::Save(_)))
                    .unwrap();
                assert_band_tiles(&trace[save..], 8 - padding, 4, 2, cell);
                terminal.assert_image(8 - padding, 4, 2);
                assert_eq!(terminal.sixel_scrolls, 0);
                assert_eq!(terminal.lf_scrolls, usize::from(1 + padding));
                assert_eq!(terminal.cursor, (0, 9 - padding).into());
                assert_eq!(render.inline_raster_checkpoint, None);
                assert_eq!(render.inline_raster_padding, padding);
                render.render().unwrap();
                let repeat = terminal.read(&std::mem::take(&mut render.out.bytes));
                assert!(tiles(&repeat).is_empty());
                assert!(!repeat.iter().any(|e| matches!(e, Event::Newline(_))));
                assert_eq!(terminal.sixel_scrolls, 0);
            }
        }
    }

    // Rows above 5 and below the owned extent represent surrounding shell text.
    // Track the saved cursor across frames, just as a real terminal does.
    struct InlineTerminal {
        cursor: Position,
        saved: Option<Position>,
    }

    impl InlineTerminal {
        fn new() -> Self {
            Self {
                cursor: (7, 5).into(),
                saved: None,
            }
        }

        fn read(&mut self, bytes: &[u8], extent: u16) -> Vec<Event> {
            let trace = events_at(bytes, &mut self.cursor, &mut self.saved, true);
            for event in &trace {
                if let Event::Print(position, _)
                | Event::Tile(position, _)
                | Event::ClearLine(position) = event
                {
                    assert!(
                        (5..5 + extent).contains(&position.y),
                        "must not overwrite/erase surrounding shell text: {event:?}"
                    );
                }
            }
            trace
        }
    }

    fn assert_inline_clear(trace: &[Event], extent: u16) {
        let cleared: Vec<_> = trace
            .iter()
            .filter_map(|event| match event {
                Event::ClearLine(position) => Some(*position),
                _ => None,
            })
            .collect();
        assert_eq!(
            cleared,
            (5..5 + extent)
                .map(|y| (0, y).into())
                .collect::<Vec<Position>>()
        );
        let last_clear = trace
            .iter()
            .rposition(|event| matches!(event, Event::ClearLine(_)))
            .unwrap();
        assert!(
            !trace[..last_clear]
                .iter()
                .any(|event| matches!(event, Event::Print(..) | Event::Tile(..))),
            "clear every owned row before repainting text or graphics"
        );
    }

    fn assert_inline_repaint(trace: &[Event], width: u16, height: u16) {
        for y in 5..5 + height {
            for x in 0..width {
                assert!(trace.iter().any(|event| matches!(event,
                    Event::Print(position, _) | Event::Tile(position, _) if *position == Position::from((x, y)))),
                    "missing repaint at ({x}, {y})");
            }
        }
    }

    #[test]
    fn inline_parser_keeps_save_for_repeated_restores_and_tracks_relative_controls() {
        let mut terminal = InlineTerminal::new();
        let trace = terminal.read(b"\r\n\x1b7\x1b[3Gx\x1b8\x1b[1F\x1b[2K\x1b8\x1b[1E.\x1b8", 3);
        assert_eq!(
            trace,
            [
                Event::Newline((0, 6).into()),
                Event::Save((0, 6).into()),
                Event::Print((2, 6).into(), 'x'),
                Event::Restore((0, 6).into()),
                Event::ClearLine((0, 5).into()),
                Event::Restore((0, 6).into()),
                Event::Print((0, 7).into(), '.'),
                Event::Restore((0, 6).into()),
            ]
        );
        assert_eq!(terminal.cursor, (0, 6).into());
        assert_eq!(terminal.saved, Some((0, 6).into()));
    }

    #[test]
    fn inline_native_reserves_before_save_ignores_absolute_origin_and_caches_repeated_frames() {
        let mut render = inline_renderer(inline_verified(), Vec::new());
        // An absolute origin would also overflow the presenter's coordinate checks.
        render.pos = (u16::MAX, u16::MAX).into();
        let mut terminal = InlineTerminal::new();
        let bytes = frame(&mut render);
        assert_synchronized(&bytes);
        let trace = terminal.read(&bytes, 2);
        assert_eq!(
            &trace[..2],
            &[Event::Newline((0, 6).into()), Event::Save((0, 6).into())]
        );
        assert_eq!(
            trace
                .iter()
                .filter(|event| matches!(event, Event::Save(_)))
                .count(),
            1
        );
        assert_eq!(
            tiles(&trace).iter().map(|tile| tile.0).collect::<Vec<_>>(),
            [(0, 5).into(), (1, 5).into()]
        );
        assert_inline_repaint(&trace, 4, 2);
        assert_print(&trace, (3, 6), '.');
        assert_eq!(trace.last(), Some(&Event::Restore((0, 6).into())));
        assert_eq!(terminal.cursor, (0, 6).into());
        assert_eq!(render.previous_inline_height, 2);
        assert_eq!(render.inline_raster_checkpoint, None);
        let stats = render.raster_stats();
        assert_eq!((stats.encodes, stats.tiles_sent), (2, 2));
        render.pos = (123, 456).into();
        for _ in 0..3 {
            let trace = terminal.read(&frame(&mut render), 2);
            assert!(!trace.iter().any(|event| matches!(
                event,
                Event::Tile(..) | Event::Print(..) | Event::ClearLine(_) | Event::Newline(_)
            )));
            assert_eq!(terminal.cursor, (0, 6).into());
            assert_eq!(render.raster_stats(), stats);
        }
    }

    #[test]
    fn inline_move_hide_show_clears_only_owned_rows_and_reuses_tiles() {
        let mut render = inline_renderer(inline_verified(), Vec::new());
        let mut terminal = InlineTerminal::new();
        let first = terminal.read(&frame(&mut render), 2);
        render.thing.origin = (1, 1).into();
        let moved = terminal.read(&frame(&mut render), 2);
        assert_inline_clear(&moved, 2);
        assert_inline_repaint(&moved, 4, 2);
        assert_print(&moved, (0, 5), '.');
        assert_eq!(
            tiles(&moved),
            [
                ((1, 6).into(), tiles(&first)[0].1),
                ((2, 6).into(), tiles(&first)[1].1)
            ]
        );
        render.thing.visible = false;
        let hidden = terminal.read(&frame(&mut render), 2);
        assert_inline_clear(&hidden, 2);
        assert_inline_repaint(&hidden, 4, 2);
        assert!(tiles(&hidden).is_empty());
        assert_print(&hidden, (1, 6), '.');
        assert_print(&hidden, (2, 6), '.');
        render.thing.visible = true;
        let shown = terminal.read(&frame(&mut render), 2);
        assert_eq!(tiles(&shown), tiles(&moved));
        assert!(!shown
            .iter()
            .any(|event| matches!(event, Event::ClearLine(_) | Event::Newline(_))));
        assert_eq!(render.raster_stats().encodes, 2);
        assert_eq!(render.raster_stats().tiles_sent, 6);
        assert_eq!(terminal.cursor, (0, 6).into());
    }

    #[test]
    fn inline_occlusion_and_raster_removal_repaint_all_survivors() {
        let mut render = inline_renderer(inline_verified(), Vec::new());
        let mut terminal = InlineTerminal::new();
        let first = terminal.read(&frame(&mut render), 2);
        render.thing.text.push(((3, 1).into(), '!'));
        let text_diff = terminal.read(&frame(&mut render), 2);
        assert_print(&text_diff, (3, 6), '!');
        assert!(tiles(&text_diff).is_empty());
        assert!(!text_diff
            .iter()
            .any(|event| matches!(event, Event::ClearLine(_))));
        render.thing.text.push(((0, 0).into(), '#'));
        let covered = terminal.read(&frame(&mut render), 2);
        assert_inline_clear(&covered, 2);
        assert_inline_repaint(&covered, 4, 2);
        assert_print(&covered, (0, 5), '#');
        assert_eq!(tiles(&covered), [((1, 5).into(), tiles(&first)[1].1)]);
        render.thing.text.pop();
        let restored = terminal.read(&frame(&mut render), 2);
        assert_eq!(tiles(&restored), [((0, 5).into(), tiles(&first)[0].1)]);
        assert!(!restored
            .iter()
            .any(|event| matches!(event, Event::ClearLine(_) | Event::Print(_, '▀'))));
        render.thing.visible = false;
        let bytes = frame(&mut render);
        assert_synchronized(&bytes);
        let text = terminal.read(&bytes, 2);
        assert_inline_clear(&text, 2);
        assert_inline_repaint(&text, 4, 2);
        assert!(tiles(&text).is_empty());
        assert_print(&text, (0, 5), '.');
        assert_print(&text, (1, 5), '.');
        assert_print(&text, (3, 6), '!');
        render.thing.visible = true;
        let restored = terminal.read(&frame(&mut render), 2);
        assert_eq!(tiles(&restored), tiles(&first));
        assert_eq!(render.raster_stats().encodes, 2);
        assert_eq!(terminal.cursor, (0, 6).into());
    }

    #[test]
    fn inline_used_height_and_resize_shrink_then_grow_from_the_new_bottom() {
        let mut render = inline_renderer(inline_verified(), Vec::new());
        render.thing.target = (2, 2).into();
        let mut terminal = InlineTerminal::new();
        let first = terminal.read(&frame(&mut render), 2);
        assert_eq!(tiles(&first).len(), 4);
        render.used_height = 1;
        let shrunk = terminal.read(&frame(&mut render), 2);
        assert_inline_clear(&shrunk, 2);
        assert_inline_repaint(&shrunk, 4, 1);
        assert_eq!(tiles(&shrunk), tiles(&first)[..2]);
        assert_eq!(terminal.cursor, (0, 5).into());
        assert_eq!(render.previous_inline_height, 1);
        render.used_height = 2;
        let grown = terminal.read(&frame(&mut render), 2);
        assert_eq!(
            &grown[..2],
            &[Event::Newline((0, 6).into()), Event::Save((0, 6).into())]
        );
        assert_eq!(tiles(&grown), tiles(&first)[2..]);
        assert_eq!(terminal.cursor, (0, 6).into());
        assert_eq!(render.previous_inline_height, 2);
        render.resize((1, 1).into());
        let cropped = terminal.read(&frame(&mut render), 2);
        assert_inline_clear(&cropped, 2);
        assert_inline_repaint(&cropped, 1, 1);
        assert_eq!(tiles(&cropped), tiles(&first)[..1]);
        assert_eq!(terminal.cursor, (0, 5).into());
        render.resize((4, 2).into());
        let grown = terminal.read(&frame(&mut render), 2);
        assert_eq!(grown.first(), Some(&Event::Newline((0, 6).into())));
        assert_inline_clear(&grown, 2);
        assert_inline_repaint(&grown, 4, 2);
        assert_eq!(tiles(&grown), tiles(&first));
        assert_eq!(terminal.cursor, (0, 6).into());
        assert_eq!(render.raster_stats().encodes, 4);
        assert_eq!(render.inline_raster_checkpoint, None);
    }

    #[test]
    fn inline_height_shrink_repaints_even_when_first_row_raster_is_unchanged() {
        let mut render = inline_renderer(inline_verified(), Vec::new());
        let mut terminal = InlineTerminal::new();
        let first = terminal.read(&frame(&mut render), 2);
        assert_inline_clear(&first, 2);
        let before = render.raster_stats();
        let raster = render.buffer.previous_raster_at((0, 0).into()).cloned();
        assert!(raster.is_some());
        assert!(render.buffer.previous_raster_at((0, 1).into()).is_none());

        render.set_used_height(1);
        assert!(
            render.buffer.previous().is_some(),
            "exercise shrink without invalidating diff"
        );
        let shrunk = terminal.read(&frame(&mut render), 2);
        assert_inline_clear(&shrunk, 2);
        assert_inline_repaint(&shrunk, 4, 1);
        assert_eq!(tiles(&shrunk), tiles(&first));
        assert_eq!(
            render.buffer.previous_raster_at((0, 0).into()),
            raster.as_ref()
        );
        assert!(!shrunk.iter().any(|event| matches!(event,
            Event::Print(position, _) | Event::Tile(position, _) if position.y != 5)));
        assert_eq!(terminal.cursor, (0, 5).into());
        assert_eq!(render.previous_inline_height, 1);
        assert_eq!(render.inline_raster_checkpoint, None);
        let after = render.raster_stats();
        assert_eq!(after.encodes, before.encodes);
        assert_eq!(after.cache_hits, before.cache_hits + 2);
        assert_eq!(after.tiles_sent, before.tiles_sent + 2);
    }

    #[test]
    fn inline_blank_growth_clears_physical_text_and_paints_spaces_despite_equal_diff() {
        let mut render = inline_renderer(inline_verified(), Vec::new());
        render.thing.underlay = ' ';
        render.set_used_height(1);
        let mut terminal = InlineTerminal::new();
        let first = terminal.read(&frame(&mut render), 1);
        assert_inline_clear(&first, 1);
        assert_eq!(render.buffer.size(), (4, 2).into());
        let previous_row = render.buffer.previous().unwrap()[4..].to_vec();
        assert!(previous_row
            .iter()
            .all(|cell| cell.content == Stylized::plain(' ')));
        let before = render.raster_stats();

        // LF moves into this existing physical row; it does not erase its tail.
        let mut physical_row = ['o', 'l', 'd', ' ', 't', 'e', 'x', 't'];
        render.set_used_height(2);
        let grown = terminal.read(&frame(&mut render), 2);
        assert_eq!(
            &grown[..2],
            &[Event::Newline((0, 6).into()), Event::Save((0, 6).into())]
        );
        let paint: Vec<_> = grown
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    Event::ClearLine(_) | Event::Print(..) | Event::Tile(..)
                )
            })
            .collect();
        let mut expected = vec![Event::ClearLine((0, 6).into())];
        expected.extend((0..4).map(|x| Event::Print((x, 6).into(), ' ')));
        assert_eq!(
            paint,
            expected.iter().collect::<Vec<_>>(),
            "only initialize the new row, clearing before painting all four spaces"
        );
        for event in &grown {
            match event {
                Event::ClearLine(position) if position.y == 6 => physical_row.fill(' '),
                Event::Print(position, character) if position.y == 6 => {
                    physical_row[position.x as usize] = *character;
                }
                _ => {}
            }
        }
        assert_eq!(
            physical_row, [' '; 8],
            "EL 2 must also erase beyond the buffer width"
        );
        assert_eq!(render.buffer.previous().unwrap()[4..], previous_row);
        assert!(tiles(&grown).is_empty());
        assert_eq!(render.raster_stats(), before);
        assert_eq!(terminal.cursor, (0, 6).into());
        assert_eq!(render.previous_inline_height, 2);
    }

    #[test]
    fn inline_viewport_shrink_clamps_anchor_and_repaints_distinct_physical_rows() {
        let assert_rows = |trace: &[Event], height: u16| {
            let mut cleared = Vec::new();
            let mut painted = Vec::new();
            for event in trace {
                match event {
                    Event::ClearLine(position) => cleared.push((position.x, position.y)),
                    Event::Print(position, _) | Event::Tile(position, _) => {
                        painted.push((position.y, position.x));
                    }
                    Event::Save(position) | Event::Restore(position) | Event::Newline(position) => {
                        assert!(
                            position.y < height,
                            "cursor outside physical viewport: {event:?}"
                        );
                    }
                    Event::Clear => panic!("inline output must not clear the screen"),
                }
            }
            assert_eq!(cleared, (0..height).map(|y| (0, y)).collect::<Vec<_>>());
            painted.sort_unstable();
            assert_eq!(
                painted,
                (0..height)
                    .flat_map(|y| (0..4).map(move |x| (y, x)))
                    .collect::<Vec<_>>(),
                "paint each cell exactly once, with no rows collapsed onto the top row"
            );
        };
        for failed_frame in [false, true] {
            let mut render = inline_renderer(inline_verified(), FaultWriter::default());
            render.resize((4, 4).into());
            render.set_inline_viewport_height(4);
            render.thing.target = (2, 2).into();
            let mut cursor = Position::default();
            let mut saved = None;
            assert_eq!(render.pos, Position::default());
            render.render().unwrap();
            let first = events_at(&render.out.bytes, &mut cursor, &mut saved, true);
            assert_rows(&first, 4);
            assert_eq!(cursor, (0, 3).into());
            render.out.bytes.clear();
            if failed_frame {
                render.out.fail_flush = true;
                assert_eq!(
                    render.render().unwrap_err().kind(),
                    io::ErrorKind::BrokenPipe
                );
                events_at(&render.out.bytes, &mut cursor, &mut saved, true);
                assert_eq!(render.inline_raster_checkpoint, Some(4));
                render.out.bytes.clear();
            } else {
                assert!(render.buffer.previous().is_some());
            }

            // A physical resize clamps the terminal cursor, unlike a UI-only shrink.
            cursor.y = cursor.y.min(1);
            saved = saved.map(|position| (position.x, position.y.min(1)).into());
            render.set_inline_viewport_height(2);
            assert_eq!(render.previous_inline_height, 2);
            assert_eq!(render.inline_raster_checkpoint, failed_frame.then_some(2));
            assert!(
                render.buffer.previous().is_none(),
                "setter invalidates diff before resize"
            );
            assert_eq!(render.buffer.size(), (4, 4).into());
            render.set_used_height(u16::MAX);
            assert_eq!(
                render.used_height, 2,
                "viewport, not capacity, limits used height"
            );
            render.resize((4, 2).into());
            render.render().unwrap();
            let shrunk = events_at(&render.out.bytes, &mut cursor, &mut saved, true);
            assert_rows(&shrunk, 2);
            assert_eq!(tiles(&shrunk), tiles(&first));
            assert_eq!(cursor, (0, 1).into());
            assert_eq!(render.previous_inline_height, 2);
            assert_eq!(render.inline_raster_checkpoint, None);
            assert_eq!(render.raster_stats().encodes, 4);

            // A reported zero-row viewport is normalized to one addressable row.
            cursor.y = 0;
            saved = saved.map(|position| (position.x, 0).into());
            render.set_inline_viewport_height(0);
            render.set_used_height(u16::MAX);
            assert_eq!(render.used_height, 1);
            render.out.bytes.clear();
            render.render().unwrap();
            assert_rows(
                &events_at(&render.out.bytes, &mut cursor, &mut saved, true),
                1,
            );
            render.set_used_height(0);
            render.out.bytes.clear();
            render.render().unwrap();
            let empty = events_at(&render.out.bytes, &mut cursor, &mut saved, true);
            assert_eq!(
                empty
                    .iter()
                    .filter(|event| matches!(event, Event::ClearLine(_)))
                    .collect::<Vec<_>>(),
                [&Event::ClearLine((0, 0).into())]
            );
            assert!(tiles(&empty).is_empty());
            assert_eq!(cursor, (0, 0).into());
            assert_eq!(render.previous_inline_height, 0);
            render.set_inline_viewport_height(2);
            render.set_used_height(u16::MAX);
            assert_eq!(render.used_height, 2);
            render.out.bytes.clear();
            render.render().unwrap();
            let restored = events_at(&render.out.bytes, &mut cursor, &mut saved, true);
            assert_rows(&restored, 2);
            assert_eq!(tiles(&restored), tiles(&first));
            assert_eq!(cursor, (0, 1).into());
            assert_eq!(render.raster_stats().encodes, 4);
        }
    }

    #[test]
    fn inline_io_failures_preserve_bottom_checkpoint_and_retry_locally_repaints() {
        for shrink in [false, true] {
            for failure in 0..3 {
                let mut render = inline_renderer(inline_verified(), FaultWriter::default());
                let mut terminal = InlineTerminal::new();
                if shrink {
                    render.thing.target = (2, 2).into();
                    render.render().unwrap();
                    terminal.read(&render.out.bytes, 2);
                    render.out.bytes.clear();
                    render.used_height = 1;
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
                    0 => render.out.remaining = Some(dcs + 25),
                    1 => {
                        render.out.remaining = Some(dcs + 25);
                        render.out.persistent = true;
                    }
                    _ => render.out.fail_flush = true,
                }
                assert_eq!(
                    render.render().unwrap_err().kind(),
                    io::ErrorKind::BrokenPipe
                );
                assert_no_raster_commit(&render, before);
                assert_eq!(render.inline_raster_checkpoint, Some(2));
                assert_eq!(render.previous_inline_height, 2);
                if failure != 1 {
                    assert!(contains(&render.out.bytes, b"\x18\x1b\\\x1b8\x1b[?2026l"));
                    terminal.read(&render.out.bytes, 2);
                    assert_eq!(
                        terminal.cursor,
                        (0, 6).into(),
                        "error restores the old bottom, even after shrink"
                    );
                    render.out.bytes.clear();
                } else {
                    assert!(!contains(&render.out.bytes[dcs..], b"\x1b\\"));
                }
                let failed_stats = render.raster_stats();
                let retry_start = render.out.bytes.len();
                render.out.remaining = None;
                render.render().unwrap();
                let retry_bytes = &render.out.bytes[retry_start..];
                assert!(
                    retry_bytes.starts_with(b"\x18\x1b\\\x1b[?2026l\x1b8"),
                    "cancel DCS and restore before any clear/repaint"
                );
                // For persistent failures parse the failed prefix together with retry,
                // so an uncancelled DCS cannot hide the retry's erase commands.
                let trace = terminal.read(&render.out.bytes, 2);
                let retry = if failure == 1 {
                    let save = trace
                        .iter()
                        .rposition(|event| matches!(event, Event::Save(_)))
                        .unwrap();
                    &trace[save..]
                } else {
                    trace.as_slice()
                };
                assert_inline_clear(retry, 2);
                assert_inline_repaint(retry, 4, if shrink { 1 } else { 2 });
                assert_eq!(tiles(retry).len(), 2);
                assert_eq!(terminal.cursor, (0, if shrink { 5 } else { 6 }).into());
                assert_eq!(render.inline_raster_checkpoint, None);
                assert_eq!(render.previous_inline_height, if shrink { 1 } else { 2 });
                assert_eq!(render.raster_stats().encodes, failed_stats.encodes);
                assert_eq!(
                    render.raster_stats().cache_hits,
                    failed_stats.cache_hits + 2
                );
                assert_eq!(render.raster_stats().tiles_sent, before.tiles_sent + 2);
                let stats = render.raster_stats();
                render.out.bytes.clear();
                render.render().unwrap();
                assert!(tiles(&terminal.read(&render.out.bytes, 2)).is_empty());
                assert_eq!(render.raster_stats(), stats);
            }
        }
    }

    #[test]
    fn commit_frame_accounts_for_bands_and_forgets_only_visible_raster() {
        for (height, padding) in [(18, 0), (20, 1), (1, 5), (2, 2)] {
            let mut render = inline_renderer(band_config(8, height), Vec::new());
            render.set_inline_viewport_height(12);
            render.render().unwrap();
            let stats = render.raster_stats();
            assert_eq!(render.inline_raster_padding, padding);
            render.out.clear();
            render.commit_frame().unwrap();
            assert_eq!(render.out, b"\r\n".repeat(usize::from(padding) + 1));
            assert_eq!(render.raster_stats(), stats);
            assert!(render.buffer.previous().is_none());
            render.out.clear();
            // Identical source in the new region must be sent again, without
            // erasing coverage belonging to the committed region.
            render.render().unwrap();
            assert!(render.raster_stats().tiles_sent > stats.tiles_sent);
            assert_eq!(render.raster_stats().encodes, stats.encodes);
            assert!(!contains(&render.out, b"\x1b[2J"));
            render.out.clear();
            render.commit_frame().unwrap();
            render.thing.visible = false;
            render.out.clear();
            render.render().unwrap();
            assert!(!contains(&render.out, b"\x1b[2K"));
            assert!(!contains(&render.out, b"\x1b[2J"));
        }
    }

    #[test]
    fn commit_frame_scrolls_past_band_padding_at_viewport_bottom() {
        for (height, padding) in [(18, 0), (20, 1), (1, 5), (2, 2)] {
            let config = band_config(8, height);
            let mut terminal = BandTerminal::new(12, 4, config.capabilities.cell_pixels.unwrap());
            let mut render = inline_renderer(config, Vec::new());
            render.set_inline_viewport_height(12);
            render.pos.y = 11;
            render.render().unwrap();
            terminal.read(&std::mem::take(&mut render.out));
            let before = terminal.lf_scrolls;
            render.commit_frame().unwrap();
            let trace = terminal.read(&std::mem::take(&mut render.out));
            assert_eq!(terminal.lf_scrolls, before + 1);
            assert_eq!(terminal.cursor, (0, 11).into());
            assert_eq!(render.pos, terminal.cursor);
            assert_eq!(terminal.sixel_scrolls, 0);
            assert_eq!(
                trace
                    .iter()
                    .filter(|event| matches!(event, Event::Newline(_)))
                    .count(),
                padding + 1
            );
            // No claim about real terminal scrollback: this model only verifies
            // cursor reservation and absence of erasure in the emitted protocol.
            assert!(!trace.iter().any(|event| matches!(event, Event::Clear)));
        }
    }

    #[test]
    fn commit_frame_write_and_flush_errors_keep_display_history() {
        for flush in [false, true] {
            let mut render = inline_renderer(band_config(8, 20), FaultWriter::default());
            render.render().unwrap();
            let stats = render.raster_stats();
            let height = render.previous_inline_height;
            render.out.bytes.clear();
            render.out.fail_flush = flush;
            render.out.remaining = (!flush).then_some(1);
            assert_eq!(
                render.commit_frame().unwrap_err().kind(),
                io::ErrorKind::BrokenPipe
            );
            assert!(render.buffer.previous().is_some());
            assert_eq!(render.previous_inline_height, height);
            assert_eq!(render.inline_raster_padding, 1);
            assert_eq!(render.raster_stats(), stats);
        }
    }

    #[test]
    fn commit_frame_refuses_uncertain_native_output() {
        let mut render = inline_renderer(band_config(8, 20), FaultWriter::default());
        render.out.fail_flush = true;
        assert!(render.render().is_err());
        render.out.bytes.clear();
        assert!(render.commit_frame().is_err());
        assert!(render.out.bytes.is_empty());
        assert!(render.raster.needs_protocol_recovery());
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
        let mut probe = renderer(verified(), FaultWriter::default());
        probe.render().unwrap();
        for offset in 0..probe.out.bytes.len() {
            let mut render = renderer(verified(), FaultWriter::default());
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
                let mut render = renderer(verified(), FaultWriter::default());
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
        let mut render = renderer(verified(), FaultWriter::default());
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
