//! Utilities for reference counting Objective-C objects.
//!
//! These utilities in this module provide ARC-like semantics for working with
//! Objective-C's reference counted objects.
//!
//! A smart pointer [`Id`] is provided to ensure that Objective-C objects are
//! retained and released when created and dropped, respectively.
//!
//! To enforce aliasing rules, an `Id` can be either owned or shared; if it is
//! owned, meaning the `Id` is the only reference to the object, it can be
//! mutably dereferenced. An owned `Id` can be converted to a shared `Id`,
//! which can be cloned to allow multiple references.
//!
//! Weak references may be created using the [`WeakId`] struct; these will not
//! retain the object, but one can attempt to load them and obtain an `Id`, or
//! safely fail if the object has been deallocated.
//!
//! See [the clang documentation][clang-arc] and [the Apple article on memory
//! management][mem-mgmt] (similar document exists [for Core Foundation][cf])
//! for more information on automatic and manual reference counting.
//!
//! It can also be useful to [enable Malloc Debugging][mem-debug] if you're trying
//! to figure out if/where your application has memory errors and leaks.
//!
//! [clang-arc]: https://clang.llvm.org/docs/AutomaticReferenceCounting.html
//! [mem-mgmt]: https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/MemoryMgmt/Articles/MemoryMgmt.html
//! [cf]: https://developer.apple.com/library/archive/documentation/CoreFoundation/Conceptual/CFMemoryMgmt/CFMemoryMgmt.html
//! [mem-debug]: https://developer.apple.com/library/archive/documentation/Performance/Conceptual/ManagingMemory/Articles/MallocDebug.html
//!
//!
//! ## Example
//!
#![cfg_attr(feature = "apple", doc = "```")]
#![cfg_attr(not(feature = "apple"), doc = "```no_run")]
//! use objc2::{class, msg_send_id};
//! use objc2::rc::{autoreleasepool, Id, Shared, WeakId};
//! use objc2::runtime::Object;
//!
//! // Id will release the object when dropped
//! let obj: Id<Object, Shared> = unsafe {
//!     msg_send_id![class!(NSObject), new]
//! };
//!
//! // Cloning retains the object an additional time
//! let cloned = obj.clone();
//! autoreleasepool(|pool| {
//!     // Autorelease consumes the Id, but won't
//!     // actually release until the end of an autoreleasepool
//!     let obj_ref: &Object = cloned.autorelease(pool);
//! });
//!
//! // Weak references won't retain the object
//! let weak = WeakId::new(&obj);
//! drop(obj);
//! assert!(weak.load().is_none());
//! ```

mod allocated;
mod autorelease;
mod id;
mod id_forwarding_impls;
mod id_traits;
mod ownership;
mod weak_id;

#[cfg(test)]
mod test_object;

pub use self::allocated::Allocated;
pub use self::autorelease::{autoreleasepool, AutoreleasePool, AutoreleaseSafe};
pub use self::id::Id;
pub use self::id_traits::{DefaultId, SliceId, SliceIdMut};
pub use self::ownership::{Owned, Ownership, Shared};
pub use self::weak_id::WeakId;

#[cfg(test)]
pub(crate) use self::test_object::{RcTestObject, ThreadTestData};

#[cfg(test)]
mod tests {
    use core::marker::PhantomData;
    use core::mem::size_of;

    use super::{Id, Owned, Ownership, Shared, WeakId};
    use crate::runtime::Object;

    #[repr(C)]
    struct TestType {
        inner: Object,
    }

    #[repr(C)]
    struct MyObject<'a> {
        inner: Object,
        p: PhantomData<&'a str>,
    }

    /// Test that `Id<T, O>` is covariant over `T`.
    #[allow(unused)]
    fn assert_id_variance<'a, 'b, O: Ownership>(
        obj: &'a Id<MyObject<'static>, O>,
    ) -> &'a Id<MyObject<'b>, O> {
        obj
    }

    /// Test that `WeakId<T>` is covariant over `T`.
    #[allow(unused)]
    fn assert_weak_id_variance<'a, 'b>(
        obj: &'a WeakId<MyObject<'static>>,
    ) -> &'a WeakId<MyObject<'b>> {
        obj
    }

    #[test]
    fn test_size_of() {
        assert_eq!(size_of::<Id<TestType, Owned>>(), size_of::<&TestType>());
        assert_eq!(size_of::<Id<TestType, Shared>>(), size_of::<&TestType>());
        assert_eq!(
            size_of::<Option<Id<TestType, Owned>>>(),
            size_of::<&TestType>()
        );
        assert_eq!(
            size_of::<Option<Id<TestType, Shared>>>(),
            size_of::<&TestType>()
        );

        assert_eq!(
            size_of::<Option<WeakId<TestType>>>(),
            size_of::<*const ()>()
        );
    }
}
