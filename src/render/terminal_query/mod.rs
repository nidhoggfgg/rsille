//! Unix-only CSI queries and the input owner for managed terminal loops.
//!
//! Once `prepare_input` or `cell_pixels` activates this broker, all subsequent
//! input must use `event_stream`, never Crossterm's reader (including cursor and
//! capability queries). Conversely, `reserve_crossterm_input` permanently blocks
//! broker activation and must precede any use of stock Crossterm input.
//! Drop the stream before restoring terminal modes. The dormant worker neither
//! reads nor holds raw mode. Only stdin/stdout terminals with a controlling tty
//! are supported; this includes Windows Terminal through WSL, not native Windows.
#![cfg(unix)]

mod decoder;
mod parser;

use crossterm::event::{Event, KeyboardEnhancementFlags};
use futures::Stream;
use std::{
    collections::VecDeque,
    fs::{File, OpenOptions},
    io::{self, IsTerminal, Read, Write},
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    pin::Pin,
    sync::{mpsc, Arc, Mutex, MutexGuard},
    task::{Context, Poll, Waker},
    thread,
    time::{Duration, Instant},
};

const QUERY_TIMEOUT: Duration = Duration::from_millis(500);
const TICK: Duration = Duration::from_millis(10);
const RESIZE_INTERVAL: Duration = Duration::from_millis(100);
const MAX_EVENTS: usize = 4096;
const MAX_EVENT_BYTES: usize = 4 * 1024 * 1024;
static BROKER: Mutex<InputOwner> = Mutex::new(InputOwner::Unclaimed);

// Both ownership decisions use this same lock, including broker construction.
// Neither a dropped stream nor a failed subsequent query releases ownership:
// Crossterm cannot transfer buffered events or join its reader on stream drop.
enum InputOwner {
    Unclaimed,
    Crossterm,
    Broker(Arc<Broker>),
}

impl InputOwner {
    fn reserve_crossterm(&mut self) -> io::Result<()> {
        match self {
            Self::Broker(_) => Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "terminal input is permanently owned by the query broker",
            )),
            Self::Unclaimed | Self::Crossterm => {
                *self = Self::Crossterm;
                Ok(())
            }
        }
    }

    fn initialize(
        &mut self,
        start: impl FnOnce() -> io::Result<Broker>,
    ) -> io::Result<Arc<Broker>> {
        match self {
            Self::Crossterm => Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "terminal input is permanently reserved for Crossterm",
            )),
            Self::Broker(broker) => Ok(broker.clone()),
            Self::Unclaimed => {
                let broker = Arc::new(start()?);
                *self = Self::Broker(broker.clone());
                Ok(broker)
            }
        }
    }

    fn broker(&self) -> Option<&Arc<Broker>> {
        match self {
            Self::Broker(broker) => Some(broker),
            Self::Unclaimed | Self::Crossterm => None,
        }
    }
}

type Grid = (u16, u16);
type Pixels = (u16, u16);
type Reply<T> = mpsc::Sender<io::Result<T>>;

// The upstream parser's private event type, deliberately not exposed to callers.
#[derive(Debug)]
#[allow(dead_code)]
enum InternalEvent {
    Event(Event),
    CursorPosition(u16, u16),
    KeyboardEnhancementFlags(KeyboardEnhancementFlags),
    PrimaryDeviceAttributes,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn disconnected() -> io::Error {
    io::Error::new(io::ErrorKind::BrokenPipe, "terminal input broker stopped")
}

fn timeout() -> io::Error {
    io::Error::new(io::ErrorKind::TimedOut, "terminal query deadline elapsed")
}

/// Reports whether input ownership has permanently transferred to this broker.
pub(super) fn is_active() -> bool {
    lock(&BROKER).broker().is_some()
}

/// Permanently reserves input for stock Crossterm before its stream, keyboard
/// query, or cursor query is used. Repeated reservations succeed, but an active
/// broker returns `AlreadyExists`. Dropping a stock stream does not release this
/// reservation; subsequent broker activation is forbidden for this process.
pub(super) fn reserve_crossterm_input() -> io::Result<()> {
    lock(&BROKER).reserve_crossterm()
}

/// Initializes the shared input owner without querying, reading, or enabling raw
/// mode. Call before constructing any input stream, including when OS metrics
/// currently work, so a later pixel fallback can share the live stream's reader.
/// Repeated calls reuse the same broker; terminal stdin/stdout are required.
/// Returns `AlreadyExists` if stock Crossterm input has been reserved.
#[cfg(feature = "sixel")]
pub(super) fn prepare_input() -> io::Result<()> {
    initialized_broker().map(|_| ())
}

fn initialized_broker() -> io::Result<Arc<Broker>> {
    lock(&BROKER).initialize(Broker::start)
}

// The worker enforces absolute query deadlines and responds only after restoring
// temporary raw mode. A caller-side recv_timeout could return before restoration
// and race TerminalGuard; attach/detach likewise need the worker's acknowledgement.
fn receive_reply<T>(receive: mpsc::Receiver<io::Result<T>>) -> io::Result<T> {
    receive.recv().map_err(|_| disconnected())?
}

/// Queries CSI 16 t, returning (width, height), cached for the current grid.
/// A failed query is cached too; a grid change permits a retry.
pub(super) fn cell_pixels() -> io::Result<Pixels> {
    let broker = initialized_broker()?;
    let (reply, receive) = mpsc::channel();
    broker.send(Command::Pixels {
        deadline: Instant::now() + QUERY_TIMEOUT,
        reply,
    })?;
    receive_reply(receive)
}

fn active_broker() -> io::Result<Arc<Broker>> {
    lock(&BROKER).broker().cloned().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotConnected,
            "terminal input broker is not active",
        )
    })
}

/// Negotiates CSI ? u / primary DA without invoking Crossterm's input reader.
/// Call only after `is_active()`; an inactive broker is not created here.
pub(super) fn keyboard_supported() -> io::Result<bool> {
    let broker = active_broker()?;
    let (reply, receive) = mpsc::channel();
    broker.send(Command::Keyboard {
        deadline: Instant::now() + QUERY_TIMEOUT,
        reply,
    })?;
    receive_reply(receive)
}

/// Queries CSI 6 n, returning the zero-based (column, row) cursor position.
/// Like keyboard negotiation, this requires an already active broker. It does
/// not cache positions and shares the same 500 ms absolute query deadline.
pub(super) fn cursor_position() -> io::Result<(u16, u16)> {
    let broker = active_broker()?;
    let (reply, receive) = mpsc::channel();
    broker.send(Command::CursorPosition {
        deadline: Instant::now() + QUERY_TIMEOUT,
        reply,
    })?;
    receive_reply(receive)
}

/// Attaches the single input consumer, after terminal setup has completed.
pub(super) fn event_stream() -> io::Result<QueryEventStream> {
    let broker = active_broker()?;
    let (reply, receive) = mpsc::channel();
    broker.send(Command::Attach(reply))?;
    receive_reply(receive)?;
    Ok(QueryEventStream { broker })
}

/// The broker-backed replacement for Crossterm's Unix `EventStream`.
pub(super) struct QueryEventStream {
    broker: Arc<Broker>,
}

impl Stream for QueryEventStream {
    type Item = io::Result<Event>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut queue = lock(&self.broker.queue);
        if let Some(event) = queue.events.pop_front() {
            queue.bytes -= event_size(&event);
            return Poll::Ready(Some(event));
        }
        if queue.overflow {
            queue.overflow = false;
            return Poll::Ready(Some(Err(io::Error::other(
                "terminal event queue overflow; some input was discarded",
            ))));
        }
        if queue.closed {
            return Poll::Ready(None);
        }
        queue.waker = Some(cx.waker().clone());
        Poll::Pending
    }
}

impl Drop for QueryEventStream {
    fn drop(&mut self) {
        let (reply, receive) = mpsc::channel();
        // Unlike ordinary requests, detach must not be lost when commands queue
        // up. The worker checks commands every bounded read/poll iteration.
        if self.broker.commands.send(Command::Detach(reply)).is_ok() {
            let _ = receive.recv();
        }
    }
}

struct Broker {
    commands: mpsc::SyncSender<Command>,
    queue: Arc<Mutex<EventQueue>>,
}

impl Broker {
    fn start() -> io::Result<Self> {
        if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "terminal queries require terminal stdin and stdout",
            ));
        }
        let tty = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_NOCTTY | libc::O_CLOEXEC)
            .open("/dev/tty")?;
        // A process may retain one controlling tty while redirecting stdin or
        // stdout to a different PTY. Never query/read the unrelated terminal.
        // tcgetsid accepts aliases such as /dev/tty and /dev/pts/N, unlike a
        // device-number comparison of the file nodes themselves.
        // SAFETY: the descriptor is owned by the live tty File.
        let session = unsafe { libc::tcgetsid(tty.as_raw_fd()) };
        if session < 0
            || [io::stdin().as_raw_fd(), io::stdout().as_raw_fd()]
                .iter()
                .any(|fd| {
                    // SAFETY: these are borrowed live terminal descriptors; tcgetsid
                    // only reads terminal metadata and does not consume input.
                    (unsafe { libc::tcgetsid(*fd) }) != session
                })
        {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "terminal queries require stdin and stdout on the same controlling terminal",
            ));
        }
        let (commands, receive) = mpsc::sync_channel(16);
        let queue = Arc::new(Mutex::new(EventQueue::default()));
        let worker_queue = queue.clone();
        thread::Builder::new()
            .name("rsille-terminal-input".into())
            .spawn(move || Worker::new(tty, worker_queue).run(receive))?;
        Ok(Self { commands, queue })
    }

    fn send(&self, command: Command) -> io::Result<()> {
        self.commands
            .try_send(command)
            .map_err(|error| match error {
                mpsc::TrySendError::Full(_) => {
                    io::Error::new(io::ErrorKind::WouldBlock, "terminal broker is busy")
                }
                mpsc::TrySendError::Disconnected(_) => disconnected(),
            })
    }
}

enum Command {
    Pixels {
        deadline: Instant,
        reply: Reply<Pixels>,
    },
    Keyboard {
        deadline: Instant,
        reply: Reply<bool>,
    },
    CursorPosition {
        deadline: Instant,
        reply: Reply<(u16, u16)>,
    },
    Attach(Reply<()>),
    Detach(Reply<()>),
}

#[derive(Default)]
struct EventQueue {
    events: VecDeque<io::Result<Event>>,
    bytes: usize,
    overflow: bool,
    closed: bool,
    waker: Option<Waker>,
}

fn event_size(event: &io::Result<Event>) -> usize {
    std::mem::size_of::<Event>()
        + match event {
            Ok(Event::Paste(text)) => text.len(),
            _ => 0,
        }
}

fn enqueue(queue: &Mutex<EventQueue>, event: io::Result<Event>) {
    let waker = {
        let mut queue = lock(queue);
        let size = event_size(&event);
        if queue.events.len() >= MAX_EVENTS || queue.bytes + size > MAX_EVENT_BYTES {
            queue.overflow = true;
        } else {
            queue.bytes += size;
            queue.events.push_back(event);
        }
        queue.waker.take()
    };
    if let Some(waker) = waker {
        waker.wake();
    }
}

#[derive(Clone)]
struct SavedError(io::ErrorKind, String);
impl SavedError {
    fn capture(error: &io::Error) -> Self {
        Self(error.kind(), error.to_string())
    }
    fn error(&self) -> io::Error {
        io::Error::new(self.0, self.1.clone())
    }
}

#[derive(Default)]
struct MetricsCache(Option<(Grid, Result<Pixels, SavedError>)>);
impl MetricsCache {
    fn get(&self, grid: Grid) -> Option<io::Result<Pixels>> {
        self.0
            .as_ref()
            .filter(|(cached, _)| *cached == grid)
            .map(|(_, value)| value.as_ref().copied().map_err(SavedError::error))
    }
    fn store(&mut self, grid: Grid, value: &io::Result<Pixels>) {
        self.0 = Some((grid, value.as_ref().copied().map_err(SavedError::capture)));
    }
    fn observe(&mut self, grid: Grid) {
        if self.0.as_ref().is_some_and(|(cached, _)| *cached != grid) {
            self.0 = None;
        }
    }
}

/// Only the worker owns this guard; responses and detach acknowledgements are
/// sent after restoration, so the caller cannot race its terminal cleanup.
struct TemporaryRaw(bool);
impl TemporaryRaw {
    fn acquire() -> io::Result<Self> {
        let enabled = !crossterm::terminal::is_raw_mode_enabled()?;
        if enabled {
            crossterm::terminal::enable_raw_mode()?;
        }
        Ok(Self(enabled))
    }
    fn restore(&mut self) -> io::Result<()> {
        if self.0 {
            crossterm::terminal::disable_raw_mode()?;
            self.0 = false;
        }
        Ok(())
    }
}
impl Drop for TemporaryRaw {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

enum Query {
    Pixels { grid: Grid, reply: Reply<Pixels> },
    Keyboard { da: bool, reply: Reply<bool> },
    CursorPosition { reply: Reply<(u16, u16)> },
}
struct Pending {
    query: Query,
    deadline: Instant,
    output: &'static [u8],
    written: usize,
    raw: TemporaryRaw,
}

enum Answer {
    Pixels(Pixels),
    Keyboard(bool),
    CursorPosition(u16, u16),
}

struct Worker {
    tty: File,
    queue: Arc<Mutex<EventQueue>>,
    decoder: decoder::Decoder,
    attached: bool,
    pending: Option<Pending>,
    cache: MetricsCache,
    grid: Option<Grid>,
    next_resize: Instant,
    failed: Option<SavedError>,
}

impl Worker {
    fn new(tty: File, queue: Arc<Mutex<EventQueue>>) -> Self {
        Self {
            tty,
            queue,
            decoder: decoder::Decoder::default(),
            attached: false,
            pending: None,
            cache: MetricsCache::default(),
            grid: None,
            next_resize: Instant::now(),
            failed: None,
        }
    }

    fn run(mut self, commands: mpsc::Receiver<Command>) {
        loop {
            if !self.attached && self.pending.is_none() {
                match commands.recv() {
                    Ok(command) => self.command(command),
                    Err(_) => break,
                }
            }
            // A bounded command batch also prevents request floods starving a
            // query deadline or the keyboard.
            for _ in 0..16 {
                match commands.try_recv() {
                    Ok(command) => self.command(command),
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => return,
                }
            }
            self.expire(Instant::now());
            if !self.attached && self.pending.is_none() {
                continue;
            }
            if self.failed.is_some() {
                // A closed tty is permanent for this broker, not a poll spin.
                match commands.recv() {
                    Ok(command) => self.command(command),
                    Err(_) => break,
                }
                continue;
            }
            self.write_query();
            self.expire(Instant::now());
            if !self.attached && self.pending.is_none() {
                continue;
            }
            let now = Instant::now();
            if self.attached && now >= self.next_resize {
                self.next_resize = now + RESIZE_INTERVAL;
                if let Ok(grid) = terminal_grid(&self.tty) {
                    self.cache.observe(grid);
                    if self.grid.replace(grid).is_some_and(|old| old != grid) {
                        enqueue(&self.queue, Ok(Event::Resize(grid.0, grid.1)));
                    }
                }
            }
            if let Some(item) = self.decoder.expire(now) {
                self.decoded(item);
            }
            let full = {
                let queue = lock(&self.queue);
                queue.events.len() >= MAX_EVENTS || queue.bytes >= MAX_EVENT_BYTES
            };
            if full && self.pending.is_none() {
                // Backpressure instead of unbounded input allocation.
                thread::sleep(TICK);
                continue;
            }
            let wait = self.pending.as_ref().map_or(TICK, |query| {
                TICK.min(query.deadline.saturating_duration_since(Instant::now()))
            });
            match readable(&self.tty, wait) {
                Ok(true) => {
                    // One bounded read per iteration. Check the absolute deadline
                    // even under an endless stream of unrelated input.
                    self.expire(Instant::now());
                    if !self.attached && self.pending.is_none() {
                        continue;
                    }
                    let mut bytes = [0; 1024];
                    match self.tty.read(&mut bytes) {
                        Ok(0) => self.fail(io::Error::new(
                            io::ErrorKind::UnexpectedEof,
                            "terminal input closed",
                        )),
                        Ok(count) => {
                            for byte in &bytes[..count] {
                                if let Some(item) = self.decoder.push(*byte, Instant::now()) {
                                    self.decoded(item);
                                }
                            }
                        }
                        Err(error)
                            if matches!(
                                error.kind(),
                                io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                            ) => {}
                        Err(error) => self.fail(error),
                    }
                }
                Ok(false) => {}
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => self.fail(error),
            }
        }
    }

    fn command(&mut self, command: Command) {
        match command {
            Command::Attach(reply) => {
                let result = if self.attached {
                    Err(io::Error::new(
                        io::ErrorKind::AlreadyExists,
                        "terminal input stream already attached",
                    ))
                } else if let Some(error) = &self.failed {
                    Err(error.error())
                } else {
                    self.attached = true;
                    self.next_resize = Instant::now();
                    // Keep the previous grid to deliver changes during dormancy.
                    if self.grid.is_none() {
                        self.grid = terminal_grid(&self.tty).ok();
                    }
                    Ok(())
                };
                // A cancelled attach must not leave an invisible active reader.
                let newly_attached = result.is_ok();
                if reply.send(result).is_err() && newly_attached {
                    self.attached = false;
                }
            }
            Command::Detach(reply) => {
                self.attached = false;
                lock(&self.queue).waker = None;
                if self.pending.is_some() {
                    self.finish(Err(io::Error::new(
                        io::ErrorKind::Interrupted,
                        "terminal stream detached",
                    )));
                }
                let _ = reply.send(Ok(()));
            }
            Command::Pixels { deadline, reply } => {
                let grid = match terminal_grid(&self.tty) {
                    Ok(grid) => grid,
                    Err(error) => {
                        let _ = reply.send(Err(error));
                        return;
                    }
                };
                self.cache.observe(grid);
                if let Some(value) = self.cache.get(grid) {
                    let _ = reply.send(value);
                    return;
                }
                self.begin(Query::Pixels { grid, reply }, deadline, b"\x1b[16t");
            }
            Command::CursorPosition { deadline, reply } => {
                self.begin(Query::CursorPosition { reply }, deadline, b"\x1b[6n");
            }
            Command::Keyboard { deadline, reply } => {
                self.begin(
                    Query::Keyboard { da: false, reply },
                    deadline,
                    b"\x1b[?u\x1b[c",
                );
            }
        }
    }

    fn begin(&mut self, query: Query, deadline: Instant, output: &'static [u8]) {
        let raw = if self.pending.is_some() {
            Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "another terminal query is pending",
            ))
        } else if let Some(error) = &self.failed {
            Err(error.error())
        } else if Instant::now() >= deadline {
            Err(timeout())
        } else {
            TemporaryRaw::acquire()
        };
        match raw {
            Ok(raw) => {
                self.pending = Some(Pending {
                    query,
                    deadline,
                    output,
                    written: 0,
                    raw,
                })
            }
            Err(error) => {
                // Busy is transient and must not poison the metrics cache.
                self.respond(query, Err(error));
            }
        }
    }

    fn write_query(&mut self) {
        let Some(query) = &mut self.pending else {
            return;
        };
        if query.written == query.output.len() {
            return;
        }
        match self.tty.write(&query.output[query.written..]) {
            Ok(0) => self.finish(Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "terminal query write returned zero",
            ))),
            Ok(count) => query.written += count,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) => {}
            Err(error) => self.finish(Err(error)),
        }
    }

    fn expire(&mut self, now: Instant) {
        if self
            .pending
            .as_ref()
            .is_some_and(|query| now >= query.deadline)
        {
            let result = match self.pending.as_ref().map(|query| &query.query) {
                Some(Query::Keyboard { da: true, .. }) => Ok(Answer::Keyboard(false)),
                _ => Err(timeout()),
            };
            self.finish(result);
        }
    }

    fn decoded(&mut self, item: decoder::Decoded) {
        self.expire(Instant::now());
        match item {
            decoder::Decoded::Event(event) => enqueue(&self.queue, Ok(event)),
            decoder::Decoded::Error(error) => enqueue(&self.queue, Err(error)),
            decoder::Decoded::Pixels(value) => {
                if matches!(
                    self.pending.as_ref().map(|p| &p.query),
                    Some(Query::Pixels { .. })
                ) {
                    self.finish(value.map(Answer::Pixels));
                }
                // Unsolicited/late terminal replies are never keyboard input.
            }
            decoder::Decoded::CursorPosition(column, row) => {
                if matches!(
                    self.pending.as_ref().map(|p| &p.query),
                    Some(Query::CursorPosition { .. })
                ) {
                    self.finish(Ok(Answer::CursorPosition(column, row)));
                }
            }
            decoder::Decoded::KeyboardFlags => {
                if matches!(
                    self.pending.as_ref().map(|p| &p.query),
                    Some(Query::Keyboard { .. })
                ) {
                    self.finish(Ok(Answer::Keyboard(true)));
                }
            }
            decoder::Decoded::DeviceAttributes => {
                if let Some(Pending {
                    query: Query::Keyboard { da, .. },
                    ..
                }) = &mut self.pending
                {
                    *da = true;
                }
                // DA can arrive before CSI ? u. Do not declare unsupported until
                // the absolute deadline, allowing either response order.
            }
        }
    }

    fn finish(&mut self, result: io::Result<Answer>) {
        if let Some(mut pending) = self.pending.take() {
            let result = pending.raw.restore().and(result);
            // Retry restoration in Drop before the caller is unblocked.
            drop(pending.raw);
            // Remaining bytes in the current read buffer may still be parsed,
            // but no further OS read occurs without an attachment or new query.
            self.respond(pending.query, result);
        }
    }

    fn respond(&mut self, query: Query, result: io::Result<Answer>) {
        match query {
            Query::Pixels { grid, reply } => {
                let result = result.and_then(|answer| match answer {
                    Answer::Pixels(value) => Ok(value),
                    _ => Err(io::Error::other("unexpected terminal query response")),
                });
                if !result.as_ref().is_err_and(|error| {
                    matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    )
                }) {
                    self.cache.store(grid, &result);
                }
                let _ = reply.send(result);
            }
            Query::CursorPosition { reply } => {
                let result = result.and_then(|answer| match answer {
                    Answer::CursorPosition(column, row) => Ok((column, row)),
                    _ => Err(io::Error::other("unexpected terminal query response")),
                });
                let _ = reply.send(result);
            }
            Query::Keyboard { reply, .. } => {
                let result = result.and_then(|answer| match answer {
                    Answer::Keyboard(value) => Ok(value),
                    _ => Err(io::Error::other("unexpected terminal query response")),
                });
                let _ = reply.send(result);
            }
        }
    }

    fn fail(&mut self, error: io::Error) {
        self.failed = Some(SavedError::capture(&error));
        self.finish(Err(io::Error::new(error.kind(), error.to_string())));
        lock(&self.queue).closed = true;
        enqueue(&self.queue, Err(error));
    }
}

fn terminal_grid(tty: &File) -> io::Result<Grid> {
    // SAFETY: winsize is a plain C struct and ioctl writes only its own size to
    // the valid pointer; the descriptor belongs to the worker's live tty File.
    let mut size: libc::winsize = unsafe { std::mem::zeroed() };
    if unsafe { libc::ioctl(tty.as_raw_fd(), libc::TIOCGWINSZ, &mut size) } == -1 {
        return Err(io::Error::last_os_error());
    }
    if size.ws_col == 0 || size.ws_row == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "terminal grid is empty",
        ));
    }
    Ok((size.ws_col, size.ws_row))
}

fn readable(tty: &File, timeout: Duration) -> io::Result<bool> {
    let mut descriptor = libc::pollfd {
        fd: tty.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    // Round up so a sub-millisecond remainder does not busy-spin.
    let milliseconds = timeout.as_nanos().div_ceil(1_000_000);
    // SAFETY: poll receives one initialized pollfd and a bounded timeout.
    let result = unsafe {
        libc::poll(
            &mut descriptor,
            1,
            milliseconds.min(i32::MAX as u128) as i32,
        )
    };
    if result < 0 {
        return Err(io::Error::last_os_error());
    }
    if descriptor.revents & libc::POLLIN != 0 {
        return Ok(true);
    }
    if descriptor.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 {
        return Err(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "terminal disconnected",
        ));
    }
    Ok(false)
}

#[cfg(test)]
mod tests;
