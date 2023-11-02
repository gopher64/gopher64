use core::cmp::Ordering;
use core::fmt;
use core::hash;
use core::panic::{RefUnwindSafe, UnwindSafe};
use std::os::raw::{
    c_char, c_double, c_float, c_int, c_longlong, c_short, c_uchar, c_uint, c_ulonglong, c_ushort,
};

use super::{
    CGFloat, NSComparisonResult, NSCopying, NSInteger, NSObject, NSString, NSUInteger, NSValue,
};
use crate::rc::{Id, Shared};
use crate::{extern_class, extern_methods, msg_send, msg_send_id, ClassType, Encoding};

extern_class!(
    /// An object wrapper for primitive scalars.
    ///
    /// This is the Objective-C equivalant of a Rust enum containing the
    /// common scalar types `i8`, `u8`, `i16`, `u16`, `i32`, `u32`, `i64`,
    /// `u64`, `f32`, `f64` and the two C types `c_long` and `c_ulong`.
    ///
    /// All accessor methods are safe, though they may return unexpected
    /// results if the number was not created from said type. Consult [Apple's
    /// documentation][apple-doc] for details.
    ///
    /// Note that due to limitations in Objective-C type encodings, it is not
    /// possible to distinguish between an `NSNumber` created from [`bool`],
    /// and one created from an [`i8`]/[`u8`]. You should use the getter
    /// methods that fit your use-case instead!
    ///
    /// This does not implement [`Eq`] nor [`Ord`], since it may contain a
    /// floating point value. Beware that the implementation of [`PartialEq`]
    /// and [`PartialOrd`] does not properly handle NaNs either. Compare
    /// [`NSNumber::encoding`] with [`Encoding::Float`] or
    /// [`Encoding::Double`], and use [`NSNumber::as_f32`] or
    /// [`NSNumber::as_f64`] to get the desired floating point value directly.
    ///
    /// See [Apple's documentation][apple-doc] for more information.
    ///
    /// [apple-doc]: https://developer.apple.com/documentation/foundation/nsnumber?language=objc
    pub struct NSNumber;

    unsafe impl ClassType for NSNumber {
        #[inherits(NSObject)]
        type Super = NSValue;
    }
);

// SAFETY: `NSNumber` is just a wrapper around an integer/float/bool, and it
// is immutable.
unsafe impl Sync for NSNumber {}
unsafe impl Send for NSNumber {}

impl UnwindSafe for NSNumber {}
impl RefUnwindSafe for NSNumber {}

macro_rules! def_new_fn {
    {$(
        $(#[$($m:meta)*])*
        ($fn_name:ident($fn_inp:ty); $method_name:ident: $method_inp:ty),
    )*} => {$(
        $(#[$($m)*])*
        pub fn $fn_name(val: $fn_inp) -> Id<Self, Shared> {
            let val = val as $method_inp;
            unsafe {
                msg_send_id![Self::class(), $method_name: val]
            }
        }
    )*}
}

/// Creation methods.
impl NSNumber {
    def_new_fn! {
        (new_bool(bool); numberWithBool: bool),
        (new_i8(i8); numberWithChar: c_char),
        (new_u8(u8); numberWithUnsignedChar: c_uchar),
        (new_i16(i16); numberWithShort: c_short),
        (new_u16(u16); numberWithUnsignedShort: c_ushort),
        (new_i32(i32); numberWithInt: c_int),
        (new_u32(u32); numberWithUnsignedInt: c_uint),
        (new_i64(i64); numberWithLongLong: c_longlong),
        (new_u64(u64); numberWithUnsignedLongLong: c_ulonglong),
        (new_isize(isize); numberWithInteger: NSInteger),
        (new_usize(usize); numberWithUnsignedInteger: NSUInteger),
        (new_f32(f32); numberWithFloat: c_float),
        (new_f64(f64); numberWithDouble: c_double),
    }

    #[inline]
    pub fn new_cgfloat(val: CGFloat) -> Id<Self, Shared> {
        #[cfg(target_pointer_width = "64")]
        {
            Self::new_f64(val)
        }
        #[cfg(not(target_pointer_width = "64"))]
        {
            Self::new_f32(val)
        }
    }
}

macro_rules! def_get_fn {
    {$(
        $(#[$($m:meta)*])*
        ($fn_name:ident -> $fn_ret:ty; $method_name:ident -> $method_ret:ty),
    )*} => {$(
        $(#[$($m)*])*
        pub fn $fn_name(&self) -> $fn_ret {
            let ret: $method_ret = unsafe { msg_send![self, $method_name] };
            ret as $fn_ret
        }
    )*}
}

/// Getter methods.
impl NSNumber {
    def_get_fn! {
        (as_bool -> bool; boolValue -> bool),
        (as_i8 -> i8; charValue -> c_char),
        (as_u8 -> u8; unsignedCharValue -> c_uchar),
        (as_i16 -> i16; shortValue -> c_short),
        (as_u16 -> u16; unsignedShortValue -> c_ushort),
        (as_i32 -> i32; intValue -> c_int),
        (as_u32 -> u32; unsignedIntValue -> c_uint),
        // TODO: Getter methods for `long` and `unsigned long`
        (as_i64 -> i64; longLongValue -> c_longlong),
        (as_u64 -> u64; unsignedLongLongValue -> c_ulonglong),
        (as_isize -> isize; integerValue -> NSInteger),
        (as_usize -> usize; unsignedIntegerValue -> NSUInteger),
        (as_f32 -> f32; floatValue -> c_float),
        (as_f64 -> f64; doubleValue -> c_double),
    }

    #[inline]
    pub fn as_cgfloat(&self) -> CGFloat {
        #[cfg(target_pointer_width = "64")]
        {
            self.as_f64()
        }
        #[cfg(not(target_pointer_width = "64"))]
        {
            self.as_f32()
        }
    }

    /// The Objective-C encoding of this `NSNumber`.
    ///
    /// This is guaranteed to return one of:
    /// - [`Encoding::Char`]
    /// - [`Encoding::UChar`]
    /// - [`Encoding::Short`]
    /// - [`Encoding::UShort`]
    /// - [`Encoding::Int`]
    /// - [`Encoding::UInt`]
    /// - [`Encoding::Long`]
    /// - [`Encoding::ULong`]
    /// - [`Encoding::LongLong`]
    /// - [`Encoding::ULongLong`]
    /// - [`Encoding::Float`]
    /// - [`Encoding::Double`]
    ///
    ///
    /// # Examples
    ///
    /// Convert an `NSNumber` to/from an enumeration describing the different
    /// number properties.
    ///
    /// ```
    /// use objc2::Encoding;
    /// use objc2::foundation::NSNumber;
    /// use objc2::rc::{Id, Shared};
    ///
    /// // Note: `bool` would convert to either `Signed` or `Unsigned`,
    /// // depending on platform
    /// #[derive(Copy, Clone)]
    /// pub enum Number {
    ///     Signed(i64),
    ///     Unsigned(u64),
    ///     Floating(f64),
    /// }
    ///
    /// impl Number {
    ///     fn into_nsnumber(self) -> Id<NSNumber, Shared> {
    ///         match self {
    ///             Self::Signed(val) => NSNumber::new_i64(val),
    ///             Self::Unsigned(val) => NSNumber::new_u64(val),
    ///             Self::Floating(val) => NSNumber::new_f64(val),
    ///         }
    ///     }
    /// }
    ///
    /// impl From<&NSNumber> for Number {
    ///     fn from(n: &NSNumber) -> Self {
    ///         match n.encoding() {
    ///             Encoding::Char
    ///             | Encoding::Short
    ///             | Encoding::Int
    ///             | Encoding::Long
    ///             | Encoding::LongLong => Self::Signed(n.as_i64()),
    ///             Encoding::UChar
    ///             | Encoding::UShort
    ///             | Encoding::UInt
    ///             | Encoding::ULong
    ///             | Encoding::ULongLong => Self::Unsigned(n.as_u64()),
    ///             Encoding::Float
    ///             | Encoding::Double => Self::Floating(n.as_f64()),
    ///             _ => unreachable!(),
    ///         }
    ///     }
    /// }
    /// ```
    pub fn encoding(&self) -> Encoding {
        // Use NSValue::encoding
        let enc = (**self)
            .encoding()
            .expect("NSNumber must have an encoding!");

        // Guaranteed under "Subclassing Notes"
        // <https://developer.apple.com/documentation/foundation/nsnumber?language=objc#1776615>
        match enc {
            "c" => Encoding::Char,
            "C" => Encoding::UChar,
            "s" => Encoding::Short,
            "S" => Encoding::UShort,
            "i" => Encoding::Int,
            "I" => Encoding::UInt,
            "l" => Encoding::Long,
            "L" => Encoding::ULong,
            "q" => Encoding::LongLong,
            "Q" => Encoding::ULongLong,
            "f" => Encoding::Float,
            "d" => Encoding::Double,
            _ => unreachable!("invalid encoding for NSNumber"),
        }
    }
}

extern_methods!(
    unsafe impl NSNumber {
        #[sel(compare:)]
        fn compare(&self, other: &Self) -> NSComparisonResult;

        #[sel(isEqualToNumber:)]
        fn is_equal_to_number(&self, other: &Self) -> bool;

        fn string(&self) -> Id<NSString, Shared> {
            unsafe { msg_send_id![self, stringValue] }
        }
    }
);

unsafe impl NSCopying for NSNumber {
    type Ownership = Shared;
    type Output = NSNumber;
}

impl alloc::borrow::ToOwned for NSNumber {
    type Owned = Id<NSNumber, Shared>;
    fn to_owned(&self) -> Self::Owned {
        self.copy()
    }
}

/// Beware: This uses the Objective-C method "isEqualToNumber:", which has
/// different floating point NaN semantics than Rust!
impl PartialEq for NSNumber {
    #[doc(alias = "isEqualToNumber:")]
    fn eq(&self, other: &Self) -> bool {
        // Use isEqualToNumber: instaed of isEqual: since it is faster
        self.is_equal_to_number(other)
    }
}

impl hash::Hash for NSNumber {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        // Delegate to NSObject
        (***self).hash(state)
    }
}

/// Beware: This uses the Objective-C method "compare:", which has different
/// floating point NaN semantics than Rust!
impl PartialOrd for NSNumber {
    #[doc(alias = "compare:")]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Use Objective-C semantics for comparison
        Some(self.compare(other).into())
    }
}

impl fmt::Display for NSNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.string(), f)
    }
}

impl fmt::Debug for NSNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Delegate to -[NSObject description]
        // (happens to return the same as -[NSNumber stringValue])
        fmt::Debug::fmt(&***self, f)
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;

    use super::*;

    #[test]
    fn basic() {
        let val = NSNumber::new_u32(13);
        assert_eq!(val.as_u32(), 13);
    }

    #[test]
    fn roundtrip() {
        assert!(NSNumber::new_bool(true).as_bool());
        assert!(!NSNumber::new_bool(false).as_bool());

        fn assert_roundtrip_signed(val: i64) {
            assert_eq!(NSNumber::new_i8(val as i8).as_i8(), val as i8);
            assert_eq!(NSNumber::new_i16(val as i16).as_i16(), val as i16);
            assert_eq!(NSNumber::new_i32(val as i32).as_i32(), val as i32);
            assert_eq!(NSNumber::new_i64(val).as_i64(), val);
            assert_eq!(NSNumber::new_isize(val as isize).as_isize(), val as isize);
        }

        assert_roundtrip_signed(i64::MIN);
        assert_roundtrip_signed(i32::MIN as i64);
        assert_roundtrip_signed(i16::MIN as i64);
        assert_roundtrip_signed(i8::MIN as i64);
        assert_roundtrip_signed(-1);
        assert_roundtrip_signed(0);
        assert_roundtrip_signed(1);
        assert_roundtrip_signed(i8::MAX as i64);
        assert_roundtrip_signed(i16::MAX as i64);
        assert_roundtrip_signed(i32::MAX as i64);
        assert_roundtrip_signed(i64::MAX);

        fn assert_roundtrip_unsigned(val: u64) {
            assert_eq!(NSNumber::new_u8(val as u8).as_u8(), val as u8);
            assert_eq!(NSNumber::new_u16(val as u16).as_u16(), val as u16);
            assert_eq!(NSNumber::new_u32(val as u32).as_u32(), val as u32);
            assert_eq!(NSNumber::new_u64(val).as_u64(), val);
            assert_eq!(NSNumber::new_usize(val as usize).as_usize(), val as usize);
        }

        assert_roundtrip_unsigned(0);
        assert_roundtrip_unsigned(1);
        assert_roundtrip_unsigned(u8::MAX as u64);
        assert_roundtrip_unsigned(u16::MAX as u64);
        assert_roundtrip_unsigned(u32::MAX as u64);
        assert_roundtrip_unsigned(u64::MAX);

        fn assert_roundtrip_float(val: f64) {
            assert_eq!(NSNumber::new_f32(val as f32).as_f32(), val as f32);
            assert_eq!(NSNumber::new_f64(val).as_f64(), val);
        }

        assert_roundtrip_float(0.0);
        assert_roundtrip_float(-1.0);
        assert_roundtrip_float(1.0);
        assert_roundtrip_float(f64::INFINITY);
        assert_roundtrip_float(-f64::INFINITY);
        assert_roundtrip_float(f64::MAX);
        assert_roundtrip_float(f64::MIN);
        assert_roundtrip_float(f64::MIN_POSITIVE);

        assert!(NSNumber::new_f32(f32::NAN).as_f32().is_nan());
        assert!(NSNumber::new_f64(f64::NAN).as_f64().is_nan());
        assert!(NSNumber::new_f32(-f32::NAN).as_f32().is_nan());
        assert!(NSNumber::new_f64(-f64::NAN).as_f64().is_nan());
    }

    #[test]
    fn cast_between_types() {
        assert_eq!(NSNumber::new_bool(true).as_i8(), 1);
        assert_eq!(NSNumber::new_i32(i32::MAX).as_u32(), i32::MAX as u32);
        assert_eq!(NSNumber::new_f32(1.0).as_u32(), 1);
        assert_eq!(NSNumber::new_f32(1.0).as_u32(), 1);
    }

    #[test]
    fn equality() {
        let val1 = NSNumber::new_u32(123);
        let val2 = NSNumber::new_u32(123);
        let val3 = NSNumber::new_u8(123);
        assert_eq!(val1, val1);
        assert_eq!(val1, val2);
        assert_eq!(val1, val3);

        let val4 = NSNumber::new_u32(456);
        assert_ne!(val1, val4);
    }

    #[test]
    fn display_debug() {
        fn assert_display_debug<T: fmt::Debug + fmt::Display>(val: T, expected: &str) {
            // The two impls for these happen to be the same
            assert_eq!(format!("{}", val), expected);
            assert_eq!(format!("{:?}", val), expected);
        }
        assert_display_debug(NSNumber::new_u8(171), "171");
        assert_display_debug(NSNumber::new_i8(-12), "-12");
        assert_display_debug(NSNumber::new_u32(0xdeadbeef), "3735928559");
        assert_display_debug(NSNumber::new_f32(1.1), "1.1");
        assert_display_debug(NSNumber::new_f32(1.0), "1");
        assert_display_debug(NSNumber::new_bool(true), "1");
        assert_display_debug(NSNumber::new_bool(false), "0");
    }
}
