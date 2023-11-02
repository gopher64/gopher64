use crate::{
    point, v2, Glyph, GlyphId, Outline, OutlinedGlyph, PxScale, PxScaleFont, Rect, ScaleFont,
};

/// Functionality required from font data.
///
/// See also [`FontArc`](struct.FontArc.html), [`FontRef`](struct.FontRef.html)
/// and [`FontVec`](struct.FontVec.html).
///
/// ## Units
///
/// Units of unscaled accessors are "font units", which is an arbitrary unit
/// defined by the font. See [`Font::units_per_em`].
///
/// ab_glyph uses a non-standard scale [`PxScale`] which is the pixel height
/// of the text. See [`Font::pt_to_px_scale`] to convert standard point sizes.
pub trait Font {
    /// Get the size of the font unit
    ///
    /// This returns "font units per em", where 1em is a base unit of font scale
    /// (typically the width of a capital 'M').
    ///
    /// Returns `None` in case the font unit size exceeds the expected range.
    /// See [`Face::units_per_em`](https://docs.rs/ttf-parser/latest/ttf_parser/struct.Face.html#method.units_per_em).
    ///
    /// May be used to calculate [`PxScale`] from pt size, see [`Font::pt_to_px_scale`].
    fn units_per_em(&self) -> Option<f32>;

    /// Converts pt units into [`PxScale`].
    ///
    /// Note: To handle a screen scale factor multiply it to the `pt_size` argument.
    ///
    /// Returns `None` in case the [`Font::units_per_em`] unit size exceeds the expected range.
    ///
    /// ## Point size (pt)
    ///
    /// Font sizes are typically specified in "points". According to the modern
    /// standard, 1pt = 1/72in. The "point size" of a font is the number of points
    /// per em.
    ///
    /// The DPI (dots-per-inch) of a screen depends on the screen in question;
    /// 96 DPI is often considered the "standard". For high-DPI displays the
    /// DPI may be specified directly or one may multiply 96 by a scale-factor.
    ///
    /// Thus, for example, a 10pt font on a 96 pixels-per-inch display has
    /// 10 / 72 * 96 = 13.333... pixels-per-em. If we divide this number by
    /// `units_per_em` we then get a scaling factor: pixels-per-font-unit.
    ///
    /// Note however that since [`PxScale`] values are relative to the text height,
    /// one further step is needed: multiply by [`Font::height_unscaled`].
    fn pt_to_px_scale(&self, pt_size: f32) -> Option<PxScale> {
        let px_per_em = pt_size * (96.0 / 72.0);
        let units_per_em = self.units_per_em()?;
        let height = self.height_unscaled();
        Some(PxScale::from(px_per_em * height / units_per_em))
    }

    /// Unscaled glyph ascent.
    ///
    /// Scaling can be done with [as_scaled](trait.Font.html#method.as_scaled).
    fn ascent_unscaled(&self) -> f32;

    /// Unscaled glyph descent.
    ///
    /// Scaling can be done with [as_scaled](trait.Font.html#method.as_scaled).
    fn descent_unscaled(&self) -> f32;

    /// Unscaled height `ascent - descent`.
    ///
    /// Scaling can be done with [as_scaled](trait.Font.html#method.as_scaled).
    #[inline]
    fn height_unscaled(&self) -> f32 {
        self.ascent_unscaled() - self.descent_unscaled()
    }

    /// Unscaled line gap.
    ///
    /// Scaling can be done with [as_scaled](trait.Font.html#method.as_scaled).
    fn line_gap_unscaled(&self) -> f32;

    /// Lookup a `GlyphId` matching a given `char`.
    ///
    /// Scaling can be done with [as_scaled](trait.Font.html#method.as_scaled).
    fn glyph_id(&self, c: char) -> GlyphId;

    /// Unscaled horizontal advance for a given glyph id.
    ///
    /// Returns `0.0` if the font does not define this value.
    ///
    /// Scaling can be done with [as_scaled](trait.Font.html#method.as_scaled).
    fn h_advance_unscaled(&self, id: GlyphId) -> f32;

    /// Unscaled horizontal side bearing for a given glyph id.
    ///
    /// Returns `0.0` if the font does not define this value.
    ///
    /// Scaling can be done with [as_scaled](trait.Font.html#method.as_scaled).
    fn h_side_bearing_unscaled(&self, id: GlyphId) -> f32;

    /// Unscaled vertical advance for a given glyph id.
    ///
    /// Returns `0.0` if the font does not define this value.
    ///
    /// Scaling can be done with [as_scaled](trait.Font.html#method.as_scaled).
    fn v_advance_unscaled(&self, id: GlyphId) -> f32;

    /// Unscaled vertical side bearing for a given glyph id.
    ///
    /// Returns `0.0` if the font does not define this value.
    ///
    /// Scaling can be done with [as_scaled](trait.Font.html#method.as_scaled).
    fn v_side_bearing_unscaled(&self, id: GlyphId) -> f32;

    /// Returns additional unscaled kerning to apply for a particular pair of glyph ids.
    ///
    /// Scaling can be done with [as_scaled](trait.Font.html#method.as_scaled).
    fn kern_unscaled(&self, first: GlyphId, second: GlyphId) -> f32;

    /// Compute unscaled glyph outline curves & bounding box.
    fn outline(&self, id: GlyphId) -> Option<Outline>;

    /// The number of glyphs present in this font. Glyph identifiers for this
    /// font will always be in the range `0..self.glyph_count()`
    fn glyph_count(&self) -> usize;

    /// Returns an iterator of all distinct `(GlyphId, char)` pairs. Not ordered.
    ///
    /// # Example
    /// ```
    /// # use ab_glyph::{Font, FontRef, GlyphId};
    /// # use std::collections::HashMap;
    /// # fn main() -> Result<(), ab_glyph::InvalidFont> {
    /// let font = FontRef::try_from_slice(include_bytes!("../../dev/fonts/Exo2-Light.otf"))?;
    ///
    /// // Iterate over pairs, each id will appear at most once.
    /// let mut codepoint_ids = font.codepoint_ids();
    /// assert_eq!(codepoint_ids.next(), Some((GlyphId(408), '\r')));
    /// assert_eq!(codepoint_ids.next(), Some((GlyphId(1), ' ')));
    /// assert_eq!(codepoint_ids.next(), Some((GlyphId(75), '!')));
    ///
    /// // Build a lookup map for all ids
    /// let map: HashMap<_, _> = font.codepoint_ids().collect();
    /// assert_eq!(map.get(&GlyphId(75)), Some(&'!'));
    /// # assert_eq!(map.len(), 908);
    /// # Ok(()) }
    /// ```
    fn codepoint_ids(&self) -> crate::CodepointIdIter<'_>;

    /// Returns a pre-rendered image of the glyph.
    ///
    /// This is normally only present when an outline is not sufficient to describe the glyph, such
    /// as emojis (particularly color ones).  The `pixel_size` parameter is in pixels per em, and will be
    /// used to select between multiple possible images (if present); the returned image will
    /// likely not match this value, requiring you to scale it to match the target resolution.
    /// To get the largest image use `u16::MAX`.
    #[allow(deprecated)]
    #[deprecated(
        since = "0.2.22",
        note = "Deprecated in favor of `glyph_raster_image2`"
    )]
    fn glyph_raster_image(&self, id: GlyphId, pixel_size: u16) -> Option<crate::GlyphImage> {
        self.glyph_raster_image2(id, pixel_size)
            .map(|i| crate::GlyphImage {
                origin: i.origin,
                scale: i.pixels_per_em.into(),
                data: i.data,
                format: i.format,
            })
    }

    /// Returns a pre-rendered image of the glyph.
    ///
    /// This is normally only present when an outline is not sufficient to describe the glyph, such
    /// as emojis (particularly color ones).  The `pixel_size` parameter is in pixels per em, and will be
    /// used to select between multiple possible images (if present); the returned image will
    /// likely not match this value, requiring you to scale it to match the target resolution.
    /// To get the largest image use `u16::MAX`.
    fn glyph_raster_image2(&self, id: GlyphId, pixel_size: u16) -> Option<v2::GlyphImage>;

    /// Returns the layout bounds of this glyph. These are different to the outline `px_bounds()`.
    ///
    /// Horizontally: Glyph position +/- h_advance/h_side_bearing.
    /// Vertically: Glyph position +/- ascent/descent.
    #[inline]
    fn glyph_bounds(&self, glyph: &Glyph) -> Rect
    where
        Self: Sized,
    {
        let sf = self.as_scaled(glyph.scale);
        let pos = glyph.position;
        Rect {
            min: point(pos.x - sf.h_side_bearing(glyph.id), pos.y - sf.ascent()),
            max: point(pos.x + sf.h_advance(glyph.id), pos.y - sf.descent()),
        }
    }

    /// Compute glyph outline ready for drawing.
    #[inline]
    fn outline_glyph(&self, glyph: Glyph) -> Option<OutlinedGlyph>
    where
        Self: Sized,
    {
        let outline = self.outline(glyph.id)?;
        let scale_factor = self.as_scaled(glyph.scale).scale_factor();
        Some(OutlinedGlyph::new(glyph, outline, scale_factor))
    }

    /// Construct a [`PxScaleFontRef`](struct.PxScaleFontRef.html) by associating with the
    /// given pixel `scale`.
    ///
    /// # Example
    /// ```
    /// # use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
    /// # fn main() -> Result<(), ab_glyph::InvalidFont> {
    /// let font = FontRef::try_from_slice(include_bytes!("../../dev/fonts/Exo2-Light.otf"))?;
    ///
    /// assert_eq!(font.descent_unscaled(), -201.0);
    ///
    /// assert_eq!(font.as_scaled(24.0).descent(), -4.02);
    /// assert_eq!(font.as_scaled(50.0).descent(), -8.375);
    /// # Ok(()) }
    /// ```
    #[inline]
    fn as_scaled<S: Into<PxScale>>(&self, scale: S) -> PxScaleFont<&'_ Self>
    where
        Self: Sized,
    {
        PxScaleFont {
            font: self,
            scale: scale.into(),
        }
    }

    /// Move into a [`PxScaleFont`](struct.PxScaleFont.html) associated with the
    /// given pixel `scale`.
    #[inline]
    fn into_scaled<S: Into<PxScale>>(self, scale: S) -> PxScaleFont<Self>
    where
        Self: core::marker::Sized,
    {
        PxScaleFont {
            font: self,
            scale: scale.into(),
        }
    }
}

impl<F: Font> Font for &F {
    #[inline]
    fn units_per_em(&self) -> Option<f32> {
        (*self).units_per_em()
    }

    #[inline]
    fn ascent_unscaled(&self) -> f32 {
        (*self).ascent_unscaled()
    }

    #[inline]
    fn descent_unscaled(&self) -> f32 {
        (*self).descent_unscaled()
    }

    #[inline]
    fn line_gap_unscaled(&self) -> f32 {
        (*self).line_gap_unscaled()
    }

    #[inline]
    fn glyph_id(&self, c: char) -> GlyphId {
        (*self).glyph_id(c)
    }

    #[inline]
    fn h_advance_unscaled(&self, id: GlyphId) -> f32 {
        (*self).h_advance_unscaled(id)
    }

    #[inline]
    fn h_side_bearing_unscaled(&self, id: GlyphId) -> f32 {
        (*self).h_side_bearing_unscaled(id)
    }

    #[inline]
    fn v_advance_unscaled(&self, id: GlyphId) -> f32 {
        (*self).v_advance_unscaled(id)
    }

    #[inline]
    fn v_side_bearing_unscaled(&self, id: GlyphId) -> f32 {
        (*self).v_side_bearing_unscaled(id)
    }

    #[inline]
    fn kern_unscaled(&self, first: GlyphId, second: GlyphId) -> f32 {
        (*self).kern_unscaled(first, second)
    }

    #[inline]
    fn outline(&self, glyph: GlyphId) -> Option<Outline> {
        (*self).outline(glyph)
    }

    #[inline]
    fn glyph_count(&self) -> usize {
        (*self).glyph_count()
    }

    #[inline]
    fn codepoint_ids(&self) -> crate::CodepointIdIter<'_> {
        (*self).codepoint_ids()
    }

    #[inline]
    fn glyph_raster_image2(&self, id: GlyphId, size: u16) -> Option<v2::GlyphImage> {
        (*self).glyph_raster_image2(id, size)
    }
}
