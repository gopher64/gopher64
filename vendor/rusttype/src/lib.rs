//! RustType is a pure Rust alternative to libraries like FreeType.
//!
//! The current capabilities of RustType:
//!
//! * Reading TrueType formatted fonts and font collections. This includes
//!   `*.ttf` as well as a subset of `*.otf` font files.
//! * Retrieving glyph shapes and commonly used properties for a font and its
//!   glyphs.
//! * Laying out glyphs horizontally using horizontal and vertical metrics, and
//!   glyph-pair-specific kerning.
//! * Rasterising glyphs with sub-pixel positioning using an accurate analytical
//!   algorithm (not based on sampling).
//! * Managing a font cache on the GPU with the `gpu_cache` module. This keeps
//!   recently used glyph renderings in a dynamic cache in GPU memory to
//!   minimise texture uploads per-frame. It also allows you keep the draw call
//!   count for text very low, as all glyphs are kept in one GPU texture.
//!
//! Notable things that RustType does not support *yet*:
//!
//! * OpenType formatted fonts that are not just TrueType fonts (OpenType is a
//!   superset of TrueType). Notably there is no support yet for cubic Bezier
//!   curves used in glyphs.
//! * Font hinting.
//! * Ligatures of any kind.
//! * Some less common TrueType sub-formats.
//! * Right-to-left and vertical text layout.
//!
//! # Getting Started
//!
//! To hit the ground running with RustType, look at the `ascii.rs` example
//! supplied with the crate. It demonstrates loading a font file, rasterising an
//! arbitrary string, and displaying the result as ASCII art. If you prefer to
//! just look at the documentation, the entry point for loading fonts is
//! `Font`, from which you can access individual fonts, then their
//! glyphs.
//!
//! # Glyphs
//!
//! The glyph API uses wrapper structs to augment a glyph with information such
//! as scaling and positioning, making relevant methods that make use of this
//! information available as appropriate. For example, given a `Glyph` `glyph`
//! obtained directly from a `Font`:
//!
//! ```no_run
//! # use rusttype::*;
//! # let glyph: Glyph<'static> = unimplemented!();
//! // One of the few things you can do with an unsized, positionless glyph is get its id.
//! let id = glyph.id();
//! let glyph = glyph.scaled(Scale::uniform(10.0));
//! // Now glyph is a ScaledGlyph, you can do more with it, as well as what you can do with Glyph.
//! // For example, you can access the correctly scaled horizontal metrics for the glyph.
//! let h_metrics = glyph.h_metrics();
//! let glyph = glyph.positioned(point(5.0, 3.0));
//! // Now glyph is a PositionedGlyph, and you can do even more with it, e.g. drawing.
//! glyph.draw(|x, y, v| {}); // In this case the pixel values are not used.
//! ```
//!
//! # Unicode terminology
//!
//! This crate uses terminology for computerised typography as specified by the
//! Unicode standard. If you are not sure of the differences between a code
//! point, a character, and a glyph, you may want to check the [official Unicode
//! glossary](http://unicode.org/glossary/), or alternatively, here's my take on
//! it from a practical perspective:
//!
//! * A character is what you would conventionally call a single symbol,
//!   independent of its appearance or representation in a particular font.
//!   Examples include `a`, `A`, `ä`, `å`, `1`, `*`, `Ω`, etc.
//! * A Unicode code point is the particular number that the Unicode standard
//!   associates with a particular character. Note however that code points also
//!   exist for things not conventionally thought of as characters by
//!   themselves, but can be combined to form characters, such as diacritics
//!   like accents. These "characters" are known in Unicode as "combining
//!   characters". E.g., a diaeresis (`¨`) has the code point U+0308. If this
//!   code point follows the code point U+0055 (the letter `u`), this sequence
//!   represents the character `ü`. Note that there is also a single codepoint
//!   for `ü`, U+00FC. This means that what visually looks like the same string
//!   can have multiple different Unicode representations. Some fonts will have
//!   glyphs (see below) for one sequence of codepoints, but not another that
//!   has the same meaning. To deal with this problem it is recommended to use
//!   Unicode normalisation, as provided by, for example, the
//!   [unicode-normalization](http://crates.io/crates/unicode-normalization)
//!   crate, to convert to code point sequences that work with the font in
//!   question. Typically a font is more likely to support a single code point
//!   vs. a sequence with the same meaning, so the best normalisation to use is
//!   "canonical recomposition", known as NFC in the normalisation crate.
//! * A glyph is a particular font's shape to draw the character for a
//!   particular Unicode code point. This will have its own identifying number
//!   unique to the font, its ID.
#![allow(
    clippy::cognitive_complexity,
    clippy::doc_markdown,
    clippy::cast_lossless,
    clippy::many_single_char_names
)]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod font;
mod geometry;
mod outliner;

#[cfg(all(feature = "libm-math", not(feature = "std")))]
mod nostd_float;

#[cfg(feature = "gpu_cache")]
pub mod gpu_cache;

pub use crate::geometry::{point, vector, Point, Rect, Vector};
pub use font::*;

use core::fmt;

#[cfg(all(feature = "libm-math", not(feature = "std")))]
use crate::nostd_float::FloatExt;

pub use owned_ttf_parser::OutlineBuilder;

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct GlyphId(pub u16);

impl From<owned_ttf_parser::GlyphId> for GlyphId {
    fn from(id: owned_ttf_parser::GlyphId) -> Self {
        Self(id.0)
    }
}
impl From<GlyphId> for owned_ttf_parser::GlyphId {
    fn from(id: GlyphId) -> Self {
        Self(id.0)
    }
}

/// A single glyph of a font.
///
/// A `Glyph` does not have an inherent scale or position associated with it. To
/// augment a glyph with a size, give it a scale using `scaled`. You can then
/// position it using `positioned`.
#[derive(Clone)]
pub struct Glyph<'font> {
    font: Font<'font>,
    id: GlyphId,
}

impl<'font> Glyph<'font> {
    /// The font to which this glyph belongs.
    pub fn font(&self) -> &Font<'font> {
        &self.font
    }

    /// The glyph identifier for this glyph.
    pub fn id(&self) -> GlyphId {
        self.id
    }

    /// Augments this glyph with scaling information, making methods that depend
    /// on the scale of the glyph available.
    pub fn scaled(self, scale: Scale) -> ScaledGlyph<'font> {
        let scale_y = self.font.scale_for_pixel_height(scale.y);
        let scale_x = scale_y * scale.x / scale.y;
        ScaledGlyph {
            g: self,
            api_scale: scale,
            scale: vector(scale_x, scale_y),
        }
    }
}

impl fmt::Debug for Glyph<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Glyph").field("id", &self.id().0).finish()
    }
}

/// The "horizontal metrics" of a glyph. This is useful for calculating the
/// horizontal offset of a glyph from the previous one in a string when laying a
/// string out horizontally.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct HMetrics {
    /// The horizontal offset that the origin of the next glyph should be from
    /// the origin of this glyph.
    pub advance_width: f32,
    /// The horizontal offset between the origin of this glyph and the leftmost
    /// edge/point of the glyph.
    pub left_side_bearing: f32,
}

/// The "vertical metrics" of a font at a particular scale. This is useful for
/// calculating the amount of vertical space to give a line of text, and for
/// computing the vertical offset between successive lines.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct VMetrics {
    /// The highest point that any glyph in the font extends to above the
    /// baseline. Typically positive.
    pub ascent: f32,
    /// The lowest point that any glyph in the font extends to below the
    /// baseline. Typically negative.
    pub descent: f32,
    /// The gap to leave between the descent of one line and the ascent of the
    /// next. This is of course only a guideline given by the font's designers.
    pub line_gap: f32,
}

impl core::ops::Mul<f32> for VMetrics {
    type Output = VMetrics;

    fn mul(self, rhs: f32) -> Self {
        Self {
            ascent: self.ascent * rhs,
            descent: self.descent * rhs,
            line_gap: self.line_gap * rhs,
        }
    }
}

/// A glyph augmented with scaling information. You can query such a glyph for
/// information that depends on the scale of the glyph.
#[derive(Clone)]
pub struct ScaledGlyph<'font> {
    g: Glyph<'font>,
    api_scale: Scale,
    scale: Vector<f32>,
}

impl<'font> ScaledGlyph<'font> {
    /// The glyph identifier for this glyph.
    pub fn id(&self) -> GlyphId {
        self.g.id()
    }

    /// The font to which this glyph belongs.
    #[inline]
    pub fn font(&self) -> &Font<'font> {
        self.g.font()
    }

    /// A reference to this glyph without the scaling
    pub fn into_unscaled(self) -> Glyph<'font> {
        self.g
    }

    /// Removes the scaling from this glyph
    pub fn unscaled(&self) -> &Glyph<'font> {
        &self.g
    }

    /// Builds the outline of the glyph with the builder specified. Returns
    /// `false` when the outline is either malformed or empty.
    pub fn build_outline(&self, builder: &mut impl OutlineBuilder) -> bool {
        let mut outliner =
            crate::outliner::OutlineScaler::new(builder, vector(self.scale.x, -self.scale.y));

        self.font()
            .inner()
            .outline_glyph(self.id().into(), &mut outliner)
            .is_some()
    }

    /// Augments this glyph with positioning information, making methods that
    /// depend on the position of the glyph available.
    pub fn positioned(self, p: Point<f32>) -> PositionedGlyph<'font> {
        let bb = self.pixel_bounds_at(p);
        PositionedGlyph {
            sg: self,
            position: p,
            bb,
        }
    }

    pub fn scale(&self) -> Scale {
        self.api_scale
    }

    /// Retrieves the "horizontal metrics" of this glyph. See `HMetrics` for
    /// more detail.
    pub fn h_metrics(&self) -> HMetrics {
        let inner = self.font().inner();
        let id = self.id().into();

        let advance = inner.glyph_hor_advance(id).unwrap();
        let left_side_bearing = inner.glyph_hor_side_bearing(id).unwrap();

        HMetrics {
            advance_width: advance as f32 * self.scale.x,
            left_side_bearing: left_side_bearing as f32 * self.scale.x,
        }
    }

    /// The bounding box of the shape of this glyph, not to be confused with
    /// `pixel_bounding_box`, the conservative pixel-boundary bounding box. The
    /// coordinates are relative to the glyph's origin.
    pub fn exact_bounding_box(&self) -> Option<Rect<f32>> {
        let owned_ttf_parser::Rect {
            x_min,
            y_min,
            x_max,
            y_max,
        } = self.font().inner().glyph_bounding_box(self.id().into())?;

        Some(Rect {
            min: point(x_min as f32 * self.scale.x, -y_max as f32 * self.scale.y),
            max: point(x_max as f32 * self.scale.x, -y_min as f32 * self.scale.y),
        })
    }

    fn glyph_bitmap_box_subpixel(
        &self,
        font: &Font<'font>,
        shift_x: f32,
        shift_y: f32,
    ) -> Option<Rect<i32>> {
        let owned_ttf_parser::Rect {
            x_min,
            y_min,
            x_max,
            y_max,
        } = font.inner().glyph_bounding_box(self.id().into())?;

        Some(Rect {
            min: point(
                (x_min as f32 * self.scale.x + shift_x).floor() as i32,
                (-y_max as f32 * self.scale.y + shift_y).floor() as i32,
            ),
            max: point(
                (x_max as f32 * self.scale.x + shift_x).ceil() as i32,
                (-y_min as f32 * self.scale.y + shift_y).ceil() as i32,
            ),
        })
    }

    #[inline]
    fn pixel_bounds_at(&self, p: Point<f32>) -> Option<Rect<i32>> {
        // Use subpixel fraction in floor/ceil rounding to eliminate rounding error
        // from identical subpixel positions
        let (x_trunc, x_fract) = (p.x.trunc() as i32, p.x.fract());
        let (y_trunc, y_fract) = (p.y.trunc() as i32, p.y.fract());

        let Rect { min, max } = self.glyph_bitmap_box_subpixel(self.font(), x_fract, y_fract)?;
        Some(Rect {
            min: point(x_trunc + min.x, y_trunc + min.y),
            max: point(x_trunc + max.x, y_trunc + max.y),
        })
    }
}

impl fmt::Debug for ScaledGlyph<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScaledGlyph")
            .field("id", &self.id().0)
            .field("scale", &self.api_scale)
            .finish()
    }
}

/// A glyph augmented with positioning and scaling information. You can query
/// such a glyph for information that depends on the scale and position of the
/// glyph.
#[derive(Clone)]
pub struct PositionedGlyph<'font> {
    sg: ScaledGlyph<'font>,
    position: Point<f32>,
    bb: Option<Rect<i32>>,
}

impl<'font> PositionedGlyph<'font> {
    /// The glyph identifier for this glyph.
    pub fn id(&self) -> GlyphId {
        self.sg.id()
    }

    /// The font to which this glyph belongs.
    #[inline]
    pub fn font(&self) -> &Font<'font> {
        self.sg.font()
    }

    /// A reference to this glyph without positioning
    pub fn unpositioned(&self) -> &ScaledGlyph<'font> {
        &self.sg
    }

    /// Removes the positioning from this glyph
    pub fn into_unpositioned(self) -> ScaledGlyph<'font> {
        self.sg
    }

    /// The conservative pixel-boundary bounding box for this glyph. This is the
    /// smallest rectangle aligned to pixel boundaries that encloses the shape
    /// of this glyph at this position. Note that the origin of the glyph, at
    /// pixel-space coordinates (0, 0), is at the top left of the bounding box.
    pub fn pixel_bounding_box(&self) -> Option<Rect<i32>> {
        self.bb
    }

    pub fn scale(&self) -> Scale {
        self.sg.api_scale
    }

    pub fn position(&self) -> Point<f32> {
        self.position
    }

    /// Builds the outline of the glyph with the builder specified. Returns
    /// `false` when the outline is either malformed or empty.
    pub fn build_outline(&self, builder: &mut impl OutlineBuilder) -> bool {
        let bb = if let Some(bb) = self.bb.as_ref() {
            bb
        } else {
            return false;
        };

        let offset = vector(bb.min.x as f32, bb.min.y as f32);

        let mut outliner = crate::outliner::OutlineTranslator::new(builder, self.position - offset);

        self.sg.build_outline(&mut outliner)
    }

    /// Rasterises this glyph. For each pixel in the rect given by
    /// `pixel_bounding_box()`, `o` is called:
    ///
    /// ```ignore
    /// o(x, y, v)
    /// ```
    ///
    /// where `x` and `y` are the coordinates of the pixel relative to the `min`
    /// coordinates of the bounding box, and `v` is the analytically calculated
    /// coverage of the pixel by the shape of the glyph. Calls to `o` proceed in
    /// horizontal scanline order, similar to this pseudo-code:
    ///
    /// ```ignore
    /// let bb = glyph.pixel_bounding_box();
    /// for y in 0..bb.height() {
    ///     for x in 0..bb.width() {
    ///         o(x, y, calc_coverage(&glyph, x, y));
    ///     }
    /// }
    /// ```
    pub fn draw<O: FnMut(u32, u32, f32)>(&self, o: O) {
        let bb = if let Some(bb) = self.bb.as_ref() {
            bb
        } else {
            return;
        };

        let width = (bb.max.x - bb.min.x) as u32;
        let height = (bb.max.y - bb.min.y) as u32;

        let mut outliner = crate::outliner::OutlineRasterizer::new(width as _, height as _);

        self.build_outline(&mut outliner);

        outliner.rasterizer.for_each_pixel_2d(o);
    }

    /// Resets positioning information and recalculates the pixel bounding box
    pub fn set_position(&mut self, p: Point<f32>) {
        let p_diff = p - self.position;
        if p_diff.x.fract().is_near_zero() && p_diff.y.fract().is_near_zero() {
            if let Some(bb) = self.bb.as_mut() {
                let rounded_diff = vector(p_diff.x.round() as i32, p_diff.y.round() as i32);
                bb.min = bb.min + rounded_diff;
                bb.max = bb.max + rounded_diff;
            }
        } else {
            self.bb = self.sg.pixel_bounds_at(p);
        }
        self.position = p;
    }
}

impl fmt::Debug for PositionedGlyph<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PositionedGlyph")
            .field("id", &self.id().0)
            .field("scale", &self.scale())
            .field("position", &self.position)
            .finish()
    }
}

/// Defines the size of a rendered face of a font, in pixels, horizontally and
/// vertically. A vertical scale of `y` pixels means that the distance between
/// the ascent and descent lines (see `VMetrics`) of the face will be `y`
/// pixels. If `x` and `y` are equal the scaling is uniform. Non-uniform scaling
/// by a factor *f* in the horizontal direction is achieved by setting `x` equal
/// to *f* times `y`.
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub struct Scale {
    /// Horizontal scale, in pixels.
    pub x: f32,
    /// Vertical scale, in pixels.
    pub y: f32,
}

impl Scale {
    /// Uniform scaling, equivalent to `Scale { x: s, y: s }`.
    #[inline]
    pub fn uniform(s: f32) -> Scale {
        Scale { x: s, y: s }
    }
}
/// A trait for types that can be converted into a `GlyphId`, in the context of
/// a specific font.
///
/// Many `rusttype` functions that operate on characters accept values of any
/// type that implements `IntoGlyphId`. Such types include `char`, `Codepoint`,
/// and obviously `GlyphId` itself.
pub trait IntoGlyphId {
    /// Convert `self` into a `GlyphId`, consulting the index map of `font` if
    /// necessary.
    fn into_glyph_id(self, font: &Font<'_>) -> GlyphId;
}
impl IntoGlyphId for char {
    #[inline]
    fn into_glyph_id(self, font: &Font<'_>) -> GlyphId {
        font.inner()
            .glyph_index(self)
            .unwrap_or(owned_ttf_parser::GlyphId(0))
            .into()
    }
}
impl<G: Into<GlyphId>> IntoGlyphId for G {
    #[inline]
    fn into_glyph_id(self, _font: &Font<'_>) -> GlyphId {
        self.into()
    }
}

#[derive(Clone)]
pub struct GlyphIter<'a, 'font, I: Iterator>
where
    I::Item: IntoGlyphId,
{
    font: &'a Font<'font>,
    itr: I,
}

impl<'a, 'font, I> Iterator for GlyphIter<'a, 'font, I>
where
    I: Iterator,
    I::Item: IntoGlyphId,
{
    type Item = Glyph<'font>;

    fn next(&mut self) -> Option<Glyph<'font>> {
        self.itr.next().map(|c| self.font.glyph(c))
    }
}

#[derive(Clone)]
pub struct LayoutIter<'a, 'font, 's> {
    font: &'a Font<'font>,
    chars: core::str::Chars<'s>,
    caret: f32,
    scale: Scale,
    start: Point<f32>,
    last_glyph: Option<GlyphId>,
}

impl<'a, 'font, 's> Iterator for LayoutIter<'a, 'font, 's> {
    type Item = PositionedGlyph<'font>;

    fn next(&mut self) -> Option<PositionedGlyph<'font>> {
        self.chars.next().map(|c| {
            let g = self.font.glyph(c).scaled(self.scale);
            if let Some(last) = self.last_glyph {
                self.caret += self.font.pair_kerning(self.scale, last, g.id());
            }
            let g = g.positioned(point(self.start.x + self.caret, self.start.y));
            self.caret += g.sg.h_metrics().advance_width;
            self.last_glyph = Some(g.id());
            g
        })
    }
}

pub(crate) trait NearZero {
    /// Returns if this number is kinda pretty much zero.
    fn is_near_zero(&self) -> bool;
}
impl NearZero for f32 {
    #[inline]
    fn is_near_zero(&self) -> bool {
        self.abs() <= core::f32::EPSILON
    }
}
