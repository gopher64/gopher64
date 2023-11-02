use std::time::{Duration, Instant};

use calloop::{
    timer::{TimeoutAction, Timer},
    EventLoop,
};

fn main() {
    let mut event_loop =
        EventLoop::try_new_high_precision().expect("Failed to initialize the event loop!");

    let before = Instant::now();

    event_loop
        .handle()
        .insert_source(
            Timer::from_duration(Duration::from_micros(20)),
            |_, _, _| TimeoutAction::Drop,
        )
        .unwrap();

    event_loop.dispatch(None, &mut ()).unwrap();

    let elapsed = before.elapsed();

    println!(
        "The event loop slept for {} microseconds.",
        elapsed.as_micros()
    );
}
