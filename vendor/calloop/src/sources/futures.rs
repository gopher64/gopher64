//! A futures executor as an event source
//!
//! Only available with the `executor` cargo feature of `calloop`.
//!
//! This executor is intended for light futures, which will be polled as part of your
//! event loop. Such futures may be waiting for IO, or for some external computation on an
//! other thread for example.
//!
//! You can create a new executor using the `executor` function, which creates a pair
//! `(Executor<T>, Scheduler<T>)` to handle futures that all evaluate to type `T`. The
//! executor should be inserted into your event loop, and will yield the return values of
//! the futures as they finish into your callback. The scheduler can be cloned and used
//! to send futures to be executed into the executor. A generic executor can be obtained
//! by choosing `T = ()` and letting futures handle the forwarding of their return values
//! (if any) by their own means.
//!
//! **Note:** The futures must have their own means of being woken up, as this executor is,
//! by itself, not I/O aware. See [`LoopHandle::adapt_io`](crate::LoopHandle#method.adapt_io)
//! for that, or you can use some other mechanism if you prefer.
use std::{future::Future, pin::Pin, sync::Arc};

use futures_util::{
    stream::{FuturesUnordered, Stream},
    task::{waker_ref, ArcWake, Context, LocalFutureObj, Poll as FutPoll},
};

use crate::{
    sources::{
        channel::{channel, Channel, ChannelError, Event, Sender},
        ping::{make_ping, Ping, PingError, PingSource},
        EventSource,
    },
    Poll, PostAction, Readiness, Token, TokenFactory,
};

/// A future executor as an event source
#[derive(Debug)]
pub struct Executor<T> {
    futures: FuturesUnordered<LocalFutureObj<'static, T>>,
    new_futures: Channel<LocalFutureObj<'static, T>>,
    ready_futures: PingSource,
    waker: Arc<ExecWaker>,
}

/// A scheduler to send futures to an executor
#[derive(Clone, Debug)]
pub struct Scheduler<T> {
    sender: Sender<LocalFutureObj<'static, T>>,
}

impl<T> Scheduler<T> {
    /// Sends the given future to the executor associated to this scheduler
    ///
    /// Returns an error if the the executor not longer exists.
    pub fn schedule<Fut: 'static>(&self, future: Fut) -> Result<(), ExecutorDestroyed>
    where
        Fut: Future<Output = T>,
    {
        let obj = LocalFutureObj::new(Box::new(future));
        self.sender.send(obj).map_err(|_| ExecutorDestroyed)
    }
}

/// Error generated when trying to schedule a future after the
/// executor was destroyed.
#[derive(thiserror::Error, Debug)]
#[error("the executor was destroyed")]
pub struct ExecutorDestroyed;

#[derive(Debug)]
struct ExecWaker {
    ping: Ping,
}

impl ArcWake for ExecWaker {
    fn wake_by_ref(arc_self: &Arc<ExecWaker>) {
        arc_self.ping.ping();
    }
}

/// Create a new executor, and its associated scheduler
///
/// May fail due to OS errors preventing calloop to setup its internal pipes (if your
/// process has reatched its file descriptor limit for example).
pub fn executor<T>() -> crate::Result<(Executor<T>, Scheduler<T>)> {
    let (ping, ready_futures) = make_ping()?;
    let (sender, new_futures) = channel();
    Ok((
        Executor {
            futures: FuturesUnordered::new(),
            new_futures,
            ready_futures,
            waker: Arc::new(ExecWaker { ping }),
        },
        Scheduler { sender },
    ))
}

impl<T> EventSource for Executor<T> {
    type Event = T;
    type Metadata = ();
    type Ret = ();
    type Error = ExecutorError;

    fn process_events<F>(
        &mut self,
        readiness: Readiness,
        token: Token,
        mut callback: F,
    ) -> Result<PostAction, Self::Error>
    where
        F: FnMut(T, &mut ()),
    {
        // fetch all newly inserted futures and push them to the container
        let futures = &mut self.futures;
        self.new_futures
            .process_events(readiness, token, |evt, _| {
                if let Event::Msg(fut) = evt {
                    futures.push(fut);
                }
            })
            .map_err(ExecutorError::NewFutureError)?;

        // process ping events to make it non-ready again
        self.ready_futures
            .process_events(readiness, token, |(), _| {})
            .map_err(ExecutorError::WakeError)?;

        // advance all available futures as much as possible
        let waker = waker_ref(&self.waker);
        let mut cx = Context::from_waker(&waker);

        while let FutPoll::Ready(Some(ret)) = Pin::new(&mut self.futures).poll_next(&mut cx) {
            callback(ret, &mut ());
        }
        Ok(PostAction::Continue)
    }

    fn register(&mut self, poll: &mut Poll, token_factory: &mut TokenFactory) -> crate::Result<()> {
        self.new_futures.register(poll, token_factory)?;
        self.ready_futures.register(poll, token_factory)?;
        Ok(())
    }

    fn reregister(
        &mut self,
        poll: &mut Poll,
        token_factory: &mut TokenFactory,
    ) -> crate::Result<()> {
        self.new_futures.reregister(poll, token_factory)?;
        self.ready_futures.reregister(poll, token_factory)?;
        Ok(())
    }

    fn unregister(&mut self, poll: &mut Poll) -> crate::Result<()> {
        self.new_futures.unregister(poll)?;
        self.ready_futures.unregister(poll)?;
        Ok(())
    }
}

/// An error arising from processing events in an async executor event source.
#[derive(thiserror::Error, Debug)]
pub enum ExecutorError {
    /// Error while reading new futures added via [`Scheduler::schedule()`].
    #[error("error adding new futures")]
    NewFutureError(ChannelError),

    /// Error while processing wake events from existing futures.
    #[error("error processing wake events")]
    WakeError(PingError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ready() {
        let mut event_loop = crate::EventLoop::<u32>::try_new().unwrap();

        let handle = event_loop.handle();

        let (exec, sched) = executor::<u32>().unwrap();

        handle
            .insert_source(exec, move |ret, &mut (), got| {
                *got = ret;
            })
            .unwrap();

        let mut got = 0;

        let fut = async { 42 };

        event_loop
            .dispatch(Some(::std::time::Duration::ZERO), &mut got)
            .unwrap();

        // the future is not yet inserted, and thus has not yet run
        assert_eq!(got, 0);

        sched.schedule(fut).unwrap();

        event_loop
            .dispatch(Some(::std::time::Duration::ZERO), &mut got)
            .unwrap();

        // the future has run
        assert_eq!(got, 42);
    }
}
