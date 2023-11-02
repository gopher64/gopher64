//! Converting standard types into `ValueBag`s.

use super::ValueBag;

macro_rules! impl_from_internal {
    ($($into_ty:ty,)*) => {
        $(
            impl<'v> From<$into_ty> for ValueBag<'v> {
                #[inline]
                fn from(value: $into_ty) -> Self {
                    ValueBag::from_internal(value)
                }
            }

            impl<'a, 'v> From<&'a $into_ty> for ValueBag<'v> {
                #[inline]
                fn from(value: &'a $into_ty) -> Self {
                    ValueBag::from_internal(*value)
                }
            }
        )*
    };
}

impl_from_internal![
    (),
    usize,
    u8,
    u16,
    u32,
    u64,
    isize,
    i8,
    i16,
    i32,
    i64,
    f32,
    f64,
    char,
    bool,
];

impl<'v> From<&'v str> for ValueBag<'v> {
    #[inline]
    fn from(value: &'v str) -> Self {
        ValueBag::from_internal(value)
    }
}

impl<'v> From<&'v u128> for ValueBag<'v> {
    #[inline]
    fn from(value: &'v u128) -> Self {
        ValueBag::from_internal(value)
    }
}

impl<'v> From<&'v i128> for ValueBag<'v> {
    #[inline]
    fn from(value: &'v i128) -> Self {
        ValueBag::from_internal(value)
    }
}

#[cfg(feature = "inline-i128")]
impl<'v> From<u128> for ValueBag<'v> {
    #[inline]
    fn from(value: u128) -> Self {
        ValueBag::from_internal(value)
    }
}

#[cfg(feature = "inline-i128")]
impl<'v> From<i128> for ValueBag<'v> {
    #[inline]
    fn from(value: i128) -> Self {
        ValueBag::from_internal(value)
    }
}

#[cfg(feature = "std")]
mod std_support {
    use super::*;

    use crate::std::string::String;

    impl<'v> From<&'v String> for ValueBag<'v> {
        #[inline]
        fn from(v: &'v String) -> ValueBag<'v> {
            ValueBag::from_internal(&**v)
        }
    }
}

#[cfg(feature = "owned")]
mod owned_support {
    use super::*;

    use crate::OwnedValueBag;

    impl<'v> From<&'v OwnedValueBag> for ValueBag<'v> {
        #[inline]
        fn from(v: &'v OwnedValueBag) -> ValueBag<'v> {
            v.by_ref()
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    use crate::{
        std::{borrow::ToOwned, string::ToString},
        test::{IntoValueBag, TestToken},
    };

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_into_display() {
        assert_eq!(42u64.into_value_bag().by_ref().to_string(), "42");
        assert_eq!(42i64.into_value_bag().by_ref().to_string(), "42");
        assert_eq!(42.01f64.into_value_bag().by_ref().to_string(), "42.01");
        assert_eq!(true.into_value_bag().by_ref().to_string(), "true");
        assert_eq!('a'.into_value_bag().by_ref().to_string(), "a");
        assert_eq!(
            "a loong string".into_value_bag().by_ref().to_string(),
            "a loong string"
        );
        assert_eq!(().into_value_bag().by_ref().to_string(), "None");
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_into_structured() {
        assert_eq!(
            42u64.into_value_bag().by_ref().to_test_token(),
            TestToken::U64(42)
        );
        assert_eq!(
            42i64.into_value_bag().by_ref().to_test_token(),
            TestToken::I64(42)
        );
        assert_eq!(
            42.01f64.into_value_bag().by_ref().to_test_token(),
            TestToken::F64(42.01)
        );
        assert_eq!(
            true.into_value_bag().by_ref().to_test_token(),
            TestToken::Bool(true)
        );
        assert_eq!(
            'a'.into_value_bag().by_ref().to_test_token(),
            TestToken::Char('a')
        );
        assert_eq!(
            "a loong string".into_value_bag().by_ref().to_test_token(),
            TestToken::Str("a loong string".to_owned())
        );
        assert_eq!(
            ().into_value_bag().by_ref().to_test_token(),
            TestToken::None
        );

        #[cfg(feature = "inline-i128")]
        {
            assert_eq!(
                42u128.into_value_bag().by_ref().to_test_token(),
                TestToken::U128(42)
            );
            assert_eq!(
                42i128.into_value_bag().by_ref().to_test_token(),
                TestToken::I128(42)
            );
        }
    }
}
