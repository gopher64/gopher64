use alloc::string::ToString;
use core::ffi::c_void;
use core::fmt;
use core::hash;
use core::mem::MaybeUninit;
use core::ptr::NonNull;
use core::str;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use super::{NSCopying, NSObject, NSPoint, NSRange, NSRect, NSSize};
use crate::rc::{Id, Shared};
use crate::{extern_class, extern_methods, msg_send, msg_send_id, ClassType, Encode};

extern_class!(
    /// A container wrapping any encodable type as an Obective-C object.
    ///
    /// Since Objective-C collections like [`NSArray`] can only contain
    /// objects, it is common to wrap pointers or structures like [`NSRange`].
    ///
    /// Note that creating `NSValue`s is not `unsafe`, but almost all usage of
    /// it is, since we cannot guarantee that the type that was used to
    /// construct it is the same as the expected output type.
    ///
    /// See also the [`NSNumber`] subclass for when you want to wrap numbers.
    ///
    /// See [Apple's documentation][apple-doc] for more information.
    ///
    /// [`NSArray`]: super::NSArray
    /// [`NSRange`]: super::NSRange
    /// [`NSNumber`]: super::NSNumber
    /// [apple-doc]: https://developer.apple.com/documentation/foundation/nsnumber?language=objc
    pub struct NSValue;

    unsafe impl ClassType for NSValue {
        type Super = NSObject;
    }
);

// We can't implement any auto traits for NSValue, since it can contain an
// arbitary object!

extern_methods!(
    /// Creation methods.
    unsafe impl NSValue {
        // Default / empty new is not provided because `-init` returns `nil` on
        // Apple and GNUStep throws an exception on all other messages to this
        // invalid instance.

        /// Create a new `NSValue` containing the given type.
        ///
        /// Be careful when using this since you may accidentally pass a reference
        /// when you wanted to pass a concrete type instead.
        ///
        ///
        /// # Examples
        ///
        /// Create an `NSValue` containing an [`NSPoint`][super::NSPoint].
        ///
        /// ```
        /// use objc2::foundation::{NSPoint, NSValue};
        /// # #[cfg(feature = "gnustep-1-7")]
        /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
        /// let val = NSValue::new::<NSPoint>(NSPoint::new(1.0, 1.0));
        /// ```
        pub fn new<T: 'static + Copy + Encode>(value: T) -> Id<Self, Shared> {
            let bytes: *const T = &value;
            let bytes: *const c_void = bytes.cast();
            let encoding = CString::new(T::ENCODING.to_string()).unwrap();
            unsafe {
                msg_send_id![
                    msg_send_id![Self::class(), alloc],
                    initWithBytes: bytes,
                    objCType: encoding.as_ptr(),
                ]
            }
        }
    }

    /// Getter methods.
    unsafe impl NSValue {
        /// Retrieve the data contained in the `NSValue`.
        ///
        /// Note that this is broken on GNUStep for some types, see
        /// [gnustep/libs-base#216].
        ///
        /// [gnustep/libs-base#216]: https://github.com/gnustep/libs-base/pull/216
        ///
        ///
        /// # Safety
        ///
        /// The type of `T` must be what the NSValue actually stores, and any
        /// safety invariants that the value has must be upheld.
        ///
        /// Note that it may be, but is not always, enough to simply check whether
        /// [`contains_encoding`] returns `true`. For example, `NonNull<T>` have
        /// the same encoding as `*const T`, but `NonNull<T>` is clearly not
        /// safe to return from this function even if you've checked the encoding
        /// beforehand.
        ///
        /// [`contains_encoding`]: Self::contains_encoding
        ///
        ///
        /// # Examples
        ///
        /// Store a pointer in `NSValue`, and retrieve it again afterwards.
        ///
        /// ```
        /// use std::ffi::c_void;
        /// use std::ptr;
        /// use objc2::foundation::NSValue;
        ///
        /// # #[cfg(feature = "gnustep-1-7")]
        /// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
        /// let val = NSValue::new::<*const c_void>(ptr::null());
        /// // SAFETY: The value was just created with a pointer
        /// let res = unsafe { val.get::<*const c_void>() };
        /// assert!(res.is_null());
        /// ```
        pub unsafe fn get<T: 'static + Copy + Encode>(&self) -> T {
            debug_assert!(
            self.contains_encoding::<T>(),
            "wrong encoding. NSValue tried to return something with encoding {}, but the encoding of the given type was {}",
            self.encoding().unwrap_or("(NULL)"),
            T::ENCODING,
        );
            let mut value = MaybeUninit::<T>::uninit();
            let ptr: *mut c_void = value.as_mut_ptr().cast();
            let _: () = unsafe { msg_send![self, getValue: ptr] };
            // SAFETY: We know that `getValue:` initialized the value, and user
            // ensures that it is safe to access.
            unsafe { value.assume_init() }
        }

        pub fn get_range(&self) -> Option<NSRange> {
            if self.contains_encoding::<NSRange>() {
                // SAFETY: We just checked that this contains an NSRange
                Some(unsafe { msg_send![self, rangeValue] })
            } else {
                None
            }
        }

        pub fn get_point(&self) -> Option<NSPoint> {
            if self.contains_encoding::<NSPoint>() {
                // SAFETY: We just checked that this contains an NSPoint
                //
                // Note: The documentation says that `pointValue`, `sizeValue` and
                // `rectValue` is only available on macOS, but turns out that they
                // are actually available everywhere!
                let res = unsafe { msg_send![self, pointValue] };
                Some(res)
            } else {
                None
            }
        }

        pub fn get_size(&self) -> Option<NSSize> {
            if self.contains_encoding::<NSSize>() {
                // SAFETY: We just checked that this contains an NSSize
                let res = unsafe { msg_send![self, sizeValue] };
                Some(res)
            } else {
                None
            }
        }

        pub fn get_rect(&self) -> Option<NSRect> {
            if self.contains_encoding::<NSRect>() {
                // SAFETY: We just checked that this contains an NSRect
                let res = unsafe { msg_send![self, rectValue] };
                Some(res)
            } else {
                None
            }
        }

        pub fn encoding(&self) -> Option<&str> {
            let result: Option<NonNull<c_char>> = unsafe { msg_send![self, objCType] };
            result.map(|s| unsafe { CStr::from_ptr(s.as_ptr()) }.to_str().unwrap())
        }

        pub fn contains_encoding<T: 'static + Copy + Encode>(&self) -> bool {
            if let Some(encoding) = self.encoding() {
                T::ENCODING.equivalent_to_str(encoding)
            } else {
                panic!("missing NSValue encoding");
            }
        }

        #[sel(isEqualToValue:)]
        fn is_equal_to_value(&self, other: &Self) -> bool;
    }
);

unsafe impl NSCopying for NSValue {
    type Ownership = Shared;
    type Output = NSValue;
}

impl alloc::borrow::ToOwned for NSValue {
    type Owned = Id<NSValue, Shared>;
    fn to_owned(&self) -> Self::Owned {
        self.copy()
    }
}

impl PartialEq for NSValue {
    #[doc(alias = "isEqualToValue:")]
    fn eq(&self, other: &Self) -> bool {
        // Use isEqualToValue: instaed of isEqual: since it is faster
        self.is_equal_to_value(other)
    }
}

impl hash::Hash for NSValue {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        // Delegate to NSObject
        (**self).hash(state)
    }
}

impl fmt::Debug for NSValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let enc = self.encoding().unwrap_or("(NULL)");
        let bytes = &**self; // Delegate to -[NSObject description]
        f.debug_struct("NSValue")
            .field("encoding", &enc)
            .field("bytes", bytes)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;
    use core::{ptr, slice};

    use super::*;
    use crate::rc::{RcTestObject, ThreadTestData};

    #[test]
    fn basic() {
        let val = NSValue::new(13u32);
        assert_eq!(unsafe { val.get::<u32>() }, 13);
    }

    #[test]
    fn does_not_retain() {
        let obj = RcTestObject::new();
        let expected = ThreadTestData::current();

        let val = NSValue::new::<*const RcTestObject>(&*obj);
        expected.assert_current();

        assert!(ptr::eq(unsafe { val.get::<*const RcTestObject>() }, &*obj));
        expected.assert_current();

        let _clone = val.clone();
        expected.assert_current();

        let _copy = val.copy();
        expected.assert_current();

        drop(val);
        expected.assert_current();
    }

    #[test]
    fn test_equality() {
        let val1 = NSValue::new(123u32);
        let val2 = NSValue::new(123u32);
        assert_eq!(val1, val1);
        assert_eq!(val1, val2);

        let val3 = NSValue::new(456u32);
        assert_ne!(val1, val3);
    }

    #[test]
    fn test_equality_across_types() {
        let val1 = NSValue::new(123i32);
        let val2 = NSValue::new(123u32);

        // Test that `objCType` is checked when comparing equality
        assert_ne!(val1, val2);
    }

    #[test]
    #[ignore = "the debug output changes depending on OS version"]
    fn test_debug() {
        let expected = if cfg!(feature = "gnustep-1-7") {
            r#"NSValue { encoding: "C", bytes: (C) <ab> }"#
        } else if cfg!(newer_apple) {
            r#"NSValue { encoding: "C", bytes: {length = 1, bytes = 0xab} }"#
        } else {
            r#"NSValue { encoding: "C", bytes: <ab> }"#
        };
        assert_eq!(format!("{:?}", NSValue::new(171u8)), expected);
    }

    #[test]
    fn nsrange() {
        let range = NSRange::from(1..2);
        let val = NSValue::new(range);
        assert_eq!(val.get_range(), Some(range));
        assert_eq!(val.get_point(), None);
        assert_eq!(val.get_size(), None);
        assert_eq!(val.get_rect(), None);
        // NSValue -getValue is broken on GNUStep for some types
        #[cfg(not(feature = "gnustep-1-7"))]
        assert_eq!(unsafe { val.get::<NSRange>() }, range);
    }

    #[test]
    fn nspoint() {
        let point = NSPoint::new(1.0, 2.0);
        let val = NSValue::new(point);
        assert_eq!(val.get_point(), Some(point));
        #[cfg(not(feature = "gnustep-1-7"))]
        assert_eq!(unsafe { val.get::<NSPoint>() }, point);
    }

    #[test]
    fn nssize() {
        let point = NSSize::new(1.0, 2.0);
        let val = NSValue::new(point);
        assert_eq!(val.get_size(), Some(point));
        #[cfg(not(feature = "gnustep-1-7"))]
        assert_eq!(unsafe { val.get::<NSSize>() }, point);
    }

    #[test]
    fn nsrect() {
        let rect = NSRect::new(NSPoint::new(1.0, 2.0), NSSize::new(3.0, 4.0));
        let val = NSValue::new(rect);
        assert_eq!(val.get_rect(), Some(rect));
        #[cfg(not(feature = "gnustep-1-7"))]
        assert_eq!(unsafe { val.get::<NSRect>() }, rect);
    }

    #[test]
    fn store_str() {
        let s = "abc";
        let val = NSValue::new(s.as_ptr());
        assert!(val.contains_encoding::<*const u8>());
        let slice = unsafe { slice::from_raw_parts(val.get(), s.len()) };
        let s2 = str::from_utf8(slice).unwrap();
        assert_eq!(s2, s);
    }

    #[test]
    fn store_cstr() {
        // The following Apple article says that NSValue can't easily store
        // C-strings, but apparently that doesn't apply to us!
        // <https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/NumbersandValues/Articles/Values.html#//apple_ref/doc/uid/20000174-BAJJHDEG>
        let s = CStr::from_bytes_with_nul(b"test123\0").unwrap();
        let val = NSValue::new(s.as_ptr());
        assert!(val.contains_encoding::<*const c_char>());
        let s2 = unsafe { CStr::from_ptr(val.get()) };
        assert_eq!(s2, s);
    }
}
