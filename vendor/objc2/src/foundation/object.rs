use core::fmt;
use core::hash;

use super::NSString;
use crate::rc::{DefaultId, Id, Owned, Shared};
use crate::runtime::{Class, Object};
use crate::{ClassType, __inner_extern_class, class, extern_methods, msg_send_id};

__inner_extern_class! {
    @__inner
    pub struct NSObject () {}

    unsafe impl () for NSObject {
        INHERITS = [Object];
    }
}

unsafe impl ClassType for NSObject {
    type Super = Object;
    const NAME: &'static str = "NSObject";

    #[inline]
    fn class() -> &'static Class {
        class!(NSObject)
    }

    fn as_super(&self) -> &Self::Super {
        &self.__inner
    }

    fn as_super_mut(&mut self) -> &mut Self::Super {
        &mut self.__inner
    }
}

extern_methods!(
    unsafe impl NSObject {
        pub fn new() -> Id<Self, Owned> {
            unsafe { msg_send_id![Self::class(), new] }
        }

        #[sel(isKindOfClass:)]
        fn is_kind_of_inner(&self, cls: &Class) -> bool;

        #[sel(isEqual:)]
        fn is_equal(&self, other: &Self) -> bool;

        #[sel(hash)]
        fn hash_code(&self) -> usize;

        /// Check if the object is an instance of the class, or one of it's
        /// subclasses.
        ///
        /// See [Apple's documentation][apple-doc] for more details on what you
        /// may (and what you may not) do with this information.
        ///
        /// [apple-doc]: https://developer.apple.com/documentation/objectivec/1418956-nsobject/1418511-iskindofclass
        #[doc(alias = "isKindOfClass:")]
        pub fn is_kind_of<T: ClassType>(&self) -> bool {
            self.is_kind_of_inner(T::class())
        }

        // Note: We don't provide a method to convert `NSObject` to `T` based on
        // `is_kind_of`, since that is not possible to do in general!
        //
        // For example, something may have a return type of `NSString`, while
        // behind the scenes they really return `NSMutableString` and expect it to
        // not be modified.
    }
);

/// Objective-C equality has approximately the same semantics as Rust
/// equality (although less aptly specified).
///
/// At the very least, equality is _expected_ to be symmetric and
/// transitive, and that's about the best we can do.
///
/// See also <https://nshipster.com/equality/>
impl PartialEq for NSObject {
    #[inline]
    #[doc(alias = "isEqual:")]
    fn eq(&self, other: &Self) -> bool {
        self.is_equal(other)
    }
}

/// Most types' equality is reflexive.
impl Eq for NSObject {}

/// Hashing in Objective-C has the exact same requirement as in Rust:
///
/// > If two objects are equal (as determined by the isEqual: method),
/// > they must have the same hash value.
///
/// See <https://developer.apple.com/documentation/objectivec/1418956-nsobject/1418859-hash>
impl hash::Hash for NSObject {
    #[inline]
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.hash_code().hash(state);
    }
}

impl fmt::Debug for NSObject {
    #[doc(alias = "description")]
    #[doc(alias = "debugDescription")]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Get description
        let description: Option<Id<NSString, Shared>> = unsafe { msg_send_id![self, description] };

        match description {
            // Attempt to format with description
            Some(description) => fmt::Display::fmt(&description, f),
            // If description was `NULL`, use `Object`'s `Debug` impl instead
            None => {
                let obj: &Object = self;
                fmt::Debug::fmt(obj, f)
            }
        }
    }
}

impl DefaultId for NSObject {
    type Ownership = Owned;

    #[inline]
    fn default_id() -> Id<Self, Self::Ownership> {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;

    #[test]
    fn test_deref() {
        let mut obj: Id<NSObject, Owned> = NSObject::new();
        let _: &NSObject = &obj;
        let _: &mut NSObject = &mut obj;
        let _: &Object = &obj;
        let _: &mut Object = &mut obj;
    }

    #[test]
    fn test_as_ref_borrow() {
        use core::borrow::{Borrow, BorrowMut};

        fn impls_as_ref<T: AsRef<U> + Borrow<U> + ?Sized, U: ?Sized>(_: &T) {}
        fn impls_as_mut<T: AsMut<U> + BorrowMut<U> + ?Sized, U: ?Sized>(_: &mut T) {}

        let mut obj = NSObject::new();
        impls_as_ref::<Id<NSObject, Owned>, NSObject>(&obj);
        impls_as_mut::<Id<NSObject, Owned>, NSObject>(&mut obj);
        impls_as_ref::<NSObject, NSObject>(&obj);
        impls_as_mut::<NSObject, NSObject>(&mut obj);
        impls_as_ref::<NSObject, Object>(&obj);
        impls_as_mut::<NSObject, Object>(&mut obj);
    }

    #[test]
    fn test_equality() {
        let obj1 = NSObject::new();
        assert_eq!(obj1, obj1);

        let obj2 = NSObject::new();
        assert_ne!(obj1, obj2);
    }

    #[test]
    fn test_hash() {
        use core::hash::Hasher;
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hash;

        let obj1 = NSObject::new();

        let mut hashstate1 = DefaultHasher::new();
        let mut hashstate2 = DefaultHasher::new();

        obj1.hash(&mut hashstate1);
        obj1.hash(&mut hashstate2);

        assert_eq!(hashstate1.finish(), hashstate2.finish());

        let obj2 = NSObject::new();
        let mut hashstate2 = DefaultHasher::new();
        obj2.hash(&mut hashstate2);
        assert_ne!(hashstate1.finish(), hashstate2.finish());
    }

    #[test]
    fn test_debug() {
        let obj = NSObject::new();
        let expected = format!("<NSObject: {:p}>", &*obj);
        assert_eq!(format!("{:?}", obj), expected);
    }

    #[test]
    fn test_is_kind_of() {
        let obj = NSObject::new();
        assert!(obj.is_kind_of::<NSObject>());
        assert!(!obj.is_kind_of::<NSString>());
    }
}
