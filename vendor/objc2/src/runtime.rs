//! A Rust interface for the functionality of the Objective-C runtime.
//!
//! For more information on foreign functions, see Apple's documentation:
//! <https://developer.apple.com/library/mac/documentation/Cocoa/Reference/ObjCRuntimeRef/index.html>

#[cfg(doc)]
use core::cell::UnsafeCell;
use core::fmt;
use core::hash;
use core::panic::{RefUnwindSafe, UnwindSafe};
use core::ptr::{self, NonNull};
use core::str;
#[cfg(feature = "malloc")]
use malloc_buf::Malloc;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
#[cfg(feature = "malloc")]
use std::os::raw::c_uint;

use crate::encode::{Encode, Encoding, RefEncode};
use crate::ffi;
#[cfg(feature = "malloc")]
use crate::{
    encode::{EncodeArguments, EncodeConvert},
    verify::verify_message_signature,
    VerificationError,
};

#[doc(inline)]
pub use crate::encode::__bool::Bool;

/// Use [`Bool`] or [`ffi::BOOL`] instead.
#[deprecated = "Use `Bool` or `ffi::BOOL` instead"]
#[allow(non_upper_case_globals)]
pub type BOOL = ffi::BOOL;

/// Use [`Bool::YES`] or [`ffi::YES`] instead.
#[deprecated = "Use `Bool::YES` or `ffi::YES` instead"]
pub const YES: ffi::BOOL = ffi::YES;

/// Use [`Bool::NO`] or [`ffi::NO`] instead.
#[deprecated = "Use `Bool::NO` or `ffi::NO` instead"]
pub const NO: ffi::BOOL = ffi::NO;

/// A method selector.
///
/// The Rust equivalent of Objective-C's `SEL` type. You can easily create
/// this using the [`sel!`] macro.
///
/// The main reason the Objective-C runtime uses a custom types for selectors
/// is to support efficient comparison - a selector is effectively just an
/// [interned string], so this makes that very easy!
///
/// This guarantees the null-pointer optimization, namely that `Option<Sel>`
/// is the same size as `Sel`.
///
/// [`sel!`]: crate::sel
/// [interned string]: https://en.wikipedia.org/wiki/String_interning
#[repr(transparent)]
// ffi::sel_isEqual is just pointer comparison, so we just generate PartialEq
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[doc(alias = "SEL")]
pub struct Sel {
    ptr: NonNull<ffi::objc_selector>,
}

/// A type that represents an instance variable.
#[repr(C)]
pub struct Ivar(ffi::objc_ivar);

/// A type that represents a method in a class definition.
#[repr(C)]
pub struct Method(ffi::objc_method);

/// A type that represents an Objective-C class.
#[repr(C)]
pub struct Class(ffi::objc_class);

/// A type that represents an Objective-C protocol.
#[repr(C)]
pub struct Protocol(ffi::objc_protocol);

macro_rules! standard_pointer_impls {
    ($($name:ident),*) => {
        $(
            impl PartialEq for $name {
                #[inline]
                fn eq(&self, other: &Self) -> bool {
                    self.as_ptr() == other.as_ptr()
                }
            }
            impl Eq for $name {}
            impl hash::Hash for $name {
                #[inline]
                fn hash<H: hash::Hasher>(&self, state: &mut H) {
                    self.as_ptr().hash(state)
                }
            }
        )*
    }
}

// Implement PartialEq, Eq and Hash using pointer semantics; there's not
// really a better way to do it.
standard_pointer_impls!(Ivar, Method, Class);

#[cfg(not(feature = "unstable-c-unwind"))]
type InnerImp = unsafe extern "C" fn();
#[cfg(feature = "unstable-c-unwind")]
type InnerImp = unsafe extern "C-unwind" fn();

/// A pointer to the start of a method implementation.
///
/// # Safety
///
/// This is a "catch all" type; it must be transmuted to the correct type
/// before being called!
pub type Imp = InnerImp;

impl Sel {
    #[inline]
    #[doc(hidden)]
    pub const unsafe fn __internal_from_ptr(ptr: *const ffi::objc_selector) -> Self {
        // Used in static selectors.
        // SAFETY: Upheld by caller.
        let ptr = unsafe { NonNull::new_unchecked(ptr as *mut ffi::objc_selector) };
        Self { ptr }
    }

    #[inline]
    pub(crate) unsafe fn from_ptr(ptr: *const ffi::objc_selector) -> Option<Self> {
        // SAFETY: Caller verifies that the pointer is valid.
        NonNull::new(ptr as *mut ffi::objc_selector).map(|ptr| Self { ptr })
    }

    /// Get a pointer to the raw selector.
    ///
    /// Useful when working with raw FFI methods.
    #[inline]
    pub const fn as_ptr(&self) -> *const ffi::objc_selector {
        self.ptr.as_ptr()
    }

    pub(crate) unsafe fn register_unchecked(name: *const c_char) -> Self {
        let ptr = unsafe { ffi::sel_registerName(name) };
        // SAFETY: `sel_registerName` declares return type as `SEL _Nonnull`,
        // at least when input is also `_Nonnull` (which it is in our case).
        //
        // Looking at the source code, it can fail and will return NULL if
        // allocating space for the selector failed (which then subsequently
        // invokes UB by calling `memcpy` with a NULL argument):
        // <https://github.com/apple-oss-distributions/objc4/blob/objc4-841.13/runtime/objc-os.h#L1002-L1004>
        //
        // I suspect this will be really uncommon in practice, since the
        // called selector is almost always going to be present in the binary
        // already; but alas, we'll handle it!
        unsafe { Self::from_ptr(ptr).unwrap() }
    }

    /// Registers a selector with the Objective-C runtime.
    ///
    /// This is the dynamic version of the [`sel!`] macro, prefer to use that
    /// when your selector is static.
    ///
    /// [`sel!`]: crate::sel
    ///
    ///
    /// # Panics
    ///
    /// Panics if `name` contains an internal NUL byte, or if the runtime
    /// failed allocating space for the selector.
    #[doc(alias = "sel_registerName")]
    pub fn register(name: &str) -> Self {
        let name = CString::new(name).unwrap();
        // SAFETY: Input is a non-null, NUL-terminated C-string pointer.
        unsafe { Self::register_unchecked(name.as_ptr()) }
    }

    /// Returns the string representation of the selector.
    ///
    /// # Panics
    ///
    /// Panics if the selector is not valid UTF-8 (however unlikely!)
    #[doc(alias = "sel_getName")]
    pub fn name(&self) -> &str {
        // SAFETY: Input is non-null selector. Declares return type as
        // `const char * _Nonnull`, source code agrees.
        let ptr = unsafe { ffi::sel_getName(self.as_ptr()) };
        // SAFETY: The string is a valid C-style NUL-terminated string, and
        // likely has static lifetime since the selector has static lifetime
        // (though we bind it to `&self` just to be safe).
        let name = unsafe { CStr::from_ptr(ptr) };
        str::from_utf8(name.to_bytes()).unwrap()
    }
}

// SAFETY: `Sel` is FFI compatible, and the encoding is of course `Sel`.
unsafe impl Encode for Sel {
    const ENCODING: Encoding = Encoding::Sel;
}

// RefEncode is not implemented for Sel, because there is literally no API
// that takes &Sel, but the user could easily get confused and accidentally
// attempt that.

// SAFETY: Sel is immutable (and can be retrieved from any thread using the
// `sel!` macro).
unsafe impl Sync for Sel {}
unsafe impl Send for Sel {}
impl UnwindSafe for Sel {}
impl RefUnwindSafe for Sel {}

impl fmt::Debug for Sel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl fmt::Pointer for Sel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&self.ptr, f)
    }
}

impl Ivar {
    pub(crate) fn as_ptr(&self) -> *const ffi::objc_ivar {
        let ptr: *const Self = self;
        ptr.cast()
    }

    /// Returns the name of self.
    pub fn name(&self) -> &str {
        let name = unsafe { CStr::from_ptr(ffi::ivar_getName(self.as_ptr())) };
        str::from_utf8(name.to_bytes()).unwrap()
    }

    /// Returns the offset of self.
    pub fn offset(&self) -> isize {
        unsafe { ffi::ivar_getOffset(self.as_ptr()) }
    }

    /// Returns the `Encoding` of self.
    pub fn type_encoding(&self) -> &str {
        let encoding = unsafe { CStr::from_ptr(ffi::ivar_getTypeEncoding(self.as_ptr())) };
        str::from_utf8(encoding.to_bytes()).unwrap()
    }
}

// SAFETY: Ivar is immutable (and can be retrieved from Class anyhow).
unsafe impl Sync for Ivar {}
unsafe impl Send for Ivar {}
impl UnwindSafe for Ivar {}
impl RefUnwindSafe for Ivar {}

impl Method {
    pub(crate) fn as_ptr(&self) -> *const ffi::objc_method {
        let ptr: *const Self = self;
        ptr.cast()
    }

    /// Returns the name of self.
    pub fn name(&self) -> Sel {
        unsafe { Sel::from_ptr(ffi::method_getName(self.as_ptr())).unwrap() }
    }

    /// Returns the `Encoding` of self's return type.
    #[cfg(feature = "malloc")]
    pub fn return_type(&self) -> Malloc<str> {
        unsafe {
            let encoding = ffi::method_copyReturnType(self.as_ptr());
            Malloc::from_c_str(encoding).unwrap()
        }
    }

    /// Returns the `Encoding` of a single parameter type of self, or
    /// [`None`] if self has no parameter at the given index.
    #[cfg(feature = "malloc")]
    pub fn argument_type(&self, index: usize) -> Option<Malloc<str>> {
        unsafe {
            let encoding = ffi::method_copyArgumentType(self.as_ptr(), index as c_uint);
            NonNull::new(encoding).map(|encoding| Malloc::from_c_str(encoding.as_ptr()).unwrap())
        }
    }

    // method_getTypeEncoding, efficient version of:
    // -> return_type() + sum(argument_type(i) for i in arguments_count())

    /// Returns the number of arguments accepted by self.
    pub fn arguments_count(&self) -> usize {
        unsafe { ffi::method_getNumberOfArguments(self.as_ptr()) as usize }
    }

    /// Returns the implementation of self.
    pub fn implementation(&self) -> Imp {
        unsafe { ffi::method_getImplementation(self.as_ptr()).expect("Null IMP") }
    }

    // unsafe fn set_implementation(&mut self, imp: Imp) -> Imp;
    // unsafe fn exchange_implementation(&mut self, other: &mut Method);
}

// SAFETY: Method is immutable (and can be retrieved from Class anyhow).
unsafe impl Sync for Method {}
unsafe impl Send for Method {}
impl UnwindSafe for Method {}
impl RefUnwindSafe for Method {}

impl Class {
    pub(crate) fn as_ptr(&self) -> *const ffi::objc_class {
        let ptr: *const Self = self;
        ptr.cast()
    }

    /// Returns the class definition of a specified class, or [`None`] if the
    /// class is not registered with the Objective-C runtime.
    pub fn get(name: &str) -> Option<&'static Self> {
        let name = CString::new(name).unwrap();
        let cls = unsafe { ffi::objc_getClass(name.as_ptr()) };
        unsafe { cls.cast::<Self>().as_ref() }
    }

    // Same as `get`, but ...
    // fn lookup(name: &str) -> Option<&'static Self>;

    /// Obtains the list of registered class definitions.
    #[cfg(feature = "malloc")]
    pub fn classes() -> Malloc<[&'static Self]> {
        unsafe {
            let mut count: c_uint = 0;
            let classes: *mut &Self = ffi::objc_copyClassList(&mut count).cast();
            Malloc::from_array(classes, count as usize)
        }
    }

    /// Returns the total number of registered classes.
    pub fn classes_count() -> usize {
        unsafe { ffi::objc_getClassList(ptr::null_mut(), 0) as usize }
    }

    /// Returns the name of the class.
    pub fn name(&self) -> &str {
        let name = unsafe { CStr::from_ptr(ffi::class_getName(self.as_ptr())) };
        str::from_utf8(name.to_bytes()).unwrap()
    }

    /// Returns the superclass of self, or [`None`] if self is a root class.
    pub fn superclass(&self) -> Option<&Class> {
        unsafe {
            let superclass = ffi::class_getSuperclass(self.as_ptr());
            superclass.cast::<Class>().as_ref()
        }
    }

    /// Returns the metaclass of self.
    pub fn metaclass(&self) -> &Self {
        let ptr: *const Self = unsafe { ffi::object_getClass(self.as_ptr().cast()) }.cast();
        unsafe { ptr.as_ref().unwrap_unchecked() }
    }

    // objc_getMetaClass -> Same as `Class::get(name).metaclass()`

    #[allow(unused)]
    pub(crate) fn is_metaclass(&self) -> bool {
        unsafe { Bool::from_raw(ffi::class_isMetaClass(self.as_ptr())).as_bool() }
    }

    /// Returns the size of instances of self.
    pub fn instance_size(&self) -> usize {
        unsafe { ffi::class_getInstanceSize(self.as_ptr()) as usize }
    }

    /// Returns a specified instance method for self, or [`None`] if self and
    /// its superclasses do not contain an instance method with the specified
    /// selector.
    pub fn instance_method(&self, sel: Sel) -> Option<&Method> {
        unsafe {
            let method = ffi::class_getInstanceMethod(self.as_ptr(), sel.as_ptr());
            method.cast::<Method>().as_ref()
        }
    }

    // fn class_method(&self, sel: Sel) -> Option<&Method>;

    /// Returns the ivar for a specified instance variable of self, or
    /// [`None`] if self has no ivar with the given name.
    pub fn instance_variable(&self, name: &str) -> Option<&Ivar> {
        let name = CString::new(name).unwrap();
        unsafe {
            let ivar = ffi::class_getInstanceVariable(self.as_ptr(), name.as_ptr());
            ivar.cast::<Ivar>().as_ref()
        }
    }

    #[allow(unused)]
    fn instance_variable_layout(&self) -> Option<&[u8]> {
        let layout: *const c_char = unsafe { ffi::class_getIvarLayout(self.as_ptr()).cast() };
        if layout.is_null() {
            None
        } else {
            Some(unsafe { CStr::from_ptr(layout) }.to_bytes())
        }
    }

    #[allow(unused)]
    fn class_variable(&self, name: &str) -> Option<&Ivar> {
        let name = CString::new(name).unwrap();
        let ivar = unsafe { ffi::class_getClassVariable(self.as_ptr(), name.as_ptr()) };
        // SAFETY: TODO
        unsafe { ivar.cast::<Ivar>().as_ref() }
    }

    /// Describes the instance methods implemented by self.
    #[cfg(feature = "malloc")]
    pub fn instance_methods(&self) -> Malloc<[&Method]> {
        unsafe {
            let mut count: c_uint = 0;
            let methods: *mut &Method = ffi::class_copyMethodList(self.as_ptr(), &mut count).cast();
            Malloc::from_array(methods, count as usize)
        }
    }

    /// Checks whether this class conforms to the specified protocol.
    pub fn conforms_to(&self, proto: &Protocol) -> bool {
        unsafe {
            Bool::from_raw(ffi::class_conformsToProtocol(self.as_ptr(), proto.as_ptr())).as_bool()
        }
    }

    /// Get a list of the protocols to which this class conforms.
    #[cfg(feature = "malloc")]
    pub fn adopted_protocols(&self) -> Malloc<[&Protocol]> {
        unsafe {
            let mut count: c_uint = 0;
            let protos: *mut &Protocol =
                ffi::class_copyProtocolList(self.as_ptr(), &mut count).cast();
            Malloc::from_array(protos, count as usize)
        }
    }

    /// Describes the instance variables declared by self.
    #[cfg(feature = "malloc")]
    pub fn instance_variables(&self) -> Malloc<[&Ivar]> {
        unsafe {
            let mut count: c_uint = 0;
            let ivars: *mut &Ivar = ffi::class_copyIvarList(self.as_ptr(), &mut count).cast();
            Malloc::from_array(ivars, count as usize)
        }
    }

    /// Check whether instances of this class respond to the given selector.
    ///
    /// This doesn't call `respondsToSelector:`, but works entirely within the
    /// runtime, which means it'll always be safe to call, but may not return
    /// exactly what you'd expect if `respondsToSelector:` has been
    /// overwritten.
    ///
    /// That said, it will always return `true` if an instance of the class
    /// responds to the selector, but may return `false` if they don't
    /// directly (e.g. does so by using forwarding instead).
    #[doc(alias = "class_respondsToSelector")]
    pub fn responds_to(&self, sel: Sel) -> bool {
        // This may call `resolveInstanceMethod:` and `resolveClassMethod:`
        // SAFETY: The selector is guaranteed non-null.
        let res = unsafe { ffi::class_respondsToSelector(self.as_ptr(), sel.as_ptr()) };
        Bool::from_raw(res).as_bool()
    }

    // fn property(&self, name: &str) -> Option<&Property>;
    // fn properties(&self) -> Malloc<[&Property]>;
    // unsafe fn replace_method(&self, name: Sel, imp: Imp, types: &str) -> Imp;
    // unsafe fn replace_property(&self, name: &str, attributes: &[ffi::objc_property_attribute_t]);
    // unsafe fn set_ivar_layout(&mut self, layout: &[u8]);
    // fn method_imp(&self, name: Sel) -> Imp; // + _stret

    // fn get_version(&self) -> u32;
    // unsafe fn set_version(&mut self, version: u32);

    /// Verify argument and return types for a given selector.
    ///
    /// This will look up the encoding of the method for the given selector
    /// and return a [`VerificationError`] if any encodings differ for the
    /// arguments `A` and return type `R`.
    ///
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "gnustep-1-7")]
    /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
    /// # use objc2::{class, sel};
    /// # use objc2::runtime::Class;
    /// let cls = class!(NSObject);
    /// let sel = sel!(isKindOfClass:);
    /// // Verify that `isKindOfClass:`:
    /// // - Exists on the class
    /// // - Takes a class as a parameter
    /// // - Returns a BOOL
    /// let result = cls.verify_sel::<(&Class,), bool>(sel);
    /// assert!(result.is_ok());
    /// ```
    #[cfg(feature = "malloc")]
    pub fn verify_sel<A, R>(&self, sel: Sel) -> Result<(), VerificationError>
    where
        A: EncodeArguments,
        R: EncodeConvert,
    {
        verify_message_signature::<A, R>(self, sel)
    }
}

// SAFETY: Class is immutable (and can be retrieved from any thread using the
// `class!` macro).
unsafe impl Sync for Class {}
unsafe impl Send for Class {}
impl UnwindSafe for Class {}
impl RefUnwindSafe for Class {}
// Note that Unpin is not applicable.

unsafe impl RefEncode for Class {
    const ENCODING_REF: Encoding = Encoding::Class;
}

impl fmt::Debug for Class {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl Protocol {
    pub(crate) fn as_ptr(&self) -> *const ffi::objc_protocol {
        let ptr: *const Self = self;
        ptr.cast()
    }

    /// Returns the protocol definition of a specified protocol, or [`None`]
    /// if the protocol is not registered with the Objective-C runtime.
    pub fn get(name: &str) -> Option<&'static Protocol> {
        let name = CString::new(name).unwrap();
        unsafe {
            let proto = ffi::objc_getProtocol(name.as_ptr());
            proto.cast::<Self>().as_ref()
        }
    }

    /// Obtains the list of registered protocol definitions.
    #[cfg(feature = "malloc")]
    pub fn protocols() -> Malloc<[&'static Protocol]> {
        unsafe {
            let mut count: c_uint = 0;
            let protocols: *mut &Protocol = ffi::objc_copyProtocolList(&mut count).cast();
            Malloc::from_array(protocols, count as usize)
        }
    }

    /// Get a list of the protocols to which this protocol conforms.
    #[cfg(feature = "malloc")]
    pub fn adopted_protocols(&self) -> Malloc<[&Protocol]> {
        unsafe {
            let mut count: c_uint = 0;
            let protocols: *mut &Protocol =
                ffi::protocol_copyProtocolList(self.as_ptr(), &mut count).cast();
            Malloc::from_array(protocols, count as usize)
        }
    }

    /// Checks whether this protocol conforms to the specified protocol.
    pub fn conforms_to(&self, proto: &Protocol) -> bool {
        unsafe {
            Bool::from_raw(ffi::protocol_conformsToProtocol(
                self.as_ptr(),
                proto.as_ptr(),
            ))
            .as_bool()
        }
    }

    /// Returns the name of self.
    pub fn name(&self) -> &str {
        let name = unsafe { CStr::from_ptr(ffi::protocol_getName(self.as_ptr())) };
        str::from_utf8(name.to_bytes()).unwrap()
    }
}

impl PartialEq for Protocol {
    /// Check whether the protocols are equal, or conform to each other.
    #[inline]
    fn eq(&self, other: &Protocol) -> bool {
        unsafe { Bool::from_raw(ffi::protocol_isEqual(self.as_ptr(), other.as_ptr())).as_bool() }
    }
}

unsafe impl RefEncode for Protocol {
    // Protocol is an object internally
    const ENCODING_REF: Encoding = Encoding::Object;
}

impl Eq for Protocol {}

impl fmt::Debug for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

// SAFETY: Protocol is immutable (and can be retrieved from Class anyhow).
unsafe impl Sync for Protocol {}
unsafe impl Send for Protocol {}
impl UnwindSafe for Protocol {}
impl RefUnwindSafe for Protocol {}
// Note that Unpin is not applicable.

pub(crate) fn ivar_offset(cls: &Class, name: &str, expected: &Encoding) -> isize {
    match cls.instance_variable(name) {
        Some(ivar) => {
            let encoding = ivar.type_encoding();
            assert!(
                expected.equivalent_to_str(encoding),
                "wrong encoding. Tried to retrieve ivar with encoding {}, but the encoding of the given type was {}",
                encoding,
                expected,
            );
            ivar.offset()
        }
        None => panic!("Ivar {} not found on class {:?}", name, cls),
    }
}

/// An Objective-C object.
///
/// This is slightly different from `NSObject` in that it may represent an
/// instance of an _arbitary_ Objective-C class (e.g. it does not have to be
/// a subclass of `NSObject`).
///
/// `Id<Object, _>` is equivalent to Objective-C's `id`.
///
/// This contains [`UnsafeCell`], and is similar to that in that one can
/// safely access and perform interior mutability on this (both via.
/// [`msg_send!`] and through ivars), so long as Rust's mutability rules are
/// upheld, and that data races are avoided.
///
/// Note: This is intentionally neither [`Sync`], [`Send`], [`UnwindSafe`],
/// [`RefUnwindSafe`] nor [`Unpin`], since that is something that may change
/// depending on the specific subclass. For example, `NSAutoreleasePool` is
/// not `Send`, it has to be deallocated on the same thread that it was
/// created. `NSLock` is not `Send` either.
///
/// [`msg_send!`]: crate::msg_send
#[doc(alias = "id")]
#[repr(C)]
pub struct Object(ffi::objc_object);

unsafe impl RefEncode for Object {
    const ENCODING_REF: Encoding = Encoding::Object;
}

impl Object {
    pub(crate) fn as_ptr(&self) -> *const ffi::objc_object {
        let ptr: *const Self = self;
        ptr.cast()
    }

    /// Dynamically find the class of this object.
    pub fn class(&self) -> &Class {
        let ptr: *const Class = unsafe { ffi::object_getClass(self.as_ptr()) }.cast();
        unsafe { ptr.as_ref().unwrap_unchecked() }
    }

    /// Offset an object pointer to get a pointer to an ivar.
    ///
    ///
    /// # Safety
    ///
    /// The offset must be valid for the given type.
    #[inline]
    pub(crate) unsafe fn ivar_at_offset<T>(ptr: NonNull<Self>, offset: isize) -> NonNull<T> {
        // `offset` is given in bytes, so we convert to `u8` and back to `T`
        let ptr: NonNull<u8> = ptr.cast();
        let ptr: *mut u8 = ptr.as_ptr();
        // SAFETY: The offset is valid
        let ptr: *mut u8 = unsafe { ptr.offset(offset) };
        // SAFETY: The offset operation is guaranteed to not end up computing
        // a NULL pointer.
        let ptr: NonNull<u8> = unsafe { NonNull::new_unchecked(ptr) };
        let ptr: NonNull<T> = ptr.cast();
        ptr
    }

    /// Returns a pointer to the instance variable / ivar with the given name.
    ///
    /// This is similar to [`UnsafeCell::get`], see that for more information
    /// on what is and isn't safe to do.
    ///
    /// Usually you will have defined the instance variable yourself with
    /// [`ClassBuilder::add_ivar`], the type of the ivar `T` must match the
    /// type used in that.
    ///
    /// Attempting to access or modify private implementation details of a
    /// class that you do no control using this is not supported, and may
    /// invoke undefined behaviour.
    ///
    /// Library implementors are strongly encouraged to expose a safe
    /// interface to the ivar.
    ///
    /// [`ClassBuilder::add_ivar`]: crate::declare::ClassBuilder::add_ivar
    ///
    ///
    /// # Panics
    ///
    /// May panic if the object has no ivar with the given name. May also
    /// panic if the type encoding of the ivar differs from the type encoding
    /// of `T`.
    ///
    /// This should purely seen as help while debugging and is not guaranteed
    /// (e.g. it may be disabled when `debug_assertions` are off).
    ///
    ///
    /// # Safety
    ///
    /// The object must have an instance variable with the given name, and it
    /// must be of type `T`. Any invariants that the object have assumed about
    /// the value of the instance variable must not be violated.
    ///
    /// No thread syncronization is done on accesses to the variable, so you
    /// must ensure that any access to the returned pointer do not cause data
    /// races, and that Rust's mutability rules are not otherwise violated.
    pub unsafe fn ivar_ptr<T: Encode>(&self, name: &str) -> *mut T {
        let offset = ivar_offset(self.class(), name, &T::ENCODING);

        let ptr = NonNull::from(self);
        // SAFETY: The offset is valid
        let ptr = unsafe { Self::ivar_at_offset::<T>(ptr, offset) };

        // Safe as *mut T because `self` is `UnsafeCell`
        ptr.as_ptr()
    }

    /// Returns a reference to the instance variable with the given name.
    ///
    /// See [`Object::ivar_ptr`] for more information, including on when this
    /// panics.
    ///
    ///
    /// # Safety
    ///
    /// The object must have an instance variable with the given name, and it
    /// must be of type `T`.
    ///
    /// No thread syncronization is done, so you must ensure that no other
    /// thread is concurrently mutating the variable. This requirement can be
    /// considered upheld if all mutation happens through [`Object::ivar_mut`]
    /// (since that  takes `&mut self`).
    pub unsafe fn ivar<T: Encode>(&self, name: &str) -> &T {
        // SAFETY: Upheld by caller.
        unsafe { self.ivar_ptr::<T>(name).as_ref().unwrap_unchecked() }
    }

    /// Use [`Object::ivar`] instead.
    ///
    ///
    /// # Safety
    ///
    /// See [`Object::ivar`].
    #[deprecated = "Use `Object::ivar` instead."]
    pub unsafe fn get_ivar<T: Encode>(&self, name: &str) -> &T {
        // SAFETY: Upheld by caller
        unsafe { self.ivar::<T>(name) }
    }

    /// Returns a mutable reference to the ivar with the given name.
    ///
    /// See [`Object::ivar_ptr`] for more information, including on when this
    /// panics.
    ///
    ///
    /// # Safety
    ///
    /// The object must have an instance variable with the given name, and it
    /// must be of type `T`.
    ///
    /// This access happens through `&mut self`, which means we know it to be
    /// the only reference, hence you do not need to do any work to ensure
    /// that data races do not happen.
    pub unsafe fn ivar_mut<T: Encode>(&mut self, name: &str) -> &mut T {
        let offset = ivar_offset(self.class(), name, &T::ENCODING);

        let ptr = NonNull::from(self);
        // SAFETY: The offset is valid
        let mut ptr = unsafe { Self::ivar_at_offset::<T>(ptr, offset) };

        // SAFETY:
        unsafe { ptr.as_mut() }
    }

    /// Use [`Object::ivar_mut`] instead.
    ///
    ///
    /// # Safety
    ///
    /// Same as [`Object::ivar_mut`].
    #[deprecated = "Use `Object::ivar_mut` instead."]
    pub unsafe fn get_mut_ivar<T: Encode>(&mut self, name: &str) -> &mut T {
        // SAFETY: Upheld by caller
        unsafe { self.ivar_mut::<T>(name) }
    }

    /// Sets the value of the ivar with the given name.
    ///
    /// This is just a helpful shorthand for [`Object::ivar_mut`], see that
    /// for more information.
    ///
    ///
    /// # Safety
    ///
    /// Same as [`Object::ivar_mut`].
    pub unsafe fn set_ivar<T: Encode>(&mut self, name: &str, value: T) {
        // SAFETY: Invariants upheld by caller
        unsafe { *self.ivar_mut::<T>(name) = value };
    }

    // objc_setAssociatedObject
    // objc_getAssociatedObject
    // objc_removeAssociatedObjects
}

impl fmt::Debug for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{:?}: {:p}>", self.class(), self.as_ptr())
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;
    use alloc::string::ToString;

    use super::{Bool, Class, Imp, Ivar, Method, Object, Protocol, Sel};
    use crate::test_utils;
    use crate::Encode;
    use crate::{msg_send, sel};

    #[test]
    fn test_selector() {
        macro_rules! test_sel {
            ($s:literal, $($tt:tt)+) => {{
                let sel = sel!($($tt)*);
                let expected = Sel::register($s);
                assert_eq!(sel, expected);
                assert_eq!(sel.name(), $s);
            }}
        }
        test_sel!("abc", abc);
        test_sel!("abc:", abc:);
        test_sel!("abc:def:", abc:def:);
        test_sel!("abc:def:ghi:", abc:def:ghi:);
    }

    #[test]
    fn test_empty_selector() {
        let sel = Sel::register("");
        assert_eq!(sel.name(), "");
        let sel = Sel::register(":");
        assert_eq!(sel.name(), ":");
    }

    #[test]
    #[should_panic = "NulError"]
    fn test_sel_register_null() {
        let _ = Sel::register("\0");
    }

    #[test]
    fn test_ivar() {
        let cls = test_utils::custom_class();
        let ivar = cls.instance_variable("_foo").unwrap();
        assert_eq!(ivar.name(), "_foo");
        assert!(<u32>::ENCODING.equivalent_to_str(ivar.type_encoding()));
        assert!(ivar.offset() > 0);

        #[cfg(feature = "malloc")]
        assert!(cls.instance_variables().len() > 0);
    }

    #[test]
    fn test_method() {
        let cls = test_utils::custom_class();
        let sel = Sel::register("foo");
        let method = cls.instance_method(sel).unwrap();
        assert_eq!(method.name().name(), "foo");
        assert_eq!(method.arguments_count(), 2);
        #[cfg(feature = "malloc")]
        {
            assert!(<u32>::ENCODING.equivalent_to_str(&method.return_type()));
            assert!(Sel::ENCODING.equivalent_to_str(&method.argument_type(1).unwrap()));

            let methods = cls.instance_methods();
            assert!(methods.len() > 0);
        }
    }

    #[test]
    fn test_class() {
        let cls = test_utils::custom_class();
        assert_eq!(cls.name(), "CustomObject");
        assert!(cls.instance_size() > 0);
        assert!(cls.superclass().is_none());

        assert!(cls.responds_to(sel!(foo)));
        assert!(cls.responds_to(sel!(setBar:)));
        assert!(!cls.responds_to(sel!(abc)));
        assert!(!cls.responds_to(sel!(addNumber:toNumber:)));

        assert_eq!(Class::get(cls.name()), Some(cls));

        let metaclass = cls.metaclass();
        // The metaclass of a root class is a subclass of the root class
        assert_eq!(metaclass.superclass().unwrap(), cls);
        assert!(metaclass.responds_to(sel!(addNumber:toNumber:)));
        // TODO: This is unexpected!
        assert!(metaclass.responds_to(sel!(foo)));

        let subclass = test_utils::custom_subclass();
        assert_eq!(subclass.superclass().unwrap(), cls);
    }

    #[test]
    fn test_classes_count() {
        assert!(Class::classes_count() > 0);
    }

    #[test]
    #[cfg(feature = "malloc")]
    fn test_classes() {
        let classes = Class::classes();
        assert!(classes.len() > 0);
    }

    #[test]
    fn test_protocol() {
        let proto = test_utils::custom_protocol();
        assert_eq!(proto.name(), "CustomProtocol");
        let class = test_utils::custom_class();
        assert!(class.conforms_to(proto));
        #[cfg(feature = "malloc")]
        assert!(class.adopted_protocols().len() > 0);
    }

    #[test]
    fn test_protocol_method() {
        let class = test_utils::custom_class();
        let result: i32 = unsafe { msg_send![class, addNumber: 1, toNumber: 2] };
        assert_eq!(result, 3);
    }

    #[test]
    fn test_subprotocols() {
        let sub_proto = test_utils::custom_subprotocol();
        let super_proto = test_utils::custom_protocol();
        assert!(sub_proto.conforms_to(super_proto));
        #[cfg(feature = "malloc")]
        assert_eq!(sub_proto.adopted_protocols()[0], super_proto);
    }

    #[test]
    fn test_protocols() {
        // Ensure that a protocol has been registered on linux
        let _ = test_utils::custom_protocol();

        #[cfg(feature = "malloc")]
        assert!(Protocol::protocols().len() > 0);
    }

    #[test]
    fn test_object() {
        let mut obj = test_utils::custom_object();
        assert_eq!(obj.class(), test_utils::custom_class());

        let result = unsafe {
            obj.set_ivar::<u32>("_foo", 4);
            *obj.ivar::<u32>("_foo")
        };
        assert_eq!(result, 4);
    }

    #[test]
    #[should_panic = "Ivar unknown not found on class CustomObject"]
    fn test_object_ivar_unknown() {
        let obj = test_utils::custom_object();
        let _ = unsafe { *obj.ivar::<u32>("unknown") };
    }

    #[test]
    #[should_panic = "wrong encoding. Tried to retrieve ivar with encoding I, but the encoding of the given type was C"]
    fn test_object_ivar_wrong_type() {
        let obj = test_utils::custom_object();
        let _ = unsafe { *obj.ivar::<u8>("_foo") };
    }

    #[test]
    fn test_encode() {
        fn assert_enc<T: Encode>(expected: &str) {
            assert_eq!(&T::ENCODING.to_string(), expected);
        }
        assert_enc::<&Object>("@");
        assert_enc::<*mut Object>("@");
        assert_enc::<&Class>("#");
        assert_enc::<Sel>(":");
        assert_enc::<Imp>("^?");
        assert_enc::<Option<Imp>>("^?");
        assert_enc::<&Protocol>("@");
    }

    #[test]
    fn test_send_sync() {
        fn assert_send_sync<T: Send + Sync + ?Sized>() {}
        assert_send_sync::<Bool>();
        assert_send_sync::<Class>();
        assert_send_sync::<Ivar>();
        assert_send_sync::<Method>();
        assert_send_sync::<Protocol>();
        assert_send_sync::<Sel>();
    }

    #[test]
    fn test_debug() {
        assert_eq!(format!("{:?}", sel!(abc:)), "abc:");
        let cls = test_utils::custom_class();
        assert_eq!(format!("{:?}", cls), "CustomObject");
        let protocol = test_utils::custom_protocol();
        assert_eq!(format!("{:?}", protocol), "CustomProtocol");

        let object = test_utils::custom_object();
        assert_eq!(
            format!("{:?}", &*object),
            format!("<CustomObject: {:p}>", &*object)
        );
    }
}
