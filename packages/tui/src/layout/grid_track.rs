//! Grid track sizing types

/// Grid track sizing unit
///
/// Defines how a grid track (row or column) should be sized.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GridTrack {
    /// Fixed size in terminal cells
    ///
    /// # Examples
    /// ```
    /// use tui::layout::GridTrack;
    ///
    /// let track = GridTrack::Fixed(20); // 20 cells wide/tall
    /// ```
    Fixed(u16),

    /// Fraction of available space
    ///
    /// Multiple fr units distribute space proportionally.
    ///
    /// # Examples
    /// ```
    /// use tui::layout::GridTrack;
    ///
    /// let track = GridTrack::Fr(1.0); // Takes 1 fraction of available space
    /// let track = GridTrack::Fr(2.0); // Takes 2 fractions (twice as much)
    /// ```
    Fr(f32),

    /// Auto-sized based on content
    ///
    /// The track will size itself to fit its content.
    ///
    /// # Examples
    /// ```
    /// use tui::layout::GridTrack;
    ///
    /// let track = GridTrack::Auto; // Sizes to content
    /// ```
    Auto,
}

impl GridTrack {
    /// Parse a track size from a string
    ///
    /// Supports:
    /// - "10" or "10px" -> Fixed(10)
    /// - "1fr" -> Fr(1.0)
    /// - "auto" -> Auto
    ///
    /// # Examples
    /// ```
    /// use tui::layout::GridTrack;
    ///
    /// assert_eq!(GridTrack::parse("20"), Some(GridTrack::Fixed(20)));
    /// assert_eq!(GridTrack::parse("1fr"), Some(GridTrack::Fr(1.0)));
    /// assert_eq!(GridTrack::parse("auto"), Some(GridTrack::Auto));
    /// ```
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();

        if s == "auto" {
            return Some(GridTrack::Auto);
        }

        if let Some(fr_str) = s.strip_suffix("fr") {
            if let Ok(value) = fr_str.trim().parse::<f32>() {
                return Some(GridTrack::Fr(value));
            }
        }

        // Try to parse as fixed size (with optional "px" suffix)
        let num_str = s.strip_suffix("px").unwrap_or(s);
        if let Ok(value) = num_str.trim().parse::<u16>() {
            return Some(GridTrack::Fixed(value));
        }

        None
    }

    /// Parse a track template string into a vector of tracks
    ///
    /// Splits by whitespace and parses each part.
    /// Supports repeat() function syntax: "repeat(3, 1fr)" expands to "1fr 1fr 1fr"
    ///
    /// # Examples
    /// ```
    /// use tui::layout::GridTrack;
    ///
    /// let tracks = GridTrack::parse_template("1fr 20 auto");
    /// assert_eq!(tracks.len(), 3);
    ///
    /// let tracks = GridTrack::parse_template("repeat(3, 1fr)");
    /// assert_eq!(tracks.len(), 3);
    /// ```
    pub fn parse_template(template: &str) -> Vec<Self> {
        let mut result = Vec::new();
        let template = template.trim();

        // Handle repeat() function
        if let Some(repeat_content) = Self::extract_repeat(template) {
            if let Some((count, track_str)) = Self::parse_repeat_args(&repeat_content) {
                let track = Self::parse(&track_str);
                if let Some(t) = track {
                    for _ in 0..count {
                        result.push(t);
                    }
                }
                return result;
            }
        }

        // Normal parsing
        template
            .split_whitespace()
            .filter_map(Self::parse)
            .collect()
    }

    /// Extract content from repeat() function
    fn extract_repeat(template: &str) -> Option<String> {
        let template = template.trim();
        if template.starts_with("repeat(") && template.ends_with(')') {
            Some(template[7..template.len() - 1].to_string())
        } else {
            None
        }
    }

    /// Parse repeat() arguments: "3, 1fr" -> (3, "1fr")
    fn parse_repeat_args(args: &str) -> Option<(usize, String)> {
        let parts: Vec<&str> = args.split(',').collect();
        if parts.len() == 2 {
            let count = parts[0].trim().parse::<usize>().ok()?;
            let track_str = parts[1].trim().to_string();
            Some((count, track_str))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_fixed() {
        assert_eq!(GridTrack::parse("20"), Some(GridTrack::Fixed(20)));
        assert_eq!(GridTrack::parse("20px"), Some(GridTrack::Fixed(20)));
        assert_eq!(GridTrack::parse("  10  "), Some(GridTrack::Fixed(10)));
    }

    #[test]
    fn test_parse_fr() {
        assert_eq!(GridTrack::parse("1fr"), Some(GridTrack::Fr(1.0)));
        assert_eq!(GridTrack::parse("2.5fr"), Some(GridTrack::Fr(2.5)));
        assert_eq!(GridTrack::parse("  1fr  "), Some(GridTrack::Fr(1.0)));
    }

    #[test]
    fn test_parse_auto() {
        assert_eq!(GridTrack::parse("auto"), Some(GridTrack::Auto));
        assert_eq!(GridTrack::parse("  auto  "), Some(GridTrack::Auto));
    }

    #[test]
    fn test_parse_invalid() {
        assert_eq!(GridTrack::parse("invalid"), None);
        assert_eq!(GridTrack::parse(""), None);
    }

    #[test]
    fn test_parse_template() {
        let tracks = GridTrack::parse_template("1fr 20 auto 2fr");
        assert_eq!(tracks.len(), 4);
        assert_eq!(tracks[0], GridTrack::Fr(1.0));
        assert_eq!(tracks[1], GridTrack::Fixed(20));
        assert_eq!(tracks[2], GridTrack::Auto);
        assert_eq!(tracks[3], GridTrack::Fr(2.0));
    }

    #[test]
    fn test_parse_template_empty() {
        let tracks = GridTrack::parse_template("");
        assert_eq!(tracks.len(), 0);
    }

    #[test]
    fn test_parse_template_repeat() {
        let tracks = GridTrack::parse_template("repeat(3, 1fr)");
        assert_eq!(tracks.len(), 3);
        assert_eq!(tracks[0], GridTrack::Fr(1.0));
        assert_eq!(tracks[1], GridTrack::Fr(1.0));
        assert_eq!(tracks[2], GridTrack::Fr(1.0));
    }

    #[test]
    fn test_parse_template_repeat_fixed() {
        let tracks = GridTrack::parse_template("repeat(2, 20)");
        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0], GridTrack::Fixed(20));
        assert_eq!(tracks[1], GridTrack::Fixed(20));
    }
}
