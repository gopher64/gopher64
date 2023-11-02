use crate::{v2, Font, FontRef, FontVec, GlyphId, InvalidFont, Outline};
use alloc::sync::Arc;
use core::fmt;

/// `Font` implementor that wraps another concrete `Font + 'static` type storing in an `Arc`.
///
/// Provides convenient type erasure & cheap clones (particularly for `FontVec`).
///
/// # Example
/// ```
/// use ab_glyph::{Font, FontArc};
///
/// # fn main() -> Result<(), ab_glyph::InvalidFont> {
/// let font = FontArc::try_from_slice(include_bytes!("../../dev/fonts/Exo2-Light.otf"))?;
///
/// assert_eq!(font.glyph_id('s'), ab_glyph::GlyphId(56));
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct FontArc(Arc<dyn Font + Send + Sync + 'static>);

impl FontArc {
    /// # Example
    /// ```
    /// # use ab_glyph::*;
    /// # fn main() -> Result<(), ab_glyph::InvalidFont> {
    /// # let font_data = include_bytes!("../../dev/fonts/Exo2-Light.otf").to_vec();
    /// # let font_vec = FontVec::try_from_vec(font_data)?;
    /// let font_arc = FontArc::new(font_vec);
    /// # Ok(()) }
    /// ```
    #[inline]
    pub fn new<F: Font + Send + Sync + 'static>(font: F) -> Self {
        Self(Arc::new(font))
    }

    /// Creates an `FontArc` from owned data.
    ///
    /// # Example
    /// ```
    /// # use ab_glyph::*;
    /// # fn main() -> Result<(), InvalidFont> {
    /// # let owned_font_data = include_bytes!("../../dev/fonts/Exo2-Light.otf").to_vec();
    /// let font = FontArc::try_from_vec(owned_font_data)?;
    /// # Ok(()) }
    /// ```
    #[inline]
    pub fn try_from_vec(data: Vec<u8>) -> Result<Self, InvalidFont> {
        Ok(FontVec::try_from_vec(data)?.into())
    }

    /// Creates an `FontArc` from a byte-slice.
    ///
    /// # Example
    /// ```
    /// # use ab_glyph::*;
    /// # fn main() -> Result<(), InvalidFont> {
    /// let font = FontArc::try_from_slice(include_bytes!("../../dev/fonts/Exo2-Light.otf"))?;
    /// # Ok(()) }
    /// ```
    #[inline]
    pub fn try_from_slice(data: &'static [u8]) -> Result<Self, InvalidFont> {
        Ok(FontRef::try_from_slice(data)?.into())
    }
}

impl fmt::Debug for FontArc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FontArc")
    }
}

impl Font for FontArc {
    #[inline]
    fn units_per_em(&self) -> Option<f32> {
        self.0.units_per_em()
    }

    #[inline]
    fn ascent_unscaled(&self) -> f32 {
        self.0.ascent_unscaled()
    }

    #[inline]
    fn descent_unscaled(&self) -> f32 {
        self.0.descent_unscaled()
    }

    #[inline]
    fn line_gap_unscaled(&self) -> f32 {
        self.0.line_gap_unscaled()
    }

    #[inline]
    fn glyph_id(&self, c: char) -> GlyphId {
        self.0.glyph_id(c)
    }

    #[inline]
    fn h_advance_unscaled(&self, id: GlyphId) -> f32 {
        self.0.h_advance_unscaled(id)
    }

    #[inline]
    fn h_side_bearing_unscaled(&self, id: GlyphId) -> f32 {
        self.0.h_side_bearing_unscaled(id)
    }

    #[inline]
    fn v_advance_unscaled(&self, id: GlyphId) -> f32 {
        self.0.v_advance_unscaled(id)
    }

    #[inline]
    fn v_side_bearing_unscaled(&self, id: GlyphId) -> f32 {
        self.0.v_side_bearing_unscaled(id)
    }

    #[inline]
    fn kern_unscaled(&self, first: GlyphId, second: GlyphId) -> f32 {
        self.0.kern_unscaled(first, second)
    }

    #[inline]
    fn outline(&self, glyph: GlyphId) -> Option<Outline> {
        self.0.outline(glyph)
    }

    #[inline]
    fn glyph_count(&self) -> usize {
        self.0.glyph_count()
    }

    #[inline]
    fn codepoint_ids(&self) -> crate::CodepointIdIter<'_> {
        self.0.codepoint_ids()
    }

    #[inline]
    fn glyph_raster_image2(&self, id: GlyphId, size: u16) -> Option<v2::GlyphImage> {
        self.0.glyph_raster_image2(id, size)
    }
}

impl From<FontVec> for FontArc {
    #[inline]
    fn from(font: FontVec) -> Self {
        Self::new(font)
    }
}
impl From<FontRef<'static>> for FontArc {
    #[inline]
    fn from(font: FontRef<'static>) -> Self {
        Self::new(font)
    }
}
impl From<Arc<dyn Font + Send + Sync + 'static>> for FontArc {
    #[inline]
    fn from(font: Arc<dyn Font + Send + Sync + 'static>) -> Self {
        Self(font)
    }
}
