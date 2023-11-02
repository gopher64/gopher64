#[doc(hidden)]
#[macro_export]
macro_rules! __rewrite_self_arg {
    {
        ($out_macro:path)
        ($($args:tt)*)
        $($macro_args:tt)*
    } => {
        $crate::__rewrite_self_arg! {
            ($out_macro)
            // Duplicate args out so that we can match on `self`, while still
            // use it as a function argument
            @($($args)*)
            @($($args)*)
            $($macro_args)*
        }
    };

    // Instance method
    {
        ($out_macro:path)
        @(&mut self $($__rest_args:tt)*)
        @(&mut $self:ident $(, $($rest:tt)*)?)
        $($macro_args:tt)*
    } => {
        $out_macro! {
            $($macro_args)*
            @(instance_method)
            @(
                $self: &mut Self,
                _: $crate::runtime::Sel,
            )
            @($($($rest)*)?)
        }
    };
    {
        ($out_macro:path)
        @(&self $($__rest_args:tt)*)
        @(&$self:ident $(, $($rest:tt)*)?)
        $($macro_args:tt)*
    } => {
        $out_macro! {
            $($macro_args)*
            @(instance_method)
            @(
                $self: &Self,
                _: $crate::runtime::Sel,
            )
            @($($($rest)*)?)
        }
    };
    {
        ($out_macro:path)
        @(mut self: $__self_ty:ty $(, $($__rest_args:tt)*)?)
        @(mut $self:ident: $self_ty:ty $(, $($rest:tt)*)?)
        $($macro_args:tt)*
    } => {
        $out_macro! {
            $($macro_args)*
            @(instance_method)
            @(
                mut $self: $self_ty,
                _: $crate::runtime::Sel,
            )
            @($($($rest)*)?)
        }
    };
    {
        ($out_macro:path)
        @(self: $__self_ty:ty $(, $($__rest_args:tt)*)?)
        @($self:ident: $self_ty:ty $(, $($rest:tt)*)?)
        $($macro_args:tt)*
    } => {
        $out_macro! {
            $($macro_args)*
            @(instance_method)
            @(
                $self: $self_ty,
                _: $crate::runtime::Sel,
            )
            @($($($rest)*)?)
        }
    };

    // Class method
    {
        ($out_macro:path)
        @($($__args:tt)*)
        @($($args:tt)*)
        $($macro_args:tt)*
    } => {
        $out_macro! {
            $($macro_args)*
            @(class_method)
            @(
                _: &$crate::runtime::Class,
                _: $crate::runtime::Sel,
            )
            @($($args)*)
        }
    };
}
