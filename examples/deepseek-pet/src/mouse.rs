//! Decode only decimal SGR mouse reports. All other bytes retain their order.
use std::collections::VecDeque;

#[derive(Default)]
pub struct Mouse(Vec<u8>);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseKind {
    LeftPress,
    LeftDrag,
    LeftRelease,
    RightPress,
    Motion,
    Other,
}

/// Decides whether to follow a pointer, and for how long.
#[derive(Default)]
pub struct Pursuit {
    pub target: Option<(u16, u16)>,
    pub curiosity: u32,
    remaining: u16,
}

impl Pursuit {
    pub fn tick(&mut self) -> Option<(u16, u16)> {
        if self.remaining == 0 {
            self.target = None;
        }
        self.remaining = self.remaining.saturating_sub(1);
        self.target
    }

    pub fn motion(&mut self, cell: (u16, u16), receptive: bool, cooldown: &mut u16) {
        if self.target.is_some() {
            self.target = Some(cell);
        } else if receptive && *cooldown == 0 {
            self.curiosity = self
                .curiosity
                .wrapping_mul(1664525)
                .wrapping_add(1013904223);
            *cooldown = 5;
            if self.curiosity >> 31 == 0 {
                self.remaining = 32;
                *cooldown = 48;
                self.target = Some(cell);
            }
        }
    }

    pub fn new(curiosity: u32) -> Self {
        Self {
            curiosity,
            ..Self::default()
        }
    }
}

impl Mouse {
    pub fn has_partial_report(&self) -> bool {
        !self.0.is_empty()
    }
    pub fn flush(&mut self, out: &mut VecDeque<u8>) {
        out.extend(self.0.drain(..));
    }

    pub fn feed(
        &mut self,
        bytes: &[u8],
        out: &mut VecDeque<u8>,
        mut report: impl FnMut(MouseKind, u16, u16),
    ) {
        for &b in bytes {
            if b == 27 {
                self.flush(out);
                self.0.push(b);
                continue;
            }
            if self.0.is_empty() {
                out.push_back(b);
                continue;
            }
            self.0.push(b);
            if b"\x1b[<".starts_with(&self.0) {
                continue;
            }
            if let Some(fields) = self.0.strip_prefix(b"\x1b[<") {
                let done = matches!(b, b'M' | b'm');
                let body = if done {
                    &fields[..fields.len() - 1]
                } else {
                    fields
                };
                let mut values = [0u16; 3];
                let mut field = 0;
                let mut digits = 0;
                let valid = body.iter().all(|&c| {
                    if c == b';' && digits > 0 && field < 2 {
                        field += 1;
                        digits = 0;
                        true
                    } else if c.is_ascii_digit() && digits < 5 {
                        digits += 1;
                        if let Some(v) = values[field]
                            .checked_mul(10)
                            .and_then(|v| v.checked_add((c - b'0') as u16))
                        {
                            values[field] = v;
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                });
                if valid && !done {
                    continue;
                }
                if valid && field == 2 && digits > 0 && values[1] > 0 && values[2] > 0 {
                    let button = values[0] & !(4 | 8 | 16); // ignore keyboard modifiers
                    let kind = match (b, button) {
                        (b'M', 0) => MouseKind::LeftPress,
                        (b'M', 32) => MouseKind::LeftDrag,
                        (b'm', 0) => MouseKind::LeftRelease,
                        (b'M', 2) => MouseKind::RightPress,
                        (b'M', 33..=35) => MouseKind::Motion,
                        _ => MouseKind::Other,
                    };
                    report(kind, values[1], values[2]);
                    self.0.clear();
                    continue;
                }
            }
            self.flush(out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn distinguishes_presses_from_release_drag_and_scroll() {
        let mut mouse = Mouse::default();
        let mut out = VecDeque::new();
        let mut kinds = vec![];
        let bytes = b"\x1b[<4;8;9M\x1b[<0;8;9m\x1b[<32;8;9M\x1b[<35;8;9M\x1b[<64;8;9M\x1b[<2;8;9M";
        for byte in bytes.chunks(1) {
            mouse.feed(byte, &mut out, |kind, _, _| kinds.push(kind));
        }
        use MouseKind::*;
        assert_eq!(
            kinds,
            [LeftPress, LeftRelease, LeftDrag, Motion, Other, RightPress]
        );
        assert!(out.is_empty());
    }

    #[test]
    fn mixed_split_input() {
        let raw = b"a\x1b[A\x1b[<1;0;3M\x1b[<0;65536;2M\x1b\x1b[<32;12;5mz\x1b";
        for split in 0..=raw.len() {
            let mut m = Mouse::default();
            let mut out = VecDeque::new();
            let mut hits = vec![];
            for part in [&raw[..split], &raw[split..]] {
                m.feed(part, &mut out, |_, x, y| hits.push((x, y)));
            }
            m.flush(&mut out);
            assert_eq!(hits, [(12, 5)]);
            assert_eq!(
                out.make_contiguous(),
                b"a\x1b[A\x1b[<1;0;3M\x1b[<0;65536;2M\x1bz\x1b"
            );
        }
    }
}
