use objc2_encode::{Encode, Encoding, RefEncode};

/// We don't know the size of NSString, so we can only hold pointers to it.
///
/// TODO: Use [`extern type`][rfc-1861] when that gets stabilized.
///
/// [rfc-1861]: https://rust-lang.github.io/rfcs/1861-extern-types.html
#[repr(C)]
struct NSString {
    _priv: [u8; 0],
}

/// Implement `RefEncode` for pointers and references to the string.
unsafe impl RefEncode for NSString {
    const ENCODING_REF: Encoding = Encoding::Object;
}

fn main() {
    println!("{}", <*const NSString>::ENCODING);
    println!("{}", <*mut NSString>::ENCODING);
    println!("{}", <&NSString>::ENCODING);
    println!("{}", <&mut NSString>::ENCODING);
    println!("{}", Option::<&NSString>::ENCODING);
    println!("{}", Option::<&mut NSString>::ENCODING);
}
