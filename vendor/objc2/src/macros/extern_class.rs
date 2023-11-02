/// Create a new type to represent an Objective-C class.
///
/// This is similar to an `@interface` declaration in Objective-C.
///
/// The given struct name should correspond to a valid Objective-C class,
/// whose instances have the encoding [`Encoding::Object`]. (as an example:
/// `NSAutoreleasePool` does not have this!)
///
/// You must specify the superclass of this class, similar to how you would
/// in Objective-C.
///
/// Due to Rust trait limitations, specifying e.g. the superclass `NSData`
/// would not give you easy access to `NSObject`'s functionality. Therefore,
/// you may specify additional parts of the inheritance chain using the
/// `#[inherits(...)]` attribute.
///
/// [`Encoding::Object`]: crate::Encoding::Object
///
///
/// # Specification
///
/// The syntax is similar enough to Rust syntax that if you invoke the macro
/// with parentheses (as opposed to curly brackets), [`rustfmt` will be able to
/// format the contents][rustfmt-macros].
///
/// This creates an opaque struct containing the superclass (which means that
/// auto traits are inherited from the superclass), and implements the
/// following traits for it to allow easier usage as an Objective-C object:
///
/// - [`RefEncode`][crate::RefEncode]
/// - [`Message`][crate::Message]
/// - [`ClassType`][crate::ClassType]
/// - [`Deref<Target = $superclass>`][core::ops::Deref]
/// - [`DerefMut`][core::ops::DerefMut]
/// - [`AsRef<$inheritance_chain>`][AsRef]
/// - [`AsMut<$inheritance_chain>`][AsMut]
/// - [`Borrow<$inheritance_chain>`][core::borrow::Borrow]
/// - [`BorrowMut<$inheritance_chain>`][core::borrow::BorrowMut]
///
/// The macro allows specifying fields on the struct, but _only_ zero-sized
/// types like [`PhantomData`] and [`declare::Ivar`] are allowed here!
///
/// [rustfmt-macros]: https://github.com/rust-lang/rustfmt/discussions/5437
/// [`PhantomData`]: core::marker::PhantomData
/// [`declare::Ivar`]: crate::declare::Ivar
///
///
/// # Safety
///
/// The specified superclass must be correct. The object must also respond to
/// standard memory management messages (this is upheld if [`NSObject`] is
/// part of its inheritance chain).
///
/// [`NSObject`]: crate::foundation::NSObject
///
///
/// # Examples
///
/// Create a new type to represent the `NSFormatter` class.
///
/// ```
/// use objc2::foundation::NSObject;
/// use objc2::rc::{Id, Shared};
/// use objc2::{ClassType, extern_class, msg_send_id};
/// #
/// # #[cfg(feature = "gnustep-1-7")]
/// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
///
/// extern_class!(
///     /// An example description.
///     #[derive(PartialEq, Eq, Hash)] // Uses the superclass' implementation
///     // Specify the class and struct name to be used
///     pub struct NSFormatter;
///
///     // Specify the superclass, in this case `NSObject`
///     unsafe impl ClassType for NSFormatter {
///         type Super = NSObject;
///     }
/// );
///
/// // Provided by the implementation of `ClassType`
/// let cls = NSFormatter::class();
///
/// // `NSFormatter` implements `Message`:
/// let obj: Id<NSFormatter, Shared> = unsafe { msg_send_id![cls, new] };
/// ```
///
/// Represent the `NSDateFormatter` class, using the `NSFormatter` type we
/// declared previously to specify as its superclass.
///
/// ```
/// use objc2::foundation::NSObject;
/// use objc2::{extern_class, ClassType};
/// #
/// # extern_class!(
/// #     #[derive(PartialEq, Eq, Hash)]
/// #     pub struct NSFormatter;
/// #
/// #     unsafe impl ClassType for NSFormatter {
/// #         type Super = NSObject;
/// #     }
/// # );
///
/// extern_class!(
///     #[derive(PartialEq, Eq, Hash)]
///     pub struct NSDateFormatter;
///
///     unsafe impl ClassType for NSDateFormatter {
///         // Specify the correct inheritance chain
///         #[inherits(NSObject)]
///         type Super = NSFormatter;
///     }
/// );
/// ```
///
/// See the source code of `objc2::foundation` in general for more examples.
#[doc(alias = "@interface")]
#[macro_export]
macro_rules! extern_class {
    (
        $(#[$m:meta])*
        $v:vis struct $name:ident;

        unsafe impl ClassType for $for:ty {
            $(#[inherits($($inheritance_rest:ty),+)])?
            type Super = $superclass:ty;
        }
    ) => {
        // Just shorthand syntax for the following
        $crate::extern_class!(
            $(#[$m])*
            $v struct $name {}

            unsafe impl ClassType for $for {
                $(#[inherits($($inheritance_rest),+)])?
                type Super = $superclass;
            }
        );
    };
    (
        $(#[$m:meta])*
        $v:vis struct $name:ident {
            $($field_vis:vis $field:ident: $field_ty:ty,)*
        }

        unsafe impl ClassType for $for:ty {
            $(#[inherits($($inheritance_rest:ty),+)])?
            type Super = $superclass:ty;
        }
    ) => {
        $crate::__inner_extern_class!(
            $(#[$m])*
            $v struct $name<> {
                $($field_vis $field: $field_ty,)*
            }

            unsafe impl<> ClassType for $for {
                $(#[inherits($($inheritance_rest),+)])?
                type Super = $superclass;
            }
        );

        const _: () = {
            if $crate::__macro_helpers::size_of::<$name>() != 0 {
                panic!(concat!(
                    "the struct ",
                    stringify!($name),
                    " is not zero-sized!",
                ))
            }
        };
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __impl_as_ref_borrow {
    {
        impl ($($t:tt)*) for $for:ty;
    } => {};
    {
        impl ($($t:tt)*) for $for:ty; $item:ty, $($tail:ty,)*
    } => {
        impl<$($t)*> $crate::__macro_helpers::AsRef<$item> for $for {
            #[inline]
            fn as_ref(&self) -> &$item {
                // Triggers Deref coercion depending on return type
                &*self
            }
        }

        impl<$($t)*> $crate::__macro_helpers::AsMut<$item> for $for {
            #[inline]
            fn as_mut(&mut self) -> &mut $item {
                // Triggers DerefMut coercion depending on return type
                &mut *self
            }
        }

        // Borrow and BorrowMut are correct, since subclasses behaves
        // identical to the class they inherit (message sending doesn't care).
        //
        // In particular, `Eq`, `Ord` and `Hash` all give the same results
        // after borrow.

        impl<$($t)*> $crate::__macro_helpers::Borrow<$item> for $for {
            #[inline]
            fn borrow(&self) -> &$item {
                // Triggers Deref coercion depending on return type
                &*self
            }
        }

        impl<$($t)*> $crate::__macro_helpers::BorrowMut<$item> for $for {
            #[inline]
            fn borrow_mut(&mut self) -> &mut $item {
                // Triggers Deref coercion depending on return type
                &mut *self
            }
        }

        $crate::__impl_as_ref_borrow! {
            impl ($($t)*) for $for; $($tail,)*
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __inner_extern_class {
    // TODO: Expose this variant in the `object` macro.
    (
        $(#[$m:meta])*
        $v:vis struct $name:ident<$($t_struct:ident $(: $b_struct:ident $(= $default:ty)?)?),*> {
            $($field_vis:vis $field:ident: $field_ty:ty,)*
        }

        unsafe impl<$($t_for:ident $(: $b_for:ident)?),*> ClassType for $for:ty {
            $(#[inherits($($inheritance_rest:ty),+)])?
            type Super = $superclass:ty;
        }
    ) => {
        $crate::__inner_extern_class! {
            @__inner
            $(#[$m])*
            $v struct $name ($($t_struct $(: $b_struct $(= $default)?)?),*) {
                $($field_vis $field: $field_ty,)*
            }

            unsafe impl ($($t_for $(: $b_for)?),*) for $for {
                INHERITS = [$superclass, $($($inheritance_rest,)+)? $crate::runtime::Object];
            }
        }

        unsafe impl<$($t_for $(: $b_for)?),*> ClassType for $for {
            type Super = $superclass;
            const NAME: &'static str = stringify!($name);

            #[inline]
            fn class() -> &'static $crate::runtime::Class {
                $crate::class!($name)
            }

            #[inline]
            fn as_super(&self) -> &Self::Super {
                &self.__inner
            }

            #[inline]
            fn as_super_mut(&mut self) -> &mut Self::Super {
                &mut self.__inner
            }
        }
    };
    (
        @__inner

        $(#[$m:meta])*
        $v:vis struct $name:ident ($($t_struct:tt)*) {
            $($field_vis:vis $field:ident: $field_ty:ty,)*
        }

        unsafe impl ($($t:tt)*) for $for:ty {
            INHERITS = [$superclass:ty $(, $inheritance_rest:ty)*];
        }
    ) => {
        $(#[$m])*
        // TODO: repr(transparent) when the inner pointer is no longer a ZST.
        #[repr(C)]
        $v struct $name<$($t_struct)*> {
            __inner: $superclass,
            // Additional fields (should only be zero-sized PhantomData or ivars).
            $($field_vis $field: $field_ty,)*
        }

        // SAFETY:
        // - The item is FFI-safe with `#[repr(C)]`.
        // - The encoding is taken from the inner item, and caller verifies
        //   that it actually inherits said object.
        // - The rest of the struct's fields are ZSTs, so they don't influence
        //   the layout.
        unsafe impl<$($t)*> $crate::RefEncode for $for {
            const ENCODING_REF: $crate::Encoding
                = <$superclass as $crate::RefEncode>::ENCODING_REF;
        }

        // SAFETY: This is essentially just a newtype wrapper over `Object`
        // (we even ensure that `Object` is always last in our inheritance
        // tree), so it is always safe to reinterpret as that.
        //
        // That the object must work with standard memory management is upheld
        // by the caller.
        unsafe impl<$($t)*> $crate::Message for $for {}

        // SAFETY: An instance can always be _used_ in exactly the same way as
        // its superclasses (though not necessarily _constructed_ in the same
        // way, but `Deref` doesn't allow this).
        //
        // Remember; while we (the Rust side) may intentionally be forgetting
        // which instance we're holding, the Objective-C side will remember,
        // and will always dispatch to the correct method implementations.
        //
        // Any lifetime information that the object may have been holding is
        // safely kept in the returned reference.
        //
        // Generics are discarded (for example in the case of `&NSArray<T, O>`
        // to `&NSObject`), but if the generic contained a lifetime, that
        // lifetime is still included in the returned reference.
        //
        // Note that you can easily have two different variables pointing to
        // the same object, `x: &T` and `y: &T::Target`, and this would be
        // perfectly safe!
        impl<$($t)*> $crate::__macro_helpers::Deref for $for {
            type Target = $superclass;

            #[inline]
            fn deref(&self) -> &Self::Target {
                &self.__inner
            }
        }

        // SAFETY: Mutability does not change anything in the above
        // consideration, the lifetime of `&mut Self::Target` is still tied to
        // `&mut self`.
        //
        // Usually we don't want to allow `&mut` of immutable objects like
        // `NSString`, because their `NSCopying` implementation returns the
        // same object, and would violate aliasing rules.
        //
        // But `&mut NSMutableString` -> `&mut NSString` safe, since the
        // `NSCopying` implementation of `NSMutableString` is used, and that
        // is guaranteed to return a different object.
        impl<$($t)*> $crate::__macro_helpers::DerefMut for $for {
            #[inline]
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.__inner
            }
        }

        impl<$($t)*> $crate::__macro_helpers::AsRef<Self> for $for {
            #[inline]
            fn as_ref(&self) -> &Self {
                self
            }
        }

        impl<$($t)*> $crate::__macro_helpers::AsMut<Self> for $for {
            #[inline]
            fn as_mut(&mut self) -> &mut Self {
                self
            }
        }

        $crate::__impl_as_ref_borrow! {
            impl ($($t)*) for $for; $superclass, $($inheritance_rest,)*
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __attribute_helper {
    // Convert a set of attributes described with `@[...]` to `#[...]`, while
    // parsing out the `sel(...)` attribute.
    {
        @strip_sel
        @[sel($($_sel_args:tt)*)]
        $(@[$($m_rest:tt)*])*

        $(#[$($m:tt)*])*
        ($($fn:tt)*)
    } => {
        $crate::__attribute_helper! {
            @strip_sel
            $(@[$($m_rest)*])*

            $(#[$($m)*])*
            ($($fn)*)
        }
    };
    {
        @strip_sel
        @[$($m_checked:tt)*]
        $(@[$($m_rest:tt)*])*

        $(#[$($m:tt)*])*
        ($($fn:tt)*)
    } => {
        $crate::__attribute_helper! {
            @strip_sel
            $(@[$($m_rest)*])*

            $(#[$($m)*])*
            #[$($m_checked)*]
            ($($fn)*)
        }
    };
    {
        @strip_sel
        $(#[$($m:tt)*])*
        ($($fn:tt)*)
    } => {
        $(#[$($m)*])*
        $($fn)*
    };

    // Extract the `#[sel(...)]` attribute and send it to another macro
    {
        @extract_sel
        ($out_macro:path)
        (
            #[sel($($sel:tt)*)]
            $($rest:tt)*
        )
        $($macro_args:tt)*
    } => {{
        $crate::__attribute_helper! {
            @extract_sel_duplicate
            $($rest)*
        }

        $out_macro!(
            $($macro_args)*
            @($($sel)*)
        )
    }};
    {
        @extract_sel
        ($out_macro:path)
        (
            #[$($m_checked:tt)*]
            $($rest:tt)*
        )
        $($macro_args:tt)*
    } => {{
        $crate::__attribute_helper! {
            @extract_sel
            ($out_macro)
            ($($rest)*)
            $($macro_args)*
        }
    }};
    {
        @extract_sel
        ($out_macro:path)
        ()
        $($macro_args:tt)*
    } => {{
        compile_error!("Must specify the desired selector using `#[sel(...)]`");
    }};

    {
        @extract_sel_duplicate
        #[sel($($_sel_args:tt)*)]
        $($rest:tt)*
    } => {{
        compile_error!("Cannot not specify a selector twice!");
    }};
    {
        @extract_sel_duplicate
        #[$($m_checked:tt)*]
        $($rest:tt)*
    } => {{
        $crate::__attribute_helper! {
            @extract_sel_duplicate
            $($rest)*
        }
    }};
    {@extract_sel_duplicate} => {};

}
