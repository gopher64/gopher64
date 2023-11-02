//! A [Font Header Table](
//! https://docs.microsoft.com/en-us/typography/opentype/spec/head) implementation.

use crate::Rect;
use crate::parser::{Stream, Fixed};

/// An index format used by the [Index to Location Table](
/// https://docs.microsoft.com/en-us/typography/opentype/spec/loca).
#[allow(missing_docs)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum IndexToLocationFormat {
    Short,
    Long,
}


/// A [Font Header Table](https://docs.microsoft.com/en-us/typography/opentype/spec/head).
#[derive(Clone, Copy, Debug)]
pub struct Table {
    /// Units per EM.
    ///
    /// Guarantee to be in a 16..=16384 range.
    pub units_per_em: u16,
    /// A bounding box that large enough to enclose any glyph from the face.
    pub global_bbox: Rect,
    /// An index format used by the [Index to Location Table](
    /// https://docs.microsoft.com/en-us/typography/opentype/spec/loca).
    pub index_to_location_format: IndexToLocationFormat,
}

impl Table {
    /// Parses a table from raw data.
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() != 54 {
            return None
        }

        let mut s = Stream::new(data);
        s.skip::<u32>(); // version
        s.skip::<Fixed>(); // font revision
        s.skip::<u32>(); // checksum adjustment
        s.skip::<u32>(); // magic number
        s.skip::<u16>(); // flags
        let units_per_em = s.read::<u16>()?;
        s.skip::<u64>(); // created time
        s.skip::<u64>(); // modified time
        let x_min = s.read::<i16>()?;
        let y_min = s.read::<i16>()?;
        let x_max = s.read::<i16>()?;
        let y_max = s.read::<i16>()?;
        s.skip::<u16>(); // mac style
        s.skip::<u16>(); // lowest PPEM
        s.skip::<i16>(); // font direction hint
        let index_to_location_format = s.read::<u16>()?;

        if !(units_per_em >= 16 && units_per_em <= 16384) {
            return None;
        }

        let index_to_location_format = match index_to_location_format {
            0 => IndexToLocationFormat::Short,
            1 => IndexToLocationFormat::Long,
            _ => return None,
        };

        Some(Table {
            units_per_em,
            global_bbox: Rect { x_min, y_min, x_max, y_max },
            index_to_location_format,
        })
    }
}
