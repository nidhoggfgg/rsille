use super::platform::{exchange, Tty};
use std::{io, time::Instant};

pub(super) fn cell_pixels(tty: &mut Tty, deadline: Instant) -> io::Result<(u16, u16)> {
    let mut scan = Scan::default();
    exchange(tty, deadline, b"\x1b[16t", |byte| scan.push(byte))
}

pub(super) fn protocols(
    tty: &mut Tty,
    deadline: Instant,
    kitty: bool,
    sixel: bool,
) -> io::Result<(bool, bool)> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static QUERY_ID: AtomicU32 = AtomicU32::new(1);
    let id = QUERY_ID.fetch_add(1, Ordering::Relaxed).max(1);
    let mut query = String::new();
    if kitty {
        query = format!("\x1b_Ga=q,i={id},s=1,v=1,f=24,t=d;AAAA\x1b\\");
    }
    if sixel {
        query.push_str("\x1b[c");
    }
    let prefix = format!("\x1b_Gi={id};");
    let mut kitty_reply = None;
    let mut da1 = None;
    let mut sequence = Vec::with_capacity(256);
    let mut esc = false;
    let result = exchange(tty, deadline, query.as_bytes(), |byte| {
        if esc && byte == b'\\' {
            if kitty {
                if let Some(status) = sequence.strip_prefix(prefix.as_bytes()) {
                    kitty_reply = Some(status == b"OK");
                }
            }
            sequence.clear();
        } else {
            if esc {
                sequence.clear();
                sequence.push(0x1b);
            }
            if byte != 0x1b && !sequence.is_empty() {
                if sixel && byte == b'c' && sequence.starts_with(b"\x1b[") {
                    if let Some(support) = da1_sixel(&sequence) {
                        da1 = Some(support);
                    }
                    sequence.clear();
                } else if sequence.len() < 256 {
                    sequence.push(byte);
                } else {
                    sequence.clear();
                }
            }
        }
        esc = byte == 0x1b;
        ((!kitty || kitty_reply.is_some()) && (!sixel || da1.is_some())).then_some(())
    });
    match result {
        // A terminal may implement just one of the two queries.
        Err(e)
            if e.kind() == io::ErrorKind::TimedOut && (kitty_reply.is_some() || da1.is_some()) => {}
        other => other?,
    }
    Ok((
        kitty && kitty_reply == Some(true),
        sixel && da1 == Some(true),
    ))
}

pub(super) fn da1_sixel(reply: &[u8]) -> Option<bool> {
    let params = std::str::from_utf8(reply.strip_prefix(b"\x1b[?")?).ok()?;
    let mut fields = params.split(';').map(|field| {
        if field.is_empty() || !field.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        field.parse::<u16>().ok()
    });
    let class = fields.next()??;
    let mut sixel = false;
    for field in fields {
        sixel |= field? == 4;
    }
    // VT100-style reports encode option bits instead of a feature list.
    Some(class >= 60 && sixel)
}

// Constant-memory matching, including arbitrarily long numeric noise.
#[derive(Default)]
pub(super) enum Scan {
    #[default]
    Seeking,
    Prefix(usize),
    Height(Option<u16>),
    Width(u16, Option<u16>),
}

impl Scan {
    pub(super) fn push(&mut self, byte: u8) -> Option<(u16, u16)> {
        if byte == 0x1b {
            *self = Self::Prefix(0);
            return None;
        }
        let mut pixels = None;
        *self = match *self {
            Self::Prefix(index) if byte == b"[6;"[index] => {
                if index == 2 {
                    Self::Height(None)
                } else {
                    Self::Prefix(index + 1)
                }
            }
            Self::Height(value) if byte.is_ascii_digit() => {
                Self::number(value, byte).map_or(Self::Seeking, |n| Self::Height(Some(n)))
            }
            Self::Height(Some(height)) if height > 0 && byte == b';' => Self::Width(height, None),
            Self::Width(height, value) if byte.is_ascii_digit() => {
                Self::number(value, byte).map_or(Self::Seeking, |n| Self::Width(height, Some(n)))
            }
            Self::Width(height, Some(width)) if width > 0 && byte == b't' => {
                pixels = Some((width, height));
                Self::Seeking
            }
            _ => Self::Seeking,
        };
        pixels
    }

    fn number(value: Option<u16>, digit: u8) -> Option<u16> {
        value
            .unwrap_or(0)
            .checked_mul(10)?
            .checked_add(u16::from(digit - b'0'))
    }
}
