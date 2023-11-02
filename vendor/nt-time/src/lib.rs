// SPDX-FileCopyrightText: 2023 Shun Sakai
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! The `nt-time` crate is a [Windows file time][file-time-docs-url] library.
//!
//! This is used as timestamps such as Windows and [7z][7z-format-url].
//!
//! # Examples
//!
//! ```
//! use core::time::Duration;
//!
//! use nt_time::{
//!     time::{macros::datetime, OffsetDateTime},
//!     FileTime,
//! };
//!
//! let ft = FileTime::NT_TIME_EPOCH;
//! assert_eq!(
//!     OffsetDateTime::try_from(ft).unwrap(),
//!     datetime!(1601-01-01 00:00 UTC)
//! );
//!
//! let ft = ft + Duration::from_secs(11_644_473_600);
//! assert_eq!(
//!     OffsetDateTime::try_from(ft).unwrap(),
//!     OffsetDateTime::UNIX_EPOCH
//! );
//! assert_eq!(ft.to_raw(), 116_444_736_000_000_000);
//!
//! assert_eq!(FileTime::new(u64::MAX), FileTime::MAX);
//! ```
//!
//! [file-time-docs-url]: https://docs.microsoft.com/en-us/windows/win32/sysinfo/file-times
//! [7z-format-url]: https://www.7-zip.org/7z.html

#![doc(html_root_url = "https://docs.rs/nt-time/0.5.3/")]
#![no_std]
#![cfg_attr(doc_cfg, feature(doc_auto_cfg, doc_cfg))]
// Lint levels of rustc.
#![forbid(unsafe_code)]
#![deny(missing_debug_implementations, missing_docs)]
#![warn(rust_2018_idioms)]
// Lint levels of Clippy.
#![warn(clippy::cargo, clippy::nursery, clippy::pedantic)]

#[cfg(test)]
#[macro_use]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

pub mod error;
mod file_time;
#[cfg(feature = "serde")]
pub mod serde_with;

#[cfg(feature = "chrono")]
pub use chrono;
#[cfg(feature = "serde")]
pub use serde;
pub use time;

pub use crate::file_time::FileTime;
