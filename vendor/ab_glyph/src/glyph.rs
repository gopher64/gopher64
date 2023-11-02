use crate::{Point, PxScale};

/// Glyph id.
///
/// # Example
/// ```
/// use ab_glyph::{Font, FontRef, GlyphId};
/// # fn main() -> Result<(), ab_glyph::InvalidFont> {
/// let font = FontRef::try_from_slice(include_bytes!("../../dev/fonts/Exo2-Light.otf"))?;
///
/// let q_id: GlyphId = font.glyph_id('q');
/// # Ok(()) }
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GlyphId(pub u16);

impl GlyphId {
    /// Construct a `Glyph` with given scale & position.
    ///
    /// # Example
    /// ```
    /// # use ab_glyph::*;
    /// # let font = FontRef::try_from_slice(include_bytes!("../../dev/fonts/Exo2-Light.otf")).unwrap();
    /// let glyph = font.glyph_id('z').with_scale_and_position(24.0, point(100.0, 0.0));
    /// ```
    #[inline]
    pub fn with_scale_and_position<S: Into<PxScale>, P: Into<Point>>(
        self,
        scale: S,
        position: P,
    ) -> Glyph {
        Glyph {
            id: self,
            scale: scale.into(),
            position: position.into(),
        }
    }

    /// Construct a `Glyph` with given scale and position `point(0.0, 0.0)`.
    ///
    /// # Example
    /// ```
    /// # use ab_glyph::*;
    /// # let font = FontRef::try_from_slice(include_bytes!("../../dev/fonts/Exo2-Light.otf")).unwrap();
    /// let glyph = font.glyph_id('w').with_scale(48.0);
    /// ```
    #[inline]
    pub fn with_scale<S: Into<PxScale>>(self, scale: S) -> Glyph {
        self.with_scale_and_position(scale, Point::default())
    }
}

/// A glyph with pixel scale & position.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub struct Glyph {
    /// Glyph id.
    pub id: GlyphId,
    /// Pixel scale of this glyph.
    pub scale: PxScale,
    /// Position of this glyph.
    pub position: Point,
}

/// Old version of [`v2::GlyphImage`].
#[deprecated(since = "0.2.22", note = "Deprecated in favor of `v2::GlyphImage`")]
#[derive(Debug, Clone)]
pub struct GlyphImage<'a> {
    /// Offset of the image from the normal origin (top at the baseline plus
    /// ascent), measured in pixels at the image's current scale.
    pub origin: Point,
    /// Current scale of the image in pixels per em.
    pub scale: f32,
    /// Raw image data, not a bitmap in the case of [`GlyphImageFormat::Png`] format.
    pub data: &'a [u8],
    /// Format of the raw data.
    pub format: GlyphImageFormat,
}

pub mod v2 {
    use crate::{GlyphImageFormat, Point};

    /// A pre-rendered image of a glyph, usually used for emojis or other glyphs
    /// that can't be represented only using an outline.
    #[non_exhaustive]
    #[derive(Debug, Clone)]
    pub struct GlyphImage<'a> {
        /// Offset of the image from the normal origin (top at the baseline plus
        /// ascent), measured in pixels at the image's current scale.
        pub origin: Point,
        /// Image width.
        ///
        /// It doesn't guarantee that this value is the same as set in the `data` in the case of
        /// [`GlyphImageFormat::Png`] format.
        pub width: u16,
        /// Image height.
        ///
        /// It doesn't guarantee that this value is the same as set in the `data` in the case of
        /// [`GlyphImageFormat::Png`] format.
        pub height: u16,
        /// Pixels per em of the selected strike.
        pub pixels_per_em: u16,
        /// Raw image data, see [`format`](GlyphImageFormat).
        pub data: &'a [u8],
        /// Format of the raw [`data`](Self::data).
        pub format: GlyphImageFormat,
    }
}

/// Valid formats for a [`GlyphImage`].
// Possible future formats: SVG, JPEG, TIFF
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum GlyphImageFormat {
    Png,

    /// A monochrome bitmap.
    ///
    /// The most significant bit of the first byte corresponds to the top-left pixel, proceeding
    /// through succeeding bits moving left to right. The data for each row is padded to a byte
    /// boundary, so the next row begins with the most significant bit of a new byte. 1 corresponds
    /// to black, and 0 to white.
    BitmapMono,

    /// A packed monochrome bitmap.
    ///
    /// The most significant bit of the first byte corresponds to the top-left pixel, proceeding
    /// through succeeding bits moving left to right. Data is tightly packed with no padding. 1
    /// corresponds to black, and 0 to white.
    BitmapMonoPacked,

    /// A grayscale bitmap with 2 bits per pixel.
    ///
    /// The most significant bits of the first byte corresponds to the top-left pixel, proceeding
    /// through succeeding bits moving left to right. The data for each row is padded to a byte
    /// boundary, so the next row begins with the most significant bit of a new byte.
    BitmapGray2,

    /// A packed grayscale bitmap with 2 bits per pixel.
    ///
    /// The most significant bits of the first byte corresponds to the top-left pixel, proceeding
    /// through succeeding bits moving left to right. Data is tightly packed with no padding.
    BitmapGray2Packed,

    /// A grayscale bitmap with 4 bits per pixel.
    ///
    /// The most significant bits of the first byte corresponds to the top-left pixel, proceeding
    /// through succeeding bits moving left to right. The data for each row is padded to a byte
    /// boundary, so the next row begins with the most significant bit of a new byte.
    BitmapGray4,

    /// A packed grayscale bitmap with 4 bits per pixel.
    ///
    /// The most significant bits of the first byte corresponds to the top-left pixel, proceeding
    /// through succeeding bits moving left to right. Data is tightly packed with no padding.
    BitmapGray4Packed,

    /// A grayscale bitmap with 8 bits per pixel.
    ///
    /// The first byte corresponds to the top-left pixel, proceeding through succeeding bytes
    /// moving left to right.
    BitmapGray8,

    /// A color bitmap with 32 bits per pixel.
    ///
    /// The first group of four bytes corresponds to the top-left pixel, proceeding through
    /// succeeding pixels moving left to right. Each byte corresponds to a color channel and the
    /// channels within a pixel are in blue, green, red, alpha order. Color values are
    /// pre-multiplied by the alpha. For example, the color "full-green with half translucency"
    /// is encoded as `\x00\x80\x00\x80`, and not `\x00\xFF\x00\x80`.
    BitmapPremulBgra32,
}
