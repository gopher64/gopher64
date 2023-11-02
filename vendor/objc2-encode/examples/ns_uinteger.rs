//! Implementing `Encode` and `RefEncode` for `NSUInteger`.
//!
//! Note that in this case `NSUInteger` could actually just be a type alias
//! for `usize`, and that's already available under `objc2::ffi::NSUInteger`.
use objc2_encode::{Encode, Encoding, RefEncode};

#[repr(transparent)]
struct NSUInteger {
    _inner: usize,
}

// SAFETY: `NSUInteger` has the same `repr` as `usize`.
unsafe impl Encode for NSUInteger {
    /// Running `@encode(NSUInteger)` gives `Q` on 64-bit systems and `I` on
    /// 32-bit systems. This corresponds exactly to `usize`, which is also how
    /// we've defined our struct.
    const ENCODING: Encoding = usize::ENCODING;
}

// SAFETY: `&NSUInteger` has the same representation as `&usize`.
unsafe impl RefEncode for NSUInteger {
    /// Running `@encode(NSUInteger*)` gives `^Q` on 64-bit systems and `^I`
    /// on 32-bit systems. So implementing `RefEncode` as a plain pointer is
    /// correct.
    const ENCODING_REF: Encoding = Encoding::Pointer(&NSUInteger::ENCODING);
}

fn main() {
    assert!(NSUInteger::ENCODING.equivalent_to_str("Q"));
    assert!(<&NSUInteger>::ENCODING.equivalent_to_str("^Q"));
    assert!(<&NSUInteger>::ENCODING.equivalent_to_str("r^Q"));
}
