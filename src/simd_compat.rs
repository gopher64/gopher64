//! Simplified SIMD compatibility layer that provides consistent interface
//! while preparing for future std::simd migration when it becomes stable.

#![allow(non_camel_case_types)]

// Re-export arch-specific intrinsics with a unified interface
#[cfg(target_arch = "x86_64")]
pub use std::arch::x86_64::*;

#[cfg(target_arch = "aarch64")]
#[derive(Copy, Clone)]
#[allow(dead_code)]
pub struct __m128i(pub std::arch::aarch64::int64x2_t);

#[cfg(target_arch = "aarch64")]
include!(concat!(env!("OUT_DIR"), "/simd_bindings.rs"));

#[cfg(target_arch = "aarch64")]
include!("compat/aarch64.rs");

// Future: Add utility functions here if needed for std::simd migration