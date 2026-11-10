use super::*;
use crossterm::event::KeyCode;

fn inert_broker() -> io::Result<Broker> {
    let (commands, _receive) = mpsc::sync_channel(1);
    Ok(Broker {
        commands,
        queue: Arc::new(Mutex::new(EventQueue::default())),
    })
}

#[test]
fn stock_input_reservation_is_permanent_and_idempotent() {
    let mut owner = InputOwner::Unclaimed;
    for _ in 0..3 {
        owner.reserve_crossterm().unwrap();
        let result = owner.initialize(|| panic!("reserved input must not start a broker"));
        assert!(matches!(result, Err(error) if error.kind() == io::ErrorKind::AlreadyExists));
        assert!(owner.broker().is_none());
        assert!(matches!(owner, InputOwner::Crossterm));
    }
}

#[test]
fn broker_ownership_rejects_stock_input_and_reuses_existing_broker() {
    let mut owner = InputOwner::Unclaimed;
    let broker = owner.initialize(inert_broker).unwrap();
    assert_eq!(
        owner.reserve_crossterm().unwrap_err().kind(),
        io::ErrorKind::AlreadyExists
    );
    let reused = owner
        .initialize(|| panic!("broker must not be restarted"))
        .unwrap();
    assert!(Arc::ptr_eq(&broker, &reused));
    drop(broker);
    drop(reused);
    // The process owner retains the broker even when all callers let go.
    assert!(owner.broker().is_some());
    assert!(owner.reserve_crossterm().is_err());
}

#[test]
fn failed_broker_start_leaves_input_unclaimed() {
    let mut owner = InputOwner::Unclaimed;
    let result = owner.initialize(|| Err(io::Error::new(io::ErrorKind::Unsupported, "no tty")));
    assert!(matches!(result, Err(error) if error.kind() == io::ErrorKind::Unsupported));
    assert!(matches!(owner, InputOwner::Unclaimed));
    owner.reserve_crossterm().unwrap();
}

#[test]
fn concurrent_stock_reservation_and_broker_activation_have_one_winner() {
    use std::sync::Barrier;

    let owner = Arc::new(Mutex::new(InputOwner::Unclaimed));
    let barrier = Arc::new(Barrier::new(3));
    let stock_owner = owner.clone();
    let stock_barrier = barrier.clone();
    let stock = thread::spawn(move || {
        stock_barrier.wait();
        lock(&stock_owner).reserve_crossterm().is_ok()
    });
    let broker_owner = owner.clone();
    let broker_barrier = barrier.clone();
    let broker = thread::spawn(move || {
        broker_barrier.wait();
        lock(&broker_owner).initialize(inert_broker).is_ok()
    });
    barrier.wait();
    let stock_won = stock.join().unwrap();
    let broker_won = broker.join().unwrap();
    assert_ne!(stock_won, broker_won);
    let mut owner = lock(&owner);
    assert_eq!(owner.broker().is_some(), broker_won);
    assert_eq!(owner.reserve_crossterm().is_ok(), stock_won);
}

fn worker() -> Worker {
    Worker::new(
        File::open("/dev/null").unwrap(),
        Arc::new(Mutex::new(EventQueue::default())),
    )
}

fn pixel_query(worker: &mut Worker, deadline: Instant) -> mpsc::Receiver<io::Result<Pixels>> {
    let (reply, receive) = mpsc::channel();
    worker.pending = Some(Pending {
        query: Query::Pixels {
            grid: (80, 24),
            reply,
        },
        deadline,
        output: b"\x1b[16t",
        written: 5,
        raw: TemporaryRaw(false),
    });
    receive
}

fn keyboard_query(worker: &mut Worker, deadline: Instant) -> mpsc::Receiver<io::Result<bool>> {
    let (reply, receive) = mpsc::channel();
    worker.pending = Some(Pending {
        query: Query::Keyboard { da: false, reply },
        deadline,
        output: b"\x1b[?u\x1b[c",
        written: 7,
        raw: TemporaryRaw(false),
    });
    receive
}

fn cursor_query(worker: &mut Worker, deadline: Instant) -> mpsc::Receiver<io::Result<(u16, u16)>> {
    let (reply, receive) = mpsc::channel();
    worker.pending = Some(Pending {
        query: Query::CursorPosition { reply },
        deadline,
        output: b"\x1b[6n",
        written: 0,
        raw: TemporaryRaw(false),
    });
    receive
}

#[test]
fn cursor_replies_route_only_to_cursor_queries_without_touching_metrics() {
    let mut worker = worker();
    let pixels = pixel_query(&mut worker, Instant::now() + QUERY_TIMEOUT);
    worker.decoded(decoder::Decoded::CursorPosition(29, 11));
    assert!(matches!(pixels.try_recv(), Err(mpsc::TryRecvError::Empty)));
    worker.decoded(decoder::Decoded::Pixels(Ok((8, 19))));
    assert_eq!(pixels.recv().unwrap().unwrap(), (8, 19));

    let cursor = cursor_query(&mut worker, Instant::now() + QUERY_TIMEOUT);
    worker.decoded(decoder::Decoded::Pixels(Ok((9, 20))));
    worker.decoded(decoder::Decoded::KeyboardFlags);
    worker.decoded(decoder::Decoded::DeviceAttributes);
    assert!(matches!(cursor.try_recv(), Err(mpsc::TryRecvError::Empty)));
    worker.decoded(decoder::Decoded::CursorPosition(29, 11));
    assert_eq!(cursor.recv().unwrap().unwrap(), (29, 11));
    assert_eq!(worker.cache.get((80, 24)).unwrap().unwrap(), (8, 19));
    worker.decoded(decoder::Decoded::CursorPosition(0, 0));
    assert!(lock(&worker.queue).events.is_empty());
}

#[test]
fn cursor_timeout_is_absolute_and_late_replies_remain_internal() {
    let mut worker = worker();
    let deadline = Instant::now() + QUERY_TIMEOUT;
    let cursor = cursor_query(&mut worker, deadline);
    for _ in 0..100 {
        worker.decoded(decoder::Decoded::Event(Event::Key(
            KeyCode::Char('a').into(),
        )));
    }
    worker.expire(deadline - Duration::from_nanos(1));
    assert!(matches!(cursor.try_recv(), Err(mpsc::TryRecvError::Empty)));
    worker.expire(deadline);
    assert_eq!(
        cursor.recv().unwrap().unwrap_err().kind(),
        io::ErrorKind::TimedOut
    );
    worker.decoded(decoder::Decoded::CursorPosition(29, 11));
    assert_eq!(lock(&worker.queue).events.len(), 100);
    assert!(worker.cache.0.is_none());

    let cursor = cursor_query(&mut worker, Instant::now() - Duration::from_millis(1));
    worker.decoded(decoder::Decoded::CursorPosition(29, 11));
    assert_eq!(
        cursor.recv().unwrap().unwrap_err().kind(),
        io::ErrorKind::TimedOut
    );
}

#[test]
fn cursor_query_uses_shared_transport_and_preserves_keys_before_and_after_reply() {
    use std::os::{fd::OwnedFd, unix::net::UnixStream};

    let (input, mut terminal) = UnixStream::pair().unwrap();
    input.set_nonblocking(true).unwrap();
    terminal
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    let queue = Arc::new(Mutex::new(EventQueue::default()));
    let mut worker = Worker::new(File::from(OwnedFd::from(input)), queue.clone());
    let reply = cursor_query(&mut worker, Instant::now() + QUERY_TIMEOUT);
    let (commands, receive) = mpsc::sync_channel(16);
    let thread = thread::spawn(move || worker.run(receive));
    let mut query = [0; 4];
    terminal.read_exact(&mut query).unwrap();
    assert_eq!(&query, b"\x1b[6n");
    terminal.write_all(b"a\x1b[12;30Rz").unwrap();
    assert_eq!(
        reply.recv_timeout(Duration::from_secs(1)).unwrap().unwrap(),
        (29, 11)
    );
    // Joining also waits for the rest of the already-read buffer to be parsed.
    drop(commands);
    thread.join().unwrap();
    let queue = lock(&queue);
    let events: Vec<_> = queue.events.iter().collect();
    assert!(
        matches!(&events[..], [Ok(Event::Key(first)), Ok(Event::Key(last))]
        if first.code == KeyCode::Char('a') && last.code == KeyCode::Char('z'))
    );
}

#[test]
fn metrics_cache_keeps_success_and_failure_only_for_the_observed_grid() {
    let mut cache = MetricsCache::default();
    assert!(cache.get((80, 24)).is_none());
    cache.store((80, 24), &Ok((8, 19)));
    for _ in 0..100 {
        assert_eq!(cache.get((80, 24)).unwrap().unwrap(), (8, 19));
    }
    assert!(cache.get((81, 24)).is_none());
    cache.observe((81, 24));
    assert!(cache.get((80, 24)).is_none());
    cache.store((81, 24), &Err(timeout()));
    for _ in 0..100 {
        assert_eq!(
            cache.get((81, 24)).unwrap().unwrap_err().kind(),
            io::ErrorKind::TimedOut
        );
    }
    cache.observe((81, 25));
    assert!(cache.get((81, 24)).is_none());
}

#[test]
fn deadline_is_absolute_and_input_does_not_extend_it() {
    let mut worker = worker();
    let deadline = Instant::now() + QUERY_TIMEOUT;
    let receive = pixel_query(&mut worker, deadline);
    for _ in 0..100 {
        worker.decoded(decoder::Decoded::Event(Event::Key(
            KeyCode::Char('a').into(),
        )));
    }
    worker.expire(deadline - Duration::from_nanos(1));
    assert!(matches!(receive.try_recv(), Err(mpsc::TryRecvError::Empty)));
    worker.expire(deadline);
    assert_eq!(
        receive.recv().unwrap().unwrap_err().kind(),
        io::ErrorKind::TimedOut
    );
    assert!(worker.pending.is_none());
    assert_eq!(
        worker.cache.get((80, 24)).unwrap().unwrap_err().kind(),
        io::ErrorKind::TimedOut
    );
    worker.decoded(decoder::Decoded::Pixels(Ok((8, 19))));
    assert_eq!(lock(&worker.queue).events.len(), 100);
    // A late report must not turn the cached failed query into a success.
    assert!(worker.cache.get((80, 24)).unwrap().is_err());
}

#[test]
fn reports_after_deadline_cannot_complete_a_query() {
    let mut worker = worker();
    let receive = pixel_query(&mut worker, Instant::now() - Duration::from_millis(1));
    worker.decoded(decoder::Decoded::Pixels(Ok((8, 19))));
    assert_eq!(
        receive.recv().unwrap().unwrap_err().kind(),
        io::ErrorKind::TimedOut
    );
}

#[test]
fn successful_and_invalid_reports_are_cached() {
    let mut worker = worker();
    let receive = pixel_query(&mut worker, Instant::now() + QUERY_TIMEOUT);
    worker.decoded(decoder::Decoded::Pixels(Ok((8, 19))));
    assert_eq!(receive.recv().unwrap().unwrap(), (8, 19));
    assert_eq!(worker.cache.get((80, 24)).unwrap().unwrap(), (8, 19));
    let receive = pixel_query(&mut worker, Instant::now() + QUERY_TIMEOUT);
    worker.decoded(decoder::Decoded::Pixels(Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "zero dimensions",
    ))));
    assert_eq!(
        receive.recv().unwrap().unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
    assert_eq!(
        worker.cache.get((80, 24)).unwrap().unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
}

#[test]
fn keyboard_negotiation_accepts_either_reply_order() {
    for da_first in [false, true] {
        let mut worker = worker();
        let receive = keyboard_query(&mut worker, Instant::now() + QUERY_TIMEOUT);
        if da_first {
            worker.decoded(decoder::Decoded::DeviceAttributes);
            assert!(matches!(receive.try_recv(), Err(mpsc::TryRecvError::Empty)));
        }
        worker.decoded(decoder::Decoded::KeyboardFlags);
        assert!(receive.recv().unwrap().unwrap());
        if !da_first {
            worker.decoded(decoder::Decoded::DeviceAttributes);
        }
        assert!(lock(&worker.queue).events.is_empty());
    }
}

#[test]
fn keyboard_da_without_flags_is_unsupported_at_deadline() {
    let mut worker = worker();
    let deadline = Instant::now() + QUERY_TIMEOUT;
    let receive = keyboard_query(&mut worker, deadline);
    worker.decoded(decoder::Decoded::DeviceAttributes);
    worker.expire(deadline);
    assert!(!receive.recv().unwrap().unwrap());
    let receive = keyboard_query(&mut worker, deadline);
    worker.expire(deadline);
    assert_eq!(
        receive.recv().unwrap().unwrap_err().kind(),
        io::ErrorKind::TimedOut
    );
}

#[test]
fn detach_acknowledges_only_after_canceling_query_and_read_ownership() {
    let mut worker = worker();
    worker.attached = true;
    let query = pixel_query(&mut worker, Instant::now() + QUERY_TIMEOUT);
    let (reply, receive) = mpsc::channel();
    worker.command(Command::Detach(reply));
    receive.recv().unwrap().unwrap();
    assert!(!worker.attached);
    assert!(worker.pending.is_none());
    assert_eq!(
        query.recv().unwrap().unwrap_err().kind(),
        io::ErrorKind::Interrupted
    );
    assert!(worker.cache.get((80, 24)).is_none());
}

#[test]
fn event_buffer_is_bounded_and_overflow_is_explicit() {
    let queue = Mutex::new(EventQueue::default());
    for _ in 0..MAX_EVENTS + 10 {
        enqueue(&queue, Ok(Event::Key(KeyCode::Char('x').into())));
    }
    let queue = lock(&queue);
    assert_eq!(queue.events.len(), MAX_EVENTS);
    assert!(queue.bytes <= MAX_EVENT_BYTES);
    assert!(queue.overflow);
}

#[test]
fn stream_is_unpin_and_delivers_preflight_queue() {
    fn assert_unpin<T: Unpin>() {}
    assert_unpin::<QueryEventStream>();
    let (commands, receive) = mpsc::sync_channel(1);
    let queue = Arc::new(Mutex::new(EventQueue::default()));
    enqueue(&queue, Ok(Event::Key(KeyCode::Char('p').into())));
    let broker = Arc::new(Broker { commands, queue });
    let mut stream = QueryEventStream { broker };
    let waker = futures::task::noop_waker();
    let mut cx = Context::from_waker(&waker);
    assert!(
        matches!(Pin::new(&mut stream).poll_next(&mut cx), Poll::Ready(Some(Ok(Event::Key(key)))) if key.code == KeyCode::Char('p'))
    );
    assert!(matches!(
        Pin::new(&mut stream).poll_next(&mut cx),
        Poll::Pending
    ));
    drop(receive); // No actual worker in this queue-only test.
}

#[test]
fn injected_transport_is_unread_while_dormant_and_after_stream_drop() {
    use std::os::{fd::OwnedFd, unix::net::UnixStream};

    let (input, mut terminal) = UnixStream::pair().unwrap();
    input.set_nonblocking(true).unwrap();
    let observer = input.try_clone().unwrap();
    let tty = File::from(OwnedFd::from(input));
    let (commands, receive) = mpsc::sync_channel(16);
    let queue = Arc::new(Mutex::new(EventQueue::default()));
    let worker = Worker::new(tty, queue.clone());
    let thread = thread::spawn(move || worker.run(receive));
    let broker = Arc::new(Broker { commands, queue });

    fn unread(input: &UnixStream) -> i32 {
        let mut count: libc::c_int = 0;
        // SAFETY: FIONREAD writes one int to a valid pointer for a live socket.
        assert_eq!(
            unsafe { libc::ioctl(input.as_raw_fd(), libc::FIONREAD, &mut count) },
            0
        );
        count
    }

    terminal.set_nonblocking(true).unwrap();
    terminal.write_all(b"a").unwrap();
    thread::sleep(TICK * 3);
    let mut output = [0; 16];
    assert_eq!(
        terminal.read(&mut output).unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
    assert_eq!(unread(&observer), 1);
    assert!(lock(&broker.queue).events.is_empty());

    let (reply, receive) = mpsc::channel();
    broker.send(Command::Attach(reply)).unwrap();
    receive
        .recv_timeout(Duration::from_secs(1))
        .unwrap()
        .unwrap();
    let mut stream = QueryEventStream {
        broker: broker.clone(),
    };
    let deadline = Instant::now() + Duration::from_secs(1);
    let waker = futures::task::noop_waker();
    let mut cx = Context::from_waker(&waker);
    loop {
        match Pin::new(&mut stream).poll_next(&mut cx) {
            Poll::Ready(Some(Ok(Event::Key(key)))) => {
                assert_eq!(key.code, KeyCode::Char('a'));
                break;
            }
            Poll::Pending => {
                assert!(
                    Instant::now() < deadline,
                    "worker did not read attached input"
                );
                thread::sleep(TICK);
            }
            event => panic!("unexpected input: {event:?}"),
        }
    }
    drop(stream); // Must acknowledge before returning, not just request shutdown.
    terminal.write_all(b"b").unwrap();
    thread::sleep(TICK * 3);
    assert_eq!(unread(&observer), 1);
    assert!(lock(&broker.queue).events.is_empty());
    drop(broker);
    thread.join().unwrap();
}
