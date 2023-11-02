//! Implementing `RefEncode` for `NSDecimal`.
use objc2_encode::{Encoding, RefEncode};

/// We choose in this case to represent `NSDecimal` as an opaque struct
/// (and in the future as an `extern type`) because we don't know much
/// about the internals.
///
/// Therefore we do not implement `Encode`, but when implementing `RefEncode`
/// the type-encoding still has to be correct.
#[repr(C)]
struct NSDecimal {
    _priv: [u8; 0],
}

// SAFETY: `&NSDecimal` is a pointer.
unsafe impl RefEncode for NSDecimal {
    // Running `@encode` on `NSDecimal*` on my 64-bit system gives `^{?=cCCC[38C]}`.
    const ENCODING_REF: Encoding = Encoding::Pointer(&Encoding::Struct(
        "?",
        &[
            Encoding::Char,
            Encoding::UChar,
            Encoding::UChar,
            Encoding::UChar,
            Encoding::Array(38, &Encoding::UChar),
        ],
    ));
}

fn main() {
    assert!(NSDecimal::ENCODING_REF.equivalent_to_str("^{?=cCCC[38C]}"));
    // Does not compile:
    // println!("{:?}", NSDecimal::ENCODING);
}
