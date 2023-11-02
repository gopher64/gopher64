//! Various common #defines and enum constants.

#[cfg(any(doc, apple))]
use std::os::raw::c_int;

use crate::{id, objc_class, BOOL};

/// The equivalent of `true` for Objective-C's [`BOOL`][`super::BOOL`] type.
#[allow(clippy::unnecessary_cast)]
pub const YES: BOOL = true as BOOL; // true -> 1

/// The equivalent of `false` for Objective-C's [`BOOL`][`super::BOOL`] type.
#[allow(clippy::unnecessary_cast)]
pub const NO: BOOL = false as BOOL; // false -> 0

/// A quick alias for a [`null_mut`][`core::ptr::null_mut`] object / instance.
pub const nil: id = 0 as *mut _;

/// A quick alias for a [`null_mut`][`core::ptr::null_mut`] class.
pub const Nil: *mut objc_class = 0 as *mut _;

pub type objc_AssociationPolicy = usize;
pub const OBJC_ASSOCIATION_ASSIGN: objc_AssociationPolicy = 0;
pub const OBJC_ASSOCIATION_RETAIN_NONATOMIC: objc_AssociationPolicy = 1;
pub const OBJC_ASSOCIATION_COPY_NONATOMIC: objc_AssociationPolicy = 3;
pub const OBJC_ASSOCIATION_RETAIN: objc_AssociationPolicy = 769;
pub const OBJC_ASSOCIATION_COPY: objc_AssociationPolicy = 771;

#[cfg(any(doc, apple))]
pub const OBJC_SYNC_SUCCESS: c_int = 0;
#[cfg(any(doc, apple))]
pub const OBJC_SYNC_NOT_OWNING_THREAD_ERROR: c_int = -1;
/// Only relevant before macOS 10.13
#[cfg(any(doc, apple))]
pub const OBJC_SYNC_TIMED_OUT: c_int = -2;
/// Only relevant before macOS 10.13
#[cfg(any(doc, apple))]
pub const OBJC_SYNC_NOT_INITIALIZED: c_int = -3;
