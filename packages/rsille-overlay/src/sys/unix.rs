//! Unix PTYs: `forkpty`, termios raw mode and `poll`.
use super::{Ready, Size, Spawn};
use crate::{shell, Shell};
use std::{
    ffi::{CString, OsString},
    fs::File,
    io::{self, Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{ffi::OsStrExt, net::UnixDatagram},
    },
    path::Path,
    time::Duration,
};

fn check(n: libc::c_int) -> io::Result<libc::c_int> {
    if n < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(n)
    }
}

fn winsize(fd: i32) -> io::Result<libc::winsize> {
    unsafe {
        let mut s = std::mem::zeroed::<libc::winsize>();
        check(libc::ioctl(fd, libc::TIOCGWINSZ, &mut s))?;
        Ok(s)
    }
}

/// The size of the terminal on stdout.
pub fn size() -> io::Result<Size> {
    let s = winsize(1)?;
    Ok(Size {
        cols: s.ws_col,
        rows: s.ws_row,
        xpixel: s.ws_xpixel,
        ypixel: s.ws_ypixel,
    })
}

fn to_winsize(size: Size) -> libc::winsize {
    libc::winsize {
        ws_col: size.cols,
        ws_row: size.rows,
        ws_xpixel: size.xpixel,
        ws_ypixel: size.ypixel,
    }
}

/// `$SHELL`, or `/bin/sh`.
pub fn default_shell() -> OsString {
    std::env::var_os("SHELL").unwrap_or_else(|| "/bin/sh".into())
}

/// The terminal's modes, saved before the session changes them.
pub struct TerminalMode {
    original: libc::termios,
}

impl TerminalMode {
    pub fn save() -> io::Result<Self> {
        let mut original = unsafe { std::mem::zeroed() };
        check(unsafe { libc::tcgetattr(0, &mut original) })?;
        Ok(Self { original })
    }

    pub fn enter_raw(&self) -> io::Result<()> {
        let mut raw = self.original;
        unsafe {
            libc::cfmakeraw(&mut raw);
        }
        check(unsafe { libc::tcsetattr(0, libc::TCSANOW, &raw) }).map(drop)
    }
}

impl Drop for TerminalMode {
    fn drop(&mut self) {
        unsafe {
            libc::tcsetattr(0, libc::TCSANOW, &self.original);
        }
    }
}

/// bash, fish and zsh hooks reporting over a datagram socket.
pub struct Hooks {
    inner: shell::Hooks,
    /// The end the child inherits. Dropped once the child has it.
    sender: Option<UnixDatagram>,
}

impl Hooks {
    pub fn prepare(shell: &Path) -> io::Result<Option<Self>> {
        Ok(shell::Hooks::prepare(shell)?.map(|(inner, sender)| Self {
            inner,
            sender: Some(sender),
        }))
    }

    pub fn shell(&self) -> Shell {
        self.inner.shell
    }

    /// One event message, or `WouldBlock`.
    pub fn recv(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.events.recv(buf)
    }
}

struct Child(i32);

impl Child {
    fn wait(&mut self) -> io::Result<i32> {
        let mut status = 0;
        loop {
            if unsafe { libc::waitpid(self.0, &mut status, 0) } >= 0 {
                self.0 = -1;
                return Ok(if libc::WIFEXITED(status) {
                    libc::WEXITSTATUS(status)
                } else {
                    128 + libc::WTERMSIG(status)
                });
            }
            let error = io::Error::last_os_error();
            if error.kind() != io::ErrorKind::Interrupted {
                return Err(error);
            }
        }
    }
}

impl Drop for Child {
    fn drop(&mut self) {
        if self.0 < 0 {
            return;
        }
        // The master is dropped first (hangup). Don't block on a shell that ignores it.
        unsafe {
            libc::kill(self.0, libc::SIGKILL);
            while libc::waitpid(self.0, std::ptr::null_mut(), 0) < 0
                && io::Error::last_os_error().kind() == io::ErrorKind::Interrupted
            {}
        }
    }
}

/// A child running on a pseudo terminal.
pub struct Pty {
    // Dropped before the child: the hangup comes first.
    master: File,
    child: Child,
}

impl Pty {
    pub fn spawn(
        spawn: &Spawn<'_>,
        mut hooks: Option<&mut Hooks>,
        mode: &TerminalMode,
    ) -> io::Result<Self> {
        let environment = hooks
            .as_ref()
            .and_then(|hooks| hooks.inner.environment.as_ref())
            .map(|environment| {
                environment
                    .iter()
                    .map(|v| v.as_ptr())
                    .chain(std::iter::once(std::ptr::null()))
                    .collect::<Vec<_>>()
            });
        let program = CString::new(spawn.program.as_bytes())?;
        let arguments = spawn
            .args
            .iter()
            .map(|arg| CString::new(arg.as_bytes()))
            .collect::<Result<Vec<_>, _>>()?;
        let mut args = vec![program.as_ptr()];
        if spawn.direct {
            args.extend(arguments.iter().map(|arg| arg.as_ptr()));
        } else {
            if let Some(hooks) = &hooks {
                args.extend(hooks.inner.args.iter().map(|arg| arg.as_ptr()));
            }
            args.push(c"-i".as_ptr());
        }
        args.push(std::ptr::null());
        // Prepared before forking: the child only calls async-signal-safe functions.
        let mut exec_error = format!("{}: cannot execute ", spawn.name).into_bytes();
        exec_error.extend_from_slice(program.as_bytes());
        exec_error.extend_from_slice(b"\r\n");
        let sender = hooks
            .as_ref()
            .and_then(|hooks| hooks.sender.as_ref())
            .map(|sender| sender.as_raw_fd());
        // Darwin declares both forkpty inputs as mutable. Keep the saved mode intact.
        let mut s = to_winsize(spawn.size);
        let mut original = mode.original;
        let mut fd = -1;
        let pid = check(unsafe {
            libc::forkpty(&mut fd, std::ptr::null_mut(), &raw mut original, &raw mut s)
        })?;
        if pid == 0 {
            unsafe {
                if let Some(sender) = sender {
                    if libc::fcntl(sender, libc::F_SETFD, 0) < 0 {
                        libc::_exit(127);
                    }
                }
                if let Some(env) = &environment {
                    libc::execve(program.as_ptr(), args.as_ptr(), env.as_ptr());
                } else if spawn.direct {
                    libc::execvp(program.as_ptr(), args.as_ptr());
                } else {
                    libc::execv(program.as_ptr(), args.as_ptr());
                }
                libc::write(2, exec_error.as_ptr().cast(), exec_error.len());
                libc::_exit(127);
            }
        }
        let child = Child(pid);
        // Drop the parent's copy of the sending socket.
        if let Some(hooks) = &mut hooks {
            hooks.sender = None;
        }
        let master = unsafe { File::from_raw_fd(fd) };
        let flags = check(unsafe { libc::fcntl(fd, libc::F_GETFL) })?;
        check(unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) })?;
        Ok(Self { master, child })
    }

    pub fn resize(&mut self, size: Size) -> io::Result<()> {
        let s = to_winsize(size);
        check(unsafe { libc::ioctl(self.master.as_raw_fd(), libc::TIOCSWINSZ, &s) }).map(drop)
    }

    /// Wait until child output, room for input to the child (when `writing`)
    /// or terminal input (when `reading`) is available, or the timeout passes.
    pub fn poll(&mut self, reading: bool, writing: bool, timeout: Duration) -> io::Result<Ready> {
        let mut fds = [
            libc::pollfd {
                fd: if reading { 0 } else { -1 },
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: self.master.as_raw_fd(),
                events: libc::POLLIN | if writing { libc::POLLOUT } else { 0 },
                revents: 0,
            },
        ];
        let ms = timeout.as_nanos().div_ceil(1_000_000).min(i32::MAX as u128) as i32;
        if let Err(e) = check(unsafe { libc::poll(fds.as_mut_ptr(), 2, ms) }) {
            if e.kind() == io::ErrorKind::Interrupted {
                return Ok(Ready::default());
            }
            return Err(e);
        }
        Ok(Ready {
            output: fds[1].revents & (libc::POLLIN | libc::POLLHUP | libc::POLLERR) != 0,
            writable: fds[1].revents & libc::POLLOUT != 0,
            input: fds[0].revents != 0,
        })
    }

    /// Child output. 0 once the child side is closed.
    pub fn read_output(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.master.read(buf) {
            Err(e) if e.raw_os_error() == Some(libc::EIO) => Ok(0),
            other => other,
        }
    }

    pub fn write_input(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.master.write(bytes)
    }

    /// Terminal input. 0 at end of input.
    pub fn read_input(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = unsafe { libc::read(0, buf.as_mut_ptr().cast(), buf.len()) };
        if n < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(n as usize)
        }
    }

    pub fn wait(&mut self) -> io::Result<i32> {
        self.child.wait()
    }
}
