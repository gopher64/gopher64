//! Objective-C type aliases.

use crate::{objc_object, objc_selector};

/// The BOOL typedef for Apple's objc4.
///
/// Don't be fooled by the backup definition in `objc.h`; __OBJC_BOOL_IS_BOOL
/// is always defined by `clang` when compiling Objective-C sources. The below
/// cfgs are determined experimentally via. cross compiling.
#[cfg(apple)]
mod inner {
    // __OBJC_BOOL_IS_BOOL
    #[cfg(any(
        // aarch64-apple-*
        target_arch = "aarch64",
        // + x86_64-apple-ios (but not x86_64-apple-ios-macabi)
        all(target_os = "ios", target_pointer_width = "64", not(target_abi_macabi)),
        // + x86_64-apple-tvos
        all(target_os = "tvos", target_pointer_width = "64"),
        // + *-apple-watchos (no Rust targets with this yet)
        target_os = "watchos",
    ))]
    // C: _Bool
    pub(crate) type BOOL = bool;

    // Inverse of the above
    #[cfg(not(any(
        target_arch = "aarch64",
        all(target_os = "ios", target_pointer_width = "64", not(target_abi_macabi)),
        all(target_os = "tvos", target_pointer_width = "64"),
        target_os = "watchos",
    )))]
    // C: (explicitly) signed char
    pub(crate) type BOOL = i8;
}

// GNUStep's and Microsoft's libobjc2
#[cfg(all(gnustep, libobjc2_strict_apple_compat))]
mod inner {
    // C: (explicitly) signed char
    pub(crate) type BOOL = i8;
}

#[cfg(all(gnustep, not(libobjc2_strict_apple_compat)))]
mod inner {
    // windows && !32bit-MinGW
    #[cfg(all(windows, not(all(target_pointer_width = "64", target_env = "gnu"))))]
    pub(crate) type BOOL = std::os::raw::c_int;

    // The inverse
    #[cfg(not(all(windows, not(all(target_pointer_width = "64", target_env = "gnu")))))]
    // C: unsigned char
    pub(crate) type BOOL = u8;
}

// ObjFW
#[cfg(objfw)]
mod inner {
    // Defined in ObjFW-RT.h
    // C: signed char
    // This has changed since v0.90, but we don't support that yet.
    pub(crate) type BOOL = i8;

    // Note that ObjFW uses `bool` in return types, but that doesn't change
    // the ABI, so we'll just use `BOOL` there for ease of use.
}

/// The Objective-C `BOOL` type.
///
/// The type of this varies across platforms, so to convert an it into a Rust
/// [`bool`], compare it with [`NO`][crate::NO].
///
/// Note that this does _not_ implement `objc2::Encode` on all platforms! You
/// should only use this on FFI boundaries, otherwise prefer
/// `objc2::runtime::Bool`.
///
/// See also the [corresponding documentation entry][docs].
///
/// [docs]: https://developer.apple.com/documentation/objectivec/bool?language=objc
pub type BOOL = inner::BOOL;

// # Why isize/usize is correct for NSInteger/NSUInteger
//
// ## Apple
// The documentation clearly states:
//
// > When building 32-bit applications, NSInteger is a 32-bit integer. A
//   64-bit application treats NSInteger as a 64-bit integer.
//
// And the header file defines them like so:
//
//     #if __LP64__ || TARGET_OS_WIN32 || NS_BUILD_32_LIKE_64
//     typedef long NSInteger;
//     typedef unsigned long NSUInteger;
//     #else
//     typedef int NSInteger;
//     typedef unsigned int NSUInteger;
//     #endif
//
// Rust (or at least `libc`) has no targets where c_int/c_uint are not 32-bit,
// so that part is correct. By manual inspection it is found that the only
// platform where c_long/c_ulong differs from isize/usize is on Windows.
// However Apple's libraries are only designed to work on 32-bit Windows, so
// this case should be fine as well.
//
// Likewise for NSUInteger.
//
// ## GNUStep / WinObjC
//
// Defined as intptr_t/uintptr_t, which is exactly the same as isize/usize.
//
// ## ObjFW
//
// Doesn't define these, but e.g. `OFString -length` returns size_t, so our
// definitions are should be correct on effectively all targets.
//
// Things might change slightly in the future, see
// <https://internals.rust-lang.org/t/pre-rfc-usize-is-not-size-t/15369>.

/// A signed integer value type.
///
/// This is guaranteed to always be a type-alias to [`isize`]. That means it
/// is valid to use `#[repr(isize)]` on enums and structs with size
/// `NSInteger`.
///
/// See also the [corresponding documentation entry][docs].
///
/// [docs]: https://developer.apple.com/documentation/objectivec/nsinteger?language=objc
///
/// # Examples
///
/// ```
/// #[repr(isize)] // NSInteger
/// pub enum NSComparisonResult {
///     NSOrderedAscending = -1,
///     NSOrderedSame = 0,
///     NSOrderedDescending = 1,
/// }
/// ```
pub type NSInteger = isize;

/// Describes an unsigned integer.
///
/// This is guaranteed to always be a type-alias to [`usize`]. That means it
/// is valid to use `#[repr(usize)]` on enums and structs with size
/// `NSUInteger`.
///
/// See also the [corresponding documentation entry][docs].
///
/// [docs]: https://developer.apple.com/documentation/objectivec/nsuinteger?language=objc
///
/// # Examples
///
/// ```
/// use objc_sys::NSUInteger;
/// // Or:
/// // use objc2::ffi::NSUInteger;
/// // use objc2::foundation::NSUInteger;
/// extern "C" {
///     fn some_external_function() -> NSUInteger;
/// }
/// ```
///
/// ```
/// #[repr(usize)] // NSUInteger
/// enum NSRoundingMode {
///     NSRoundPlain = 0,
///     NSRoundDown = 1,
///     NSRoundUp = 2,
///     NSRoundBankers = 3,
/// };
/// ```
pub type NSUInteger = usize;

/// The maximum value for an NSInteger.
pub const NSIntegerMax: NSInteger = NSInteger::MAX;

/// The minimum value for an NSInteger.
pub const NSIntegerMin: NSInteger = NSInteger::MIN;

/// The maximum value for an NSUInteger.
pub const NSUIntegerMax: NSUInteger = NSUInteger::MAX;

/// An immutable pointer to a selector.
///
/// Type alias provided for convenience. See `objc2::runtime::Sel` for a
/// higher level binding.
pub type SEL = *const objc_selector;

/// A mutable pointer to an object / instance.
///
/// Type alias provided for convenience. See `objc2::runtime::Object` for a
/// higher level binding, and `objc2::rc::Id` for an easier way of handling
/// objects.
pub type id = *mut objc_object;
