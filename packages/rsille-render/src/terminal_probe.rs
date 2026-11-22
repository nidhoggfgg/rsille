//! Explicit startup-only terminal queries, before managed input owns the tty.
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
        unix::query(timeout, unix::cell_pixels)
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
pub(super) fn sixel(timeout: Duration) -> io::Result<bool> {
    before_input(|| unix::query(timeout, unix::sixel))
}

/// Caller holds the input lifecycle guard for protocol and metric discovery.
pub(super) fn protocols(kitty: bool, sixel: bool, timeout: Duration) -> io::Result<(bool, bool)> {
    #[cfg(unix)]
    {
        unix::query(timeout, |tty, deadline| {
            unix::protocols(tty, deadline, kitty, sixel)
        })
    }
    #[cfg(not(unix))]
    {
        let _ = (kitty, sixel, timeout);
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "raster protocol negotiation requires Unix or WSL; supply a verified session",
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

    pub(super) fn query<T>(
        timeout: Duration,
        exchange: impl FnOnce(&mut File, Instant) -> io::Result<T>,
    ) -> io::Result<T> {
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
        probe_tty(&mut tty, deadline, exchange)
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

    fn probe_tty<T>(
        tty: &mut File,
        deadline: Instant,
        exchange: impl FnOnce(&mut File, Instant) -> io::Result<T>,
    ) -> io::Result<T> {
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
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "terminal probe timed out"))
    }

    fn wait(tty: &File, events: libc::c_short, deadline: Instant) -> io::Result<()> {
        loop {
            let left = remaining(deadline)?;
            match wait_once(tty.as_raw_fd(), events, left) {
                Ok(true) => return remaining(deadline).map(|_| ()),
                Ok(false) => {}
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn wait_once(fd: libc::c_int, events: libc::c_short, left: Duration) -> io::Result<bool> {
        let millis = left.as_nanos().div_ceil(1_000_000).min(i32::MAX as u128);
        let mut pollfd = libc::pollfd {
            fd,
            events,
            revents: 0,
        };
        // SAFETY: poll receives one initialized pollfd and a bounded timeout.
        if unsafe { libc::poll(&mut pollfd, 1, millis as i32) } < 0 {
            return Err(io::Error::last_os_error());
        }
        if pollfd.revents & libc::POLLNVAL != 0 {
            return Err(io::Error::from_raw_os_error(libc::EBADF));
        }
        Ok(pollfd.revents & (events | libc::POLLHUP | libc::POLLERR) != 0)
    }

    // Darwin's poll returns POLLNVAL for the /dev/tty alias. select supports it.
    #[cfg(target_os = "macos")]
    fn wait_once(fd: libc::c_int, events: libc::c_short, left: Duration) -> io::Result<bool> {
        if fd as usize >= libc::FD_SETSIZE {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "terminal fd exceeds select capacity",
            ));
        }
        let mut timeout = libc::timeval {
            tv_sec: left.as_secs().min(i32::MAX as u64) as _,
            tv_usec: left.subsec_micros() as _,
        };
        // SAFETY: fd belongs to the open tty and fits in the initialized set.
        let result = unsafe {
            let mut set = std::mem::zeroed();
            libc::FD_SET(fd, &mut set);
            let null = std::ptr::null_mut();
            let (read, write) = if events == libc::POLLIN {
                (&mut set as *mut _, null)
            } else {
                (null, &mut set as *mut _)
            };
            libc::select(fd + 1, read, write, null, &mut timeout)
        };
        if result < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(result > 0)
        }
    }

    pub(super) fn cell_pixels(tty: &mut File, deadline: Instant) -> io::Result<(u16, u16)> {
        let mut scan = Scan::default();
        exchange(tty, deadline, b"\x1b[16t", |byte| scan.push(byte))
    }

    pub(super) fn sixel(tty: &mut File, deadline: Instant) -> io::Result<bool> {
        protocols(tty, deadline, false, true).map(|(_, supported)| supported)
    }

    pub(super) fn protocols(
        tty: &mut File,
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
                if e.kind() == io::ErrorKind::TimedOut
                    && (kitty_reply.is_some() || da1.is_some()) => {}
            other => other?,
        }
        Ok((
            kitty && kitty_reply == Some(true),
            sixel && da1 == Some(true),
        ))
    }

    fn da1_sixel(reply: &[u8]) -> Option<bool> {
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

    fn exchange<T>(
        tty: &mut File,
        deadline: Instant,
        mut query: &[u8],
        mut scan: impl FnMut(u8) -> Option<T>,
    ) -> io::Result<T> {
        while !query.is_empty() {
            wait(tty, libc::POLLOUT, deadline)?;
            match tty.write(query) {
                Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
                Ok(count) => query = &query[count..],
                Err(error) if retryable(&error) => {}
                Err(error) => return Err(error),
            }
        }
        // Leave bytes following the last expected reply for the input owner.
        let mut bytes = [0; 1];
        loop {
            wait(tty, libc::POLLIN, deadline)?;
            match tty.read(&mut bytes) {
                Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
                Ok(_) => {
                    remaining(deadline)?;
                    if let Some(reply) = scan(bytes[0]) {
                        return Ok(reply);
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
            // Darwin sets PENDIN when canonical input is re-enabled.
            #[cfg(target_os = "macos")]
            let mask = !libc::PENDIN;
            #[cfg(not(target_os = "macos"))]
            let mask = !0;
            assert_eq!(before.c_lflag & mask, after.c_lflag & mask);
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
        fn da1_requires_unsigned_decimal_fields_even_after_a_matching_feature() {
            for params in ["+65;4", "65;+4", "65;4;", "65;;4", "65;4; 2", "65;４"] {
                assert_eq!(da1_sixel(format!("\x1b[?{params}").as_bytes()), None);
            }
            assert_eq!(da1_sixel(b"\x1b[?65;14;24"), Some(false));
            assert_eq!(da1_sixel(b"\x1b[?65;4"), Some(true));
        }

        #[test]
        fn protocol_queries_share_a_deadline_consume_replies_and_leave_the_next_key() {
            for (kitty_status, da, expected) in [
                (Some("OK"), Some("65;4;22"), Some((true, true))),
                (Some("ENOENT"), Some("65;4"), Some((false, true))),
                (Some("OK"), None, Some((true, false))),
                (None, Some("65;4"), Some((false, true))),
                (Some("ERROR"), Some("65;14;24"), Some((false, false))),
                (None, None, None),
            ] {
                let (mut master, mut slave) = pty();
                let saved = attributes(&slave);
                thread::scope(|scope| {
                    scope.spawn(|| {
                        let deadline = Instant::now() + Duration::from_secs(2);
                        let mut query = Vec::new();
                        while !query.ends_with(b"\x1b[c") {
                            wait(&master, libc::POLLIN, deadline).unwrap();
                            let mut bytes = [0; 128];
                            let n = master.read(&mut bytes).unwrap();
                            assert!(n > 0);
                            query.extend_from_slice(&bytes[..n]);
                        }
                        let query = std::str::from_utf8(&query).unwrap();
                        let id = query.split("i=").nth(1).unwrap().split(',').next().unwrap();
                        let mut replies = String::from("noise\x1b_Gi=0;OK\x1b\\");
                        if let Some(da) = da {
                            replies.push_str(&format!("\x1b[?{da}c"));
                        }
                        if let Some(status) = kitty_status {
                            replies.push_str(&format!("\x1b_Gi={id};{status}\x1b\\"));
                        }
                        if kitty_status.is_some() && da.is_some() {
                            replies.push_str("k\n");
                        }
                        let (first, rest) = replies.split_at(8);
                        master.write_all(first.as_bytes()).unwrap();
                        thread::sleep(Duration::from_millis(3));
                        master.write_all(rest.as_bytes()).unwrap();
                    });
                    let received = probe_tty(
                        &mut slave,
                        Instant::now() + Duration::from_millis(200),
                        |tty, deadline| protocols(tty, deadline, true, true),
                    );
                    match expected {
                        Some(pair) => assert_eq!(received.unwrap(), pair),
                        None => assert_eq!(received.unwrap_err().kind(), io::ErrorKind::TimedOut),
                    }
                });
                assert_restored(&slave, saved);
                if kitty_status.is_some() && da.is_some() {
                    let mut key = [0; 2];
                    slave.read_exact(&mut key).unwrap();
                    assert_eq!(&key, b"k\n");
                } else if expected.is_none() {
                    // A late response belongs to the subsequent reader. The
                    // startup transport does not flush or consume it later.
                    master.write_all(b"\x1b[?65;4c\n").unwrap();
                    let mut late = [0; 9];
                    slave.read_exact(&mut late).unwrap();
                    assert_eq!(&late, b"\x1b[?65;4c\n");
                }
            }
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
                    probe_tty(
                        &mut slave,
                        Instant::now() + Duration::from_secs(2),
                        cell_pixels
                    )
                    .unwrap(),
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
                    probe_tty(
                        &mut slave,
                        Instant::now() + Duration::from_secs(2),
                        cell_pixels
                    )
                    .unwrap(),
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
                cell_pixels(&mut input, Instant::now() + Duration::from_millis(30))
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
                probe_tty(
                    &mut slave,
                    Instant::now() + Duration::from_millis(30),
                    cell_pixels
                )
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
                let result = probe_tty(&mut slave, start + Duration::from_millis(100), cell_pixels);
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
                    cell_pixels(&mut input, Instant::now() + Duration::from_secs(2))
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
