use alloc::vec::Vec;
use core::fmt;
use core::marker::PhantomData;
use core::ops::{Index, IndexMut};
use core::panic::{RefUnwindSafe, UnwindSafe};
use core::ptr;

use super::{NSArray, NSCopying, NSDictionary, NSFastEnumeration, NSObject};
use crate::rc::{DefaultId, Id, Owned, Shared};
use crate::{ClassType, __inner_extern_class, extern_methods, msg_send_id, Message};

__inner_extern_class!(
    /// A mutable collection of objects associated with unique keys.
    ///
    /// See the documentation for [`NSDictionary`] and/or [Apple's
    /// documentation][apple-doc] for more information.
    ///
    /// [apple-doc]: https://developer.apple.com/documentation/foundation/nsmutabledictionary?language=objc
    #[derive(PartialEq, Eq, Hash)]
    pub struct NSMutableDictionary<K: Message, V: Message> {
        key: PhantomData<Id<K, Shared>>,
        obj: PhantomData<Id<V, Owned>>,
    }

    unsafe impl<K: Message, V: Message> ClassType for NSMutableDictionary<K, V> {
        #[inherits(NSObject)]
        type Super = NSDictionary<K, V>;
    }
);

// Same as `NSDictionary<K, V>`
unsafe impl<K: Message + Sync + Send, V: Message + Sync> Sync for NSMutableDictionary<K, V> {}
unsafe impl<K: Message + Sync + Send, V: Message + Send> Send for NSMutableDictionary<K, V> {}

// Same as `NSDictionary<K, V>`
impl<K: Message + UnwindSafe, V: Message + UnwindSafe> UnwindSafe for NSMutableDictionary<K, V> {}
impl<K: Message + RefUnwindSafe, V: Message + RefUnwindSafe> RefUnwindSafe
    for NSMutableDictionary<K, V>
{
}

extern_methods!(
    unsafe impl<K: Message, V: Message> NSMutableDictionary<K, V> {
        /// Creates an empty [`NSMutableDictionary`].
        ///
        /// # Examples
        ///
        /// ```
        /// use objc2::foundation::{NSMutableDictionary, NSObject, NSString};
        /// # #[cfg(feature = "gnustep-1-7")]
        /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
        ///
        /// let dict = NSMutableDictionary::<NSString, NSObject>::new();
        /// ```
        pub fn new() -> Id<Self, Owned> {
            // SAFETY:
            // Mutable dictionaries are always unique, so it's safe to return
            // `Id<Self, Owned>`
            unsafe { msg_send_id![Self::class(), new] }
        }

        #[sel(setDictionary:)]
        fn set_dictionary(&mut self, dict: &NSDictionary<K, V>);

        /// Creates an [`NSMutableDictionary`] from a slice of keys and a
        /// vector of values.
        ///
        /// # Examples
        ///
        /// ```
        /// use objc2::foundation::{NSMutableDictionary, NSNumber, NSObject};
        /// # #[cfg(feature = "gnustep-1-7")]
        /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
        /// let dict = NSMutableDictionary::from_keys_and_objects(
        ///    &[
        ///        &*NSNumber::new_i32(1),
        ///        &*NSNumber::new_i32(2),
        ///        &*NSNumber::new_i32(3),
        ///    ],
        ///    vec![NSObject::new(), NSObject::new(), NSObject::new()],
        /// );
        /// ```
        pub fn from_keys_and_objects<T>(keys: &[&T], vals: Vec<Id<V, Owned>>) -> Id<Self, Owned>
        where
            T: NSCopying<Output = K>,
        {
            let mut dict = NSMutableDictionary::new();
            dict.set_dictionary(&*NSDictionary::from_keys_and_objects(keys, vals));
            dict
        }

        /// Returns a mutable reference to the value corresponding to the key.
        ///
        /// # Examples
        ///
        /// ```
        /// use objc2::foundation::{NSMutableDictionary, NSObject, NSString};
        /// use objc2::ns_string;
        /// # #[cfg(feature = "gnustep-1-7")]
        /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
        ///
        /// let mut dict = NSMutableDictionary::new();
        /// dict.insert(NSString::from_str("one"), NSObject::new());
        /// println!("{:?}", dict.get_mut(ns_string!("one")));
        /// ```
        #[doc(alias = "objectForKey:")]
        #[sel(objectForKey:)]
        pub fn get_mut(&mut self, key: &K) -> Option<&mut V>;

        #[sel(getObjects:andKeys:)]
        unsafe fn get_objects_and_keys(&self, objects: *mut &mut V, keys: *mut &K);

        /// Returns a vector of mutable references to the values in the dictionary.
        ///
        /// # Examples
        ///
        /// ```
        /// use objc2::foundation::{NSMutableDictionary, NSObject, NSString};
        /// # #[cfg(feature = "gnustep-1-7")]
        /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
        ///
        /// let mut dict = NSMutableDictionary::new();
        /// dict.insert(NSString::from_str("one"), NSObject::new());
        /// for val in dict.values_mut() {
        ///     println!("{:?}", val);
        /// }
        /// ```
        #[doc(alias = "getObjects:andKeys:")]
        pub fn values_mut(&mut self) -> Vec<&mut V> {
            let len = self.len();
            let mut vals: Vec<&mut V> = Vec::with_capacity(len);
            // SAFETY: `vals` is not null
            unsafe {
                self.get_objects_and_keys(vals.as_mut_ptr(), ptr::null_mut());
                vals.set_len(len);
            }
            vals
        }

        #[sel(setObject:forKey:)]
        fn set_object_for_key(&mut self, object: &V, key: &K);

        /// Inserts a key-value pair into the dictionary.
        ///
        /// If the dictionary did not have this key present, None is returned.
        /// If the dictionary did have this key present, the value is updated,
        /// and the old value is returned.
        ///
        /// # Examples
        ///
        /// ```
        /// use objc2::foundation::{NSMutableDictionary, NSObject, NSString};
        /// # #[cfg(feature = "gnustep-1-7")]
        /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
        ///
        /// let mut dict = NSMutableDictionary::new();
        /// dict.insert(NSString::from_str("one"), NSObject::new());
        /// ```
        #[doc(alias = "setObject:forKey:")]
        pub fn insert(&mut self, key: Id<K, Shared>, value: Id<V, Owned>) -> Option<Id<V, Owned>> {
            // SAFETY:
            // `obj` is a reference to a value in the dictionary so it's safe
            // to cast it to a pointer and pass it to `Id::retain_autoreleased`
            let obj = self.get(&*key).map(|obj| unsafe {
                Id::retain_autoreleased(obj as *const V as *mut V).unwrap_unchecked()
            });
            self.set_object_for_key(&*value, &*key);
            obj
        }

        #[sel(removeObjectForKey:)]
        fn remove_object_for_key(&mut self, key: &K);

        /// Removes a key from the dictionary, returning the value at the key
        /// if the key was previously in the dictionary.
        ///
        /// # Examples
        ///
        /// ```
        /// use objc2::foundation::{NSMutableDictionary, NSObject, NSString};
        /// use objc2::ns_string;
        /// # #[cfg(feature = "gnustep-1-7")]
        /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
        ///
        /// let mut dict = NSMutableDictionary::new();
        /// dict.insert(NSString::from_str("one"), NSObject::new());
        /// dict.remove(ns_string!("one"));
        /// assert!(dict.is_empty());
        /// ```
        #[doc(alias = "removeObjectForKey:")]
        pub fn remove(&mut self, key: &K) -> Option<Id<V, Owned>> {
            // SAFETY:
            // `obj` is a reference to a value in the dictionary so it's safe
            // to cast it to a pointer and pass it to `Id::retain_autoreleased`
            let obj = self.get(key).map(|obj| unsafe {
                Id::retain_autoreleased(obj as *const V as *mut V).unwrap_unchecked()
            });
            self.remove_object_for_key(key);
            obj
        }

        /// Clears the dictionary, removing all key-value pairs.
        ///
        /// # Examples
        ///
        /// ```
        /// use objc2::foundation::{NSMutableDictionary, NSObject, NSString};
        /// # #[cfg(feature = "gnustep-1-7")]
        /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
        ///
        /// let mut dict = NSMutableDictionary::new();
        /// dict.insert(NSString::from_str("one"), NSObject::new());
        /// dict.clear();
        /// assert!(dict.is_empty());
        /// ```
        #[doc(alias = "removeAllObjects")]
        #[sel(removeAllObjects)]
        pub fn clear(&mut self);

        /// Returns an [`NSArray`] containing the dictionary's values,
        /// consuming the dictionary.
        ///
        /// # Examples
        ///
        /// ```
        /// use objc2::foundation::{NSMutableDictionary, NSObject, NSString};
        /// # #[cfg(feature = "gnustep-1-7")]
        /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
        ///
        /// let mut dict = NSMutableDictionary::new();
        /// dict.insert(NSString::from_str("one"), NSObject::new());
        /// let array = NSMutableDictionary::into_values_array(dict);
        /// println!("{:?}", array);
        /// ```
        pub fn into_values_array(dict: Id<Self, Owned>) -> Id<NSArray<V, Owned>, Shared> {
            unsafe { msg_send_id![&dict, allValues] }
        }
    }
);

unsafe impl<K: Message, V: Message> NSFastEnumeration for NSMutableDictionary<K, V> {
    type Item = K;
}

impl<'a, K: Message, V: Message> Index<&'a K> for NSMutableDictionary<K, V> {
    type Output = V;

    fn index<'s>(&'s self, index: &'a K) -> &'s V {
        self.get(index).unwrap()
    }
}

impl<'a, K: Message, V: Message> IndexMut<&'a K> for NSMutableDictionary<K, V> {
    fn index_mut<'s>(&'s mut self, index: &'a K) -> &'s mut V {
        self.get_mut(index).unwrap()
    }
}

impl<K: Message, V: Message> DefaultId for NSMutableDictionary<K, V> {
    type Ownership = Owned;

    #[inline]
    fn default_id() -> Id<Self, Self::Ownership> {
        Self::new()
    }
}

impl<K: fmt::Debug + Message, V: fmt::Debug + Message> fmt::Debug for NSMutableDictionary<K, V> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    use crate::{
        foundation::{NSNumber, NSString},
        rc::{RcTestObject, ThreadTestData},
    };

    fn sample_dict() -> Id<NSMutableDictionary<NSNumber, NSObject>, Owned> {
        NSMutableDictionary::from_keys_and_objects(
            &[
                &*NSNumber::new_i32(1),
                &*NSNumber::new_i32(2),
                &*NSNumber::new_i32(3),
            ],
            vec![NSObject::new(), NSObject::new(), NSObject::new()],
        )
    }

    #[test]
    fn test_new() {
        let dict = NSMutableDictionary::<NSString, NSObject>::new();
        assert!(dict.is_empty());
    }

    #[test]
    fn test_get_mut() {
        let mut dict = sample_dict();
        assert!(dict.get_mut(&NSNumber::new_i32(1)).is_some());
        assert!(dict.get_mut(&NSNumber::new_i32(2)).is_some());
        assert!(dict.get_mut(&NSNumber::new_i32(4)).is_none());
    }

    #[test]
    fn test_values_mut() {
        let mut dict = sample_dict();
        let vec = dict.values_mut();
        assert_eq!(vec.len(), 3);
    }

    #[test]
    fn test_insert() {
        let mut dict = NSMutableDictionary::new();
        assert!(dict.insert(NSNumber::new_i32(1), NSObject::new()).is_none());
        assert!(dict.insert(NSNumber::new_i32(2), NSObject::new()).is_none());
        assert!(dict.insert(NSNumber::new_i32(3), NSObject::new()).is_none());
        assert!(dict.insert(NSNumber::new_i32(1), NSObject::new()).is_some());
        assert_eq!(dict.len(), 3);
    }

    #[test]
    fn test_insert_retain_release() {
        let mut dict = NSMutableDictionary::new();
        dict.insert(NSNumber::new_i32(1), RcTestObject::new());
        let mut expected = ThreadTestData::current();

        let old = dict.insert(NSNumber::new_i32(1), RcTestObject::new());
        expected.alloc += 1;
        expected.init += 1;
        expected.retain += 2;
        expected.release += 2;
        expected.assert_current();

        drop(old);
        expected.release += 1;
        expected.dealloc += 1;
        expected.assert_current();
    }

    #[test]
    fn test_remove() {
        let mut dict = sample_dict();
        assert_eq!(dict.len(), 3);
        assert!(dict.remove(&NSNumber::new_i32(1)).is_some());
        assert!(dict.remove(&NSNumber::new_i32(2)).is_some());
        assert!(dict.remove(&NSNumber::new_i32(1)).is_none());
        assert!(dict.remove(&NSNumber::new_i32(4)).is_none());
        assert_eq!(dict.len(), 1);
    }

    #[test]
    fn test_clear() {
        let mut dict = sample_dict();
        assert_eq!(dict.len(), 3);

        dict.clear();
        assert!(dict.is_empty());
    }

    #[test]
    fn test_remove_clear_release_dealloc() {
        let mut dict = NSMutableDictionary::new();
        for i in 0..4 {
            dict.insert(NSNumber::new_i32(i), RcTestObject::new());
        }
        let mut expected = ThreadTestData::current();

        let _obj = dict.remove(&NSNumber::new_i32(1));
        expected.retain += 1;
        expected.release += 1;
        expected.assert_current();
        assert_eq!(dict.len(), 3);

        let _obj = dict.remove(&NSNumber::new_i32(2));
        expected.retain += 1;
        expected.release += 1;
        expected.assert_current();
        assert_eq!(dict.len(), 2);

        dict.clear();
        expected.release += 2;
        expected.dealloc += 2;
        expected.assert_current();
        assert_eq!(dict.len(), 0);
    }

    #[test]
    fn test_into_values_array() {
        let dict = sample_dict();
        let array = NSMutableDictionary::into_values_array(dict);
        assert_eq!(array.len(), 3);
    }
}
