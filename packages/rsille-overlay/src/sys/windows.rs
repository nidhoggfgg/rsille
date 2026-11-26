//! Windows console input and a child hosted in ConPTY.
mod conpty;
mod input;
use super::{Ready, Size, Spawn};
use conpty::Console;
use std::{
    ffi::OsString,
    io::{self, Read, Write},
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    sync::{
        mpsc::{self, Receiver, Sender, TryRecvError},
        Arc,
    },
    thread,
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::{HANDLE, WAIT_FAILED, WAIT_OBJECT_0},
    System::{Console::*, Threading::*},
};

fn check(ok: i32) -> io::Result<()> {
    if ok == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub fn default_shell() -> OsString {
    std::env::var_os("COMSPEC").unwrap_or_else(|| "cmd.exe".into())
}

pub fn size() -> io::Result<Size> {
    let (cols, rows) = crossterm::terminal::size()?;
    Ok(Size { cols, rows })
}

fn mode(handle: HANDLE) -> io::Result<u32> {
    let mut mode = 0;
    check(unsafe { GetConsoleMode(handle, &mut mode) })?;
    Ok(mode)
}

pub struct TerminalMode {
    input: u32,
    output: u32,
    cursor: CONSOLE_CURSOR_INFO,
}
impl TerminalMode {
    pub fn save() -> io::Result<Self> {
        let mut cursor = CONSOLE_CURSOR_INFO::default();
        check(unsafe { GetConsoleCursorInfo(io::stdout().as_raw_handle(), &mut cursor) })?;
        Ok(Self {
            cursor,
            input: mode(io::stdin().as_raw_handle())?,
            output: mode(io::stdout().as_raw_handle())?,
        })
    }
    pub fn enter_raw(&self) -> io::Result<()> {
        // Keep native input records so function keys, key-up, IME and Ctrl+C
        // can be sent through ConPTY's Win32 input protocol without guessing.
        let flags = (self.input | ENABLE_MOUSE_INPUT | ENABLE_EXTENDED_FLAGS)
            & !(ENABLE_LINE_INPUT
                | ENABLE_ECHO_INPUT
                | ENABLE_PROCESSED_INPUT
                | ENABLE_QUICK_EDIT_MODE
                | ENABLE_VIRTUAL_TERMINAL_INPUT);
        unsafe {
            check(SetConsoleMode(io::stdin().as_raw_handle(), flags))?;
            check(SetConsoleMode(
                io::stdout().as_raw_handle(),
                self.output
                    | ENABLE_PROCESSED_OUTPUT
                    | ENABLE_VIRTUAL_TERMINAL_PROCESSING
                    | DISABLE_NEWLINE_AUTO_RETURN,
            ))
        }
    }
}
impl Drop for TerminalMode {
    fn drop(&mut self) {
        unsafe {
            SetConsoleMode(io::stdin().as_raw_handle(), self.input);
            SetConsoleMode(io::stdout().as_raw_handle(), self.output);
            SetConsoleCursorInfo(io::stdout().as_raw_handle(), &self.cursor);
        }
    }
}

struct Wake(OwnedHandle);
impl Wake {
    fn new() -> io::Result<Self> {
        let handle = unsafe { CreateEventW(std::ptr::null(), 0, 0, std::ptr::null()) };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(unsafe { OwnedHandle::from_raw_handle(handle) }))
    }
    fn signal(&self) {
        unsafe {
            SetEvent(self.0.as_raw_handle());
        }
    }
}

pub struct Pty {
    // Drop the channels before closing the pseudoconsole. The reader switches
    // to draining the pipe so even a synchronous ClosePseudoConsole can finish.
    output: Receiver<io::Result<Vec<u8>>>,
    writes: Sender<Vec<u8>>,
    written: Receiver<io::Result<usize>>,
    in_flight: bool,
    ack: Option<io::Result<usize>>,
    chunk: io::Cursor<Vec<u8>>,
    wake: Arc<Wake>,
    process: OwnedHandle,
    console: Option<Console>,
    input: input::Input,
}

impl Pty {
    pub fn spawn(spawn: &Spawn<'_>) -> io::Result<Self> {
        let wake = Arc::new(Wake::new()?);
        let (console, mut reader, mut writer) = Console::new(spawn.size)?;
        let (output_tx, output) = mpsc::sync_channel(2);
        let notify = wake.clone();
        thread::Builder::new()
            .name("ConPTY output".into())
            .spawn(move || {
                let mut buf = [0; 8192];
                loop {
                    let chunk = match reader.read(&mut buf) {
                        Ok(n) => Ok(buf[..n].to_vec()),
                        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => Ok(Vec::new()),
                        Err(e) => Err(e),
                    };
                    let done = !matches!(&chunk, Ok(bytes) if !bytes.is_empty());
                    if output_tx.send(chunk).is_err() {
                        let _ = io::copy(&mut reader, &mut io::sink());
                        break;
                    }
                    notify.signal();
                    if done {
                        break;
                    }
                }
            })?;
        let (writes, requests) = mpsc::channel::<Vec<u8>>();
        let (ack, written) = mpsc::channel();
        let notify = wake.clone();
        thread::Builder::new()
            .name("ConPTY input".into())
            .spawn(move || {
                for bytes in requests {
                    let result = writer.write(&bytes);
                    if ack.send(result).is_err() {
                        break;
                    }
                    notify.signal();
                }
            })?;
        let process = console.launch(spawn)?;
        Ok(Self {
            output,
            writes,
            written,
            in_flight: false,
            ack: None,
            chunk: Default::default(),
            wake,
            process,
            console: Some(console),
            input: Default::default(),
        })
    }

    pub fn resize(&mut self, size: Size) -> io::Result<()> {
        self.console.as_ref().map_or(Ok(()), |c| c.resize(size))
    }

    pub fn poll(&mut self, reading: bool, writing: bool, timeout: Duration) -> io::Result<Ready> {
        let mut ready = Ready::default();
        if self.chunk.position() < self.chunk.get_ref().len() as u64 {
            ready.output = true;
        } else {
            match self.output.try_recv() {
                Ok(part) => {
                    self.chunk = io::Cursor::new(part?);
                    ready.output = true;
                }
                Err(TryRecvError::Disconnected) => ready.output = true,
                Err(TryRecvError::Empty) => {}
            }
        }
        if self.in_flight && self.ack.is_none() {
            self.ack = match self.written.try_recv() {
                Ok(result) => Some(result),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => return Err(io::ErrorKind::BrokenPipe.into()),
            };
        }
        ready.writable = writing && (!self.in_flight || self.ack.is_some());
        let mut handles = vec![self.wake.0.as_raw_handle()];
        if reading {
            handles.push(io::stdin().as_raw_handle());
        }
        if self.console.is_some() {
            handles.push(self.process.as_raw_handle());
        }
        let timeout = if ready.output || ready.writable {
            0
        } else {
            timeout
                .as_nanos()
                .div_ceil(1_000_000)
                .min(u128::from(u32::MAX - 1)) as u32
        };
        let result =
            unsafe { WaitForMultipleObjects(handles.len() as u32, handles.as_ptr(), 0, timeout) };
        if result == WAIT_FAILED {
            return Err(io::Error::last_os_error());
        }
        if reading && result == WAIT_OBJECT_0 + 1 {
            ready.input = true;
        }
        if self.console.is_some() && result == WAIT_OBJECT_0 + handles.len() as u32 - 1 {
            // Close can flush a final frame. Continue consuming output while it runs.
            let console = self.console.take();
            thread::spawn(move || drop(console));
        }
        Ok(ready)
    }

    pub fn read_output(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.chunk.read(buf)
    }

    pub fn write_input(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if let Some(result) = self.ack.take() {
            self.in_flight = false;
            return result;
        }
        self.writes
            .send(bytes.to_vec())
            .map_err(|_| io::ErrorKind::BrokenPipe)?;
        self.in_flight = true;
        Err(io::ErrorKind::WouldBlock.into())
    }

    pub fn read_input(&mut self, buf: &mut [u8], detach: Option<u8>) -> io::Result<usize> {
        self.input.read(buf, detach)
    }

    pub fn wait(&mut self) -> io::Result<i32> {
        unsafe {
            if WaitForSingleObject(self.process.as_raw_handle(), INFINITE) == WAIT_FAILED {
                return Err(io::Error::last_os_error());
            }
            let mut code = 0;
            check(GetExitCodeProcess(self.process.as_raw_handle(), &mut code))?;
            Ok(code as i32)
        }
    }
}

#[cfg(test)]
mod tests;
