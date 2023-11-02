use alloc::vec::Vec;
use core::fmt;
use core::marker::PhantomData;
use core::panic::{RefUnwindSafe, UnwindSafe};

use super::{
    NSArray, NSCopying, NSEnumerator, NSFastEnumeration, NSFastEnumerator, NSMutableCopying,
    NSMutableSet, NSObject,
};
use crate::rc::{DefaultId, Id, Owned, Ownership, Shared, SliceId};
use crate::runtime::Class;
use crate::{ClassType, Message, __inner_extern_class, extern_methods, msg_send, msg_send_id};

__inner_extern_class!(
    /// An immutable unordered collection of unique objects.
    ///
    /// See [Apple's documentation][apple-doc].
    ///
    /// [apple-doc]: https://developer.apple.com/documentation/foundation/nsset?language=objc
    #[derive(PartialEq, Eq, Hash)]
    pub struct NSSet<T: Message, O: Ownership = Shared> {
        item: PhantomData<Id<T, O>>,
        notunwindsafe: PhantomData<&'static mut ()>,
    }

    unsafe impl<T: Message, O: Ownership> ClassType for NSSet<T, O> {
        type Super = NSObject;
    }
);

// SAFETY: Same as NSArray<T, O>
unsafe impl<T: Message + Sync + Send> Sync for NSSet<T, Shared> {}
unsafe impl<T: Message + Sync + Send> Send for NSSet<T, Shared> {}
unsafe impl<T: Message + Sync> Sync for NSSet<T, Owned> {}
unsafe impl<T: Message + Send> Send for NSSet<T, Owned> {}

// SAFETY: Same as NSArray<T, O>
impl<T: Message + RefUnwindSafe, O: Ownership> RefUnwindSafe for NSSet<T, O> {}
impl<T: Message + RefUnwindSafe> UnwindSafe for NSSet<T, Shared> {}
impl<T: Message + UnwindSafe> UnwindSafe for NSSet<T, Owned> {}

#[track_caller]
pub(crate) unsafe fn with_objects<T: Message + ?Sized, R: Message, O: Ownership>(
    cls: &Class,
    objects: &[&T],
) -> Id<R, O> {
    unsafe {
        msg_send_id![
            msg_send_id![cls, alloc],
            initWithObjects: objects.as_ptr(),
            count: objects.len()
        ]
    }
}

extern_methods!(
    unsafe impl<T: Message, O: Ownership> NSSet<T, O> {
        /// Creates an empty [`NSSet`].
        ///
        /// # Examples
        ///
        /// ```
        /// use objc2::foundation::{NSSet, NSString};
        /// # #[cfg(feature = "gnustep-1-7")]
        /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
        ///
        /// let set = NSSet::<NSString>::new();
        /// ```
        pub fn new() -> Id<Self, Shared> {
            // SAFETY:
            // - `new` may not create a new object, but instead return a shared
            //   instance. We remedy this by returning `Id<Self, Shared>`.
            // - `O` don't actually matter here! E.g. `NSSet<T, Owned>` is
            //   perfectly legal, since the set doesn't have any elements, and
            //   hence the notion of ownership over the elements is void.
            unsafe { msg_send_id![Self::class(), new] }
        }

        /// Creates an [`NSSet`] from a vector.
        ///
        /// # Examples
        ///
        /// ```
        /// use objc2::foundation::{NSSet, NSString};
        /// # #[cfg(feature = "gnustep-1-7")]
        /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
        ///
        /// let strs = ["one", "two", "three"].map(NSString::from_str).to_vec();
        /// let set = NSSet::from_vec(strs);
        /// ```
        pub fn from_vec(vec: Vec<Id<T, O>>) -> Id<Self, O> {
            // SAFETY:
            // When we know that we have ownership over the variables, we also
            // know that there cannot be another set in existence with the same
            // objects, so `Id<NSSet<T, Owned>, Owned>` is safe to return when
            // we receive `Vec<Id<T, Owned>>`.
            unsafe { with_objects(Self::class(), vec.as_slice_ref()) }
        }

        /// Returns the number of elements in the set.
        ///
        /// # Examples
        ///
        /// ```
        /// use objc2::foundation::{NSSet, NSString};
        /// # #[cfg(feature = "gnustep-1-7")]
        /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
        ///
        /// let strs = ["one", "two", "three"].map(NSString::from_str);
        /// let set = NSSet::from_slice(&strs);
        /// assert_eq!(set.len(), 3);
        /// ```
        #[doc(alias = "count")]
        #[sel(count)]
        pub fn len(&self) -> usize;

        /// Returns `true` if the set contains no elements.
        ///
        /// # Examples
        ///
        /// ```
        /// use objc2::foundation::{NSSet, NSString};
        /// # #[cfg(feature = "gnustep-1-7")]
        /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
        ///
        /// let set = NSSet::<NSString>::new();
        /// assert!(set.is_empty());
        /// ```
        pub fn is_empty(&self) -> bool {
            self.len() == 0
        }

        /// Returns a reference to one of the objects in the set, or `None` if
        /// the set is empty.
        ///
        /// # Examples
        ///
        /// ```
        /// use objc2::foundation::{NSSet, NSString};
        /// # #[cfg(feature = "gnustep-1-7")]
        /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
        ///
        /// let strs = ["one", "two", "three"].map(NSString::from_str);
        /// let set = NSSet::from_slice(&strs);
        /// let any = set.get_any().unwrap();
        /// assert!(any == &*strs[0] || any == &*strs[1] || any == &*strs[2]);
        /// ```
        #[doc(alias = "anyObject")]
        #[sel(anyObject)]
        pub fn get_any(&self) -> Option<&T>;

        /// An iterator visiting all elements in arbitrary order.
        ///
        /// # Examples
        ///
        /// ```
        /// use objc2::foundation::{NSSet, NSString};
        /// # #[cfg(feature = "gnustep-1-7")]
        /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
        ///
        /// let strs = ["one", "two", "three"].map(NSString::from_str);
        /// let set = NSSet::from_slice(&strs);
        /// for s in set.iter() {
        ///     println!("{s}");
        /// }
        /// ```
        #[doc(alias = "objectEnumerator")]
        pub fn iter(&self) -> NSEnumerator<'_, T> {
            unsafe {
                let result = msg_send![self, objectEnumerator];
                NSEnumerator::from_ptr(result)
            }
        }

        /// Returns a [`Vec`] containing the set's elements, consuming the set.
        ///
        /// # Examples
        ///
        /// ```
        /// use objc2::foundation::{NSMutableString, NSSet};
        /// # #[cfg(feature = "gnustep-1-7")]
        /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
        ///
        /// let strs = vec![
        ///     NSMutableString::from_str("one"),
        ///     NSMutableString::from_str("two"),
        ///     NSMutableString::from_str("three"),
        /// ];
        /// let set = NSSet::from_vec(strs);
        /// let vec = NSSet::into_vec(set);
        /// assert_eq!(vec.len(), 3);
        /// ```
        pub fn into_vec(set: Id<Self, O>) -> Vec<Id<T, O>> {
            set.into_iter()
                .map(|obj| unsafe { Id::retain(obj as *const T as *mut T).unwrap_unchecked() })
                .collect()
        }
    }

    unsafe impl<T: Message> NSSet<T, Shared> {
        /// Creates an [`NSSet`] from a slice.
        ///
        /// # Examples
        ///
        /// ```
        /// use objc2::foundation::{NSSet, NSString};
        /// # #[cfg(feature = "gnustep-1-7")]
        /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
        ///
        /// let strs = ["one", "two", "three"].map(NSString::from_str);
        /// let set = NSSet::from_slice(&strs);
        /// ```
        pub fn from_slice(slice: &[Id<T, Shared>]) -> Id<Self, Shared> {
            // SAFETY:
            // Taking `&T` would not be sound, since the `&T` could come from
            // an `Id<T, Owned>` that would now no longer be owned!
            //
            // We always return `Id<NSSet<T, Shared>, Shared>` because the
            // elements are shared.
            unsafe { with_objects(Self::class(), slice.as_slice_ref()) }
        }

        /// Returns an [`NSArray`] containing the set's elements, or an empty
        /// array if the set is empty.
        ///
        /// # Examples
        ///
        /// ```
        /// use objc2::foundation::{NSNumber, NSSet, NSString};
        /// # #[cfg(feature = "gnustep-1-7")]
        /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
        ///
        /// let nums = [1, 2, 3];
        /// let set = NSSet::from_slice(&nums.map(NSNumber::new_i32));
        ///
        /// assert_eq!(set.to_array().len(), 3);
        /// assert!(set.to_array().iter().all(|i| nums.contains(&i.as_i32())));
        /// ```
        #[doc(alias = "allObjects")]
        pub fn to_array(&self) -> Id<NSArray<T, Shared>, Shared> {
            // SAFETY:
            // We only define this method for sets with shared elements
            // because we can't return copies of owned elements.
            unsafe { msg_send_id![self, allObjects] }
        }
    }

    // We're explicit about `T` being `PartialEq` for these methods because the
    // set compares the input value(s) with elements in the set
    // For comparison: Rust's HashSet requires similar methods to be `Hash` + `Eq`
    unsafe impl<T: Message + PartialEq, O: Ownership> NSSet<T, O> {
        /// Returns `true` if the set contains a value.
        ///
        /// # Examples
        ///
        /// ```
        /// use objc2::foundation::{NSSet, NSString};
        /// use objc2::ns_string;
        /// # #[cfg(feature = "gnustep-1-7")]
        /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
        ///
        /// let strs = ["one", "two", "three"].map(NSString::from_str);
        /// let set = NSSet::from_slice(&strs);
        /// assert!(set.contains(ns_string!("one")));
        /// ```
        #[doc(alias = "containsObject:")]
        #[sel(containsObject:)]
        pub fn contains(&self, value: &T) -> bool;

        /// Returns a reference to the value in the set, if any, that is equal
        /// to the given value.
        ///
        /// # Examples
        ///
        /// ```
        /// use objc2::foundation::{NSSet, NSString};
        /// use objc2::ns_string;
        /// # #[cfg(feature = "gnustep-1-7")]
        /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
        ///
        /// let strs = ["one", "two", "three"].map(NSString::from_str);
        /// let set = NSSet::from_slice(&strs);
        /// assert_eq!(set.get(ns_string!("one")), Some(&*strs[0]));
        /// assert_eq!(set.get(ns_string!("four")), None);
        /// ```
        #[doc(alias = "member:")]
        #[sel(member:)]
        pub fn get(&self, value: &T) -> Option<&T>;

        /// Returns `true` if the set is a subset of another, i.e., `other`
        /// contains at least all the values in `self`.
        ///
        /// # Examples
        ///
        /// ```
        /// use objc2::foundation::{NSSet, NSString};
        /// # #[cfg(feature = "gnustep-1-7")]
        /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
        ///
        /// let set1 = NSSet::from_slice(&["one", "two"].map(NSString::from_str));
        /// let set2 = NSSet::from_slice(&["one", "two", "three"].map(NSString::from_str));
        ///
        /// assert!(set1.is_subset(&set2));
        /// assert!(!set2.is_subset(&set1));
        /// ```
        #[doc(alias = "isSubsetOfSet:")]
        #[sel(isSubsetOfSet:)]
        pub fn is_subset(&self, other: &NSSet<T, O>) -> bool;

        /// Returns `true` if the set is a superset of another, i.e., `self`
        /// contains at least all the values in `other`.
        ///
        /// # Examples
        ///
        /// ```
        /// use objc2::foundation::{NSSet, NSString};
        /// # #[cfg(feature = "gnustep-1-7")]
        /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
        ///
        /// let set1 = NSSet::from_slice(&["one", "two"].map(NSString::from_str));
        /// let set2 = NSSet::from_slice(&["one", "two", "three"].map(NSString::from_str));
        ///
        /// assert!(!set1.is_superset(&set2));
        /// assert!(set2.is_superset(&set1));
        /// ```
        pub fn is_superset(&self, other: &NSSet<T, O>) -> bool {
            other.is_subset(self)
        }

        #[sel(intersectsSet:)]
        fn intersects_set(&self, other: &NSSet<T, O>) -> bool;

        /// Returns `true` if `self` has no elements in common with `other`.
        ///
        /// # Examples
        ///
        /// ```
        /// use objc2::foundation::{NSSet, NSString};
        /// # #[cfg(feature = "gnustep-1-7")]
        /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
        ///
        /// let set1 = NSSet::from_slice(&["one", "two"].map(NSString::from_str));
        /// let set2 = NSSet::from_slice(&["one", "two", "three"].map(NSString::from_str));
        /// let set3 = NSSet::from_slice(&["four", "five", "six"].map(NSString::from_str));
        ///
        /// assert!(!set1.is_disjoint(&set2));
        /// assert!(set1.is_disjoint(&set3));
        /// assert!(set2.is_disjoint(&set3));
        /// ```
        pub fn is_disjoint(&self, other: &NSSet<T, O>) -> bool {
            !self.intersects_set(other)
        }
    }
);

unsafe impl<T: Message> NSCopying for NSSet<T, Shared> {
    type Ownership = Shared;
    type Output = NSSet<T, Shared>;
}

unsafe impl<T: Message> NSMutableCopying for NSSet<T, Shared> {
    type Output = NSMutableSet<T, Shared>;
}

impl<T: Message> alloc::borrow::ToOwned for NSSet<T, Shared> {
    type Owned = Id<NSSet<T, Shared>, Shared>;
    fn to_owned(&self) -> Self::Owned {
        self.copy()
    }
}

unsafe impl<T: Message, O: Ownership> NSFastEnumeration for NSSet<T, O> {
    type Item = T;
}

impl<'a, T: Message, O: Ownership> IntoIterator for &'a NSSet<T, O> {
    type Item = &'a T;
    type IntoIter = NSFastEnumerator<'a, NSSet<T, O>>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_fast()
    }
}

impl<T: Message, O: Ownership> DefaultId for NSSet<T, O> {
    type Ownership = Shared;

    #[inline]
    fn default_id() -> Id<Self, Self::Ownership> {
        Self::new()
    }
}

impl<T: fmt::Debug + Message, O: Ownership> fmt::Debug for NSSet<T, O> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter_fast()).finish()
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;
    use alloc::vec;

    use super::*;
    use crate::foundation::{NSMutableString, NSNumber, NSString};
    use crate::ns_string;
    use crate::rc::{RcTestObject, ThreadTestData};

    #[test]
    fn test_new() {
        let set = NSSet::<NSString>::new();
        assert!(set.is_empty());
    }

    #[test]
    fn test_from_vec() {
        let set = NSSet::<NSString>::from_vec(Vec::new());
        assert!(set.is_empty());

        let strs = ["one", "two", "three"].map(NSString::from_str);
        let set = NSSet::from_vec(strs.to_vec());
        assert!(strs.into_iter().all(|s| set.contains(&s)));

        let nums = [1, 2, 3].map(NSNumber::new_i32);
        let set = NSSet::from_vec(nums.to_vec());
        assert!(nums.into_iter().all(|n| set.contains(&n)));
    }

    #[test]
    fn test_from_slice() {
        let set = NSSet::<NSString>::from_slice(&[]);
        assert!(set.is_empty());

        let strs = ["one", "two", "three"].map(NSString::from_str);
        let set = NSSet::from_slice(&strs);
        assert!(strs.into_iter().all(|s| set.contains(&s)));

        let nums = [1, 2, 3].map(NSNumber::new_i32);
        let set = NSSet::from_slice(&nums);
        assert!(nums.into_iter().all(|n| set.contains(&n)));
    }

    #[test]
    fn test_len() {
        let set = NSSet::<NSString>::new();
        assert!(set.is_empty());

        let set = NSSet::from_slice(&["one", "two", "two"].map(NSString::from_str));
        assert_eq!(set.len(), 2);

        let set = NSSet::from_vec(vec![NSObject::new(), NSObject::new(), NSObject::new()]);
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn test_get() {
        let set = NSSet::<NSString>::new();
        assert!(set.get(ns_string!("one")).is_none());

        let set = NSSet::from_slice(&["one", "two", "two"].map(NSString::from_str));
        assert!(set.get(ns_string!("two")).is_some());
        assert!(set.get(ns_string!("three")).is_none());
    }

    #[test]
    fn test_get_return_lifetime() {
        let set = NSSet::from_slice(&["one", "two", "two"].map(NSString::from_str));

        let res = {
            let value = NSString::from_str("one");
            set.get(&value)
        };

        assert_eq!(res, Some(ns_string!("one")));
    }

    #[test]
    fn test_get_any() {
        let set = NSSet::<NSString>::new();
        assert!(set.get_any().is_none());

        let strs = ["one", "two", "three"].map(NSString::from_str);
        let set = NSSet::from_slice(&strs);
        let any = set.get_any().unwrap();
        assert!(any == &*strs[0] || any == &*strs[1] || any == &*strs[2]);
    }

    #[test]
    fn test_contains() {
        let set = NSSet::<NSString>::new();
        assert!(!set.contains(ns_string!("one")));

        let set = NSSet::from_slice(&["one", "two", "two"].map(NSString::from_str));
        assert!(set.contains(ns_string!("one")));
        assert!(!set.contains(ns_string!("three")));
    }

    #[test]
    fn test_is_subset() {
        let set1 = NSSet::from_slice(&["one", "two"].map(NSString::from_str));
        let set2 = NSSet::from_slice(&["one", "two", "three"].map(NSString::from_str));

        assert!(set1.is_subset(&set2));
        assert!(!set2.is_subset(&set1));
    }

    #[test]
    fn test_is_superset() {
        let set1 = NSSet::from_slice(&["one", "two"].map(NSString::from_str));
        let set2 = NSSet::from_slice(&["one", "two", "three"].map(NSString::from_str));

        assert!(!set1.is_superset(&set2));
        assert!(set2.is_superset(&set1));
    }

    #[test]
    fn test_is_disjoint() {
        let set1 = NSSet::from_slice(&["one", "two"].map(NSString::from_str));
        let set2 = NSSet::from_slice(&["one", "two", "three"].map(NSString::from_str));
        let set3 = NSSet::from_slice(&["four", "five", "six"].map(NSString::from_str));

        assert!(!set1.is_disjoint(&set2));
        assert!(set1.is_disjoint(&set3));
        assert!(set2.is_disjoint(&set3));
    }

    #[test]
    fn test_to_array() {
        let nums = [1, 2, 3];
        let set = NSSet::from_slice(&nums.map(NSNumber::new_i32));

        assert_eq!(set.to_array().len(), 3);
        assert!(set.to_array().iter().all(|i| nums.contains(&i.as_i32())));
    }

    #[test]
    fn test_iter() {
        let nums = [1, 2, 3];
        let set = NSSet::from_slice(&nums.map(NSNumber::new_i32));

        assert_eq!(set.iter().count(), 3);
        assert!(set.iter().all(|i| nums.contains(&i.as_i32())));
    }

    #[test]
    fn test_iter_fast() {
        let nums = [1, 2, 3];
        let set = NSSet::from_slice(&nums.map(NSNumber::new_i32));

        assert_eq!(set.iter_fast().count(), 3);
        assert!(set.iter_fast().all(|i| nums.contains(&i.as_i32())));
    }

    #[test]
    fn test_into_iter() {
        let nums = [1, 2, 3];
        let set = NSSet::from_slice(&nums.map(NSNumber::new_i32));

        assert!(set.into_iter().all(|i| nums.contains(&i.as_i32())));
    }

    #[test]
    fn test_into_vec() {
        let strs = vec![
            NSMutableString::from_str("one"),
            NSMutableString::from_str("two"),
            NSMutableString::from_str("three"),
        ];
        let set = NSSet::from_vec(strs);

        let mut vec = NSSet::into_vec(set);
        for str in vec.iter_mut() {
            str.push_nsstring(ns_string!(" times zero is zero"));
        }

        assert_eq!(vec.len(), 3);
        let suffix = ns_string!("zero");
        assert!(vec.iter().all(|str| str.has_suffix(suffix)));
    }

    #[test]
    fn test_equality() {
        let set1 = NSSet::<NSString>::new();
        let set2 = NSSet::<NSString>::new();
        assert_eq!(set1, set2);
    }

    #[test]
    fn test_copy() {
        let set1 = NSSet::from_slice(&["one", "two", "three"].map(NSString::from_str));
        let set2 = set1.copy();
        assert_eq!(set1, set2);
    }

    #[test]
    fn test_debug() {
        let set = NSSet::<NSString>::new();
        assert_eq!(format!("{:?}", set), "{}");

        let set = NSSet::from_slice(&["one", "two"].map(NSString::from_str));
        assert!(matches!(
            format!("{:?}", set).as_str(),
            "{\"one\", \"two\"}" | "{\"two\", \"one\"}"
        ));
    }

    #[test]
    fn test_retains_stored() {
        let obj = Id::into_shared(RcTestObject::new());
        let mut expected = ThreadTestData::current();

        let input = [obj.clone(), obj.clone()];
        expected.retain += 2;
        expected.assert_current();

        let set = NSSet::from_slice(&input);
        expected.retain += 1;
        expected.assert_current();

        let _obj = set.get_any().unwrap();
        expected.assert_current();

        drop(set);
        expected.release += 1;
        expected.assert_current();

        let set = NSSet::from_vec(Vec::from(input));
        expected.retain += 1;
        expected.release += 2;
        expected.assert_current();

        drop(set);
        expected.release += 1;
        expected.assert_current();

        drop(obj);
        expected.release += 1;
        expected.dealloc += 1;
        expected.assert_current();
    }

    #[test]
    fn test_nscopying_uses_retain() {
        let obj = Id::into_shared(RcTestObject::new());
        let set = NSSet::from_slice(&[obj]);
        let mut expected = ThreadTestData::current();

        let _copy = set.copy();
        expected.assert_current();

        let _copy = set.mutable_copy();
        expected.retain += 1;
        expected.assert_current();
    }

    #[test]
    #[cfg_attr(
        feature = "apple",
        ignore = "this works differently on different framework versions"
    )]
    fn test_iter_no_retain() {
        let obj = Id::into_shared(RcTestObject::new());
        let set = NSSet::from_slice(&[obj]);
        let mut expected = ThreadTestData::current();

        let iter = set.iter();
        expected.retain += 0;
        expected.assert_current();

        assert_eq!(iter.count(), 1);
        expected.autorelease += 0;
        expected.assert_current();

        let iter = set.iter_fast();
        assert_eq!(iter.count(), 1);
        expected.assert_current();
    }
}
