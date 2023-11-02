use crate::{internal, ValueBag};

/// A dynamic structured value.
///
/// This type is an owned variant of [`ValueBag`] that can be
/// constructed using its [`to_owned`](struct.ValueBag.html#method.to_owned) method.
/// `OwnedValueBag`s are suitable for storing and sharing across threads.
///
/// `OwnedValueBag`s can be inspected by converting back into a regular `ValueBag`
/// using the [`by_ref`](#method.by_ref) method.
#[derive(Clone)]
pub struct OwnedValueBag {
    inner: internal::owned::OwnedInternal,
}

impl<'v> ValueBag<'v> {
    /// Buffer this value into an [`OwnedValueBag`].
    pub fn to_owned(&self) -> OwnedValueBag {
        OwnedValueBag {
            inner: self.inner.to_owned(),
        }
    }
}

impl OwnedValueBag {
    /// Get a regular [`ValueBag`] from this type.
    ///
    /// Once a `ValueBag` has been buffered, it will behave
    /// slightly differently when converted back:
    ///
    /// - `fmt::Debug` won't use formatting flags.
    /// - `serde::Serialize` will use the text-based representation.
    /// - The original type will change, so downcasting won't work.
    pub fn by_ref<'v>(&'v self) -> ValueBag<'v> {
        ValueBag {
            inner: self.inner.by_ref(),
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    use super::*;

    use crate::{
        fill,
        std::{mem, string::ToString},
    };

    const SIZE_LIMIT_U64: usize = 4;

    #[test]
    fn is_send_sync() {
        fn assert<T: Send + Sync + 'static>() {}

        assert::<OwnedValueBag>();
    }

    #[test]
    fn owned_value_bag_size() {
        let size = mem::size_of::<OwnedValueBag>();
        let limit = mem::size_of::<u64>() * SIZE_LIMIT_U64;

        if size > limit {
            panic!(
                "`OwnedValueBag` size ({} bytes) is too large (expected up to {} bytes)",
                size, limit,
            );
        }
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn fill_to_owned() {
        let value = ValueBag::from_fill(&|slot: fill::Slot| slot.fill_any(42u64)).to_owned();

        assert!(matches!(
            value.inner,
            internal::owned::OwnedInternal::BigUnsigned(42)
        ));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn fmt_to_owned() {
        let debug = ValueBag::from_debug(&"a value").to_owned();
        let display = ValueBag::from_display(&"a value").to_owned();

        assert!(matches!(
            debug.inner,
            internal::owned::OwnedInternal::Debug(_)
        ));
        assert!(matches!(
            display.inner,
            internal::owned::OwnedInternal::Display(_)
        ));

        assert_eq!("\"a value\"", debug.to_string());
        assert_eq!("a value", display.to_string());

        let debug = debug.by_ref();
        let display = display.by_ref();

        assert!(matches!(debug.inner, internal::Internal::AnonDebug(_)));
        assert!(matches!(display.inner, internal::Internal::AnonDisplay(_)));
    }

    #[test]
    #[cfg(feature = "error")]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn error_to_owned() {
        use crate::std::io;

        let value =
            ValueBag::from_dyn_error(&io::Error::new(io::ErrorKind::Other, "something failed!"))
                .to_owned();

        assert!(matches!(
            value.inner,
            internal::owned::OwnedInternal::Error(_)
        ));

        let value = value.by_ref();

        assert!(matches!(value.inner, internal::Internal::AnonError(_)));

        assert!(value.to_borrowed_error().is_some());
    }

    #[test]
    #[cfg(feature = "serde1")]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn serde1_to_owned() {
        let value = ValueBag::from_serde1(&42u64).to_owned();

        assert!(matches!(
            value.inner,
            internal::owned::OwnedInternal::Serde1(_)
        ));

        let value = value.by_ref();

        assert!(matches!(value.inner, internal::Internal::AnonSerde1(_)));
    }

    #[test]
    #[cfg(feature = "sval2")]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn sval2_to_owned() {
        let value = ValueBag::from_sval2(&42u64).to_owned();

        assert!(matches!(
            value.inner,
            internal::owned::OwnedInternal::Sval2(_)
        ));

        let value = value.by_ref();

        assert!(matches!(value.inner, internal::Internal::AnonSval2(_)));
    }
}
