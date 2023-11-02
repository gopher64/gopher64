#[cfg(all(feature = "libm", not(feature = "std")))]
use crate::nostd_float::FloatExt;
use crate::{Font, Glyph, GlyphId, OutlinedGlyph, Rect};

/// Pixel scale.
///
/// This is the pixel-height of text.
///
/// Usually one uses `x == y`, but one may use a different ratio to stretch a
/// font horizontally or vertically.
///
/// To convert pt size into pixel-scale see [`Font::pt_to_px_scale`].
///
/// # Example
/// ```
/// use ab_glyph::PxScale;
///
/// let uniform_scale_24px = PxScale::from(24.0);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct PxScale {
    /// Horizontal scale in pixels.
    pub x: f32,
    /// Vertical scale in pixels.
    ///
    /// By definition, this is the pixel-height of a font.
    pub y: f32,
}

impl PxScale {
    /// Returns a `PxScale` with both x & y scale values set to the nearest integer.
    #[inline]
    pub fn round(self) -> Self {
        Self {
            x: self.x.round(),
            y: self.y.round(),
        }
    }
}

impl From<f32> for PxScale {
    /// Uniform scaling where x & y are the same.
    #[inline]
    fn from(s: f32) -> Self {
        PxScale { x: s, y: s }
    }
}

/// 2D scale factors for use with unscaled metrics.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct PxScaleFactor {
    pub horizontal: f32,
    pub vertical: f32,
}

/// A [`Font`](trait.Font.html) with an associated pixel scale. This can be used to provide
/// pixel scale values for glyph advances, heights etc.
///
/// # Example
/// ```
/// use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
///
/// # fn main() -> Result<(), ab_glyph::InvalidFont> {
/// let font = FontRef::try_from_slice(include_bytes!("../../dev/fonts/Exo2-Light.otf"))?;
///
/// // Associate the font with a scale of 45px
/// let scaled_font = font.as_scaled(PxScale::from(45.0));
///
/// assert_eq!(scaled_font.height(), 45.0);
/// assert_eq!(scaled_font.h_advance(scaled_font.glyph_id('b')), 21.225);
///
/// // Replace associated scale with another
/// let scaled_font = scaled_font.with_scale(180.0);
///
/// assert_eq!(scaled_font.height(), 180.0);
/// assert_eq!(scaled_font.h_advance(scaled_font.glyph_id('b')), 84.9);
/// # Ok(()) }
/// ```
pub trait ScaleFont<F: Font> {
    /// Returns the pixel scale associated with this font.
    fn scale(&self) -> PxScale;

    /// Returns a font reference.
    fn font(&self) -> &F;

    /// Scale factor for unscaled font horizontal values.
    #[inline]
    fn h_scale_factor(&self) -> f32 {
        self.scale().x / self.font().height_unscaled()
    }

    /// Scale factor for unscaled font vertical values.
    #[inline]
    fn v_scale_factor(&self) -> f32 {
        self.scale().y / self.font().height_unscaled()
    }

    #[inline]
    fn scale_factor(&self) -> PxScaleFactor {
        PxScaleFactor {
            horizontal: self.h_scale_factor(),
            vertical: self.v_scale_factor(),
        }
    }

    /// Pixel scaled glyph ascent.
    #[inline]
    fn ascent(&self) -> f32 {
        self.v_scale_factor() * self.font().ascent_unscaled()
    }

    /// Pixel scaled glyph descent.
    #[inline]
    fn descent(&self) -> f32 {
        self.v_scale_factor() * self.font().descent_unscaled()
    }

    /// Pixel scaled height `ascent - descent`.
    ///
    /// By definition of [`PxScale`], this is `self.scale().y`.
    #[inline]
    fn height(&self) -> f32 {
        self.scale().y
    }

    /// Pixel scaled line gap.
    #[inline]
    fn line_gap(&self) -> f32 {
        self.v_scale_factor() * self.font().line_gap_unscaled()
    }

    /// Lookup a `GlyphId` matching a given `char`.
    #[inline]
    fn glyph_id(&self, c: char) -> GlyphId {
        self.font().glyph_id(c)
    }

    /// Construct a [`Glyph`](struct.Glyph.html) with the font's pixel scale at
    /// position `point(0.0, 0.0)`.
    ///
    /// # Example
    /// ```
    /// # use ab_glyph::*;
    /// # let font = FontRef::try_from_slice(include_bytes!("../../dev/fonts/Exo2-Light.otf")).unwrap();
    /// let scaled_font = font.as_scaled(50.0);
    ///
    /// let a1 = scaled_font.scaled_glyph('a');
    /// let a2 = font.glyph_id('a').with_scale(50.0); // equivalent
    ///
    /// # assert_eq!(a1.id, a2.id);
    /// assert_eq!(a1.scale, PxScale::from(50.0));
    /// assert_eq!(a1.position, point(0.0, 0.0));
    /// ```
    #[inline]
    fn scaled_glyph(&self, c: char) -> Glyph {
        self.font().glyph_id(c).with_scale(self.scale())
    }

    /// Pixel scaled horizontal advance for a given glyph.
    #[inline]
    fn h_advance(&self, id: GlyphId) -> f32 {
        self.h_scale_factor() * self.font().h_advance_unscaled(id)
    }

    /// Pixel scaled horizontal side bearing for a given glyph.
    #[inline]
    fn h_side_bearing(&self, id: GlyphId) -> f32 {
        self.h_scale_factor() * self.font().h_side_bearing_unscaled(id)
    }

    /// Pixel scaled vertical advance for a given glyph.
    #[inline]
    fn v_advance(&self, id: GlyphId) -> f32 {
        self.v_scale_factor() * self.font().v_advance_unscaled(id)
    }

    /// Pixel scaled vertical side bearing for a given glyph.
    #[inline]
    fn v_side_bearing(&self, id: GlyphId) -> f32 {
        self.v_scale_factor() * self.font().v_side_bearing_unscaled(id)
    }

    /// Returns additional pixel scaled kerning to apply for a particular pair of glyphs.
    #[inline]
    fn kern(&self, first: GlyphId, second: GlyphId) -> f32 {
        self.h_scale_factor() * self.font().kern_unscaled(first, second)
    }

    /// Returns the layout bounds of this glyph. These are different to the outline `px_bounds()`.
    ///
    /// Horizontally: Glyph position +/- h_advance/h_side_bearing.
    /// Vertically: Glyph position +/- ascent/descent.
    ///
    /// Note this method does not make use of the associated scale, as `Glyph`
    /// already includes one of it's own.
    #[inline]
    fn glyph_bounds(&self, glyph: &Glyph) -> Rect {
        self.font().glyph_bounds(glyph)
    }

    /// The number of glyphs present in this font. Glyph identifiers for this
    /// font will always be in the range `0..self.glyph_count()`
    #[inline]
    fn glyph_count(&self) -> usize {
        self.font().glyph_count()
    }

    /// Returns an iterator of all distinct `(GlyphId, char)` pairs. Not ordered.
    ///
    /// Same as [`Font::codepoint_ids`](trait.Font.html#tymethod.codepoint_ids).
    fn codepoint_ids(&self) -> crate::CodepointIdIter<'_>;

    /// Compute glyph outline ready for drawing.
    ///
    /// Note this method does not make use of the associated scale, as `Glyph`
    /// already includes one of it's own.
    #[inline]
    fn outline_glyph(&self, glyph: Glyph) -> Option<OutlinedGlyph> {
        self.font().outline_glyph(glyph)
    }
}

impl<F: Font, SF: ScaleFont<F>> ScaleFont<F> for &SF {
    #[inline]
    fn scale(&self) -> PxScale {
        (*self).scale()
    }

    #[inline]
    fn font(&self) -> &F {
        (*self).font()
    }

    #[inline]
    fn codepoint_ids(&self) -> crate::CodepointIdIter<'_> {
        (*self).codepoint_ids()
    }
}

/// A [`Font`](trait.Font.html) and an associated pixel scale.
#[derive(Clone, Copy, Debug)]
pub struct PxScaleFont<F> {
    pub font: F,
    pub scale: PxScale,
}

impl<F> PxScaleFont<F> {
    #[inline]
    pub fn with_scale<S: Into<PxScale>>(mut self, scale: S) -> Self {
        self.scale = scale.into();
        self
    }
}

impl<F: Font> ScaleFont<F> for PxScaleFont<F> {
    #[inline]
    fn scale(&self) -> PxScale {
        self.scale
    }

    #[inline]
    fn font(&self) -> &F {
        &self.font
    }

    #[inline]
    fn codepoint_ids(&self) -> crate::CodepointIdIter<'_> {
        self.font.codepoint_ids()
    }
}
