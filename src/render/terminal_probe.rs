//! Explicit startup-only cell-size discovery, before managed input owns the tty.
//!
//! This scan may consume keys, paste, or other unrelated input. It does not flush
//! input, preserve events, or retain a reader after returning. External input
//! readers are not tracked: callers must arrange exclusive access themselves.

use std::{io, sync::Mutex, time::Duration};

static INPUT: InputLifecycle = InputLifecycle(Mutex::new(false));

struct InputLifecycle(Mutex<bool>);

impl InputLifecycle {
    fn before_input<T>(&self, operation: impl FnOnce() -> io::Result<T>) -> io::Result<T> {
        let initialized = self
            .0
            .lock()
            .map_err(|_| io::Error::other("terminal input lifecycle lock poisoned"))?;
        if *initialized {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "terminal probing must precede managed input initialization",
            ));
        }
        let result = operation();
        // Keep OS discovery and the entire fallback inside the lifecycle guard.
        drop(initialized);
        result
    }

    fn reserve_input(&self) -> io::Result<()> {
        *self
            .0
            .lock()
            .map_err(|_| io::Error::other("terminal input lifecycle lock poisoned"))? = true;
        Ok(())
    }
}

/// Serializes the whole public probe (including OS discovery) against setup.
pub(super) fn before_input<T>(operation: impl FnOnce() -> io::Result<T>) -> io::Result<T> {
    INPUT.before_input(operation)
}

/// Permanently disallows probing, even if managed setup subsequently fails.
pub(super) fn reserve_input() -> io::Result<()> {
    INPUT.reserve_input()
}

/// CSI 16 t fallback. The caller must hold `before_input` around this operation.
pub(super) fn cell_pixels(timeout: Duration) -> io::Result<(u16, u16)> {
    #[cfg(unix)]
    {
        unix::cell_pixels(timeout)
    }
    #[cfg(not(unix))]
    {
        let _ = timeout;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "terminal cell-size query is only supported on Unix",
        ))
    }
}

#[cfg(unix)]
mod unix {
    use std::{
        fs::{File, OpenOptions},
        io::{self, IsTerminal, Read, Write},
        os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
        time::{Duration, Instant},
    };

    pub(super) fn cell_pixels(timeout: Duration) -> io::Result<(u16, u16)> {
        let deadline = Instant::now().checked_add(timeout).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "terminal probe timeout is too large",
            )
        })?;
        if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "terminal probing requires terminal stdin and stdout",
            ));
        }
        let mut tty = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_NOCTTY | libc::O_CLOEXEC)
            .open("/dev/tty")?;
        // Session identity handles /dev/tty aliases, unlike device-node identity.
        // A session has at most one controlling terminal.
        // SAFETY: all descriptors below are borrowed from live handles.
        let session = unsafe { libc::tcgetsid(tty.as_raw_fd()) };
        if session < 0
            || [io::stdin().as_raw_fd(), io::stdout().as_raw_fd()]
                .iter()
                .any(|fd| unsafe { libc::tcgetsid(*fd) } != session)
        {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "terminal probing requires stdin and stdout on the same controlling terminal",
            ));
        }
        probe_tty(&mut tty, deadline)
    }

    struct RawMode {
        fd: libc::c_int,
        saved: libc::termios,
        restored: bool,
    }

    impl RawMode {
        fn acquire(tty: &File) -> io::Result<Self> {
            let mut saved = std::mem::MaybeUninit::uninit();
            // SAFETY: tcgetattr initializes the termios value on success.
            if unsafe { libc::tcgetattr(tty.as_raw_fd(), saved.as_mut_ptr()) } != 0 {
                return Err(io::Error::last_os_error());
            }
            let guard = Self {
                fd: tty.as_raw_fd(),
                saved: unsafe { saved.assume_init() },
                restored: false,
            };
            let mut raw = guard.saved;
            make_raw(&mut raw);
            set_attributes(guard.fd, &raw)?;
            Ok(guard)
        }

        fn restore(&mut self) -> io::Result<()> {
            set_attributes(self.fd, &self.saved)?;
            self.restored = true;
            Ok(())
        }
    }

    impl Drop for RawMode {
        fn drop(&mut self) {
            if !self.restored {
                let _ = self.restore();
            }
        }
    }

    fn make_raw(raw: &mut libc::termios) {
        // cfmakeraw is not available on every Unix supported by crossterm.
        raw.c_iflag &= !(libc::IGNBRK
            | libc::BRKINT
            | libc::PARMRK
            | libc::ISTRIP
            | libc::INLCR
            | libc::IGNCR
            | libc::ICRNL
            | libc::IXON);
        raw.c_oflag &= !libc::OPOST;
        raw.c_lflag &= !(libc::ECHO | libc::ECHONL | libc::ICANON | libc::ISIG | libc::IEXTEN);
        raw.c_cflag = (raw.c_cflag & !(libc::CSIZE | libc::PARENB)) | libc::CS8;
        raw.c_cc[libc::VMIN] = 1;
        raw.c_cc[libc::VTIME] = 0;
    }

    fn set_attributes(fd: libc::c_int, attributes: &libc::termios) -> io::Result<()> {
        loop {
            // TCSANOW neither flushes input nor waits for queued output to drain.
            // SAFETY: the descriptor stays live until after the guard is dropped.
            if unsafe { libc::tcsetattr(fd, libc::TCSANOW, attributes) } == 0 {
                return Ok(());
            }
            let error = io::Error::last_os_error();
            if error.kind() != io::ErrorKind::Interrupted {
                return Err(error);
            }
        }
    }

    fn probe_tty(tty: &mut File, deadline: Instant) -> io::Result<(u16, u16)> {
        remaining(deadline)?;
        let mut raw = RawMode::acquire(tty)?;
        let result = exchange(tty, deadline);
        // Report restoration failures rather than silently claiming a clean exit.
        raw.restore()?;
        result
    }

    fn remaining(deadline: Instant) -> io::Result<Duration> {
        deadline
            .checked_duration_since(Instant::now())
            .filter(|remaining| !remaining.is_zero())
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::TimedOut,
                    "terminal cell-size probe timed out",
                )
            })
    }

    fn wait(tty: &File, events: libc::c_short, deadline: Instant) -> io::Result<()> {
        loop {
            let left = remaining(deadline)?;
            let millis = left
                .as_millis()
                .saturating_add(u128::from(left.subsec_nanos() % 1_000_000 != 0));
            let mut fd = libc::pollfd {
                fd: tty.as_raw_fd(),
                events,
                revents: 0,
            };
            // SAFETY: poll receives one initialized pollfd and a bounded timeout.
            let result = unsafe { libc::poll(&mut fd, 1, millis.min(i32::MAX as u128) as i32) };
            if result < 0 {
                let error = io::Error::last_os_error();
                if error.kind() != io::ErrorKind::Interrupted {
                    return Err(error);
                }
            } else if result > 0 {
                remaining(deadline)?;
                if fd.revents & libc::POLLNVAL != 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "invalid terminal descriptor",
                    ));
                }
                if fd.revents & (events | libc::POLLHUP | libc::POLLERR) != 0 {
                    return Ok(());
                }
            }
        }
    }

    fn exchange(tty: &mut File, deadline: Instant) -> io::Result<(u16, u16)> {
        let mut query = &b"\x1b[16t"[..];
        while !query.is_empty() {
            wait(tty, libc::POLLOUT, deadline)?;
            match tty.write(query) {
                Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
                Ok(count) => query = &query[count..],
                Err(error) if retryable(&error) => {}
                Err(error) => return Err(error),
            }
        }
        let mut scan = Scan::default();
        let mut bytes = [0; 256];
        loop {
            wait(tty, libc::POLLIN, deadline)?;
            match tty.read(&mut bytes) {
                Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
                Ok(count) => {
                    remaining(deadline)?;
                    for byte in &bytes[..count] {
                        if let Some(pixels) = scan.push(*byte) {
                            return Ok(pixels);
                        }
                    }
                }
                Err(error) if retryable(&error) => {}
                Err(error) => return Err(error),
            }
        }
    }

    fn retryable(error: &io::Error) -> bool {
        matches!(
            error.kind(),
            io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock
        )
    }

    // Constant-memory matching, including arbitrarily long numeric noise.
    #[derive(Default)]
    enum Scan {
        #[default]
        Seeking,
        Prefix(usize),
        Height(Option<u16>),
        Width(u16, Option<u16>),
    }

    impl Scan {
        fn push(&mut self, byte: u8) -> Option<(u16, u16)> {
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
                Self::Height(Some(height)) if height > 0 && byte == b';' => {
                    Self::Width(height, None)
                }
                Self::Width(height, value) if byte.is_ascii_digit() => Self::number(value, byte)
                    .map_or(Self::Seeking, |n| Self::Width(height, Some(n))),
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

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::{
            os::fd::{FromRawFd, OwnedFd},
            thread,
        };

        fn pty() -> (File, File) {
            let (mut master, mut slave) = (-1, -1);
            // SAFETY: openpty initializes both descriptors; optional arguments are null.
            assert_eq!(
                unsafe {
                    libc::openpty(
                        &mut master,
                        &mut slave,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                    )
                },
                0
            );
            // SAFETY: successful openpty returned two newly owned descriptors.
            let pair = unsafe { (File::from_raw_fd(master), File::from_raw_fd(slave)) };
            for file in [&pair.0, &pair.1] {
                assert_eq!(
                    unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK) },
                    0
                );
            }
            pair
        }

        fn attributes(tty: &File) -> libc::termios {
            let mut value = std::mem::MaybeUninit::uninit();
            assert_eq!(
                unsafe { libc::tcgetattr(tty.as_raw_fd(), value.as_mut_ptr()) },
                0
            );
            unsafe { value.assume_init() }
        }

        fn assert_restored(tty: &File, before: libc::termios) {
            let after = attributes(tty);
            assert_eq!(before.c_iflag, after.c_iflag);
            assert_eq!(before.c_oflag, after.c_oflag);
            assert_eq!(before.c_cflag, after.c_cflag);
            assert_eq!(before.c_lflag, after.c_lflag);
            assert_eq!(before.c_cc, after.c_cc);
            assert_eq!(unsafe { libc::cfgetispeed(&before) }, unsafe {
                libc::cfgetispeed(&after)
            });
            assert_eq!(unsafe { libc::cfgetospeed(&before) }, unsafe {
                libc::cfgetospeed(&after)
            });
        }

        fn receive_query(master: &mut File) {
            let deadline = Instant::now() + Duration::from_secs(2);
            let mut bytes = [0; 5];
            let mut read = 0;
            while read < bytes.len() {
                wait(master, libc::POLLIN, deadline).unwrap();
                match master.read(&mut bytes[read..]) {
                    Ok(0) => panic!("probe disconnected"),
                    Ok(count) => read += count,
                    Err(error) if retryable(&error) => {}
                    Err(error) => panic!("{error}"),
                }
            }
            assert_eq!(&bytes, b"\x1b[16t");
        }

        #[test]
        fn scan_skips_noise_invalid_and_overflow_reports() {
            let input = b"keys\x1b[8;20;10t\x1b[6;0;10t\x1b[6;20;0t\x1b[6;;10t\x1b[6;65536;1t\x1b[6;1;65536t\x1b[6;2;3;4t\x1b[6;20;\x1b[6;00020;00010t\x1b[6;1;2t";
            let mut scan = Scan::default();
            assert_eq!(
                input.iter().find_map(|byte| scan.push(*byte)),
                Some((10, 20))
            );
            let mut scan = Scan::default();
            assert_eq!(
                b"\x1b[6;65535;65535t".iter().find_map(|b| scan.push(*b)),
                Some((65535, 65535))
            );
        }

        #[test]
        fn fragmented_noisy_reply_returns_width_height_and_restores_modes() {
            let (mut master, mut slave) = pty();
            let saved = attributes(&slave);
            thread::scope(|scope| {
                scope.spawn(|| {
                    receive_query(&mut master);
                    for part in [b"keys\x1b[".as_slice(), b"6;2", b"0;", b"10t"] {
                        master.write_all(part).unwrap();
                        thread::sleep(Duration::from_millis(5));
                    }
                });
                assert_eq!(
                    probe_tty(&mut slave, Instant::now() + Duration::from_secs(2)).unwrap(),
                    (10, 20)
                );
            });
            assert_restored(&slave, saved);
        }

        #[test]
        fn queued_input_is_not_flushed_and_existing_raw_modes_are_restored() {
            for already_raw in [false, true] {
                let (mut master, mut slave) = pty();
                if already_raw {
                    let mut raw = attributes(&slave);
                    make_raw(&mut raw);
                    set_attributes(slave.as_raw_fd(), &raw).unwrap();
                }
                let saved = attributes(&slave);
                master.write_all(b"keys\x1b[6;20;10t\n").unwrap();
                assert_eq!(
                    probe_tty(&mut slave, Instant::now() + Duration::from_secs(2)).unwrap(),
                    (10, 20)
                );
                assert_restored(&slave, saved);
            }
        }

        #[test]
        fn blocked_query_write_obeys_deadline() {
            use std::os::unix::net::UnixStream;
            let (input, _peer) = UnixStream::pair().unwrap();
            input.set_nonblocking(true).unwrap();
            let mut input = File::from(OwnedFd::from(input));
            let limit = Instant::now() + Duration::from_secs(2);
            loop {
                assert!(Instant::now() < limit);
                match input.write(&[0; 4096]) {
                    Ok(_) => {}
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                    Err(error) => panic!("{error}"),
                }
            }
            assert_eq!(
                exchange(&mut input, Instant::now() + Duration::from_millis(30))
                    .unwrap_err()
                    .kind(),
                io::ErrorKind::TimedOut
            );
        }

        #[test]
        fn silent_timeout_restores_modes() {
            let (_master, mut slave) = pty();
            let saved = attributes(&slave);
            assert_eq!(
                probe_tty(&mut slave, Instant::now() + Duration::from_millis(30))
                    .unwrap_err()
                    .kind(),
                io::ErrorKind::TimedOut
            );
            assert_restored(&slave, saved);
        }

        #[test]
        fn continuous_noise_does_not_extend_deadline_and_restores_modes() {
            use std::sync::atomic::{AtomicBool, Ordering};
            let (mut master, mut slave) = pty();
            let saved = attributes(&slave);
            let done = AtomicBool::new(false);
            let start = Instant::now();
            thread::scope(|scope| {
                scope.spawn(|| {
                    receive_query(&mut master);
                    let limit = Instant::now() + Duration::from_secs(2);
                    while !done.load(Ordering::Relaxed) && Instant::now() < limit {
                        match master.write(&[b'x'; 256]) {
                            Ok(_) => {}
                            Err(error) if retryable(&error) => thread::yield_now(),
                            Err(error) => panic!("{error}"),
                        }
                    }
                });
                let result = probe_tty(&mut slave, start + Duration::from_millis(100));
                done.store(true, Ordering::Relaxed);
                assert_eq!(result.unwrap_err().kind(), io::ErrorKind::TimedOut);
            });
            assert!(start.elapsed() < Duration::from_secs(1));
            assert_restored(&slave, saved);
        }

        #[test]
        fn eof_ends_scan_without_waiting_for_deadline() {
            use std::os::unix::net::UnixStream;
            let (input, mut peer) = UnixStream::pair().unwrap();
            input.set_nonblocking(true).unwrap();
            peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
            let mut input = File::from(OwnedFd::from(input));
            thread::scope(|scope| {
                scope.spawn(move || {
                    let mut query = [0; 5];
                    peer.read_exact(&mut query).unwrap();
                    assert_eq!(&query, b"\x1b[16t");
                });
                assert_eq!(
                    exchange(&mut input, Instant::now() + Duration::from_secs(2))
                        .unwrap_err()
                        .kind(),
                    io::ErrorKind::UnexpectedEof
                );
            });
        }
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    use std::{sync::mpsc, thread};

    #[test]
    fn reservation_is_permanent_idempotent_and_rejects_the_whole_operation() {
        let state = InputLifecycle(Mutex::new(false));
        assert!(state
            .before_input(|| Err::<(), _>(io::ErrorKind::TimedOut.into()))
            .is_err());
        assert_eq!(state.before_input(|| Ok(42)).unwrap(), 42);
        for _ in 0..2 {
            state.reserve_input().unwrap();
            assert_eq!(
                state
                    .before_input(|| -> io::Result<()> {
                        panic!("OS lookup must also be excluded")
                    })
                    .unwrap_err()
                    .kind(),
                io::ErrorKind::AlreadyExists
            );
        }
    }

    #[test]
    fn reservation_waits_for_the_entire_probe() {
        let state = InputLifecycle(Mutex::new(false));
        let (entered, enter) = mpsc::channel();
        let (release, released) = mpsc::channel();
        let (reserved, reserve) = mpsc::channel();
        thread::scope(|scope| {
            let state = &state;
            scope.spawn(move || {
                state
                    .before_input(|| {
                        entered.send(()).unwrap();
                        released.recv_timeout(Duration::from_secs(2)).unwrap();
                        Ok(())
                    })
                    .unwrap()
            });
            enter.recv_timeout(Duration::from_secs(2)).unwrap();
            // Directly verify ownership, rather than relying only on scheduling.
            assert!(matches!(
                state.0.try_lock(),
                Err(std::sync::TryLockError::WouldBlock)
            ));
            scope.spawn(move || {
                state.reserve_input().unwrap();
                reserved.send(()).unwrap();
            });
            assert!(reserve.recv_timeout(Duration::from_millis(20)).is_err());
            release.send(()).unwrap();
            reserve.recv_timeout(Duration::from_secs(2)).unwrap();
        });
        assert!(state.before_input(|| Ok(())).is_err());
    }
}
