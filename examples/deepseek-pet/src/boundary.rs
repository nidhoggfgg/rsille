//! Tracks syntactic boundaries only; terminal modes and cursor state are not modeled.

#[derive(Clone, Copy, Default)]
enum State {
    #[default]
    Ground,
    Escape,
    Intermediate,
    Csi,
    String { osc: bool, esc: bool },
}

#[derive(Default)]
pub struct Boundary {
    state: State,
    continuation: u8,
}

impl Boundary {
    pub fn ready(&self) -> bool {
        matches!(self.state, State::Ground) && self.continuation == 0
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        use self::State::*;
        for &b in bytes {
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
            self.state = match (self.state, b) {
                (_, 0x18 | 0x1a | 0x9c) => Ground, // CAN, SUB, ST
                (String { osc: true, .. }, 7) | (String { esc: true, .. }, b'\\') => Ground,
                (String { osc, .. }, _) => String { osc, esc: b == 0x1b },
                (_, 0x1b) => Escape,
                (Escape, b'[') | (_, 0x9b) => Csi,
                (Escape, b']') | (_, 0x9d) => String { osc: true, esc: false },
                (Escape, b'P' | b'X' | b'^' | b'_') | (_, 0x90 | 0x98 | 0x9e | 0x9f) => {
                    String { osc: false, esc: false }
                }
                (Escape | Intermediate, 0x20..=0x2f) => Intermediate,
                (Escape | Intermediate, 0x30..=0x7e) | (Csi, 0x40..=0x7e) => Ground,
                (s, _) => s,
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Boundary;

    #[test]
    fn only_ready_after_final_byte() {
        for seq in [
            &b"\x1b[?25l"[..], b"\x1b[1;2 q", b"\x1b(B", b"\x1b#8", b"\x1b7",
            b"\x1b]0;title\x07", b"\x1b]0;t\x1b\\", b"\x1bPq\x07abc\x1b\\",
            b"\x9b31m", b"\x9fabc\x9c", b"\x9d\xc3\x9c\x07",
            "¢".as_bytes(), "界".as_bytes(), "🦀".as_bytes(),
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
}
