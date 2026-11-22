//! Session selection and frame preflight shared by the native presenters.
#[cfg(feature = "kitty")]
use super::kitty::{KittyFrame, KittyPresenter};
use super::output::{
    CellPixels, RasterBackend, RasterCapabilities, RasterConfig, RasterOutput, RasterSession,
    RasterStats,
};
use super::sixel_presenter::{SixelFrame, SixelPresenter};
use crate::render::{area::Position, buffer::Buffer};
use std::{io, sync::Arc, time::Duration};

pub(super) const MAX_NATIVE_CELLS: usize = 32 * 1024;
const MAX_NATIVE_PIXELS: u64 = 16 * 1024 * 1024;
#[cfg(any(feature = "sixel", feature = "kitty"))]
pub(super) const MAX_FRAME_BYTES: usize = 32 * 1024 * 1024;

pub(super) fn check_limits(
    buffer: &Buffer,
    origin: Position,
    pixels: CellPixels,
) -> io::Result<()> {
    let count = buffer.raster_cells().count() + buffer.overlay_cells().count();
    let pixel_count = count as u64 * u64::from(pixels.width) * u64::from(pixels.height);
    let coordinates_valid = origin.x.checked_add(buffer.size().width).is_some()
        && origin.y.checked_add(buffer.size().height).is_some();
    let valid = count <= MAX_NATIVE_CELLS && pixel_count <= MAX_NATIVE_PIXELS && coordinates_valid;
    if !valid {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!(
                "native raster frame exceeds resource or coordinate limits: \
                             {count} cells (limit {MAX_NATIVE_CELLS}), \
                             {pixel_count} physical pixels (limit {MAX_NATIVE_PIXELS}), \
                             cell pixels {}x{}, coordinates valid: {coordinates_valid}",
                pixels.width, pixels.height,
            ),
        ));
    }
    Ok(())
}

pub(super) type EncodedCell = (u16, u16, Arc<[u8]>);
pub(in crate::render) struct RasterPlan {
    pub native: bool,
    /// Text damage or protocol-only changes may need a synchronized frame
    /// even after the last visible image has been removed.
    pub overlay_frame: bool,
    pub damage_pixels: Option<CellPixels>,
    pub clear: bool,
    pub inline_padding: u16,
    pub(super) payload: Payload,
}

pub(super) enum Payload {
    Text,
    Sixel(SixelFrame),
    #[cfg(feature = "kitty")]
    Kitty(KittyFrame),
}

impl RasterPlan {
    pub fn sixel_tiles(&self) -> &[EncodedCell] {
        match &self.payload {
            Payload::Text => &[],
            Payload::Sixel(frame) => &frame.encoded,
            #[cfg(feature = "kitty")]
            Payload::Kitty(_) => &[],
        }
    }

    pub fn has_commands(&self) -> bool {
        match &self.payload {
            Payload::Text => false,
            Payload::Sixel(frame) => !frame.encoded.is_empty(),
            #[cfg(feature = "kitty")]
            Payload::Kitty(frame) => !frame.commands.is_empty(),
        }
    }

    pub fn kitty_commands(&self) -> &[u8] {
        match &self.payload {
            Payload::Text | Payload::Sixel(_) => &[],
            #[cfg(feature = "kitty")]
            Payload::Kitty(frame) => &frame.commands,
        }
    }

    pub fn uses_kitty(&self) -> bool {
        match self.payload {
            Payload::Text | Payload::Sixel(_) => false,
            #[cfg(feature = "kitty")]
            Payload::Kitty(_) => true,
        }
    }
}

#[derive(Debug, Clone)]
enum Backend {
    Sixel(SixelPresenter),
    #[cfg(feature = "kitty")]
    Kitty(KittyPresenter),
    Pending(RasterBackend),
    Unavailable(Arc<io::Error>),
}

#[derive(Debug, Clone)]
pub(in crate::render) struct RasterPresenter {
    backend: Backend,
    pub alternate_screen: bool,
    session: Option<RasterSession>,
}
impl Default for RasterPresenter {
    fn default() -> Self {
        Self::new(RasterConfig::Output(Default::default()), false)
    }
}

impl RasterPresenter {
    pub fn new(config: RasterConfig, alternate_screen: bool) -> Self {
        let backend = match config {
            RasterConfig::Output(output) => {
                Backend::Sixel(SixelPresenter::new(output, alternate_screen))
            }
            RasterConfig::Request(request) => Backend::Pending(request),
            RasterConfig::Session(
                session @ (RasterSession::SixelFullscreen(pixels)
                | RasterSession::SixelInline(pixels)),
            ) => Backend::Sixel(SixelPresenter::new(
                RasterOutput {
                    capabilities: RasterCapabilities {
                        sixel: true,
                        reliable_erase: matches!(session, RasterSession::SixelFullscreen(_)),
                        reliable_inline_erase: matches!(session, RasterSession::SixelInline(_)),
                        no_scroll: true,
                        cell_pixels: Some(pixels),
                    },
                },
                alternate_screen,
            )),
            #[cfg(feature = "kitty")]
            RasterConfig::Session(
                RasterSession::KittyFullscreen(pixels) | RasterSession::KittyInline(pixels),
            ) => Backend::Kitty(KittyPresenter::new(pixels)),
            #[cfg(not(feature = "kitty"))]
            RasterConfig::Session(
                RasterSession::KittyFullscreen(_) | RasterSession::KittyInline(_),
            ) => Backend::Unavailable(Arc::new(unsupported("the kitty Cargo feature is disabled"))),
        };
        Self {
            backend,
            alternate_screen,
            session: match config {
                RasterConfig::Session(session) => Some(session),
                _ => None,
            },
        }
    }

    pub fn negotiate(&mut self, inline: bool, append_newline: bool) -> io::Result<()> {
        let Backend::Pending(request) = self.backend else {
            return Ok(());
        };
        let result = self
            .fullscreen(inline, append_newline)
            .and_then(|()| request.negotiate_fullscreen(None, Duration::from_millis(500)));
        match result {
            Ok(session) => *self = Self::new(RasterConfig::Session(session), self.alternate_screen),
            Err(e) if request == RasterBackend::Auto => {
                self.backend = Backend::Unavailable(Arc::new(e))
            }
            Err(e) => return Err(e),
        }
        Ok(())
    }

    fn fullscreen(&self, inline: bool, newline: bool) -> io::Result<()> {
        if inline {
            return Err(unsupported(
                "fullscreen snapshots do not support inline output; use an inline raster session",
            ));
        }
        if !self.alternate_screen || newline {
            return Err(unsupported(
                "native fullscreen graphics require an alternate screen without appended newlines",
            ));
        }
        Ok(())
    }

    pub fn set_output(&mut self, config: RasterOutput) {
        if self.session.is_none() {
            if let Backend::Sixel(s) = &mut self.backend {
                s.config = config;
            }
        }
    }

    fn validate(&self, buffer: &Buffer, inline: bool, newline: bool) -> io::Result<()> {
        if buffer.has_raster() || buffer.has_unsupported_raster_composition() {
            match &self.backend {
                Backend::Pending(_) => return Err(unsupported(
                    "custom writers require a verified raster_session or explicit raster_output",
                )),
                Backend::Unavailable(e) => {
                    return Err(io::Error::new(io::ErrorKind::Unsupported, e.clone()))
                }
                _ => match self.session {
                    Some(RasterSession::KittyInline(_) | RasterSession::SixelInline(_))
                        if !inline || newline =>
                    {
                        return Err(unsupported(
                            "inline graphics require inline mode without appended newlines",
                        ))
                    }
                    Some(RasterSession::KittyFullscreen(_) | RasterSession::SixelFullscreen(_)) => {
                        self.fullscreen(inline, newline)?
                    }
                    _ => {}
                },
            }
        }
        Ok(())
    }

    pub fn inline_padding_rows(&self, buffer: &Buffer) -> io::Result<u16> {
        self.validate(buffer, true, false)?;
        match &self.backend {
            Backend::Sixel(s) => s.inline_padding_rows(buffer),
            _ => Ok(0),
        }
    }

    pub fn prepare(
        &mut self,
        buffer: &Buffer,
        origin: Position,
        inline_extent: Option<u16>,
        append_newline: bool,
        inline_shrank: bool,
    ) -> io::Result<RasterPlan> {
        let inline = inline_extent.is_some();
        self.validate(buffer, inline, append_newline)?;
        match &mut self.backend {
            Backend::Sixel(s) => {
                s.alternate_screen = self.alternate_screen;
                s.prepare(buffer, origin, inline, append_newline, inline_shrank)
            }
            #[cfg(feature = "kitty")]
            Backend::Kitty(k) => k.prepare(buffer, origin, inline_extent),
            _ => Ok(RasterPlan {
                native: false,
                overlay_frame: false,
                damage_pixels: None,
                clear: false,
                inline_padding: 0,
                payload: Payload::Text,
            }),
        }
    }
    pub fn begin(&mut self, plan: &RasterPlan) {
        match &mut self.backend {
            Backend::Sixel(s) => s.begin(plan),
            #[cfg(feature = "kitty")]
            Backend::Kitty(k) => {
                if let Payload::Kitty(frame) = &plan.payload {
                    k.begin(frame)
                }
            }
            _ => {}
        }
    }
    pub fn commit(&mut self, plan: RasterPlan) {
        match &mut self.backend {
            Backend::Sixel(s) => s.commit(plan),
            #[cfg(feature = "kitty")]
            Backend::Kitty(k) => {
                if let Payload::Kitty(frame) = plan.payload {
                    k.commit(frame)
                }
            }
            _ => {}
        }
    }
    pub fn needs_protocol_recovery(&self) -> bool {
        match &self.backend {
            Backend::Sixel(s) => s.needs_protocol_recovery(),
            #[cfg(feature = "kitty")]
            Backend::Kitty(k) => k.uncertain(),
            _ => false,
        }
    }
    pub fn cleanup_kitty(&mut self, _out: &mut impl io::Write) -> io::Result<()> {
        #[cfg(feature = "kitty")]
        if let Backend::Kitty(k) = &mut self.backend {
            k.cleanup(_out)?;
        }
        Ok(())
    }
    pub fn forget_region(&mut self) {
        #[cfg(feature = "kitty")]
        if let Backend::Kitty(k) = &mut self.backend {
            k.forget_region();
        }
        if let Backend::Sixel(s) = &mut self.backend {
            s.forget_region();
        }
    }
    pub fn stats(&self) -> RasterStats {
        match &self.backend {
            Backend::Sixel(s) => s.stats(),
            _ => RasterStats::default(),
        }
    }
}

pub(super) fn unsupported(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::Unsupported, message)
}
