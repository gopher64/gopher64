use alloc::boxed::Box;
use core::ffi::c_void;
use core::marker::PhantomData;
use core::ptr::NonNull;

use crate::encode::{EncodeConvert, Encoding};
use crate::rc::{Id, Ownership};
use crate::Message;

use super::InnerIvarType;

/// A helper type to allow putting certain types that may drop into ivars.
///
/// This is used to work around current limitations in the type system.
/// Consider this type "temporary" in the sense that one day it may just
/// become `type IvarDrop<T> = T`.
///
/// This currently works with the following types:
/// - `Box<T>`
/// - `Option<Box<T>>`
/// - `Id<T, O>`
/// - `Option<Id<T, O>>`
///
/// Further may be added when the standard library guarantees their layout.
///
/// See `examples/delegate.rs` for usage.
pub struct IvarDrop<T> {
    /// For proper variance and auto traits.
    p: PhantomData<T>,
}

impl<T: Sized> super::ivar::private::Sealed for IvarDrop<Box<T>> {}
// SAFETY: The memory layout of `Box<T: Sized>` is guaranteed to be a pointer:
// <https://doc.rust-lang.org/1.62.1/std/boxed/index.html#memory-layout>
//
// The user ensures that the Box has been initialized in an `init` method
// before being used.
unsafe impl<T: Sized> InnerIvarType for IvarDrop<Box<T>> {
    // Note that we use `*const c_void` and not `*const T` to allow _any_
    // type, not just types that can be encoded by Objective-C
    const __ENCODING: Encoding = <*const c_void as EncodeConvert>::__ENCODING;

    type __Inner = Option<Box<T>>;
    type Output = Box<T>;

    const __MAY_DROP: bool = true;

    #[inline]
    unsafe fn __to_ref(inner: &Self::__Inner) -> &Self::Output {
        match inner {
            Some(inner) => inner,
            None => unsafe { box_unreachable() },
        }
    }

    #[inline]
    unsafe fn __to_mut(inner: &mut Self::__Inner) -> &mut Self::Output {
        match inner {
            Some(inner) => inner,
            None => unsafe { box_unreachable() },
        }
    }

    #[inline]
    fn __to_ptr(inner: NonNull<Self::__Inner>) -> NonNull<Self::Output> {
        inner.cast()
    }
}

impl<T: Sized> super::ivar::private::Sealed for IvarDrop<Option<Box<T>>> {}
// SAFETY: `Option<Box<T>>` guarantees the null-pointer optimization, so for
// `T: Sized` the layout is just a pointer:
// <https://doc.rust-lang.org/1.62.1/std/option/index.html#representation>
//
// This is valid to initialize as all-zeroes, so the user doesn't have to do
// anything to initialize it.
unsafe impl<T: Sized> InnerIvarType for IvarDrop<Option<Box<T>>> {
    const __ENCODING: Encoding = <*const c_void as EncodeConvert>::__ENCODING;

    type __Inner = Option<Box<T>>;
    type Output = Option<Box<T>>;

    const __MAY_DROP: bool = true;

    #[inline]
    unsafe fn __to_ref(this: &Self::__Inner) -> &Self::Output {
        this
    }

    #[inline]
    unsafe fn __to_mut(this: &mut Self::__Inner) -> &mut Self::Output {
        this
    }

    #[inline]
    fn __to_ptr(inner: NonNull<Self::__Inner>) -> NonNull<Self::Output> {
        inner.cast()
    }
}

impl<T: Message, O: Ownership> super::ivar::private::Sealed for IvarDrop<Id<T, O>> {}
// SAFETY: `Id` is `NonNull<T>`, and hence safe to store as a pointer.
//
// The user ensures that the Id has been initialized in an `init` method
// before being used.
//
// Note: We could technically do `impl InnerIvarType for Ivar<Id<T, O>>`
// directly today, but since we can't do so for `Box` (because that is
// `#[fundamental]`), I think it makes sense to handle them similarly.
unsafe impl<T: Message, O: Ownership> InnerIvarType for IvarDrop<Id<T, O>> {
    const __ENCODING: Encoding = <*const T as EncodeConvert>::__ENCODING;

    type __Inner = Option<Id<T, O>>;
    type Output = Id<T, O>;

    const __MAY_DROP: bool = true;

    #[inline]
    unsafe fn __to_ref(inner: &Self::__Inner) -> &Self::Output {
        match inner {
            Some(inner) => inner,
            None => unsafe { id_unreachable() },
        }
    }

    #[inline]
    unsafe fn __to_mut(inner: &mut Self::__Inner) -> &mut Self::Output {
        match inner {
            Some(inner) => inner,
            None => unsafe { id_unreachable() },
        }
    }

    #[inline]
    fn __to_ptr(inner: NonNull<Self::__Inner>) -> NonNull<Self::Output> {
        inner.cast()
    }
}

impl<T: Message, O: Ownership> super::ivar::private::Sealed for IvarDrop<Option<Id<T, O>>> {}
// SAFETY: `Id<T, O>` guarantees the null-pointer optimization.
//
// This is valid to initialize as all-zeroes, so the user doesn't have to do
// anything to initialize it.
unsafe impl<T: Message, O: Ownership> InnerIvarType for IvarDrop<Option<Id<T, O>>> {
    const __ENCODING: Encoding = <*const T as EncodeConvert>::__ENCODING;

    type __Inner = Option<Id<T, O>>;
    type Output = Option<Id<T, O>>;

    const __MAY_DROP: bool = true;

    #[inline]
    unsafe fn __to_ref(this: &Self::__Inner) -> &Self::Output {
        this
    }

    #[inline]
    unsafe fn __to_mut(this: &mut Self::__Inner) -> &mut Self::Output {
        this
    }

    #[inline]
    fn __to_ptr(inner: NonNull<Self::__Inner>) -> NonNull<Self::Output> {
        inner.cast()
    }
}

// TODO: Allow the following once their layout is guaranteed by `std`:
// - Arc<T>
// - Option<Arc<T>>
// - sync::Weak<T>
// - Rc<T>
// - Option<Rc<T>>
// - rc::Weak<T>
// - Vec<T>
// - String

// TODO: Allow `WeakId` once we figure out how to allow it being initialized
// by default.

#[inline]
unsafe fn id_unreachable() -> ! {
    #[cfg(debug_assertions)]
    {
        unreachable!("an Id in instance variables must always be initialized before use!")
    }
    // SAFETY: Checked by caller
    #[cfg(not(debug_assertions))]
    unsafe {
        core::hint::unreachable_unchecked()
    }
}

#[inline]
unsafe fn box_unreachable() -> ! {
    #[cfg(debug_assertions)]
    {
        unreachable!("a Box in instance variables must always be initialized before use!")
    }
    // SAFETY: Checked by caller
    #[cfg(not(debug_assertions))]
    unsafe {
        core::hint::unreachable_unchecked()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::declare::{Ivar, IvarType};
    use crate::foundation::NSObject;
    use crate::rc::{Allocated, Owned, RcTestObject, Shared, ThreadTestData};
    use crate::runtime::Object;
    use crate::{declare_class, msg_send, msg_send_id, ClassType};

    struct TestIvar1;
    unsafe impl IvarType for TestIvar1 {
        type Type = IvarDrop<Box<u8>>;
        const NAME: &'static str = "_abc";
    }

    struct TestIvar2;
    unsafe impl IvarType for TestIvar2 {
        type Type = IvarDrop<Option<Box<u8>>>;
        const NAME: &'static str = "_abc";
    }

    struct TestIvar3;
    unsafe impl IvarType for TestIvar3 {
        type Type = IvarDrop<Id<Object, Shared>>;
        const NAME: &'static str = "_abc";
    }

    struct TestIvar4;
    unsafe impl IvarType for TestIvar4 {
        type Type = IvarDrop<Option<Id<Object, Owned>>>;
        const NAME: &'static str = "_abc";
    }

    declare_class!(
        #[derive(Debug, PartialEq)]
        struct IvarTester {
            ivar1: IvarDrop<Id<RcTestObject, Shared>>,
            ivar2: IvarDrop<Option<Id<RcTestObject, Owned>>>,
            ivar3: IvarDrop<Box<Id<RcTestObject, Owned>>>,
            ivar4: IvarDrop<Option<Box<Id<RcTestObject, Owned>>>>,
        }

        unsafe impl ClassType for IvarTester {
            type Super = NSObject;
        }

        unsafe impl IvarTester {
            #[sel(init)]
            fn init(&mut self) -> Option<&mut Self> {
                let this: Option<&mut Self> = unsafe { msg_send![super(self), init] };
                this.map(|this| {
                    Ivar::write(&mut this.ivar1, Id::into_shared(RcTestObject::new()));
                    *this.ivar2 = Some(RcTestObject::new());
                    Ivar::write(&mut this.ivar3, Box::new(RcTestObject::new()));
                    *this.ivar4 = Some(Box::new(RcTestObject::new()));
                    this
                })
            }

            #[sel(initInvalid)]
            fn init_invalid(&mut self) -> Option<&mut Self> {
                // Don't actually initialize anything here; this creates an
                // invalid instance, where accessing the two ivars `ivar1`
                // and `ivar3` is UB
                unsafe { msg_send![super(self), init] }
            }
        }
    );

    #[test]
    fn test_alloc_dealloc() {
        let expected = ThreadTestData::current();

        let obj: Id<Allocated<IvarTester>, Owned> =
            unsafe { msg_send_id![IvarTester::class(), alloc] };
        expected.assert_current();

        drop(obj);
        expected.assert_current();
    }

    #[test]
    fn test_init_drop() {
        let mut expected = ThreadTestData::current();

        let mut obj: Id<IvarTester, Owned> = unsafe { msg_send_id![IvarTester::class(), new] };
        expected.alloc += 4;
        expected.init += 4;
        expected.assert_current();

        *obj.ivar1 = (*obj.ivar1).clone();
        expected.retain += 1;
        expected.release += 1;
        expected.assert_current();

        *obj.ivar2 = None;
        expected.release += 1;
        expected.dealloc += 1;
        expected.assert_current();

        drop(obj);
        expected.release += 3;
        expected.dealloc += 3;
        expected.assert_current();
    }

    #[test]
    #[cfg_attr(not(debug_assertions), ignore = "only panics in debug mode")]
    #[should_panic = "an Id in instance variables must always be initialized before use"]
    fn test_init_invalid_ref() {
        let obj: Id<IvarTester, Owned> =
            unsafe { msg_send_id![msg_send_id![IvarTester::class(), alloc], initInvalid] };

        std::println!("{:?}", obj.ivar1);
    }

    #[test]
    #[cfg_attr(not(debug_assertions), ignore = "only panics in debug mode")]
    #[should_panic = "an Id in instance variables must always be initialized before use"]
    fn test_init_invalid_mut() {
        let mut obj: Id<IvarTester, Owned> =
            unsafe { msg_send_id![msg_send_id![IvarTester::class(), alloc], initInvalid] };

        *obj.ivar1 = RcTestObject::new().into();
    }
}
