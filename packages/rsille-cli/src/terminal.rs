//! Fullscreen host for Unix and VT-capable Windows consoles.
//! The entry point must reject inline mode first.
//!
//! Call once, after consuming input data, with exclusive ownership of console
//! input and terminal modes. No interface bytes are written to stdout.

pub fn run(config: crate::config::Config) -> Result<Option<serde_json::Value>, String> {
    #[cfg(any(unix, windows))]
    {
        host::run(config)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = config;
        Err("the interactive CLI requires Unix or Windows".into())
    }
}

#[cfg(windows)]
mod windows {
    use crossterm::{
        cursor::MoveTo,
        event::{Event, KeyCode, KeyEvent, KeyModifiers},
        style::{ResetColor, SetAttributes, SetColors},
        Command,
    };
    use rsille::render::buffer::Buffer;
    use std::{
        fs::{File, OpenOptions},
        io,
        os::windows::io::AsRawHandle,
        time::{Duration, Instant},
    };
    use windows_sys::Win32::{
        Foundation::{HANDLE, WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT},
        System::{Console::*, Threading::WaitForSingleObject},
    };

    fn handle(file: &File) -> HANDLE {
        file.as_raw_handle() as HANDLE
    }
    fn checked(result: i32) -> io::Result<()> {
        if result == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
    fn mode(file: &File) -> io::Result<u32> {
        let mut mode = 0;
        // SAFETY: the owned handle and output pointer are valid.
        checked(unsafe { GetConsoleMode(handle(file), &mut mode) })?;
        Ok(mode)
    }
    fn set_mode(file: &File, mode: u32) -> io::Result<()> {
        // SAFETY: the file owns a live console handle.
        checked(unsafe { SetConsoleMode(handle(file), mode) })
    }

    pub(super) struct Terminal {
        input: File,
        output: File,
        input_mode: u32,
        output_mode: u32,
        cursor: CONSOLE_CURSOR_INFO,
        raw: bool,
        vt: bool,
        screen: bool,
    }
    impl Terminal {
        pub(super) fn open() -> io::Result<Self> {
            // Never use SetStdHandle: stdin is JSON and stdout is the result.
            let input = OpenOptions::new().read(true).write(true).open("CONIN$")?;
            let output = OpenOptions::new().read(true).write(true).open("CONOUT$")?;
            let input_mode = mode(&input)?;
            let output_mode = mode(&output)?;
            // SAFETY: this integer-only structure is filled through a valid pointer.
            let mut cursor = unsafe { std::mem::zeroed() };
            checked(unsafe { GetConsoleCursorInfo(handle(&output), &mut cursor) })?;
            let terminal = Self {
                input,
                output,
                input_mode,
                output_mode,
                cursor,
                raw: false,
                vt: false,
                screen: false,
            };
            terminal.size()?;
            Ok(terminal)
        }
        pub(super) fn enter(&mut self) -> io::Result<()> {
            // Crossterm 0.28.1 opens CONIN$ for raw mode and event polling,
            // but disable_raw_mode does not restore the exact original mode.
            // Keep mode ownership and input on our own handles instead.
            set_mode(
                &self.input,
                (self.input_mode
                    & !(ENABLE_ECHO_INPUT
                        | ENABLE_LINE_INPUT
                        | ENABLE_PROCESSED_INPUT
                        | ENABLE_QUICK_EDIT_MODE
                        | ENABLE_VIRTUAL_TERMINAL_INPUT
                        | ENABLE_MOUSE_INPUT))
                    | ENABLE_EXTENDED_FLAGS
                    | ENABLE_WINDOW_INPUT,
            )?;
            self.raw = true;
            set_mode(
                &self.output,
                self.output_mode | ENABLE_PROCESSED_OUTPUT | ENABLE_VIRTUAL_TERMINAL_PROCESSING,
            )?;
            self.vt = true;
            self.screen = true;
            self.write("\x1b[?1049h\x1b[?7l\x1b[?25l\x1b[0m\x1b[2J\x1b[H")
        }
        pub(super) fn check_connection(&self) -> io::Result<()> {
            mode(&self.input)?;
            mode(&self.output)?;
            Ok(())
        }
        pub(super) fn size(&self) -> io::Result<(u16, u16)> {
            // SAFETY: the API fills the structure through a valid pointer.
            let mut info: CONSOLE_SCREEN_BUFFER_INFO = unsafe { std::mem::zeroed() };
            checked(unsafe { GetConsoleScreenBufferInfo(handle(&self.output), &mut info) })?;
            let width = i32::from(info.srWindow.Right) - i32::from(info.srWindow.Left) + 1;
            let height = i32::from(info.srWindow.Bottom) - i32::from(info.srWindow.Top) + 1;
            if width <= 0 || height <= 0 {
                return Err(io::Error::other("console has zero dimensions"));
            }
            Ok((width as u16, height as u16))
        }
        fn write(&self, text: &str) -> io::Result<()> {
            // Unicode output without changing the global console code page.
            let wide: Vec<u16> = text.encode_utf16().collect();
            let mut remaining = wide.as_slice();
            while !remaining.is_empty() {
                let mut len = remaining.len().min(8192);
                if len < remaining.len() && (0xd800..=0xdbff).contains(&remaining[len - 1]) {
                    len -= 1;
                }
                let mut written = 0;
                // SAFETY: remaining contains at least len initialized UTF-16 units.
                checked(unsafe {
                    WriteConsoleW(
                        handle(&self.output),
                        remaining.as_ptr().cast(),
                        len as u32,
                        &mut written,
                        std::ptr::null(),
                    )
                })?;
                if written == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "console write made no progress",
                    ));
                }
                remaining = &remaining[written as usize..];
            }
            Ok(())
        }
        pub(super) fn present(&mut self, buffer: &Buffer) -> io::Result<()> {
            let mut text = String::new();
            let size = buffer.size();
            for y in 0..size.height {
                for x in 0..size.width {
                    if let Some(cell) = buffer.get((x, y).into()) {
                        // queue!/Display/Cell::queue can probe/change VT state
                        // or select a WinAPI fallback, even for an in-memory
                        // writer. write_ansi bypasses that global dispatch.
                        ansi(&mut text, MoveTo(x, y))?;
                        ansi(&mut text, ResetColor)?;
                        let content = cell;
                        if content.is_empty() || content.width() == 0 {
                            continue;
                        }
                        if let Some(colors) = content.style.colors {
                            ansi(&mut text, SetColors(colors))?;
                        }
                        if let Some(attr) = content.style.attr {
                            ansi(&mut text, SetAttributes(attr))?;
                        }
                        if content.as_char() == Some('\t') {
                            text.push_str(&" ".repeat(content.width()));
                        } else {
                            text.push_str(&content.symbol());
                        }
                    }
                }
            }
            ansi(&mut text, ResetColor)?;
            self.write(&text)
        }
        pub(super) fn restore(&mut self) -> io::Result<()> {
            let mut first = None;
            if self.screen {
                // A retry after partial cleanup may need VT re-enabled first.
                if let Err(e) = set_mode(
                    &self.output,
                    self.output_mode | ENABLE_PROCESSED_OUTPUT | ENABLE_VIRTUAL_TERMINAL_PROCESSING,
                ) {
                    first.get_or_insert(e);
                } else {
                    self.vt = true;
                    for text in ["\x1b[0m", "\x1b[?7h", "\x1b[?25h", "\x1b[?1049l"] {
                        if let Err(e) = self.write(text) {
                            first.get_or_insert(e);
                        }
                    }
                    // SAFETY: the saved cursor structure and owned handle are valid.
                    if let Err(e) =
                        checked(unsafe { SetConsoleCursorInfo(handle(&self.output), &self.cursor) })
                    {
                        first.get_or_insert(e);
                    }
                    if first.is_none() {
                        self.screen = false;
                    }
                }
            }
            if self.vt {
                match set_mode(&self.output, self.output_mode) {
                    Ok(()) => self.vt = false,
                    Err(e) => {
                        first.get_or_insert(e);
                    }
                }
            }
            if self.raw {
                match set_mode(&self.input, self.input_mode) {
                    Ok(()) => self.raw = false,
                    Err(e) => {
                        first.get_or_insert(e);
                    }
                }
            }
            first.map_or(Ok(()), Err)
        }
    }
    fn ansi(text: &mut String, command: impl Command) -> io::Result<()> {
        command
            .write_ansi(text)
            .map_err(|_| io::Error::other("encode terminal command"))
    }
    impl Drop for Terminal {
        fn drop(&mut self) {
            let _ = self.restore();
        }
    }

    pub(super) struct Input {
        console: File,
        decoder: KeyDecoder,
        repeated: Option<(Event, u16)>,
    }
    impl Input {
        pub(super) fn start(terminal: &Terminal) -> io::Result<Self> {
            Ok(Self {
                console: terminal.input.try_clone()?,
                decoder: KeyDecoder::default(),
                repeated: None,
            })
        }
        pub(super) fn read_event(&mut self, timeout: Duration) -> io::Result<Option<Event>> {
            if let Some((event, count)) = self.repeated.take() {
                if count > 1 {
                    self.repeated = Some((event.clone(), count - 1));
                }
                return Ok(Some(event));
            }
            let deadline = Instant::now() + timeout;
            loop {
                let remaining = deadline.saturating_duration_since(Instant::now());
                // SAFETY: the owned console handle stays live throughout the wait.
                match unsafe {
                    WaitForSingleObject(handle(&self.console), remaining.as_millis() as u32)
                } {
                    WAIT_TIMEOUT => return Ok(None),
                    WAIT_FAILED => return Err(io::Error::last_os_error()),
                    WAIT_OBJECT_0 => {}
                    _ => return Err(io::Error::other("unexpected console wait result")),
                }
                // Exclusive input ownership is required: no competing reader may
                // drain the signalled queue before this read. No detached worker.
                let mut record: INPUT_RECORD = unsafe { std::mem::zeroed() };
                let mut read = 0;
                // SAFETY: record and read point to writable initialized storage.
                checked(unsafe {
                    ReadConsoleInputW(handle(&self.console), &mut record, 1, &mut read)
                })?;
                if read != 0 && record.EventType == KEY_EVENT as u16 {
                    // SAFETY: EventType identifies the active union member.
                    let key = unsafe { record.Event.KeyEvent };
                    if let Some(event) = self.decoder.decode(key) {
                        if key.wRepeatCount > 1 {
                            self.repeated = Some((event.clone(), key.wRepeatCount - 1));
                        }
                        return Ok(Some(event));
                    }
                }
                // Resize is queried from our output handle by the shared loop.
                // Mouse reporting is disabled, matching the Unix host.
                if Instant::now() >= deadline {
                    return Ok(None);
                }
            }
        }
    }
    #[derive(Default)]
    struct KeyDecoder {
        surrogate: Option<u16>,
    }
    impl KeyDecoder {
        fn decode(&mut self, key: KEY_EVENT_RECORD) -> Option<Event> {
            // SAFETY: ReadConsoleInputW supplies the Unicode union member.
            let unit = unsafe { key.uChar.UnicodeChar };
            let alt_code = key.wVirtualKeyCode == 0x12 && key.bKeyDown == 0 && unit != 0;
            if key.bKeyDown == 0 && !alt_code {
                return None;
            }
            let state = key.dwControlKeyState;
            let mut modifiers = KeyModifiers::empty();
            if state & SHIFT_PRESSED != 0 {
                modifiers |= KeyModifiers::SHIFT;
            }
            if state & (LEFT_CTRL_PRESSED | RIGHT_CTRL_PRESSED) != 0 {
                modifiers |= KeyModifiers::CONTROL;
            }
            if state & (LEFT_ALT_PRESSED | RIGHT_ALT_PRESSED) != 0 {
                modifiers |= KeyModifiers::ALT;
            }
            // AltGr produces text, not a Ctrl+Alt shortcut.
            if unit >= 0x20 && state & RIGHT_ALT_PRESSED != 0 && state & LEFT_CTRL_PRESSED != 0 {
                modifiers.remove(KeyModifiers::CONTROL | KeyModifiers::ALT);
            }
            if alt_code {
                modifiers.remove(KeyModifiers::ALT);
            }
            if (0x60..=0x69).contains(&key.wVirtualKeyCode) && modifiers == KeyModifiers::ALT {
                return None;
            }
            let code = if alt_code {
                None
            } else {
                match key.wVirtualKeyCode {
                    0x10..=0x12 => return None,
                    0x08 => Some(KeyCode::Backspace),
                    0x09 if modifiers.contains(KeyModifiers::SHIFT) => Some(KeyCode::BackTab),
                    0x09 => Some(KeyCode::Tab),
                    0x0d => Some(KeyCode::Enter),
                    0x1b => Some(KeyCode::Esc),
                    0x21 => Some(KeyCode::PageUp),
                    0x22 => Some(KeyCode::PageDown),
                    0x23 => Some(KeyCode::End),
                    0x24 => Some(KeyCode::Home),
                    0x25 => Some(KeyCode::Left),
                    0x26 => Some(KeyCode::Up),
                    0x27 => Some(KeyCode::Right),
                    0x28 => Some(KeyCode::Down),
                    0x2d => Some(KeyCode::Insert),
                    0x2e => Some(KeyCode::Delete),
                    0x70..=0x87 => Some(KeyCode::F((key.wVirtualKeyCode - 0x6f) as u8)),
                    _ => None,
                }
            };
            let code = match code {
                Some(code) => {
                    self.surrogate = None;
                    code
                }
                None => {
                    if (0xd800..=0xdbff).contains(&unit) {
                        self.surrogate = Some(unit);
                        return None;
                    }
                    let high = self.surrogate.take();
                    let ch = if (0xdc00..=0xdfff).contains(&unit) {
                        let high = high?;
                        char::from_u32(
                            0x10000 + ((u32::from(high) - 0xd800) << 10) + u32::from(unit) - 0xdc00,
                        )?
                    } else if unit < 0x20
                        && modifiers.contains(KeyModifiers::CONTROL)
                        && (0x41..=0x5a).contains(&key.wVirtualKeyCode)
                    {
                        let ch = char::from_u32(u32::from(key.wVirtualKeyCode))?;
                        if modifiers.contains(KeyModifiers::SHIFT) {
                            ch
                        } else {
                            ch.to_ascii_lowercase()
                        }
                    } else if unit == 0
                        && modifiers.contains(KeyModifiers::CONTROL)
                        && key.wVirtualKeyCode == 0x20
                    {
                        ' '
                    } else if unit < 0x20 {
                        return None;
                    } else {
                        char::from_u32(u32::from(unit))?
                    };
                    KeyCode::Char(ch)
                }
            };
            Some(Event::Key(KeyEvent::new(code, modifiers)))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn key(vk: u16, unit: u16, state: u32) -> KEY_EVENT_RECORD {
            // SAFETY: all-zero is valid for this C integer/union structure.
            let mut key: KEY_EVENT_RECORD = unsafe { std::mem::zeroed() };
            key.bKeyDown = 1;
            key.wRepeatCount = 1;
            key.wVirtualKeyCode = vk;
            key.uChar.UnicodeChar = unit;
            key.dwControlKeyState = state;
            key
        }

        fn event(code: KeyCode, modifiers: KeyModifiers) -> Option<Event> {
            Some(Event::Key(KeyEvent::new(code, modifiers)))
        }

        #[test]
        fn shortcuts_navigation_and_release() {
            let mut decoder = KeyDecoder::default();
            assert_eq!(
                decoder.decode(key(0x43, 3, LEFT_CTRL_PRESSED)),
                event(KeyCode::Char('c'), KeyModifiers::CONTROL)
            );
            assert_eq!(
                decoder.decode(key(0x09, 9, SHIFT_PRESSED)),
                event(KeyCode::BackTab, KeyModifiers::SHIFT)
            );
            assert_eq!(
                decoder.decode(key(0x25, 0, 0)),
                event(KeyCode::Left, KeyModifiers::empty())
            );
            let mut release = key(0x0d, 13, 0);
            release.bKeyDown = 0;
            assert_eq!(decoder.decode(release), None);
        }

        #[test]
        fn unicode_altgr_and_alt_codes() {
            let mut decoder = KeyDecoder::default();
            assert_eq!(decoder.decode(key(0, 0xd83d, 0)), None);
            assert_eq!(
                decoder.decode(key(0, 0xde00, 0)),
                event(KeyCode::Char('😀'), KeyModifiers::empty())
            );
            assert_eq!(decoder.decode(key(0, 0xde00, 0)), None);
            assert_eq!(
                decoder.decode(key(0x51, '@' as u16, LEFT_CTRL_PRESSED | RIGHT_ALT_PRESSED)),
                event(KeyCode::Char('@'), KeyModifiers::empty())
            );
            let mut alt = key(0x12, 'é' as u16, 0);
            alt.bKeyDown = 0;
            assert_eq!(
                decoder.decode(alt),
                event(KeyCode::Char('é'), KeyModifiers::empty())
            );
        }

        #[test]
        fn ansi_encoding_does_not_require_standard_console_handles() {
            let mut text = String::new();
            ansi(&mut text, MoveTo(2, 3)).unwrap();
            ansi(&mut text, ResetColor).unwrap();
            assert_eq!(text, "\x1b[4;3H\x1b[0m");
        }
    }
}

#[cfg(unix)]
mod unix {
    use std::{
        fs::{File, OpenOptions},
        io::{self, Write},
        os::fd::AsRawFd,
        sync::{
            atomic::{AtomicBool, Ordering},
            mpsc::{self, Receiver, RecvTimeoutError},
            Arc,
        },
        thread::{self, JoinHandle},
        time::{Duration, Instant},
    };

    use crossterm::{
        cursor::{Hide, MoveTo},
        event::{self, EnableBracketedPaste, Event},
        execute, queue,
        style::ResetColor,
        terminal::{self, Clear, ClearType, DisableLineWrap, EnterAlternateScreen},
    };
    use rsille::render::buffer::Buffer;

    const INTERVAL: Duration = Duration::from_millis(50);

    pub(super) struct Terminal {
        tty: File,
        screen: bool,
        raw: bool,
    }

    impl Terminal {
        pub(super) fn open() -> io::Result<Self> {
            let tty = OpenOptions::new().read(true).write(true).open("/dev/tty")?;
            let terminal = Self {
                tty,
                screen: false,
                raw: false,
            };
            terminal.check_connection()?;
            terminal.size()?;
            // Background jobs must fail rather than be suspended by SIGTTIN.
            // SAFETY: tty owns a live file descriptor; these calls take no pointers.
            let foreground = unsafe { libc::tcgetpgrp(terminal.tty.as_raw_fd()) };
            if foreground < 0 {
                return Err(io::Error::last_os_error());
            }
            if foreground != unsafe { libc::getpgrp() } {
                return Err(io::Error::other(
                    "interactive CLI must run in the foreground",
                ));
            }
            Ok(terminal)
        }

        pub(super) fn enter(&mut self) -> io::Result<()> {
            terminal::enable_raw_mode()?;
            self.raw = true;
            // Mark before writing: a failed write may still have changed modes.
            self.screen = true;
            execute!(
                self.tty,
                EnterAlternateScreen,
                DisableLineWrap,
                Hide,
                EnableBracketedPaste,
                ResetColor,
                Clear(ClearType::All),
                MoveTo(0, 0)
            )
        }

        pub(super) fn check_connection(&self) -> io::Result<()> {
            let mut fd = libc::pollfd {
                fd: self.tty.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            // SAFETY: fd is initialized and valid for one pollfd.
            let result = unsafe { libc::poll(&mut fd, 1, 0) };
            if result < 0 {
                let error = io::Error::last_os_error();
                if error.kind() != io::ErrorKind::Interrupted {
                    return Err(error);
                }
            }
            if fd.revents & (libc::POLLHUP | libc::POLLERR | libc::POLLNVAL) != 0 {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "tty hung up"));
            }
            Ok(())
        }

        pub(super) fn size(&self) -> io::Result<(u16, u16)> {
            // SAFETY: winsize is a C integer-only structure; ioctl initializes it.
            let mut size: libc::winsize = unsafe { std::mem::zeroed() };
            if unsafe { libc::ioctl(self.tty.as_raw_fd(), libc::TIOCGWINSZ, &mut size) } < 0 {
                return Err(io::Error::last_os_error());
            }
            if size.ws_col == 0 || size.ws_row == 0 {
                return Err(io::Error::other("tty has zero dimensions"));
            }
            Ok((size.ws_col, size.ws_row))
        }

        pub(super) fn present(&mut self, buffer: &Buffer) -> io::Result<()> {
            let mut bytes = Vec::new();
            let size = buffer.size();
            for y in 0..size.height {
                for x in 0..size.width {
                    if let Some(cell) = buffer.get((x, y).into()) {
                        // Absolute positioning skips wide-grapheme continuation
                        // cells. SGR reset prevents style leakage between cells.
                        queue!(bytes, MoveTo(x, y), ResetColor)?;
                        cell.queue(&mut bytes)?;
                    }
                }
            }
            queue!(bytes, ResetColor)?;
            self.tty.write_all(&bytes)?;
            self.tty.flush()
        }

        pub(super) fn restore(&mut self) -> io::Result<()> {
            let mut first = None;
            if self.screen {
                // Separate writes ensure one failed operation does not prevent
                // attempts to undo the remaining modes.
                for bytes in [
                    b"\x1b[0m".as_slice(),
                    b"\x1b[?2004l",
                    b"\x1b[?7h",
                    b"\x1b[?25h",
                    b"\x1b[?1049l",
                ] {
                    if let Err(error) = self.tty.write_all(bytes) {
                        first.get_or_insert(error);
                    }
                }
                if let Err(error) = self.tty.flush() {
                    first.get_or_insert(error);
                }
                if first.is_none() {
                    self.screen = false;
                }
            }
            if self.raw {
                match terminal::disable_raw_mode() {
                    Ok(()) => self.raw = false,
                    Err(error) => {
                        first.get_or_insert(error);
                    }
                }
            }
            first.map_or(Ok(()), Err)
        }
    }

    impl Drop for Terminal {
        fn drop(&mut self) {
            let _ = self.restore();
        }
    }

    pub(super) struct Input {
        events: Receiver<io::Result<Event>>,
        stop: Arc<AtomicBool>,
        worker: Option<JoinHandle<()>>,
    }

    impl Input {
        pub(super) fn read_event(&mut self, timeout: Duration) -> io::Result<Option<Event>> {
            match self.events.recv_timeout(timeout) {
                Ok(event) => event.map(Some),
                Err(RecvTimeoutError::Timeout) => Ok(None),
                Err(RecvTimeoutError::Disconnected) => {
                    Err(io::Error::other("terminal input reader stopped"))
                }
            }
        }

        pub(super) fn start(_: &Terminal) -> io::Result<Self> {
            // Verified in crossterm 0.28.1: both Unix event backends use
            // tty_fd(), which opens /dev/tty when stdin is a pipe. No dup2 or
            // stdout redirection is needed. Do not also use EventStream.
            let (sender, events) = mpsc::sync_channel(32);
            let stop = Arc::new(AtomicBool::new(false));
            let stopping = stop.clone();
            let worker = thread::Builder::new()
                .name("cli-terminal-input".into())
                .spawn(move || {
                    while !stopping.load(Ordering::Acquire) {
                        let event = match event::poll(INTERVAL) {
                            Ok(false) => continue,
                            Ok(true) => event::read(),
                            Err(error) => Err(error),
                        };
                        let failed = event.is_err();
                        if sender.send(event).is_err() || failed {
                            break;
                        }
                    }
                })?;
            Ok(Self {
                events,
                stop,
                worker: Some(worker),
            })
        }
    }

    impl Drop for Input {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Release);
            if let Some(worker) = self.worker.take() {
                // Let ordinary timed polls finish before restoring raw mode.
                // Drain the bounded queue so a full channel cannot hold it up.
                let deadline = Instant::now() + INTERVAL * 2;
                while !worker.is_finished() && Instant::now() < deadline {
                    while self.events.try_recv().is_ok() {}
                    thread::sleep(Duration::from_millis(1));
                }
                // 0.28's mio backend can spin on EOF or block on an incomplete
                // escape sequence, even inside poll(timeout). Never join such
                // a reader during cleanup. This one-shot CLI host lets process
                // exit reclaim it; it must not be reused as a library session.
                if worker.is_finished() {
                    let _ = worker.join();
                }
            }
        }
    }
}

#[cfg(any(unix, windows))]
mod host {
    #[cfg(unix)]
    use super::unix::{Input, Terminal};
    #[cfg(windows)]
    use super::windows::{Input, Terminal};
    use crossterm::event::Event;
    use std::time::Duration;
    const INTERVAL: Duration = Duration::from_millis(50);
    pub(super) fn run(config: crate::config::Config) -> Result<Option<serde_json::Value>, String> {
        let mut terminal = Terminal::open().map_err(|e| format!("open terminal: {e}"))?;
        let result = (|| {
            terminal
                .enter()
                .map_err(|e| format!("set up terminal: {e}"))?;
            let mut ui = crate::ui::build(config);
            let mut size = terminal.size().map_err(|e| format!("terminal size: {e}"))?;
            ui.resize(size.0, size.1);
            terminal
                .present(ui.draw()?)
                .map_err(|e| format!("draw terminal: {e}"))?;
            let mut input =
                Input::start(&terminal).map_err(|e| format!("start terminal input: {e}"))?;
            loop {
                if let Some(outcome) = ui.outcome() {
                    return Ok(outcome.ok());
                }
                terminal
                    .check_connection()
                    .map_err(|e| format!("terminal disconnected: {e}"))?;
                // Query the owned terminal, never the redirected standard handles.
                let current = terminal.size().map_err(|e| format!("terminal size: {e}"))?;
                let mut dirty = current != size;
                if dirty {
                    size = current;
                    ui.resize(size.0, size.1);
                }
                match input.read_event(INTERVAL) {
                    Ok(Some(Event::Resize(_, _))) => {
                        // Re-query next iteration; queued resize events can be stale.
                    }
                    Ok(Some(event)) => {
                        ui.event(event)?;
                        dirty = true;
                    }
                    Err(error) => return Err(format!("read terminal input: {error}")),
                    Ok(None) => {}
                }
                if let Some(outcome) = ui.outcome() {
                    return Ok(outcome.ok());
                }
                if dirty {
                    terminal
                        .present(ui.draw()?)
                        .map_err(|e| format!("draw terminal: {e}"))?;
                }
            }
        })();
        // Report restoration failures on ordinary exits; Drop also retries them
        // and covers unwinding panics and partially completed setup.
        match (result, terminal.restore()) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), Ok(())) => Err(error),
            (Ok(_), Err(error)) => Err(format!("restore terminal: {error}")),
            (Err(error), Err(cleanup)) => Err(format!("{error}; restore terminal: {cleanup}")),
        }
    }
}
