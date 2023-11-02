//! Convert closures into wakers.
//!
//! A [`Waker`] is just a fancy callback. This crate converts regular closures into wakers.

#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs, missing_debug_implementations, rust_2018_idioms)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/smol-rs/smol/master/assets/images/logo_fullsize_transparent.png"
)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/smol-rs/smol/master/assets/images/logo_fullsize_transparent.png"
)]

extern crate alloc;

use alloc::sync::Arc;
use alloc::task::Wake;
use core::task::Waker;

/// Converts a closure into a [`Waker`].
///
/// The closure gets called every time the waker is woken.
///
/// # Examples
///
/// ```
/// use waker_fn::waker_fn;
///
/// let waker = waker_fn(|| println!("woken"));
///
/// waker.wake_by_ref(); // Prints "woken".
/// waker.wake();        // Prints "woken".
/// ```
pub fn waker_fn<F: Fn() + Send + Sync + 'static>(f: F) -> Waker {
    Waker::from(Arc::new(Helper(f)))
}

struct Helper<F>(F);

impl<F: Fn() + Send + Sync + 'static> Wake for Helper<F> {
    fn wake(self: Arc<Self>) {
        (self.0)();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        (self.0)();
    }
}
