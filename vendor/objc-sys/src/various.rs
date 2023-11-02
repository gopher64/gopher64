use core::ffi::c_void;
#[cfg(any(doc, not(objfw)))]
use std::os::raw::c_char;
use std::os::raw::c_int;
#[cfg(any(doc, apple))]
use std::os::raw::c_uint;

#[cfg(any(doc, not(objfw)))]
use crate::{objc_AssociationPolicy, BOOL};
use crate::{objc_object, OpaqueData};

/// An opaque type that represents an instance variable.
#[repr(C)]
pub struct objc_ivar {
    _priv: [u8; 0],
    _p: OpaqueData,
}

#[cfg(not(feature = "unstable-c-unwind"))]
type InnerImp = unsafe extern "C" fn();
#[cfg(feature = "unstable-c-unwind")]
type InnerImp = unsafe extern "C-unwind" fn();

/// A nullable pointer to the start of a method implementation.
///
/// Not all APIs are guaranteed to take NULL values; read the docs!
pub type IMP = Option<InnerImp>;

// /// Remember that this is non-null!
// #[cfg(any(doc, apple_new))]
// pub type objc_hook_getClass =
//     unsafe extern "C" fn(name: *const c_char, out_cls: *mut *const crate::objc_class) -> BOOL;
//
// /// Remember that this is non-null!
// #[cfg(any(doc, apple_new))]
// pub type objc_hook_lazyClassNamer =
//     unsafe extern "C" fn(cls: *const crate::objc_class) -> *const c_char;

extern_c_unwind! {
    // Instead of being able to change this, it's a weak symbol on GNUStep.
    #[cfg(any(doc, apple, objfw))]
    pub fn objc_enumerationMutation(obj: *mut objc_object);
}

extern_c! {
    #[cfg(any(doc, not(objfw)))]
    pub fn imp_getBlock(imp: IMP) -> *mut objc_object;
    #[cfg(any(doc, not(objfw)))]
    pub fn imp_implementationWithBlock(block: *mut objc_object) -> IMP;
    #[cfg(any(doc, not(objfw)))]
    pub fn imp_removeBlock(imp: IMP) -> BOOL;

    #[cfg(any(doc, not(objfw)))]
    pub fn ivar_getName(ivar: *const objc_ivar) -> *const c_char;
    #[cfg(any(doc, not(objfw)))]
    pub fn ivar_getOffset(ivar: *const objc_ivar) -> isize;
    #[cfg(any(doc, not(objfw)))]
    pub fn ivar_getTypeEncoding(ivar: *const objc_ivar) -> *const c_char;

    #[cfg(any(doc, apple))]
    pub fn objc_copyClassNamesForImage(
        image: *const c_char,
        out_len: *mut c_uint,
    ) -> *mut *const c_char;
    #[cfg(any(doc, apple))]
    pub fn objc_copyImageNames(out_len: *mut c_uint) -> *mut *const c_char;

    #[cfg(any(doc, apple, objfw))]
    pub fn objc_setEnumerationMutationHandler(
        handler: Option<unsafe extern "C" fn(obj: *mut objc_object)>,
    );

    #[cfg(any(doc, not(objfw)))]
    pub fn objc_getAssociatedObject(
        object: *const objc_object,
        key: *const c_void,
    ) -> *const objc_object;
    #[cfg(any(doc, not(objfw)))]
    pub fn objc_setAssociatedObject(
        object: *mut objc_object,
        key: *const c_void,
        value: *mut objc_object,
        policy: objc_AssociationPolicy,
    );
    #[cfg(any(doc, not(objfw)))]
    pub fn objc_removeAssociatedObjects(object: *mut objc_object);

    #[cfg(any(doc, apple, objfw))]
    pub fn objc_setForwardHandler(fwd: *mut c_void, fwd_stret: *mut c_void);
    // These two are defined in:
    // - Apple: objc-sync.h
    // - GNUStep: dtable.h / associate.m
    // - ObjFW: ObjFW-RT.h
    pub fn objc_sync_enter(obj: *mut objc_object) -> c_int;
    pub fn objc_sync_exit(obj: *mut objc_object) -> c_int;

    // Available in macOS 10.14.4
    // /// Remember that this is non-null!
    // #[cfg(any(doc, apple_new))]
    // pub fn objc_setHook_getClass(
    //     new_value: objc_hook_getClass,
    //     out_old_value: *mut objc_hook_getClass,
    // );
    // Available in macOS 11
    // /// Remember that this is non-null!
    // #[cfg(any(doc, apple_new))]
    // pub fn objc_setHook_lazyClassNamer(
    //     new_value: objc_hook_lazyClassNamer,
    //     out_old_value: *mut objc_hook_lazyClassNamer,
    // );

    // #[deprecated = "not recommended"]
    // #[cfg(any(doc, apple))]
    // pub fn _objc_flush_caches

    // #[cfg(any(doc, gnustep))]
    // objc_test_capability
}
