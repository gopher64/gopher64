use std::cell::{Cell, RefCell};
use std::fmt::Debug;
use std::io;
use std::os::unix::io::AsRawFd;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use slotmap::SlotMap;

use crate::sources::{Dispatcher, EventSource, Idle, IdleDispatcher};
use crate::{EventDispatcher, InsertError, Poll, PostAction, TokenFactory};

type IdleCallback<'i, Data> = Rc<RefCell<dyn IdleDispatcher<Data> + 'i>>;

slotmap::new_key_type! {
    pub(crate) struct CalloopKey;
}

/// A token representing a registration in the [`EventLoop`].
///
/// This token is given to you by the [`EventLoop`] when an [`EventSource`] is inserted or
/// a [`Dispatcher`] is registered. You can use it to [disable](LoopHandle#method.disable),
/// [enable](LoopHandle#method.enable), [update`](LoopHandle#method.update),
/// [remove](LoopHandle#method.remove) or [kill](LoopHandle#method.kill) it.
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct RegistrationToken {
    key: CalloopKey,
}

pub(crate) struct LoopInner<'l, Data> {
    pub(crate) poll: RefCell<Poll>,
    pub(crate) sources: RefCell<SlotMap<CalloopKey, Rc<dyn EventDispatcher<Data> + 'l>>>,
    idles: RefCell<Vec<IdleCallback<'l, Data>>>,
    pending_action: Cell<PostAction>,
}

/// An handle to an event loop
///
/// This handle allows you to insert new sources and idles in this event loop,
/// it can be cloned, and it is possible to insert new sources from within a source
/// callback.
pub struct LoopHandle<'l, Data> {
    inner: Rc<LoopInner<'l, Data>>,
}

impl<'l, Data> std::fmt::Debug for LoopHandle<'l, Data> {
    #[cfg_attr(coverage, no_coverage)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LoopHandle { ... }")
    }
}

impl<'l, Data> Clone for LoopHandle<'l, Data> {
    #[cfg_attr(coverage, no_coverage)]
    fn clone(&self) -> Self {
        LoopHandle {
            inner: self.inner.clone(),
        }
    }
}

impl<'l, Data> LoopHandle<'l, Data> {
    /// Inserts a new event source in the loop.
    ///
    /// The provided callback will be called during the dispatching cycles whenever the
    /// associated source generates events, see `EventLoop::dispatch(..)` for details.
    ///
    /// This function takes ownership of the event source. Use `register_dispatcher`
    /// if you need access to the event source after this call.
    pub fn insert_source<S, F>(
        &self,
        source: S,
        callback: F,
    ) -> Result<RegistrationToken, InsertError<S>>
    where
        S: EventSource + 'l,
        F: FnMut(S::Event, &mut S::Metadata, &mut Data) -> S::Ret + 'l,
    {
        let dispatcher = Dispatcher::new(source, callback);
        self.register_dispatcher(dispatcher.clone())
            .map_err(|error| InsertError {
                error,
                inserted: dispatcher.into_source_inner(),
            })
    }

    /// Registers a `Dispatcher` in the loop.
    ///
    /// Use this function if you need access to the event source after its insertion in the loop.
    ///
    /// See also `insert_source`.
    pub fn register_dispatcher<S>(
        &self,
        dispatcher: Dispatcher<'l, S, Data>,
    ) -> crate::Result<RegistrationToken>
    where
        S: EventSource + 'l,
    {
        let mut sources = self.inner.sources.borrow_mut();
        let mut poll = self.inner.poll.borrow_mut();

        let key = sources.insert(dispatcher.clone_as_event_dispatcher());
        let ret = sources
            .get(key)
            .unwrap()
            .register(&mut *poll, &mut TokenFactory::new(key));

        if let Err(error) = ret {
            sources.remove(key).expect("Source was just inserted?!");
            return Err(error);
        }

        Ok(RegistrationToken { key })
    }

    /// Inserts an idle callback.
    ///
    /// This callback will be called during a dispatching cycle when the event loop has
    /// finished processing all pending events from the sources and becomes idle.
    pub fn insert_idle<'i, F: FnOnce(&mut Data) + 'l + 'i>(&self, callback: F) -> Idle<'i> {
        let mut opt_cb = Some(callback);
        let callback = Rc::new(RefCell::new(Some(move |data: &mut Data| {
            if let Some(cb) = opt_cb.take() {
                cb(data);
            }
        })));
        self.inner.idles.borrow_mut().push(callback.clone());
        Idle { callback }
    }

    /// Enables this previously disabled event source.
    ///
    /// This previously disabled source will start generating events again.
    ///
    /// **Note:** this cannot be done from within the source callback.
    pub fn enable(&self, token: &RegistrationToken) -> crate::Result<()> {
        if let Some(source) = self.inner.sources.borrow().get(token.key) {
            source.register(
                &mut *self.inner.poll.borrow_mut(),
                &mut TokenFactory::new(token.key),
            )?;
        }
        Ok(())
    }

    /// Makes this source update its registration.
    ///
    /// If after accessing the source you changed its parameters in a way that requires
    /// updating its registration.
    pub fn update(&self, token: &RegistrationToken) -> crate::Result<()> {
        if let Some(source) = self.inner.sources.borrow().get(token.key) {
            if !source.reregister(
                &mut *self.inner.poll.borrow_mut(),
                &mut TokenFactory::new(token.key),
            )? {
                // we are in a callback, store for later processing
                self.inner.pending_action.set(PostAction::Reregister);
            }
        }
        Ok(())
    }

    /// Disables this event source.
    ///
    /// The source remains in the event loop, but it'll no longer generate events
    pub fn disable(&self, token: &RegistrationToken) -> crate::Result<()> {
        if let Some(source) = self.inner.sources.borrow().get(token.key) {
            if !source.unregister(&mut *self.inner.poll.borrow_mut())? {
                // we are in a callback, store for later processing
                self.inner.pending_action.set(PostAction::Disable);
            }
        }
        Ok(())
    }

    /// Removes this source from the event loop.
    pub fn remove(&self, token: RegistrationToken) {
        if let Some(source) = self.inner.sources.borrow_mut().remove(token.key) {
            if let Err(e) = source.unregister(&mut self.inner.poll.borrow_mut()) {
                log::warn!(
                    "[calloop] Failed to unregister source from the polling system: {:?}",
                    e
                );
            }
        }
    }

    /// Wrap an IO object into an async adapter
    ///
    /// This adapter turns the IO object into an async-aware one that can be used in futures.
    /// The readiness of these futures will be driven by the event loop.
    ///
    /// The produced futures can be polled in any executor, and notably the one provided by
    /// calloop.
    pub fn adapt_io<F: AsRawFd>(&self, fd: F) -> crate::Result<crate::io::Async<'l, F>> {
        crate::io::Async::new(self.inner.clone(), fd)
    }
}

/// An event loop
///
/// This loop can host several event sources, that can be dynamically added or removed.
pub struct EventLoop<'l, Data> {
    handle: LoopHandle<'l, Data>,
    stop_signal: Arc<AtomicBool>,
    ping: crate::sources::ping::Ping,
}

impl<'l, Data> std::fmt::Debug for EventLoop<'l, Data> {
    #[cfg_attr(coverage, no_coverage)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EventLoop { ... }")
    }
}

impl<'l, Data> EventLoop<'l, Data> {
    /// Create a new event loop
    ///
    /// Fails if the initialization of the polling system failed.
    pub fn try_new() -> crate::Result<Self> {
        Self::inner_new(false)
    }

    /// Create a new event loop in high precision mode
    ///
    /// On some platforms it requires to setup more resources to enable high-precision
    /// (sub millisecond) capabilities, so you should use this constructor if you need
    /// this kind of precision.
    ///
    /// Fails if the initialization of the polling system failed.
    pub fn try_new_high_precision() -> crate::Result<Self> {
        Self::inner_new(true)
    }

    fn inner_new(high_precision: bool) -> crate::Result<Self> {
        let poll = Poll::new(high_precision)?;
        let handle = LoopHandle {
            inner: Rc::new(LoopInner {
                poll: RefCell::new(poll),
                sources: RefCell::new(SlotMap::with_key()),
                idles: RefCell::new(Vec::new()),
                pending_action: Cell::new(PostAction::Continue),
            }),
        };
        let (ping, ping_source) = crate::sources::ping::make_ping()?;
        handle.insert_source(ping_source, |_, _, _| {})?;
        Ok(EventLoop {
            handle,
            stop_signal: Arc::new(AtomicBool::new(false)),
            ping,
        })
    }

    /// Retrieve a loop handle
    pub fn handle(&self) -> LoopHandle<'l, Data> {
        self.handle.clone()
    }

    fn dispatch_events(
        &mut self,
        mut timeout: Option<Duration>,
        data: &mut Data,
    ) -> crate::Result<()> {
        let now = Instant::now();
        let events = {
            let mut poll = self.handle.inner.poll.borrow_mut();
            loop {
                let result = poll.poll(timeout);

                match result {
                    Ok(events) => break events,
                    Err(crate::Error::IoError(err)) if err.kind() == io::ErrorKind::Interrupted => {
                        // Interrupted by a signal. Update timeout and retry.
                        if let Some(to) = timeout {
                            let elapsed = now.elapsed();
                            if elapsed >= to {
                                return Ok(());
                            } else {
                                timeout = Some(to - elapsed);
                            }
                        }
                    }
                    Err(err) => return Err(err),
                };
            }
        };

        for event in events {
            let opt_disp = self
                .handle
                .inner
                .sources
                .borrow()
                .get(event.token.key)
                .cloned();

            if let Some(disp) = opt_disp {
                let mut ret = disp.process_events(event.readiness, event.token, data)?;

                // if the returned PostAction is Continue, it may be overwritten by an user-specified pending action
                let pending_action = self
                    .handle
                    .inner
                    .pending_action
                    .replace(PostAction::Continue);
                if let PostAction::Continue = ret {
                    ret = pending_action;
                }

                match ret {
                    PostAction::Reregister => {
                        disp.reregister(
                            &mut self.handle.inner.poll.borrow_mut(),
                            &mut TokenFactory::new(event.token.key),
                        )?;
                    }
                    PostAction::Disable => {
                        disp.unregister(&mut self.handle.inner.poll.borrow_mut())?;
                    }
                    PostAction::Remove => {
                        // delete the source from the list, it'll be cleaned up with the if just below
                        self.handle
                            .inner
                            .sources
                            .borrow_mut()
                            .remove(event.token.key);
                    }
                    PostAction::Continue => {}
                }

                if !self
                    .handle
                    .inner
                    .sources
                    .borrow()
                    .contains_key(event.token.key)
                {
                    // the source has been removed from within its callback, unregister it
                    let mut poll = self.handle.inner.poll.borrow_mut();
                    if let Err(e) = disp.unregister(&mut *poll) {
                        log::warn!(
                            "[calloop] Failed to unregister source from the polling system: {:?}",
                            e
                        );
                    }
                }
            } else {
                log::warn!(
                    "[calloop] Received an event for non-existence source: {:?}",
                    event.token.key
                );
            }
        }

        Ok(())
    }

    fn dispatch_idles(&mut self, data: &mut Data) {
        let idles = std::mem::take(&mut *self.handle.inner.idles.borrow_mut());
        for idle in idles {
            idle.borrow_mut().dispatch(data);
        }
    }

    fn invoke_pre_run(&self, data: &mut Data) -> crate::Result<()> {
        let sources = self
            .handle
            .inner
            .sources
            .borrow()
            .values()
            .cloned()
            .collect::<Vec<_>>();

        for source in sources {
            source.pre_run(data)?;
        }

        Ok(())
    }

    fn invoke_post_run(&self, data: &mut Data) -> crate::Result<()> {
        let sources = self
            .handle
            .inner
            .sources
            .borrow()
            .values()
            .cloned()
            .collect::<Vec<_>>();

        for source in sources {
            source.post_run(data)?;
        }

        Ok(())
    }

    /// Dispatch pending events to their callbacks
    ///
    /// If some sources have events available, their callbacks will be immediatly called.
    /// Otherwise this will wait until an event is receive or the provided `timeout`
    /// is reached. If `timeout` is `None`, it will wait without a duration limit.
    ///
    /// Once pending events have been processed or the timeout is reached, all pending
    /// idle callbacks will be fired before this method returns.
    pub fn dispatch<D: Into<Option<Duration>>>(
        &mut self,
        timeout: D,
        data: &mut Data,
    ) -> crate::Result<()> {
        self.invoke_pre_run(data)?;
        self.dispatch_events(timeout.into(), data)?;
        self.dispatch_idles(data);
        self.invoke_post_run(data)?;

        Ok(())
    }

    /// Get a signal to stop this event loop from running
    ///
    /// To be used in conjunction with the `run()` method.
    pub fn get_signal(&self) -> LoopSignal {
        LoopSignal {
            signal: self.stop_signal.clone(),
            ping: self.ping.clone(),
        }
    }

    /// Run this event loop
    ///
    /// This will repeatedly try to dispatch events (see the `dispatch()` method) on
    /// this event loop, waiting at most `timeout` every time.
    ///
    /// Between each dispatch wait, your provided callback will be called.
    ///
    /// You can use the `get_signal()` method to retrieve a way to stop or wakeup
    /// the event loop from anywhere.
    pub fn run<F, D: Into<Option<Duration>>>(
        &mut self,
        timeout: D,
        data: &mut Data,
        mut cb: F,
    ) -> crate::Result<()>
    where
        F: FnMut(&mut Data),
    {
        let timeout = timeout.into();
        self.stop_signal.store(false, Ordering::Release);
        self.invoke_pre_run(data)?;
        while !self.stop_signal.load(Ordering::Acquire) {
            self.dispatch_events(timeout, data)?;
            self.dispatch_idles(data);
            cb(data);
        }
        self.invoke_post_run(data)?;
        Ok(())
    }
}

/// A signal that can be shared between thread to stop or wakeup a running
/// event loop
#[derive(Clone)]
pub struct LoopSignal {
    signal: Arc<AtomicBool>,
    ping: crate::sources::ping::Ping,
}

impl std::fmt::Debug for LoopSignal {
    #[cfg_attr(coverage, no_coverage)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LoopSignal { ... }")
    }
}

impl LoopSignal {
    /// Stop the event loop
    ///
    /// Once this method is called, the next time the event loop has finished
    /// waiting for events, it will return rather than starting to wait again.
    ///
    /// This is only usefull if you are using the `EventLoop::run()` method.
    pub fn stop(&self) {
        self.signal.store(true, Ordering::Release);
    }

    /// Wake up the event loop
    ///
    /// This sends a dummy event to the event loop to simulate the reception
    /// of an event, making the wait return early. Called after `stop()`, this
    /// ensures the event loop will terminate quickly if you specified a long
    /// timeout (or no timeout at all) to the `dispatch` or `run` method.
    pub fn wakeup(&self) {
        self.ping.ping();
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::{
        generic::Generic, ping::*, Dispatcher, Interest, Mode, Poll, PostAction, Readiness,
        RegistrationToken, Token, TokenFactory,
    };

    use super::EventLoop;

    #[test]
    fn dispatch_idle() {
        let mut event_loop = EventLoop::try_new().unwrap();

        let mut dispatched = false;

        event_loop.handle().insert_idle(|d| {
            *d = true;
        });

        event_loop
            .dispatch(Some(Duration::ZERO), &mut dispatched)
            .unwrap();

        assert!(dispatched);
    }

    #[test]
    fn cancel_idle() {
        let mut event_loop = EventLoop::try_new().unwrap();

        let mut dispatched = false;

        let handle = event_loop.handle();
        let idle = handle.insert_idle(move |d| {
            *d = true;
        });

        idle.cancel();

        event_loop
            .dispatch(Duration::ZERO, &mut dispatched)
            .unwrap();

        assert!(!dispatched);
    }

    #[test]
    fn wakeup() {
        let mut event_loop = EventLoop::try_new().unwrap();

        let signal = event_loop.get_signal();

        ::std::thread::spawn(move || {
            ::std::thread::sleep(Duration::from_millis(500));
            signal.wakeup();
        });

        // the test should return
        event_loop.dispatch(None, &mut ()).unwrap();
    }

    #[test]
    fn wakeup_stop() {
        let mut event_loop = EventLoop::try_new().unwrap();

        let signal = event_loop.get_signal();

        ::std::thread::spawn(move || {
            ::std::thread::sleep(Duration::from_millis(500));
            signal.stop();
            signal.wakeup();
        });

        // the test should return
        event_loop.run(None, &mut (), |_| {}).unwrap();
    }

    #[test]
    fn insert_bad_source() {
        let event_loop = EventLoop::<()>::try_new().unwrap();
        let ret = event_loop.handle().insert_source(
            crate::sources::generic::Generic::new(420, Interest::READ, Mode::Level),
            |_, _, _| Ok(PostAction::Continue),
        );
        assert!(ret.is_err());
    }

    #[test]
    fn insert_source_no_interest() {
        let event_loop = EventLoop::<()>::try_new().unwrap();
        let ret = event_loop.handle().insert_source(
            crate::sources::generic::Generic::new(0, Interest::EMPTY, Mode::Level),
            |_, _, _| Ok(PostAction::Continue),
        );
        assert!(ret.is_ok());
    }

    #[test]
    fn disarm_rearm() {
        let mut event_loop = EventLoop::<bool>::try_new().unwrap();
        let (ping, ping_source) = make_ping().unwrap();

        let ping_token = event_loop
            .handle()
            .insert_source(ping_source, |(), &mut (), dispatched| {
                *dispatched = true;
            })
            .unwrap();

        ping.ping();
        let mut dispatched = false;
        event_loop
            .dispatch(Duration::ZERO, &mut dispatched)
            .unwrap();
        assert!(dispatched);

        // disable the source
        ping.ping();
        event_loop.handle().disable(&ping_token).unwrap();
        let mut dispatched = false;
        event_loop
            .dispatch(Duration::ZERO, &mut dispatched)
            .unwrap();
        assert!(!dispatched);

        // disabling it again is an error
        event_loop.handle().disable(&ping_token).unwrap_err();

        // reenable it, the previous ping now gets dispatched
        event_loop.handle().enable(&ping_token).unwrap();
        let mut dispatched = false;
        event_loop
            .dispatch(Duration::ZERO, &mut dispatched)
            .unwrap();
        assert!(dispatched);
    }

    #[test]
    fn multiple_tokens() {
        struct DoubleSource {
            ping1: PingSource,
            ping2: PingSource,
        }

        impl crate::EventSource for DoubleSource {
            type Event = u32;
            type Metadata = ();
            type Ret = ();
            type Error = PingError;

            fn process_events<F>(
                &mut self,
                readiness: Readiness,
                token: Token,
                mut callback: F,
            ) -> Result<PostAction, Self::Error>
            where
                F: FnMut(Self::Event, &mut Self::Metadata) -> Self::Ret,
            {
                self.ping1
                    .process_events(readiness, token, |(), &mut ()| callback(1, &mut ()))?;
                self.ping2
                    .process_events(readiness, token, |(), &mut ()| callback(2, &mut ()))?;
                Ok(PostAction::Continue)
            }

            fn register(
                &mut self,
                poll: &mut Poll,
                token_factory: &mut TokenFactory,
            ) -> crate::Result<()> {
                self.ping1.register(poll, token_factory)?;
                self.ping2.register(poll, token_factory)?;
                Ok(())
            }

            fn reregister(
                &mut self,
                poll: &mut Poll,
                token_factory: &mut TokenFactory,
            ) -> crate::Result<()> {
                self.ping1.reregister(poll, token_factory)?;
                self.ping2.reregister(poll, token_factory)?;
                Ok(())
            }

            fn unregister(&mut self, poll: &mut Poll) -> crate::Result<()> {
                self.ping1.unregister(poll)?;
                self.ping2.unregister(poll)?;
                Ok(())
            }
        }

        let mut event_loop = EventLoop::<u32>::try_new().unwrap();

        let (ping1, source1) = make_ping().unwrap();
        let (ping2, source2) = make_ping().unwrap();

        let source = DoubleSource {
            ping1: source1,
            ping2: source2,
        };

        event_loop
            .handle()
            .insert_source(source, |i, _, d| {
                eprintln!("Dispatching {}", i);
                *d += i
            })
            .unwrap();

        let mut dispatched = 0;
        ping1.ping();
        event_loop
            .dispatch(Duration::ZERO, &mut dispatched)
            .unwrap();
        assert_eq!(dispatched, 1);

        dispatched = 0;
        ping2.ping();
        event_loop
            .dispatch(Duration::ZERO, &mut dispatched)
            .unwrap();
        assert_eq!(dispatched, 2);

        dispatched = 0;
        ping1.ping();
        ping2.ping();
        event_loop
            .dispatch(Duration::ZERO, &mut dispatched)
            .unwrap();
        assert_eq!(dispatched, 3);
    }

    #[test]
    fn change_interests() {
        use nix::sys::socket::{recv, socketpair, AddressFamily, MsgFlags, SockFlag, SockType};
        use nix::unistd::write;
        let mut event_loop = EventLoop::<bool>::try_new().unwrap();

        let (sock1, sock2) = socketpair(
            AddressFamily::Unix,
            SockType::Stream,
            None,
            SockFlag::empty(), // recv with DONTWAIT will suffice for platforms without SockFlag::SOCK_NONBLOCKING such as macOS
        )
        .unwrap();

        let source = Generic::new(sock1, Interest::READ, Mode::Level);
        let dispatcher = Dispatcher::new(source, |_, &mut fd, dispatched| {
            *dispatched = true;
            // read all contents available to drain the socket
            let mut buf = [0u8; 32];
            loop {
                match recv(fd, &mut buf, MsgFlags::MSG_DONTWAIT) {
                    Ok(0) => break, // closed pipe, we are now inert
                    Ok(_) => {}
                    Err(e) => {
                        let e: std::io::Error = e.into();
                        if e.kind() == std::io::ErrorKind::WouldBlock {
                            break;
                        // nothing more to read
                        } else {
                            // propagate error
                            return Err(e);
                        }
                    }
                }
            }
            Ok(PostAction::Continue)
        });

        let sock_token_1 = event_loop
            .handle()
            .register_dispatcher(dispatcher.clone())
            .unwrap();

        // first dispatch, nothing is readable
        let mut dispatched = false;
        event_loop
            .dispatch(Duration::ZERO, &mut dispatched)
            .unwrap();
        assert!(!dispatched);

        // write something, the socket becomes readable
        write(sock2, &[1, 2, 3]).unwrap();
        dispatched = false;
        event_loop
            .dispatch(Duration::ZERO, &mut dispatched)
            .unwrap();
        assert!(dispatched);

        // All has been read, no longer readable
        dispatched = false;
        event_loop
            .dispatch(Duration::ZERO, &mut dispatched)
            .unwrap();
        assert!(!dispatched);

        // change the interests for writability instead
        dispatcher.as_source_mut().interest = Interest::WRITE;
        event_loop.handle().update(&sock_token_1).unwrap();

        // the socket is writable
        dispatched = false;
        event_loop
            .dispatch(Duration::ZERO, &mut dispatched)
            .unwrap();
        assert!(dispatched);

        // change back to readable
        dispatcher.as_source_mut().interest = Interest::READ;
        event_loop.handle().update(&sock_token_1).unwrap();

        // the socket is not readable
        dispatched = false;
        event_loop
            .dispatch(Duration::ZERO, &mut dispatched)
            .unwrap();
        assert!(!dispatched);
    }

    #[test]
    fn kill_source() {
        let mut event_loop = EventLoop::<Option<RegistrationToken>>::try_new().unwrap();

        let handle = event_loop.handle();
        let (ping, ping_source) = make_ping().unwrap();
        let ping_token = event_loop
            .handle()
            .insert_source(ping_source, move |(), &mut (), opt_src| {
                if let Some(src) = opt_src.take() {
                    handle.remove(src);
                }
            })
            .unwrap();

        ping.ping();

        let mut opt_src = Some(ping_token);

        event_loop.dispatch(Duration::ZERO, &mut opt_src).unwrap();

        assert!(opt_src.is_none());
    }

    #[test]
    fn non_static_data() {
        use std::sync::mpsc;

        let (sender, receiver) = mpsc::channel();

        {
            struct RefSender<'a>(&'a mpsc::Sender<()>);
            let mut ref_sender = RefSender(&sender);

            let mut event_loop = EventLoop::<RefSender<'_>>::try_new().unwrap();
            let (ping, ping_source) = make_ping().unwrap();
            let _ping_token = event_loop
                .handle()
                .insert_source(ping_source, |_, _, ref_sender| {
                    ref_sender.0.send(()).unwrap();
                })
                .unwrap();

            ping.ping();

            event_loop
                .dispatch(Duration::ZERO, &mut ref_sender)
                .unwrap();
        }

        receiver.recv().unwrap();
        // sender still usable (e.g. for another EventLoop)
        drop(sender);
    }

    // A dummy EventSource to test insertion and removal of sources
    struct DummySource;

    impl crate::EventSource for DummySource {
        type Event = ();
        type Metadata = ();
        type Ret = ();
        type Error = crate::Error;

        fn process_events<F>(
            &mut self,
            _: Readiness,
            _: Token,
            mut callback: F,
        ) -> Result<PostAction, Self::Error>
        where
            F: FnMut(Self::Event, &mut Self::Metadata) -> Self::Ret,
        {
            callback((), &mut ());
            Ok(PostAction::Continue)
        }

        fn register(&mut self, _: &mut Poll, _: &mut TokenFactory) -> crate::Result<()> {
            Ok(())
        }

        fn reregister(&mut self, _: &mut Poll, _: &mut TokenFactory) -> crate::Result<()> {
            Ok(())
        }

        fn unregister(&mut self, _: &mut Poll) -> crate::Result<()> {
            Ok(())
        }
    }
}
