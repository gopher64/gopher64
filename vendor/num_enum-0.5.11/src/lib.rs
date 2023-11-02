// Wrap this in two cfg_attrs so that it continues to parse pre-1.54.0.
// See https://github.com/rust-lang/rust/issues/82768
#![cfg_attr(feature = "external_doc", cfg_attr(all(), doc = include_str!("../README.md")))]
#![cfg_attr(
    not(feature = "external_doc"),
    doc = "See <https://docs.rs/num_enum> for more info about this crate."
)]
#![cfg_attr(not(feature = "std"), no_std)]

pub use ::num_enum_derive::{
    Default, FromPrimitive, IntoPrimitive, TryFromPrimitive, UnsafeFromPrimitive,
};

use ::core::fmt;

pub trait FromPrimitive: Sized {
    type Primitive: Copy + Eq;

    fn from_primitive(number: Self::Primitive) -> Self;
}

pub trait TryFromPrimitive: Sized {
    type Primitive: Copy + Eq + fmt::Debug;

    const NAME: &'static str;

    fn try_from_primitive(number: Self::Primitive) -> Result<Self, TryFromPrimitiveError<Self>>;
}

pub struct TryFromPrimitiveError<Enum: TryFromPrimitive> {
    pub number: Enum::Primitive,
}

impl<Enum: TryFromPrimitive> Copy for TryFromPrimitiveError<Enum> {}
impl<Enum: TryFromPrimitive> Clone for TryFromPrimitiveError<Enum> {
    fn clone(&self) -> Self {
        TryFromPrimitiveError {
            number: self.number,
        }
    }
}
impl<Enum: TryFromPrimitive> Eq for TryFromPrimitiveError<Enum> {}
impl<Enum: TryFromPrimitive> PartialEq for TryFromPrimitiveError<Enum> {
    fn eq(&self, other: &Self) -> bool {
        self.number == other.number
    }
}
impl<Enum: TryFromPrimitive> fmt::Debug for TryFromPrimitiveError<Enum> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("TryFromPrimitiveError")
            .field("number", &self.number)
            .finish()
    }
}
impl<Enum: TryFromPrimitive> fmt::Display for TryFromPrimitiveError<Enum> {
    fn fmt(&self, stream: &'_ mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            stream,
            "No discriminant in enum `{name}` matches the value `{input:?}`",
            name = Enum::NAME,
            input = self.number,
        )
    }
}

#[cfg(feature = "std")]
impl<Enum: TryFromPrimitive> ::std::error::Error for TryFromPrimitiveError<Enum> {}
