//! API for loading, scaling, positioning and rasterizing OpenType font glyphs.
//!
//! # Example
//! ```
//! use ab_glyph::{point, Font, FontRef, Glyph};
//!
//! # fn main() -> Result<(), ab_glyph::InvalidFont> {
//! let font = FontRef::try_from_slice(include_bytes!("../../dev/fonts/Exo2-Light.otf"))?;
//!
//! // Get a glyph for 'q' with a scale & position.
//! let q_glyph: Glyph = font
//!     .glyph_id('q')
//!     .with_scale_and_position(24.0, point(100.0, 0.0));
//!
//! // Draw it.
//! if let Some(q) = font.outline_glyph(q_glyph) {
//!     q.draw(|x, y, c| { /* draw pixel `(x, y)` with coverage: `c` */ });
//! }
//! # Ok(()) }
//! ```
#![warn(missing_debug_implementations)]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod codepoint_ids;
mod err;
mod font;
#[cfg(feature = "std")]
mod font_arc;
mod glyph;
#[cfg(all(feature = "libm", not(feature = "std")))]
mod nostd_float;
mod outlined;
mod scale;
mod ttfp;
#[cfg(feature = "variable-fonts")]
mod variable;

#[cfg(feature = "std")]
pub use crate::font_arc::*;
#[allow(deprecated)]
pub use crate::{
    codepoint_ids::*,
    err::*,
    font::*,
    glyph::*,
    outlined::*,
    scale::*,
    ttfp::{FontRef, FontVec},
};
pub use ab_glyph_rasterizer::{point, Point};
#[cfg(feature = "variable-fonts")]
pub use variable::*;
