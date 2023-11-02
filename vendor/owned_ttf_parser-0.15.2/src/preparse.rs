//! Logic to avoid re-parsing subtables in ttf_parser::Face methods
use crate::{AsFaceRef, FaceMut, OwnedFace};
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
use core::fmt;
use ttf_parser::{cmap, kern, Face, GlyphId};

/// A `Face` with cmap & kern subtables parsed once on initialization.
///
/// Provides much faster [`PreParsedSubtables::glyph_index`] &
/// [`PreParsedSubtables::glyphs_hor_kerning`] methods compared to the
/// `.as_face_ref()` equivalents that must parse their subtables on each call.
///
/// # Example
/// ```
/// use owned_ttf_parser::{AsFaceRef, GlyphId, OwnedFace, PreParsedSubtables};
///
/// # let owned_font_data = include_bytes!("../fonts/font.ttf").to_vec();
/// let owned_face = OwnedFace::from_vec(owned_font_data, 0).unwrap();
/// let faster_face = PreParsedSubtables::from(owned_face);
///
/// // Lookup a GlyphId using the pre-parsed cmap subtables
/// // this is much faster than doing: .as_face_ref().glyph_index('x')
/// assert_eq!(faster_face.glyph_index('x'), Some(GlyphId(91)));
///
/// // The rest of the methods are still available as normal
/// assert_eq!(faster_face.as_face_ref().ascender(), 2254);
/// ```
#[derive(Clone)]
pub struct PreParsedSubtables<'face, F> {
    pub(crate) face: F,
    // note must not be public as could be self-referencing
    pub(crate) subtables: FaceSubtables<'face>,
}

impl<'face> From<Face<'face>> for PreParsedSubtables<'face, Face<'face>> {
    fn from(face: Face<'face>) -> Self {
        let subtables = FaceSubtables::from(&face);
        Self { face, subtables }
    }
}

impl From<OwnedFace> for PreParsedSubtables<'static, OwnedFace> {
    fn from(face: OwnedFace) -> Self {
        face.pre_parse_subtables()
    }
}

impl<F> fmt::Debug for PreParsedSubtables<'_, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PreParsedSubtables")
    }
}

#[derive(Clone)]
pub(crate) struct FaceSubtables<'face> {
    /// Unicode cmap subtables.
    cmap: Vec<cmap::Subtable<'face>>,
    /// Horizontal kern subtables.
    h_kern: Vec<kern::Subtable<'face>>,
}

impl<'face> From<&Face<'face>> for FaceSubtables<'face> {
    fn from(face: &Face<'face>) -> Self {
        let cmap = face
            .tables()
            .cmap
            .iter()
            .flat_map(|cmap| cmap.subtables)
            .filter(|st| st.is_unicode())
            .collect();
        let h_kern = face
            .tables()
            .kern
            .iter()
            .flat_map(|c| c.subtables)
            .filter(|st| st.horizontal && !st.variable)
            .collect();
        Self { cmap, h_kern }
    }
}

impl<F> PreParsedSubtables<'_, F> {
    /// Maps a character to a `GlyphId` using pre-parsed unicode cmap subtables.
    #[inline]
    pub fn glyph_index(&self, c: char) -> Option<GlyphId> {
        self.subtables
            .cmap
            .iter()
            .find_map(|t| t.glyph_index(c.into()))
    }

    /// Returns horizontal kerning for a pair of glyphs using pre-parsed kern subtables.
    #[inline]
    pub fn glyphs_hor_kerning(&self, first: GlyphId, second: GlyphId) -> Option<i16> {
        self.subtables
            .h_kern
            .iter()
            .find_map(|st| st.glyphs_kerning(first, second))
    }
}

impl<F> AsFaceRef for PreParsedSubtables<'_, F>
where
    F: AsFaceRef,
{
    #[inline]
    fn as_face_ref(&self) -> &ttf_parser::Face<'_> {
        self.face.as_face_ref()
    }
}
impl<F> AsFaceRef for &PreParsedSubtables<'_, F>
where
    F: AsFaceRef,
{
    #[inline]
    fn as_face_ref(&self) -> &ttf_parser::Face<'_> {
        (*self).as_face_ref()
    }
}

impl<F> FaceMut for PreParsedSubtables<'_, F>
where
    F: FaceMut,
{
    #[inline]
    fn set_variation(&mut self, axis: ttf_parser::Tag, value: f32) -> Option<()> {
        self.face.set_variation(axis, value)
    }
}
impl<F> FaceMut for &mut PreParsedSubtables<'_, F>
where
    F: FaceMut,
{
    #[inline]
    fn set_variation(&mut self, axis: ttf_parser::Tag, value: f32) -> Option<()> {
        (*self).set_variation(axis, value)
    }
}
