//! Decode only decimal SGR mouse reports. All other bytes retain their order.
use std::collections::VecDeque;

#[derive(Default)]
pub struct Mouse {
    partial: Vec<u8>,
    paste: bool,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct MouseModes {
    pub tracking: u16,
    pub encoding: u16,
}

#[derive(Clone, Copy)]
pub struct Report<'a> {
    pub button: u16,
    pub release: bool,
    pub x: u16,
    pub y: u16,
    raw: &'a [u8],
}

impl Report<'_> {
    pub fn kind(self) -> MouseKind {
        match (self.release, self.button & !(4 | 8 | 16)) {
            (false, 0) => MouseKind::LeftPress,
            (false, 32) => MouseKind::LeftDrag,
            (true, 0) => MouseKind::LeftRelease,
            (false, 2) => MouseKind::RightPress,
            (false, 33..=35) => MouseKind::Motion,
            _ => MouseKind::Other,
        }
    }

    pub fn cell(self, encoding: u16, pixels: (u16, u16)) -> (u16, u16) {
        if encoding == 1016 {
            (
                (self.x - 1) / pixels.0.max(1) + 1,
                (self.y - 1) / pixels.1.max(1) + 1,
            )
        } else {
            (self.x, self.y)
        }
    }

    pub fn forward(self, modes: MouseModes, out: &mut VecDeque<u8>) {
        let motion = self.button & 32 != 0;
        let subscribed = match modes.tracking {
            9 => !self.release && self.button & !(4 | 8 | 16) <= 2,
            1000 | 1001 => !motion,
            1002 => !motion || self.button & 3 != 3,
            1003 => true,
            _ => false,
        };
        if !subscribed {
            return;
        }
        let cb = if modes.tracking == 9 {
            self.button & 3
        } else if self.release {
            self.button & (4 | 8 | 16) | 3
        } else {
            self.button
        };
        if matches!(modes.encoding, 1006 | 1016) {
            if cb != self.button && modes.tracking == 9 {
                use std::io::Write;
                write!(out, "\x1b[<{cb};{};{}M", self.x, self.y).unwrap();
            } else {
                out.extend(self.raw);
            }
        } else if modes.encoding == 1015 {
            use std::io::Write;
            write!(out, "\x1b[{};{};{}M", u32::from(cb) + 32, self.x, self.y).unwrap();
        } else {
            let fields = [cb, self.x, self.y];
            let limit = if modes.encoding == 1005 { 2015 } else { 223 };
            if fields.iter().any(|&v| v > limit) {
                return;
            }
            out.extend(b"\x1b[M");
            for v in fields {
                if modes.encoding == 1005 {
                    out.extend(
                        char::from_u32(u32::from(v) + 32)
                            .unwrap()
                            .encode_utf8(&mut [0; 4])
                            .as_bytes(),
                    );
                } else {
                    out.push_back(v as u8 + 32);
                }
            }
        }
    }
}

/// Each button keeps the recipient of its press through motion and release.
#[derive(Default)]
pub struct Buttons(u16);

impl Buttons {
    pub fn pet_owns(&mut self, r: Report<'_>, hit: bool) -> bool {
        let wheel = r.button & 64 != 0;
        let hover = r.button & 32 != 0 && r.button & 3 == 3;
        // The overlay handles clicks and drags. The child keeps its subscribed
        // hover and wheel events, including over the image.
        if wheel || hover {
            return false;
        }
        let index = (r.button & 3) + if r.button & 128 != 0 { 4 } else { 0 };
        let bit = 1 << index;
        if r.release {
            let owned = self.0 & bit != 0;
            self.0 &= !bit;
            owned
        } else if r.button & 32 != 0 {
            self.0 & bit != 0
        } else {
            if hit {
                self.0 |= bit;
            } else {
                self.0 &= !bit;
            }
            hit
        }
    }
}

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
        !self.partial.is_empty()
    }
    pub fn flush(&mut self, out: &mut VecDeque<u8>) {
        out.extend(self.partial.drain(..));
    }

    pub fn feed(
        &mut self,
        bytes: &[u8],
        out: &mut VecDeque<u8>,
        mut report: impl FnMut(Report, &mut VecDeque<u8>),
    ) {
        for &b in bytes {
            if b == 27 {
                self.flush(out);
                self.partial.push(b);
                continue;
            }
            if self.partial.is_empty() {
                out.push_back(b);
                continue;
            }
            self.partial.push(b);
            let marker = if self.paste {
                b"\x1b[201~"
            } else {
                b"\x1b[200~"
            };
            if marker.starts_with(&self.partial) {
                if self.partial == marker {
                    self.paste = !self.paste;
                    self.flush(out);
                }
                continue;
            }
            if self.paste {
                self.flush(out);
                continue;
            }
            if b"\x1b[<".starts_with(&self.partial) {
                continue;
            }
            if let Some(fields) = self.partial.strip_prefix(b"\x1b[<") {
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
                    report(
                        Report {
                            button: values[0],
                            release: b == b'm',
                            x: values[1],
                            y: values[2],
                            raw: &self.partial,
                        },
                        out,
                    );
                    self.partial.clear();
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
    fn a_split_gesture_belongs_to_its_press_and_paste_is_literal() {
        let mut mouse = Mouse::default();
        let mut buttons = Buttons::default();
        let mut out = VecDeque::new();
        let pet_drag = b"\x1b[<0;10;1M\x1b[<32;2;1M\x1b[<0;2;1m";
        let app_drag = b"\x1b[<4;2;1M\x1b[<36;10;1M\x1b[<4;10;1m";
        let wheel = b"\x1b[<64;10;1M";
        let paste = b"\x1b[200~\x1b[<0;10;1M\x1b[201~";
        for &byte in pet_drag.iter().chain(app_drag).chain(wheel).chain(paste) {
            mouse.feed(&[byte], &mut out, |r, out| {
                if !buttons.pet_owns(r, r.x == 10) {
                    r.forward(
                        MouseModes {
                            tracking: 1002,
                            encoding: 1006,
                        },
                        out,
                    );
                }
            });
        }
        assert_eq!(
            out.make_contiguous(),
            [app_drag.as_slice(), wheel, paste].concat()
        );
    }

    #[test]
    fn forward_only_subscribed_events_in_the_child_encoding() {
        let bytes = b"\x1b[<4;4;5M\x1b[<36;7;8M\x1b[<4;7;8m";
        for (tracking, encoding, expected) in [
            (0, 1006, b"".as_slice()),
            (1000, 0, b"\x1b[M$$%\x1b[M''("),
            (1000, 1015, b"\x1b[36;4;5M\x1b[39;7;8M"),
            (1002, 1006, bytes),
        ] {
            let mut out = VecDeque::new();
            Mouse::default().feed(bytes, &mut out, |r, out| {
                r.forward(MouseModes { tracking, encoding }, out);
            });
            assert_eq!(out.make_contiguous(), expected);
        }
        let mut out = VecDeque::new();
        Mouse::default().feed(b"\x1b[<0;101;40M", &mut out, |r, out| {
            assert_eq!(r.cell(1016, (10, 20)), (11, 2));
            r.forward(
                MouseModes {
                    tracking: 1002,
                    encoding: 1016,
                },
                out,
            );
        });
        assert_eq!(out.make_contiguous(), b"\x1b[<0;101;40M");
    }

    #[test]
    fn distinguishes_presses_from_release_drag_and_scroll() {
        let mut mouse = Mouse::default();
        let mut out = VecDeque::new();
        let mut kinds = vec![];
        let bytes = b"\x1b[<4;8;9M\x1b[<0;8;9m\x1b[<32;8;9M\x1b[<35;8;9M\x1b[<64;8;9M\x1b[<2;8;9M";
        for byte in bytes.chunks(1) {
            mouse.feed(byte, &mut out, |r, _| kinds.push(r.kind()));
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
                m.feed(part, &mut out, |r, _| hits.push((r.x, r.y)));
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
