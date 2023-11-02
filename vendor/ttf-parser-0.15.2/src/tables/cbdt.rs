//! A [Color Bitmap Data Table](
//! https://docs.microsoft.com/en-us/typography/opentype/spec/cbdt) implementation.

use crate::{GlyphId, RasterGlyphImage, RasterImageFormat};
use crate::parser::{Stream, NumFrom};
use super::cblc::{self, BitmapFormat};

/// A [Color Bitmap Data Table](
/// https://docs.microsoft.com/en-us/typography/opentype/spec/cbdt).
#[derive(Clone, Copy)]
pub struct Table<'a> {
    locations: cblc::Table<'a>,
    data: &'a [u8],
}

impl<'a> Table<'a> {
    /// Parses a table from raw data.
    pub fn parse(locations: cblc::Table<'a>, data: &'a [u8]) -> Option<Self> {
        Some(Self { locations, data })
    }

    /// Returns a raster image for the glyph.
    pub fn get(&self, glyph_id: GlyphId, pixels_per_em: u16) -> Option<RasterGlyphImage<'a>> {
        let location = self.locations.get(glyph_id, pixels_per_em)?;
        let mut s = Stream::new_at(self.data, location.offset)?;
        match location.format {
            BitmapFormat::Format17 => {
                let height = s.read::<u8>()?;
                let width = s.read::<u8>()?;
                let bearing_x = s.read::<i8>()?;
                let bearing_y = s.read::<i8>()?;
                s.skip::<u8>(); // advance
                let data_len = s.read::<u32>()?;
                let data = s.read_bytes(usize::num_from(data_len))?;
                Some(RasterGlyphImage {
                    x: i16::from(bearing_x),
                    // `y` in CBDT is a bottom bound, not top one.
                    y: i16::from(bearing_y) - i16::from(height),
                    width: u16::from(width),
                    height: u16::from(height),
                    pixels_per_em: location.ppem,
                    format: RasterImageFormat::PNG,
                    data,
                })
            }
            BitmapFormat::Format18 => {
                let height = s.read::<u8>()?;
                let width = s.read::<u8>()?;
                let hor_bearing_x = s.read::<i8>()?;
                let hor_bearing_y = s.read::<i8>()?;
                s.skip::<u8>(); // hor_advance
                s.skip::<i8>(); // ver_bearing_x
                s.skip::<i8>(); // ver_bearing_y
                s.skip::<u8>(); // ver_advance
                let data_len = s.read::<u32>()?;
                let data = s.read_bytes(usize::num_from(data_len))?;
                Some(RasterGlyphImage {
                    x: i16::from(hor_bearing_x),
                    // `y` in CBDT is a bottom bound, not top one.
                    y: i16::from(hor_bearing_y) - i16::from(height),
                    width: u16::from(width),
                    height: u16::from(height),
                    pixels_per_em: location.ppem,
                    format: RasterImageFormat::PNG,
                    data,
                })
            }
            BitmapFormat::Format19 => {
                let data_len = s.read::<u32>()?;
                let data = s.read_bytes(usize::num_from(data_len))?;
                Some(RasterGlyphImage {
                    x: i16::from(location.metrics.x),
                    // `y` in CBDT is a bottom bound, not top one.
                    y: i16::from(location.metrics.y) - i16::from(location.metrics.height),
                    width: u16::from(location.metrics.width),
                    height: u16::from(location.metrics.height),
                    pixels_per_em: location.ppem,
                    format: RasterImageFormat::PNG,
                    data,
                })
            }
        }
    }
}

impl core::fmt::Debug for Table<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "Table {{ ... }}")
    }
}
