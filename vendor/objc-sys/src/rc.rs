//! ARC functions.
//!
//! Is available in Clang's [documentation][ARC] so these are safe to rely on.
//!
//! All available since macOS `10.7`.
//!
//! Defined in:
//! - Apple: `objc-internal.h`
//! - GNUStep: `objc-arc.h`
//! - ObjFW: `runtime/arc.m`
//!
//! [ARC]: https://clang.llvm.org/docs/AutomaticReferenceCounting.html#runtime-support>
use core::ffi::c_void;

use crate::objc_object;

// All of these very rarely unwind, but may if the user defined methods
// `retain`, `release`, `autorelease` or `dealloc` do.
extern_c_unwind! {
    // Autoreleasepool
    // ObjFW: Defined in `autorelease.h`, not available with libobjfw-rt!

    #[cfg(any(doc, not(objfw)))]
    pub fn objc_autoreleasePoolPop(pool: *mut c_void);
    #[cfg(any(doc, not(objfw)))]
    pub fn objc_autoreleasePoolPush() -> *mut c_void;

    // Autorelease

    pub fn objc_autorelease(value: *mut objc_object) -> *mut objc_object;
    pub fn objc_autoreleaseReturnValue(value: *mut objc_object) -> *mut objc_object;

    // Weak pointers

    pub fn objc_copyWeak(to: *mut *mut objc_object, from: *mut *mut objc_object);
    pub fn objc_destroyWeak(addr: *mut *mut objc_object);
    pub fn objc_initWeak(addr: *mut *mut objc_object, value: *mut objc_object) -> *mut objc_object;
    // Defined in runtime.h
    pub fn objc_loadWeak(addr: *mut *mut objc_object) -> *mut objc_object;
    pub fn objc_loadWeakRetained(addr: *mut *mut objc_object) -> *mut objc_object;
    pub fn objc_moveWeak(to: *mut *mut objc_object, from: *mut *mut objc_object);

    // Retain / release

    pub fn objc_release(value: *mut objc_object);
    pub fn objc_retain(value: *mut objc_object) -> *mut objc_object;
    pub fn objc_retainAutorelease(value: *mut objc_object) -> *mut objc_object;
    pub fn objc_retainAutoreleaseReturnValue(value: *mut objc_object) -> *mut objc_object;
    pub fn objc_retainAutoreleasedReturnValue(value: *mut objc_object) -> *mut objc_object;
    // Defined in objc-abi.h
    pub fn objc_retainBlock(value: *mut objc_object) -> *mut objc_object;

    // Storing values

    pub fn objc_storeStrong(addr: *mut *mut objc_object, value: *mut objc_object);
    // Defined in runtime.h
    pub fn objc_storeWeak(addr: *mut *mut objc_object, value: *mut objc_object)
        -> *mut objc_object;

    // TODO: Decide about nonstandard extensions like these:
    // #[cfg(any(doc, gnustep))]
    // pub fn objc_delete_weak_refs(obj: *mut objc_object) -> BOOL;
}
