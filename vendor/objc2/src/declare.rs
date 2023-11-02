//! Functionality for dynamically declaring Objective-C classes.
//!
//! Classes can be declared using the [`ClassBuilder`] struct. Instance
//! variables and methods can then be added before the class is ultimately
//! registered.
//!
//! **Note**: You likely don't need the dynamicism that this module provides!
//! Consider using the [`declare_class!`][crate::declare_class] macro instead.
//!
//!
//! # Example
//!
//! The following example demonstrates declaring a class named `MyNumber` that
//! has one ivar, a `u32` named `_number` and a few methods for constructor
//! methods and methods for interfacing with the number.
//!
//! ```
//! use objc2::declare::ClassBuilder;
//! use objc2::foundation::NSObject;
//! use objc2::rc::{Id, Owned};
//! use objc2::runtime::{Class, Object, Sel};
//! use objc2::{class, sel, msg_send, msg_send_id, ClassType};
//! # #[cfg(feature = "gnustep-1-7")]
//! # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
//!
//! fn register_class() -> &'static Class {
//!     // Inherit from NSObject
//!     let mut builder = ClassBuilder::new("MyNumber", NSObject::class())
//!         .expect("a class with the name MyNumber likely already exists");
//!
//!     // Add an instance variable of type `u32`
//!     builder.add_ivar::<u32>("_number");
//!
//!     // Add an Objective-C method for initializing an instance with a number
//!     unsafe extern "C" fn init_with_number(
//!         this: &mut Object,
//!         _cmd: Sel,
//!         number: u32,
//!     ) -> Option<&mut Object> {
//!         let this: Option<&mut Object> = msg_send![super(this, NSObject::class()), init];
//!         this.map(|this| {
//!             // SAFETY: The ivar is added with the same type above
//!             this.set_ivar::<u32>("_number", number);
//!             this
//!         })
//!     }
//!     unsafe {
//!         builder.add_method(
//!             sel!(initWithNumber:),
//!             init_with_number as unsafe extern "C" fn(_, _, _) -> _,
//!         );
//!     }
//!
//!     // Add convenience method for getting a new instance with the number
//!     extern "C" fn with_number(
//!         cls: &Class,
//!         _cmd: Sel,
//!         number: u32,
//!     ) -> *mut Object {
//!         let obj: Option<Id<Object, Owned>> = unsafe {
//!             msg_send_id![
//!                 msg_send_id![cls, alloc],
//!                 initWithNumber: number,
//!             ]
//!         };
//!         obj.map(|obj| obj.autorelease_return()).unwrap_or(std::ptr::null_mut())
//!     }
//!     unsafe {
//!         builder.add_class_method(
//!             sel!(withNumber:),
//!             with_number as extern "C" fn(_, _, _) -> _,
//!         );
//!     }
//!
//!     // Add an Objective-C method for setting the number
//!     extern "C" fn my_number_set(this: &mut Object, _cmd: Sel, number: u32) {
//!         // SAFETY: The ivar is added with the same type above
//!         unsafe { this.set_ivar::<u32>("_number", number) }
//!     }
//!     unsafe {
//!         builder.add_method(sel!(setNumber:), my_number_set as extern "C" fn(_, _, _));
//!     }
//!
//!     // Add an Objective-C method for getting the number
//!     extern "C" fn my_number_get(this: &Object, _cmd: Sel) -> u32 {
//!         // SAFETY: The ivar is added with the same type above
//!         unsafe { *this.ivar::<u32>("_number") }
//!     }
//!     unsafe {
//!         builder.add_method(sel!(number), my_number_get as extern "C" fn(_, _) -> _);
//!     }
//!
//!     builder.register()
//! }
//!
//! // Usage
//!
//! // Note: you should only do class registration once! This can be ensure
//! // with `std::sync::Once` or the `once_cell` crate.
//! let cls = register_class();
//!
//! let obj: Id<Object, Owned> = unsafe {
//!     msg_send_id![cls, withNumber: 42u32]
//! };
//!
//! let n: u32 = unsafe { msg_send![&obj, number] };
//! assert_eq!(n, 42);
//!
//! let _: () = unsafe { msg_send![&obj, setNumber: 12u32] };
//! let n: u32 = unsafe { msg_send![&obj, number] };
//! assert_eq!(n, 12);
//! ```

mod ivar;
mod ivar_drop;
mod ivar_forwarding_impls;

use alloc::format;
use alloc::string::ToString;
use core::mem;
use core::mem::ManuallyDrop;
use core::ptr;
use core::ptr::NonNull;
use std::ffi::CString;

use crate::encode::{Encode, EncodeArguments, Encoding, RefEncode};
use crate::ffi;
use crate::runtime::{Bool, Class, Imp, Object, Protocol, Sel};
use crate::sel;
use crate::Message;

pub use ivar::{InnerIvarType, Ivar, IvarType};
pub use ivar_drop::IvarDrop;

pub(crate) mod private {
    pub trait Sealed {}
}

/// Types that can be used as the implementation of an Objective-C method.
///
/// This is a sealed trait that is implemented for a lot of `extern "C"`
/// function pointer types.
pub trait MethodImplementation: private::Sealed {
    /// The callee type of the method.
    type Callee: RefEncode + ?Sized;
    /// The return type of the method.
    type Ret: Encode;
    /// The argument types of the method.
    type Args: EncodeArguments;

    #[doc(hidden)]
    fn __imp(self) -> Imp;
}

macro_rules! method_decl_impl {
    (@<$($l:lifetime),*> T, $r:ident, $f:ty, $($t:ident),*) => {
        impl<$($l,)* T, $r, $($t),*> private::Sealed for $f
        where
            T: Message + ?Sized,
            $r: Encode,
            $($t: Encode,)*
        {}

        impl<$($l,)* T, $r, $($t),*> MethodImplementation for $f
        where
            T: Message + ?Sized,
            $r: Encode,
            $($t: Encode,)*
        {
            type Callee = T;
            type Ret = $r;
            type Args = ($($t,)*);

            fn __imp(self) -> Imp {
                unsafe { mem::transmute(self) }
            }
        }
    };
    (@<$($l:lifetime),*> Class, $r:ident, $f:ty, $($t:ident),*) => {
        impl<$($l,)* $r, $($t),*> private::Sealed for $f
        where
            $r: Encode,
            $($t: Encode,)*
        {}

        impl<$($l,)* $r, $($t),*> MethodImplementation for $f
        where
            $r: Encode,
            $($t: Encode,)*
        {
            type Callee = Class;
            type Ret = $r;
            type Args = ($($t,)*);

            fn __imp(self) -> Imp {
                unsafe { mem::transmute(self) }
            }
        }
    };
    (# $abi:literal; $($t:ident),*) => {
        method_decl_impl!(@<'a> T, R, extern $abi fn(&'a T, Sel $(, $t)*) -> R, $($t),*);
        method_decl_impl!(@<'a> T, R, extern $abi fn(&'a mut T, Sel $(, $t)*) -> R, $($t),*);
        method_decl_impl!(@<> T, R, unsafe extern $abi fn(*const T, Sel $(, $t)*) -> R, $($t),*);
        method_decl_impl!(@<> T, R, unsafe extern $abi fn(*mut T, Sel $(, $t)*) -> R, $($t),*);
        method_decl_impl!(@<'a> T, R, unsafe extern $abi fn(&'a T, Sel $(, $t)*) -> R, $($t),*);
        method_decl_impl!(@<'a> T, R, unsafe extern $abi fn(&'a mut T, Sel $(, $t)*) -> R, $($t),*);

        method_decl_impl!(@<'a> Class, R, extern $abi fn(&'a Class, Sel $(, $t)*) -> R, $($t),*);
        method_decl_impl!(@<> Class, R, unsafe extern $abi fn(*const Class, Sel $(, $t)*) -> R, $($t),*);
        method_decl_impl!(@<'a> Class, R, unsafe extern $abi fn(&'a Class, Sel $(, $t)*) -> R, $($t),*);
    };
    ($($t:ident),*) => {
        method_decl_impl!(# "C"; $($t),*);
        #[cfg(feature = "unstable-c-unwind")]
        method_decl_impl!(# "C-unwind"; $($t),*);
    };
}

method_decl_impl!();
method_decl_impl!(A);
method_decl_impl!(A, B);
method_decl_impl!(A, B, C);
method_decl_impl!(A, B, C, D);
method_decl_impl!(A, B, C, D, E);
method_decl_impl!(A, B, C, D, E, F);
method_decl_impl!(A, B, C, D, E, F, G);
method_decl_impl!(A, B, C, D, E, F, G, H);
method_decl_impl!(A, B, C, D, E, F, G, H, I);
method_decl_impl!(A, B, C, D, E, F, G, H, I, J);
method_decl_impl!(A, B, C, D, E, F, G, H, I, J, K);
method_decl_impl!(A, B, C, D, E, F, G, H, I, J, K, L);

fn count_args(sel: Sel) -> usize {
    sel.name().chars().filter(|&c| c == ':').count()
}

fn method_type_encoding(ret: &Encoding, args: &[Encoding]) -> CString {
    // First two arguments are always self and the selector
    let mut types = format!("{}{}{}", ret, <*mut Object>::ENCODING, Sel::ENCODING);
    for enc in args {
        use core::fmt::Write;
        write!(&mut types, "{}", enc).unwrap();
    }
    CString::new(types).unwrap()
}

fn log2_align_of<T>() -> u8 {
    let align = mem::align_of::<T>();
    // Alignments are required to be powers of 2
    debug_assert!(align.count_ones() == 1);
    // log2 of a power of 2 is the number of trailing zeros
    align.trailing_zeros() as u8
}

/// A type for declaring a new class and adding new methods and ivars to it
/// before registering it.
#[derive(Debug)]
pub struct ClassBuilder {
    // Note: Don't ever construct a &mut Class, since it is possible to get
    // this pointer using `Class::classes`!
    cls: NonNull<Class>,
}

#[doc(hidden)]
#[deprecated = "Use `ClassBuilder` instead."]
pub type ClassDecl = ClassBuilder;

// SAFETY: The stuff that touch global state does so using locks internally.
//
// Modifying the class itself can only be done through `&mut`, so Sync is
// safe (e.g. we can't accidentally call `add_ivar` at the same time from two
// different threads).
//
// (Though actually, that would be safe since the entire runtime is locked
// when doing so...).
//
// Finally, there are no requirements that the class must be registered on the
// same thread that allocated it (so Send is safe).
unsafe impl Send for ClassBuilder {}
unsafe impl Sync for ClassBuilder {}

impl ClassBuilder {
    fn as_mut_ptr(&mut self) -> *mut ffi::objc_class {
        self.cls.as_ptr().cast()
    }

    fn with_superclass(name: &str, superclass: Option<&Class>) -> Option<Self> {
        let name = CString::new(name).unwrap();
        let super_ptr = superclass.map_or(ptr::null(), |c| c).cast();
        let cls = unsafe { ffi::objc_allocateClassPair(super_ptr, name.as_ptr(), 0) };
        NonNull::new(cls.cast()).map(|cls| Self { cls })
    }

    /// Constructs a [`ClassBuilder`] with the given name and superclass.
    ///
    /// Returns [`None`] if the class couldn't be allocated, or a class with
    /// that name already exist.
    pub fn new(name: &str, superclass: &Class) -> Option<Self> {
        Self::with_superclass(name, Some(superclass))
    }

    /// Constructs a [`ClassBuilder`] declaring a new root class with the
    /// given name.
    ///
    /// Returns [`None`] if the class couldn't be allocated.
    ///
    /// An implementation for `+initialize` must also be given; the runtime
    /// calls this method for all classes, so it must be defined on root
    /// classes.
    ///
    /// Note that implementing a root class is not a simple endeavor!
    /// For example, your class probably cannot be passed to Cocoa code unless
    /// the entire `NSObject` protocol is implemented.
    /// Functionality it expects, like implementations of `-retain` and
    /// `-release` used by ARC, will not be present otherwise.
    pub fn root<F>(name: &str, intitialize_fn: F) -> Option<Self>
    where
        F: MethodImplementation<Callee = Class, Args = (), Ret = ()>,
    {
        Self::with_superclass(name, None).map(|mut this| {
            unsafe { this.add_class_method(sel!(initialize), intitialize_fn) };
            this
        })
    }

    /// Adds a method with the given name and implementation.
    ///
    /// # Panics
    ///
    /// Panics if the method wasn't sucessfully added or if the selector and
    /// function take different numbers of arguments.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the types match those that are expected
    /// when the method is invoked from Objective-C.
    pub unsafe fn add_method<T, F>(&mut self, sel: Sel, func: F)
    where
        T: Message + ?Sized,
        F: MethodImplementation<Callee = T>,
    {
        let encs = F::Args::ENCODINGS;
        let sel_args = count_args(sel);
        assert_eq!(
            sel_args,
            encs.len(),
            "Selector {:?} accepts {} arguments, but function accepts {}",
            sel,
            sel_args,
            encs.len(),
        );

        let types = method_type_encoding(&F::Ret::ENCODING, encs);
        let success = Bool::from_raw(unsafe {
            ffi::class_addMethod(
                self.as_mut_ptr(),
                sel.as_ptr(),
                Some(func.__imp()),
                types.as_ptr(),
            )
        });
        assert!(success.as_bool(), "Failed to add method {:?}", sel);
    }

    fn metaclass_mut(&mut self) -> *mut ffi::objc_class {
        unsafe { ffi::object_getClass(self.as_mut_ptr().cast()) as *mut ffi::objc_class }
    }

    /// Adds a class method with the given name and implementation.
    ///
    /// # Panics
    ///
    /// Panics if the method wasn't sucessfully added or if the selector and
    /// function take different numbers of arguments.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the types match those that are expected
    /// when the method is invoked from Objective-C.
    pub unsafe fn add_class_method<F>(&mut self, sel: Sel, func: F)
    where
        F: MethodImplementation<Callee = Class>,
    {
        let encs = F::Args::ENCODINGS;
        let sel_args = count_args(sel);
        assert_eq!(
            sel_args,
            encs.len(),
            "Selector {:?} accepts {} arguments, but function accepts {}",
            sel,
            sel_args,
            encs.len(),
        );

        let types = method_type_encoding(&F::Ret::ENCODING, encs);
        let success = Bool::from_raw(unsafe {
            ffi::class_addMethod(
                self.metaclass_mut(),
                sel.as_ptr(),
                Some(func.__imp()),
                types.as_ptr(),
            )
        });
        assert!(success.as_bool(), "Failed to add class method {:?}", sel);
    }

    /// Adds an ivar with type `T` and the provided name.
    ///
    ///
    /// # Panics
    ///
    /// If the ivar wasn't successfully added for some reason - this usually
    /// happens if there already was an ivar with that name.
    pub fn add_ivar<T: Encode>(&mut self, name: &str) {
        // SAFETY: The encoding is correct
        unsafe { self.add_ivar_inner::<T>(name, &T::ENCODING) }
    }

    unsafe fn add_ivar_inner<T>(&mut self, name: &str, encoding: &Encoding) {
        let c_name = CString::new(name).unwrap();
        let encoding = CString::new(encoding.to_string()).unwrap();
        let size = mem::size_of::<T>();
        let align = log2_align_of::<T>();
        let success = Bool::from_raw(unsafe {
            ffi::class_addIvar(
                self.as_mut_ptr(),
                c_name.as_ptr(),
                size,
                align,
                encoding.as_ptr(),
            )
        });
        assert!(success.as_bool(), "Failed to add ivar {}", name);
    }

    /// Adds an instance variable from an [`IvarType`].
    ///
    ///
    /// # Panics
    ///
    /// Same as [`ClassBuilder::add_ivar`].
    pub fn add_static_ivar<T: IvarType>(&mut self) {
        // SAFETY: The encoding is correct
        unsafe {
            self.add_ivar_inner::<<T::Type as InnerIvarType>::__Inner>(
                T::NAME,
                &T::Type::__ENCODING,
            )
        }
    }

    /// Adds the given protocol to self.
    ///
    /// # Panics
    ///
    /// If the protocol wasn't successfully added.
    pub fn add_protocol(&mut self, proto: &Protocol) {
        let success = unsafe { ffi::class_addProtocol(self.as_mut_ptr(), proto.as_ptr()) };
        let success = Bool::from_raw(success).as_bool();
        assert!(success, "Failed to add protocol {:?}", proto);
    }

    // fn add_property(&self, name: &str, attributes: &[ffi::objc_property_attribute_t]);

    /// Registers the [`ClassBuilder`], consuming it, and returns a reference
    /// to the newly registered [`Class`].
    pub fn register(self) -> &'static Class {
        // Forget self, otherwise the class will be disposed in drop
        let mut this = ManuallyDrop::new(self);
        unsafe { ffi::objc_registerClassPair(this.as_mut_ptr()) };
        unsafe { this.cls.as_ref() }
    }
}

impl Drop for ClassBuilder {
    fn drop(&mut self) {
        unsafe { ffi::objc_disposeClassPair(self.as_mut_ptr()) }
    }
}

/// A type for declaring a new protocol and adding new methods to it
/// before registering it.
#[derive(Debug)]
pub struct ProtocolBuilder {
    proto: NonNull<Protocol>,
}

#[doc(hidden)]
#[deprecated = "Use `ProtocolBuilder` instead."]
pub type ProtocolDecl = ProtocolBuilder;

// SAFETY: Similar to ClassBuilder
unsafe impl Send for ProtocolBuilder {}
unsafe impl Sync for ProtocolBuilder {}

impl ProtocolBuilder {
    fn as_mut_ptr(&mut self) -> *mut ffi::objc_protocol {
        self.proto.as_ptr().cast()
    }

    /// Constructs a [`ProtocolBuilder`] with the given name.
    ///
    /// Returns [`None`] if the protocol couldn't be allocated.
    pub fn new(name: &str) -> Option<Self> {
        let c_name = CString::new(name).unwrap();
        let proto = unsafe { ffi::objc_allocateProtocol(c_name.as_ptr()) };
        NonNull::new(proto.cast()).map(|proto| Self { proto })
    }

    fn add_method_description_common<Args, Ret>(
        &mut self,
        sel: Sel,
        is_required: bool,
        is_instance_method: bool,
    ) where
        Args: EncodeArguments,
        Ret: Encode,
    {
        let encs = Args::ENCODINGS;
        let sel_args = count_args(sel);
        assert_eq!(
            sel_args,
            encs.len(),
            "Selector {:?} accepts {} arguments, but function accepts {}",
            sel,
            sel_args,
            encs.len(),
        );
        let types = method_type_encoding(&Ret::ENCODING, encs);
        unsafe {
            ffi::protocol_addMethodDescription(
                self.as_mut_ptr(),
                sel.as_ptr(),
                types.as_ptr(),
                Bool::new(is_required).as_raw(),
                Bool::new(is_instance_method).as_raw(),
            );
        }
    }

    /// Adds an instance method declaration with a given description.
    pub fn add_method_description<Args, Ret>(&mut self, sel: Sel, is_required: bool)
    where
        Args: EncodeArguments,
        Ret: Encode,
    {
        self.add_method_description_common::<Args, Ret>(sel, is_required, true)
    }

    /// Adds a class method declaration with a given description.
    pub fn add_class_method_description<Args, Ret>(&mut self, sel: Sel, is_required: bool)
    where
        Args: EncodeArguments,
        Ret: Encode,
    {
        self.add_method_description_common::<Args, Ret>(sel, is_required, false)
    }

    /// Adds a requirement on another protocol.
    pub fn add_protocol(&mut self, proto: &Protocol) {
        unsafe {
            ffi::protocol_addProtocol(self.as_mut_ptr(), proto.as_ptr());
        }
    }

    /// Registers the [`ProtocolBuilder`], consuming it and returning a reference
    /// to the newly registered [`Protocol`].
    pub fn register(mut self) -> &'static Protocol {
        unsafe {
            ffi::objc_registerProtocol(self.as_mut_ptr());
            self.proto.as_ref()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg_send;
    use crate::test_utils;

    #[test]
    fn test_classbuilder_duplicate() {
        let cls = test_utils::custom_class();
        let builder = ClassBuilder::new("TestClassBuilderDuplicate", cls).unwrap();
        let _ = builder.register();

        assert!(ClassBuilder::new("TestClassBuilderDuplicate", cls).is_none());
    }

    #[test]
    #[should_panic = "Failed to add ivar xyz"]
    fn duplicate_ivar() {
        let cls = test_utils::custom_class();
        let builder = ClassBuilder::new("TestClassBuilderDuplicateIvar", cls).unwrap();
        // ManuallyDrop to work around GNUStep issue
        let mut builder = ManuallyDrop::new(builder);

        builder.add_ivar::<i32>("xyz");
        // Should panic:
        builder.add_ivar::<i32>("xyz");
    }

    #[test]
    #[should_panic = "Failed to add method xyz"]
    fn duplicate_method() {
        let cls = test_utils::custom_class();
        let builder = ClassBuilder::new("TestClassBuilderDuplicateMethod", cls).unwrap();
        let mut builder = ManuallyDrop::new(builder);

        extern "C" fn xyz(_this: &Object, _cmd: Sel) {}

        unsafe {
            builder.add_method(sel!(xyz), xyz as extern "C" fn(_, _));
            // Should panic:
            builder.add_method(sel!(xyz), xyz as extern "C" fn(_, _));
        }
    }

    #[test]
    #[should_panic = "Failed to add protocol NSObject"]
    fn duplicate_protocol() {
        let cls = test_utils::custom_class();
        let builder = ClassBuilder::new("TestClassBuilderDuplicateProtocol", cls).unwrap();
        let mut builder = ManuallyDrop::new(builder);

        let protocol = Protocol::get("NSObject").unwrap();

        builder.add_protocol(protocol);
        // Should panic:
        builder.add_protocol(protocol);
    }

    #[test]
    #[cfg_attr(
        feature = "gnustep-1-7",
        ignore = "Dropping ClassBuilder has weird threading side effects on GNUStep"
    )]
    fn test_classbuilder_drop() {
        let cls = test_utils::custom_class();
        let builder = ClassBuilder::new("TestClassBuilderDrop", cls).unwrap();
        drop(builder);
        // After we dropped the class, we can create a new one with the same name:
        let _builder = ClassBuilder::new("TestClassBuilderDrop", cls).unwrap();
    }

    #[test]
    fn test_custom_class() {
        // Registering the custom class is in test_utils
        let mut obj = test_utils::custom_object();
        let _: () = unsafe { msg_send![&mut obj, setFoo: 13u32] };
        let result: u32 = unsafe { msg_send![&obj, foo] };
        assert_eq!(result, 13);
    }

    #[test]
    #[cfg(feature = "malloc")]
    fn test_in_all_classes() {
        fn is_present(cls: *const Class) -> bool {
            // Check whether the class is present in Class::classes()
            Class::classes()
                .into_iter()
                .find(|&item| ptr::eq(cls, *item))
                .is_some()
        }

        let superclass = test_utils::custom_class();
        let builder = ClassBuilder::new("TestFetchWhileCreatingClass", superclass).unwrap();

        if cfg!(all(feature = "apple", target_arch = "x86_64")) {
            // It is IMO a bug that it is present here!
            assert!(is_present(builder.cls.as_ptr()));
        } else {
            assert!(!is_present(builder.cls.as_ptr()));
        }

        let cls = builder.register();
        assert!(is_present(cls));
    }

    #[test]
    fn test_class_method() {
        let cls = test_utils::custom_class();
        let result: u32 = unsafe { msg_send![cls, classFoo] };
        assert_eq!(result, 7);
    }
}
