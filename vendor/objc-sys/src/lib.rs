//! # Raw bindings to Objective-C runtimes
//!
//! These bindings contain almost no documentation, so it is highly
//! recommended to read the documentation of the original libraries:
//! - Apple's [official documentation][apple].
//! - Apple's `objc4` [source code][objc4], in particular `runtime.h`.
//! - GNUStep's `libobjc2` [source code][libobjc2], in particular `runtime.h`.
//!
//! See also the [`README.md`](https://crates.io/crates/objc-sys) for more
//! background information, and for how to configure the desired runtime.
//!
//! [apple]: https://developer.apple.com/documentation/objectivec/objective-c_runtime?language=objc
//! [libobjc2]: https://github.com/gnustep/libobjc2/tree/v2.1/objc
//! [objc4]: https://github.com/apple-oss-distributions/objc4

#![no_std]
#![warn(elided_lifetimes_in_paths)]
#![deny(non_ascii_idents)]
#![warn(unreachable_pub)]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(clippy::cargo)]
#![warn(clippy::ptr_as_ptr)]
#![allow(clippy::upper_case_acronyms)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![doc(html_root_url = "https://docs.rs/objc-sys/0.2.0-beta.2")]
#![cfg_attr(feature = "unstable-c-unwind", feature(c_unwind))]
#![cfg_attr(feature = "unstable-docsrs", feature(doc_auto_cfg, doc_cfg_hide))]
#![cfg_attr(feature = "unstable-docsrs", doc(cfg_hide(doc)))]

// TODO: Remove this and add "no-std" category to Cargo.toml
// Requires a better solution for C-types in `no_std` crates.
// See https://github.com/japaric/cty/issues/14.
extern crate std;

#[cfg(not(feature = "std"))]
compile_error!("The `std` feature currently must be enabled.");

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
extern "C" {}

use core::cell::UnsafeCell;
use core::marker::{PhantomData, PhantomPinned};

macro_rules! generate_linking_tests {
    {
        extern $abi:literal {$(
            $(#[$m:meta])*
            $v:vis fn $name:ident(
                $($(#[$a_m:meta])* $a:ident: $t:ty),* $(,)?
            ) $(-> $r:ty)?;
        )+}
        mod $test_name:ident;
    } => {
        extern $abi {$(
            $(#[$m])*
            $v fn $name($($(#[$a_m])* $a: $t),*) $(-> $r)?;
        )+}

        #[allow(deprecated)]
        #[cfg(test)]
        mod $test_name {
            #[allow(unused)]
            use super::*;

            $(
                $(#[$m])*
                #[test]
                fn $name() {
                    // Get function pointer to make the linker require the
                    // symbol to be available.
                    let f: unsafe extern $abi fn($($(#[$a_m])* $t),*) $(-> $r)? = crate::$name;
                    // Workaround for https://github.com/rust-lang/rust/pull/92964
                    #[cfg(feature = "unstable-c-unwind")]
                    #[allow(clippy::useless_transmute)]
                    let f: unsafe extern "C" fn() = unsafe { core::mem::transmute(f) };
                    // Execute side-effect to ensure it is not optimized away.
                    std::println!("{:p}", f);
                }
            )+
        }
    };
}

macro_rules! extern_c {
    {
        $(
            $(#[$m:meta])*
            $v:vis fn $name:ident(
                $($(#[$a_m:meta])* $a:ident: $t:ty),* $(,)?
            ) $(-> $r:ty)?;
        )+
    } => {
        generate_linking_tests! {
            extern "C" {$(
                $(#[$m])*
                $v fn $name($($(#[$a_m])* $a: $t),*) $(-> $r)?;
            )+}
            mod test_linkable;
        }
    };
}

// A lot of places may call `+initialize`, but the runtime guards those calls
// with `@try/@catch` blocks already, so we don't need to mark every function
// "C-unwind", only certain ones!
macro_rules! extern_c_unwind {
    {
        $(
            $(#[$m:meta])*
            $v:vis fn $name:ident(
                $($(#[$a_m:meta])* $a:ident: $t:ty),* $(,)?
            ) $(-> $r:ty)?;
        )+
    } => {
        #[cfg(not(feature = "unstable-c-unwind"))]
        generate_linking_tests! {
            extern "C" {$(
                $(#[$m])*
                $v fn $name($($(#[$a_m])* $a: $t),*) $(-> $r)?;
            )+}
            mod test_linkable_unwind;
        }

        #[cfg(feature = "unstable-c-unwind")]
        generate_linking_tests! {
            extern "C-unwind" {$(
                $(#[$m])*
                $v fn $name($($(#[$a_m])* $a: $t),*) $(-> $r)?;
            )+}
            mod test_linkable_unwind;
        }
    };
}

mod class;
mod constants;

mod exception;
mod image_info;
mod message;
mod method;
mod object;
mod property;
mod protocol;
mod rc;
mod selector;
mod types;
mod various;

pub use class::*;
pub use constants::*;
pub use exception::*;
pub use image_info::*;
pub use message::*;
pub use method::*;
pub use object::*;
pub use property::*;
pub use protocol::*;
pub use rc::*;
pub use selector::*;
pub use types::*;
pub use various::*;

/// We don't know much about the actual structs, so better mark them `!Send`,
/// `!Sync`, `!UnwindSafe`, `!RefUnwindSafe`, `!Unpin` and as mutable behind
/// shared references.
///
/// Downstream libraries can always manually opt in to these types afterwards.
/// (It's also less of a breaking change on our part if we re-add these).
///
/// TODO: Replace this with `extern type` to also mark it as `!Sized`.
type OpaqueData = UnsafeCell<PhantomData<(*const UnsafeCell<()>, PhantomPinned)>>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn smoke() {
        // Verify that this library links and works fine by itself
        let name = CStr::from_bytes_with_nul(b"abc:def:\0").unwrap();
        let sel = unsafe { sel_registerName(name.as_ptr()) };
        let rtn = unsafe { CStr::from_ptr(sel_getName(sel)) };
        assert_eq!(name, rtn);
    }
}
