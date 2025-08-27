//! Unified SIMD compatibility layer to replace architecture-specific intrinsics
//! with a common interface that can be easily migrated to std::simd in the future.

#![allow(non_camel_case_types)]

// Define the arch-specific __m128i type for aarch64 here to avoid circular dependency
#[cfg(target_arch = "aarch64")]
#[derive(Copy, Clone)]
#[allow(dead_code)]
pub struct __m128i(pub std::arch::aarch64::int64x2_t);

// Conditional imports based on target architecture
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

#[cfg(target_arch = "aarch64")]
include!(concat!(env!("OUT_DIR"), "/simd_bindings.rs"));

/// 128-bit SIMD vector type, unified across architectures
#[cfg(target_arch = "x86_64")]
pub type simd128 = std::arch::x86_64::__m128i;

#[cfg(target_arch = "aarch64")]
pub type simd128 = __m128i;

/// Create a zero vector
#[inline]
pub fn simd_setzero() -> simd128 {
    #[cfg(target_arch = "x86_64")]
    unsafe { std::arch::x86_64::_mm_setzero_si128() }
    #[cfg(target_arch = "aarch64")]
    unsafe { _mm_setzero_si128() }
}

/// Set 8 16-bit values
#[inline]
pub fn simd_set_epi16(e7: i16, e6: i16, e5: i16, e4: i16, e3: i16, e2: i16, e1: i16, e0: i16) -> simd128 {
    #[cfg(target_arch = "x86_64")]
    unsafe { std::arch::x86_64::_mm_set_epi16(e7, e6, e5, e4, e3, e2, e1, e0) }
    #[cfg(target_arch = "aarch64")]
    unsafe { _mm_set_epi16(e7, e6, e5, e4, e3, e2, e1, e0) }
}

/// Set 16 8-bit values
#[inline]
pub fn simd_set_epi8(
    e15: i8, e14: i8, e13: i8, e12: i8, e11: i8, e10: i8, e9: i8, e8: i8,
    e7: i8, e6: i8, e5: i8, e4: i8, e3: i8, e2: i8, e1: i8, e0: i8
) -> simd128 {
    #[cfg(target_arch = "x86_64")]
    unsafe { std::arch::x86_64::_mm_set_epi8(e15, e14, e13, e12, e11, e10, e9, e8, e7, e6, e5, e4, e3, e2, e1, e0) }
    #[cfg(target_arch = "aarch64")]
    unsafe { _mm_set_epi8(e15, e14, e13, e12, e11, e10, e9, e8, e7, e6, e5, e4, e3, e2, e1, e0) }
}

/// Set all 16-bit elements to the same value
#[inline]
pub fn simd_set1_epi16(a: i16) -> simd128 {
    #[cfg(target_arch = "x86_64")]
    unsafe { std::arch::x86_64::_mm_set1_epi16(a) }
    #[cfg(target_arch = "aarch64")]
    unsafe { _mm_set1_epi16(a) }
}

/// Set all 8-bit elements to the same value
#[inline]
pub fn simd_set1_epi8(a: i8) -> simd128 {
    #[cfg(target_arch = "x86_64")]
    unsafe { std::arch::x86_64::_mm_set1_epi8(a) }
    #[cfg(target_arch = "aarch64")]
    unsafe { _mm_set1_epi8(a) }
}

/// Set all 32-bit elements to the same value
#[inline]
pub fn simd_set1_epi32(a: i32) -> simd128 {
    #[cfg(target_arch = "x86_64")]
    unsafe { std::arch::x86_64::_mm_set1_epi32(a) }
    #[cfg(target_arch = "aarch64")]
    unsafe { _mm_set1_epi32(a) }
}

/// Add 16-bit integers
#[inline]
pub fn simd_add_epi16(a: simd128, b: simd128) -> simd128 {
    #[cfg(target_arch = "x86_64")]
    unsafe { std::arch::x86_64::_mm_add_epi16(a, b) }
    #[cfg(target_arch = "aarch64")]
    unsafe { _mm_add_epi16(a, b) }
}

/// Subtract 16-bit integers
#[inline]
pub fn simd_sub_epi16(a: simd128, b: simd128) -> simd128 {
    unsafe { _mm_sub_epi16(a, b) }
}

/// Multiply low 16-bit integers
#[inline]
pub fn simd_mullo_epi16(a: simd128, b: simd128) -> simd128 {
    unsafe { _mm_mullo_epi16(a, b) }
}

/// Multiply high 16-bit signed integers
#[inline]
pub fn simd_mulhi_epi16(a: simd128, b: simd128) -> simd128 {
    unsafe { _mm_mulhi_epi16(a, b) }
}

/// Multiply high 16-bit unsigned integers
#[inline]
pub fn simd_mulhi_epu16(a: simd128, b: simd128) -> simd128 {
    unsafe { _mm_mulhi_epu16(a, b) }
}

/// Compare 16-bit integers for equality
#[inline]
pub fn simd_cmpeq_epi16(a: simd128, b: simd128) -> simd128 {
    unsafe { _mm_cmpeq_epi16(a, b) }
}

/// Compare 16-bit integers for greater than
#[inline]
pub fn simd_cmpgt_epi16(a: simd128, b: simd128) -> simd128 {
    unsafe { _mm_cmpgt_epi16(a, b) }
}

/// Compare 16-bit integers for less than
#[inline]
pub fn simd_cmplt_epi16(a: simd128, b: simd128) -> simd128 {
    unsafe { _mm_cmplt_epi16(a, b) }
}

/// Bitwise AND
#[inline]
pub fn simd_and(a: simd128, b: simd128) -> simd128 {
    unsafe { _mm_and_si128(a, b) }
}

/// Bitwise OR
#[inline]
pub fn simd_or(a: simd128, b: simd128) -> simd128 {
    unsafe { _mm_or_si128(a, b) }
}

/// Bitwise XOR
#[inline]
pub fn simd_xor(a: simd128, b: simd128) -> simd128 {
    unsafe { _mm_xor_si128(a, b) }
}

/// Bitwise AND NOT
#[inline]
pub fn simd_andnot(a: simd128, b: simd128) -> simd128 {
    unsafe { _mm_andnot_si128(a, b) }
}

/// Arithmetic right shift 16-bit integers
#[inline]
pub fn simd_srai_epi16(a: simd128, imm8: i32) -> simd128 {
    unsafe { _mm_srai_epi16(a, imm8) }
}

/// Logical left shift 16-bit integers
#[inline]
pub fn simd_slli_epi16(a: simd128, imm8: i32) -> simd128 {
    unsafe { _mm_slli_epi16(a, imm8) }
}

/// Logical right shift 16-bit integers
#[inline]
pub fn simd_srli_epi16(a: simd128, imm8: i32) -> simd128 {
    unsafe { _mm_srli_epi16(a, imm8) }
}

/// Add 16-bit unsigned integers with saturation
#[inline]
pub fn simd_adds_epu16(a: simd128, b: simd128) -> simd128 {
    unsafe { _mm_adds_epu16(a, b) }
}

/// Add 16-bit signed integers with saturation
#[inline]
pub fn simd_adds_epi16(a: simd128, b: simd128) -> simd128 {
    unsafe { _mm_adds_epi16(a, b) }
}

/// Subtract 16-bit signed integers with saturation
#[inline]
pub fn simd_subs_epi16(a: simd128, b: simd128) -> simd128 {
    unsafe { _mm_subs_epi16(a, b) }
}

/// Subtract 16-bit unsigned integers with saturation
#[inline]
pub fn simd_subs_epu16(a: simd128, b: simd128) -> simd128 {
    unsafe { _mm_subs_epu16(a, b) }
}

/// Min of 16-bit signed integers
#[inline]
pub fn simd_min_epi16(a: simd128, b: simd128) -> simd128 {
    unsafe { _mm_min_epi16(a, b) }
}

/// Max of 16-bit signed integers
#[inline]
pub fn simd_max_epi16(a: simd128, b: simd128) -> simd128 {
    unsafe { _mm_max_epi16(a, b) }
}

/// Unpack low 16-bit values
#[inline]
pub fn simd_unpacklo_epi16(a: simd128, b: simd128) -> simd128 {
    unsafe { _mm_unpacklo_epi16(a, b) }
}

/// Unpack high 16-bit values
#[inline]
pub fn simd_unpackhi_epi16(a: simd128, b: simd128) -> simd128 {
    unsafe { _mm_unpackhi_epi16(a, b) }
}

/// Pack 32-bit signed integers to 16-bit signed integers with saturation
#[inline]
pub fn simd_packs_epi32(a: simd128, b: simd128) -> simd128 {
    unsafe { _mm_packs_epi32(a, b) }
}

/// Pack 16-bit signed integers to 8-bit signed integers with saturation
#[inline]
pub fn simd_packs_epi16(a: simd128, b: simd128) -> simd128 {
    unsafe { _mm_packs_epi16(a, b) }
}

/// Get movemask from 8-bit values
#[inline]
pub fn simd_movemask_epi8(a: simd128) -> i32 {
    unsafe { _mm_movemask_epi8(a) }
}

/// Shuffle bytes according to indices
#[inline]
pub fn simd_shuffle_epi8(a: simd128, b: simd128) -> simd128 {
    unsafe { _mm_shuffle_epi8(a, b) }
}

/// Conditional blend based on mask
#[inline]
pub fn simd_blendv_epi8(a: simd128, b: simd128, mask: simd128) -> simd128 {
    unsafe { _mm_blendv_epi8(a, b, mask) }
}

/// Extract 8-bit integer
#[inline]
pub fn simd_extract_epi8(a: simd128, imm8: i32) -> i32 {
    unsafe { _mm_extract_epi8(a, imm8) }
}

/// Insert 8-bit integer
#[inline]
pub fn simd_insert_epi8(a: simd128, i: i32, imm8: i32) -> simd128 {
    unsafe { _mm_insert_epi8(a, i, imm8) }
}

/// Extract 16-bit integer
#[inline]
pub fn simd_extract_epi16(a: simd128, imm8: i32) -> i32 {
    unsafe { _mm_extract_epi16(a, imm8) }
}

/// Insert 16-bit integer
#[inline]
pub fn simd_insert_epi16(a: simd128, i: i32, imm8: i32) -> simd128 {
    unsafe { _mm_insert_epi16(a, i, imm8) }
}

/// Extract 32-bit integer
#[inline]
pub fn simd_extract_epi32(a: simd128, imm8: i32) -> i32 {
    unsafe { _mm_extract_epi32(a, imm8) }
}

/// Insert 32-bit integer
#[inline]
pub fn simd_insert_epi32(a: simd128, i: i32, imm8: i32) -> simd128 {
    unsafe { _mm_insert_epi32(a, i, imm8) }
}

/// Extract 64-bit integer
#[inline]
pub fn simd_extract_epi64(a: simd128, imm8: i32) -> i64 {
    unsafe { _mm_extract_epi64(a, imm8) }
}

/// Insert 64-bit integer
#[inline]
pub fn simd_insert_epi64(a: simd128, i: i64, imm8: i32) -> simd128 {
    unsafe { _mm_insert_epi64(a, i, imm8) }
}

/// Zero vector constant for initialization
#[inline]
pub fn simd_zero() -> simd128 {
    unsafe { _mm_setzero_si128() }
}

// Convenience re-exports of the original intrinsic names for compatibility
pub use simd_setzero as _mm_setzero_si128;
pub use simd_set_epi16 as _mm_set_epi16;
pub use simd_set_epi8 as _mm_set_epi8;
pub use simd_set1_epi16 as _mm_set1_epi16;
pub use simd_set1_epi8 as _mm_set1_epi8;
pub use simd_set1_epi32 as _mm_set1_epi32;
pub use simd_add_epi16 as _mm_add_epi16;
pub use simd_sub_epi16 as _mm_sub_epi16;
pub use simd_mullo_epi16 as _mm_mullo_epi16;
pub use simd_mulhi_epi16 as _mm_mulhi_epi16;
pub use simd_mulhi_epu16 as _mm_mulhi_epu16;
pub use simd_cmpeq_epi16 as _mm_cmpeq_epi16;
pub use simd_cmpgt_epi16 as _mm_cmpgt_epi16;
pub use simd_cmplt_epi16 as _mm_cmplt_epi16;
pub use simd_and as _mm_and_si128;
pub use simd_or as _mm_or_si128;
pub use simd_xor as _mm_xor_si128;
pub use simd_andnot as _mm_andnot_si128;
pub use simd_srai_epi16 as _mm_srai_epi16;
pub use simd_slli_epi16 as _mm_slli_epi16;
pub use simd_srli_epi16 as _mm_srli_epi16;
pub use simd_adds_epu16 as _mm_adds_epu16;
pub use simd_adds_epi16 as _mm_adds_epi16;
pub use simd_subs_epi16 as _mm_subs_epi16;
pub use simd_subs_epu16 as _mm_subs_epu16;
pub use simd_min_epi16 as _mm_min_epi16;
pub use simd_max_epi16 as _mm_max_epi16;
pub use simd_unpacklo_epi16 as _mm_unpacklo_epi16;
pub use simd_unpackhi_epi16 as _mm_unpackhi_epi16;
pub use simd_packs_epi32 as _mm_packs_epi32;
pub use simd_packs_epi16 as _mm_packs_epi16;
pub use simd_movemask_epi8 as _mm_movemask_epi8;
pub use simd_shuffle_epi8 as _mm_shuffle_epi8;
pub use simd_blendv_epi8 as _mm_blendv_epi8;
pub use simd_extract_epi8 as _mm_extract_epi8;
pub use simd_insert_epi8 as _mm_insert_epi8;
pub use simd_extract_epi16 as _mm_extract_epi16;
pub use simd_insert_epi16 as _mm_insert_epi16;
pub use simd_extract_epi32 as _mm_extract_epi32;
pub use simd_insert_epi32 as _mm_insert_epi32;
pub use simd_extract_epi64 as _mm_extract_epi64;
pub use simd_insert_epi64 as _mm_insert_epi64;

// Re-export the type with its original name for compatibility
pub use simd128 as __m128i;