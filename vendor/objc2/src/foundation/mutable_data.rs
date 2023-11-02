#[cfg(feature = "block")]
use alloc::vec::Vec;
use core::ffi::c_void;
use core::fmt;
use core::ops::{Index, IndexMut, Range};
use core::slice::{self, SliceIndex};
use std::io;

use super::data::with_slice;
use super::{NSCopying, NSData, NSMutableCopying, NSObject, NSRange};
use crate::rc::{DefaultId, Id, Owned, Shared};
use crate::{extern_class, extern_methods, msg_send_id, ClassType};

extern_class!(
    /// A dynamic byte buffer in memory.
    ///
    /// This is the Objective-C equivalent of a [`Vec`] containing [`u8`].
    ///
    /// See [Apple's documentation](https://developer.apple.com/documentation/foundation/nsmutabledata?language=objc).
    ///
    /// [`Vec`]: std::vec::Vec
    #[derive(PartialEq, Eq, Hash)]
    pub struct NSMutableData;

    unsafe impl ClassType for NSMutableData {
        #[inherits(NSObject)]
        type Super = NSData;
    }
);

extern_methods!(
    /// Creation methods
    unsafe impl NSMutableData {
        pub fn new() -> Id<Self, Owned> {
            unsafe { msg_send_id![Self::class(), new] }
        }

        pub fn with_bytes(bytes: &[u8]) -> Id<Self, Owned> {
            unsafe { Id::from_shared(Id::cast(with_slice(Self::class(), bytes))) }
        }

        #[cfg(feature = "block")]
        pub fn from_vec(bytes: Vec<u8>) -> Id<Self, Owned> {
            unsafe { Id::from_shared(Id::cast(super::data::with_vec(Self::class(), bytes))) }
        }

        // TODO: Use malloc_buf/mbox and `initWithBytesNoCopy:...`?

        #[doc(alias = "initWithData:")]
        pub fn from_data(data: &NSData) -> Id<Self, Owned> {
            // Not provided on NSData, one should just use NSData::copy or similar
            unsafe {
                let obj = msg_send_id![Self::class(), alloc];
                msg_send_id![obj, initWithData: data]
            }
        }

        #[doc(alias = "initWithCapacity:")]
        pub fn with_capacity(capacity: usize) -> Id<Self, Owned> {
            unsafe {
                let obj = msg_send_id![Self::class(), alloc];
                msg_send_id![obj, initWithCapacity: capacity]
            }
        }
    }

    /// Mutation methods
    unsafe impl NSMutableData {
        /// Expands with zeroes, or truncates the buffer.
        #[doc(alias = "setLength:")]
        #[sel(setLength:)]
        pub fn set_len(&mut self, len: usize);

        #[sel(mutableBytes)]
        fn bytes_mut_raw(&mut self) -> *mut c_void;

        #[doc(alias = "mutableBytes")]
        pub fn bytes_mut(&mut self) -> &mut [u8] {
            let ptr = self.bytes_mut_raw();
            let ptr: *mut u8 = ptr.cast();
            // The bytes pointer may be null for length zero
            if ptr.is_null() {
                &mut []
            } else {
                unsafe { slice::from_raw_parts_mut(ptr, self.len()) }
            }
        }

        #[sel(appendBytes:length:)]
        unsafe fn append_raw(&mut self, ptr: *const c_void, len: usize);

        #[doc(alias = "appendBytes:length:")]
        pub fn extend_from_slice(&mut self, bytes: &[u8]) {
            let bytes_ptr: *const c_void = bytes.as_ptr().cast();
            unsafe { self.append_raw(bytes_ptr, bytes.len()) }
        }

        pub fn push(&mut self, byte: u8) {
            self.extend_from_slice(&[byte])
        }

        #[sel(replaceBytesInRange:withBytes:length:)]
        unsafe fn replace_raw(&mut self, range: NSRange, ptr: *const c_void, len: usize);

        #[doc(alias = "replaceBytesInRange:withBytes:length:")]
        pub fn replace_range(&mut self, range: Range<usize>, bytes: &[u8]) {
            let range = NSRange::from(range);
            // No need to verify the length of the range here,
            // `replaceBytesInRange:` just zero-fills if out of bounds.
            let ptr: *const c_void = bytes.as_ptr().cast();
            unsafe { self.replace_raw(range, ptr, bytes.len()) }
        }

        pub fn set_bytes(&mut self, bytes: &[u8]) {
            let len = self.len();
            self.replace_range(0..len, bytes);
        }
    }
);

unsafe impl NSCopying for NSMutableData {
    type Ownership = Shared;
    type Output = NSData;
}

unsafe impl NSMutableCopying for NSMutableData {
    type Output = NSMutableData;
}

impl alloc::borrow::ToOwned for NSMutableData {
    type Owned = Id<NSMutableData, Owned>;
    fn to_owned(&self) -> Self::Owned {
        self.mutable_copy()
    }
}

impl AsRef<[u8]> for NSMutableData {
    fn as_ref(&self) -> &[u8] {
        self.bytes()
    }
}

impl AsMut<[u8]> for NSMutableData {
    fn as_mut(&mut self) -> &mut [u8] {
        self.bytes_mut()
    }
}

impl<I: SliceIndex<[u8]>> Index<I> for NSMutableData {
    type Output = I::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        Index::index(self.bytes(), index)
    }
}

impl<I: SliceIndex<[u8]>> IndexMut<I> for NSMutableData {
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        IndexMut::index_mut(self.bytes_mut(), index)
    }
}

// impl FromIterator<u8> for Id<NSMutableData, Owned> {
//     fn from_iter<T: IntoIterator<Item = u8>>(iter: T) -> Self {
//         let iter = iter.into_iter();
//         let (lower, _) = iter.size_hint();
//         let data = Self::with_capacity(lower);
//         for item in iter {
//             data.push(item);
//         }
//         data
//     }
// }

impl Extend<u8> for NSMutableData {
    /// You should use [`extend_from_slice`] whenever possible, it is more
    /// performant.
    ///
    /// [`extend_from_slice`]: Self::extend_from_slice
    fn extend<T: IntoIterator<Item = u8>>(&mut self, iter: T) {
        let iterator = iter.into_iter();
        iterator.for_each(move |item| self.push(item));
    }
}

// Vec also has this impl
impl<'a> Extend<&'a u8> for NSMutableData {
    fn extend<T: IntoIterator<Item = &'a u8>>(&mut self, iter: T) {
        let iterator = iter.into_iter();
        iterator.for_each(move |item| self.push(*item));
    }
}

impl io::Write for NSMutableData {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.extend_from_slice(buf);
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl DefaultId for NSMutableData {
    type Ownership = Owned;

    #[inline]
    fn default_id() -> Id<Self, Self::Ownership> {
        Self::new()
    }
}

impl fmt::Debug for NSMutableData {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<'a> IntoIterator for &'a NSMutableData {
    type Item = &'a u8;
    type IntoIter = core::slice::Iter<'a, u8>;

    fn into_iter(self) -> Self::IntoIter {
        self.bytes().iter()
    }
}

impl<'a> IntoIterator for &'a mut NSMutableData {
    type Item = &'a mut u8;
    type IntoIter = core::slice::IterMut<'a, u8>;

    fn into_iter(self) -> Self::IntoIter {
        self.bytes_mut().iter_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::Object;

    #[test]
    fn test_bytes_mut() {
        let mut data = NSMutableData::with_bytes(&[7, 16]);
        data.bytes_mut()[0] = 3;
        assert_eq!(data.bytes(), [3, 16]);
    }

    #[test]
    fn test_set_len() {
        let mut data = NSMutableData::with_bytes(&[7, 16]);
        data.set_len(4);
        assert_eq!(data.len(), 4);
        assert_eq!(data.bytes(), [7, 16, 0, 0]);

        data.set_len(1);
        assert_eq!(data.len(), 1);
        assert_eq!(data.bytes(), [7]);
    }

    #[test]
    fn test_append() {
        let mut data = NSMutableData::with_bytes(&[7, 16]);
        data.extend_from_slice(&[3, 52]);
        assert_eq!(data.len(), 4);
        assert_eq!(data.bytes(), [7, 16, 3, 52]);
    }

    #[test]
    fn test_replace() {
        let mut data = NSMutableData::with_bytes(&[7, 16]);
        data.replace_range(0..0, &[3]);
        assert_eq!(data.bytes(), [3, 7, 16]);

        data.replace_range(1..2, &[52, 13]);
        assert_eq!(data.bytes(), [3, 52, 13, 16]);

        data.replace_range(2..4, &[6]);
        assert_eq!(data.bytes(), [3, 52, 6]);

        data.set_bytes(&[8, 17]);
        assert_eq!(data.bytes(), [8, 17]);
    }

    #[test]
    fn test_from_data() {
        let data = NSData::with_bytes(&[1, 2]);
        let mut_data = NSMutableData::from_data(&data);
        assert_eq!(&*data, &**mut_data);
    }

    #[test]
    fn test_with_capacity() {
        let mut data = NSMutableData::with_capacity(5);
        assert_eq!(data.bytes(), &[]);
        data.extend_from_slice(&[1, 2, 3, 4, 5]);
        assert_eq!(data.bytes(), &[1, 2, 3, 4, 5]);
        data.extend_from_slice(&[6, 7]);
        assert_eq!(data.bytes(), &[1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn test_extend() {
        let mut data = NSMutableData::with_bytes(&[1, 2]);
        data.extend(3..=5);
        assert_eq!(data.bytes(), &[1, 2, 3, 4, 5]);
        data.extend(&*NSData::with_bytes(&[6, 7]));
        assert_eq!(data.bytes(), &[1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn test_as_ref_borrow() {
        use core::borrow::{Borrow, BorrowMut};

        fn impls_borrow<T: AsRef<U> + Borrow<U> + ?Sized, U: ?Sized>(_: &T) {}
        fn impls_borrow_mut<T: AsMut<U> + BorrowMut<U> + ?Sized, U: ?Sized>(_: &mut T) {}

        let mut obj = NSMutableData::new();
        impls_borrow::<Id<NSMutableData, Owned>, NSMutableData>(&obj);
        impls_borrow_mut::<Id<NSMutableData, Owned>, NSMutableData>(&mut obj);

        impls_borrow::<NSMutableData, NSMutableData>(&obj);
        impls_borrow_mut::<NSMutableData, NSMutableData>(&mut obj);
        impls_borrow::<NSMutableData, NSData>(&obj);
        impls_borrow_mut::<NSMutableData, NSData>(&mut obj);
        impls_borrow::<NSMutableData, NSObject>(&obj);
        impls_borrow_mut::<NSMutableData, NSObject>(&mut obj);
        impls_borrow::<NSMutableData, Object>(&obj);
        impls_borrow_mut::<NSMutableData, Object>(&mut obj);

        impls_borrow::<NSData, NSData>(&obj);
        impls_borrow_mut::<NSData, NSData>(&mut obj);
        impls_borrow::<NSData, NSObject>(&obj);
        impls_borrow_mut::<NSData, NSObject>(&mut obj);
        impls_borrow::<NSData, Object>(&obj);
        impls_borrow_mut::<NSData, Object>(&mut obj);

        fn impls_as_ref<T: AsRef<U> + ?Sized, U: ?Sized>(_: &T) {}
        fn impls_as_mut<T: AsMut<U> + ?Sized, U: ?Sized>(_: &mut T) {}

        impls_as_ref::<NSMutableData, [u8]>(&obj);
        impls_as_mut::<NSMutableData, [u8]>(&mut obj);
        impls_as_ref::<NSData, [u8]>(&obj);

        let obj: &mut NSMutableData = &mut obj;
        let _: &[u8] = obj.as_ref();
        let _: &mut [u8] = obj.as_mut();

        let obj: &mut NSData = obj;
        let _: &[u8] = obj.as_ref();
    }
}
