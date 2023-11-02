// Not supported by MSRV
#![allow(clippy::uninlined_format_args)]

extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use std::collections::BTreeSet;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, parse_quote,
    spanned::Spanned,
    Attribute, Data, DeriveInput, Error, Expr, ExprLit, ExprUnary, Fields, Ident, Lit, LitInt,
    LitStr, Meta, Result, UnOp,
};

macro_rules! die {
    ($spanned:expr=>
        $msg:expr
    ) => {
        return Err(Error::new_spanned($spanned, $msg))
    };

    (
        $msg:expr
    ) => {
        return Err(Error::new(Span::call_site(), $msg))
    };
}

fn literal(i: i128) -> Expr {
    Expr::Lit(ExprLit {
        lit: Lit::Int(LitInt::new(&i.to_string(), Span::call_site())),
        attrs: vec![],
    })
}

enum DiscriminantValue {
    Literal(i128),
    Expr(Expr),
}

fn parse_discriminant(val_exp: &Expr) -> Result<DiscriminantValue> {
    let mut sign = 1;
    let mut unsigned_expr = val_exp;
    if let Expr::Unary(ExprUnary {
        op: UnOp::Neg(..),
        expr,
        ..
    }) = val_exp
    {
        unsigned_expr = expr;
        sign = -1;
    }
    if let Expr::Lit(ExprLit {
        lit: Lit::Int(ref lit_int),
        ..
    }) = unsigned_expr
    {
        Ok(DiscriminantValue::Literal(
            sign * lit_int.base10_parse::<i128>()?,
        ))
    } else {
        Ok(DiscriminantValue::Expr(val_exp.clone()))
    }
}

#[cfg(feature = "complex-expressions")]
fn parse_alternative_values(val_expr: &Expr) -> Result<Vec<DiscriminantValue>> {
    fn range_expr_value_to_number(
        parent_range_expr: &Expr,
        range_bound_value: &Option<Box<Expr>>,
    ) -> Result<i128> {
        // Avoid needing to calculate what the lower and upper bound would be - these are type dependent,
        // and also may not be obvious in context (e.g. an omitted bound could reasonably mean "from the last discriminant" or "from the lower bound of the type").
        if let Some(range_bound_value) = range_bound_value {
            let range_bound_value = parse_discriminant(range_bound_value.as_ref())?;
            // If non-literals are used, we can't expand to the mapped values, so can't write a nice match statement or do exhaustiveness checking.
            // Require literals instead.
            if let DiscriminantValue::Literal(value) = range_bound_value {
                return Ok(value);
            }
        }
        die!(parent_range_expr => "When ranges are used for alternate values, both bounds most be explicitly specified numeric literals")
    }

    if let Expr::Range(syn::ExprRange {
        from, to, limits, ..
    }) = val_expr
    {
        let lower = range_expr_value_to_number(val_expr, from)?;
        let upper = range_expr_value_to_number(val_expr, to)?;
        // While this is technically allowed in Rust, and results in an empty range, it's almost certainly a mistake in this context.
        if lower > upper {
            die!(val_expr => "When using ranges for alternate values, upper bound must not be less than lower bound");
        }
        let mut values = Vec::with_capacity((upper - lower) as usize);
        let mut next = lower;
        loop {
            match limits {
                syn::RangeLimits::HalfOpen(..) => {
                    if next == upper {
                        break;
                    }
                }
                syn::RangeLimits::Closed(..) => {
                    if next > upper {
                        break;
                    }
                }
            }
            values.push(DiscriminantValue::Literal(next));
            next += 1;
        }
        return Ok(values);
    }
    parse_discriminant(val_expr).map(|v| vec![v])
}

#[cfg(not(feature = "complex-expressions"))]
fn parse_alternative_values(val_expr: &Expr) -> Result<Vec<DiscriminantValue>> {
    parse_discriminant(val_expr).map(|v| vec![v])
}

mod kw {
    syn::custom_keyword!(default);
    syn::custom_keyword!(catch_all);
    syn::custom_keyword!(alternatives);
}

struct NumEnumVariantAttributes {
    items: syn::punctuated::Punctuated<NumEnumVariantAttributeItem, syn::Token![,]>,
}

impl Parse for NumEnumVariantAttributes {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        Ok(Self {
            items: input.parse_terminated(NumEnumVariantAttributeItem::parse)?,
        })
    }
}

enum NumEnumVariantAttributeItem {
    Default(VariantDefaultAttribute),
    CatchAll(VariantCatchAllAttribute),
    Alternatives(VariantAlternativesAttribute),
}

impl Parse for NumEnumVariantAttributeItem {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::default) {
            input.parse().map(Self::Default)
        } else if lookahead.peek(kw::catch_all) {
            input.parse().map(Self::CatchAll)
        } else if lookahead.peek(kw::alternatives) {
            input.parse().map(Self::Alternatives)
        } else {
            Err(lookahead.error())
        }
    }
}

struct VariantDefaultAttribute {
    keyword: kw::default,
}

impl Parse for VariantDefaultAttribute {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            keyword: input.parse()?,
        })
    }
}

impl Spanned for VariantDefaultAttribute {
    fn span(&self) -> Span {
        self.keyword.span()
    }
}

struct VariantCatchAllAttribute {
    keyword: kw::catch_all,
}

impl Parse for VariantCatchAllAttribute {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            keyword: input.parse()?,
        })
    }
}

impl Spanned for VariantCatchAllAttribute {
    fn span(&self) -> Span {
        self.keyword.span()
    }
}

struct VariantAlternativesAttribute {
    keyword: kw::alternatives,
    _eq_token: syn::Token![=],
    _bracket_token: syn::token::Bracket,
    expressions: syn::punctuated::Punctuated<Expr, syn::Token![,]>,
}

impl Parse for VariantAlternativesAttribute {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        let keyword = input.parse()?;
        let _eq_token = input.parse()?;
        let _bracket_token = syn::bracketed!(content in input);
        let expressions = content.parse_terminated(Expr::parse)?;
        Ok(Self {
            keyword,
            _eq_token,
            _bracket_token,
            expressions,
        })
    }
}

impl Spanned for VariantAlternativesAttribute {
    fn span(&self) -> Span {
        self.keyword.span()
    }
}

#[derive(::core::default::Default)]
struct AttributeSpans {
    default: Vec<Span>,
    catch_all: Vec<Span>,
    alternatives: Vec<Span>,
}

struct VariantInfo {
    ident: Ident,
    attr_spans: AttributeSpans,
    is_default: bool,
    is_catch_all: bool,
    canonical_value: Expr,
    alternative_values: Vec<Expr>,
}

impl VariantInfo {
    fn all_values(&self) -> impl Iterator<Item = &Expr> {
        ::core::iter::once(&self.canonical_value).chain(self.alternative_values.iter())
    }

    fn is_complex(&self) -> bool {
        !self.alternative_values.is_empty()
    }
}

struct EnumInfo {
    name: Ident,
    repr: Ident,
    variants: Vec<VariantInfo>,
}

impl EnumInfo {
    /// Returns whether the number of variants (ignoring defaults, catch-alls, etc) is the same as
    /// the capacity of the repr.
    fn is_naturally_exhaustive(&self) -> Result<bool> {
        let repr_str = self.repr.to_string();
        if !repr_str.is_empty() {
            let suffix = repr_str
                .strip_prefix('i')
                .or_else(|| repr_str.strip_prefix('u'));
            if let Some(suffix) = suffix {
                if let Ok(bits) = suffix.parse::<u32>() {
                    let variants = 1usize.checked_shl(bits);
                    return Ok(variants.map_or(false, |v| {
                        v == self
                            .variants
                            .iter()
                            .map(|v| v.alternative_values.len() + 1)
                            .sum()
                    }));
                }
            }
        }
        die!(self.repr.clone() => "Failed to parse repr into bit size");
    }

    fn has_default_variant(&self) -> bool {
        self.default().is_some()
    }

    fn has_complex_variant(&self) -> bool {
        self.variants.iter().any(|info| info.is_complex())
    }

    fn default(&self) -> Option<&Ident> {
        self.variants
            .iter()
            .find(|info| info.is_default)
            .map(|info| &info.ident)
    }

    fn catch_all(&self) -> Option<&Ident> {
        self.variants
            .iter()
            .find(|info| info.is_catch_all)
            .map(|info| &info.ident)
    }

    fn first_default_attr_span(&self) -> Option<&Span> {
        self.variants
            .iter()
            .find_map(|info| info.attr_spans.default.first())
    }

    fn first_alternatives_attr_span(&self) -> Option<&Span> {
        self.variants
            .iter()
            .find_map(|info| info.attr_spans.alternatives.first())
    }

    fn variant_idents(&self) -> Vec<Ident> {
        self.variants
            .iter()
            .map(|variant| variant.ident.clone())
            .collect()
    }

    fn expression_idents(&self) -> Vec<Vec<Ident>> {
        self.variants
            .iter()
            .filter(|variant| !variant.is_catch_all)
            .map(|info| {
                let indices = 0..(info.alternative_values.len() + 1);
                indices
                    .map(|index| format_ident!("{}__num_enum_{}__", info.ident, index))
                    .collect()
            })
            .collect()
    }

    fn variant_expressions(&self) -> Vec<Vec<Expr>> {
        self.variants
            .iter()
            .map(|variant| variant.all_values().cloned().collect())
            .collect()
    }
}

impl Parse for EnumInfo {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok({
            let input: DeriveInput = input.parse()?;
            let name = input.ident;
            let data = match input.data {
                Data::Enum(data) => data,
                Data::Union(data) => die!(data.union_token => "Expected enum but found union"),
                Data::Struct(data) => die!(data.struct_token => "Expected enum but found struct"),
            };

            let repr: Ident = {
                let mut attrs = input.attrs.into_iter();
                loop {
                    if let Some(attr) = attrs.next() {
                        if let Ok(Meta::List(meta_list)) = attr.parse_meta() {
                            if let Some(ident) = meta_list.path.get_ident() {
                                if ident == "repr" {
                                    let mut nested = meta_list.nested.iter();
                                    if nested.len() != 1 {
                                        die!(attr =>
                                            "Expected exactly one `repr` argument"
                                        );
                                    }
                                    let repr = nested.next().unwrap();
                                    let repr: Ident = parse_quote! {
                                        #repr
                                    };
                                    if repr == "C" {
                                        die!(repr =>
                                            "repr(C) doesn't have a well defined size"
                                        );
                                    } else {
                                        break repr;
                                    }
                                }
                            }
                        }
                    } else {
                        die!("Missing `#[repr({Integer})]` attribute");
                    }
                }
            };

            let mut variants: Vec<VariantInfo> = vec![];
            let mut has_default_variant: bool = false;
            let mut has_catch_all_variant: bool = false;

            // Vec to keep track of the used discriminants and alt values.
            let mut discriminant_int_val_set = BTreeSet::new();

            let mut next_discriminant = literal(0);
            for variant in data.variants.into_iter() {
                let ident = variant.ident.clone();

                let discriminant = match &variant.discriminant {
                    Some(d) => d.1.clone(),
                    None => next_discriminant.clone(),
                };

                let mut attr_spans: AttributeSpans = Default::default();
                let mut raw_alternative_values: Vec<Expr> = vec![];
                // Keep the attribute around for better error reporting.
                let mut alt_attr_ref: Vec<&Attribute> = vec![];

                // `#[num_enum(default)]` is required by `#[derive(FromPrimitive)]`
                // and forbidden by `#[derive(UnsafeFromPrimitive)]`, so we need to
                // keep track of whether we encountered such an attribute:
                let mut is_default: bool = false;
                let mut is_catch_all: bool = false;

                for attribute in &variant.attrs {
                    if attribute.path.is_ident("default") {
                        if has_default_variant {
                            die!(attribute =>
                                "Multiple variants marked `#[default]` or `#[num_enum(default)]` found"
                            );
                        } else if has_catch_all_variant {
                            die!(attribute =>
                                "Attribute `default` is mutually exclusive with `catch_all`"
                            );
                        }
                        attr_spans.default.push(attribute.span());
                        is_default = true;
                        has_default_variant = true;
                    }

                    if attribute.path.is_ident("num_enum") {
                        match attribute.parse_args_with(NumEnumVariantAttributes::parse) {
                            Ok(variant_attributes) => {
                                for variant_attribute in variant_attributes.items {
                                    match variant_attribute {
                                        NumEnumVariantAttributeItem::Default(default) => {
                                            if has_default_variant {
                                                die!(default.keyword =>
                                                    "Multiple variants marked `#[default]` or `#[num_enum(default)]` found"
                                                );
                                            } else if has_catch_all_variant {
                                                die!(default.keyword =>
                                                    "Attribute `default` is mutually exclusive with `catch_all`"
                                                );
                                            }
                                            attr_spans.default.push(default.span());
                                            is_default = true;
                                            has_default_variant = true;
                                        }
                                        NumEnumVariantAttributeItem::CatchAll(catch_all) => {
                                            if has_catch_all_variant {
                                                die!(catch_all.keyword =>
                                                    "Multiple variants marked with `#[num_enum(catch_all)]`"
                                                );
                                            } else if has_default_variant {
                                                die!(catch_all.keyword =>
                                                    "Attribute `catch_all` is mutually exclusive with `default`"
                                                );
                                            }

                                            match variant
                                                .fields
                                                .iter()
                                                .collect::<Vec<_>>()
                                                .as_slice()
                                            {
                                                [syn::Field {
                                                    ty: syn::Type::Path(syn::TypePath { path, .. }),
                                                    ..
                                                }] if path.is_ident(&repr) => {
                                                    attr_spans.catch_all.push(catch_all.span());
                                                    is_catch_all = true;
                                                    has_catch_all_variant = true;
                                                }
                                                _ => {
                                                    die!(catch_all.keyword =>
                                                        "Variant with `catch_all` must be a tuple with exactly 1 field matching the repr type"
                                                    );
                                                }
                                            }
                                        }
                                        NumEnumVariantAttributeItem::Alternatives(alternatives) => {
                                            attr_spans.alternatives.push(alternatives.span());
                                            raw_alternative_values.extend(alternatives.expressions);
                                            alt_attr_ref.push(attribute);
                                        }
                                    }
                                }
                            }
                            Err(err) => {
                                if cfg!(not(feature = "complex-expressions")) {
                                    let attribute_str = format!("{}", attribute.tokens);
                                    if attribute_str.contains("alternatives")
                                        && attribute_str.contains("..")
                                    {
                                        // Give a nice error message suggesting how to fix the problem.
                                        die!(attribute => "Ranges are only supported as num_enum alternate values if the `complex-expressions` feature of the crate `num_enum` is enabled".to_string())
                                    }
                                }
                                die!(attribute =>
                                    format!("Invalid attribute: {}", err)
                                );
                            }
                        }
                    }
                }

                if !is_catch_all {
                    match &variant.fields {
                        Fields::Named(_) | Fields::Unnamed(_) => {
                            die!(variant => format!("`{}` only supports unit variants (with no associated data), but `{}::{}` was not a unit variant.", get_crate_name(), name, ident));
                        }
                        Fields::Unit => {}
                    }
                }

                let discriminant_value = parse_discriminant(&discriminant)?;

                // Check for collision.
                // We can't do const evaluation, or even compare arbitrary Exprs,
                // so unfortunately we can't check for duplicates.
                // That's not the end of the world, just we'll end up with compile errors for
                // matches with duplicate branches in generated code instead of nice friendly error messages.
                if let DiscriminantValue::Literal(canonical_value_int) = discriminant_value {
                    if discriminant_int_val_set.contains(&canonical_value_int) {
                        die!(ident => format!("The discriminant '{}' collides with a value attributed to a previous variant", canonical_value_int))
                    }
                }

                // Deal with the alternative values.
                let mut flattened_alternative_values = Vec::new();
                let mut flattened_raw_alternative_values = Vec::new();
                for raw_alternative_value in raw_alternative_values {
                    let expanded_values = parse_alternative_values(&raw_alternative_value)?;
                    for expanded_value in expanded_values {
                        flattened_alternative_values.push(expanded_value);
                        flattened_raw_alternative_values.push(raw_alternative_value.clone())
                    }
                }

                if !flattened_alternative_values.is_empty() {
                    let alternate_int_values = flattened_alternative_values
                        .into_iter()
                        .map(|v| {
                            match v {
                                DiscriminantValue::Literal(value) => Ok(value),
                                DiscriminantValue::Expr(expr) => {
                                    if let Expr::Range(_) = expr {
                                        if cfg!(not(feature = "complex-expressions")) {
                                            // Give a nice error message suggesting how to fix the problem.
                                            die!(expr => "Ranges are only supported as num_enum alternate values if the `complex-expressions` feature of the crate `num_enum` is enabled".to_string())
                                        }
                                    }
                                    // We can't do uniqueness checking on non-literals, so we don't allow them as alternate values.
                                    // We could probably allow them, but there doesn't seem to be much of a use-case,
                                    // and it's easier to give good error messages about duplicate values this way,
                                    // rather than rustc errors on conflicting match branches.
                                    die!(expr => "Only literals are allowed as num_enum alternate values".to_string())
                                },
                            }
                        })
                        .collect::<Result<Vec<i128>>>()?;
                    let mut sorted_alternate_int_values = alternate_int_values.clone();
                    sorted_alternate_int_values.sort_unstable();
                    let sorted_alternate_int_values = sorted_alternate_int_values;

                    // Check if the current discriminant is not in the alternative values.
                    if let DiscriminantValue::Literal(canonical_value_int) = discriminant_value {
                        if let Some(index) = alternate_int_values
                            .iter()
                            .position(|&x| x == canonical_value_int)
                        {
                            die!(&flattened_raw_alternative_values[index] => format!("'{}' in the alternative values is already attributed as the discriminant of this variant", canonical_value_int));
                        }
                    }

                    // Search for duplicates, the vec is sorted. Warn about them.
                    if (1..sorted_alternate_int_values.len()).any(|i| {
                        sorted_alternate_int_values[i] == sorted_alternate_int_values[i - 1]
                    }) {
                        let attr = *alt_attr_ref.last().unwrap();
                        die!(attr => "There is duplication in the alternative values");
                    }
                    // Search if those discriminant_int_val_set where already attributed.
                    // (discriminant_int_val_set is BTreeSet, and iter().next_back() is the is the maximum in the set.)
                    if let Some(last_upper_val) = discriminant_int_val_set.iter().next_back() {
                        if sorted_alternate_int_values.first().unwrap() <= last_upper_val {
                            for (index, val) in alternate_int_values.iter().enumerate() {
                                if discriminant_int_val_set.contains(val) {
                                    die!(&flattened_raw_alternative_values[index] => format!("'{}' in the alternative values is already attributed to a previous variant", val));
                                }
                            }
                        }
                    }

                    // Reconstruct the alternative_values vec of Expr but sorted.
                    flattened_raw_alternative_values = sorted_alternate_int_values
                        .iter()
                        .map(|val| literal(val.to_owned()))
                        .collect();

                    // Add the alternative values to the the set to keep track.
                    discriminant_int_val_set.extend(sorted_alternate_int_values);
                }

                // Add the current discriminant to the the set to keep track.
                if let DiscriminantValue::Literal(canonical_value_int) = discriminant_value {
                    discriminant_int_val_set.insert(canonical_value_int);
                }

                variants.push(VariantInfo {
                    ident,
                    attr_spans,
                    is_default,
                    is_catch_all,
                    canonical_value: discriminant,
                    alternative_values: flattened_raw_alternative_values,
                });

                // Get the next value for the discriminant.
                next_discriminant = match discriminant_value {
                    DiscriminantValue::Literal(int_value) => literal(int_value.wrapping_add(1)),
                    DiscriminantValue::Expr(expr) => {
                        parse_quote! {
                            #repr::wrapping_add(#expr, 1)
                        }
                    }
                }
            }

            EnumInfo {
                name,
                repr,
                variants,
            }
        })
    }
}

/// Implements `Into<Primitive>` for a `#[repr(Primitive)] enum`.
///
/// (It actually implements `From<Enum> for Primitive`)
///
/// ## Allows turning an enum into a primitive.
///
/// ```rust
/// use num_enum::IntoPrimitive;
///
/// #[derive(IntoPrimitive)]
/// #[repr(u8)]
/// enum Number {
///     Zero,
///     One,
/// }
///
/// let zero: u8 = Number::Zero.into();
/// assert_eq!(zero, 0u8);
/// ```
#[proc_macro_derive(IntoPrimitive, attributes(num_enum, catch_all))]
pub fn derive_into_primitive(input: TokenStream) -> TokenStream {
    let enum_info = parse_macro_input!(input as EnumInfo);
    let catch_all = enum_info.catch_all();
    let name = &enum_info.name;
    let repr = &enum_info.repr;

    let body = if let Some(catch_all_ident) = catch_all {
        quote! {
            match enum_value {
                #name::#catch_all_ident(raw) => raw,
                rest => unsafe { *(&rest as *const #name as *const Self) }
            }
        }
    } else {
        quote! { enum_value as Self }
    };

    TokenStream::from(quote! {
        impl From<#name> for #repr {
            #[inline]
            fn from (enum_value: #name) -> Self
            {
                #body
            }
        }
    })
}

/// Implements `From<Primitive>` for a `#[repr(Primitive)] enum`.
///
/// Turning a primitive into an enum with `from`.
/// ----------------------------------------------
///
/// ```rust
/// use num_enum::FromPrimitive;
///
/// #[derive(Debug, Eq, PartialEq, FromPrimitive)]
/// #[repr(u8)]
/// enum Number {
///     Zero,
///     #[num_enum(default)]
///     NonZero,
/// }
///
/// let zero = Number::from(0u8);
/// assert_eq!(zero, Number::Zero);
///
/// let one = Number::from(1u8);
/// assert_eq!(one, Number::NonZero);
///
/// let two = Number::from(2u8);
/// assert_eq!(two, Number::NonZero);
/// ```
#[proc_macro_derive(FromPrimitive, attributes(num_enum, default, catch_all))]
pub fn derive_from_primitive(input: TokenStream) -> TokenStream {
    let enum_info: EnumInfo = parse_macro_input!(input);
    let krate = Ident::new(&get_crate_name(), Span::call_site());

    let is_naturally_exhaustive = enum_info.is_naturally_exhaustive();
    let catch_all_body = match is_naturally_exhaustive {
        Ok(is_naturally_exhaustive) => {
            if is_naturally_exhaustive {
                quote! { unreachable!("exhaustive enum") }
            } else if let Some(default_ident) = enum_info.default() {
                quote! { Self::#default_ident }
            } else if let Some(catch_all_ident) = enum_info.catch_all() {
                quote! { Self::#catch_all_ident(number) }
            } else {
                let span = Span::call_site();
                let message =
                    "#[derive(num_enum::FromPrimitive)] requires enum to be exhaustive, or a variant marked with `#[default]`, `#[num_enum(default)]`, or `#[num_enum(catch_all)`";
                return syn::Error::new(span, message).to_compile_error().into();
            }
        }
        Err(err) => {
            return err.to_compile_error().into();
        }
    };

    let EnumInfo {
        ref name, ref repr, ..
    } = enum_info;

    let variant_idents: Vec<Ident> = enum_info.variant_idents();
    let expression_idents: Vec<Vec<Ident>> = enum_info.expression_idents();
    let variant_expressions: Vec<Vec<Expr>> = enum_info.variant_expressions();

    debug_assert_eq!(variant_idents.len(), variant_expressions.len());

    TokenStream::from(quote! {
        impl ::#krate::FromPrimitive for #name {
            type Primitive = #repr;

            fn from_primitive(number: Self::Primitive) -> Self {
                // Use intermediate const(s) so that enums defined like
                // `Two = ONE + 1u8` work properly.
                #![allow(non_upper_case_globals)]
                #(
                    #(
                        const #expression_idents: #repr = #variant_expressions;
                    )*
                )*
                #[deny(unreachable_patterns)]
                match number {
                    #(
                        #( #expression_idents )|*
                        => Self::#variant_idents,
                    )*
                    #[allow(unreachable_patterns)]
                    _ => #catch_all_body,
                }
            }
        }

        impl ::core::convert::From<#repr> for #name {
            #[inline]
            fn from (
                number: #repr,
            ) -> Self {
                ::#krate::FromPrimitive::from_primitive(number)
            }
        }

        // The Rust stdlib will implement `#name: From<#repr>` for us for free!

        impl ::#krate::TryFromPrimitive for #name {
            type Primitive = #repr;

            const NAME: &'static str = stringify!(#name);

            #[inline]
            fn try_from_primitive (
                number: Self::Primitive,
            ) -> ::core::result::Result<
                Self,
                ::#krate::TryFromPrimitiveError<Self>,
            >
            {
                Ok(::#krate::FromPrimitive::from_primitive(number))
            }
        }
    })
}

/// Implements `TryFrom<Primitive>` for a `#[repr(Primitive)] enum`.
///
/// Attempting to turn a primitive into an enum with `try_from`.
/// ----------------------------------------------
///
/// ```rust
/// use num_enum::TryFromPrimitive;
/// use std::convert::TryFrom;
///
/// #[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
/// #[repr(u8)]
/// enum Number {
///     Zero,
///     One,
/// }
///
/// let zero = Number::try_from(0u8);
/// assert_eq!(zero, Ok(Number::Zero));
///
/// let three = Number::try_from(3u8);
/// assert_eq!(
///     three.unwrap_err().to_string(),
///     "No discriminant in enum `Number` matches the value `3`",
/// );
/// ```
#[proc_macro_derive(TryFromPrimitive, attributes(num_enum))]
pub fn derive_try_from_primitive(input: TokenStream) -> TokenStream {
    let enum_info: EnumInfo = parse_macro_input!(input);
    let krate = Ident::new(&get_crate_name(), Span::call_site());

    let EnumInfo {
        ref name, ref repr, ..
    } = enum_info;

    let variant_idents: Vec<Ident> = enum_info.variant_idents();
    let expression_idents: Vec<Vec<Ident>> = enum_info.expression_idents();
    let variant_expressions: Vec<Vec<Expr>> = enum_info.variant_expressions();

    debug_assert_eq!(variant_idents.len(), variant_expressions.len());

    let default_arm = match enum_info.default() {
        Some(ident) => {
            quote! {
                _ => ::core::result::Result::Ok(
                    #name::#ident
                )
            }
        }
        None => {
            quote! {
                _ => ::core::result::Result::Err(
                    ::#krate::TryFromPrimitiveError { number }
                )
            }
        }
    };

    TokenStream::from(quote! {
        impl ::#krate::TryFromPrimitive for #name {
            type Primitive = #repr;

            const NAME: &'static str = stringify!(#name);

            fn try_from_primitive (
                number: Self::Primitive,
            ) -> ::core::result::Result<
                Self,
                ::#krate::TryFromPrimitiveError<Self>
            > {
                // Use intermediate const(s) so that enums defined like
                // `Two = ONE + 1u8` work properly.
                #![allow(non_upper_case_globals)]
                #(
                    #(
                        const #expression_idents: #repr = #variant_expressions;
                    )*
                )*
                #[deny(unreachable_patterns)]
                match number {
                    #(
                        #( #expression_idents )|*
                        => ::core::result::Result::Ok(Self::#variant_idents),
                    )*
                    #[allow(unreachable_patterns)]
                    #default_arm,
                }
            }
        }

        impl ::core::convert::TryFrom<#repr> for #name {
            type Error = ::#krate::TryFromPrimitiveError<Self>;

            #[inline]
            fn try_from (
                number: #repr,
            ) -> ::core::result::Result<Self, ::#krate::TryFromPrimitiveError<Self>>
            {
                ::#krate::TryFromPrimitive::try_from_primitive(number)
            }
        }
    })
}

#[cfg(feature = "proc-macro-crate")]
fn get_crate_name() -> String {
    let found_crate = proc_macro_crate::crate_name("num_enum").unwrap_or_else(|err| {
        eprintln!("Warning: {}\n    => defaulting to `num_enum`", err,);
        proc_macro_crate::FoundCrate::Itself
    });

    match found_crate {
        proc_macro_crate::FoundCrate::Itself => String::from("num_enum"),
        proc_macro_crate::FoundCrate::Name(name) => name,
    }
}

// Don't depend on proc-macro-crate in no_std environments because it causes an awkward dependency
// on serde with std.
//
// no_std dependees on num_enum cannot rename the num_enum crate when they depend on it. Sorry.
//
// See https://github.com/illicitonion/num_enum/issues/18
#[cfg(not(feature = "proc-macro-crate"))]
fn get_crate_name() -> String {
    String::from("num_enum")
}

/// Generates a `unsafe fn from_unchecked (number: Primitive) -> Self`
/// associated function.
///
/// Allows unsafely turning a primitive into an enum with from_unchecked.
/// -------------------------------------------------------------
///
/// If you're really certain a conversion will succeed, and want to avoid a small amount of overhead, you can use unsafe
/// code to do this conversion. Unless you have data showing that the match statement generated in the `try_from` above is a
/// bottleneck for you, you should avoid doing this, as the unsafe code has potential to cause serious memory issues in
/// your program.
///
/// ```rust
/// use num_enum::UnsafeFromPrimitive;
///
/// #[derive(Debug, Eq, PartialEq, UnsafeFromPrimitive)]
/// #[repr(u8)]
/// enum Number {
///     Zero,
///     One,
/// }
///
/// fn main() {
///     assert_eq!(
///         Number::Zero,
///         unsafe { Number::from_unchecked(0_u8) },
///     );
///     assert_eq!(
///         Number::One,
///         unsafe { Number::from_unchecked(1_u8) },
///     );
/// }
///
/// unsafe fn undefined_behavior() {
///     let _ = Number::from_unchecked(2); // 2 is not a valid discriminant!
/// }
/// ```
#[proc_macro_derive(UnsafeFromPrimitive, attributes(num_enum))]
pub fn derive_unsafe_from_primitive(stream: TokenStream) -> TokenStream {
    let enum_info = parse_macro_input!(stream as EnumInfo);

    if enum_info.has_default_variant() {
        let span = enum_info
            .first_default_attr_span()
            .cloned()
            .expect("Expected span");
        let message = "#[derive(UnsafeFromPrimitive)] does not support `#[num_enum(default)]`";
        return syn::Error::new(span, message).to_compile_error().into();
    }

    if enum_info.has_complex_variant() {
        let span = enum_info
            .first_alternatives_attr_span()
            .cloned()
            .expect("Expected span");
        let message =
            "#[derive(UnsafeFromPrimitive)] does not support `#[num_enum(alternatives = [..])]`";
        return syn::Error::new(span, message).to_compile_error().into();
    }

    let EnumInfo {
        ref name, ref repr, ..
    } = enum_info;

    let doc_string = LitStr::new(
        &format!(
            r#"
Transmutes `number: {repr}` into a [`{name}`].

# Safety

  - `number` must represent a valid discriminant of [`{name}`]
"#,
            repr = repr,
            name = name,
        ),
        Span::call_site(),
    );

    TokenStream::from(quote! {
        impl #name {
            #[doc = #doc_string]
            #[inline]
            pub unsafe fn from_unchecked(number: #repr) -> Self {
                ::core::mem::transmute(number)
            }
        }
    })
}

/// Implements `core::default::Default` for a `#[repr(Primitive)] enum`.
///
/// Whichever variant has the `#[default]` or `#[num_enum(default)]` attribute will be returned.
/// ----------------------------------------------
///
/// ```rust
/// #[derive(Debug, Eq, PartialEq, num_enum::Default)]
/// #[repr(u8)]
/// enum Number {
///     Zero,
///     #[default]
///     One,
/// }
///
/// assert_eq!(Number::One, Number::default());
/// assert_eq!(Number::One, <Number as ::core::default::Default>::default());
/// ```
#[proc_macro_derive(Default, attributes(num_enum, default))]
pub fn derive_default(stream: TokenStream) -> TokenStream {
    let enum_info = parse_macro_input!(stream as EnumInfo);

    let default_ident = match enum_info.default() {
        Some(ident) => ident,
        None => {
            let span = Span::call_site();
            let message =
                "#[derive(num_enum::Default)] requires enum to be exhaustive, or a variant marked with `#[default]` or `#[num_enum(default)]`";
            return syn::Error::new(span, message).to_compile_error().into();
        }
    };

    let EnumInfo { ref name, .. } = enum_info;

    TokenStream::from(quote! {
        impl ::core::default::Default for #name {
            #[inline]
            fn default() -> Self {
                Self::#default_ident
            }
        }
    })
}
