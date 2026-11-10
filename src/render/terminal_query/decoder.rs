//! Bounded framing around Crossterm's complete Unix event parser.
use super::{parser, InternalEvent, Pixels};
use crossterm::event::Event;
use std::{
    io,
    time::{Duration, Instant},
};

const MAX_SEQUENCE: usize = 1024;
const MAX_PASTE: usize = 1024 * 1024;
const ESC_DELAY: Duration = Duration::from_millis(30);
const INCOMPLETE_IDLE: Duration = Duration::from_secs(2);
const PASTE_START: &[u8] = b"\x1b[200~";
const PASTE_END: &[u8] = b"\x1b[201~";

#[derive(Debug)]
pub(super) enum Decoded {
    Event(Event),
    Pixels(io::Result<Pixels>),
    CursorPosition(u16, u16),
    KeyboardFlags,
    DeviceAttributes,
    Error(io::Error),
}

#[derive(Default)]
pub(super) struct Decoder {
    bytes: Vec<u8>,
    last_byte: Option<Instant>,
    paste: bool,
    discard_paste: bool,
    discard_csi: bool,
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

/// CSI 6 ; height ; width t is deliberately converted to (width, height).
/// Restrict both syntax and magnitude, rather than accepting signed numbers or
/// trusting a terminal's reported allocation sizes.
fn pixels(bytes: &[u8]) -> io::Result<Pixels> {
    let body = bytes
        .strip_prefix(b"\x1b[6;")
        .and_then(|body| body.strip_suffix(b"t"))
        .ok_or_else(|| invalid("invalid cell pixel report"))?;
    let mut fields = body.split(|byte| *byte == b';');
    fn dimension(bytes: Option<&[u8]>) -> io::Result<u16> {
        let bytes = bytes.ok_or_else(|| invalid("missing cell pixel dimension"))?;
        if bytes.is_empty() || bytes.len() > 5 || !bytes.iter().all(u8::is_ascii_digit) {
            return Err(invalid("invalid cell pixel dimension"));
        }
        let value = bytes
            .iter()
            .try_fold(0u16, |value, byte| {
                value.checked_mul(10)?.checked_add(u16::from(byte - b'0'))
            })
            .filter(|value| *value != 0);
        value.ok_or_else(|| invalid("cell pixel dimension is zero or exceeds u16"))
    }
    let height = dimension(fields.next())?;
    let width = dimension(fields.next())?;
    if fields.next().is_some() {
        return Err(invalid("extra cell pixel dimensions"));
    }
    Ok((width, height))
}

impl Decoder {
    pub(super) fn push(&mut self, byte: u8, now: Instant) -> Option<Decoded> {
        self.last_byte = Some(now);
        if self.paste {
            self.bytes.push(byte);
            if self.discard_paste && self.bytes.len() > PASTE_END.len() {
                self.bytes.remove(0);
            }
            if self.bytes.ends_with(PASTE_END) {
                let result = if self.discard_paste {
                    None
                } else {
                    self.parse()
                };
                self.clear();
                return result;
            }
            if !self.discard_paste && self.bytes.len() >= MAX_PASTE {
                self.bytes = self.bytes[self.bytes.len() - PASTE_END.len()..].to_vec();
                self.discard_paste = true;
                return Some(Decoded::Error(invalid(
                    "bracketed paste exceeds 1 MiB; discarded",
                )));
            }
            return None;
        }
        if self.discard_csi {
            if byte == 0x1b {
                self.clear();
            } else {
                if (0x40..=0x7e).contains(&byte) {
                    self.clear();
                }
                return None;
            }
        }
        // An escape begins a new frame after an unfinished sequence. Do not do
        // this inside X10 mouse payload, where ESC is data, or bracketed paste.
        // ESC ESC retains the upstream parser's legacy Escape behavior.
        if byte == 0x1b && self.bytes.len() > 1 && !self.bytes.starts_with(b"\x1b[M") {
            self.clear();
            self.last_byte = Some(now);
        }
        // Recover a normal character following malformed/truncated UTF-8.
        let utf8_offset = usize::from(self.bytes.first() == Some(&0x1b));
        if self
            .bytes
            .get(utf8_offset)
            .is_some_and(|first| *first >= 0xc0)
            && byte & 0xc0 != 0x80
        {
            self.clear();
            self.last_byte = Some(now);
        }
        self.bytes.push(byte);
        if self.bytes == PASTE_START {
            self.paste = true;
            return None;
        }
        if self.bytes.len() > MAX_SEQUENCE {
            let pixel_report = self.bytes.starts_with(b"\x1b[6;");
            self.clear();
            self.discard_csi = !(0x40..=0x7e).contains(&byte);
            self.last_byte = Some(now);
            let error = invalid("terminal escape sequence exceeds 1024 bytes");
            return Some(if pixel_report {
                Decoded::Pixels(Err(error))
            } else {
                Decoded::Error(error)
            });
        }
        if self.bytes.starts_with(b"\x1b[")
            && self.bytes.get(2).is_some_and(u8::is_ascii_digit)
            && byte == b't'
        {
            let result = self
                .bytes
                .starts_with(b"\x1b[6;")
                .then(|| Decoded::Pixels(pixels(&self.bytes)));
            // Other window-operation replies are internal, too.
            self.clear();
            return result;
        }
        let result = self.parse();
        if result.is_some() {
            self.clear();
        } else if self.bytes.starts_with(b"\x1b[?")
            && self.bytes.len() > 3
            && (0x40..=0x7e).contains(&byte)
        {
            // Unknown private replies must not keep the parser waiting forever.
            self.clear();
        }
        result
    }

    fn parse(&mut self) -> Option<Decoded> {
        match parser::parse_event(&self.bytes, true) {
            Ok(Some(InternalEvent::Event(event))) => Some(Decoded::Event(event)),
            Ok(Some(InternalEvent::KeyboardEnhancementFlags(_))) => Some(Decoded::KeyboardFlags),
            Ok(Some(InternalEvent::PrimaryDeviceAttributes)) => Some(Decoded::DeviceAttributes),
            Ok(Some(InternalEvent::CursorPosition(column, row))) => {
                Some(Decoded::CursorPosition(column, row))
            }
            Ok(None) => None,
            Err(_) => Some(Decoded::Error(invalid("malformed terminal input sequence"))),
        }
    }

    pub(super) fn expire(&mut self, now: Instant) -> Option<Decoded> {
        let elapsed = now.saturating_duration_since(self.last_byte?);
        if self.bytes == b"\x1b" && !self.paste && elapsed >= ESC_DELAY {
            self.clear();
            return Some(Decoded::Event(Event::Key(
                crossterm::event::KeyCode::Esc.into(),
            )));
        }
        if elapsed >= INCOMPLETE_IDLE {
            let incomplete = !self.bytes.is_empty() || self.discard_csi || self.paste;
            self.clear();
            if incomplete {
                return Some(Decoded::Error(invalid(
                    "incomplete terminal input sequence expired",
                )));
            }
        }
        None
    }

    fn clear(&mut self) {
        self.bytes.clear();
        // Do not retain a megabyte allocation after a large completed paste.
        if self.bytes.capacity() > MAX_SEQUENCE * 2 {
            self.bytes = Vec::new();
        }
        self.last_byte = None;
        self.paste = false;
        self.discard_paste = false;
        self.discard_csi = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers, MouseEventKind};

    fn feed(decoder: &mut Decoder, bytes: &[u8]) -> Vec<Decoded> {
        bytes
            .iter()
            .filter_map(|byte| decoder.push(*byte, Instant::now()))
            .collect()
    }

    #[test]
    fn fragmented_report_has_width_height_order_and_preserves_input() {
        let bytes = b"a\x1b[6;19;8t\x1b[<0;3;4Mz\x1b[1;5A";
        for split in 0..=bytes.len() {
            let mut decoder = Decoder::default();
            let mut events = feed(&mut decoder, &bytes[..split]);
            events.extend(feed(&mut decoder, &bytes[split..]));
            assert_eq!(events.len(), 5, "split {split}: {events:?}");
            assert!(
                matches!(&events[0], Decoded::Event(Event::Key(key)) if key.code == KeyCode::Char('a'))
            );
            assert!(matches!(events[1], Decoded::Pixels(Ok((8, 19)))));
            assert!(
                matches!(&events[2], Decoded::Event(Event::Mouse(mouse)) if mouse.column == 2 && mouse.row == 3)
            );
            assert!(
                matches!(&events[3], Decoded::Event(Event::Key(key)) if key.code == KeyCode::Char('z'))
            );
            assert!(
                matches!(&events[4], Decoded::Event(Event::Key(key)) if key.code == KeyCode::Up && key.modifiers == KeyModifiers::CONTROL)
            );
        }
        assert_eq!(pixels(b"\x1b[6;8;19t").unwrap(), (19, 8));
    }

    #[test]
    fn rejects_zero_oversized_and_malformed_reports() {
        for report in [
            &b"\x1b[6;0;8t"[..],
            b"\x1b[6;19;0t",
            b"\x1b[6;65536;8t",
            b"\x1b[6;19;999999999999999999999t",
            b"\x1b[6;;8t",
            b"\x1b[6;19;8;1t",
        ] {
            let mut decoder = Decoder::default();
            let events = feed(&mut decoder, report);
            assert!(
                matches!(&events[..], [Decoded::Pixels(Err(error))] if error.kind() == io::ErrorKind::InvalidData),
                "{events:?}"
            );
            assert!(matches!(
                &feed(&mut decoder, b"k")[..],
                [Decoded::Event(Event::Key(_))]
            ));
        }
        assert_eq!(pixels(b"\x1b[6;65535;65535t").unwrap(), (65535, 65535));
    }

    #[test]
    fn reply_text_in_fragmented_paste_is_not_a_reply() {
        let text = "x\x1b[6;19;8t\x1b[?15u\x1b[?1;2c\x1b[12;30Ry";
        let bytes = format!("\x1b[200~{text}\x1b[201~\x1b[6;20;9t");
        let events = feed(&mut Decoder::default(), bytes.as_bytes());
        assert!(
            matches!(&events[..], [Decoded::Event(Event::Paste(paste)), Decoded::Pixels(Ok((9, 20)))] if paste == text)
        );
    }

    #[test]
    fn full_parser_keeps_focus_mouse_utf8_and_enhanced_key_events() {
        let events = feed(
            &mut Decoder::default(),
            "é\x1b[I\x1b[O\x1b[97;5:3u\x1b[<64;4;5M\x1bOP\x1b[57399u".as_bytes(),
        );
        assert!(
            matches!(&events[0], Decoded::Event(Event::Key(key)) if key.code == KeyCode::Char('é'))
        );
        assert!(matches!(events[1], Decoded::Event(Event::FocusGained)));
        assert!(matches!(events[2], Decoded::Event(Event::FocusLost)));
        assert!(
            matches!(&events[3], Decoded::Event(Event::Key(key)) if key.kind == KeyEventKind::Release && key.modifiers == KeyModifiers::CONTROL)
        );
        assert!(
            matches!(&events[4], Decoded::Event(Event::Mouse(mouse)) if mouse.kind == MouseEventKind::ScrollUp)
        );
        assert!(matches!(&events[5], Decoded::Event(Event::Key(key)) if key.code == KeyCode::F(1)));
        assert!(
            matches!(&events[6], Decoded::Event(Event::Key(key)) if key.state.contains(crossterm::event::KeyEventState::KEYPAD))
        );
    }

    #[test]
    fn late_replies_are_internal_and_keyboard_response_order_is_preserved() {
        for bytes in [&b"\x1b[?1;2c\x1b[?15u"[..], &b"\x1b[?15u\x1b[?1;2c"[..]] {
            let events = feed(&mut Decoder::default(), bytes);
            assert_eq!(events.len(), 2);
            assert!(events
                .iter()
                .any(|event| matches!(event, Decoded::KeyboardFlags)));
            assert!(events
                .iter()
                .any(|event| matches!(event, Decoded::DeviceAttributes)));
        }
        let events = feed(&mut Decoder::default(), b"\x1b[6;19;8t\x1b[12;30Rk");
        assert_eq!(events.len(), 3);
        assert!(matches!(events[0], Decoded::Pixels(Ok(_))));
        assert!(matches!(events[1], Decoded::CursorPosition(29, 11)));
        assert!(matches!(events[2], Decoded::Event(Event::Key(_))));
    }

    #[test]
    fn fragmented_cursor_reports_are_zero_based_column_row_with_adjacent_keys() {
        let bytes = b"a\x1b[12;30Rz\x1b[1;1R\x1b[65535;65535R";
        for split in 0..=bytes.len() {
            let mut decoder = Decoder::default();
            let mut events = feed(&mut decoder, &bytes[..split]);
            events.extend(feed(&mut decoder, &bytes[split..]));
            assert!(
                matches!(&events[..], [
                Decoded::Event(Event::Key(first)),
                Decoded::CursorPosition(29, 11),
                Decoded::Event(Event::Key(last)),
                Decoded::CursorPosition(0, 0),
                Decoded::CursorPosition(65534, 65534),
            ] if first.code == KeyCode::Char('a') && last.code == KeyCode::Char('z')),
                "split {split}: {events:?}"
            );
        }
    }

    #[test]
    fn x10_mouse_payload_is_not_a_window_reply() {
        let events = feed(&mut Decoder::default(), b"\x1b[M tt\x1b[6;19;8t");
        assert!(
            matches!(&events[..], [Decoded::Event(Event::Mouse(mouse)), Decoded::Pixels(Ok((8, 19)))] if mouse.column == 83 && mouse.row == 83)
        );
    }

    #[test]
    fn malformed_coordinates_do_not_panic_or_disable_keyboard() {
        for bytes in [
            &b"\x1b[<0;0;0M"[..],
            b"\x1b[32;0;0M",
            b"\x1b[M\x20\x00\x00",
            b"\x1b[0;0R",
        ] {
            let mut decoder = Decoder::default();
            let events = feed(&mut decoder, bytes);
            assert!(matches!(&events[..], [Decoded::Error(_)]), "{events:?}");
            assert!(matches!(
                &feed(&mut decoder, b"z")[..],
                [Decoded::Event(Event::Key(_))]
            ));
        }
    }

    #[test]
    fn bounded_paste_and_sequences_recover_at_their_terminator() {
        let mut decoder = Decoder::default();
        feed(&mut decoder, PASTE_START);
        let events = feed(&mut decoder, &vec![b'x'; MAX_PASTE + 100]);
        assert_eq!(events.len(), 1);
        assert!(decoder.bytes.len() <= PASTE_END.len());
        assert!(feed(&mut decoder, b"\x1b[6;20;9t").is_empty());
        let events = feed(&mut decoder, b"\x1b[201~k");
        assert!(
            matches!(&events[..], [Decoded::Event(Event::Key(key))] if key.code == KeyCode::Char('k'))
        );
        feed(&mut decoder, b"\x1b[6;");
        let events = feed(&mut decoder, &vec![b'9'; MAX_SEQUENCE * 4]);
        assert!(matches!(&events[..], [Decoded::Pixels(Err(_))]));
        let events = feed(&mut decoder, b"tk");
        assert!(matches!(&events[..], [Decoded::Event(Event::Key(_))]));
    }

    #[test]
    fn escape_and_abandoned_sequences_have_bounded_idle_lifetimes() {
        let mut decoder = Decoder::default();
        let now = Instant::now();
        decoder.push(0x1b, now);
        assert!(decoder.expire(now + ESC_DELAY / 2).is_none());
        assert!(
            matches!(decoder.expire(now + ESC_DELAY), Some(Decoded::Event(Event::Key(key))) if key.code == KeyCode::Esc)
        );
        feed(&mut decoder, b"\x1b[200~unfinished");
        assert!(matches!(
            decoder.expire(Instant::now() + INCOMPLETE_IDLE),
            Some(Decoded::Error(_))
        ));
        assert!(matches!(
            &feed(&mut decoder, b"k")[..],
            [Decoded::Event(Event::Key(_))]
        ));
        feed(&mut decoder, b"\x1b[?123");
        let events = feed(&mut decoder, b"\x1b[6;19;8t");
        assert!(matches!(&events[..], [Decoded::Pixels(Ok((8, 19)))]));
    }
}
