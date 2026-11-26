//! Startup queries on the native console. Reading one input record at a time
//! leaves keys after the final reply queued for the session.
use std::{
    io::{self, Write},
    os::windows::io::AsRawHandle,
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT},
    System::{Console::*, Threading::WaitForSingleObject},
};

fn check(ok: i32) -> io::Result<()> {
    if ok == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

struct Mode {
    handle: HANDLE,
    saved: u32,
    restored: bool,
}
impl Mode {
    fn set(handle: HANDLE, enable: u32, disable: u32) -> io::Result<Self> {
        let mut saved = 0;
        check(unsafe { GetConsoleMode(handle, &mut saved) })?;
        check(unsafe { SetConsoleMode(handle, (saved | enable) & !disable) })?;
        Ok(Self {
            handle,
            saved,
            restored: false,
        })
    }
    fn restore(&mut self) -> io::Result<()> {
        check(unsafe { SetConsoleMode(self.handle, self.saved) })?;
        self.restored = true;
        Ok(())
    }
}
impl Drop for Mode {
    fn drop(&mut self) {
        if !self.restored {
            let _ = self.restore();
        }
    }
}

pub(super) struct Tty(HANDLE);

pub(super) fn query<T>(
    timeout: Duration,
    operation: impl FnOnce(&mut Tty, Instant) -> io::Result<T>,
) -> io::Result<T> {
    let deadline = Instant::now()
        .checked_add(timeout)
        .ok_or(io::ErrorKind::InvalidInput)?;
    let mut input = Mode::set(
        io::stdin().as_raw_handle(),
        ENABLE_VIRTUAL_TERMINAL_INPUT,
        ENABLE_ECHO_INPUT | ENABLE_LINE_INPUT | ENABLE_PROCESSED_INPUT,
    )?;
    let mut output = Mode::set(
        io::stdout().as_raw_handle(),
        ENABLE_VIRTUAL_TERMINAL_PROCESSING | ENABLE_PROCESSED_OUTPUT,
        0,
    )?;
    let result = operation(&mut Tty(input.handle), deadline);
    output.restore()?;
    input.restore()?;
    result
}

pub(super) fn exchange<T>(
    tty: &mut Tty,
    deadline: Instant,
    query: &[u8],
    mut scan: impl FnMut(u8) -> Option<T>,
) -> io::Result<T> {
    let mut out = io::stdout().lock();
    out.write_all(query)?;
    out.flush()?;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(io::ErrorKind::TimedOut.into());
        }
        let ms = left
            .as_nanos()
            .div_ceil(1_000_000)
            .min(u128::from(u32::MAX - 1)) as u32;
        match unsafe { WaitForSingleObject(tty.0, ms) } {
            WAIT_OBJECT_0 => {}
            WAIT_TIMEOUT => continue,
            _ => return Err(io::Error::last_os_error()),
        }
        let mut record = INPUT_RECORD::default();
        let mut count = 0;
        check(unsafe { ReadConsoleInputW(tty.0, &mut record, 1, &mut count) })?;
        if u32::from(record.EventType) == KEY_EVENT {
            let key = unsafe { record.Event.KeyEvent };
            if key.bKeyDown != 0 {
                if let Ok(byte) = u8::try_from(unsafe { key.uChar.UnicodeChar }) {
                    if let Some(reply) = scan(byte) {
                        return Ok(reply);
                    }
                }
            }
        }
    }
}
