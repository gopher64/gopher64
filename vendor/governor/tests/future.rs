#![cfg(feature = "std")]

use all_asserts::*;
use futures::executor::block_on;
use governor::{Quota, RateLimiter};
use nonzero_ext::*;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// The time that our "real" clock tests may take, indicating that no
/// blocking waits have occurred.
const MAX_TEST_RUN_DURATION: Duration = Duration::from_micros(200);

#[test]
fn pauses() {
    let lim = RateLimiter::direct(Quota::per_second(nonzero!(10u32)));

    // exhaust the limiter:
    loop {
        if lim.check().is_err() {
            break;
        }
    }
    let i = Instant::now();
    block_on(lim.until_ready());
    assert_ge!(i.elapsed(), Duration::from_millis(100));
}

#[test]
fn pauses_n() {
    let lim = RateLimiter::direct(Quota::per_second(nonzero!(10u32)));

    for _ in 0..6 {
        lim.check().unwrap();
    }
    let i = Instant::now();
    block_on(lim.until_n_ready(nonzero!(5u32))).unwrap();
    assert_ge!(i.elapsed(), Duration::from_millis(100));
}

#[test]
fn pauses_keyed() {
    let i = Instant::now();
    let lim = RateLimiter::keyed(Quota::per_second(nonzero!(10u32)));

    // exhaust the limiter:
    loop {
        if lim.check_key(&1u32).is_err() {
            break;
        }
    }

    block_on(lim.until_key_ready(&1u32));
    assert_ge!(i.elapsed(), Duration::from_millis(100));
}

#[test]
fn proceeds() {
    let lim = RateLimiter::direct(Quota::per_second(nonzero!(2u32)));
    let i = Instant::now();
    block_on(lim.until_ready());
    assert_le!(i.elapsed(), MAX_TEST_RUN_DURATION);
}

#[test]
fn proceeds_n() {
    let lim = RateLimiter::direct(Quota::per_second(nonzero!(3u32)));
    let i = Instant::now();
    block_on(lim.until_n_ready(nonzero!(2u32))).unwrap();
    assert_le!(i.elapsed(), MAX_TEST_RUN_DURATION);
}

#[test]
fn proceeds_keyed() {
    let lim = RateLimiter::keyed(Quota::per_second(nonzero!(2u32)));
    let i = Instant::now();
    block_on(lim.until_key_ready(&1u32));
    assert_le!(i.elapsed(), MAX_TEST_RUN_DURATION);
}

#[test]
fn multiple() {
    let lim = Arc::new(RateLimiter::direct(Quota::per_second(nonzero!(10u32))));
    let mut children = vec![];
    let i = Instant::now();
    for _i in 0..20 {
        let lim = Arc::clone(&lim);
        children.push(thread::spawn(move || {
            block_on(lim.until_ready());
        }));
    }
    for child in children {
        child.join().unwrap();
    }
    // by now we've waited for, on average, 10ms; but sometimes the
    // test finishes early; let's assume it takes at least 8ms:
    let elapsed = i.elapsed();
    assert_ge!(elapsed, Duration::from_millis(8),);
}

#[test]
fn multiple_keyed() {
    let lim = Arc::new(RateLimiter::keyed(Quota::per_second(nonzero!(10u32))));
    let mut children = vec![];

    let i = Instant::now();
    for _i in 0..20 {
        let lim = Arc::clone(&lim);
        children.push(thread::spawn(move || {
            block_on(lim.until_key_ready(&1u32));
        }));
    }
    for child in children {
        child.join().unwrap();
    }
    // by now we've waited for, on average, 10ms; but sometimes the
    // test finishes early; let's assume it takes at least 8ms:
    let elapsed = i.elapsed();
    assert_ge!(elapsed, Duration::from_millis(8),);
}

#[test]
fn errors_on_exceeded_capacity() {
    let lim = RateLimiter::direct(Quota::per_second(nonzero!(10u32)));

    block_on(lim.until_n_ready(nonzero!(11u32))).unwrap_err();
}
