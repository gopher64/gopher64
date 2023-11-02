//! Extends [ttf_parser](https://docs.rs/ttf-parser) with owned version of
//! [`Face`](struct.Face.html): [`OwnedFace`](struct.OwnedFace.html).
//!
//! Re-exports `ttf_parser::*`.
//!
//! # Example
//! ```
//! use owned_ttf_parser::{AsFaceRef, OwnedFace, Face};
//!
//! # let owned_font_data = include_bytes!("../fonts/font.ttf").to_vec();
//! let owned_face = OwnedFace::from_vec(owned_font_data, 0).unwrap();
//! let face_ref: &Face<'_> = owned_face.as_face_ref();
//!
//! assert_eq!(face_ref.ascender(), 2254);
//! ```
#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

mod convert;
mod owned;
mod preparse;

pub use convert::*;
pub use owned::*;
pub use preparse::*;
pub use ttf_parser::*;
