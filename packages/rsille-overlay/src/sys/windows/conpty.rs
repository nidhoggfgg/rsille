//! Create a child attached to a pseudoconsole with synchronous, non-inherited pipes.
use super::{check, Size, Spawn};
use std::{
    ffi::OsStr,
    fs::File,
    io,
    mem::size_of,
    os::windows::{
        ffi::OsStrExt,
        io::{AsRawHandle, FromRawHandle, OwnedHandle},
    },
    ptr::{null, null_mut},
};
use windows_sys::Win32::{
    Foundation::INVALID_HANDLE_VALUE,
    System::{Console::*, Pipes::CreatePipe, Threading::*},
};

const QUOTE: u16 = b'"' as u16;
const SLASH: u16 = b'\\' as u16;

pub(super) struct Console(HPCON);

impl Drop for Console {
    fn drop(&mut self) {
        unsafe { ClosePseudoConsole(self.0) }
    }
}

fn coord(size: Size) -> COORD {
    COORD {
        X: size.cols as i16,
        Y: size.rows as i16,
    }
}

fn hresult(status: i32) -> io::Result<()> {
    if status < 0 {
        Err(io::Error::from_raw_os_error(status))
    } else {
        Ok(())
    }
}

fn pipe() -> io::Result<(File, File)> {
    let (mut read, mut write) = (null_mut(), null_mut());
    check(unsafe { CreatePipe(&mut read, &mut write, null(), 0) })?;
    Ok(unsafe { (File::from_raw_handle(read), File::from_raw_handle(write)) })
}

impl Console {
    pub(super) fn new(size: Size) -> io::Result<(Self, File, File)> {
        let (input, writer) = pipe()?;
        let (reader, output) = pipe()?;
        let mut console = 0;
        hresult(unsafe {
            CreatePseudoConsole(
                coord(size),
                input.as_raw_handle(),
                output.as_raw_handle(),
                0,
                &mut console,
            )
        })?;
        Ok((Self(console), reader, writer))
    }

    pub(super) fn resize(&self, size: Size) -> io::Result<()> {
        hresult(unsafe { ResizePseudoConsole(self.0, coord(size)) })
    }

    pub(super) fn launch(&self, spawn: &Spawn<'_>) -> io::Result<OwnedHandle> {
        let mut command = Vec::new();
        // argv[0] has its own Windows parsing rule: quotes delimit the executable.
        command.push(QUOTE);
        for c in spawn.program.encode_wide() {
            if matches!(c, 0 | QUOTE) {
                return Err(io::ErrorKind::InvalidInput.into());
            }
            command.push(c);
        }
        command.push(QUOTE);
        for arg in spawn.args {
            command.push(u16::from(b' '));
            append_arg(&mut command, arg)?;
        }
        command.push(0);
        let mut bytes = 0;
        unsafe {
            InitializeProcThreadAttributeList(null_mut(), 1, 0, &mut bytes);
        }
        // Allocate pointer-aligned storage for the opaque attribute list.
        let mut storage = vec![0usize; bytes.div_ceil(size_of::<usize>())];
        let list = storage.as_mut_ptr().cast();
        check(unsafe { InitializeProcThreadAttributeList(list, 1, 0, &mut bytes) })?;
        let mut startup = STARTUPINFOEXW::default();
        startup.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
        startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
        startup.StartupInfo.hStdInput = INVALID_HANDLE_VALUE;
        startup.StartupInfo.hStdOutput = INVALID_HANDLE_VALUE;
        startup.StartupInfo.hStdError = INVALID_HANDLE_VALUE;
        startup.lpAttributeList = list;
        let mut info = PROCESS_INFORMATION::default();
        let result = unsafe {
            check(UpdateProcThreadAttribute(
                list,
                0,
                PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize,
                self.0 as _,
                size_of::<HPCON>(),
                null_mut(),
                null(),
            ))
            .and_then(|_| {
                check(CreateProcessW(
                    null(),
                    command.as_mut_ptr(),
                    null(),
                    null(),
                    0,
                    EXTENDED_STARTUPINFO_PRESENT,
                    null(),
                    null(),
                    &startup.StartupInfo,
                    &mut info,
                ))
            })
        };
        unsafe {
            DeleteProcThreadAttributeList(list);
        }
        result?;
        unsafe {
            drop(OwnedHandle::from_raw_handle(info.hThread));
            Ok(OwnedHandle::from_raw_handle(info.hProcess))
        }
    }
}

// Windows CRT argv rules: double backslashes before a quote or closing delimiter.
fn append_arg(command: &mut Vec<u16>, arg: &OsStr) -> io::Result<()> {
    let quoted = arg.is_empty() || arg.encode_wide().any(|c| matches!(c, 9 | 32));
    if quoted {
        command.push(QUOTE);
    }
    let mut slashes = 0;
    for c in arg.encode_wide() {
        if c == 0 {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        if c == SLASH {
            slashes += 1;
            continue;
        }
        let count = if c == QUOTE { slashes * 2 + 1 } else { slashes };
        command.extend(std::iter::repeat_n(SLASH, count));
        command.push(c);
        slashes = 0;
    }
    command.extend(std::iter::repeat_n(
        SLASH,
        if quoted { slashes * 2 } else { slashes },
    ));
    if quoted {
        command.push(QUOTE);
    }
    Ok(())
}
