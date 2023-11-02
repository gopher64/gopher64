//! An executor where you can only push a limited number of tasks.

use async_executor::{Executor, Task};
use event_listener::{Event, EventListener};
use futures_lite::pin;
use std::{
    future::Future,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

/// An executor where you can only push a limited number of tasks.
struct LimitedExecutor {
    /// Inner running executor.
    executor: Executor<'static>,

    /// Shared state.
    shared: Arc<SharedState>,
}

struct SharedState {
    /// The maximum number of tasks that can be pushed.
    max: usize,

    /// The current number of active tasks.
    active: AtomicUsize,

    /// Event listeners for when a new task is available.
    slot_available: Event,
}

impl LimitedExecutor {
    fn new(max: usize) -> Self {
        Self {
            executor: Executor::new(),
            shared: Arc::new(SharedState {
                max,
                active: AtomicUsize::new(0),
                slot_available: Event::new(),
            }),
        }
    }

    /// Spawn a task, waiting until there is a slot available.
    async fn spawn<F: Future + Send + 'static>(&self, future: F) -> Task<F::Output>
    where
        F::Output: Send + 'static,
    {
        let listener = EventListener::new(&self.shared.slot_available);
        pin!(listener);

        // Load the current number of active tasks.
        let mut active = self.shared.active.load(Ordering::Acquire);

        loop {
            // Check if there is a slot available.
            if active < self.shared.max {
                // Try to set the slot to what would be the new number of tasks.
                let new_active = active + 1;
                match self.shared.active.compare_exchange(
                    active,
                    new_active,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                ) {
                    Ok(_) => {
                        // Wrap the future in another future that decrements the active count
                        // when it's done.
                        let future = {
                            let shared = self.shared.clone();
                            async move {
                                struct DecOnDrop(Arc<SharedState>);

                                impl Drop for DecOnDrop {
                                    fn drop(&mut self) {
                                        // Decrement the count and notify someone.
                                        self.0.active.fetch_sub(1, Ordering::SeqCst);
                                        self.0.slot_available.notify(usize::MAX);
                                    }
                                }

                                let _dec = DecOnDrop(shared);
                                future.await
                            }
                        };

                        // Wake up another waiter, in case there is one.
                        self.shared.slot_available.notify(1);

                        // Spawn the task.
                        return self.executor.spawn(future);
                    }

                    Err(actual) => {
                        // Try again.
                        active = actual;
                    }
                }
            } else {
                // Start waiting for a slot to become available.
                if listener.as_ref().is_listening() {
                    listener.as_mut().await;
                } else {
                    listener.as_mut().listen();
                }

                active = self.shared.active.load(Ordering::Acquire);
            }
        }
    }

    /// Run a future to completion.
    async fn run<F: Future>(&self, future: F) -> F::Output {
        self.executor.run(future).await
    }
}

fn main() {
    futures_lite::future::block_on(async {
        let ex = Arc::new(LimitedExecutor::new(10));
        ex.run({
            let ex = ex.clone();
            async move {
                // Spawn a bunch of tasks that wait for a while.
                for i in 0..15 {
                    ex.spawn(async move {
                        async_io::Timer::after(Duration::from_millis(fastrand::u64(1..3))).await;
                        println!("Waiting task #{i} finished!");
                    })
                    .await
                    .detach();
                }

                let (start_tx, start_rx) = async_channel::bounded::<()>(1);
                let mut current_rx = start_rx;

                // Send the first message.
                start_tx.send(()).await.unwrap();

                // Spawn a bunch of channel tasks that wake eachother up.
                for i in 0..25 {
                    let (next_tx, next_rx) = async_channel::bounded::<()>(1);

                    ex.spawn(async move {
                        current_rx.recv().await.unwrap();
                        println!("Channel task {i} woken up!");
                        next_tx.send(()).await.unwrap();
                        println!("Channel task {i} finished!");
                    })
                    .await
                    .detach();

                    current_rx = next_rx;
                }

                // Wait for the last task to finish.
                current_rx.recv().await.unwrap();

                println!("All tasks finished!");
            }
        })
        .await;
    });
}
