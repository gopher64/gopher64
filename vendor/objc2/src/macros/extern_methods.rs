/// Define methods on an external class.
///
/// This is a convenience macro to easily generate associated functions and
/// methods that call [`msg_send!`][crate::msg_send] appropriately.
///
///
/// # Specification
///
/// Within the `impl` block you can define two types of functions without
/// bodies; ["associated functions"] and ["methods"]. These are then mapped to
/// the Objective-C equivalents "class methods" and "instance methods", and an
/// appropriate body is created for you. In particular, if you use `self` your
/// method will assumbed to be an instance method, and if you don't it will be
/// assumed to be a class method.
///
/// The desired selector can be specified using the `#[sel(my:selector:)]`
/// attribute. The name of the function doesn't matter.
///
/// If you specify a function/method with a body, the macro will simply ignore
/// it.
///
/// ["associated functions"]: https://doc.rust-lang.org/reference/items/associated-items.html#methods
/// ["methods"]: https://doc.rust-lang.org/reference/items/associated-items.html#methods
///
///
/// # Safety
///
/// You must ensure that any methods you declare with the `#[sel(...)]`
/// attribute upholds the safety guarantees decribed in the
/// [`msg_send!`][crate::msg_send] macro, _or_ are marked `unsafe`.
///
///
/// # Examples
///
/// Let's create a quick interface to the [`NSCalendar`] class:
///
/// [`NSCalendar`]: https://developer.apple.com/documentation/foundation/nscalendar?language=objc
///
/// ```
/// use objc2::foundation::{NSObject, NSRange, NSString, NSUInteger};
/// use objc2::rc::{Id, Shared};
/// use objc2::{extern_class, extern_methods, msg_send_id, Encode, Encoding, ClassType};
/// #
/// # #[cfg(feature = "gnustep-1-7")]
/// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
///
/// extern_class!(
///     #[derive(PartialEq, Eq, Hash)]
///     pub struct NSCalendar;
///
///     unsafe impl ClassType for NSCalendar {
///         type Super = NSObject;
///     }
/// );
///
/// pub type NSCalendarIdentifier = NSString;
///
/// #[repr(usize)] // NSUInteger
/// pub enum NSCalendarUnit {
///     Hour = 32,
///     Minute = 64,
///     Second = 128,
///     // TODO: More units
/// }
///
/// unsafe impl Encode for NSCalendarUnit {
///     const ENCODING: Encoding = usize::ENCODING;
/// }
///
/// extern_methods!(
///     /// Creation methods.
///     // TODO: Support methods returning `Id`
///     unsafe impl NSCalendar {
///         pub fn current() -> Id<Self, Shared> {
///             unsafe { msg_send_id![Self::class(), currentCalendar] }
///         }
///
///         pub fn new(identifier: &NSCalendarIdentifier) -> Id<Self, Shared> {
///             unsafe {
///                 msg_send_id![
///                     msg_send_id![Self::class(), alloc],
///                     initWithCalendarIdentifier: identifier,
///                 ]
///             }
///         }
///     }
///
///     /// Accessor methods.
///     // SAFETY: `first_weekday` is correctly defined
///     unsafe impl NSCalendar {
///         #[sel(firstWeekday)]
///         pub fn first_weekday(&self) -> NSUInteger;
///
///         pub fn am_symbol(&self) -> Id<NSString, Shared> {
///             unsafe { msg_send_id![self, amSymbol] }
///         }
///
///         #[sel(date:matchesComponents:)]
///         // `unsafe` because we don't have definitions for `NSDate` and
///         // `NSDateComponents` yet, so the user must ensure that is what's
///         // passed.
///         pub unsafe fn date_matches(&self, date: &NSObject, components: &NSObject) -> bool;
///
///         #[sel(maximumRangeOfUnit:)]
///         pub fn max_range(&self, unit: NSCalendarUnit) -> NSRange;
///     }
/// );
/// ```
///
/// The `extern_methods!` declaration then becomes:
///
/// ```
/// # use objc2::foundation::{NSObject, NSRange, NSString, NSUInteger};
/// # use objc2::rc::{Id, Shared};
/// # use objc2::{extern_class, extern_methods, msg_send_id, Encode, Encoding, ClassType};
/// #
/// # #[cfg(feature = "gnustep-1-7")]
/// # unsafe { objc2::__gnustep_hack::get_class_to_force_linkage() };
/// #
/// # extern_class!(
/// #     #[derive(PartialEq, Eq, Hash)]
/// #     pub struct NSCalendar;
/// #
/// #     unsafe impl ClassType for NSCalendar {
/// #         type Super = NSObject;
/// #     }
/// # );
/// #
/// # pub type NSCalendarIdentifier = NSString;
/// #
/// # #[repr(usize)] // NSUInteger
/// # pub enum NSCalendarUnit {
/// #     Hour = 32,
/// #     Minute = 64,
/// #     Second = 128,
/// #     // TODO: More units
/// # }
/// #
/// # unsafe impl Encode for NSCalendarUnit {
/// #     const ENCODING: Encoding = usize::ENCODING;
/// # }
/// #
/// # use objc2::msg_send;
/// /// Creation methods.
/// impl NSCalendar {
///     pub fn current() -> Id<Self, Shared> {
///         unsafe { msg_send_id![Self::class(), currentCalendar] }
///     }
///
///     pub fn new(identifier: &NSCalendarIdentifier) -> Id<Self, Shared> {
///         unsafe {
///             msg_send_id![
///                 msg_send_id![Self::class(), alloc],
///                 initWithCalendarIdentifier: identifier,
///             ]
///         }
///     }
/// }
///
/// /// Accessor methods.
/// impl NSCalendar {
///     pub fn first_weekday(&self) -> NSUInteger {
///         unsafe { msg_send![self, firstWeekday] }
///     }
///
///     pub fn am_symbol(&self) -> Id<NSString, Shared> {
///         unsafe { msg_send_id![self, amSymbol] }
///     }
///
///     pub unsafe fn date_matches(&self, date: &NSObject, components: &NSObject) -> bool {
///         unsafe { msg_send![self, date: date, matchesComponents: components] }
///     }
///
///     pub fn max_range(&self, unit: NSCalendarUnit) -> NSRange {
///         unsafe { msg_send![self, maximumRangeOfUnit: unit] }
///     }
/// }
/// ```
#[macro_export]
macro_rules! extern_methods {
    (
        $(
            $(#[$impl_m:meta])*
            unsafe impl<$($t:ident $(: $b:ident $(+ $rest:ident)*)?),*> $type:ty {
                $($methods:tt)*
            }
        )+
    ) => {
        $(
            $(#[$impl_m])*
            impl<$($t $(: $b $(+ $rest)*)?),*> $type {
                $crate::__inner_extern_methods! {
                    @rewrite_methods
                    $($methods)*
                }
            }
        )+
    };
    (
        $(
            $(#[$impl_m:meta])*
            unsafe impl $type:ty {
                $($methods:tt)*
            }
        )+
    ) => {
        $(
            $(#[$impl_m])*
            impl $type {
                $crate::__inner_extern_methods! {
                    @rewrite_methods
                    $($methods)*
                }
            }
        )+
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __inner_extern_methods {
    {@rewrite_methods} => {};
    {
        @rewrite_methods
        // Unsafe variant
        $(#[$($m:tt)*])*
        $v:vis unsafe fn $name:ident($($args:tt)*) $(-> $ret:ty)?;

        $($rest:tt)*
    } => {
        // Detect instance vs. class method.
        $crate::__rewrite_self_arg! {
            ($crate::__inner_extern_methods)
            ($($args)*)
            @method_out
            @($(#[$($m)*])*)
            @($v unsafe fn $name($($args)*) $(-> $ret)?)
            // Will add @(kind)
            // Will add @(args_start)
            // Will add @(args_rest)
        }

        $crate::__inner_extern_methods! {
            @rewrite_methods
            $($rest)*
        }
    };
    {
        @rewrite_methods
        // Safe variant
        $(#[$($m:tt)*])*
        $v:vis fn $name:ident($($args:tt)*) $(-> $ret:ty)?;

        $($rest:tt)*
    } => {
        $crate::__rewrite_self_arg! {
            ($crate::__inner_extern_methods)
            ($($args)*)
            @method_out
            @($(#[$($m)*])*)
            @($v fn $name($($args)*) $(-> $ret)?)
        }

        $crate::__inner_extern_methods! {
            @rewrite_methods
            $($rest)*
        }
    };
    {
        @rewrite_methods
        // Other items that people might want to put here (e.g. functions with
        // a body).
        $associated_item:item

        $($rest:tt)*
    } => {
        $associated_item

        $crate::__inner_extern_methods! {
            @rewrite_methods
            $($rest)*
        }
    };
    {
        @method_out
        @($(#[$($m:tt)*])*)
        @($($function_start:tt)*)
        @($($kind:tt)*)
        @($($args_start:tt)*)
        @($($args_rest:tt)*)
    } => {
        $crate::__attribute_helper! {
            @strip_sel
            $(@[$($m)*])*
            ($($function_start)* {
                #[allow(unused_unsafe)]
                unsafe {
                    $crate::__attribute_helper! {
                        @extract_sel
                        ($crate::__inner_extern_methods)
                        ($(#[$($m)*])*)
                        @unsafe_method_body
                        @($($kind)*)
                        @($($args_start)*)
                        @($($args_rest)*)
                    }
                }
            })
        }
    };

    {
        @unsafe_method_body
        @(instance_method)
        @(
            $self:ident: $self_ty:ty,
            _: $sel_ty:ty,
        )
        @($($args_rest:tt)*)
        @($($sel:tt)*)
    } => {
        $crate::__collect_msg_send!(
            $crate::msg_send;
            $self;
            ($($sel)*);
            ($($args_rest)*);
        )
    };
    {
        @unsafe_method_body
        @(class_method)
        @(
            _: $cls_ty:ty,
            _: $sel_ty:ty,
        )
        @($($args_rest:tt)*)
        @($($sel:tt)*)
    } => {
        $crate::__collect_msg_send!(
            $crate::msg_send;
            Self::class();
            ($($sel)*);
            ($($args_rest)*);
        )
    };
}

/// Zip selector and arguments, and forward to macro.
#[doc(hidden)]
#[macro_export]
macro_rules! __collect_msg_send {
    // Selector with no arguments
    (
        $macro:path;
        $obj:expr;
        ($sel:ident);
        ();
    ) => {{
        $macro![$obj, $sel]
    }};

    // Base case
    (
        $macro:path;
        $obj:expr;
        ();
        ();
        $($output:tt)+
    ) => {{
        $macro![$obj, $($output)+]
    }};

    // tt-munch each argument
    (
        $macro:path;
        $obj:expr;
        ($sel:ident : $($sel_rest:tt)*);
        ($arg:ident: $arg_ty:ty $(, $($args_rest:tt)*)?);
        $($output:tt)*
    ) => {{
        $crate::__collect_msg_send!(
            $macro;
            $obj;
            ($($sel_rest)*);
            ($($($args_rest)*)?);
            $($output)*
            $sel: $arg,
        )
    }};

    // If couldn't zip selector and arguments, show useful error message
    ($($_any:tt)*) => {{
        compile_error!("Number of arguments in function and selector did not match!")
    }};
}
