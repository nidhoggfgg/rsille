//! A deliberately partial cursor model. Never inspect the renderer's own bytes.
use crate::boundary::Boundary;

type Cell = (u16, u16); // one-based terminal coordinates
pub struct Food {
    syntax: Boundary,
    cursor: Option<Cell>,
    seq: Vec<u8>,
    absolute_safe: bool,
}

impl Food {
    pub fn home() -> Self {
        Self {
            syntax: Boundary::default(),
            cursor: Some((1, 1)),
            seq: Vec::new(),
            absolute_safe: true,
        }
    }
    pub fn resize(&mut self) {
        self.cursor = None;
    }

    // None: no change; Some(None): withdraw; Some(Some(cell)): fresh glyph.
    pub fn feed(&mut self, bytes: &[u8], cols: u16, rows: u16) -> Option<Option<Cell>> {
        let mut change = None;
        for &b in bytes {
            let ground = self.syntax.ready();
            self.syntax.feed(&[b]);
            if !ground || !self.syntax.ready() {
                if ground {
                    self.seq.clear();
                    change = Some(None);
                }
                if self.seq.len() < 64 {
                    self.seq.push(b);
                }
                if self.syntax.ready() {
                    let seq = self.seq.as_slice();
                    if seq.len() < 64
                        && seq.starts_with(b"\x1b[")
                        && seq.ends_with(b"m")
                        && seq[2..seq.len() - 1]
                            .iter()
                            .all(|b| b.is_ascii_digit() || matches!(b, b';' | b':'))
                    {
                        continue;
                    }
                    if seq.starts_with(b"\x1b]0;")
                        || seq.starts_with(b"\x1b]2;")
                        || matches!(seq, b"\x1b[?2004h" | b"\x1b[?2004l")
                    {
                        continue;
                    }
                    self.cursor = None;
                    if self.absolute_safe
                        && seq.len() < 64
                        && seq.starts_with(b"\x1b[")
                        && matches!(seq.last(), Some(b'H' | b'f'))
                    {
                        let mut p = seq[2..seq.len() - 1].split(|&b| b == b';');
                        let mut param = || {
                            let v = p.next().unwrap_or_default();
                            if v.is_empty() {
                                Some(1)
                            } else {
                                std::str::from_utf8(v)
                                    .ok()?
                                    .parse::<u16>()
                                    .ok()
                                    .map(|n| n.max(1))
                            }
                        };
                        if let (Some(y), Some(x)) = (param(), param()) {
                            if p.next().is_none() && x <= cols && y <= rows {
                                self.cursor = Some((x, y));
                            }
                        }
                    } else if seq.starts_with(b"\x1b")
                        && !(seq.starts_with(b"\x1b[")
                            && matches!(seq.last(), Some(b'A'..=b'G' | b'J' | b'K')))
                    {
                        self.absolute_safe = false;
                    }
                }
                continue;
            }
            if matches!(b, 0 | 7) {
                continue;
            }
            change = Some(None);
            if let Some((x, y)) = self.cursor {
                self.cursor = match b {
                    b'\r' => Some((1, y)),
                    b'\n' => Some((x, y.saturating_add(1).min(rows))),
                    b' '..=b'~' if x < cols && y <= rows => {
                        if b != b' ' {
                            change = Some(Some((x, y)));
                        }
                        Some((x + 1, y))
                    }
                    _ => None,
                };
            }
        }
        change
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn positions_and_uncertainty() {
        let mut f = Food::home();
        assert_eq!(f.feed(b"ab\r\nc", 8, 2), Some(Some((1, 2))));
        assert_eq!(f.feed(b"\n", 8, 2), Some(None));
        assert_eq!(f.feed(b"\x1b[2", 8, 2), Some(None));
        assert_eq!(f.feed(b";3HX", 8, 2), Some(Some((3, 2))));
        f.resize();
        assert_eq!(f.feed(b"x", 8, 2), Some(None));
        assert_eq!(f.feed(b"\x1b[?6h\x1b[Hx", 8, 2), Some(None));
    }
}
