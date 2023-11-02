use ::num_enum::UnsafeFromPrimitive;

// Guard against https://github.com/illicitonion/num_enum/issues/27
mod alloc {}
mod core {}
mod num_enum {}
mod std {}

#[test]
fn has_unsafe_from_primitive_number() {
    #[derive(Debug, Eq, PartialEq, UnsafeFromPrimitive)]
    #[repr(u8)]
    enum Enum {
        Zero,
        One,
    }

    unsafe {
        assert_eq!(Enum::from_unchecked(0_u8), Enum::Zero);
        assert_eq!(Enum::from_unchecked(1_u8), Enum::One);
    }
}
