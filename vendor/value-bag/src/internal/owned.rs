use crate::{
    internal::{self, Internal, InternalVisitor},
    std::boxed::Box,
    Error,
};

#[derive(Clone)]
pub(crate) enum OwnedInternal {
    /// An extra large signed integer.
    BigSigned(i128),
    /// An extra large unsigned integer.
    BigUnsigned(u128),
    /// A floating point number.
    Float(f64),
    /// A boolean value.
    Bool(bool),
    /// A UTF8 codepoint.
    Char(char),
    /// A UTF8 string.
    Str(Box<str>),
    /// An empty value.
    None,

    /// A debuggable value.
    Debug(internal::fmt::owned::OwnedFmt),
    /// A displayable value.
    Display(internal::fmt::owned::OwnedFmt),

    #[cfg(feature = "error")]
    Error(internal::error::owned::OwnedError),

    #[cfg(feature = "serde1")]
    Serde1(internal::serde::v1::owned::OwnedSerialize),

    #[cfg(feature = "sval2")]
    Sval2(internal::sval::v2::owned::OwnedValue),

    /// A poisoned value.
    Poisoned(&'static str),
}

impl OwnedInternal {
    pub(crate) fn by_ref<'v>(&'v self) -> Internal<'v> {
        match self {
            #[cfg(not(feature = "inline-i128"))]
            OwnedInternal::BigSigned(v) => Internal::BigSigned(v),
            #[cfg(feature = "inline-i128")]
            OwnedInternal::BigSigned(v) => Internal::BigSigned(*v),
            #[cfg(not(feature = "inline-i128"))]
            OwnedInternal::BigUnsigned(v) => Internal::BigUnsigned(v),
            #[cfg(feature = "inline-i128")]
            OwnedInternal::BigUnsigned(v) => Internal::BigUnsigned(*v),
            OwnedInternal::Float(v) => Internal::Float(*v),
            OwnedInternal::Bool(v) => Internal::Bool(*v),
            OwnedInternal::Char(v) => Internal::Char(*v),
            OwnedInternal::Str(v) => Internal::Str(v),
            OwnedInternal::None => Internal::None,

            OwnedInternal::Debug(v) => Internal::AnonDebug(v),
            OwnedInternal::Display(v) => Internal::AnonDisplay(v),

            #[cfg(feature = "error")]
            OwnedInternal::Error(v) => Internal::AnonError(v),

            #[cfg(feature = "serde1")]
            OwnedInternal::Serde1(v) => Internal::AnonSerde1(v),

            #[cfg(feature = "sval2")]
            OwnedInternal::Sval2(v) => Internal::AnonSval2(v),

            OwnedInternal::Poisoned(msg) => Internal::Poisoned(msg),
        }
    }
}

impl<'v> Internal<'v> {
    pub(crate) fn to_owned(&self) -> OwnedInternal {
        struct OwnedVisitor(OwnedInternal);

        impl<'v> InternalVisitor<'v> for OwnedVisitor {
            fn debug(&mut self, v: &dyn internal::fmt::Debug) -> Result<(), Error> {
                self.0 = OwnedInternal::Debug(internal::fmt::owned::buffer_debug(v));
                Ok(())
            }

            fn display(&mut self, v: &dyn internal::fmt::Display) -> Result<(), Error> {
                self.0 = OwnedInternal::Display(internal::fmt::owned::buffer_display(v));
                Ok(())
            }

            fn u64(&mut self, v: u64) -> Result<(), Error> {
                self.0 = OwnedInternal::BigUnsigned(v as u128);
                Ok(())
            }

            fn i64(&mut self, v: i64) -> Result<(), Error> {
                self.0 = OwnedInternal::BigSigned(v as i128);
                Ok(())
            }

            fn u128(&mut self, v: &u128) -> Result<(), Error> {
                self.0 = OwnedInternal::BigUnsigned(*v);
                Ok(())
            }

            fn i128(&mut self, v: &i128) -> Result<(), Error> {
                self.0 = OwnedInternal::BigSigned(*v);
                Ok(())
            }

            fn f64(&mut self, v: f64) -> Result<(), Error> {
                self.0 = OwnedInternal::Float(v);
                Ok(())
            }

            fn bool(&mut self, v: bool) -> Result<(), Error> {
                self.0 = OwnedInternal::Bool(v);
                Ok(())
            }

            fn char(&mut self, v: char) -> Result<(), Error> {
                self.0 = OwnedInternal::Char(v);
                Ok(())
            }

            fn str(&mut self, v: &str) -> Result<(), Error> {
                self.0 = OwnedInternal::Str(v.into());
                Ok(())
            }

            fn none(&mut self) -> Result<(), Error> {
                self.0 = OwnedInternal::None;
                Ok(())
            }

            #[cfg(feature = "error")]
            fn error(&mut self, v: &(dyn internal::error::Error + 'static)) -> Result<(), Error> {
                self.0 = OwnedInternal::Error(internal::error::owned::buffer(v));
                Ok(())
            }

            #[cfg(feature = "sval2")]
            fn sval2(&mut self, v: &dyn internal::sval::v2::Value) -> Result<(), Error> {
                self.0 = internal::sval::v2::owned::buffer(v)
                    .map(OwnedInternal::Sval2)
                    .unwrap_or(OwnedInternal::Poisoned("failed to buffer the value"));
                Ok(())
            }

            #[cfg(feature = "serde1")]
            fn serde1(&mut self, v: &dyn internal::serde::v1::Serialize) -> Result<(), Error> {
                self.0 = internal::serde::v1::owned::buffer(v)
                    .map(OwnedInternal::Serde1)
                    .unwrap_or(OwnedInternal::Poisoned("failed to buffer the value"));
                Ok(())
            }

            fn poisoned(&mut self, msg: &'static str) -> Result<(), Error> {
                self.0 = OwnedInternal::Poisoned(msg);
                Ok(())
            }
        }

        let mut visitor = OwnedVisitor(OwnedInternal::None);

        let _ = self.internal_visit(&mut visitor);

        visitor.0
    }
}
