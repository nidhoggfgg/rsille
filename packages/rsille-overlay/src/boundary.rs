//! Track safe output boundaries and child screen changes across PTY reads.
//!
//! [`Boundary`] is a small state machine over terminal output. It knows when
//! the stream is between escape sequences and UTF-8 characters, and when no
//! Kitty upload, synchronized update or saved cursor of the child is open, so
//! an overlay can write its own sequences without corrupting the child's.

use crate::mouse::MouseModes;

#[derive(Clone, Copy, Default)]
enum State {
    #[default]
    Ground,
    Escape,
    Intermediate,
    Csi,
    ApcStart,
    KittyHeader,
    String {
        osc: bool,
        esc: bool,
    },
}

pub(crate) enum Event {
    MouseHandoff,
    Clear,
    SyncReset {
        csi_start: Option<usize>,
        replacement: Option<Vec<u8>>,
    },
}

/// Parser state for one output stream. Feed every byte the child writes.
#[derive(Default)]
pub struct Boundary {
    state: State,
    continuation: u8,
    csi: Vec<u8>,
    kitty_header: Vec<u8>,
    uploading: bool,
    synchronized: bool,
    clear_pending: [bool; 2],
    saved_cursor: [bool; 2],
    pub(crate) mouse: MouseModes,
    pub(crate) alternate: bool,
    /// The last batch included a screen selection, even if it switched back.
    pub(crate) screen_selected: bool,
    /// The last batch may have scrolled or cleared placements on screen.
    pub(crate) placement_disturbed: bool,
    /// The last batch may have deleted uploaded Kitty graphics data.
    pub(crate) graphics_deleted: bool,
}

impl Boundary {
    /// Between escape sequences and UTF-8 characters.
    pub fn ready(&self) -> bool {
        matches!(self.state, State::Ground) && self.continuation == 0
    }

    /// [`Self::ready`], and no upload, synchronized update or saved cursor is open.
    pub fn can_draw(&self) -> bool {
        self.ready()
            && !self.uploading
            && !self.synchronized
            && !self.saved_cursor[usize::from(self.alternate)]
    }

    /// The child shows its alternate screen.
    pub fn alternate(&self) -> bool {
        self.alternate
    }

    pub(crate) fn synchronized(&self) -> bool {
        self.synchronized
    }

    pub(crate) fn child_mouse(&self) -> bool {
        self.alternate || self.mouse.tracking != 0
    }

    fn private_mode(&mut self, param: &[u8], set: bool) -> bool {
        match param {
            b"47" | b"1047" | b"1049" => {
                self.alternate = set;
                self.screen_selected = true;
                if set && param == b"1049" {
                    self.saved_cursor[1] = false;
                }
            }
            b"2026" => self.synchronized = set,
            b"1048" => self.saved_cursor[usize::from(self.alternate)] = set,
            b"9" | b"1000" | b"1001" | b"1002" | b"1003" => {
                self.mouse.tracking = if set { decimal(param) } else { 0 };
                return true;
            }
            b"1005" | b"1006" | b"1015" | b"1016" => {
                self.mouse.encoding = if set { decimal(param) } else { 0 };
                return true;
            }
            _ => {}
        }
        false
    }

    // APC boundaries and upload boundaries differ: m=1 leaves a transfer
    // open even after ST, until the final continuation (or a delete).
    fn finish_kitty_header(&mut self) {
        let mut more = false;
        let mut action = b"t".as_slice();
        for field in self.kitty_header.split(|&c| c == b',') {
            if let Some(a) = field.strip_prefix(b"a=") {
                action = a;
            }
            more |= field == b"m=1";
        }
        match action {
            b"t" | b"T" | b"f" => self.uploading = more,
            b"d" => {
                self.uploading = false;
                self.graphics_deleted = true;
            }
            _ => {}
        }
    }

    /// Reports an audible BEL in ordinary output. BEL also terminates OSC strings.
    pub fn feed(&mut self, bytes: &[u8]) -> bool {
        self.feed_with(bytes, |_, _, _| {})
    }

    pub(crate) fn resized(&mut self) {
        // A hidden primary screen can reflow too. Clear it when the child returns.
        self.clear_pending = [true, self.alternate];
    }

    fn clear_if_ready(&mut self, offset: usize, notify: &mut impl FnMut(usize, &Self, Event)) {
        let screen = usize::from(self.alternate);
        if self.clear_pending[screen] && self.ready() {
            self.clear_pending[screen] = false;
            self.placement_disturbed = true;
            notify(offset, self, Event::Clear);
        }
    }

    pub(crate) fn feed_with(
        &mut self,
        bytes: &[u8],
        mut notify: impl FnMut(usize, &Self, Event),
    ) -> bool {
        use self::State::*;
        let mut rang = false;
        self.screen_selected = false;
        self.placement_disturbed = false;
        self.graphics_deleted = false;
        let mut esc_idx = None;
        let mut csi_start = None;
        for (idx, &b) in bytes.iter().enumerate() {
            self.clear_if_ready(idx, &mut notify);
            rang |= b == 7 && self.ready();
            // A continuation byte can look exactly like an eight-bit C1 control.
            if self.continuation != 0 && matches!(b, 0x80..=0xbf) {
                self.continuation -= 1;
                continue;
            }
            // On malformed input, reconsider the interrupting byte as a fresh byte.
            self.continuation = match b {
                0xc2..=0xdf => 1,
                0xe0..=0xef => 2,
                0xf0..=0xf4 => 3,
                _ => 0,
            };
            if matches!(self.state, Csi) {
                if matches!(b, 0x40..=0x7e) && self.csi.len() < 128 {
                    if matches!(b, b'h' | b'l') && self.csi.starts_with(b"?") {
                        let alternate = self.alternate;
                        let csi = std::mem::take(&mut self.csi);
                        let mut mouse_changed = false;
                        let mut sync_reset = false;
                        for param in csi[1..].split(|&c| c == b';') {
                            mouse_changed |= self.private_mode(param, b == b'h');
                            sync_reset |= b == b'l' && param == b"2026";
                        }
                        if sync_reset {
                            let replacement = strip_sync_param(&csi[1..]);
                            notify(
                                idx + 1,
                                self,
                                Event::SyncReset {
                                    csi_start,
                                    replacement,
                                },
                            );
                        }
                        self.csi = csi;
                        if mouse_changed || alternate != self.alternate {
                            notify(idx + 1, self, Event::MouseHandoff);
                        }
                    } else if b == b'J'
                        || (matches!(b, b'S' | b'T' | b'L' | b'M')
                            && !self.csi.starts_with(b"?")
                            && !self.csi.starts_with(b">"))
                    {
                        self.placement_disturbed = true;
                    } else if b == b'p' && self.csi == b"!" {
                        self.graphics_deleted = true;
                    }
                } else if self.csi.len() < 128 {
                    self.csi.push(b);
                }
            } else {
                self.csi.clear();
            }
            if matches!(self.state, Escape) {
                if matches!(b, b'7' | b'8') {
                    self.saved_cursor[usize::from(self.alternate)] = b == b'7';
                } else if matches!(b, b'D' | b'E' | b'M') {
                    self.placement_disturbed = true;
                } else if b == b'c' {
                    self.graphics_deleted = true;
                }
            }
            if self.ready() && matches!(b, b'\n' | 0x0b | 0x0c | 0x84 | 0x85 | 0x8d) {
                self.placement_disturbed = true;
            }
            if matches!(self.state, KittyHeader) {
                if matches!(b, b';' | 0x1b | 0x9c) {
                    self.finish_kitty_header();
                } else if self.kitty_header.len() < 1024 {
                    self.kitty_header.push(b);
                }
            }
            let prev_state = self.state;
            self.state = match (self.state, b) {
                (_, 0x18 | 0x1a | 0x9c) => Ground, // CAN, SUB, ST
                (String { osc: true, .. }, 7) | (String { esc: true, .. }, b'\\') => Ground,
                (String { osc, .. }, _) => String {
                    osc,
                    esc: b == 0x1b,
                },
                (ApcStart, b'G') => {
                    self.kitty_header.clear();
                    KittyHeader
                }
                (KittyHeader, b';' | 0x1b) | (ApcStart, _) => String {
                    osc: false,
                    esc: b == 0x1b,
                },
                (Escape, b'_') | (_, 0x9f) => ApcStart,
                (_, 0x1b) => Escape,
                (Escape, b'[') | (_, 0x9b) => Csi,
                (Escape, b']') | (_, 0x9d) => String {
                    osc: true,
                    esc: false,
                },
                (Escape, b'P' | b'X' | b'^') | (_, 0x90 | 0x98 | 0x9e) => String {
                    osc: false,
                    esc: false,
                },
                (Escape | Intermediate, 0x20..=0x2f) => Intermediate,
                (Escape | Intermediate, 0x30..=0x7e) | (Csi, 0x40..=0x7e) => Ground,
                (s, _) => s,
            };
            match self.state {
                Escape => {
                    esc_idx = Some(idx);
                    csi_start = None;
                }
                Csi => {
                    if matches!(prev_state, Escape) && b == b'[' {
                        csi_start = esc_idx.take();
                    } else if b == 0x9b {
                        self.csi.clear();
                        csi_start = Some(idx);
                        esc_idx = None;
                    }
                }
                _ => {
                    esc_idx = None;
                    csi_start = None;
                }
            }
        }
        self.clear_if_ready(bytes.len(), &mut notify);
        rang
    }
}

fn strip_sync_param(params: &[u8]) -> Option<Vec<u8>> {
    let mut kept = params
        .split(|&c| c == b';')
        .filter(|&p| p != b"2026")
        .peekable();
    kept.peek()?;
    let mut out = Vec::with_capacity(params.len() + 4);
    out.extend_from_slice(b"\x1b[?");
    let mut first = true;
    for param in kept {
        if !first {
            out.push(b';');
        }
        first = false;
        out.extend_from_slice(param);
    }
    out.push(b'l');
    Some(out)
}

fn decimal(digits: &[u8]) -> u16 {
    digits.iter().fold(0, |n, c| n * 10 + u16::from(c - b'0'))
}

#[cfg(test)]
mod tests {
    use super::Boundary;

    #[test]
    fn defers_overlay_across_chunked_uploads_and_child_transactions() {
        for (open, close) in [
            (
                b"\x1b_Ga=T,i=12,m=1;abcd\x1b\\".as_slice(),
                b"\x1b_Gm=0;efgh\x1b\\".as_slice(),
            ),
            (b"\x1b_Ga=t,m=1;abcd\x1b\\", b"\x1b_Ga=d,d=A\x1b\\"),
            (b"\x1b[?2026h", b"\x1b[?2026l"),
            (b"\x1b7", b"\x1b8"),
        ] {
            let mut b = Boundary::default();
            b.feed(b"\x1b[?1049h");
            for byte in open {
                b.feed(&[*byte]);
                assert!(!b.can_draw());
            }
            assert!(b.ready());
            for (index, byte) in close.iter().enumerate() {
                b.feed(&[*byte]);
                assert_eq!(b.can_draw(), index + 1 == close.len());
            }
        }
    }

    #[test]
    fn only_ready_after_final_byte() {
        for seq in [
            &b"\x1b[?25l"[..],
            b"\x1b[1;2 q",
            b"\x1b(B",
            b"\x1b#8",
            b"\x1b7",
            b"\x1b]0;title\x07",
            b"\x1b]0;t\x1b\\",
            b"\x1bPq\x07abc\x1b\\",
            b"\x9b31m",
            b"\x9fabc\x9c",
            b"\x9d\xc3\x9c\x07",
            "¢".as_bytes(),
            "界".as_bytes(),
            "🦀".as_bytes(),
        ] {
            let mut boundary = Boundary::default();
            assert!(boundary.ready());
            for (i, b) in seq.iter().enumerate() {
                boundary.feed(std::slice::from_ref(b));
                assert_eq!(boundary.ready(), i + 1 == seq.len(), "{seq:?} at {i}");
            }
        }
    }

    #[test]
    fn bell_is_distinct_from_a_control_string_terminator() {
        for seq in [
            b"\x1b]2;title\x07".as_slice(),
            b"\x1bPq\x07\x1b\\",
            b"\x9dtitle\x07",
        ] {
            for split in 0..=seq.len() {
                let mut b = Boundary::default();
                assert!(!b.feed(&seq[..split]));
                assert!(!b.feed(&seq[split..]));
                assert!(b.feed(b"x\x07y"));
            }
        }
    }

    #[test]
    fn tracks_child_screen_across_reads_and_ignores_control_strings() {
        let mut boundary = Boundary::default();
        for byte in b"\x1b[?25;1049h" {
            boundary.feed(&[*byte]);
        }
        assert!(boundary.alternate);
        boundary.feed(b"\x1b]0;\x1b[?1049l\x07");
        assert!(boundary.alternate);
        boundary.feed(b"\x1b[?1049l");
        assert!(!boundary.alternate);
        assert!(boundary.screen_selected);
        boundary.feed(b"prompt");
        assert!(!boundary.screen_selected);
        boundary.feed(b"\x1b[?47h\x1b[?47l");
        assert!(!boundary.alternate);
        assert!(boundary.screen_selected);
    }

    #[test]
    fn interrupted_input_resynchronizes() {
        let mut b = Boundary::default();
        b.feed(b"plain\xe2\x1b[12\x18\x1b]title");
        b.feed(b"");
        assert!(!b.ready());
        b.feed(b"\x1a");
        assert!(b.ready());
        b.feed(b"\xf0\x1b[");
        assert!(!b.ready());
        b.feed(b"mtext");
        assert!(b.ready());
    }

    #[test]
    fn distinguishes_ordinary_edits_placement_disturbances_and_graphics_deletes() {
        let mut b = Boundary::default();
        b.feed(b"\x1b[?1049h");

        // Cursor movement, SGR, EL, plain text, UTF-8 containing 0x84/0x85/0x8d bytes,
        // and XTSMGRAPHICS (?..S) do not disturb Kitty placements.
        for benign in [
            b"\x1b[5;10Hhello\x1b[32m\x1b[K".as_slice(),
            "\u{2014}\u{2015}\u{201d}鲸鱼".as_bytes(),
            b"\x1b[?2;1;0S",
            b"\x1b[>0T",
        ] {
            b.feed(benign);
            assert!(!b.placement_disturbed, "{benign:?}");
            assert!(!b.graphics_deleted, "{benign:?}");
        }

        // Clearing and scrolling disturb placements without deleting uploaded image data.
        for disturb in [
            b"\x1b[2J".as_slice(),
            b"\x1b[J",
            b"\n",
            b"\x1bD",
            b"\x1bE",
            b"\x1bM",
            b"\x1b[S",
            b"\x1b[2T",
            b"\x1b[L",
            b"\x1b[3M",
        ] {
            b.feed(disturb);
            assert!(b.placement_disturbed, "{disturb:?}");
            assert!(!b.graphics_deleted, "{disturb:?}");
        }

        // Terminal resets and child Kitty deletes may drop uploaded pixel data.
        for delete in [b"\x1b_Ga=d,d=A\x1b\\".as_slice(), b"\x1bc", b"\x1b[!p"] {
            b.feed(delete);
            assert!(b.graphics_deleted, "{delete:?}");
        }
    }
}
