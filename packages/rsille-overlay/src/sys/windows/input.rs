use super::check;
use std::{
    io::{self, Write},
    os::windows::io::AsRawHandle,
};
use windows_sys::Win32::System::Console::*;

#[derive(Default)]
pub(super) struct Input {
    buttons: u32,
}

impl Input {
    pub fn read(&mut self, mut buf: &mut [u8], detach: Option<u8>) -> io::Result<usize> {
        let capacity = buf.len();
        // At most seven SGR reports per mouse record, each under 32 bytes.
        // Batch keystrokes from a paste into the same ConPTY pipe write.
        let mut records = [INPUT_RECORD::default(); 16];
        let limit = records.len().min(capacity / (7 * 32));
        if limit == 0 {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let mut count = 0;
        check(unsafe {
            ReadConsoleInputW(
                io::stdin().as_raw_handle(),
                records.as_mut_ptr(),
                limit as u32,
                &mut count,
            )
        })?;
        for record in &records[..count as usize] {
            if !self.encode(record, &mut buf, detach)? {
                return Ok(0);
            }
        }
        let written = capacity - buf.len();
        if written == 0 {
            Err(io::ErrorKind::WouldBlock.into())
        } else {
            Ok(written)
        }
    }
    // False means the configured detach key was pressed.
    fn encode(
        &mut self,
        record: &INPUT_RECORD,
        buf: &mut &mut [u8],
        detach: Option<u8>,
    ) -> io::Result<bool> {
        match u32::from(record.EventType) {
            KEY_EVENT => {
                let key = unsafe { record.Event.KeyEvent };
                let ch = unsafe { key.uChar.UnicodeChar };
                if key.bKeyDown != 0 && detach.is_some_and(|d| u16::from(d) == ch) {
                    return Ok(false);
                }
                // ConPTY reconstructs the original Win32 key record, including
                // UTF-16 surrogates, modifiers, repeat counts and key releases.
                write!(
                    buf,
                    "\x1b[{};{};{};{};{};{}_",
                    key.wVirtualKeyCode,
                    key.wVirtualScanCode,
                    ch,
                    u8::from(key.bKeyDown != 0),
                    key.dwControlKeyState,
                    key.wRepeatCount
                )?;
            }
            MOUSE_EVENT => {
                let mouse = unsafe { record.Event.MouseEvent };
                let mut info = CONSOLE_SCREEN_BUFFER_INFO::default();
                check(unsafe {
                    GetConsoleScreenBufferInfo(io::stdout().as_raw_handle(), &mut info)
                })?;
                let x = i32::from(mouse.dwMousePosition.X) - i32::from(info.srWindow.Left) + 1;
                let y = i32::from(mouse.dwMousePosition.Y) - i32::from(info.srWindow.Top) + 1;
                let flags = mouse.dwControlKeyState;
                let modifiers = 4 * u8::from(flags & SHIFT_PRESSED != 0)
                    + 8 * u8::from(flags & (LEFT_ALT_PRESSED | RIGHT_ALT_PRESSED) != 0)
                    + 16 * u8::from(flags & (LEFT_CTRL_PRESSED | RIGHT_CTRL_PRESSED) != 0);
                let mut emit = |button: u8, up: bool| {
                    write!(
                        buf,
                        "\x1b[<{};{};{}{}",
                        button | modifiers,
                        x.max(1),
                        y.max(1),
                        if up { 'm' } else { 'M' }
                    )
                };
                let buttons = mouse.dwButtonState & 0x1f;
                let mapping = [(1, 0), (4, 1), (2, 2), (8, 128), (16, 129)];
                for (mask, code) in mapping {
                    if (buttons ^ self.buttons) & mask != 0 {
                        emit(code, buttons & mask == 0)?;
                    }
                }
                self.buttons = buttons;
                if mouse.dwEventFlags == MOUSE_MOVED {
                    let button = mapping
                        .iter()
                        .find(|(mask, _)| buttons & mask != 0)
                        .map_or(3, |&(_, code)| code);
                    emit(button | 32, false)?;
                } else if matches!(mouse.dwEventFlags, MOUSE_WHEELED | MOUSE_HWHEELED) {
                    let down = (mouse.dwButtonState as i32) < 0;
                    emit(
                        if mouse.dwEventFlags == MOUSE_WHEELED {
                            64
                        } else {
                            66
                        } + u8::from(down),
                        false,
                    )?;
                }
            }
            FOCUS_EVENT => {
                if unsafe { record.Event.FocusEvent.bSetFocus } != 0 {
                    buf.write_all(b"\x1b[I")?;
                } else {
                    buf.write_all(b"\x1b[O")?;
                }
            }
            _ => {}
        }
        Ok(true)
    }
}
