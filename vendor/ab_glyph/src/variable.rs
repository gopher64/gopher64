#[cfg(not(feature = "std"))]
use alloc::string::String;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

/// Logic for variable fonts.
///
/// Requires feature `variable-fonts` (enabled by default).
pub trait VariableFont {
    /// Sets a variation axis coordinate value by it's tag.
    ///
    /// Returns false if there is no such axis tag.
    ///
    /// # Example
    /// ```
    /// use ab_glyph::{FontRef, VariableFont};
    ///
    /// # fn main() -> Result<(), ab_glyph::InvalidFont> {
    /// let mut font = FontRef::try_from_slice(include_bytes!("../../dev/fonts/Cantarell-VF.otf"))?;
    ///
    /// // set weight to 600
    /// assert!(font.set_variation(b"wght", 600.0));
    ///
    /// // no such variation tag "foob" so return false
    /// assert!(!font.set_variation(b"foob", 200.0));
    /// # Ok(()) }
    /// ```
    fn set_variation(&mut self, tag: &[u8; 4], value: f32) -> bool;

    /// Returns variation axes.
    ///
    /// # Example
    /// ```
    /// use ab_glyph::{FontRef, VariableFont};
    ///
    /// # fn main() -> Result<(), ab_glyph::InvalidFont> {
    /// let font = FontRef::try_from_slice(include_bytes!("../../dev/fonts/Cantarell-VF.otf"))?;
    /// let var = &font.variations()[0];
    /// # eprintln!("{var:#?}");
    ///
    /// assert_eq!(var.tag, *b"wght");
    /// assert_eq!(var.name.as_deref(), Some("Weight"));
    /// assert!((var.min_value - 100.0).abs() < f32::EPSILON);
    /// assert!((var.default_value - 400.0).abs() < f32::EPSILON);
    /// assert!((var.max_value - 800.0).abs() < f32::EPSILON);
    /// assert!(!var.hidden);
    /// # Ok(()) }
    /// ```
    fn variations(&self) -> Vec<VariationAxis>;
}

#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct VariationAxis {
    /// Tag identifying the design variation for the axis.
    pub tag: [u8; 4],
    /// Unicode name.
    pub name: Option<String>,
    /// The minimum coordinate value for the axis.
    pub min_value: f32,
    /// The default coordinate value for the axis.
    pub default_value: f32,
    /// The maximum coordinate value for the axis.
    pub max_value: f32,
    /// Whether the axis should be exposed directly in user interfaces.
    pub hidden: bool,
}
