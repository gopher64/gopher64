//! Defined in:
//! Apple: `objc-exception.h`
//! GNUStep: `eh_personality.c`, which is a bit brittle to rely on, but I
//!   think it's fine...
use core::ffi::c_void;
#[cfg(any(doc, apple_new))]
use std::os::raw::c_int;
#[cfg(feature = "unstable-exception")]
use std::os::raw::c_uchar;

#[cfg(any(doc, apple_new))]
use crate::objc_class;
use crate::objc_object;

/// Remember that this is non-null!
#[cfg(any(doc, apple_new))]
pub type objc_exception_matcher =
    unsafe extern "C" fn(catch_type: *mut objc_class, exception: *mut objc_object) -> c_int;

/// Remember that this is non-null!
#[cfg(any(doc, apple_new))]
pub type objc_exception_preprocessor =
    unsafe extern "C" fn(exception: *mut objc_object) -> *mut objc_object;

/// Remember that this is non-null!
#[cfg(any(doc, apple_new))]
pub type objc_uncaught_exception_handler = unsafe extern "C" fn(exception: *mut objc_object);

#[cfg(objfw)]
pub type objc_uncaught_exception_handler =
    Option<unsafe extern "C" fn(exception: *mut objc_object)>;

/// Remember that this is non-null!
#[cfg(any(doc, all(apple_new, target_os = "macos")))]
pub type objc_exception_handler =
    unsafe extern "C" fn(unused: *mut objc_object, context: *mut c_void);

#[cfg(all(feature = "unstable-exception", not(feature = "unstable-c-unwind")))]
type TryCatchClosure = extern "C" fn(*mut c_void);
#[cfg(all(feature = "unstable-exception", feature = "unstable-c-unwind"))]
type TryCatchClosure = extern "C-unwind" fn(*mut c_void);

extern_c_unwind! {
    /// See [`objc-exception.h`].
    ///
    /// [`objc-exception.h`]: https://github.com/apple-oss-distributions/objc4/blob/objc4-818.2/runtime/objc-exception.h
    pub fn objc_exception_throw(exception: *mut objc_object) -> !;

    #[cfg(apple_new)]
    pub fn objc_exception_rethrow() -> !;

    #[cfg(gnustep)]
    pub fn objc_exception_rethrow(exc_buf: *mut c_void) -> !;
}

extern_c! {
    #[cfg(any(doc, gnustep, apple_new))]
    pub fn objc_begin_catch(exc_buf: *mut c_void) -> *mut objc_object;
    #[cfg(any(doc, gnustep, apple_new))]
    pub fn objc_end_catch();

    #[cfg(any(doc, apple_old))]
    pub fn objc_exception_try_enter(exception_data: *const c_void);

    #[cfg(any(doc, apple_old))]
    pub fn objc_exception_try_exit(exception_data: *const c_void);

    // objc_exception_extract
    // objc_exception_match
    // objc_exception_get_functions
    // objc_exception_set_functions

    #[cfg(any(doc, apple_new))]
    pub fn objc_setExceptionMatcher(f: objc_exception_matcher) -> objc_exception_matcher;
    #[cfg(any(doc, apple_new))]
    pub fn objc_setExceptionPreprocessor(
        f: objc_exception_preprocessor,
    ) -> objc_exception_preprocessor;
    #[cfg(any(doc, apple_new, objfw))]
    pub fn objc_setUncaughtExceptionHandler(
        f: objc_uncaught_exception_handler,
    ) -> objc_uncaught_exception_handler;

    #[cfg(any(doc, all(apple_new, target_os = "macos")))]
    pub fn objc_addExceptionHandler(f: objc_exception_handler, context: *mut c_void) -> usize;
    #[cfg(any(doc, all(apple_new, target_os = "macos")))]
    pub fn objc_removeExceptionHandler(token: usize);

    // Only available when ENABLE_OBJCXX is set, and a useable C++ runtime is
    // present when building libobjc2.
    //
    // #[cfg(any(doc, gnustep))]
    // pub fn objc_set_apple_compatible_objcxx_exceptions(newValue: c_int) -> c_int;

    /// Call the given function inside an Objective-C `@try/@catch` block.
    ///
    /// Defined in `extern/exception.m` and compiled in `build.rs`.
    ///
    /// Alternatively, we could manually write assembly for this function like
    /// [`objrs` does][manual-asm] does, that would cut down on a build stage
    /// (and would probably give us a bit better performance), but it gets
    /// unwieldy _very_ quickly, so I chose the much more stable option.
    ///
    /// Another thing to remember: While Rust's and Objective-C's unwinding
    /// mechanisms are similar now, Rust's is explicitly unspecified, and they
    /// may diverge significantly in the future; so handling this in pure Rust
    /// (using mechanisms like core::intrinsics::r#try) is not an option!
    ///
    /// [manual-asm]: https://gitlab.com/objrs/objrs/-/blob/b4f6598696b3fa622e6fddce7aff281770b0a8c2/src/exception.rs
    #[cfg(feature = "unstable-exception")]
    pub fn rust_objc_sys_0_2_try_catch_exception(
        f: TryCatchClosure,
        context: *mut c_void,
        error: *mut *mut objc_object,
    ) -> c_uchar;
}
