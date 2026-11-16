//! Unix fullscreen host. The entry point must reject inline mode first.
//!
//! Call once, after consuming input data, with exclusive ownership of crossterm
//! input and terminal modes. No interface bytes are written to stdout.

pub fn run(config: crate::config::Config) -> Result<Option<serde_json::Value>, String> {
    #[cfg(unix)]
    {
        unix::run(config)
    }
    #[cfg(not(unix))]
    {
        let _ = config;
        Err("the interactive CLI currently supports Unix only".into())
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
            let input = Input::start().map_err(|e| format!("start terminal input: {e}"))?;
            loop {
                if let Some(outcome) = ui.outcome() {
                    return Ok(outcome.ok());
                }
                terminal
                    .check_connection()
                    .map_err(|e| format!("terminal disconnected: {e}"))?;
                // Query our own tty, not stdout or crossterm's tput fallback.
                let current = terminal.size().map_err(|e| format!("terminal size: {e}"))?;
                let mut dirty = current != size;
                if dirty {
                    size = current;
                    ui.resize(size.0, size.1);
                }
                match input.events.recv_timeout(INTERVAL) {
                    Ok(Ok(Event::Resize(_, _))) => {
                        // Re-query next iteration; queued resize events can be stale.
                    }
                    Ok(Ok(event)) => {
                        ui.event(event)?;
                        dirty = true;
                    }
                    Ok(Err(error)) => return Err(format!("read terminal input: {error}")),
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => {
                        return Err("terminal input reader stopped".into());
                    }
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

    struct Terminal {
        tty: File,
        screen: bool,
        raw: bool,
    }

    impl Terminal {
        fn open() -> io::Result<Self> {
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

        fn enter(&mut self) -> io::Result<()> {
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

        fn check_connection(&self) -> io::Result<()> {
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

        fn size(&self) -> io::Result<(u16, u16)> {
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

        fn present(&mut self, buffer: &Buffer) -> io::Result<()> {
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

        fn restore(&mut self) -> io::Result<()> {
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

    struct Input {
        events: Receiver<io::Result<Event>>,
        stop: Arc<AtomicBool>,
        worker: Option<JoinHandle<()>>,
    }

    impl Input {
        fn start() -> io::Result<Self> {
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
