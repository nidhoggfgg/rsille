//! Fullscreen and inline host for Unix and VT-capable Windows consoles.
//!
//! Call once, after consuming input data, with exclusive ownership of console
//! input and terminal modes. No interface bytes are written to stdout.

pub fn run(config: crate::config::Config) -> Result<Option<serde_json::Value>, String> {
    #[cfg(any(unix, windows))]
    {
        host::run(config)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = config;
        Err("the interactive CLI requires Unix or Windows".into())
    }
}

#[cfg(any(unix, windows))]
use rsille::render::raster_output::{CellPixels, RasterCapabilities, RasterOutput};

#[cfg(any(unix, windows))]
fn explicit_pixels(value: &str, grid: (u16, u16), force: bool) -> Result<CellPixels, String> {
    let (width, height) = value.split_once('x').ok_or("--cell-pixels requires WxH")?;
    let pixels = CellPixels {
        width: width.parse().map_err(|_| "invalid cell pixel width")?,
        height: height.parse().map_err(|_| "invalid cell pixel height")?,
    };
    let pixels = CellPixels::resolve(Some(pixels)).map_err(|e| e.to_string())?;
    if (pixels.width, pixels.height) == grid && !force {
        return Err("--cell-pixels matches columns x rows; provide measured physical cell pixels or confirm with --force-cell-pixels".into());
    }
    Ok(pixels)
}

// Use the core encoder rather than flattening raster ownership into placeholder
// text. Each frame is staged before touching the terminal; the host repaints its
// owned region, so no renderer history can outlive a failed terminal write.
#[cfg(any(unix, windows))]
fn raster_frame(
    buffer: &rsille::render::buffer::Buffer,
    pixels: CellPixels,
    inline: &mut Option<Inline>,
    dimensions: (u16, u16),
) -> std::io::Result<String> {
    use rsille::render::{area::Size, buffer::Buffer, chunk::Chunk, Builder, Draw, DrawErr};
    if buffer.has_unsupported_raster_composition() {
        return Err(std::io::Error::other("unsupported raster composition"));
    }
    let pixels = CellPixels::resolve(Some(pixels))?;
    let is_inline = inline.is_some();
    let padding = if is_inline && buffer.has_raster() {
        ((6 - pixels.height % 6) % 6).div_ceil(pixels.height)
    } else {
        0
    };
    let available = dimensions.1.saturating_sub(padding);
    if available == 0 || dimensions.0 == 0 {
        return Err(std::io::Error::other(
            "viewport is too short for raster content and Sixel padding",
        ));
    }
    // Clip before encoding, preserving each tile's original target and crop.
    // In particular, if clipping removes the final raster cell, use our local
    // text transport rather than the core's text-only first-frame clear path.
    let size = buffer.size();
    let mut clipped =
        Buffer::new((size.width.min(dimensions.0), size.height.min(available)).into());
    for (x, y, cell) in buffer.all_cells() {
        if x >= clipped.size().width || y >= clipped.size().height {
            continue;
        }
        if let Some(raster) = buffer.raster_at((x, y).into()) {
            clipped
                .set_raster((x, y).into(), raster.clone())
                .map_err(std::io::Error::other)?;
        } else if cell.width() > 0
            && usize::from(x) + cell.width() <= usize::from(clipped.size().width)
        {
            clipped
                .set((x, y).into(), cell.content.clone())
                .map_err(std::io::Error::other)?;
        }
    }
    let buffer = &clipped;
    struct Snapshot<'a>(&'a Buffer);
    impl Draw for Snapshot<'_> {
        fn draw(&mut self, mut target: Chunk) -> Result<Size, DrawErr> {
            for (x, y, cell) in self.0.all_cells() {
                if let Some(raster) = self.0.raster_at((x, y).into()) {
                    target.set_raster_cell(x, y, raster.clone())?;
                } else {
                    target.set_forced(x, y, cell.content.clone())?;
                }
            }
            Ok(self.0.size())
        }
    }
    let mut bytes = Vec::new();
    let mut builder = Builder::new();
    builder
        .size(buffer.size().into())
        .inline_mode(is_inline)
        .clear(!is_inline)
        .inline_max_height(buffer.size().height)
        .raster_output(RasterOutput {
            capabilities: RasterCapabilities {
                sixel: true,
                reliable_erase: !is_inline,
                reliable_inline_erase: is_inline,
                no_scroll: true,
                cell_pixels: Some(pixels),
            },
        });
    if !is_inline {
        builder.enable_alt_screen();
    }
    // Inline text frames retain the existing relative transport (and erase any
    // preceding graphics). The core's text-only first-frame clear is global.
    if is_inline && !buffer.has_raster() {
        return inline.as_mut().unwrap().present(buffer, dimensions);
    }
    let mut renderer = builder.build_render(Snapshot(buffer), &mut bytes);
    renderer.set_inline_viewport_height(dimensions.1);
    renderer.render()?;
    drop(renderer);
    let encoded = String::from_utf8(bytes).map_err(std::io::Error::other)?;
    let mut text = String::new();
    if let Some(region) = inline {
        let previous = region.rows;
        text.push_str(&region.clear(dimensions.1));
        if previous == 0 {
            text.push_str("\r\n");
        }
        let content = buffer
            .size()
            .height
            .min(dimensions.1.saturating_sub(padding));
        region.rows = content.saturating_add(padding);
        text.push_str(&encoded);
        if padding > 0 {
            use std::fmt::Write;
            write!(text, "\x1b[{padding}E").unwrap();
        }
        text.push_str("\x1b[0m\r");
    } else {
        text.push_str(&encoded);
    }
    Ok(text)
}

#[cfg(any(unix, windows))]
#[derive(Default)]
struct Inline {
    // Between frames the cursor stays on the last owned row, in column one.
    rows: u16,
}

#[cfg(any(unix, windows))]
impl Inline {
    fn height(requested: u16, available: u16) -> u16 {
        requested.max(1).min(available.saturating_sub(1).max(1))
    }

    fn write_failed(&mut self) {
        // A partial write may stop anywhere in reservation, text, or DCS data.
        // Without a known bottom anchor, moving up could erase the caller's
        // output. Restore modes/protocol state, but leave uncertain rows alone.
        self.rows = 0;
    }

    fn clear(&mut self, available: u16) -> String {
        // A shrink may have scrolled the top of our region off-screen. Never
        // move up by the old, potentially larger-than-screen height.
        let rows = self.rows.min(available.max(1));
        let mut text = String::from("\x1b[0m\r");
        for row in 0..rows {
            text.push_str("\x1b[2K");
            if row + 1 < rows {
                text.push_str("\x1b[1A");
            }
        }
        self.rows = 0;
        text
    }

    fn present(
        &mut self,
        buffer: &rsille::render::buffer::Buffer,
        dimensions: (u16, u16),
    ) -> std::io::Result<String> {
        use crossterm::{
            style::{SetAttributes, SetColors},
            Command,
        };
        use std::fmt::Write;
        let size = buffer.size();
        let (width, available) = dimensions;
        let width = size.width.min(width);
        let rows = size.height.min(available.max(1));
        let previous = self.rows.min(available.max(1));
        let mut text = self.clear(available);
        // clear leaves us on the first surviving owned row. On first entry,
        // leave the caller's current line untouched and start below it.
        if previous == 0 {
            text.push_str("\r\n");
        }
        for row in 1..rows {
            if row < previous {
                text.push_str("\r\x1b[1B");
            } else {
                text.push_str("\r\n");
            }
        }
        self.rows = rows;
        if rows > 1 {
            write!(text, "\x1b[{}A", rows - 1).unwrap();
        }
        for y in 0..rows {
            text.push_str("\r\x1b[0m\x1b[2K");
            for x in 0..width {
                if let Some(cell) = buffer.get((x, y).into()) {
                    if cell.is_empty()
                        || cell.width() == 0
                        || usize::from(x) + cell.width() > usize::from(width)
                    {
                        continue;
                    }
                    write!(text, "\x1b[{}G\x1b[0m", x + 1).unwrap();
                    if let Some(colors) = cell.style.colors {
                        SetColors(colors)
                            .write_ansi(&mut text)
                            .map_err(|_| std::io::Error::other("encode terminal colors"))?;
                    }
                    if let Some(attr) = cell.style.attr {
                        SetAttributes(attr)
                            .write_ansi(&mut text)
                            .map_err(|_| std::io::Error::other("encode terminal attributes"))?;
                    }
                    if cell.as_char() == Some('\t') {
                        text.push_str(&" ".repeat(cell.width()));
                    } else {
                        text.push_str(&cell.symbol());
                    }
                }
            }
            if y + 1 < rows {
                text.push_str("\r\x1b[1B");
            }
        }
        text.push_str("\x1b[0m\r");
        Ok(text)
    }
}

#[cfg(all(test, any(unix, windows)))]
mod inline_tests {
    use super::Inline;
    use rsille::render::buffer::Buffer;

    fn raster_buffer() -> Buffer {
        use rsille::render::raster::{RasterCanvas, RasterCell};
        let mut buffer = Buffer::new((2, 2).into());
        buffer
            .set_raster(
                (0, 0).into(),
                RasterCell {
                    canvas: RasterCanvas::new(2, 2, Default::default()).unwrap(),
                    target: (1, 1).into(),
                    x: 0,
                    y: 0,
                    options: Default::default(),
                },
            )
            .unwrap();
        buffer
    }

    #[test]
    fn explicit_metrics_are_validated_without_probing() {
        for value in ["0x20", "10x0", "1025x1", "256x257", "8", "axb"] {
            assert!(super::explicit_pixels(value, (80, 24), true).is_err());
        }
        assert!(super::explicit_pixels("80x24", (80, 24), false).is_err());
        assert!(super::explicit_pixels("80x24", (80, 24), true).is_ok());
        assert_eq!(
            super::explicit_pixels("8x18", (80, 24), false).unwrap(),
            super::CellPixels {
                width: 8,
                height: 18
            }
        );
    }

    #[test]
    fn raster_transport_preserves_metadata_and_owns_guards() {
        for height in [1, 2, 18, 20] {
            let pixels = super::CellPixels { width: 8, height };
            let padding = ((6 - height % 6) % 6).div_ceil(height);
            let mut inline = Some(Inline::default());
            let buffer = raster_buffer();
            for _ in 0..2 {
                let text = super::raster_frame(&buffer, pixels, &mut inline, (80, 24)).unwrap();
                assert!(text.contains("\x1bP"));
                assert!(text.contains("\x1b\\"));
                assert!(!text.contains("\x1b[2J"));
                assert!(!text.contains("1049"));
                assert_eq!(inline.as_ref().unwrap().rows, 2 + padding);
            }
            let cleanup = inline.as_mut().unwrap().clear(24);
            assert_eq!(cleanup.matches("\x1b[2K").count(), usize::from(2 + padding));
            let mut fullscreen = Buffer::new((2, 8).into());
            fullscreen
                .set_raster(
                    (0, 0).into(),
                    buffer.raster_at((0, 0).into()).unwrap().clone(),
                )
                .unwrap();
            let text = super::raster_frame(&fullscreen, pixels, &mut None, (80, 24)).unwrap();
            assert!(text.contains("\x1bP"));
            assert!(text.contains("\x1b[2J"));
        }
    }

    #[test]
    fn invalid_raster_frame_does_not_change_owned_rows() {
        let pixels = super::CellPixels {
            width: 8,
            height: 1,
        };
        let mut inline = Some(Inline { rows: 2 });
        let mut buffer = raster_buffer();
        assert!(super::raster_frame(&buffer, pixels, &mut inline, (80, 2)).is_err());
        assert_eq!(inline.as_ref().unwrap().rows, 2);
        buffer.mark_unsupported_raster_composition();
        assert!(super::raster_frame(&buffer, pixels, &mut inline, (80, 24)).is_err());
        assert_eq!(inline.as_ref().unwrap().rows, 2);
    }

    #[test]
    fn clipping_the_last_raster_cell_never_clears_the_screen() {
        let mut buffer = Buffer::new((2, 4).into());
        buffer
            .set_raster(
                (0, 3).into(),
                raster_buffer().raster_at((0, 0).into()).unwrap().clone(),
            )
            .unwrap();
        let mut inline = Some(Inline { rows: 6 });
        let text = super::raster_frame(
            &buffer,
            super::CellPixels {
                width: 8,
                height: 2,
            },
            &mut inline,
            (2, 4),
        )
        .unwrap();
        assert!(!text.contains("\x1b[2J"));
        assert!(!text.contains("\x1bP"));
        assert_eq!(inline.as_ref().unwrap().rows, 2);
        assert_eq!(text.matches("\x1b[2K").count(), 6);
    }

    #[test]
    fn raster_resize_clips_before_drawing_and_reserves_only_visible_rows() {
        let mut inline = Some(Inline { rows: 8 });
        let buffer = raster_buffer();
        let text = super::raster_frame(
            &buffer,
            super::CellPixels {
                width: 8,
                height: 2,
            },
            &mut inline,
            (1, 3),
        )
        .unwrap();
        assert!(text.contains("\x1bP"));
        assert!(!text.contains("\x1b[2J"));
        assert_eq!(inline.as_ref().unwrap().rows, 3);
        // One surviving content row plus two blank band guards. Reservation
        // precedes DCS and cleanup includes the guards, not the old eight rows.
        assert_eq!(text.matches("\r\n").count(), 2);
        assert!(text.find("\r\n").unwrap() < text.find("\x1bP").unwrap());
        assert_eq!(
            inline.as_mut().unwrap().clear(3).matches("\x1b[2K").count(),
            3
        );
    }

    #[test]
    fn partial_write_does_not_erase_from_an_unknown_cursor() {
        let mut inline = Inline { rows: 8 };
        inline.write_failed();
        let cleanup = inline.clear(24);
        assert!(!cleanup.contains("\x1b[1A"));
        assert!(!cleanup.contains("\x1b[2K"));
    }

    #[test]
    fn height_is_bounded_and_nonzero() {
        assert_eq!(Inline::height(8, 24), 8);
        assert_eq!(Inline::height(30, 24), 23);
        assert_eq!(Inline::height(0, 24), 1);
        assert_eq!(Inline::height(8, 1), 1);
    }

    #[test]
    fn reservation_redraw_and_cleanup_are_relative() {
        let mut inline = Inline::default();
        let buffer = Buffer::new((10, 3).into());
        let first = inline.present(&buffer, (80, 24)).unwrap();
        assert_eq!(first.matches("\r\n").count(), 3);
        let redraw = inline.present(&buffer, (80, 24)).unwrap();
        assert_eq!(redraw.matches("\r\n").count(), 0);
        for text in [&first, &redraw] {
            assert!(!text.contains("1049"));
            assert!(!text.contains("\x1b[2J"));
            assert!(!text.contains("\x1b[6n"));
            assert!(text.ends_with("\x1b[0m\r"));
        }
        let cleanup = inline.clear(24);
        assert_eq!(cleanup.matches("\x1b[2K").count(), 3);
        assert_eq!(cleanup.matches("\x1b[1A").count(), 2);
        assert!(!inline.clear(24).contains("\x1b[2K"));
    }

    #[test]
    fn shrink_bounds_cleanup_and_rendering() {
        let mut inline = Inline { rows: 12 };
        let cleanup = inline.clear(3);
        assert_eq!(cleanup.matches("\x1b[2K").count(), 3);
        assert_eq!(cleanup.matches("\x1b[1A").count(), 2);
        let buffer = Buffer::new((10, 8).into());
        let text = inline.present(&buffer, (4, 2)).unwrap();
        assert_eq!(inline.rows, 2);
        assert!(!text.contains("\x1b[5G"));
    }
}

#[cfg(windows)]
mod windows {
    use crossterm::{
        cursor::MoveTo,
        event::{Event, KeyCode, KeyEvent, KeyModifiers},
        style::{ResetColor, SetAttributes, SetColors},
        Command,
    };
    use rsille::render::buffer::Buffer;
    use std::{
        fs::{File, OpenOptions},
        io,
        os::windows::io::AsRawHandle,
        time::{Duration, Instant},
    };
    use windows_sys::Win32::{
        Foundation::{HANDLE, WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT},
        System::{Console::*, Threading::WaitForSingleObject},
    };

    fn handle(file: &File) -> HANDLE {
        file.as_raw_handle() as HANDLE
    }
    fn checked(result: i32) -> io::Result<()> {
        if result == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
    fn mode(file: &File) -> io::Result<u32> {
        let mut mode = 0;
        // SAFETY: the owned handle and output pointer are valid.
        checked(unsafe { GetConsoleMode(handle(file), &mut mode) })?;
        Ok(mode)
    }
    fn set_mode(file: &File, mode: u32) -> io::Result<()> {
        // SAFETY: the file owns a live console handle.
        checked(unsafe { SetConsoleMode(handle(file), mode) })
    }

    pub(super) struct Terminal {
        input: File,
        output: File,
        input_mode: u32,
        output_mode: u32,
        cursor: CONSOLE_CURSOR_INFO,
        raw: bool,
        vt: bool,
        screen: bool,
        inline: Option<super::Inline>,
    }
    impl Terminal {
        pub(super) fn open() -> io::Result<Self> {
            // Never use SetStdHandle: stdin is JSON and stdout is the result.
            let input = OpenOptions::new().read(true).write(true).open("CONIN$")?;
            let output = OpenOptions::new().read(true).write(true).open("CONOUT$")?;
            let input_mode = mode(&input)?;
            let output_mode = mode(&output)?;
            // SAFETY: this integer-only structure is filled through a valid pointer.
            let mut cursor = unsafe { std::mem::zeroed() };
            checked(unsafe { GetConsoleCursorInfo(handle(&output), &mut cursor) })?;
            let terminal = Self {
                input,
                output,
                input_mode,
                output_mode,
                cursor,
                raw: false,
                vt: false,
                screen: false,
                inline: None,
            };
            terminal.size()?;
            Ok(terminal)
        }
        pub(super) fn enter(&mut self, inline: bool) -> io::Result<()> {
            self.inline = inline.then(super::Inline::default);
            // Crossterm 0.28.1 opens CONIN$ for raw mode and event polling,
            // but disable_raw_mode does not restore the exact original mode.
            // Keep mode ownership and input on our own handles instead.
            set_mode(
                &self.input,
                (self.input_mode
                    & !(ENABLE_ECHO_INPUT
                        | ENABLE_LINE_INPUT
                        | ENABLE_PROCESSED_INPUT
                        | ENABLE_QUICK_EDIT_MODE
                        | ENABLE_VIRTUAL_TERMINAL_INPUT
                        | ENABLE_MOUSE_INPUT))
                    | ENABLE_EXTENDED_FLAGS
                    | ENABLE_WINDOW_INPUT,
            )?;
            self.raw = true;
            set_mode(
                &self.output,
                self.output_mode | ENABLE_PROCESSED_OUTPUT | ENABLE_VIRTUAL_TERMINAL_PROCESSING,
            )?;
            self.vt = true;
            // Core queue! must encode ANSI into the staging buffer, never use
            // WinAPI fallbacks. Crossterm 0.28 checks CONOUT$, whose VT mode we
            // have enabled and will restore, not the redirected result handle.
            if !crossterm::ansi_support::supports_ansi() {
                return Err(io::Error::other(
                    "ANSI encoding unavailable for console transport",
                ));
            }
            self.screen = true;
            if inline {
                self.write("\x1b[?7l\x1b[?25l\x1b[0m")
            } else {
                self.write("\x1b[?1049h\x1b[?7l\x1b[?25l\x1b[0m\x1b[2J\x1b[H")
            }
        }
        pub(super) fn check_connection(&self) -> io::Result<()> {
            mode(&self.input)?;
            mode(&self.output)?;
            Ok(())
        }
        pub(super) fn size(&self) -> io::Result<(u16, u16)> {
            // SAFETY: the API fills the structure through a valid pointer.
            let mut info: CONSOLE_SCREEN_BUFFER_INFO = unsafe { std::mem::zeroed() };
            checked(unsafe { GetConsoleScreenBufferInfo(handle(&self.output), &mut info) })?;
            let width = i32::from(info.srWindow.Right) - i32::from(info.srWindow.Left) + 1;
            let height = i32::from(info.srWindow.Bottom) - i32::from(info.srWindow.Top) + 1;
            if width <= 0 || height <= 0 {
                return Err(io::Error::other("console has zero dimensions"));
            }
            Ok((width as u16, height as u16))
        }
        fn write(&self, text: &str) -> io::Result<()> {
            // Unicode output without changing the global console code page.
            let wide: Vec<u16> = text.encode_utf16().collect();
            let mut remaining = wide.as_slice();
            while !remaining.is_empty() {
                let mut len = remaining.len().min(8192);
                if len < remaining.len() && (0xd800..=0xdbff).contains(&remaining[len - 1]) {
                    len -= 1;
                }
                let mut written = 0;
                // SAFETY: remaining contains at least len initialized UTF-16 units.
                checked(unsafe {
                    WriteConsoleW(
                        handle(&self.output),
                        remaining.as_ptr().cast(),
                        len as u32,
                        &mut written,
                        std::ptr::null(),
                    )
                })?;
                if written == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "console write made no progress",
                    ));
                }
                remaining = &remaining[written as usize..];
            }
            Ok(())
        }
        pub(super) fn cell_pixels(&self) -> Option<super::CellPixels> {
            let mut info: CONSOLE_FONT_INFO = unsafe { std::mem::zeroed() };
            // SAFETY: the owned console handle and output pointer are valid.
            checked(unsafe { GetCurrentConsoleFont(handle(&self.output), 0, &mut info) }).ok()?;
            Some(super::CellPixels {
                width: u16::try_from(info.dwFontSize.X).ok()?,
                height: u16::try_from(info.dwFontSize.Y).ok()?,
            })
        }
        pub(super) fn present(
            &mut self,
            buffer: &Buffer,
            pixels: Option<super::CellPixels>,
        ) -> io::Result<()> {
            if let Some(pixels) = pixels {
                let dimensions = self.size()?;
                let text = super::raster_frame(buffer, pixels, &mut self.inline, dimensions)?;
                return self.write_frame(&text);
            }
            if buffer.has_raster() || buffer.has_unsupported_raster_composition() {
                return Err(io::Error::other("raster output requires --sixel"));
            }
            if self.inline.is_some() {
                let dimensions = self.size()?;
                let text = self.inline.as_mut().unwrap().present(buffer, dimensions)?;
                return self.write_frame(&text);
            }
            let mut text = String::new();
            let size = buffer.size();
            for y in 0..size.height {
                for x in 0..size.width {
                    if let Some(cell) = buffer.get((x, y).into()) {
                        // queue!/Display/Cell::queue can probe/change VT state
                        // or select a WinAPI fallback, even for an in-memory
                        // writer. write_ansi bypasses that global dispatch.
                        ansi(&mut text, MoveTo(x, y))?;
                        ansi(&mut text, ResetColor)?;
                        let content = cell;
                        if content.is_empty() || content.width() == 0 {
                            continue;
                        }
                        if let Some(colors) = content.style.colors {
                            ansi(&mut text, SetColors(colors))?;
                        }
                        if let Some(attr) = content.style.attr {
                            ansi(&mut text, SetAttributes(attr))?;
                        }
                        if content.as_char() == Some('\t') {
                            text.push_str(&" ".repeat(content.width()));
                        } else {
                            text.push_str(&content.symbol());
                        }
                    }
                }
            }
            ansi(&mut text, ResetColor)?;
            self.write_frame(&text)
        }
        fn write_frame(&mut self, text: &str) -> io::Result<()> {
            let result = self.write(text);
            if result.is_err() {
                if let Some(region) = &mut self.inline {
                    region.write_failed();
                }
            }
            result
        }
        pub(super) fn restore(&mut self) -> io::Result<()> {
            let mut first = None;
            if self.screen {
                // A retry after partial cleanup may need VT re-enabled first.
                if let Err(e) = set_mode(
                    &self.output,
                    self.output_mode | ENABLE_PROCESSED_OUTPUT | ENABLE_VIRTUAL_TERMINAL_PROCESSING,
                ) {
                    first.get_or_insert(e);
                } else {
                    self.vt = true;
                    let cleanup = if self.inline.is_some() {
                        let height = self.size().map(|s| s.1).unwrap_or(1);
                        self.inline.as_mut().unwrap().clear(height)
                    } else {
                        String::from("\x1b[?1049l")
                    };
                    for text in [
                        "\x18\x1b\\\x1b[?2026l",
                        "\x1b[0m",
                        "\x1b[?7h",
                        "\x1b[?25h",
                        &cleanup,
                    ] {
                        if let Err(e) = self.write(text) {
                            first.get_or_insert(e);
                        }
                    }
                    // SAFETY: the saved cursor structure and owned handle are valid.
                    if let Err(e) =
                        checked(unsafe { SetConsoleCursorInfo(handle(&self.output), &self.cursor) })
                    {
                        first.get_or_insert(e);
                    }
                    if first.is_none() {
                        self.screen = false;
                    }
                }
            }
            if self.vt {
                match set_mode(&self.output, self.output_mode) {
                    Ok(()) => self.vt = false,
                    Err(e) => {
                        first.get_or_insert(e);
                    }
                }
            }
            if self.raw {
                match set_mode(&self.input, self.input_mode) {
                    Ok(()) => self.raw = false,
                    Err(e) => {
                        first.get_or_insert(e);
                    }
                }
            }
            first.map_or(Ok(()), Err)
        }
    }
    fn ansi(text: &mut String, command: impl Command) -> io::Result<()> {
        command
            .write_ansi(text)
            .map_err(|_| io::Error::other("encode terminal command"))
    }
    impl Drop for Terminal {
        fn drop(&mut self) {
            let _ = self.restore();
        }
    }

    pub(super) struct Input {
        console: File,
        decoder: KeyDecoder,
        repeated: Option<(Event, u16)>,
    }
    impl Input {
        pub(super) fn start(terminal: &Terminal) -> io::Result<Self> {
            Ok(Self {
                console: terminal.input.try_clone()?,
                decoder: KeyDecoder::default(),
                repeated: None,
            })
        }
        pub(super) fn read_event(&mut self, timeout: Duration) -> io::Result<Option<Event>> {
            if let Some((event, count)) = self.repeated.take() {
                if count > 1 {
                    self.repeated = Some((event.clone(), count - 1));
                }
                return Ok(Some(event));
            }
            let deadline = Instant::now() + timeout;
            loop {
                let remaining = deadline.saturating_duration_since(Instant::now());
                // SAFETY: the owned console handle stays live throughout the wait.
                match unsafe {
                    WaitForSingleObject(handle(&self.console), remaining.as_millis() as u32)
                } {
                    WAIT_TIMEOUT => return Ok(None),
                    WAIT_FAILED => return Err(io::Error::last_os_error()),
                    WAIT_OBJECT_0 => {}
                    _ => return Err(io::Error::other("unexpected console wait result")),
                }
                // Exclusive input ownership is required: no competing reader may
                // drain the signalled queue before this read. No detached worker.
                let mut record: INPUT_RECORD = unsafe { std::mem::zeroed() };
                let mut read = 0;
                // SAFETY: record and read point to writable initialized storage.
                checked(unsafe {
                    ReadConsoleInputW(handle(&self.console), &mut record, 1, &mut read)
                })?;
                if read != 0 && record.EventType == KEY_EVENT as u16 {
                    // SAFETY: EventType identifies the active union member.
                    let key = unsafe { record.Event.KeyEvent };
                    if let Some(event) = self.decoder.decode(key) {
                        if key.wRepeatCount > 1 {
                            self.repeated = Some((event.clone(), key.wRepeatCount - 1));
                        }
                        return Ok(Some(event));
                    }
                }
                // Resize is queried from our output handle by the shared loop.
                // Mouse reporting is disabled, matching the Unix host.
                if Instant::now() >= deadline {
                    return Ok(None);
                }
            }
        }
    }
    #[derive(Default)]
    struct KeyDecoder {
        surrogate: Option<u16>,
    }
    impl KeyDecoder {
        fn decode(&mut self, key: KEY_EVENT_RECORD) -> Option<Event> {
            // SAFETY: ReadConsoleInputW supplies the Unicode union member.
            let unit = unsafe { key.uChar.UnicodeChar };
            let alt_code = key.wVirtualKeyCode == 0x12 && key.bKeyDown == 0 && unit != 0;
            if key.bKeyDown == 0 && !alt_code {
                return None;
            }
            let state = key.dwControlKeyState;
            let mut modifiers = KeyModifiers::empty();
            if state & SHIFT_PRESSED != 0 {
                modifiers |= KeyModifiers::SHIFT;
            }
            if state & (LEFT_CTRL_PRESSED | RIGHT_CTRL_PRESSED) != 0 {
                modifiers |= KeyModifiers::CONTROL;
            }
            if state & (LEFT_ALT_PRESSED | RIGHT_ALT_PRESSED) != 0 {
                modifiers |= KeyModifiers::ALT;
            }
            // AltGr produces text, not a Ctrl+Alt shortcut.
            if unit >= 0x20 && state & RIGHT_ALT_PRESSED != 0 && state & LEFT_CTRL_PRESSED != 0 {
                modifiers.remove(KeyModifiers::CONTROL | KeyModifiers::ALT);
            }
            if alt_code {
                modifiers.remove(KeyModifiers::ALT);
            }
            if (0x60..=0x69).contains(&key.wVirtualKeyCode) && modifiers == KeyModifiers::ALT {
                return None;
            }
            let code = if alt_code {
                None
            } else {
                match key.wVirtualKeyCode {
                    0x10..=0x12 => return None,
                    0x08 => Some(KeyCode::Backspace),
                    0x09 if modifiers.contains(KeyModifiers::SHIFT) => Some(KeyCode::BackTab),
                    0x09 => Some(KeyCode::Tab),
                    0x0d => Some(KeyCode::Enter),
                    0x1b => Some(KeyCode::Esc),
                    0x21 => Some(KeyCode::PageUp),
                    0x22 => Some(KeyCode::PageDown),
                    0x23 => Some(KeyCode::End),
                    0x24 => Some(KeyCode::Home),
                    0x25 => Some(KeyCode::Left),
                    0x26 => Some(KeyCode::Up),
                    0x27 => Some(KeyCode::Right),
                    0x28 => Some(KeyCode::Down),
                    0x2d => Some(KeyCode::Insert),
                    0x2e => Some(KeyCode::Delete),
                    0x70..=0x87 => Some(KeyCode::F((key.wVirtualKeyCode - 0x6f) as u8)),
                    _ => None,
                }
            };
            let code = match code {
                Some(code) => {
                    self.surrogate = None;
                    code
                }
                None => {
                    if (0xd800..=0xdbff).contains(&unit) {
                        self.surrogate = Some(unit);
                        return None;
                    }
                    let high = self.surrogate.take();
                    let ch = if (0xdc00..=0xdfff).contains(&unit) {
                        let high = high?;
                        char::from_u32(
                            0x10000 + ((u32::from(high) - 0xd800) << 10) + u32::from(unit) - 0xdc00,
                        )?
                    } else if unit < 0x20
                        && modifiers.contains(KeyModifiers::CONTROL)
                        && (0x41..=0x5a).contains(&key.wVirtualKeyCode)
                    {
                        let ch = char::from_u32(u32::from(key.wVirtualKeyCode))?;
                        if modifiers.contains(KeyModifiers::SHIFT) {
                            ch
                        } else {
                            ch.to_ascii_lowercase()
                        }
                    } else if unit == 0
                        && modifiers.contains(KeyModifiers::CONTROL)
                        && key.wVirtualKeyCode == 0x20
                    {
                        ' '
                    } else if unit < 0x20 {
                        return None;
                    } else {
                        char::from_u32(u32::from(unit))?
                    };
                    KeyCode::Char(ch)
                }
            };
            Some(Event::Key(KeyEvent::new(code, modifiers)))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn key(vk: u16, unit: u16, state: u32) -> KEY_EVENT_RECORD {
            // SAFETY: all-zero is valid for this C integer/union structure.
            let mut key: KEY_EVENT_RECORD = unsafe { std::mem::zeroed() };
            key.bKeyDown = 1;
            key.wRepeatCount = 1;
            key.wVirtualKeyCode = vk;
            key.uChar.UnicodeChar = unit;
            key.dwControlKeyState = state;
            key
        }

        fn event(code: KeyCode, modifiers: KeyModifiers) -> Option<Event> {
            Some(Event::Key(KeyEvent::new(code, modifiers)))
        }

        #[test]
        fn shortcuts_navigation_and_release() {
            let mut decoder = KeyDecoder::default();
            assert_eq!(
                decoder.decode(key(0x43, 3, LEFT_CTRL_PRESSED)),
                event(KeyCode::Char('c'), KeyModifiers::CONTROL)
            );
            assert_eq!(
                decoder.decode(key(0x09, 9, SHIFT_PRESSED)),
                event(KeyCode::BackTab, KeyModifiers::SHIFT)
            );
            assert_eq!(
                decoder.decode(key(0x25, 0, 0)),
                event(KeyCode::Left, KeyModifiers::empty())
            );
            let mut release = key(0x0d, 13, 0);
            release.bKeyDown = 0;
            assert_eq!(decoder.decode(release), None);
        }

        #[test]
        fn unicode_altgr_and_alt_codes() {
            let mut decoder = KeyDecoder::default();
            assert_eq!(decoder.decode(key(0, 0xd83d, 0)), None);
            assert_eq!(
                decoder.decode(key(0, 0xde00, 0)),
                event(KeyCode::Char('😀'), KeyModifiers::empty())
            );
            assert_eq!(decoder.decode(key(0, 0xde00, 0)), None);
            assert_eq!(
                decoder.decode(key(0x51, '@' as u16, LEFT_CTRL_PRESSED | RIGHT_ALT_PRESSED)),
                event(KeyCode::Char('@'), KeyModifiers::empty())
            );
            let mut alt = key(0x12, 'é' as u16, 0);
            alt.bKeyDown = 0;
            assert_eq!(
                decoder.decode(alt),
                event(KeyCode::Char('é'), KeyModifiers::empty())
            );
        }

        #[test]
        fn ansi_encoding_does_not_require_standard_console_handles() {
            let mut text = String::new();
            ansi(&mut text, MoveTo(2, 3)).unwrap();
            ansi(&mut text, ResetColor).unwrap();
            assert_eq!(text, "\x1b[4;3H\x1b[0m");
        }
    }
}

#[cfg(unix)]
mod unix {
    use std::{
        fs::{File, OpenOptions},
        io::{self, Write},
        os::fd::AsRawFd,
        sync::{
            atomic::{AtomicBool, Ordering},
            mpsc::{self, Receiver, RecvTimeoutError},
            Arc,
        },
        thread::{self, JoinHandle},
        time::{Duration, Instant},
    };

    use crossterm::{
        cursor::{Hide, MoveTo},
        event::{self, EnableBracketedPaste, Event},
        execute, queue,
        style::ResetColor,
        terminal::{self, Clear, ClearType, DisableLineWrap, EnterAlternateScreen},
    };
    use rsille::render::buffer::Buffer;

    const INTERVAL: Duration = Duration::from_millis(50);

    pub(super) struct Terminal {
        tty: File,
        screen: bool,
        raw: bool,
        inline: Option<super::Inline>,
    }

    impl Terminal {
        pub(super) fn open() -> io::Result<Self> {
            let tty = OpenOptions::new().read(true).write(true).open("/dev/tty")?;
            let terminal = Self {
                tty,
                screen: false,
                raw: false,
                inline: None,
            };
            terminal.check_connection()?;
            terminal.size()?;
            // Background jobs must fail rather than be suspended by SIGTTIN.
            // SAFETY: tty owns a live file descriptor; these calls take no pointers.
            let foreground = unsafe { libc::tcgetpgrp(terminal.tty.as_raw_fd()) };
            if foreground < 0 {
                return Err(io::Error::last_os_error());
            }
            if foreground != unsafe { libc::getpgrp() } {
                return Err(io::Error::other(
                    "interactive CLI must run in the foreground",
                ));
            }
            Ok(terminal)
        }

        pub(super) fn enter(&mut self, inline: bool) -> io::Result<()> {
            self.inline = inline.then(super::Inline::default);
            terminal::enable_raw_mode()?;
            self.raw = true;
            // Mark before writing: a failed write may still have changed modes.
            self.screen = true;
            if inline {
                return execute!(
                    self.tty,
                    DisableLineWrap,
                    Hide,
                    EnableBracketedPaste,
                    ResetColor
                );
            }
            execute!(
                self.tty,
                EnterAlternateScreen,
                DisableLineWrap,
                Hide,
                EnableBracketedPaste,
                ResetColor,
                Clear(ClearType::All),
                MoveTo(0, 0)
            )
        }

        pub(super) fn check_connection(&self) -> io::Result<()> {
            let mut fd = libc::pollfd {
                fd: self.tty.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            // SAFETY: fd is initialized and valid for one pollfd.
            let result = unsafe { libc::poll(&mut fd, 1, 0) };
            if result < 0 {
                let error = io::Error::last_os_error();
                if error.kind() != io::ErrorKind::Interrupted {
                    return Err(error);
                }
            }
            if fd.revents & (libc::POLLHUP | libc::POLLERR | libc::POLLNVAL) != 0 {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "tty hung up"));
            }
            Ok(())
        }

        pub(super) fn size(&self) -> io::Result<(u16, u16)> {
            // SAFETY: winsize is a C integer-only structure; ioctl initializes it.
            let mut size: libc::winsize = unsafe { std::mem::zeroed() };
            if unsafe { libc::ioctl(self.tty.as_raw_fd(), libc::TIOCGWINSZ, &mut size) } < 0 {
                return Err(io::Error::last_os_error());
            }
            if size.ws_col == 0 || size.ws_row == 0 {
                return Err(io::Error::other("tty has zero dimensions"));
            }
            Ok((size.ws_col, size.ws_row))
        }

        pub(super) fn cell_pixels(&self) -> Option<super::CellPixels> {
            // SAFETY: ioctl fills a valid winsize using our controlling tty.
            let mut size: libc::winsize = unsafe { std::mem::zeroed() };
            if unsafe { libc::ioctl(self.tty.as_raw_fd(), libc::TIOCGWINSZ, &mut size) } < 0
                || size.ws_col == 0
                || size.ws_row == 0
                || size.ws_xpixel == 0
                || size.ws_ypixel == 0
                || size.ws_xpixel % size.ws_col != 0
                || size.ws_ypixel % size.ws_row != 0
            {
                return None;
            }
            Some(super::CellPixels {
                width: size.ws_xpixel / size.ws_col,
                height: size.ws_ypixel / size.ws_row,
            })
        }
        pub(super) fn present(
            &mut self,
            buffer: &Buffer,
            pixels: Option<super::CellPixels>,
        ) -> io::Result<()> {
            if let Some(pixels) = pixels {
                let dimensions = self.size()?;
                let text = super::raster_frame(buffer, pixels, &mut self.inline, dimensions)?;
                return self.write_frame(text.as_bytes());
            }
            if buffer.has_raster() || buffer.has_unsupported_raster_composition() {
                return Err(io::Error::other("raster output requires --sixel"));
            }
            if self.inline.is_some() {
                let dimensions = self.size()?;
                let text = self.inline.as_mut().unwrap().present(buffer, dimensions)?;
                return self.write_frame(text.as_bytes());
            }
            let mut bytes = Vec::new();
            let size = buffer.size();
            for y in 0..size.height {
                for x in 0..size.width {
                    if let Some(cell) = buffer.get((x, y).into()) {
                        // Absolute positioning skips wide-grapheme continuation
                        // cells. SGR reset prevents style leakage between cells.
                        queue!(bytes, MoveTo(x, y), ResetColor)?;
                        cell.queue(&mut bytes)?;
                    }
                }
            }
            queue!(bytes, ResetColor)?;
            self.write_frame(&bytes)
        }

        fn write_frame(&mut self, bytes: &[u8]) -> io::Result<()> {
            let result = self.tty.write_all(bytes).and_then(|()| self.tty.flush());
            if result.is_err() {
                if let Some(region) = &mut self.inline {
                    region.write_failed();
                }
            }
            result
        }

        pub(super) fn restore(&mut self) -> io::Result<()> {
            let mut first = None;
            if self.screen {
                let cleanup = if self.inline.is_some() {
                    let height = self.size().map(|s| s.1).unwrap_or(1);
                    self.inline.as_mut().unwrap().clear(height)
                } else {
                    String::from("\x1b[?1049l")
                };
                // Separate writes ensure one failed operation does not prevent
                // attempts to undo the remaining modes.
                for bytes in [
                    b"\x18\x1b\\\x1b[?2026l".as_slice(),
                    b"\x1b[0m".as_slice(),
                    b"\x1b[?2004l",
                    b"\x1b[?7h",
                    b"\x1b[?25h",
                    cleanup.as_bytes(),
                ] {
                    if let Err(error) = self.tty.write_all(bytes) {
                        first.get_or_insert(error);
                    }
                }
                if let Err(error) = self.tty.flush() {
                    first.get_or_insert(error);
                }
                if first.is_none() {
                    self.screen = false;
                }
            }
            if self.raw {
                match terminal::disable_raw_mode() {
                    Ok(()) => self.raw = false,
                    Err(error) => {
                        first.get_or_insert(error);
                    }
                }
            }
            first.map_or(Ok(()), Err)
        }
    }

    impl Drop for Terminal {
        fn drop(&mut self) {
            let _ = self.restore();
        }
    }

    pub(super) struct Input {
        events: Receiver<io::Result<Event>>,
        stop: Arc<AtomicBool>,
        worker: Option<JoinHandle<()>>,
    }

    impl Input {
        pub(super) fn read_event(&mut self, timeout: Duration) -> io::Result<Option<Event>> {
            match self.events.recv_timeout(timeout) {
                Ok(event) => event.map(Some),
                Err(RecvTimeoutError::Timeout) => Ok(None),
                Err(RecvTimeoutError::Disconnected) => {
                    Err(io::Error::other("terminal input reader stopped"))
                }
            }
        }

        pub(super) fn start(_: &Terminal) -> io::Result<Self> {
            // Verified in crossterm 0.28.1: both Unix event backends use
            // tty_fd(), which opens /dev/tty when stdin is a pipe. No dup2 or
            // stdout redirection is needed. Do not also use EventStream.
            let (sender, events) = mpsc::sync_channel(32);
            let stop = Arc::new(AtomicBool::new(false));
            let stopping = stop.clone();
            let worker = thread::Builder::new()
                .name("cli-terminal-input".into())
                .spawn(move || {
                    while !stopping.load(Ordering::Acquire) {
                        let event = match event::poll(INTERVAL) {
                            Ok(false) => continue,
                            Ok(true) => event::read(),
                            Err(error) => Err(error),
                        };
                        let failed = event.is_err();
                        if sender.send(event).is_err() || failed {
                            break;
                        }
                    }
                })?;
            Ok(Self {
                events,
                stop,
                worker: Some(worker),
            })
        }
    }

    impl Drop for Input {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Release);
            if let Some(worker) = self.worker.take() {
                // Let ordinary timed polls finish before restoring raw mode.
                // Drain the bounded queue so a full channel cannot hold it up.
                let deadline = Instant::now() + INTERVAL * 2;
                while !worker.is_finished() && Instant::now() < deadline {
                    while self.events.try_recv().is_ok() {}
                    thread::sleep(Duration::from_millis(1));
                }
                // 0.28's mio backend can spin on EOF or block on an incomplete
                // escape sequence, even inside poll(timeout). Never join such
                // a reader during cleanup. This one-shot CLI host lets process
                // exit reclaim it; it must not be reused as a library session.
                if worker.is_finished() {
                    let _ = worker.join();
                }
            }
        }
    }
}

#[cfg(any(unix, windows))]
mod host {
    #[cfg(unix)]
    use super::unix::{Input, Terminal};
    #[cfg(windows)]
    use super::windows::{Input, Terminal};
    use crossterm::event::Event;
    use std::time::Duration;
    const INTERVAL: Duration = Duration::from_millis(50);
    pub(super) fn run(config: crate::config::Config) -> Result<Option<serde_json::Value>, String> {
        let inline = config.mode == "inline";
        let mut terminal = Terminal::open().map_err(|e| format!("open terminal: {e}"))?;
        let result = (|| {
            let mut size = terminal.size().map_err(|e| format!("terminal size: {e}"))?;
            // Resolve before Ui construction or any input reader. Never issue a
            // query on redirected standard streams (JSON input/result output).
            let pixels = if config.sixel {
                Some(if let Some(value) = &config.cell_pixels {
                    super::explicit_pixels(value, size, config.force_cell_pixels)?
                } else {
                    use std::io::IsTerminal;
                    terminal.cell_pixels()
                        .and_then(|p| super::CellPixels::resolve(Some(p)).ok())
                        .map(Ok)
                        .unwrap_or_else(|| {
                            if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
                                super::CellPixels::resolve(None)
                            } else {
                                Err(std::io::Error::other("automatic probing requires controlling-terminal stdin/stdout"))
                            }
                        })
                        .map_err(|e| format!("cannot resolve cell pixels: {e}; use --cell-pixels WxH with measured physical pixels per character cell (not columns x rows)"))?
                })
            } else {
                None
            };
            let mut ui = if let Some(pixels) = pixels {
                crate::ui::Ui::new_with_cell_pixels(config, size, pixels)?
            } else {
                crate::ui::build(config)
            };
            terminal
                .enter(inline)
                .map_err(|e| format!("set up terminal: {e}"))?;
            let height = if inline {
                super::Inline::height(ui.inline_height(), size.1)
            } else {
                size.1
            };
            ui.resize(size.0, height);
            terminal
                .present(ui.draw()?, pixels)
                .map_err(|e| format!("draw terminal: {e}"))?;
            let mut input =
                Input::start(&terminal).map_err(|e| format!("start terminal input: {e}"))?;
            loop {
                if let Some(outcome) = ui.outcome() {
                    return Ok(outcome.ok());
                }
                terminal
                    .check_connection()
                    .map_err(|e| format!("terminal disconnected: {e}"))?;
                // Query the owned terminal, never the redirected standard handles.
                let current = terminal.size().map_err(|e| format!("terminal size: {e}"))?;
                let mut dirty = current != size;
                if dirty {
                    size = current;
                    let height = if inline {
                        super::Inline::height(ui.inline_height(), size.1)
                    } else {
                        size.1
                    };
                    ui.resize(size.0, height);
                }
                match input.read_event(INTERVAL) {
                    Ok(Some(Event::Resize(_, _))) => {
                        // Re-query next iteration; queued resize events can be stale.
                    }
                    Ok(Some(event)) => {
                        ui.event(event)?;
                        dirty = true;
                    }
                    Err(error) => return Err(format!("read terminal input: {error}")),
                    Ok(None) => {}
                }
                if let Some(outcome) = ui.outcome() {
                    return Ok(outcome.ok());
                }
                if dirty {
                    if inline {
                        ui.resize(size.0, super::Inline::height(ui.inline_height(), size.1));
                    }
                    terminal
                        .present(ui.draw()?, pixels)
                        .map_err(|e| format!("draw terminal: {e}"))?;
                }
            }
        })();
        // Report restoration failures on ordinary exits; Drop also retries them
        // and covers unwinding panics and partially completed setup.
        match (result, terminal.restore()) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), Ok(())) => Err(error),
            (Ok(_), Err(error)) => Err(format!("restore terminal: {error}")),
            (Err(error), Err(cleanup)) => Err(format!("{error}; restore terminal: {cleanup}")),
        }
    }
}
