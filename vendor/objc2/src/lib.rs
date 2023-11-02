//! # Objective-C interface and runtime bindings
//!
//! Objective-C is[^note] the standard programming language on Apple platforms
//! like macOS, iOS, iPadOS, tvOS and watchOS. It is an object-oriented
//! language centered around "sending messages" to its instances - this can
//! for the most part be viewed as a simple method call.
//!
//! Most of the core libraries and frameworks that are in use on Apple systems
//! are written in Objective-C, and hence we would like the ability to
//! interract with these using Rust; this crate enables you to do that, in
//! as safe a manner as possible.
//!
//! [^note]: Yes, I know, "was", Swift now exists. All the existing frameworks
//!   are written in Objective-C though, so the point still holds.
//!
//!
//! ## Basic usage
//!
//! This example illustrates major parts of the functionality in this crate:
//!
//! First, we get a reference to the `NSObject`'s [`runtime::Class`] using the
//! [`class!`] macro.
//! Next, we creates a new [`runtime::Object`] pointer, and ensure it is
//! deallocated after we've used it by putting it into an [`rc::Owned`]
//! [`rc::Id`].
//! Now we're free to send messages to the object to our hearts desire using
//! the [`msg_send!`] or [`msg_send_id!`] macros (depending on the return type
//! of the method).
//! Finally, the `Id<Object, _>` goes out of scope, and the object is released
//! and deallocated.
//!
#![cfg_attr(feature = "apple", doc = "```")]
#![cfg_attr(not(feature = "apple"), doc = "```no_run")]
//! use objc2::{class, msg_send, msg_send_id};
//! use objc2::ffi::NSUInteger;
//! use objc2::rc::{Id, Owned, Shared};
//! use objc2::runtime::Object;
//!
//! let cls = class!(NSObject);
//!
//! // Creation
//!
//! let obj1: Id<Object, Owned> = unsafe { msg_send_id![cls, new] };
//! let obj2: Id<Object, Owned> = unsafe {
//!     // Equivalent to using `new`
//!     msg_send_id![msg_send_id![cls, alloc], init]
//! };
//!
//! // Usage
//!
//! let hash1: NSUInteger = unsafe { msg_send![&obj1, hash] };
//! let hash2: NSUInteger = unsafe { msg_send![&obj2, hash] };
//! assert_ne!(hash1, hash2);
//!
//! let is_kind: bool = unsafe { msg_send![&obj1, isKindOfClass: cls] };
//! assert!(is_kind);
//!
//! // We're going to create a new reference to the first object, so
//! // relinquish mutable ownership.
//! let obj1: Id<Object, Shared> = obj1.into();
//! let obj1_self: Id<Object, Shared> = unsafe { msg_send_id![&obj1, self] };
//! let is_equal: bool = unsafe { msg_send![&obj1, isEqual: &*obj1_self] };
//! assert!(is_equal);
//!
//! // Deallocation on drop
//! ```
//!
//! Note that this very simple example contains **a lot** of `unsafe` (which
//! should all ideally be justified with a `// SAFETY` comment). This is
//! required because our compiler can verify very little about the Objective-C
//! invocation, including all argument and return types used in [`msg_send!`];
//! we could have just as easily accidentally made `hash` an `f32`, or any
//! other type, and this would trigger undefined behaviour!
//!
//! Making the ergonomics better is something that is currently being worked
//! on, the [`foundation`] module contains more ergonomic usage of at
//! least parts of the `Foundation` framework.
//!
//! Anyhow, all of this `unsafe` nicely leads us to another feature that this
//! crate has:
//!
//! [`runtime::Class`]: crate::runtime::Class
//! [`runtime::Object`]: crate::runtime::Object
//! [`rc::Owned`]: crate::rc::Owned
//! [`rc::Id`]: crate::rc::Id
//! [`foundation`]: crate::foundation
//!
//!
//! ## Encodings and message type verification
//!
//! The Objective-C runtime includes encodings for each method that describe
//! the argument and return types. See the [`objc2-encode`] crate for the
//! full overview of what this is (its types are re-exported in this crate).
//!
//! The important part is: To make message sending safer, all arguments and
//! return values for messages must implement [`Encode`]. This allows the Rust
//! compiler to prevent you from passing e.g. a [`Box`] into Objective-C,
//! which would both be UB and leak the box.
//!
//! Furthermore, we can take advantage of the encodings provided by the
//! runtime to verify that the types used in Rust actually match the types
//! encoded for the method. This is not a perfect solution for ensuring safety
//! (some Rust types have the same Objective-C encoding, but are not
//! equivalent), but it gets us much closer to it!
//!
//! To use this functionality, enable the `"verify_message"` cargo feature
//! while debugging. With this feature enabled, encodings are checked every
//! time you send a message, and the message send will panic if they are not
//! equivalent.
//!
//! To take the example above, if we changed the `hash` method's return type
//! as in the following example, it panics when the feature is enabled:
//!
#![cfg_attr(
    all(feature = "apple", feature = "verify_message"),
    doc = "```should_panic"
)]
#![cfg_attr(
    not(all(feature = "apple", feature = "verify_message")),
    doc = "```no_run"
)]
//! # use objc2::{class, msg_send, msg_send_id};
//! # use objc2::rc::{Id, Owned};
//! # use objc2::runtime::Object;
//! #
//! # let cls = class!(NSObject);
//! # let obj1: Id<Object, Owned> = unsafe { msg_send_id![cls, new] };
//! #
//! // Wrong return type - this is UB!
//! let hash1: f32 = unsafe { msg_send![&obj1, hash] };
//! ```
//!
//! [`objc2-encode`]: objc2_encode
//! [`Box`]: std::boxed::Box
//!
//!
//! ## Crate features
//!
//! This crate exports several optional cargo features, see [`Cargo.toml`] for
//! an overview and description of these.
//!
//! [`Cargo.toml`]: https://github.com/madsmtm/objc2/blob/master/objc2/Cargo.toml
//!
//!
//! ## Support for other Operating Systems
//!
//! The bindings can be used on Linux or *BSD utilizing the
//! [GNUstep Objective-C runtime](https://www.github.com/gnustep/libobjc2),
//! see the [`objc-sys`][`objc_sys`] crate for how to configure this.
//!
//!
//! ## Other functionality
//!
//! That was a quick introduction, this library also has [support for handling
//! exceptions][exc], [the ability to dynamically declare Objective-C
//! classes][declare], [advanced reference-counting utilities][rc], and more -
//! peruse the documentation at will!
//!
//! [exc]: crate::exception
//! [declare]: crate::declare
//! [rc]: crate::rc

#![no_std]
#![cfg_attr(
    feature = "unstable-autoreleasesafe",
    feature(negative_impls, auto_traits)
)]
#![cfg_attr(feature = "unstable-c-unwind", feature(c_unwind))]
#![cfg_attr(feature = "unstable-docsrs", feature(doc_auto_cfg))]
#![warn(elided_lifetimes_in_paths)]
#![warn(missing_docs)]
#![deny(non_ascii_idents)]
#![warn(unreachable_pub)]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(clippy::cargo)]
#![warn(clippy::ptr_as_ptr)]
// Update in Cargo.toml as well.
#![doc(html_root_url = "https://docs.rs/objc2/0.3.0-beta.3.patch-leaks.3")]

#[cfg(not(feature = "alloc"))]
compile_error!("The `alloc` feature currently must be enabled.");

#[cfg(not(feature = "std"))]
compile_error!("The `std` feature currently must be enabled.");

extern crate alloc;
extern crate std;

// The example uses NSObject without doing the __gnustep_hack
#[cfg(all(feature = "apple", doctest))]
#[doc = include_str!("../README.md")]
extern "C" {}

pub use objc2_encode as encode;
pub use objc_sys as ffi;

#[doc(no_inline)]
pub use objc2_encode::{Encode, EncodeArguments, Encoding, RefEncode};

pub use crate::class_type::ClassType;
pub use crate::message::{Message, MessageArguments, MessageReceiver};
#[cfg(feature = "malloc")]
pub use crate::verify::VerificationError;

#[cfg(feature = "objc2-proc-macros")]
#[doc(hidden)]
pub use objc2_proc_macros::__hash_idents;

#[cfg(not(feature = "objc2-proc-macros"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __hash_idents {
    // Noop; used to make our other macros a bit easier to read
    ($($x:tt)*) => {$($x)*};
}

#[doc(hidden)]
pub mod __macro_helpers;
mod cache;
mod class_type;
pub mod declare;
pub mod exception;
#[cfg(feature = "foundation")]
pub mod foundation;
mod macros;
mod message;
pub mod rc;
pub mod runtime;
#[cfg(test)]
mod test_utils;
#[cfg(feature = "malloc")]
mod verify;

/// Hacky way to make GNUStep link properly to Foundation while testing.
///
/// This is a temporary solution to make our CI work for now!
#[doc(hidden)]
#[cfg(feature = "gnustep-1-7")]
pub mod __gnustep_hack {
    use super::runtime::Class;

    extern "C" {
        // The linking changed in libobjc2 v2.0
        #[cfg_attr(feature = "gnustep-2-0", link_name = "._OBJC_CLASS_NSObject")]
        #[cfg_attr(not(feature = "gnustep-2-0"), link_name = "_OBJC_CLASS_NSObject")]
        static OBJC_CLASS_NSObject: Class;
        // Others:
        // __objc_class_name_NSObject
        // _OBJC_CLASS_REF_NSObject
    }

    pub unsafe fn get_class_to_force_linkage() -> &'static Class {
        unsafe { core::ptr::read_volatile(&&OBJC_CLASS_NSObject) }
    }

    #[test]
    fn ensure_linkage() {
        unsafe { get_class_to_force_linkage() };
    }
}
