use core::fmt;
use core::marker::PhantomData;
use core::mem::{self, ManuallyDrop};
use core::ops::{Deref, DerefMut};
use core::panic::{RefUnwindSafe, UnwindSafe};
use core::ptr::NonNull;

use super::Allocated;
use super::AutoreleasePool;
use super::{Owned, Ownership, Shared};
use crate::ffi;
use crate::{ClassType, Message};

/// An pointer for Objective-C reference counted objects.
///
/// [`Id`] strongly references or "retains" the given object `T`, and
/// "releases" it again when dropped, thereby ensuring it will be deallocated
/// at the right time.
///
/// An [`Id`] can either be [`Owned`] or [`Shared`], represented with the `O`
/// type parameter.
///
/// If owned, it is guaranteed that there are no other references to the
/// object, and the [`Id`] can therefore be mutably dereferenced.
///
/// If shared, however, it can only be immutably dereferenced because there
/// may be other references to the object, since a shared [`Id`] can be cloned
/// to provide exactly that.
///
/// An [`Id<T, Owned>`] can be safely converted to a [`Id<T, Shared>`] using
/// [`Id::into_shared`] or `From`/`Into`. The opposite is not safely possible,
/// but the unsafe option [`Id::from_shared`] is provided.
///
/// `Option<Id<T, O>>` is guaranteed to have the same size as a pointer to the
/// object.
///
///
/// # Comparison to `std` types
///
/// `Id<T, Owned>` can be thought of as the Objective-C equivalent of [`Box`]
/// from the standard library: It is a unique pointer to some allocated
/// object, and that means you're allowed to get a mutable reference to it.
///
/// Likewise, `Id<T, Shared>` is the Objective-C equivalent of [`Arc`]: It is
/// a reference-counting pointer that, when cloned, increases the reference
/// count.
///
/// [`Box`]: alloc::boxed::Box
/// [`Arc`]: alloc::sync::Arc
///
/// # Caveats
///
/// If the inner type implements [`Drop`], that implementation will not be
/// called, since there is no way to ensure that the Objective-C runtime will
/// do so. If you need to run some code when the object is destroyed,
/// implement the `dealloc` method instead.
///
/// This allows `?Sized` types `T`, but the intention is to only support when
/// `T` is an `extern type` (yet unstable).
///
/// # Examples
///
/// ```no_run
/// use objc2::msg_send_id;
/// use objc2::runtime::{Class, Object};
/// use objc2::rc::{Id, Owned, Shared, WeakId};
///
/// let cls = Class::get("NSObject").unwrap();
/// let obj: Id<Object, Owned> = unsafe { msg_send_id![cls, new] };
/// // obj will be released when it goes out of scope
///
/// // share the object so we can clone it
/// let obj: Id<_, Shared> = obj.into();
/// let another_ref = obj.clone();
/// // dropping our other reference will decrement the retain count
/// drop(another_ref);
///
/// let weak = WeakId::new(&obj);
/// assert!(weak.load().is_some());
/// // After the object is deallocated, our weak pointer returns none
/// drop(obj);
/// assert!(weak.load().is_none());
/// ```
///
/// ```no_run
/// # use objc2::{class, msg_send_id};
/// # use objc2::runtime::Object;
/// # use objc2::rc::{Id, Owned, Shared};
/// # type T = Object;
/// let mut owned: Id<T, Owned>;
/// # owned = unsafe { msg_send_id![class!(NSObject), new] };
/// let mut_ref: &mut T = &mut *owned;
/// // Do something with `&mut T` here
///
/// let shared: Id<T, Shared> = owned.into();
/// let cloned: Id<T, Shared> = shared.clone();
/// // Do something with `&T` here
/// ```
#[repr(transparent)]
// TODO: Figure out if `Message` bound on `T` would be better here?
// TODO: Add `ptr::Thin` bound on `T` to allow for only extern types
// TODO: Consider changing the name of Id -> Retain
pub struct Id<T: ?Sized, O: Ownership> {
    /// A pointer to the contained object. The pointer is always retained.
    ///
    /// It is important that this is `NonNull`, since we want to dereference
    /// it later, and be able to use the null-pointer optimization.
    ///
    /// Additionally, covariance is correct because we're either the unique
    /// owner of `T` (O = Owned), or `T` is immutable (O = Shared).
    ptr: NonNull<T>,
    /// Necessary for dropck even though we never actually run T's destructor,
    /// because it might have a `dealloc` that assumes that contained
    /// references outlive the type.
    ///
    /// See <https://doc.rust-lang.org/nightly/nomicon/phantom-data.html>
    item: PhantomData<T>,
    /// To prevent warnings about unused type parameters.
    own: PhantomData<O>,
    /// Marks the type as !UnwindSafe. Later on we'll re-enable this.
    ///
    /// See <https://github.com/rust-lang/rust/issues/93367> for why this is
    /// required.
    notunwindsafe: PhantomData<&'static mut ()>,
}

impl<T: ?Sized, O: Ownership> Id<T, O> {
    #[inline]
    unsafe fn new_nonnull(ptr: NonNull<T>) -> Self {
        Self {
            ptr,
            item: PhantomData,
            own: PhantomData,
            notunwindsafe: PhantomData,
        }
    }
}

impl<T: Message + ?Sized, O: Ownership> Id<Allocated<T>, O> {
    #[inline]
    pub(crate) unsafe fn new_allocated(ptr: *mut T) -> Option<Self> {
        // SAFETY: Upheld by the caller
        NonNull::new(ptr as *mut Allocated<T>).map(|ptr| unsafe { Self::new_nonnull(ptr) })
    }

    #[inline]
    pub(crate) unsafe fn assume_init(this: Self) -> Id<T, O> {
        let ptr = ManuallyDrop::new(this).ptr;

        // NonNull::cast
        let ptr = ptr.as_ptr() as *mut T;
        let ptr = unsafe { NonNull::new_unchecked(ptr) };

        // SAFETY: The pointer is valid.
        // Caller verifies that the object is allocated.
        unsafe { Id::new_nonnull(ptr) }
    }
}

impl<T: Message + ?Sized, O: Ownership> Id<T, O> {
    /// Constructs an [`Id`] to an object that already has +1 retain count.
    ///
    /// This is useful when you have a retain count that has been handed off
    /// from somewhere else, usually Objective-C methods like `init`, `alloc`,
    /// `new`, `copy`, or methods with the `ns_returns_retained` attribute.
    ///
    /// Since most of the above methods create new objects, and you therefore
    /// hold unique access to the object, you would often set the ownership to
    /// be [`Owned`].
    ///
    /// But some immutable objects (like `NSString`) don't always return
    /// unique references, so in those case you would use [`Shared`].
    ///
    /// Returns `None` if the pointer was null.
    ///
    ///
    /// # Safety
    ///
    /// The caller must ensure the given object has +1 retain count, and that
    /// the object pointer otherwise follows the same safety requirements as
    /// in [`Id::retain`].
    ///
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use objc2::{class, msg_send, msg_send_id};
    /// # use objc2::runtime::{Class, Object};
    /// # use objc2::rc::{Id, Owned};
    /// let cls: &Class;
    /// # let cls = class!(NSObject);
    /// let obj: &mut Object = unsafe { msg_send![cls, alloc] };
    /// let obj: Id<Object, Owned> = unsafe { Id::new(msg_send![obj, init]).unwrap() };
    /// // Or utilizing `msg_send_id`:
    /// let obj = unsafe { msg_send_id![cls, alloc] };
    /// let obj: Id<Object, Owned> = unsafe { msg_send_id![obj, init] };
    /// // Or in this case simply just:
    /// let obj: Id<Object, Owned> = unsafe { msg_send_id![cls, new] };
    /// ```
    ///
    /// ```no_run
    /// # use objc2::{class, msg_send_id};
    /// # use objc2::runtime::Object;
    /// # use objc2::rc::{Id, Shared};
    /// # type NSString = Object;
    /// let cls = class!(NSString);
    /// // NSString is immutable, so don't create an owned reference to it
    /// let obj: Id<NSString, Shared> = unsafe { msg_send_id![cls, new] };
    /// ```
    #[inline]
    // Note: We don't take a reference as a parameter since it would be too
    // easy to accidentally create two aliasing mutable references.
    pub unsafe fn new(ptr: *mut T) -> Option<Id<T, O>> {
        // Should optimize down to nothing.
        // SAFETY: Upheld by the caller
        NonNull::new(ptr).map(|ptr| unsafe { Id::new_nonnull(ptr) })
    }

    /// Returns a raw pointer to the object.
    ///
    /// The pointer is valid for at least as long as the `Id` is held.
    ///
    /// See [`Id::as_mut_ptr`] for the mutable equivalent.
    ///
    /// This is an associated method, and must be called as `Id::as_ptr(obj)`.
    #[inline]
    pub fn as_ptr(this: &Id<T, O>) -> *const T {
        this.ptr.as_ptr()
    }

    #[inline]
    pub(crate) fn consume_as_ptr(this: ManuallyDrop<Self>) -> *mut T {
        this.ptr.as_ptr()
    }

    #[inline]
    pub(crate) fn option_into_ptr(obj: Option<Self>) -> *mut T {
        // Difficult to write this in an ergonomic way with ?Sized
        // So we just hack it with transmute!

        // SAFETY: Option<Id<T, _>> has the same size as *mut T
        unsafe { mem::transmute::<ManuallyDrop<Option<Self>>, *mut T>(ManuallyDrop::new(obj)) }
    }
}

impl<T: Message + ?Sized> Id<T, Owned> {
    /// Returns a raw mutable pointer to the object.
    ///
    /// The pointer is valid for at least as long as the `Id` is held.
    ///
    /// See [`Id::as_ptr`] for the immutable equivalent.
    ///
    /// This is an associated method, and must be called as
    /// `Id::as_mut_ptr(obj)`.
    #[inline]
    pub fn as_mut_ptr(this: &mut Id<T, Owned>) -> *mut T {
        this.ptr.as_ptr()
    }
}

// TODO: Add ?Sized bound
impl<T: Message, O: Ownership> Id<T, O> {
    /// Convert the type of the given object to another.
    ///
    /// This is equivalent to a `cast` between two pointers.
    ///
    /// See [`Id::into_super`] for a safe alternative.
    ///
    /// This is common to do when you know that an object is a subclass of
    /// a specific class (e.g. casting an instance of `NSString` to `NSObject`
    /// is safe because `NSString` is a subclass of `NSObject`).
    ///
    /// All `'static` objects can safely be cast to [`Object`], since that
    /// assumes no specific class.
    ///
    /// [`Object`]: crate::runtime::Object
    ///
    ///
    /// # Safety
    ///
    /// You must ensure that the object can be reinterpreted as the given
    /// type.
    ///
    /// If `T` is not `'static`, you must ensure that `U` ensures that the
    /// data contained by `T` is kept alive for as long as `U` lives.
    ///
    /// Additionally, you must ensure that any safety invariants that the new
    /// type has are upheld.
    #[inline]
    pub unsafe fn cast<U: Message>(this: Self) -> Id<U, O> {
        let ptr = ManuallyDrop::new(this).ptr.cast();
        // SAFETY: The object is forgotten, so we have +1 retain count.
        //
        // Caller verifies that the returned object is of the correct type.
        unsafe { Id::new_nonnull(ptr) }
    }

    /// Retains the given object pointer.
    ///
    /// This is useful when you have been given a pointer to an object from
    /// some API, and you would like to ensure that the object stays around
    /// so that you can work with it.
    ///
    /// If said API is a normal Objective-C method, you probably want to use
    /// [`Id::retain_autoreleased`] instead.
    ///
    /// This is rarely used to construct owned [`Id`]s, see [`Id::new`] for
    /// that.
    ///
    /// Returns `None` if the pointer was null.
    ///
    ///
    /// # Safety
    ///
    /// The caller must ensure that the ownership is correct; that is, there
    /// must be no [`Owned`] pointers or mutable references to the same
    /// object, and when creating owned [`Id`]s, there must be no other
    /// pointers or references to the object.
    ///
    /// Additionally, the pointer must be valid as a reference (aligned,
    /// dereferencable and initialized, see the [`std::ptr`] module for more
    /// information).
    ///
    /// Finally, if you do not know the concrete type of `T`, it may not be
    /// `'static`, and hence you must ensure that the data that `T` references
    /// lives for as long as `T`.
    ///
    /// [`std::ptr`]: core::ptr
    //
    // This would be illegal:
    // ```no_run
    // let owned: Id<T, Owned>;
    // // Lifetime information is discarded
    // let retained: Id<T, Shared> = unsafe { Id::retain(&*owned) };
    // // Which means we can still mutate `Owned`:
    // let x: &mut T = &mut *owned;
    // // While we have an immutable reference
    // let y: &T = &*retained;
    // ```
    #[doc(alias = "objc_retain")]
    #[inline]
    pub unsafe fn retain(ptr: *mut T) -> Option<Id<T, O>> {
        // SAFETY: The caller upholds that the pointer is valid
        let res: *mut T = unsafe { ffi::objc_retain(ptr.cast()) }.cast();
        debug_assert_eq!(res, ptr, "objc_retain did not return the same pointer");
        // SAFETY: We just retained the object, so it has +1 retain count
        unsafe { Self::new(res) }
    }

    /// Retains a previously autoreleased object pointer.
    ///
    /// This is useful when calling Objective-C methods that return
    /// autoreleased objects, see [Cocoa's Memory Management Policy][mmRules].
    ///
    /// This has exactly the same semantics as [`Id::retain`], except it can
    /// sometimes avoid putting the object into the autorelease pool, possibly
    /// yielding increased speed and reducing memory pressure.
    ///
    /// Note: This relies heavily on being inlined right after [`msg_send!`],
    /// be careful not accidentally require instructions between these.
    ///
    /// [mmRules]: https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/MemoryMgmt/Articles/mmRules.html
    /// [`msg_send!`]: crate::msg_send
    ///
    ///
    /// # Safety
    ///
    /// Same as [`Id::retain`].
    #[doc(alias = "objc_retainAutoreleasedReturnValue")]
    #[inline]
    pub unsafe fn retain_autoreleased(ptr: *mut T) -> Option<Id<T, O>> {
        // Add magic nop instruction to participate in the fast autorelease
        // scheme.
        //
        // See `callerAcceptsOptimizedReturn` in `objc-object.h`:
        // https://github.com/apple-oss-distributions/objc4/blob/objc4-838/runtime/objc-object.h#L1209-L1377
        //
        // We will unconditionally emit these instructions, even if they end
        // up being unused (for example because we're unlucky with inlining,
        // some other work is done between the objc_msgSend and this, or the
        // runtime version is too old to support it).
        //
        // It may seem like there should be a better way to do this, but
        // emitting raw assembly is exactly what Clang and Swift does:
        // swiftc: https://github.com/apple/swift/blob/swift-5.5.3-RELEASE/lib/IRGen/GenObjC.cpp#L148-L173
        // Clang: https://github.com/llvm/llvm-project/blob/889317d47b7f046cf0e68746da8f7f264582fb5b/clang/lib/CodeGen/CGObjC.cpp#L2339-L2373
        //
        // Resources:
        // - https://www.mikeash.com/pyblog/friday-qa-2011-09-30-automatic-reference-counting.html
        // - https://www.galloway.me.uk/2012/02/how-does-objc_retainautoreleasedreturnvalue-work/
        // - https://github.com/gfx-rs/metal-rs/issues/222
        // - https://news.ycombinator.com/item?id=29311736
        // - https://stackoverflow.com/a/23765612
        //
        // SAFETY:
        // Based on https://doc.rust-lang.org/stable/reference/inline-assembly.html#rules-for-inline-assembly
        //
        // We don't care about the value of the register (so it's okay to be
        // undefined), and its value is preserved.
        //
        // nomem: No reads or writes to memory are performed (this `mov`
        //   operates entirely on registers).
        // preserves_flags: `mov` doesn't modify any flags.
        // nostack: We don't touch the stack.

        // Only worth doing on the Apple runtime.
        // Not supported on TARGET_OS_WIN32.
        #[cfg(all(feature = "apple", not(target_os = "windows")))]
        {
            // Supported since macOS 10.7.
            #[cfg(target_arch = "x86_64")]
            {
                // x86_64 looks at the next call instruction.
                //
                // This is expected to be a PLT entry - if the user specifies
                // `-Zplt=no`, a GOT entry will be created instead, and this
                // will not work.
            }

            // Supported since macOS 10.8.
            #[cfg(target_arch = "arm")]
            unsafe {
                core::arch::asm!("mov r7, r7", options(nomem, preserves_flags, nostack))
            };

            // Supported since macOS 10.10.
            #[cfg(target_arch = "aarch64")]
            unsafe {
                core::arch::asm!("mov fp, fp", options(nomem, preserves_flags, nostack))
            };

            // Supported since macOS 10.12.
            #[cfg(target_arch = "x86")]
            unsafe {
                core::arch::asm!("mov ebp, ebp", options(nomem, preserves_flags, nostack))
            };
        }

        // SAFETY: Same as `retain`, this is just an optimization.
        let res: *mut T = unsafe { ffi::objc_retainAutoreleasedReturnValue(ptr.cast()) }.cast();

        // Ideally, we'd be able to specify that the above call should never
        // be tail-call optimized (become a `jmp` instruction instead of a
        // `call`); Rust doesn't really have a way of doing this currently, so
        // we just emit a simple `nop` to make such tail-call optimizations
        // less likely to occur.
        //
        // This is brittle! We should find a better solution!
        #[cfg(all(feature = "apple", not(target_os = "windows"), target_arch = "x86_64"))]
        {
            // SAFETY: Similar to above.
            unsafe { core::arch::asm!("nop", options(nomem, preserves_flags, nostack)) };
            // TODO: Possibly more efficient alternative? Also consider PLT.
            // #![feature(asm_sym)]
            // core::arch::asm!(
            //     "mov rdi, rax",
            //     "call {}",
            //     sym objc2::ffi::objc_retainAutoreleasedReturnValue,
            //     inout("rax") obj,
            //     clobber_abi("C"),
            // );
        }

        debug_assert_eq!(
            res, ptr,
            "objc_retainAutoreleasedReturnValue did not return the same pointer"
        );
        unsafe { Self::new(res) }
    }

    #[inline]
    fn autorelease_inner(self) -> *mut T {
        // Note that this (and the actual `autorelease`) is not an associated
        // function. This breaks the guideline that smart pointers shouldn't
        // add inherent methods, but since autoreleasing only works on already
        // retained objects it is hard to imagine a case where the inner type
        // has a method with the same name.

        let ptr = ManuallyDrop::new(self).ptr.as_ptr();
        // SAFETY: The `ptr` is guaranteed to be valid and have at least one
        // retain count.
        // And because of the ManuallyDrop, we don't call the Drop
        // implementation, so the object won't also be released there.
        let res: *mut T = unsafe { ffi::objc_autorelease(ptr.cast()) }.cast();
        debug_assert_eq!(res, ptr, "objc_autorelease did not return the same pointer");
        res
    }

    /// Autoreleases and prepares the [`Id`] to be returned to Objective-C.
    ///
    /// The object is not immediately released, but will be when the innermost
    /// autorelease pool is drained.
    ///
    /// This is useful when [declaring your own methods][declare] where you
    /// will often find yourself in need of returning autoreleased objects to
    /// properly follow [Cocoa's Memory Management Policy][mmRules].
    ///
    /// To that end, you could use [`Id::autorelease`], but that would require
    /// you to have an [`AutoreleasePool`] object at hand, which you clearly
    /// won't have in such cases. This function doesn't require a `pool`
    /// object (but as a downside returns a pointer instead of a reference).
    ///
    /// This is also more efficient than a normal `autorelease`, it makes a
    /// best effort attempt to hand off ownership of the retain count to a
    /// subsequent call to `objc_retainAutoreleasedReturnValue` /
    /// [`Id::retain_autoreleased`] in the enclosing call frame. Note: This
    /// optimization relies heavily on this function being tail called, so be
    /// careful to call this function at the end of your method.
    ///
    /// [declare]: crate::declare
    /// [mmRules]: https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/MemoryMgmt/Articles/mmRules.html
    ///
    ///
    /// # Examples
    ///
    /// ```
    /// use objc2::{class, msg_send_id, sel};
    /// use objc2::declare::ClassBuilder;
    /// use objc2::rc::{Id, Owned};
    /// use objc2::runtime::{Class, Object, Sel};
    /// #
    /// # #[cfg(feature = "gnustep-1-7")]
    /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
    ///
    /// let mut builder = ClassBuilder::new("ExampleObject", class!(NSObject)).unwrap();
    ///
    /// extern "C" fn get(cls: &Class, _cmd: Sel) -> *mut Object {
    ///     let obj: Id<Object, Owned> = unsafe { msg_send_id![cls, new] };
    ///     obj.autorelease_return()
    /// }
    ///
    /// unsafe {
    ///     builder.add_class_method(
    ///         sel!(get),
    ///         get as extern "C" fn(_, _) -> _,
    ///     );
    /// }
    ///
    /// let cls = builder.register();
    /// ```
    #[doc(alias = "objc_autoreleaseReturnValue")]
    #[must_use = "If you don't intend to use the object any more, just drop it as usual"]
    #[inline]
    pub fn autorelease_return(self) -> *mut T {
        // See `autorelease_inner` for why this is an inherent method

        let ptr = ManuallyDrop::new(self).ptr.as_ptr();
        // SAFETY: Same as `autorelease_inner`, this is just an optimization.
        let res: *mut T = unsafe { ffi::objc_autoreleaseReturnValue(ptr.cast()) }.cast();
        debug_assert_eq!(
            res, ptr,
            "objc_autoreleaseReturnValue did not return the same pointer"
        );
        res
    }
}

// TODO: Consider something like this
// #[cfg(block)]
// impl<T: Block, O> Id<T, O> {
//     #[doc(alias = "objc_retainBlock")]
//     pub unsafe fn retain_block(block: *mut T) -> Option<Self> {
//         todo!()
//     }
// }

// TODO: Add ?Sized bound
impl<T: Message> Id<T, Owned> {
    /// Autoreleases the owned [`Id`], returning a mutable reference bound to
    /// the pool.
    ///
    /// The object is not immediately released, but will be when the innermost
    /// / current autorelease pool (given as a parameter) is drained.
    #[doc(alias = "objc_autorelease")]
    #[must_use = "If you don't intend to use the object any more, just drop it as usual"]
    #[inline]
    #[allow(clippy::needless_lifetimes)]
    #[allow(clippy::mut_from_ref)]
    pub fn autorelease<'p>(self, pool: &'p AutoreleasePool) -> &'p mut T {
        let ptr = self.autorelease_inner();
        // SAFETY: The pointer is valid as a reference, and we've consumed
        // the unique access to the `Id` so mutability is safe.
        unsafe { pool.ptr_as_mut(ptr) }
    }

    /// Promote a shared [`Id`] to an owned one, allowing it to be mutated.
    ///
    ///
    /// # Safety
    ///
    /// The caller must ensure that there are no other pointers (including
    /// [`WeakId`][`super::WeakId`] pointers) to the same object.
    ///
    /// This also means that the given [`Id`] should have a retain count of
    /// exactly 1 (except when autoreleases are involved).
    ///
    /// In general, this is wildly unsafe, do see if you can find a different
    /// solution!
    #[inline]
    pub unsafe fn from_shared(obj: Id<T, Shared>) -> Self {
        // Note: We can't debug_assert retainCount because of autoreleases
        let ptr = ManuallyDrop::new(obj).ptr;
        // SAFETY: The pointer is valid
        // Ownership rules are upheld by the caller
        unsafe { <Id<T, Owned>>::new_nonnull(ptr) }
    }

    /// Convert an owned to a shared [`Id`], allowing it to be cloned.
    ///
    /// This is also implemented as a `From` conversion, but this name is more
    /// explicit, which may be useful in some cases.
    #[inline]
    pub fn into_shared(obj: Self) -> Id<T, Shared> {
        let ptr = ManuallyDrop::new(obj).ptr;
        // SAFETY: The pointer is valid, and ownership is simply decreased
        unsafe { <Id<T, Shared>>::new_nonnull(ptr) }
    }
}

// TODO: Add ?Sized bound
impl<T: Message> Id<T, Shared> {
    /// Autoreleases the shared [`Id`], returning an aliased reference bound
    /// to the pool.
    ///
    /// The object is not immediately released, but will be when the innermost
    /// / current autorelease pool (given as a parameter) is drained.
    #[doc(alias = "objc_autorelease")]
    #[must_use = "If you don't intend to use the object any more, just drop it as usual"]
    #[inline]
    #[allow(clippy::needless_lifetimes)]
    pub fn autorelease<'p>(self, pool: &'p AutoreleasePool) -> &'p T {
        let ptr = self.autorelease_inner();
        // SAFETY: The pointer is valid as a reference
        unsafe { pool.ptr_as_ref(ptr) }
    }
}

impl<T: ClassType + 'static, O: Ownership> Id<T, O>
where
    T::Super: 'static,
{
    /// Convert the object into it's superclass.
    #[inline]
    pub fn into_super(this: Self) -> Id<T::Super, O> {
        // SAFETY:
        // - The casted-to type is a superclass of the type.
        // - Both types are `'static` (this could maybe be relaxed a bit, but
        //   let's just be on the safe side)!
        unsafe { Self::cast::<T::Super>(this) }
    }
}

impl<T: Message> From<Id<T, Owned>> for Id<T, Shared> {
    /// Convert an owned to a shared [`Id`], allowing it to be cloned.
    ///
    /// Same as [`Id::into_shared`].
    #[inline]
    fn from(obj: Id<T, Owned>) -> Self {
        Id::into_shared(obj)
    }
}

// TODO: Add ?Sized bound
impl<T: Message> Clone for Id<T, Shared> {
    /// Makes a clone of the shared object.
    ///
    /// This increases the object's reference count.
    #[doc(alias = "objc_retain")]
    #[doc(alias = "retain")]
    #[inline]
    fn clone(&self) -> Self {
        // SAFETY: The pointer is valid
        let obj = unsafe { Id::retain(self.ptr.as_ptr()) };
        // SAFETY: `objc_retain` always returns the same object pointer, and
        // the pointer is guaranteed non-null by Id.
        unsafe { obj.unwrap_unchecked() }
    }
}

/// `#[may_dangle]` (see [this][dropck_eyepatch]) doesn't apply here since we
/// don't run `T`'s destructor (rather, we want to discourage having `T`s with
/// a destructor); and even if we did run the destructor, it would not be safe
/// to add since we cannot verify that a `dealloc` method doesn't access
/// borrowed data.
///
/// [dropck_eyepatch]: https://doc.rust-lang.org/nightly/nomicon/dropck.html#an-escape-hatch
impl<T: ?Sized, O: Ownership> Drop for Id<T, O> {
    /// Releases the retained object.
    ///
    /// The contained object's destructor (if it has one) is never run!
    #[doc(alias = "objc_release")]
    #[doc(alias = "release")]
    #[inline]
    fn drop(&mut self) {
        // We could technically run the destructor for `T` when `O = Owned`,
        // and when `O = Shared` with (retainCount == 1), but that would be
        // confusing and inconsistent since we cannot guarantee that it's run.

        // SAFETY: The `ptr` is guaranteed to be valid and have at least one
        // retain count
        unsafe { ffi::objc_release(self.ptr.as_ptr().cast()) };
    }
}

// https://doc.rust-lang.org/nomicon/arc-mutex/arc-base.html#send-and-sync
/// The `Send` implementation requires `T: Sync` because `Id<T, Shared>` give
/// access to `&T`.
///
/// Additiontally, it requires `T: Send` because if `T: !Send`, you could
/// clone a `Id<T, Shared>`, send it to another thread, and drop the clone
/// last, making `dealloc` get called on the other thread, and violate
/// `T: !Send`.
unsafe impl<T: Sync + Send + ?Sized> Send for Id<T, Shared> {}

/// The `Sync` implementation requires `T: Sync` because `&Id<T, Shared>` give
/// access to `&T`.
///
/// Additiontally, it requires `T: Send`, because if `T: !Send`, you could
/// clone a `&Id<T, Shared>` from another thread, and drop the clone last,
/// making `dealloc` get called on the other thread, and violate `T: !Send`.
unsafe impl<T: Sync + Send + ?Sized> Sync for Id<T, Shared> {}

/// `Id<T, Owned>` are `Send` if `T` is `Send` because they give the same
/// access as having a T directly.
unsafe impl<T: Send + ?Sized> Send for Id<T, Owned> {}

/// `Id<T, Owned>` are `Sync` if `T` is `Sync` because they give the same
/// access as having a `T` directly.
unsafe impl<T: Sync + ?Sized> Sync for Id<T, Owned> {}

impl<T: ?Sized, O: Ownership> Deref for Id<T, O> {
    type Target = T;

    /// Obtain an immutable reference to the object.
    // Box doesn't inline, but that's because it's a compiler built-in
    #[inline]
    fn deref(&self) -> &T {
        // SAFETY: The pointer's validity is verified when the type is created
        unsafe { self.ptr.as_ref() }
    }
}

impl<T: ?Sized> DerefMut for Id<T, Owned> {
    /// Obtain a mutable reference to the object.
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: The pointer's validity is verified when the type is created
        // Additionally, the owned `Id` is the unique owner of the object, so
        // mutability is safe.
        unsafe { self.ptr.as_mut() }
    }
}

impl<T: ?Sized, O: Ownership> fmt::Pointer for Id<T, O> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&self.ptr.as_ptr(), f)
    }
}

// This is valid without `T: Unpin` because we don't implement any projection.
//
// See https://doc.rust-lang.org/1.54.0/src/alloc/boxed.rs.html#1652-1675
// and the `Arc` implementation.
impl<T: ?Sized, O: Ownership> Unpin for Id<T, O> {}

impl<T: RefUnwindSafe + ?Sized, O: Ownership> RefUnwindSafe for Id<T, O> {}

// Same as `Arc<T>`.
impl<T: RefUnwindSafe + ?Sized> UnwindSafe for Id<T, Shared> {}

// Same as `Box<T>`.
impl<T: UnwindSafe + ?Sized> UnwindSafe for Id<T, Owned> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg_send;
    use crate::rc::{autoreleasepool, RcTestObject, ThreadTestData};
    use crate::runtime::Object;

    #[track_caller]
    fn assert_retain_count(obj: &Object, expected: usize) {
        let retain_count: usize = unsafe { msg_send![obj, retainCount] };
        assert_eq!(retain_count, expected);
    }

    #[test]
    fn test_drop() {
        let mut expected = ThreadTestData::current();

        let obj = RcTestObject::new();
        expected.alloc += 1;
        expected.init += 1;
        expected.assert_current();

        drop(obj);
        expected.release += 1;
        expected.dealloc += 1;
        expected.assert_current();
    }

    #[test]
    fn test_autorelease() {
        let obj: Id<_, Shared> = RcTestObject::new().into();
        let cloned = obj.clone();
        let mut expected = ThreadTestData::current();

        autoreleasepool(|pool| {
            let _ref = obj.autorelease(pool);
            expected.autorelease += 1;
            expected.assert_current();
            assert_retain_count(&cloned, 2);
        });
        expected.release += 1;
        expected.assert_current();
        assert_retain_count(&cloned, 1);

        autoreleasepool(|pool| {
            let _ref = cloned.autorelease(pool);
            expected.autorelease += 1;
            expected.assert_current();
        });
        expected.release += 1;
        expected.dealloc += 1;
        expected.assert_current();
    }

    #[test]
    fn test_clone() {
        let obj: Id<_, Owned> = RcTestObject::new();
        assert_retain_count(&obj, 1);
        let mut expected = ThreadTestData::current();

        let obj: Id<_, Shared> = obj.into();
        expected.assert_current();
        assert_retain_count(&obj, 1);

        let cloned = obj.clone();
        expected.retain += 1;
        expected.assert_current();
        assert_retain_count(&cloned, 2);
        assert_retain_count(&obj, 2);

        drop(obj);
        expected.release += 1;
        expected.assert_current();
        assert_retain_count(&cloned, 1);

        drop(cloned);
        expected.release += 1;
        expected.dealloc += 1;
        expected.assert_current();
    }

    #[test]
    fn test_retain_autoreleased_works_as_retain() {
        let obj: Id<_, Shared> = RcTestObject::new().into();
        let mut expected = ThreadTestData::current();

        let ptr = Id::as_ptr(&obj) as *mut RcTestObject;
        let _obj2: Id<_, Shared> = unsafe { Id::retain_autoreleased(ptr) }.unwrap();
        expected.retain += 1;
        expected.assert_current();
    }

    #[test]
    fn test_cast() {
        let obj: Id<RcTestObject, _> = RcTestObject::new();
        let expected = ThreadTestData::current();

        // SAFETY: Any object can be cast to `Object`
        let obj: Id<Object, _> = unsafe { Id::cast(obj) };
        expected.assert_current();

        // SAFETY: The object was originally `RcTestObject`
        let _obj: Id<RcTestObject, _> = unsafe { Id::cast(obj) };
        expected.assert_current();
    }
}
