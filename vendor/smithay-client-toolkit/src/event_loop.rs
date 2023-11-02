use std::{io, os::unix::io::RawFd};

use calloop::{
    generic::Generic, EventSource, InsertError, Interest, LoopHandle, Mode, PostAction,
    RegistrationToken, TokenFactory,
};

use wayland_client::{EventQueue, ReadEventsGuard};

/// An adapter to insert a Wayland `EventQueue` into a calloop event loop
///
/// This is a struct that implements `calloop::EventSource`. It generates an
/// event whenever events need to be dispatched. At this point your calloop callback
/// will be given access to the `EventQueue` and you should call `.dispatch_pending()`
/// and forward its return value, allowing you to handle orphan events as you prefer.
///
/// If you don't use orphan events, the `quick_insert` method will directly
/// insert the source into a provided `LoopHandle` with an adapter which will panic
/// whenever an oprhan event is encountered.
#[derive(Debug)]
pub struct WaylandSource {
    queue: EventQueue,
    fd: Generic<RawFd>,
    read_guard: Option<ReadEventsGuard>,
}

impl WaylandSource {
    /// Wrap an `EventQueue` as a `WaylandSource`.
    pub fn new(queue: EventQueue) -> WaylandSource {
        let fd = queue.display().get_connection_fd();
        WaylandSource { queue, fd: Generic::new(fd, Interest::READ, Mode::Level), read_guard: None }
    }

    /// Insert this source into given event loop with an adapter that panics on orphan events
    ///
    /// The adapter will pass the event loop's global shared data as `dispatch_data` too all
    /// callbacks.
    pub fn quick_insert<Data: 'static>(
        self,
        handle: LoopHandle<Data>,
    ) -> Result<RegistrationToken, InsertError<WaylandSource>> {
        handle.insert_source(self, |(), queue, ddata| {
            queue.dispatch_pending(ddata, |event, object, _| {
                panic!(
                    "[calloop] Encountered an orphan event: {}@{} : {}",
                    event.interface,
                    object.as_ref().id(),
                    event.name
                );
            })
        })
    }

    /// Access the underlying event queue
    ///
    /// This method can be used if you need to access the underlying `EventQueue` while this
    /// `WaylandSource` is currently inserted in an event loop.
    ///
    /// Note that you should be careful when interacting with it if you invoke methods that
    /// interact with the wayland socket (such as `dispatch()` or `prepare_read()`). These may
    /// interefere with the proper waking up of this event source in the event loop.
    pub fn queue(&mut self) -> &mut EventQueue {
        &mut self.queue
    }
}

impl EventSource for WaylandSource {
    type Event = ();
    type Error = std::io::Error;
    type Metadata = EventQueue;
    type Ret = std::io::Result<u32>;

    fn process_events<F>(
        &mut self,
        readiness: calloop::Readiness,
        token: calloop::Token,
        mut callback: F,
    ) -> std::io::Result<PostAction>
    where
        F: FnMut((), &mut EventQueue) -> std::io::Result<u32>,
    {
        let queue = &mut self.queue;
        let read_guard = &mut self.read_guard;
        self.fd.process_events(readiness, token, |_, _| {
            // 1. read events from the socket if any are available
            if let Some(guard) = read_guard.take() {
                // might be None if some other thread read events before us, concurently
                if let Err(e) = guard.read_events() {
                    if e.kind() != io::ErrorKind::WouldBlock {
                        return Err(e);
                    }
                }
            }
            // 2. dispatch any pending event in the queue (that's callback's job)
            loop {
                match queue.prepare_read() {
                    Some(guard) => {
                        *read_guard = Some(guard);
                        break;
                    }
                    None => {
                        callback((), queue)?;
                    }
                }
            }
            // 3. Once dispatching is finished, flush the responses to the compositor
            if let Err(e) = queue.display().flush() {
                if e.kind() != io::ErrorKind::WouldBlock {
                    // in case of error, forward it and fast-exit
                    return Err(e);
                }
                // WouldBlock error means the compositor could not process all our messages
                // quickly. Either it is slowed down or we are a spammer.
                // Should not really happen, if it does we do nothing and will flush again later
            }
            Ok(PostAction::Continue)
        })
    }

    fn register(
        &mut self,
        poll: &mut calloop::Poll,
        token_factory: &mut TokenFactory,
    ) -> calloop::Result<()> {
        self.fd.register(poll, token_factory)
    }

    fn reregister(
        &mut self,
        poll: &mut calloop::Poll,
        token_factory: &mut TokenFactory,
    ) -> calloop::Result<()> {
        self.fd.reregister(poll, token_factory)
    }

    fn unregister(&mut self, poll: &mut calloop::Poll) -> calloop::Result<()> {
        self.fd.unregister(poll)
    }

    fn pre_run<F>(&mut self, mut callback: F) -> calloop::Result<()>
    where
        F: FnMut((), &mut EventQueue) -> std::io::Result<u32>,
    {
        debug_assert!(self.read_guard.is_none());
        // flush the display before starting to poll
        if let Err(e) = self.queue.display().flush() {
            if e.kind() != io::ErrorKind::WouldBlock {
                // in case of error, don't prepare a read, if the error is persitent,
                // it'll trigger in other wayland methods anyway
                log::error!("Error trying to flush the wayland display: {}", e);
                return Err(e.into());
            }
        }

        loop {
            match self.queue.prepare_read() {
                Some(guard) => {
                    self.read_guard = Some(guard);
                    break;
                }
                None => {
                    callback((), &mut self.queue)?;
                }
            }
        }

        Ok(())
    }

    fn post_run<F>(&mut self, _: F) -> calloop::Result<()>
    where
        F: FnMut((), &mut EventQueue) -> std::io::Result<u32>,
    {
        // the destructor of ReadEventsGuard does the cleanup
        self.read_guard = None;
        Ok(())
    }
}
