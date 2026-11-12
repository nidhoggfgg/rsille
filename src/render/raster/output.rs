//! Explicit terminal capabilities for opaque Sixel raster frames.
//!
//! Unknown graphics capabilities reject raster presentation. Native output
//! requires explicit cell metrics, optionally obtained with a startup probe.
//! Rendering never reads terminal input or discovers dimensions. Measuring pixels
//! does not negotiate Sixel support.

use std::{io, time::Duration};

/// Actual terminal-cell pixel dimensions, not Canvas samples per cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellPixels {
    /// Physical width of one terminal character cell in pixels, not columns.
    pub width: u16,
    /// Physical height of one terminal character cell in pixels, not rows.
    pub height: u16,
}

impl CellPixels {
    /// Converts image pixels to the smallest enclosing terminal-cell size.
    ///
    /// Each axis is rounded up independently. Returns `None` for zero cell
    /// metrics or a result exceeding `u16::MAX` on either axis. Zero image axes
    /// produce zero cell axes. This pure calculation neither probes the terminal
    /// nor enforces Sixel backend limits; metrics are supplied by the caller.
    ///
    /// ```
    /// use rsille::render::raster_output::CellPixels;
    /// let size = CellPixels { width: 8, height: 16 }.size_for_pixels(17, 33).unwrap();
    /// assert_eq!((size.width, size.height), (3, 3));
    /// ```
    pub fn size_for_pixels(self, width: u32, height: u32) -> Option<crate::render::area::Size> {
        if self.width == 0 || self.height == 0 {
            return None;
        }
        Some(crate::render::area::Size {
            width: u16::try_from(width.div_ceil(u32::from(self.width))).ok()?,
            height: u16::try_from(height.div_ceil(u32::from(self.height))).ok()?,
        })
    }

    /// Reads OS-reported window pixels without querying or reading terminal input.
    ///
    /// Returns `None` for unavailable, zero, or non-integral cell metrics. This
    /// says nothing about Sixel support and does not validate backend limits;
    /// use [`Self::resolve`] with `Some(pixels)` before storing the result.
    /// Call again after resize/font changes.
    pub fn from_terminal() -> Option<Self> {
        Self::read_terminal().ok()
    }

    /// Validates explicit pixels, or probes once at startup with a 500 ms timeout.
    ///
    /// `Some(pixels)` performs validation only and is safe after input starts.
    /// `None` calls [`Self::probe`]; its startup-only restrictions apply. Store the
    /// result in [`RasterCapabilities::cell_pixels`] as `Some(pixels)` before
    /// running the application. Rendering never calls this function implicitly.
    ///
    /// # Errors
    ///
    /// Returns a probe error or rejects dimensions outside native raster limits
    /// (1..=1024 per dimension and at most 65,536 pixels).
    ///
    /// ```
    /// use rsille::render::raster_output::CellPixels;
    /// let pixels = CellPixels { width: 10, height: 20 };
    /// assert_eq!(CellPixels::resolve(Some(pixels)).unwrap(), pixels);
    /// ```
    pub fn resolve(explicit: Option<Self>) -> io::Result<Self> {
        Self::resolve_with(explicit, || Self::probe(Duration::from_millis(500)))
    }

    /// Measures physical pixels per character cell once, before starting input.
    ///
    /// Reads OS window pixels first. If unavailable or invalid, Unix/WSL falls
    /// back to `CSI 16 t`, returning on the first valid reply or after `timeout`.
    /// The timeout bounds the terminal exchange, not a mandatory waiting period.
    /// Native Windows currently has no pixel-size discovery implementation.
    /// Store the returned snapshot as `Some(pixels)` in the raster configuration;
    /// font, DPI, and window changes do not automatically update it.
    ///
    /// Call before **any** terminal input reader, including external Crossterm
    /// readers. The query may consume unrelated input; it neither preserves keys
    /// nor flushes the input queue. A reply arriving after timeout may reach the
    /// subsequent reader. Terminal modes are restored before returning.
    ///
    /// # Errors
    ///
    /// Returns `AlreadyExists` after managed rsille input has been initialized,
    /// even if that loop has ended. External readers cannot be detected. Returns
    /// an error if neither source provides valid dimensions; the query requires
    /// stdin/stdout on the controlling terminal. Metrics do not prove Sixel support.
    /// Use [`Self::from_terminal`] for an OS-only lookup without input restrictions.
    ///
    /// ```no_run
    /// use std::time::Duration;
    /// use rsille::render::raster_output::{CellPixels, RasterCapabilities};
    /// let pixels = CellPixels::probe(Duration::from_millis(500))?;
    /// let capabilities = RasterCapabilities {
    ///     cell_pixels: Some(pixels),
    ///     ..Default::default() // Sixel safety capabilities must be verified separately.
    /// };
    /// # Ok::<(), std::io::Error>(())
    /// ```
    pub fn probe(timeout: Duration) -> io::Result<Self> {
        crate::render::terminal_probe::before_input(|| {
            Self::detect_with(Self::read_terminal, || {
                let (width, height) = crate::render::terminal_probe::cell_pixels(timeout)?;
                Ok(Self { width, height })
            })
        })
    }

    fn resolve_with(
        explicit: Option<Self>,
        read_os: impl FnOnce() -> io::Result<Self>,
    ) -> io::Result<Self> {
        let pixels = match explicit {
            Some(pixels) => pixels,
            None => read_os().map_err(|error| {
                io::Error::new(error.kind(), format!(
                    "automatic cell pixels are unavailable: {error}; set RasterCapabilities.cell_pixels to Some(CellPixels {{ width, height }}) with measured physical pixels per character cell, not terminal columns x rows"
                ))
            })?,
        };
        Self::validate(pixels)
    }

    fn validate(pixels: Self) -> io::Result<Self> {
        if !pixels.supported() {
            return Err(io::Error::new(io::ErrorKind::Unsupported, format!(
                "unsupported cell pixels {}x{}: dimensions must be 1..=1024 and area <=65536; use actual physical pixels per character cell, not terminal columns x rows",
                pixels.width, pixels.height,
            )));
        }
        Ok(pixels)
    }

    fn detect_with(
        read_os: impl FnOnce() -> io::Result<Self>,
        query: impl FnOnce() -> io::Result<Self>,
    ) -> io::Result<Self> {
        match read_os().and_then(Self::validate) {
            Ok(pixels) => Ok(pixels),
            Err(os_error) => query().and_then(Self::validate).map_err(|query_error| {
                io::Error::new(
                    query_error.kind(),
                    format!("OS metrics: {os_error}; CSI 16 t query: {query_error}"),
                )
            }),
        }
    }

    fn read_terminal() -> io::Result<Self> {
        Self::from_window_size(&crossterm::terminal::window_size()?)
    }

    fn from_window_size(size: &crossterm::terminal::WindowSize) -> io::Result<Self> {
        if size.columns == 0 || size.rows == 0 || size.width == 0 || size.height == 0 {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "OS window pixels or character grid are zero",
            ));
        }
        if !size.width.is_multiple_of(size.columns) || !size.height.is_multiple_of(size.rows) {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "OS window pixels are not an integral multiple of columns and rows",
            ));
        }
        Ok(Self {
            width: size.width / size.columns,
            height: size.height / size.rows,
        })
    }

    pub(super) fn inline_padding_rows(self) -> u16 {
        // Some terminals reserve the whole final six-pixel band, even when its
        // unused bits are transparent and raster attributes specify a shorter
        // image. Keep that padding above the scrolling boundary.
        let padding = (6 - self.height % 6) % 6;
        padding.div_ceil(self.height.max(1))
    }

    pub(super) fn supported(self) -> bool {
        // The encoder leaves unused bits in a partial final band transparent;
        // physical cell dimensions must not be rounded to six-pixel multiples.
        self.width > 0
            && self.width <= 1024
            && self.height > 0
            && self.height <= 1024
            && u32::from(self.width) * u32::from(self.height) <= 65_536
    }
}

/// Explicit host assertions about the entire terminal/multiplexer path.
///
/// All fields default to unknown/false. Setting these flags asserts that the
/// behavior was tested in the selected screen and output mode; it is not a
/// request to change terminal modes. The library has no certified terminal list.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct RasterCapabilities {
    /// Sixel DCS output and RGB palette definitions reach the terminal intact.
    pub sixel: bool,
    /// ED 2 erases graphics as well as characters in the alternate screen.
    pub reliable_erase: bool,
    /// EL 2 erases graphics and characters only on the addressed inline row.
    ///
    /// Inline output owns its complete rows, not just the buffer's columns.
    /// Graphics must also follow text scrolling when rows are reserved or the
    /// viewport is followed. This is independent of alternate-screen ED 2;
    /// leave it false unless the entire terminal/multiplexer path was verified.
    pub reliable_inline_erase: bool,
    /// Cell-positioned graphics do not scroll or move their origin, and retain
    /// their colors when subsequent palettes are sent. Unset bits in a partial
    /// final six-pixel band must not erase adjacent cells.
    /// Inline reserves trailing rows for band padding. Fullscreen moves bottom
    /// tile anchors upward with transparent prefixes so complete bands fit.
    /// Transparent prefix pixels must preserve existing text and graphics.
    pub no_scroll: bool,
    /// Physical pixels per character cell, required for native Sixel output.
    /// Set `Some(pixels)` explicitly or store the result of [`CellPixels::probe`]
    /// before starting input. Rendering never measures or refreshes this value.
    /// Missing or invalid metrics reject raster frames before terminal output.
    pub cell_pixels: Option<CellPixels>,
}

/// Terminal raster settings shared by render builders and TUI applications.
///
/// Raster presentation uses only Sixel and requires the default-enabled `sixel` feature.
/// Defaults leave every capability unknown: frames without raster content work
/// normally, while raster frames fail until the required capabilities are set.
/// Both modes require `sixel`, `no_scroll`, and valid `cell_pixels`. Fullscreen
/// additionally requires an active alternate screen and `reliable_erase`; inline
/// instead requires `reliable_inline_erase`. Appended-newline output and
/// character-effect composition of raster content are unsupported.
///
/// Native frames are limited to 32,768 visible raster cells, 16,777,216 physical
/// pixels, and 32 MiB of staged encoded payload; each tile is limited to 1 MiB.
/// These independent safety bounds are neither a global memory budget for source
/// snapshots and caches nor a terminal throughput guarantee.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct RasterOutput {
    /// Verified host behavior and measured cell dimensions for native output.
    pub capabilities: RasterCapabilities,
}

impl RasterOutput {
    pub(super) fn resolve(
        self,
        has_raster: bool,
        unsupported_composition: bool,
        alternate_screen: bool,
        inline: bool,
    ) -> std::io::Result<Option<CellPixels>> {
        if !has_raster && !unsupported_composition {
            return Ok(None);
        }
        let reason = if unsupported_composition {
            Some("Sixel raster output does not support this visual composition")
        } else if !cfg!(feature = "sixel") {
            Some("the sixel Cargo feature is disabled")
        } else if !inline && !alternate_screen {
            Some("native Sixel requires inline mode or an alternate screen")
        } else if !self.capabilities.sixel {
            Some("Sixel support is unknown or unavailable")
        } else if inline && !self.capabilities.reliable_inline_erase {
            Some("inline Sixel requires verified EL 2 graphics erasure and graphics following text scrolling")
        } else if (!inline && !self.capabilities.reliable_erase) || !self.capabilities.no_scroll {
            Some("graphics erase, palette, and non-scrolling behavior have not been verified")
        } else {
            None
        };
        match reason {
            Some(reason) => Err(std::io::Error::new(std::io::ErrorKind::Unsupported, reason)),

            None if has_raster => {
                let pixels = self.capabilities.cell_pixels.ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::Unsupported,
                        "native Sixel requires RasterCapabilities.cell_pixels: set Some(CellPixels { width, height }) explicitly or call CellPixels::probe before starting input",
                    )
                }).and_then(CellPixels::validate);
                pixels.map(Some)
            }
            None => Ok(None),
        }
    }
}

/// Cumulative Sixel work and current bounded cache occupancy for one renderer.
///
/// Unchanged visible tiles can skip cache lookup entirely. Cache hits avoid
/// encoding, not necessarily transmission or cleanup of a tile's old position.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RasterStats {
    /// Cumulative number of tiles encoded rather than reused from the cache.
    pub encodes: u64,
    /// Cumulative number of encoded-tile cache lookups that reused an entry.
    pub cache_hits: u64,
    /// Encoded payload bytes currently retained in the cache.
    pub cache_bytes: usize,
    /// Number of encoded tiles currently retained in the cache.
    pub cache_entries: usize,
    /// Cumulative number of tiles sent in successfully committed frames.
    pub tiles_sent: u64,
    /// Sixel payload bytes, excluding cursor positioning and character output.
    pub bytes_sent: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn default_capabilities_are_unknown_and_raster_is_rejected() {
        let config = RasterOutput::default();
        assert_eq!(config.capabilities, RasterCapabilities::default());
        assert_eq!(
            config.resolve(true, false, true, false).unwrap_err().kind(),
            io::ErrorKind::Unsupported
        );
        assert_eq!(config.resolve(false, false, false, true).unwrap(), None);
    }

    #[test]
    fn strict_is_lazy_and_rejects_unverified_inline_and_effects() {
        let unknown = RasterOutput::default();
        assert!(unknown.resolve(false, false, false, true).is_ok());
        assert!(unknown.resolve(true, false, true, false).is_err());
        let config = verified();
        assert!(config.resolve(true, false, true, true).is_err());
        assert!(config.resolve(true, false, false, false).is_err());
        for has_raster in [false, true] {
            let error = config.resolve(has_raster, true, true, false).unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::Unsupported);
            assert!(error.to_string().contains("composition"));
        }
    }

    #[test]
    fn feature_and_all_safety_conditions_are_required() {
        let config = verified();
        assert_eq!(
            config.resolve(true, false, true, false).is_ok(),
            cfg!(feature = "sixel")
        );
        for missing in 0..4 {
            let mut config = config;
            match missing {
                0 => config.capabilities.sixel = false,
                1 => config.capabilities.reliable_erase = false,
                2 => config.capabilities.no_scroll = false,
                _ => config.capabilities.cell_pixels = None,
            }
            assert!(config.resolve(true, false, true, false).is_err());
        }
    }

    #[test]
    fn explicit_pixels_never_read_os_even_when_invalid() {
        let pixels = CellPixels {
            width: 10,
            height: 20,
        };
        assert_eq!(CellPixels::resolve(Some(pixels)).unwrap(), pixels);
        assert_eq!(
            CellPixels::resolve_with(Some(pixels), || panic!("must not read OS")).unwrap(),
            pixels
        );
        for (width, height) in [(0, 20), (10, 0), (1025, 20), (10, 1025), (257, 256)] {
            let error = CellPixels::resolve_with(Some(CellPixels { width, height }), || {
                panic!("invalid explicit pixels must not fall back to OS")
            })
            .unwrap_err();
            assert!(error.to_string().contains("unsupported cell pixels"));
        }
    }

    #[test]
    fn automatic_pixels_use_os_without_guessing_or_rounding() {
        let window = crossterm::terminal::WindowSize {
            columns: 120,
            rows: 36,
            width: 1200,
            height: 720,
        };
        let pixels =
            CellPixels::resolve_with(None, || CellPixels::from_window_size(&window)).unwrap();
        assert_eq!(
            pixels,
            CellPixels {
                width: 10,
                height: 20
            }
        );
        for size in [
            crossterm::terminal::WindowSize {
                columns: 0,
                ..window
            },
            crossterm::terminal::WindowSize { rows: 0, ..window },
            crossterm::terminal::WindowSize { width: 0, ..window },
            crossterm::terminal::WindowSize {
                height: 0,
                ..window
            },
            crossterm::terminal::WindowSize {
                width: 1201,
                ..window
            },
            crossterm::terminal::WindowSize {
                height: 721,
                ..window
            },
            crossterm::terminal::WindowSize {
                width: 119,
                ..window
            },
        ] {
            let error =
                CellPixels::resolve_with(None, || CellPixels::from_window_size(&size)).unwrap_err();
            assert!(error
                .to_string()
                .contains("automatic cell pixels are unavailable"));
            assert!(error.to_string().contains("RasterCapabilities.cell_pixels"));
        }
        let error = CellPixels::resolve_with(None, || {
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "OS query denied",
            ))
        })
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        assert!(error.to_string().contains("OS query denied"));
        assert!(CellPixels::resolve_with(None, || Ok(CellPixels {
            width: 1025,
            height: 20
        }))
        .is_err());
    }

    #[test]
    fn automatic_detection_prefers_os_and_only_then_queries() {
        let pixels = CellPixels {
            width: 10,
            height: 20,
        };
        assert_eq!(
            CellPixels::detect_with(|| Ok(pixels), || panic!("OS metrics suffice")).unwrap(),
            pixels
        );
        for os_result in [
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "no pixel information",
            )),
            Ok(CellPixels {
                width: 0,
                height: 0,
            }),
            Ok(CellPixels {
                width: 1025,
                height: 20,
            }),
        ] {
            assert_eq!(
                CellPixels::detect_with(|| os_result, || Ok(pixels)).unwrap(),
                pixels
            );
        }
        let error = CellPixels::detect_with(
            || Err(io::Error::other("OS gave zero pixels")),
            || {
                Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "terminal did not answer",
                ))
            },
        )
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        assert!(error.to_string().contains("OS gave zero pixels"));
        assert!(error
            .to_string()
            .contains("CSI 16 t query: terminal did not answer"));
        assert!(CellPixels::detect_with(
            || Err(io::Error::other("OS unavailable")),
            || Ok(CellPixels {
                width: 0,
                height: 20
            }),
        )
        .is_err());
    }

    #[test]
    fn non_native_paths_do_not_require_pixels() {
        let mut config = verified();
        config.capabilities.cell_pixels = None;
        assert_eq!(config.resolve(false, false, true, false).unwrap(), None);
    }

    #[cfg(feature = "sixel")]
    #[test]
    fn native_output_requires_explicit_valid_pixels_and_never_discovers_them() {
        let mut config = verified();
        for pixels in [
            None,
            Some(CellPixels {
                width: 0,
                height: 20,
            }),
        ] {
            config.capabilities.cell_pixels = pixels;
            let result = config.resolve(true, false, true, false);

            let error = result.unwrap_err();
            assert!(error.to_string().contains(if pixels.is_none() {
                "CellPixels::probe before starting input"
            } else {
                "unsupported cell pixels"
            }));
        }
        let pixels = CellPixels {
            width: 10,
            height: 20,
        };
        config.capabilities.cell_pixels = Some(pixels);
        for _ in 0..3 {
            assert_eq!(
                config.resolve(true, false, true, false).unwrap(),
                Some(pixels)
            );
        }
    }

    #[test]
    fn invalid_pixel_metrics_are_rejected() {
        for (width, height) in [(0, 18), (8, 0), (1025, 18), (8, 1025), (1024, 1020)] {
            assert!(!CellPixels { width, height }.supported());
        }
        for (width, height) in [(1, 1), (8, 16), (8, 18), (10, 20), (1, 1024), (64, 1024)] {
            assert!(CellPixels { width, height }.supported());
        }
    }

    #[cfg(feature = "sixel")]
    #[test]
    fn partial_band_heights_resolve_without_rounding() {
        for height in 1..=25 {
            let pixels = CellPixels { width: 10, height };
            let mut config = verified();
            config.capabilities.cell_pixels = Some(pixels);
            assert_eq!(
                config.resolve(true, false, true, false).unwrap(),
                Some(pixels)
            );
        }
    }
}
