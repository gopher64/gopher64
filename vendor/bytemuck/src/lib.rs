#![no_std]
#![warn(missing_docs)]
#![allow(clippy::match_like_matches_macro)]
#![allow(clippy::uninlined_format_args)]
#![cfg_attr(feature = "nightly_docs", feature(doc_cfg))]
#![cfg_attr(feature = "nightly_portable_simd", feature(portable_simd))]
#![cfg_attr(feature = "nightly_stdsimd", feature(stdsimd))]

//! This crate gives small utilities for casting between plain data types.
//!
//! ## Basics
//!
//! Data comes in five basic forms in Rust, so we have five basic casting
//! functions:
//!
//! * `T` uses [`cast`]
//! * `&T` uses [`cast_ref`]
//! * `&mut T` uses [`cast_mut`]
//! * `&[T]` uses [`cast_slice`]
//! * `&mut [T]` uses [`cast_slice_mut`]
//!
//! Some casts will never fail (eg: `cast::<u32, f32>` always works), other
//! casts might fail (eg: `cast_ref::<[u8; 4], u32>` will fail if the reference
//! isn't already aligned to 4). Each casting function has a "try" version which
//! will return a `Result`, and the "normal" version which will simply panic on
//! invalid input.
//!
//! ## Using Your Own Types
//!
//! All the functions here are guarded by the [`Pod`] trait, which is a
//! sub-trait of the [`Zeroable`] trait.
//!
//! If you're very sure that your type is eligible, you can implement those
//! traits for your type and then they'll have full casting support. However,
//! these traits are `unsafe`, and you should carefully read the requirements
//! before adding the them to your own types.
//!
//! ## Features
//!
//! * This crate is core only by default, but if you're using Rust 1.36 or later
//!   you can enable the `extern_crate_alloc` cargo feature for some additional
//!   methods related to `Box` and `Vec`. Note that the `docs.rs` documentation
//!   is always built with `extern_crate_alloc` cargo feature enabled.

#[cfg(all(target_arch = "aarch64", feature = "aarch64_simd"))]
use core::arch::aarch64;
#[cfg(all(target_arch = "wasm32", feature = "wasm_simd"))]
use core::arch::wasm32;
#[cfg(target_arch = "x86")]
use core::arch::x86;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64;
//
use core::{marker::*, mem::*, num::*, ptr::*};

// Used from macros to ensure we aren't using some locally defined name and
// actually are referencing libcore. This also would allow pre-2018 edition
// crates to use our macros, but I'm not sure how important that is.
#[doc(hidden)]
pub use ::core as __core;

#[cfg(not(feature = "min_const_generics"))]
macro_rules! impl_unsafe_marker_for_array {
  ( $marker:ident , $( $n:expr ),* ) => {
    $(unsafe impl<T> $marker for [T; $n] where T: $marker {})*
  }
}

/// A macro to transmute between two types without requiring knowing size
/// statically.
macro_rules! transmute {
  ($val:expr) => {
    ::core::mem::transmute_copy(&::core::mem::ManuallyDrop::new($val))
  };
}

/// A macro to implement marker traits for various simd types.
/// #[allow(unused)] because the impls are only compiled on relevant platforms
/// with relevant cargo features enabled.
#[allow(unused)]
macro_rules! impl_unsafe_marker_for_simd {
  ($(#[cfg($cfg_predicate:meta)])? unsafe impl $trait:ident for $platform:ident :: {}) => {};
  ($(#[cfg($cfg_predicate:meta)])? unsafe impl $trait:ident for $platform:ident :: { $first_type:ident $(, $types:ident)* $(,)? }) => {
    $( #[cfg($cfg_predicate)] )?
    $( #[cfg_attr(feature = "nightly_docs", doc(cfg($cfg_predicate)))] )?
    unsafe impl $trait for $platform::$first_type {}
    $( #[cfg($cfg_predicate)] )? // To prevent recursion errors if nothing is going to be expanded anyway.
    impl_unsafe_marker_for_simd!($( #[cfg($cfg_predicate)] )? unsafe impl $trait for $platform::{ $( $types ),* });
  };
}

#[cfg(feature = "extern_crate_std")]
extern crate std;

#[cfg(feature = "extern_crate_alloc")]
extern crate alloc;
#[cfg(feature = "extern_crate_alloc")]
#[cfg_attr(feature = "nightly_docs", doc(cfg(feature = "extern_crate_alloc")))]
pub mod allocation;
#[cfg(feature = "extern_crate_alloc")]
pub use allocation::*;

mod anybitpattern;
pub use anybitpattern::*;

pub mod checked;
pub use checked::CheckedBitPattern;

mod internal;

mod zeroable;
pub use zeroable::*;
mod zeroable_in_option;
pub use zeroable_in_option::*;

mod pod;
pub use pod::*;
mod pod_in_option;
pub use pod_in_option::*;

#[cfg(feature = "must_cast")]
mod must;
#[cfg(feature = "must_cast")]
#[cfg_attr(feature = "nightly_docs", doc(cfg(feature = "must_cast")))]
pub use must::*;

mod no_uninit;
pub use no_uninit::*;

mod contiguous;
pub use contiguous::*;

mod offset_of;
pub use offset_of::*;

mod transparent;
pub use transparent::*;

#[cfg(feature = "derive")]
#[cfg_attr(feature = "nightly_docs", doc(cfg(feature = "derive")))]
pub use bytemuck_derive::{
  AnyBitPattern, ByteEq, ByteHash, CheckedBitPattern, Contiguous, NoUninit,
  Pod, TransparentWrapper, Zeroable,
};

/// The things that can go wrong when casting between [`Pod`] data forms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PodCastError {
  /// You tried to cast a slice to an element type with a higher alignment
  /// requirement but the slice wasn't aligned.
  TargetAlignmentGreaterAndInputNotAligned,
  /// If the element size changes then the output slice changes length
  /// accordingly. If the output slice wouldn't be a whole number of elements
  /// then the conversion fails.
  OutputSliceWouldHaveSlop,
  /// When casting a slice you can't convert between ZST elements and non-ZST
  /// elements. When casting an individual `T`, `&T`, or `&mut T` value the
  /// source size and destination size must be an exact match.
  SizeMismatch,
  /// For this type of cast the alignments must be exactly the same and they
  /// were not so now you're sad.
  ///
  /// This error is generated **only** by operations that cast allocated types
  /// (such as `Box` and `Vec`), because in that case the alignment must stay
  /// exact.
  AlignmentMismatch,
}
#[cfg(not(target_arch = "spirv"))]
impl core::fmt::Display for PodCastError {
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    write!(f, "{:?}", self)
  }
}
#[cfg(feature = "extern_crate_std")]
#[cfg_attr(feature = "nightly_docs", doc(cfg(feature = "extern_crate_std")))]
impl std::error::Error for PodCastError {}

/// Re-interprets `&T` as `&[u8]`.
///
/// Any ZST becomes an empty slice, and in that case the pointer value of that
/// empty slice might not match the pointer value of the input reference.
#[inline]
pub fn bytes_of<T: NoUninit>(t: &T) -> &[u8] {
  unsafe { internal::bytes_of(t) }
}

/// Re-interprets `&mut T` as `&mut [u8]`.
///
/// Any ZST becomes an empty slice, and in that case the pointer value of that
/// empty slice might not match the pointer value of the input reference.
#[inline]
pub fn bytes_of_mut<T: NoUninit + AnyBitPattern>(t: &mut T) -> &mut [u8] {
  unsafe { internal::bytes_of_mut(t) }
}

/// Re-interprets `&[u8]` as `&T`.
///
/// ## Panics
///
/// This is [`try_from_bytes`] but will panic on error.
#[inline]
pub fn from_bytes<T: AnyBitPattern>(s: &[u8]) -> &T {
  unsafe { internal::from_bytes(s) }
}

/// Re-interprets `&mut [u8]` as `&mut T`.
///
/// ## Panics
///
/// This is [`try_from_bytes_mut`] but will panic on error.
#[inline]
pub fn from_bytes_mut<T: NoUninit + AnyBitPattern>(s: &mut [u8]) -> &mut T {
  unsafe { internal::from_bytes_mut(s) }
}

/// Reads from the bytes as if they were a `T`.
///
/// ## Failure
/// * If the `bytes` length is not equal to `size_of::<T>()`.
#[inline]
pub fn try_pod_read_unaligned<T: AnyBitPattern>(
  bytes: &[u8],
) -> Result<T, PodCastError> {
  unsafe { internal::try_pod_read_unaligned(bytes) }
}

/// Reads the slice into a `T` value.
///
/// ## Panics
/// * This is like `try_pod_read_unaligned` but will panic on failure.
#[inline]
pub fn pod_read_unaligned<T: AnyBitPattern>(bytes: &[u8]) -> T {
  unsafe { internal::pod_read_unaligned(bytes) }
}

/// Re-interprets `&[u8]` as `&T`.
///
/// ## Failure
///
/// * If the slice isn't aligned for the new type
/// * If the slice's length isn’t exactly the size of the new type
#[inline]
pub fn try_from_bytes<T: AnyBitPattern>(s: &[u8]) -> Result<&T, PodCastError> {
  unsafe { internal::try_from_bytes(s) }
}

/// Re-interprets `&mut [u8]` as `&mut T`.
///
/// ## Failure
///
/// * If the slice isn't aligned for the new type
/// * If the slice's length isn’t exactly the size of the new type
#[inline]
pub fn try_from_bytes_mut<T: NoUninit + AnyBitPattern>(
  s: &mut [u8],
) -> Result<&mut T, PodCastError> {
  unsafe { internal::try_from_bytes_mut(s) }
}

/// Cast `T` into `U`
///
/// ## Panics
///
/// * This is like [`try_cast`](try_cast), but will panic on a size mismatch.
#[inline]
pub fn cast<A: NoUninit, B: AnyBitPattern>(a: A) -> B {
  unsafe { internal::cast(a) }
}

/// Cast `&mut T` into `&mut U`.
///
/// ## Panics
///
/// This is [`try_cast_mut`] but will panic on error.
#[inline]
pub fn cast_mut<A: NoUninit + AnyBitPattern, B: NoUninit + AnyBitPattern>(
  a: &mut A,
) -> &mut B {
  unsafe { internal::cast_mut(a) }
}

/// Cast `&T` into `&U`.
///
/// ## Panics
///
/// This is [`try_cast_ref`] but will panic on error.
#[inline]
pub fn cast_ref<A: NoUninit, B: AnyBitPattern>(a: &A) -> &B {
  unsafe { internal::cast_ref(a) }
}

/// Cast `&[A]` into `&[B]`.
///
/// ## Panics
///
/// This is [`try_cast_slice`] but will panic on error.
#[inline]
pub fn cast_slice<A: NoUninit, B: AnyBitPattern>(a: &[A]) -> &[B] {
  unsafe { internal::cast_slice(a) }
}

/// Cast `&mut [T]` into `&mut [U]`.
///
/// ## Panics
///
/// This is [`try_cast_slice_mut`] but will panic on error.
#[inline]
pub fn cast_slice_mut<
  A: NoUninit + AnyBitPattern,
  B: NoUninit + AnyBitPattern,
>(
  a: &mut [A],
) -> &mut [B] {
  unsafe { internal::cast_slice_mut(a) }
}

/// As `align_to`, but safe because of the [`Pod`] bound.
#[inline]
pub fn pod_align_to<T: NoUninit, U: AnyBitPattern>(
  vals: &[T],
) -> (&[T], &[U], &[T]) {
  unsafe { vals.align_to::<U>() }
}

/// As `align_to_mut`, but safe because of the [`Pod`] bound.
#[inline]
pub fn pod_align_to_mut<
  T: NoUninit + AnyBitPattern,
  U: NoUninit + AnyBitPattern,
>(
  vals: &mut [T],
) -> (&mut [T], &mut [U], &mut [T]) {
  unsafe { vals.align_to_mut::<U>() }
}

/// Try to cast `T` into `U`.
///
/// Note that for this particular type of cast, alignment isn't a factor. The
/// input value is semantically copied into the function and then returned to a
/// new memory location which will have whatever the required alignment of the
/// output type is.
///
/// ## Failure
///
/// * If the types don't have the same size this fails.
#[inline]
pub fn try_cast<A: NoUninit, B: AnyBitPattern>(
  a: A,
) -> Result<B, PodCastError> {
  unsafe { internal::try_cast(a) }
}

/// Try to convert a `&T` into `&U`.
///
/// ## Failure
///
/// * If the reference isn't aligned in the new type
/// * If the source type and target type aren't the same size.
#[inline]
pub fn try_cast_ref<A: NoUninit, B: AnyBitPattern>(
  a: &A,
) -> Result<&B, PodCastError> {
  unsafe { internal::try_cast_ref(a) }
}

/// Try to convert a `&mut T` into `&mut U`.
///
/// As [`try_cast_ref`], but `mut`.
#[inline]
pub fn try_cast_mut<
  A: NoUninit + AnyBitPattern,
  B: NoUninit + AnyBitPattern,
>(
  a: &mut A,
) -> Result<&mut B, PodCastError> {
  unsafe { internal::try_cast_mut(a) }
}

/// Try to convert `&[A]` into `&[B]` (possibly with a change in length).
///
/// * `input.as_ptr() as usize == output.as_ptr() as usize`
/// * `input.len() * size_of::<A>() == output.len() * size_of::<B>()`
///
/// ## Failure
///
/// * If the target type has a greater alignment requirement and the input slice
///   isn't aligned.
/// * If the target element type is a different size from the current element
///   type, and the output slice wouldn't be a whole number of elements when
///   accounting for the size change (eg: 3 `u16` values is 1.5 `u32` values, so
///   that's a failure).
/// * Similarly, you can't convert between a [ZST](https://doc.rust-lang.org/nomicon/exotic-sizes.html#zero-sized-types-zsts)
///   and a non-ZST.
#[inline]
pub fn try_cast_slice<A: NoUninit, B: AnyBitPattern>(
  a: &[A],
) -> Result<&[B], PodCastError> {
  unsafe { internal::try_cast_slice(a) }
}

/// Try to convert `&mut [A]` into `&mut [B]` (possibly with a change in
/// length).
///
/// As [`try_cast_slice`], but `&mut`.
#[inline]
pub fn try_cast_slice_mut<
  A: NoUninit + AnyBitPattern,
  B: NoUninit + AnyBitPattern,
>(
  a: &mut [A],
) -> Result<&mut [B], PodCastError> {
  unsafe { internal::try_cast_slice_mut(a) }
}

/// Fill all bytes of `target` with zeroes (see [`Zeroable`]).
///
/// This is similar to `*target = Zeroable::zeroed()`, but guarantees that any
/// padding bytes in `target` are zeroed as well.
///
/// See also [`fill_zeroes`], if you have a slice rather than a single value.
#[inline]
pub fn write_zeroes<T: Zeroable>(target: &mut T) {
  struct EnsureZeroWrite<T>(*mut T);
  impl<T> Drop for EnsureZeroWrite<T> {
    #[inline(always)]
    fn drop(&mut self) {
      unsafe {
        core::ptr::write_bytes(self.0, 0u8, 1);
      }
    }
  }
  unsafe {
    let guard = EnsureZeroWrite(target);
    core::ptr::drop_in_place(guard.0);
    drop(guard);
  }
}

/// Fill all bytes of `slice` with zeroes (see [`Zeroable`]).
///
/// This is similar to `slice.fill(Zeroable::zeroed())`, but guarantees that any
/// padding bytes in `slice` are zeroed as well.
///
/// See also [`write_zeroes`], which zeroes all bytes of a single value rather
/// than a slice.
#[inline]
pub fn fill_zeroes<T: Zeroable>(slice: &mut [T]) {
  if core::mem::needs_drop::<T>() {
    // If `T` needs to be dropped then we have to do this one item at a time, in
    // case one of the intermediate drops does a panic.
    slice.iter_mut().for_each(write_zeroes);
  } else {
    // Otherwise we can be really fast and just fill everthing with zeros.
    let len = core::mem::size_of_val::<[T]>(slice);
    unsafe { core::ptr::write_bytes(slice.as_mut_ptr() as *mut u8, 0u8, len) }
  }
}
