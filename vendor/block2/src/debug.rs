use alloc::format;
use core::ffi::c_void;
use core::fmt::{Debug, DebugStruct, Error, Formatter};
use core::ptr;
use std::ffi::CStr;

use crate::{ffi, Block, ConcreteBlock, GlobalBlock, RcBlock};

#[derive(Clone, Copy, PartialEq, Eq)]
struct Isa(*const ffi::Class);

impl Isa {
    fn is_global(self) -> bool {
        ptr::eq(unsafe { &ffi::_NSConcreteGlobalBlock }, self.0)
    }

    fn is_stack(self) -> bool {
        ptr::eq(unsafe { &ffi::_NSConcreteStackBlock }, self.0)
    }

    fn is_malloc(self) -> bool {
        ptr::eq(unsafe { &ffi::_NSConcreteMallocBlock }, self.0)
    }
}

impl Debug for Isa {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        if self.is_global() {
            f.write_str("_NSConcreteGlobalBlock")
        } else if self.is_stack() {
            f.write_str("_NSConcreteStackBlock")
        } else if self.is_malloc() {
            f.write_str("_NSConcreteMallocBlock")
        } else {
            write!(f, "{:?}", self.0)
        }
    }
}

fn debug_block_layout(layout: &ffi::Block_layout, f: &mut DebugStruct<'_, '_>) {
    f.field("isa", &Isa(layout.isa));
    f.field("flags", &BlockFlags(layout.flags));
    f.field("reserved", &layout.reserved);
    f.field("invoke", &layout.invoke);
    f.field(
        "descriptor",
        &BlockDescriptor {
            has_copy_dispose: layout.flags & ffi::BLOCK_HAS_COPY_DISPOSE != 0,
            has_signature: layout.flags & ffi::BLOCK_HAS_SIGNATURE != 0,
            descriptor: layout.descriptor,
        },
    );
}

impl<A, R> Debug for Block<A, R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let mut f = f.debug_struct("Block");
        let ptr: *const Self = self;
        let layout = unsafe { ptr.cast::<ffi::Block_layout>().as_ref().unwrap() };
        debug_block_layout(layout, &mut f);
        f.finish_non_exhaustive()
    }
}

impl<A, R> Debug for RcBlock<A, R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let mut f = f.debug_struct("RcBlock");
        let layout = unsafe { self.ptr.cast::<ffi::Block_layout>().as_ref().unwrap() };
        debug_block_layout(layout, &mut f);
        f.finish_non_exhaustive()
    }
}

impl<A, R, F: Debug> Debug for ConcreteBlock<A, R, F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let mut f = f.debug_struct("ConcreteBlock");
        debug_block_layout(&self.layout, &mut f);
        f.field("closure", &self.closure);
        f.finish()
    }
}

impl<A, R> Debug for GlobalBlock<A, R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let mut f = f.debug_struct("GlobalBlock");
        debug_block_layout(&self.layout, &mut f);
        f.finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct BlockFlags(ffi::block_flags);

impl Debug for BlockFlags {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        let mut f = f.debug_struct("BlockFlags");
        f.field("value", &format!("{:032b}", self.0));

        macro_rules! test_flags {
            {$(
                $(#[$m:meta])?
                $name:ident: $flag:ident
            );* $(;)?} => ($(
                $(#[$m])?
                f.field(stringify!($name), &(self.0 & ffi::$flag != 0));
            )*)
        }
        test_flags! {
            #[cfg(feature = "apple")]
            deallocating: BLOCK_DEALLOCATING;
            #[cfg(feature = "apple")]
            inline_layout_string: BLOCK_INLINE_LAYOUT_STRING;
            #[cfg(feature = "apple")]
            small_descriptor: BLOCK_SMALL_DESCRIPTOR;
            #[cfg(feature = "apple")]
            is_noescape: BLOCK_IS_NOESCAPE;
            #[cfg(feature = "apple")]
            needs_free: BLOCK_NEEDS_FREE;
            has_copy_dispose: BLOCK_HAS_COPY_DISPOSE;
            has_ctor: BLOCK_HAS_CTOR;
            #[cfg(feature = "apple")]
            is_gc: BLOCK_IS_GC;
            is_global: BLOCK_IS_GLOBAL;
            use_stret: BLOCK_USE_STRET;
            has_signature: BLOCK_HAS_SIGNATURE;
            #[cfg(feature = "apple")]
            has_extended_layout: BLOCK_HAS_EXTENDED_LAYOUT;
        }

        f.field(
            "over_referenced",
            &(self.0 & ffi::BLOCK_REFCOUNT_MASK == ffi::BLOCK_REFCOUNT_MASK),
        );
        f.field(
            "reference_count",
            &((self.0 & ffi::BLOCK_REFCOUNT_MASK) >> 1),
        );
        f.finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct BlockDescriptor {
    has_copy_dispose: bool,
    has_signature: bool,
    descriptor: *const c_void,
}

impl Debug for BlockDescriptor {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        if self.descriptor.is_null() {
            return f.write_str("(null)");
        }

        let mut f = f.debug_struct("BlockDescriptor");

        let header = unsafe {
            self.descriptor
                .cast::<ffi::Block_descriptor_header>()
                .as_ref()
                .unwrap()
        };

        f.field("reserved", &header.reserved);
        f.field("size", &header.size);

        match (self.has_copy_dispose, self.has_signature) {
            (false, false) => {}
            (true, false) => {
                let descriptor = unsafe {
                    self.descriptor
                        .cast::<ffi::Block_descriptor>()
                        .as_ref()
                        .unwrap()
                };
                f.field("copy", &descriptor.copy);
                f.field("dispose", &descriptor.dispose);
            }
            (false, true) => {
                let descriptor = unsafe {
                    self.descriptor
                        .cast::<ffi::Block_descriptor_basic>()
                        .as_ref()
                        .unwrap()
                };
                f.field(
                    "encoding",
                    &if descriptor.encoding.is_null() {
                        None
                    } else {
                        Some(unsafe { CStr::from_ptr(descriptor.encoding) })
                    },
                );
            }
            (true, true) => {
                let descriptor = unsafe {
                    self.descriptor
                        .cast::<ffi::Block_descriptor_with_signature>()
                        .as_ref()
                        .unwrap()
                };
                f.field("copy", &descriptor.copy);
                f.field("dispose", &descriptor.dispose);
                f.field(
                    "encoding",
                    &if descriptor.encoding.is_null() {
                        None
                    } else {
                        Some(unsafe { CStr::from_ptr(descriptor.encoding) })
                    },
                );
            }
        }

        f.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_isa() {
        let isa = Isa(unsafe { &ffi::_NSConcreteGlobalBlock });
        assert!(isa.is_global());
        assert!(!isa.is_stack());
        assert!(!isa.is_malloc());
        let isa = Isa(unsafe { &ffi::_NSConcreteStackBlock });
        assert!(!isa.is_global());
        assert!(isa.is_stack());
        assert!(!isa.is_malloc());
        let isa = Isa(unsafe { &ffi::_NSConcreteMallocBlock });
        assert!(!isa.is_global());
        assert!(!isa.is_stack());
        assert!(isa.is_malloc());
        let isa = Isa(ptr::null());
        assert!(!isa.is_global());
        assert!(!isa.is_stack());
        assert!(!isa.is_malloc());
    }
}
