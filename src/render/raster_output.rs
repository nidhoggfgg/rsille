//! Conservative terminal policy for opaque raster frames.
//!
//! Unknown graphics capabilities always use character fallback. On Unix, automatic
//! cell metrics try OS information then CSI 16 t through a shared terminal input
//! owner. Measuring pixels does not negotiate Sixel support. No terminal names or
//! environment heuristics are treated as proof of graphics support.

use std::io;

/// Selects native raster output or the always-available character representation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum RasterPolicy {
    /// Use Sixel only when every required capability is explicitly known.
    #[default]
    Auto,
    /// Never probe for or emit Sixel.
    TextOnly,
    /// Reject unsupported raster frames before writing any frame bytes.
    RequireSixel,
}

/// Actual terminal-cell pixel dimensions, not Canvas samples per cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellPixels {
    pub width: u16,
    pub height: u16,
}

impl CellPixels {
    /// Reads OS-reported window pixels without querying or reading terminal input.
    ///
    /// Returns `None` for unavailable, zero, or non-integral cell metrics. This
    /// says nothing about Sixel support. Call again after resize/font changes.
    pub fn from_terminal() -> Option<Self> {
        Self::read_terminal().ok()
    }

    /// Resolves and validates physical cell pixels for native raster output.
    ///
    /// `Some(pixels)` uses only the explicit value, without reading OS information.
    /// `None` first reads OS window pixels and divides them by columns and rows.
    /// If unusable, Unix terminals (including WSL) are queried with CSI 16 t with
    /// a 500 ms deadline. Query results, including failures, are cached until the
    /// terminal grid changes; dimensions are never guessed or rounded. Native
    /// Windows currently supports only OS information or an explicit override.
    ///
    /// Call before starting an input reader. The Unix fallback takes ownership
    /// of terminal input; managed rsille event loops then use the same reader,
    /// preserving keys/mouse events received during the query. Do not mix this
    /// with external Crossterm `read`/`poll`/`EventStream` or another stdin reader.
    /// The renderer uses this rule too. Keep `RasterCapabilities::cell_pixels`
    /// as `None` for automatic updates rather than storing a startup measurement.
    ///
    /// # Errors
    ///
    /// Returns an error if neither OS information nor the terminal query provides
    /// usable metrics, or if explicit dimensions exceed native raster limits
    /// (1..=1024 per dimension and at most 65,536 pixels). Querying requires a
    /// controlling terminal with terminal stdin/stdout. Metrics do not establish
    /// Sixel support.
    ///
    /// ```
    /// use rsille::render::raster_output::CellPixels;
    /// let pixels = CellPixels { width: 10, height: 20 };
    /// assert_eq!(CellPixels::resolve(Some(pixels)).unwrap(), pixels);
    /// ```
    pub fn resolve(explicit: Option<Self>) -> io::Result<Self> {
        Self::resolve_with(explicit, Self::read_automatic)
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

    fn read_automatic() -> io::Result<Self> {
        Self::detect_with(Self::read_terminal, || {
            #[cfg(unix)]
            {
                let (width, height) = super::terminal_query::cell_pixels()?;
                Ok(Self { width, height })
            }
            #[cfg(not(unix))]
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "CSI 16 t input routing is currently supported only on Unix/WSL",
            ))
        })
    }

    fn detect_with(
        read_os: impl FnOnce() -> io::Result<Self>,
        query: impl FnOnce() -> io::Result<Self>,
    ) -> io::Result<Self> {
        match read_os().and_then(Self::validate) {
            Ok(pixels) => Ok(pixels),
            Err(os_error) => query().and_then(Self::validate).map_err(|query_error| {
                io::Error::new(
                    io::ErrorKind::Unsupported,
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
/// behavior was tested in the active alternate screen; it is not a request to
/// change terminal modes. The library currently has no certified terminal list.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct RasterCapabilities {
    /// Sixel DCS output and RGB palette definitions reach the terminal intact.
    pub sixel: bool,
    /// ED 2 erases graphics as well as characters in the alternate screen.
    pub reliable_erase: bool,
    /// Cell-positioned graphics (including the last row) do not scroll or move
    /// their origin, and retain their colors when subsequent palettes are sent.
    /// This includes transparent padding in a partial final six-pixel band:
    /// unset bits must not erase adjacent cells or cause bottom-edge scrolling.
    pub no_scroll: bool,
    /// Optional physical pixels per character cell. `Some` uses only that value;
    /// `None` (the default) tries OS window metrics, then a bounded CSI 16 t query
    /// on Unix/WSL. Query results are cached until the character grid changes.
    /// Unavailable/invalid metrics fail `RequireSixel`; `Auto` falls back to text.
    /// Use [`CellPixels::resolve`] for optional startup validation.
    pub cell_pixels: Option<CellPixels>,
}

/// Terminal raster settings shared by render builders and TUI applications.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct RasterOutput {
    pub policy: RasterPolicy,
    pub capabilities: RasterCapabilities,
}

impl RasterOutput {
    /// Character-only output, without capability queries.
    pub const fn text_only() -> Self {
        Self {
            policy: RasterPolicy::TextOnly,
            capabilities: RasterCapabilities {
                sixel: false,
                reliable_erase: false,
                no_scroll: false,
                cell_pixels: None,
            },
        }
    }

    pub(super) fn resolve(
        self,
        has_raster: bool,
        unsupported_composition: bool,
        alternate_screen: bool,
        inline: bool,
    ) -> std::io::Result<Option<CellPixels>> {
        self.resolve_with(
            has_raster,
            unsupported_composition,
            alternate_screen,
            inline,
            CellPixels::read_automatic,
        )
    }

    fn resolve_with(
        self,
        has_raster: bool,
        unsupported_composition: bool,
        alternate_screen: bool,
        inline: bool,
        read_os: impl FnOnce() -> io::Result<CellPixels>,
    ) -> io::Result<Option<CellPixels>> {
        if self.policy == RasterPolicy::TextOnly || (!has_raster && !unsupported_composition) {
            return Ok(None);
        }
        let reason = if unsupported_composition && self.policy == RasterPolicy::RequireSixel {
            Some("visual effects require character raster fallback")
        } else if !cfg!(feature = "sixel") {
            Some("the sixel Cargo feature is disabled")
        } else if inline || !alternate_screen {
            Some("native Sixel requires an alternate screen without inline or appended-newline output")
        } else if !self.capabilities.sixel {
            Some("Sixel support is unknown or unavailable")
        } else if !self.capabilities.reliable_erase || !self.capabilities.no_scroll {
            Some("graphics erase, palette, and non-scrolling behavior have not been verified")
        } else {
            None
        };
        match reason {
            Some(reason) if self.policy == RasterPolicy::RequireSixel => {
                Err(std::io::Error::new(std::io::ErrorKind::Unsupported, reason))
            }
            Some(_) => Ok(None),
            None if has_raster => {
                match CellPixels::resolve_with(self.capabilities.cell_pixels, read_os) {
                    Ok(pixels) => Ok(Some(pixels)),
                    Err(error) if self.policy == RasterPolicy::RequireSixel => Err(error),
                    Err(_) => Ok(None),
                }
            }
            None => Ok(None),
        }
    }
}

/// Cumulative Sixel work and current bounded cache occupancy for one renderer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RasterStats {
    pub encodes: u64,
    pub cache_hits: u64,
    pub cache_bytes: usize,
    pub cache_entries: usize,
    pub tiles_sent: u64,
    /// Sixel payload bytes, excluding cursor positioning and character output.
    pub bytes_sent: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn unknown_auto_falls_back_and_text_only_ignores_capabilities() {
        assert_eq!(
            RasterOutput::default()
                .resolve(true, false, true, false)
                .unwrap(),
            None
        );
        assert_eq!(
            verified(RasterPolicy::TextOnly)
                .resolve(true, true, false, true)
                .unwrap(),
            None
        );
    }

    #[test]
    fn strict_is_lazy_and_rejects_unknown_inline_and_effects() {
        let unknown = RasterOutput {
            policy: RasterPolicy::RequireSixel,
            ..Default::default()
        };
        assert!(unknown.resolve(false, false, false, true).is_ok());
        assert!(unknown.resolve(true, false, true, false).is_err());
        let config = verified(RasterPolicy::RequireSixel);
        assert!(config.resolve(true, false, true, true).is_err());
        assert!(config.resolve(true, false, false, false).is_err());
        assert!(config.resolve(false, true, true, false).is_err());
    }

    #[test]
    fn feature_and_all_safety_conditions_are_required() {
        let config = verified(RasterPolicy::Auto);
        assert_eq!(
            config.resolve(true, false, true, false).unwrap().is_some(),
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
            assert_eq!(
                config
                    .resolve_with(true, false, true, false, || {
                        Err(io::Error::new(
                            io::ErrorKind::Unsupported,
                            "no OS metrics (test)",
                        ))
                    })
                    .unwrap(),
                None
            );
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
        assert_eq!(error.kind(), io::ErrorKind::Unsupported);
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
    fn non_native_paths_do_not_read_os_metrics() {
        for policy in [
            RasterPolicy::Auto,
            RasterPolicy::TextOnly,
            RasterPolicy::RequireSixel,
        ] {
            let mut config = verified(policy);
            config.capabilities.cell_pixels = None;
            assert_eq!(
                config
                    .resolve_with(false, false, true, false, || panic!(
                        "no raster, no OS lookup"
                    ))
                    .unwrap(),
                None
            );
            if policy == RasterPolicy::TextOnly {
                assert_eq!(
                    config
                        .resolve_with(true, false, true, false, || panic!(
                            "text only must not query OS"
                        ))
                        .unwrap(),
                    None
                );
            }
            config.capabilities.sixel = false;
            let _ = config.resolve_with(true, false, true, false, || {
                panic!("unknown Sixel capability must not query OS")
            });
            config.capabilities.sixel = true;
            let _ = config.resolve_with(true, false, false, false, || {
                panic!("non-alternate screen must not query OS")
            });
            let _ = config.resolve_with(true, false, true, true, || {
                panic!("inline output must not query OS")
            });
            config.capabilities.no_scroll = false;
            let _ = config.resolve_with(true, false, true, false, || {
                panic!("unverified graphics must not query OS")
            });
        }
        #[cfg(not(feature = "sixel"))]
        {
            let mut config = verified(RasterPolicy::Auto);
            config.capabilities.cell_pixels = None;
            assert_eq!(
                config
                    .resolve_with(true, false, true, false, || panic!(
                        "disabled feature must not query OS"
                    ))
                    .unwrap(),
                None
            );
        }
    }

    #[cfg(feature = "sixel")]
    #[test]
    fn automatic_policy_resolution_stays_live_and_recovers_after_missing_metrics() {
        for policy in [RasterPolicy::Auto, RasterPolicy::RequireSixel] {
            let mut config = verified(policy);
            config.capabilities.cell_pixels = None;
            for pixels in [
                CellPixels {
                    width: 10,
                    height: 20,
                },
                CellPixels {
                    width: 12,
                    height: 24,
                },
            ] {
                assert_eq!(
                    config
                        .resolve_with(true, false, true, false, || Ok(pixels))
                        .unwrap(),
                    Some(pixels)
                );
                assert_eq!(config.capabilities.cell_pixels, None);
            }
            let missing = config.resolve_with(true, false, true, false, || {
                Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "OS metrics disappeared",
                ))
            });
            if policy == RasterPolicy::RequireSixel {
                assert!(missing
                    .unwrap_err()
                    .to_string()
                    .contains("OS metrics disappeared"));
            } else {
                assert_eq!(missing.unwrap(), None);
            }
            let pixels = CellPixels {
                width: 10,
                height: 20,
            };
            assert_eq!(
                config
                    .resolve_with(true, false, true, false, || Ok(pixels))
                    .unwrap(),
                Some(pixels)
            );
            config.capabilities.cell_pixels = Some(pixels);
            assert_eq!(
                config
                    .resolve_with(true, false, true, false, || panic!(
                        "explicit mode must not query OS"
                    ))
                    .unwrap(),
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
        for policy in [RasterPolicy::Auto, RasterPolicy::RequireSixel] {
            for height in 1..=25 {
                let pixels = CellPixels { width: 10, height };
                let mut config = verified(policy);
                config.capabilities.cell_pixels = Some(pixels);
                assert_eq!(
                    config.resolve(true, false, true, false).unwrap(),
                    Some(pixels)
                );
            }
        }
    }
}
