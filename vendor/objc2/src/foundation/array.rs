use alloc::vec::Vec;
use core::fmt;
use core::marker::PhantomData;
use core::ops::{Index, IndexMut, Range};
use core::panic::{RefUnwindSafe, UnwindSafe};

use super::{
    NSCopying, NSEnumerator, NSFastEnumeration, NSFastEnumerator, NSMutableArray, NSMutableCopying,
    NSObject, NSRange,
};
use crate::rc::{DefaultId, Id, Owned, Ownership, Shared, SliceId};
use crate::runtime::{Class, Object};
use crate::{ClassType, Message, __inner_extern_class, extern_methods, msg_send, msg_send_id};

__inner_extern_class!(
    /// An immutable ordered collection of objects.
    ///
    /// This is the Objective-C equivalent of a "boxed slice" (`Box<[T]>`),
    /// so effectively a `Vec<T>` where you can't change the number of
    /// elements.
    ///
    /// The type of the contained objects is described by the generic
    /// parameter `T`, and the ownership of the objects is described with the
    /// generic parameter `O`.
    ///
    ///
    /// # Ownership
    ///
    /// While `NSArray` _itself_ is immutable, i.e. the number of objects it
    /// contains can't change, it is still possible to modify the contained
    /// objects themselves, if you know you're the sole owner of them -
    /// quite similar to how you can modify elements in `Box<[T]>`.
    ///
    /// To mutate the contained objects the ownership must be `O = Owned`. A
    /// summary of what the different "types" of arrays allow you to do can be
    /// found below. `Array` refers to either `NSArray` or `NSMutableArray`.
    /// - `Id<NSMutableArray<T, Owned>, Owned>`: Allows you to mutate the
    ///   objects, and the array itself.
    /// - `Id<NSMutableArray<T, Shared>, Owned>`: Allows you to mutate the
    ///   array itself, but not it's contents.
    /// - `Id<NSArray<T, Owned>, Owned>`: Allows you to mutate the objects,
    ///   but not the array itself.
    /// - `Id<NSArray<T, Shared>, Owned>`: Effectively the same as the below.
    /// - `Id<Array<T, Shared>, Shared>`: Allows you to copy the array, but
    ///   does not allow you to modify it in any way.
    /// - `Id<Array<T, Owned>, Shared>`: Pretty useless compared to the
    ///   others, avoid this.
    ///
    /// See [Apple's documentation][apple-doc].
    ///
    /// [apple-doc]: https://developer.apple.com/documentation/foundation/nsarray?language=objc
    // `T: PartialEq` bound correct because `NSArray` does deep (instead of
    // shallow) equality comparisons.
    #[derive(PartialEq, Eq, Hash)]
    pub struct NSArray<T: Message, O: Ownership = Shared> {
        item: PhantomData<Id<T, O>>,
        notunwindsafe: PhantomData<&'static mut ()>,
    }

    unsafe impl<T: Message, O: Ownership> ClassType for NSArray<T, O> {
        type Super = NSObject;
    }
);

// SAFETY: Same as Id<T, O> (which is what NSArray effectively stores).
unsafe impl<T: Message + Sync + Send> Sync for NSArray<T, Shared> {}
unsafe impl<T: Message + Sync + Send> Send for NSArray<T, Shared> {}
unsafe impl<T: Message + Sync> Sync for NSArray<T, Owned> {}
unsafe impl<T: Message + Send> Send for NSArray<T, Owned> {}

// Also same as Id<T, O>
impl<T: Message + RefUnwindSafe, O: Ownership> RefUnwindSafe for NSArray<T, O> {}
impl<T: Message + RefUnwindSafe> UnwindSafe for NSArray<T, Shared> {}
impl<T: Message + UnwindSafe> UnwindSafe for NSArray<T, Owned> {}

#[track_caller]
pub(crate) unsafe fn with_objects<T: Message + ?Sized, R: Message, O: Ownership>(
    cls: &Class,
    objects: &[&T],
) -> Id<R, O> {
    unsafe {
        msg_send_id![
            msg_send_id![cls, alloc],
            initWithObjects: objects.as_ptr(),
            count: objects.len(),
        ]
    }
}

extern_methods!(
    /// Generic creation methods.
    unsafe impl<T: Message, O: Ownership> NSArray<T, O> {
        /// Get an empty array.
        pub fn new() -> Id<Self, Shared> {
            // SAFETY:
            // - `new` may not create a new object, but instead return a shared
            //   instance. We remedy this by returning `Id<Self, Shared>`.
            // - `O` don't actually matter here! E.g. `NSArray<T, Owned>` is
            //   perfectly legal, since the array doesn't have any elements, and
            //   hence the notion of ownership over the elements is void.
            unsafe { msg_send_id![Self::class(), new] }
        }

        pub fn from_vec(vec: Vec<Id<T, O>>) -> Id<Self, O> {
            // SAFETY:
            // `initWithObjects:` may choose to deduplicate arrays (I could
            // imagine it having a special case for arrays with one `NSNumber`
            // object), and returning mutable references to those would be
            // unsound!
            // However, when we know that we have ownership over the variables, we
            // also know that there cannot be another array in existence with the
            // same objects, so `Id<NSArray<T, Owned>, Owned>` is safe to return.
            //
            // In essence, we can choose between always returning `Id<T, Shared>`
            // or `Id<T, O>`, and the latter is probably the most useful, as we
            // would like to know when we're the only owner of the array, to
            // allow mutation of the array's items.
            unsafe { with_objects(Self::class(), vec.as_slice_ref()) }
        }
    }

    /// Creation methods that produce shared arrays.
    unsafe impl<T: Message> NSArray<T, Shared> {
        pub fn from_slice(slice: &[Id<T, Shared>]) -> Id<Self, Shared> {
            // SAFETY: Taking `&T` would not be sound, since the `&T` could come
            // from an `Id<T, Owned>` that would now no longer be owned!
            //
            // (Note that NSArray internally retains all the objects it is given,
            // effectively making the safety requirements the same as
            // `Id::retain`).
            unsafe { with_objects(Self::class(), slice.as_slice_ref()) }
        }
    }

    /// Generic accessor methods.
    unsafe impl<T: Message, O: Ownership> NSArray<T, O> {
        #[doc(alias = "count")]
        #[sel(count)]
        pub fn len(&self) -> usize;

        pub fn is_empty(&self) -> bool {
            self.len() == 0
        }

        #[sel(objectAtIndex:)]
        unsafe fn get_unchecked(&self, index: usize) -> &T;

        #[doc(alias = "objectAtIndex:")]
        pub fn get(&self, index: usize) -> Option<&T> {
            // TODO: Replace this check with catching the thrown NSRangeException
            if index < self.len() {
                // SAFETY: The index is checked to be in bounds.
                Some(unsafe { self.get_unchecked(index) })
            } else {
                None
            }
        }

        #[doc(alias = "firstObject")]
        #[sel(firstObject)]
        pub fn first(&self) -> Option<&T>;

        #[doc(alias = "lastObject")]
        #[sel(lastObject)]
        pub fn last(&self) -> Option<&T>;

        #[doc(alias = "objectEnumerator")]
        pub fn iter(&self) -> NSEnumerator<'_, T> {
            unsafe {
                let result: *mut Object = msg_send![self, objectEnumerator];
                NSEnumerator::from_ptr(result)
            }
        }

        #[sel(getObjects:range:)]
        unsafe fn get_objects(&self, ptr: *mut &T, range: NSRange);

        pub fn objects_in_range(&self, range: Range<usize>) -> Vec<&T> {
            let range = NSRange::from(range);
            let mut vec = Vec::with_capacity(range.length);
            unsafe {
                self.get_objects(vec.as_mut_ptr(), range);
                vec.set_len(range.length);
            }
            vec
        }

        pub fn to_vec(&self) -> Vec<&T> {
            self.objects_in_range(0..self.len())
        }

        // TODO: Take Id<Self, Self::ItemOwnership> ?
        pub fn into_vec(array: Id<Self, Owned>) -> Vec<Id<T, O>> {
            array
                .to_vec()
                .into_iter()
                .map(|obj| unsafe { Id::retain(obj as *const T as *mut T).unwrap_unchecked() })
                .collect()
        }
    }

    /// Accessor methods that work on shared arrays.
    unsafe impl<T: Message> NSArray<T, Shared> {
        #[doc(alias = "objectAtIndex:")]
        pub fn get_retained(&self, index: usize) -> Id<T, Shared> {
            let obj = self.get(index).unwrap();
            // SAFETY: The object is originally shared (see `where` bound).
            unsafe { Id::retain_autoreleased(obj as *const T as *mut T).unwrap_unchecked() }
        }

        pub fn to_shared_vec(&self) -> Vec<Id<T, Shared>> {
            self.to_vec()
                .into_iter()
                .map(|obj| unsafe { Id::retain(obj as *const T as *mut T).unwrap_unchecked() })
                .collect()
        }
    }

    /// Accessor methods that work on owned arrays.
    unsafe impl<T: Message> NSArray<T, Owned> {
        #[doc(alias = "objectAtIndex:")]
        pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
            // TODO: Replace this check with catching the thrown NSRangeException
            if index < self.len() {
                // SAFETY: The index is checked to be in bounds.
                Some(unsafe { msg_send![self, objectAtIndex: index] })
            } else {
                None
            }
        }

        #[doc(alias = "firstObject")]
        #[sel(firstObject)]
        pub fn first_mut(&mut self) -> Option<&mut T>;

        #[doc(alias = "lastObject")]
        #[sel(lastObject)]
        pub fn last_mut(&mut self) -> Option<&mut T>;
    }
);

/// This is implemented as a shallow copy.
///
/// As such, it is only possible when the array's contents are `Shared`.
unsafe impl<T: Message> NSCopying for NSArray<T, Shared> {
    type Ownership = Shared;
    type Output = NSArray<T, Shared>;
}

/// This is implemented as a shallow copy.
unsafe impl<T: Message> NSMutableCopying for NSArray<T, Shared> {
    type Output = NSMutableArray<T, Shared>;
}

impl<T: Message> alloc::borrow::ToOwned for NSArray<T, Shared> {
    type Owned = Id<NSArray<T, Shared>, Shared>;
    fn to_owned(&self) -> Self::Owned {
        self.copy()
    }
}

unsafe impl<T: Message, O: Ownership> NSFastEnumeration for NSArray<T, O> {
    type Item = T;
}

impl<'a, T: Message, O: Ownership> IntoIterator for &'a NSArray<T, O> {
    type Item = &'a T;
    type IntoIter = NSFastEnumerator<'a, NSArray<T, O>>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_fast()
    }
}

impl<T: Message, O: Ownership> Index<usize> for NSArray<T, O> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        self.get(index).unwrap()
    }
}

impl<T: Message> IndexMut<usize> for NSArray<T, Owned> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        self.get_mut(index).unwrap()
    }
}

impl<T: Message, O: Ownership> DefaultId for NSArray<T, O> {
    type Ownership = Shared;

    #[inline]
    fn default_id() -> Id<Self, Self::Ownership> {
        Self::new()
    }
}

impl<T: fmt::Debug + Message, O: Ownership> fmt::Debug for NSArray<T, O> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter_fast()).finish()
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;
    use alloc::vec::Vec;

    use super::*;
    use crate::foundation::{NSNumber, NSString};
    use crate::rc::{RcTestObject, ThreadTestData};

    fn sample_array(len: usize) -> Id<NSArray<NSObject, Owned>, Owned> {
        let mut vec = Vec::with_capacity(len);
        for _ in 0..len {
            vec.push(NSObject::new());
        }
        NSArray::from_vec(vec)
    }

    fn sample_number_array(len: u8) -> Id<NSArray<NSNumber, Shared>, Shared> {
        let mut vec = Vec::with_capacity(len as usize);
        for i in 0..len {
            vec.push(NSNumber::new_u8(i));
        }
        NSArray::from_vec(vec)
    }

    #[test]
    fn test_two_empty() {
        let _empty_array1 = NSArray::<NSObject>::new();
        let _empty_array2 = NSArray::<NSObject>::new();
    }

    #[test]
    fn test_len() {
        let empty_array = NSArray::<NSObject>::new();
        assert_eq!(empty_array.len(), 0);

        let array = sample_array(4);
        assert_eq!(array.len(), 4);
    }

    #[test]
    fn test_equality() {
        let array1 = sample_array(3);
        let array2 = sample_array(3);
        assert_ne!(array1, array2);

        let array1 = sample_number_array(3);
        let array2 = sample_number_array(3);
        assert_eq!(array1, array2);

        let array1 = sample_number_array(3);
        let array2 = sample_number_array(4);
        assert_ne!(array1, array2);
    }

    #[test]
    fn test_debug() {
        let obj = sample_number_array(0);
        assert_eq!(format!("{:?}", obj), "[]");
        let obj = sample_number_array(3);
        assert_eq!(format!("{:?}", obj), "[0, 1, 2]");
    }

    #[test]
    fn test_get() {
        let array = sample_array(4);
        assert_ne!(array.get(0), array.get(3));
        assert_eq!(array.first(), array.get(0));
        assert_eq!(array.last(), array.get(3));

        let empty_array = <NSArray<NSObject>>::new();
        assert!(empty_array.first().is_none());
        assert!(empty_array.last().is_none());
    }

    #[test]
    fn test_retains_stored() {
        let obj = Id::into_shared(RcTestObject::new());
        let mut expected = ThreadTestData::current();

        let input = [obj.clone(), obj.clone()];
        expected.retain += 2;
        expected.assert_current();

        let array = NSArray::from_slice(&input);
        expected.retain += 2;
        expected.assert_current();

        let _obj = array.first().unwrap();
        expected.assert_current();

        drop(array);
        expected.release += 2;
        expected.assert_current();

        let array = NSArray::from_vec(Vec::from(input));
        expected.retain += 2;
        expected.release += 2;
        expected.assert_current();

        let _obj = array.get(0).unwrap();
        let _obj = array.get(1).unwrap();
        assert!(array.get(2).is_none());
        expected.assert_current();

        drop(array);
        expected.release += 2;
        expected.assert_current();

        drop(obj);
        expected.release += 1;
        expected.dealloc += 1;
        expected.assert_current();
    }

    #[test]
    fn test_nscopying_uses_retain() {
        let obj = Id::into_shared(RcTestObject::new());
        let array = NSArray::from_slice(&[obj]);
        let mut expected = ThreadTestData::current();

        let _copy = array.copy();
        expected.assert_current();

        let _copy = array.mutable_copy();
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
        let array = NSArray::from_slice(&[obj]);
        let mut expected = ThreadTestData::current();

        let iter = array.iter();
        expected.retain += 0;
        expected.assert_current();

        assert_eq!(iter.count(), 1);
        expected.autorelease += 0;
        expected.assert_current();

        let iter = array.iter_fast();
        assert_eq!(iter.count(), 1);
        expected.assert_current();
    }

    #[test]
    fn test_iter() {
        let array = sample_array(4);

        assert_eq!(array.iter().count(), 4);
        assert!(array
            .iter()
            .enumerate()
            .all(|(i, obj)| Some(obj) == array.get(i)));
    }

    #[test]
    fn test_objects_in_range() {
        let array = sample_array(4);

        let middle_objs = array.objects_in_range(1..3);
        assert_eq!(middle_objs.len(), 2);
        assert_eq!(middle_objs[0], array.get(1).unwrap());
        assert_eq!(middle_objs[1], array.get(2).unwrap());

        let empty_objs = array.objects_in_range(1..1);
        assert!(empty_objs.is_empty());

        let all_objs = array.objects_in_range(0..4);
        assert_eq!(all_objs.len(), 4);
    }

    #[test]
    fn test_into_vec() {
        let array = sample_array(4);

        let vec = NSArray::into_vec(array);
        assert_eq!(vec.len(), 4);
    }

    #[test]
    fn test_generic_ownership_traits() {
        fn assert_partialeq<T: PartialEq>() {}

        assert_partialeq::<NSArray<NSString, Shared>>();
        assert_partialeq::<NSArray<NSString, Owned>>();

        fn test_ownership_implies_partialeq<O: Ownership>() {
            assert_partialeq::<NSArray<NSString, O>>();
        }

        test_ownership_implies_partialeq::<Shared>();
        test_ownership_implies_partialeq::<Owned>();
    }
}
