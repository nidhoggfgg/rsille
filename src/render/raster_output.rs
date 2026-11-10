//! Conservative terminal policy for opaque raster frames.
//!
//! Unknown capabilities always use character fallback. This release does not send
//! DA/CSI queries: Crossterm's input stream does not expose their replies, and a
//! competing stdin reader would consume application input. Hosts may provide
//! measured capabilities explicitly. No terminal names or environment heuristics
//! are treated as proof of graphics support.

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
        let size = crossterm::terminal::window_size().ok()?;
        if size.columns == 0
            || size.rows == 0
            || size.width == 0
            || size.height == 0
            || size.width % size.columns != 0
            || size.height % size.rows != 0
        {
            return None;
        }
        Some(Self {
            width: size.width / size.columns,
            height: size.height / size.rows,
        })
    }

    pub(super) fn supported(self) -> bool {
        // Until partial-band allocation is verified on real terminals, avoid
        // even allocating a Sixel band outside a cell at the bottom clip edge.
        self.width > 0
            && self.width <= 1024
            && self.height > 0
            && self.height <= 1024
            && self.height.is_multiple_of(6)
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
    pub no_scroll: bool,
    /// Explicit metrics; when absent, an event loop may use OS window metrics.
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
        } else if !self
            .capabilities
            .cell_pixels
            .is_some_and(CellPixels::supported)
        {
            Some("cell pixels are unknown, oversized, or height is not a multiple of six")
        } else {
            None
        };
        match reason {
            Some(reason) if self.policy == RasterPolicy::RequireSixel => {
                Err(std::io::Error::new(std::io::ErrorKind::Unsupported, reason))
            }
            Some(_) => Ok(None),
            None if has_raster => Ok(self.capabilities.cell_pixels),
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
            assert_eq!(config.resolve(true, false, true, false).unwrap(), None);
        }
    }

    #[test]
    fn invalid_pixel_metrics_are_rejected() {
        for (width, height) in [(0, 18), (8, 0), (8, 16), (1025, 18), (1024, 1020)] {
            assert!(!CellPixels { width, height }.supported());
        }
        assert!(CellPixels {
            width: 8,
            height: 18
        }
        .supported());
    }
}
