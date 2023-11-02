//! Internal utilities
use std::io::{Error, Result};

/// Turns standard `<errno.h>` status codes - typically rewrapped by Android's [`Errors.h`] - into
/// Rust's [`std::io::Error`].
///
/// [`Errors.h`]: https://cs.android.com/android/platform/superproject/+/master:system/core/libutils/include/utils/Errors.h
pub(crate) fn status_to_io_result<T>(status: i32, value: T) -> Result<T> {
    match status {
        0 => Ok(value),
        r if r < 0 => Err(Error::from_raw_os_error(-r)),
        r => unreachable!("Status is positive integer {}", r),
    }
}
