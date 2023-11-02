//! The bindings are pre-generated and the right one for the platform is selected at compile time.

// Bindgen lints
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(improper_ctypes)]
#![allow(clippy::all)]
// Temporarily allow UB nullptr dereference in bindgen layout tests until fixed upstream:
// https://github.com/rust-lang/rust-bindgen/pull/2055
// https://github.com/rust-lang/rust-bindgen/pull/2064
#![allow(deref_nullptr)]
#![allow(dead_code)]

use jni_sys::*;
use libc::{pthread_cond_t, pthread_mutex_t, pthread_t, size_t};
use ndk_sys::{AAssetManager, AConfiguration, ALooper, ALooper_callbackFunc, ANativeWindow, ARect};

#[cfg(all(
    any(target_os = "android", feature = "test"),
    any(target_arch = "arm", target_arch = "armv7")
))]
include!("ffi_arm.rs");

#[cfg(all(any(target_os = "android", feature = "test"), target_arch = "aarch64"))]
include!("ffi_aarch64.rs");

#[cfg(all(any(target_os = "android", feature = "test"), target_arch = "x86"))]
include!("ffi_i686.rs");

#[cfg(all(any(target_os = "android", feature = "test"), target_arch = "x86_64"))]
include!("ffi_x86_64.rs");
