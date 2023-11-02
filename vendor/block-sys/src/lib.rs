//! # Raw bindings to Apple's C language extension of blocks
//!
//! The documentation for these bindings is a mix from GNUStep's and Apple's
//! sources, but the [ABI specification][ABI] is really the place you should
//! be looking!
//!
//! See also the [`README.md`](https://crates.io/crates/block-sys) for more
//! background information, and for how to configure the desired runtime.
//!
//! [ABI]: https://clang.llvm.org/docs/Block-ABI-Apple.html

// TODO: Replace `extern "C"` with `extern "C-unwind"` where applicable.
// See https://rust-lang.github.io/rfcs/2945-c-unwind-abi.html.

#![no_std]
#![warn(elided_lifetimes_in_paths)]
#![deny(non_ascii_idents)]
#![warn(unreachable_pub)]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(clippy::cargo)]
#![warn(clippy::ptr_as_ptr)]
// Update in Cargo.toml as well.
#![doc(html_root_url = "https://docs.rs/block-sys/0.1.0-beta.1")]
#![cfg_attr(feature = "unstable-docsrs", feature(doc_auto_cfg, doc_cfg_hide))]
#![cfg_attr(feature = "unstable-docsrs", doc(cfg_hide(doc)))]

extern crate std;

#[cfg(not(feature = "std"))]
compile_error!("The `std` feature currently must be enabled.");

// Ensure linkage actually happens
#[cfg(feature = "gnustep-1-7")]
extern crate objc_sys as _;

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
extern "C" {}

use core::cell::UnsafeCell;
use core::ffi::c_void;
use core::marker::{PhantomData, PhantomPinned};
use std::os::raw::{c_char, c_ulong};

#[repr(C)]
pub struct Class {
    #[cfg(any(feature = "apple", feature = "compiler-rt"))]
    _priv: [*mut c_void; 32],

    #[cfg(any(feature = "gnustep-1-7", feature = "objfw"))]
    // The size of this is unknown
    _priv: [u8; 0],

    /// See objc_sys::OpaqueData
    _opaque: UnsafeCell<PhantomData<(*const UnsafeCell<()>, PhantomPinned)>>,
}

/// Block descriptor flags.
/// Values for Block_layout->flags to describe block objects
#[allow(non_camel_case_types)]
pub type block_flags = i32;

#[cfg(any(doc, feature = "apple"))]
pub const BLOCK_DEALLOCATING: block_flags = 0x0001;

pub const BLOCK_REFCOUNT_MASK: block_flags = if cfg!(feature = "gnustep-1-7") {
    // Mask for the reference count in byref structure's flags field. The low
    // 3 bytes are reserved for the reference count, the top byte for the flags.
    0x00ffffff
} else if cfg!(any(feature = "compiler-rt", feature = "objfw")) {
    0xffff
} else if cfg!(feature = "apple") {
    0xfffe // runtime
} else {
    0
};

#[cfg(any(doc, feature = "apple"))]
/// compiler
pub const BLOCK_INLINE_LAYOUT_STRING: block_flags = 1 << 21;

#[cfg(any(doc, feature = "apple"))]
/// compiler
pub const BLOCK_SMALL_DESCRIPTOR: block_flags = 1 << 22;

#[cfg(any(doc, feature = "apple"))] // Part of ABI?
/// compiler
pub const BLOCK_IS_NOESCAPE: block_flags = 1 << 23;

#[cfg(any(doc, feature = "apple"))]
/// runtime
pub const BLOCK_NEEDS_FREE: block_flags = 1 << 24;

/// The block descriptor contains copy and dispose helpers.
/// compiler
pub const BLOCK_HAS_COPY_DISPOSE: block_flags = 1 << 25;

/// The helpers have C++ code.
/// compiler: helpers have C++ code
pub const BLOCK_HAS_CTOR: block_flags = 1 << 26;

#[cfg(any(doc, feature = "apple"))]
/// compiler
pub const BLOCK_IS_GC: block_flags = 1 << 27;

/// Block is stored in global memory and does not need to be copied.
/// compiler
pub const BLOCK_IS_GLOBAL: block_flags = 1 << 28;

/// Block function uses a calling convention that returns a structure via a
/// pointer passed in by the caller.
///
/// match (BLOCK_USE_STRET, BLOCK_HAS_SIGNATURE) {
///     (false, false) => 10.6.ABI, no signature field available
///     (true, false)  => 10.6.ABI, no signature field available
///     (false, true)  => ABI.2010.3.16, regular calling convention, presence of signature field
///     (true, true)   => ABI.2010.3.16, stret calling convention, presence of signature field,
/// }
///
/// See <https://clang.llvm.org/docs/Block-ABI-Apple.html#high-level>
#[doc(alias = "BLOCK_USE_SRET")]
#[doc(alias = "BLOCK_HAS_DESCRIPTOR")] // compiler-rt || macOS 10.6
pub const BLOCK_USE_STRET: block_flags = 1 << 29;

/// Block has an Objective-C type encoding.
/// compiler
pub const BLOCK_HAS_SIGNATURE: block_flags = 1 << 30;

#[cfg(any(doc, feature = "apple"))]
/// compiler
pub const BLOCK_HAS_EXTENDED_LAYOUT: block_flags = 1 << 31;

/// Flags used in the final argument to _Block_object_assign() and
/// _Block_object_dispose().  These indicate the type of copy or dispose to
/// perform.
/// Values for _Block_object_assign() and _Block_object_dispose() parameters
///
/// This is a helper type, in the sources this type does not have a name!
#[allow(non_camel_case_types)]
pub type block_assign_dispose_flags = i32;

/// The value is of some id-like type, and should be copied as an Objective-C
/// object: i.e. by sending -retain or via the GC assign functions in GC mode
/// (not yet supported).
///
/// id, NSObject, __attribute__((NSObject)), block, ...
pub const BLOCK_FIELD_IS_OBJECT: block_assign_dispose_flags = 3;

/// The field is a block.  This must be copied by the block copy functions.
///
/// a block variable
pub const BLOCK_FIELD_IS_BLOCK: block_assign_dispose_flags = 7;

/// The field is an indirect reference to a variable declared with the __block
/// storage qualifier.
///
/// the on stack structure holding the __block variable
pub const BLOCK_FIELD_IS_BYREF: block_assign_dispose_flags = 8;

/// The field is an indirect reference to a variable declared with the __block
/// storage qualifier.
///
/// declared __weak, only used in byref copy helpers
pub const BLOCK_FIELD_IS_WEAK: block_assign_dispose_flags = 16;

/// The field is an indirect reference to a variable declared with the __block
/// storage qualifier.
///
/// called from __block (byref) copy/dispose support routines.
pub const BLOCK_BYREF_CALLER: block_assign_dispose_flags = 128;

#[cfg(any(doc, feature = "apple"))]
pub const BLOCK_ALL_COPY_DISPOSE_FLAGS: block_assign_dispose_flags = BLOCK_FIELD_IS_OBJECT
    | BLOCK_FIELD_IS_BLOCK
    | BLOCK_FIELD_IS_BYREF
    | BLOCK_FIELD_IS_WEAK
    | BLOCK_BYREF_CALLER;

// TODO: BLOCK_LAYOUT_X

extern "C" {
    // the raw data space for runtime classes for blocks
    // class+meta used for stack, malloc, and collectable based blocks

    pub static _NSConcreteGlobalBlock: Class;
    pub static _NSConcreteStackBlock: Class;
    pub static _NSConcreteMallocBlock: Class;
    #[cfg(any(doc, feature = "apple", feature = "compiler-rt"))]
    pub static _NSConcreteAutoBlock: Class;
    #[cfg(any(doc, feature = "apple", feature = "compiler-rt"))]
    pub static _NSConcreteFinalizingBlock: Class;
    #[cfg(any(doc, feature = "apple", feature = "compiler-rt"))]
    pub static _NSConcreteWeakBlockVariable: Class;

    pub fn _Block_copy(block: *const c_void) -> *mut c_void;
    pub fn _Block_release(block: *const c_void);

    /// Runtime entry point called by compiler when assigning objects inside
    /// copy helper routines
    pub fn _Block_object_assign(
        dest_addr: *mut c_void,
        object: *const c_void,
        flags: block_assign_dispose_flags,
    );

    /// runtime entry point called by the compiler when disposing of objects
    /// inside dispose helper routine
    pub fn _Block_object_dispose(object: *const c_void, flags: block_assign_dispose_flags);

    #[cfg(any(doc, feature = "apple", feature = "compiler-rt"))]
    pub fn Block_size(block: *mut c_void) -> c_ulong; // usize

    // Whether the return value of the block is on the stack.
    // macOS 10.7
    // #[cfg(any(doc, feature = "apple"))]
    // pub fn _Block_use_stret(block: *mut c_void) -> bool;

    // Returns a string describing the block's GC layout.
    // This uses the GC skip/scan encoding.
    // May return NULL.
    // macOS 10.7
    // #[cfg(any(doc, feature = "apple"))]
    // pub fn _Block_layout(block: *mut c_void) -> *const c_char;

    // Returns a string describing the block's layout.
    // This uses the "extended layout" form described above.
    // May return NULL.
    // macOS 10.8
    // #[cfg(any(doc, feature = "apple"))]
    // pub fn _Block_extended_layout(block: *mut c_void) -> *const c_char;

    // Callable only from the ARR weak subsystem while in exclusion zone
    // macOS 10.7
    // #[cfg(any(doc, feature = "apple"))]
    // pub fn _Block_tryRetain(block: *const c_void) -> bool;

    // Callable only from the ARR weak subsystem while in exclusion zone
    // macOS 10.7
    // #[cfg(any(doc, feature = "apple"))]
    // pub fn _Block_isDeallocating(block: *const c_void) -> bool;

    // indicates whether block was compiled with compiler that sets the ABI
    // related metadata bits
    // macOS 10.7
    // #[cfg(any(doc, feature = "apple", feature = "gnustep-1-7"))]
    // pub fn _Block_has_signature(block: *mut c_void) -> bool;

    // Returns a string describing the block's parameter and return types.
    // The encoding scheme is the same as Objective-C @encode.
    // Returns NULL for blocks compiled with some compilers.
    // macOS 10.7
    // #[cfg(any(doc, feature = "apple", feature = "gnustep-1-7"))]
    // pub fn _Block_signature(block: *mut c_void) -> *const c_char;
}

#[repr(C)]
pub struct Block_layout {
    /// Class pointer. Always initialised to &_NSConcreteStackBlock for blocks
    /// that are created on the stack or &_NSConcreteGlobalBlock for blocks
    /// that are created in global storage.
    pub isa: *const Class,
    /// Flags.
    /// See the `block_flags` enumerated type for possible values.
    /// Contains ref count in Apple and ObjFW.
    pub flags: block_flags,
    /// Reserved - always initialised to 0 by the compiler (but this is not
    /// said in the specification).
    ///
    /// Used for the reference count in GNUStep and WinObjC.
    pub reserved: i32,
    /// The function that implements the block.  The first argument is this
    /// structure, the subsequent arguments are the block's explicit
    /// parameters. If the BLOCK_USE_SRET & BLOCK_HAS_SIGNATURE flag is set,
    /// there is an additional hidden argument, which is a pointer to the
    /// space on the stack allocated to hold the return value.
    pub invoke: Option<unsafe extern "C" fn()>,
    /// The block's descriptor. The actual type of this is:
    /// ```pseudo-code
    /// match (BLOCK_HAS_COPY_DISPOSE, BLOCK_HAS_SIGNATURE) {
    ///     (false, false) => Block_descriptor_header,
    ///     (true, false) => Block_descriptor,
    ///     (false, true) => Block_descriptor_basic,
    ///     (true, true) => Block_descriptor_with_signature,
    /// }
    /// ```
    ///
    /// But it is safe to access this through just `Block_descriptor_header`.
    // Note: Important to use `*const c_void` until we know which type it is!
    pub descriptor: *const c_void,
}

#[repr(C)]
pub struct Block_descriptor_header {
    /// Reserved for future use. Currently always 0.
    pub reserved: c_ulong, // usize
    /// Size of the block.
    pub size: c_ulong, // usize
}

/// Block descriptor that contains copy and dispose operations.
///
/// Requires BLOCK_HAS_COPY_DISPOSE
#[repr(C)]
pub struct Block_descriptor {
    pub header: Block_descriptor_header,

    /// Copy function, generated by the compiler to help copy the block if it
    /// contains nontrivial copy operations.
    pub copy: Option<unsafe extern "C" fn(dst: *mut c_void, src: *mut c_void)>,
    /// Dispose function, generated by the compiler to help copy the block if
    /// it contains nontrivial destructors.
    pub dispose: Option<unsafe extern "C" fn(src: *mut c_void)>,
}

/// Extended block descriptor that does not contain copy and dispose helper
/// functions.
///
/// Requires BLOCK_HAS_SIGNATURE
#[repr(C)]
pub struct Block_descriptor_basic {
    pub header: Block_descriptor_header,

    /// Objective-C type encoding of the block.
    #[doc(alias = "signature")]
    pub encoding: *const c_char,
}

/// Requires BLOCK_HAS_COPY_DISPOSE and BLOCK_HAS_SIGNATURE
#[repr(C)]
pub struct Block_descriptor_with_signature {
    pub header: Block_descriptor_header,

    /// Same as [`Block_descriptor::copy`].
    pub copy: Option<unsafe extern "C" fn(dst: *mut c_void, src: *mut c_void)>,
    /// Same as [`Block_descriptor::dispose`].
    pub dispose: Option<unsafe extern "C" fn(src: *mut c_void)>,

    /// Objective-C type encoding of the block.
    #[doc(alias = "signature")]
    pub encoding: *const c_char,
}

// #[cfg(any(doc, feature = "apple"))]
// pub layout: *const c_char,

// #[repr(C)]
// pub struct Block_descriptor_small {
//     pub size: u32,
//     pub signature: i32,
//     pub layout: i32,
//     pub copy: i32,
//     pub dispose: i32,
// }

// #[repr(C)]
// pub struct Block_basic {
//     pub isa: *mut c_void,
//     pub Block_flags: i32,
//     pub Block_size: i32,
//     pub Block_invoke: Option<unsafe extern "C" fn()>,
//     pub Block_copy: Option<unsafe extern "C" fn(dst: *mut c_void, src: *mut c_void)>,
//     pub Block_dispose: Option<unsafe extern "C" fn(block: *mut c_void)>,
// }
// Example usage: https://github.com/apple-oss-distributions/libdispatch/blob/libdispatch-84.5/src/once.c

/// Structure used for on-stack variables that are referenced by blocks.
#[repr(C)]
#[doc(alias = "Block_byref_1")]
pub struct Block_byref_header {
    /// Class pointer. Currently unused on GNUStep and always NULL. Could be
    /// used in the future to support introspection.
    pub isa: *const Class,
    /// The pointer to the structure that contains the real version of the
    /// data. All accesses go via this pointer. If an on-stack byref structure
    /// is copied to the heap, then its forwarding pointer should point to the
    /// heap version. Otherwise it should point to itself.
    pub forwarding: *mut Block_byref_header,
    /// Flags and reference count.
    ///
    /// TODO: Volatile!
    pub flags: block_flags,
    #[cfg(feature = "apple")]
    /// Size of this structure.
    pub size: u32,
    #[cfg(not(feature = "apple"))]
    /// Size of this structure.
    pub size: i32,
}

/// Structure used for on-stack variables that are referenced by blocks.
///
/// requires BLOCK_BYREF_HAS_COPY_DISPOSE
#[repr(C)]
#[doc(alias = "Block_byref_2")]
pub struct Block_byref {
    pub header: Block_byref_header,
    /// Copy function.
    pub keep: Option<unsafe extern "C" fn(dst: *mut c_void, src: *mut c_void)>,
    /// Dispose function.
    pub destroy: Option<unsafe extern "C" fn(src: *mut c_void)>,
}

#[cfg(any(doc, feature = "apple"))]
/// Structure used for on-stack variables that are referenced by blocks.
///
/// requires BLOCK_BYREF_LAYOUT_EXTENDED
#[repr(C)]
#[doc(alias = "Block_byref_3")]
pub struct Block_byref_extended {
    pub header: Block_byref_header,
    /// Same as [`Block_byref::keep`].
    pub keep: Option<unsafe extern "C" fn(dst: *mut c_void, src: *mut c_void)>,
    /// Same as [`Block_byref::destroy`].
    pub destroy: Option<unsafe extern "C" fn(src: *mut c_void)>,
    pub layout: *const c_char,
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::ptr;
    use std::println;

    #[test]
    fn smoke() {
        assert_eq!(unsafe { _Block_copy(ptr::null_mut()) }, ptr::null_mut());
    }

    #[test]
    fn test_linkable() {
        println!("{:p}", unsafe { &_NSConcreteGlobalBlock });
        println!("{:p}", unsafe { &_NSConcreteStackBlock });
        println!("{:p}", unsafe { &_NSConcreteMallocBlock });
        println!("{:p}", _Block_copy as unsafe extern "C" fn(_) -> _);
        println!(
            "{:p}",
            _Block_object_assign as unsafe extern "C" fn(_, _, _)
        );
        println!("{:p}", _Block_object_dispose as unsafe extern "C" fn(_, _));
        println!("{:p}", _Block_release as unsafe extern "C" fn(_));
        #[cfg(any(feature = "apple", feature = "compiler-rt"))]
        {
            println!("{:p}", unsafe { &_NSConcreteAutoBlock });
            println!("{:p}", unsafe { &_NSConcreteFinalizingBlock });
            println!("{:p}", unsafe { &_NSConcreteWeakBlockVariable });
            println!("{:p}", Block_size as unsafe extern "C" fn(_) -> _);
        }
    }
}
