//! Persistent presentation, independent of terminal mode and input ownership.

use std::io::{self, Write};

use rsille::render::{buffer::Buffer, Builder, Render};

use super::{CellPixels, RasterCapabilities, RasterOutput};

#[cfg(test)]
#[path = "presentation_tests.rs"]
mod tests;

/// Sending a frame must include the real transport flush, not just staging.
pub(super) trait FrameSink {
    fn send(&mut self, bytes: &[u8]) -> io::Result<()>;
}

/// Buffer command fragments (including split UTF-8) until the renderer flushes.
/// Never flush on drop or replay a payload after a partial transport failure.
pub(super) struct FrameWriter<S> {
    sink: S,
    pending: Vec<u8>,
}

impl<S> FrameWriter<S> {
    fn new(sink: S) -> Self {
        Self {
            sink,
            pending: Vec::new(),
        }
    }
}

impl<S: FrameSink> Write for FrameWriter<S> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.pending.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.pending.is_empty() {
            return Ok(());
        }
        let result = self.sink.send(&self.pending);
        // Recovery commands must start a new payload even if send failed.
        self.pending.clear();
        result
    }
}

pub(super) fn inline_height(requested: u16, available: u16) -> u16 {
    requested.max(1).min(available.saturating_sub(1).max(1))
}

pub(super) struct Presentation<S: FrameSink> {
    renderer: Render<FrameWriter<S>, Buffer>,
    inline: bool,
}

impl<S: FrameSink> Presentation<S> {
    pub(super) fn new(sink: S, inline: bool, pixels: Option<CellPixels>) -> io::Result<Self> {
        let mut builder = Builder::new();
        builder
            .inline_mode(inline)
            .clear(!inline)
            .append_newline(false);
        if !inline {
            // The platform host has already entered the alternate screen.
            builder.enable_alt_screen();
        }
        if let Some(pixels) = pixels {
            let pixels = CellPixels::resolve(Some(pixels))?;
            builder.raster_output(RasterOutput {
                capabilities: RasterCapabilities {
                    sixel: true,
                    reliable_erase: !inline,
                    reliable_inline_erase: inline,
                    no_scroll: true,
                    cell_pixels: Some(pixels),
                },
            });
        }
        let mut writer = FrameWriter::new(sink);
        if inline {
            // Leave the caller's current line intact. Subsequent reservation,
            // positioning and guard rows belong exclusively to Render.
            writer.write_all(b"\r\n")?;
            writer.flush()?;
        }
        Ok(Self {
            renderer: builder.build_render(Buffer::default(), writer),
            inline,
        })
    }

    pub(super) fn present(&mut self, buffer: &Buffer, dimensions: (u16, u16)) -> io::Result<()> {
        self.renderer.set_inline_viewport_height(dimensions.1);
        let size = buffer.size();
        self.renderer
            .resize((size.width.min(dimensions.0), size.height.min(dimensions.1)).into());
        // Buffer's Draw implementation preserves text metrics, raster ownership
        // and unsupported-composition metadata, and clips without resampling.
        *self.renderer.thing_mut() = buffer.clone();
        self.renderer.render()
    }

    pub(super) fn preserve_inline(&mut self) -> io::Result<()> {
        if self.inline {
            self.renderer.commit_frame()?;
        }
        Ok(())
    }

    pub(super) fn clear_inline(&mut self, viewport_height: u16) -> io::Result<()> {
        if self.inline {
            self.renderer.set_inline_viewport_height(viewport_height);
            self.renderer.clear_inline()?;
        }
        Ok(())
    }
}
