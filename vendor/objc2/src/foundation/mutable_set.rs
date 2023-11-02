use alloc::vec::Vec;
use core::fmt;
use core::marker::PhantomData;

use super::set::with_objects;
use super::{NSCopying, NSFastEnumeration, NSFastEnumerator, NSMutableCopying, NSObject, NSSet};
use crate::rc::{DefaultId, Id, Owned, Ownership, Shared, SliceId};
use crate::{ClassType, Message, __inner_extern_class, extern_methods, msg_send_id};

__inner_extern_class!(
    /// A growable unordered collection of unique objects.
    ///
    /// See the documentation for [`NSSet`] and/or [Apple's
    /// documentation][apple-doc] for more information.
    ///
    /// [apple-doc]: https://developer.apple.com/documentation/foundation/nsmutableset?language=objc
    #[derive(PartialEq, Eq, Hash)]
    pub struct NSMutableSet<T: Message, O: Ownership = Owned> {
        p: PhantomData<*mut ()>,
    }

    unsafe impl<T: Message, O: Ownership> ClassType for NSMutableSet<T, O> {
        #[inherits(NSObject)]
        type Super = NSSet<T, O>;
    }
);

// SAFETY: Same as NSSet<T, O>
unsafe impl<T: Message + Sync + Send> Sync for NSMutableSet<T, Shared> {}
unsafe impl<T: Message + Sync + Send> Send for NSMutableSet<T, Shared> {}
unsafe impl<T: Message + Sync> Sync for NSMutableSet<T, Owned> {}
unsafe impl<T: Message + Send> Send for NSMutableSet<T, Owned> {}

extern_methods!(
    unsafe impl<T: Message, O: Ownership> NSMutableSet<T, O> {
        /// Creates an empty [`NSMutableSet`].
        ///
        /// # Examples
        ///
        /// ```
        /// use objc2::foundation::{NSMutableSet, NSString};
        /// # #[cfg(feature = "gnustep-1-7")]
        /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
        ///
        /// let set = NSMutableSet::<NSString>::new();
        /// ```
        pub fn new() -> Id<Self, Owned> {
            // SAFETY:
            // Same as `NSSet::new`, except mutable sets are always unique.
            unsafe { msg_send_id![Self::class(), new] }
        }

        /// Creates an [`NSMutableSet`] from a vector.
        ///
        /// # Examples
        ///
        /// ```
        /// use objc2::foundation::{NSMutableSet, NSString};
        /// # #[cfg(feature = "gnustep-1-7")]
        /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
        ///
        /// let strs = ["one", "two", "three"].map(NSString::from_str).to_vec();
        /// let set = NSMutableSet::from_vec(strs);
        /// ```
        pub fn from_vec(vec: Vec<Id<T, O>>) -> Id<Self, Owned> {
            // SAFETY:
            // We always return `Id<NSMutableSet<T, O>, Owned>` because mutable
            // sets are always unique.
            unsafe { with_objects(Self::class(), vec.as_slice_ref()) }
        }

        /// Clears the set, removing all values.
        ///
        /// # Examples
        ///
        /// ```
        /// use objc2::foundation::{NSMutableSet, NSString};
        /// # #[cfg(feature = "gnustep-1-7")]
        /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
        ///
        /// let mut set = NSMutableSet::new();
        /// set.insert(NSString::from_str("one"));
        /// set.clear();
        /// assert!(set.is_empty());
        /// ```
        #[doc(alias = "removeAllObjects")]
        #[sel(removeAllObjects)]
        pub fn clear(&mut self);

        /// Returns a [`Vec`] containing the set's elements, consuming the set.
        ///
        /// # Examples
        ///
        /// ```
        /// use objc2::foundation::{NSMutableSet, NSMutableString};
        /// # #[cfg(feature = "gnustep-1-7")]
        /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
        ///
        /// let strs = vec![
        ///     NSMutableString::from_str("one"),
        ///     NSMutableString::from_str("two"),
        ///     NSMutableString::from_str("three"),
        /// ];
        /// let set = NSMutableSet::from_vec(strs);
        /// let vec = NSMutableSet::into_vec(set);
        /// assert_eq!(vec.len(), 3);
        /// ```
        pub fn into_vec(set: Id<Self, Owned>) -> Vec<Id<T, O>> {
            set.into_iter()
                .map(|obj| unsafe { Id::retain(obj as *const T as *mut T).unwrap_unchecked() })
                .collect()
        }
    }

    unsafe impl<T: Message> NSMutableSet<T, Shared> {
        /// Creates an [`NSMutableSet`] from a slice.
        ///
        /// # Examples
        ///
        /// ```
        /// use objc2::foundation::{NSMutableSet, NSString};
        /// # #[cfg(feature = "gnustep-1-7")]
        /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
        ///
        /// let strs = ["one", "two", "three"].map(NSString::from_str);
        /// let set = NSMutableSet::from_slice(&strs);
        /// ```
        pub fn from_slice(slice: &[Id<T, Shared>]) -> Id<Self, Owned> {
            // SAFETY:
            // Taking `&T` would not be sound, since the `&T` could come from
            // an `Id<T, Owned>` that would now no longer be owned!
            //
            // We always return `Id<NSMutableSet<T, Shared>, Owned>` because
            // the elements are shared and mutable sets are always unique.
            unsafe { with_objects(Self::class(), slice.as_slice_ref()) }
        }
    }

    // We're explicit about `T` being `PartialEq` for these methods because the
    // set compares the input value with elements in the set
    // For comparison: Rust's HashSet requires similar methods to be `Hash` + `Eq`
    unsafe impl<T: Message + PartialEq, O: Ownership> NSMutableSet<T, O> {
        #[sel(addObject:)]
        fn add_object(&mut self, value: &T);

        /// Adds a value to the set. Returns whether the value was
        /// newly inserted.
        ///
        /// # Examples
        ///
        /// ```
        /// use objc2::foundation::{NSMutableSet, NSString};
        /// # #[cfg(feature = "gnustep-1-7")]
        /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
        ///
        /// let mut set = NSMutableSet::new();
        ///
        /// assert_eq!(set.insert(NSString::from_str("one")), true);
        /// assert_eq!(set.insert(NSString::from_str("one")), false);
        /// assert_eq!(set.len(), 1);
        /// ```
        #[doc(alias = "addObject:")]
        pub fn insert(&mut self, value: Id<T, O>) -> bool {
            // SAFETY:
            // We take `Id<T, O>` instead of `&T` because `&T` could be a
            // reference to an owned object which would cause us to have a copy
            // of an owned object in our set. By taking `Id<T, O>`, we force the
            // caller to transfer ownership of the value to us, making it safe
            // to insert the owned object into the set.
            let contains_value = self.contains(&value);
            self.add_object(&*value);
            !contains_value
        }

        #[sel(removeObject:)]
        fn remove_object(&mut self, value: &T);

        /// Removes a value from the set. Returns whether the value was present
        /// in the set.
        ///
        /// # Examples
        ///
        /// ```
        /// use objc2::foundation::{NSMutableSet, NSString};
        /// use objc2::ns_string;
        /// # #[cfg(feature = "gnustep-1-7")]
        /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
        ///
        /// let mut set = NSMutableSet::new();
        ///
        /// set.insert(NSString::from_str("one"));
        /// assert_eq!(set.remove(ns_string!("one")), true);
        /// assert_eq!(set.remove(ns_string!("one")), false);
        /// ```
        #[doc(alias = "removeObject:")]
        pub fn remove(&mut self, value: &T) -> bool {
            let contains_value = self.contains(value);
            self.remove_object(value);
            contains_value
        }
    }
);

unsafe impl<T: Message> NSCopying for NSMutableSet<T, Shared> {
    type Ownership = Shared;
    type Output = NSSet<T, Shared>;
}

unsafe impl<T: Message> NSMutableCopying for NSMutableSet<T, Shared> {
    type Output = NSMutableSet<T, Shared>;
}

impl<T: Message> alloc::borrow::ToOwned for NSMutableSet<T, Shared> {
    type Owned = Id<NSMutableSet<T, Shared>, Owned>;
    fn to_owned(&self) -> Self::Owned {
        self.mutable_copy()
    }
}

unsafe impl<T: Message, O: Ownership> NSFastEnumeration for NSMutableSet<T, O> {
    type Item = T;
}

impl<'a, T: Message, O: Ownership> IntoIterator for &'a NSMutableSet<T, O> {
    type Item = &'a T;
    type IntoIter = NSFastEnumerator<'a, NSMutableSet<T, O>>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_fast()
    }
}

impl<T: Message + PartialEq, O: Ownership> Extend<Id<T, O>> for NSMutableSet<T, O> {
    fn extend<I: IntoIterator<Item = Id<T, O>>>(&mut self, iter: I) {
        for item in iter {
            self.insert(item);
        }
    }
}

impl<T: Message, O: Ownership> DefaultId for NSMutableSet<T, O> {
    type Ownership = Owned;

    #[inline]
    fn default_id() -> Id<Self, Self::Ownership> {
        Self::new()
    }
}

impl<T: fmt::Debug + Message, O: Ownership> fmt::Debug for NSMutableSet<T, O> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;
    use crate::foundation::{NSMutableString, NSString};
    use crate::ns_string;
    use crate::rc::{RcTestObject, ThreadTestData};

    #[test]
    fn test_insert() {
        let mut set = NSMutableSet::new();
        assert!(set.is_empty());

        assert!(set.insert(NSString::from_str("one")));
        assert!(!set.insert(NSString::from_str("one")));
        assert!(set.insert(NSString::from_str("two")));
    }

    #[test]
    fn test_remove() {
        let strs = ["one", "two", "three"].map(NSString::from_str);
        let mut set = NSMutableSet::from_slice(&strs);

        assert!(set.remove(ns_string!("one")));
        assert!(!set.remove(ns_string!("one")));
    }

    #[test]
    fn test_clear() {
        let strs = ["one", "two", "three"].map(NSString::from_str);
        let mut set = NSMutableSet::from_slice(&strs);
        assert_eq!(set.len(), 3);

        set.clear();
        assert!(set.is_empty());
    }

    #[test]
    fn test_into_vec() {
        let strs = vec![
            NSMutableString::from_str("one"),
            NSMutableString::from_str("two"),
            NSMutableString::from_str("three"),
        ];
        let set = NSMutableSet::from_vec(strs);

        let mut vec = NSMutableSet::into_vec(set);
        for str in vec.iter_mut() {
            str.push_nsstring(ns_string!(" times zero is zero"));
        }

        assert_eq!(vec.len(), 3);
        let suffix = ns_string!("zero");
        assert!(vec.iter().all(|str| str.has_suffix(suffix)));
    }

    #[test]
    fn test_extend() {
        let mut set = NSMutableSet::new();
        assert!(set.is_empty());

        set.extend(["one", "two", "three"].map(NSString::from_str));
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn test_mutable_copy() {
        let set1 = NSSet::from_slice(&["one", "two", "three"].map(NSString::from_str));
        let mut set2 = set1.mutable_copy();
        set2.insert(NSString::from_str("four"));

        assert!(set1.is_subset(&set2));
        assert_ne!(set1.mutable_copy(), set2);
    }

    #[test]
    fn test_insert_retain_release() {
        let mut set = NSMutableSet::new();
        let obj1 = RcTestObject::new();
        let obj2 = RcTestObject::new();
        let mut expected = ThreadTestData::current();

        set.insert(obj1);
        expected.retain += 1;
        expected.release += 1;
        expected.assert_current();
        assert_eq!(set.len(), 1);
        assert_eq!(set.get_any(), set.get_any());

        set.insert(obj2);
        expected.retain += 1;
        expected.release += 1;
        expected.assert_current();
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_clear_release_dealloc() {
        let mut set = NSMutableSet::new();
        for _ in 0..4 {
            set.insert(RcTestObject::new());
        }
        let mut expected = ThreadTestData::current();

        set.clear();
        expected.release += 4;
        expected.dealloc += 4;
        expected.assert_current();
        assert_eq!(set.len(), 0);
    }
}
