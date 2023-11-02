use std::pin::Pin;
use std::sync::{Arc, Mutex};

use std::task::{Context, Poll, Waker};

struct FutureState<R> {
    waker: Option<Waker>,
    data: Option<R>,
}

unsafe impl<R> Send for FutureState<R> {}

pub struct ThreadFuture<R> {
    state: Arc<Mutex<FutureState<R>>>,
}

unsafe impl<R> Send for ThreadFuture<R> {}

impl<R: 'static> ThreadFuture<R> {
    pub fn new<F: FnOnce(&mut Option<R>) + Send + 'static>(f: F) -> Self {
        let state = Arc::new(Mutex::new(FutureState {
            waker: None,
            data: None,
        }));

        {
            let state = state.clone();
            std::thread::spawn(move || {
                let mut state = state.lock().unwrap();

                f(&mut state.data);

                if let Some(waker) = state.waker.take() {
                    waker.wake();
                }
            });
        }

        Self { state }
    }
}

impl<R> std::future::Future for ThreadFuture<R> {
    type Output = R;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.state.lock().unwrap();

        if state.data.is_some() {
            Poll::Ready(state.data.take().unwrap())
        } else {
            state.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}
