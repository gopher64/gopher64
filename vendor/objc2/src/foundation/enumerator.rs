use core::marker::PhantomData;
use core::mem;
use core::ptr;
use core::slice;
use std::os::raw::c_ulong;

use crate::rc::{Id, Owned};
use crate::runtime::Object;
use crate::{msg_send, Encode, Encoding, Message, RefEncode};

// TODO: https://doc.rust-lang.org/stable/reference/trait-bounds.html#lifetime-bounds
pub struct NSEnumerator<'a, T: Message> {
    id: Id<Object, Owned>,
    item: PhantomData<&'a T>,
}

impl<'a, T: Message> NSEnumerator<'a, T> {
    /// TODO
    ///
    /// # Safety
    ///
    /// The object pointer must be a valid `NSEnumerator` with `Owned`
    /// ownership.
    pub unsafe fn from_ptr(ptr: *mut Object) -> Self {
        Self {
            id: unsafe { Id::retain_autoreleased(ptr) }.unwrap(),
            item: PhantomData,
        }
    }
}

impl<'a, T: Message> Iterator for NSEnumerator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        unsafe { msg_send![&mut self.id, nextObject] }
    }
}

pub unsafe trait NSFastEnumeration: Message {
    type Item: Message;

    fn iter_fast(&self) -> NSFastEnumerator<'_, Self> {
        NSFastEnumerator::new(self)
    }
}

#[repr(C)]
struct NSFastEnumerationState<T: Message> {
    state: c_ulong, // TODO: Verify this is actually always 64 bit
    items_ptr: *const *const T,
    mutations_ptr: *mut c_ulong,
    extra: [c_ulong; 5],
}

unsafe impl<T: Message> Encode for NSFastEnumerationState<T> {
    const ENCODING: Encoding = Encoding::Struct(
        "?",
        &[
            Encoding::C_ULONG,
            Encoding::Pointer(&Encoding::Object), // <*const *const T>::ENCODING
            Encoding::Pointer(&Encoding::C_ULONG),
            Encoding::Array(5, &Encoding::C_ULONG),
        ],
    );
}

unsafe impl<T: Message> RefEncode for NSFastEnumerationState<T> {
    const ENCODING_REF: Encoding = Encoding::Pointer(&Self::ENCODING);
}

fn enumerate<'a, 'b: 'a, C: NSFastEnumeration + ?Sized>(
    object: &'b C,
    state: &mut NSFastEnumerationState<C::Item>,
    buf: &'a mut [*const C::Item],
) -> Option<&'a [*const C::Item]> {
    let count: usize = unsafe {
        // Reborrow state so that we don't move it
        let state = &mut *state;
        msg_send![
            object,
            countByEnumeratingWithState: state,
            objects: buf.as_mut_ptr(),
            count: buf.len(),
        ]
    };

    if count > 0 {
        unsafe { Some(slice::from_raw_parts(state.items_ptr, count)) }
    } else {
        None
    }
}

const FAST_ENUM_BUF_SIZE: usize = 16;

pub struct NSFastEnumerator<'a, C: 'a + NSFastEnumeration + ?Sized> {
    object: &'a C,

    ptr: *const *const C::Item,
    end: *const *const C::Item,

    state: NSFastEnumerationState<C::Item>,
    buf: [*const C::Item; FAST_ENUM_BUF_SIZE],
}

impl<'a, C: NSFastEnumeration + ?Sized> NSFastEnumerator<'a, C> {
    fn new(object: &'a C) -> Self {
        Self {
            object,

            ptr: ptr::null(),
            end: ptr::null(),

            state: unsafe { mem::zeroed() },
            buf: [ptr::null(); FAST_ENUM_BUF_SIZE],
        }
    }

    fn update_buf(&mut self) -> bool {
        // If this isn't our first time enumerating, record the previous value
        // from the mutations pointer.
        let mutations = if !self.ptr.is_null() {
            Some(unsafe { *self.state.mutations_ptr })
        } else {
            None
        };

        let next_buf = enumerate(self.object, &mut self.state, &mut self.buf);
        if let Some(buf) = next_buf {
            // Check if the collection was mutated
            if let Some(mutations) = mutations {
                assert_eq!(
                    mutations,
                    unsafe { *self.state.mutations_ptr },
                    "Mutation detected during enumeration of object {:p}",
                    self.object
                );
            }

            self.ptr = buf.as_ptr();
            self.end = unsafe { self.ptr.add(buf.len()) };
            true
        } else {
            self.ptr = ptr::null();
            self.end = ptr::null();
            false
        }
    }
}

impl<'a, C: NSFastEnumeration + ?Sized> Iterator for NSFastEnumerator<'a, C> {
    type Item = &'a C::Item;

    fn next(&mut self) -> Option<&'a C::Item> {
        if self.ptr == self.end && !self.update_buf() {
            None
        } else {
            unsafe {
                let obj = *self.ptr;
                self.ptr = self.ptr.offset(1);
                Some(obj.as_ref().unwrap_unchecked())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::NSFastEnumeration;
    use crate::foundation::{NSArray, NSNumber};

    #[test]
    fn test_enumerator() {
        let vec = (0..4).map(NSNumber::new_usize).collect();
        let array = NSArray::from_vec(vec);

        let enumerator = array.iter();
        assert_eq!(enumerator.count(), 4);

        let enumerator = array.iter();
        assert!(enumerator.enumerate().all(|(i, obj)| obj.as_usize() == i));
    }

    #[test]
    fn test_fast_enumerator() {
        let vec = (0..4).map(NSNumber::new_usize).collect();
        let array = NSArray::from_vec(vec);

        let enumerator = array.iter_fast();
        assert_eq!(enumerator.count(), 4);

        let enumerator = array.iter_fast();
        assert!(enumerator.enumerate().all(|(i, obj)| obj.as_usize() == i));
    }
}
