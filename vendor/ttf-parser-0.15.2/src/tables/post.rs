//! A [PostScript Table](
//! https://docs.microsoft.com/en-us/typography/opentype/spec/post) implementation.

use crate::LineMetrics;
use crate::parser::{Stream, Fixed, LazyArray16};
#[cfg(feature = "glyph-names")] use crate::GlyphId;

const TABLE_SIZE: usize = 32;
const ITALIC_ANGLE_OFFSET: usize = 4;
const UNDERLINE_POSITION_OFFSET: usize = 8;
const UNDERLINE_THICKNESS_OFFSET: usize = 10;
const IS_FIXED_PITCH_OFFSET: usize = 12;

// https://developer.apple.com/fonts/TrueType-Reference-Manual/RM06/Chap6post.html
/// A list of Macintosh glyph names.
#[cfg(feature = "glyph-names")]
const MACINTOSH_NAMES: &[&str] = &[
    ".notdef",
    ".null",
    "nonmarkingreturn",
    "space",
    "exclam",
    "quotedbl",
    "numbersign",
    "dollar",
    "percent",
    "ampersand",
    "quotesingle",
    "parenleft",
    "parenright",
    "asterisk",
    "plus",
    "comma",
    "hyphen",
    "period",
    "slash",
    "zero",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "colon",
    "semicolon",
    "less",
    "equal",
    "greater",
    "question",
    "at",
    "A",
    "B",
    "C",
    "D",
    "E",
    "F",
    "G",
    "H",
    "I",
    "J",
    "K",
    "L",
    "M",
    "N",
    "O",
    "P",
    "Q",
    "R",
    "S",
    "T",
    "U",
    "V",
    "W",
    "X",
    "Y",
    "Z",
    "bracketleft",
    "backslash",
    "bracketright",
    "asciicircum",
    "underscore",
    "grave",
    "a",
    "b",
    "c",
    "d",
    "e",
    "f",
    "g",
    "h",
    "i",
    "j",
    "k",
    "l",
    "m",
    "n",
    "o",
    "p",
    "q",
    "r",
    "s",
    "t",
    "u",
    "v",
    "w",
    "x",
    "y",
    "z",
    "braceleft",
    "bar",
    "braceright",
    "asciitilde",
    "Adieresis",
    "Aring",
    "Ccedilla",
    "Eacute",
    "Ntilde",
    "Odieresis",
    "Udieresis",
    "aacute",
    "agrave",
    "acircumflex",
    "adieresis",
    "atilde",
    "aring",
    "ccedilla",
    "eacute",
    "egrave",
    "ecircumflex",
    "edieresis",
    "iacute",
    "igrave",
    "icircumflex",
    "idieresis",
    "ntilde",
    "oacute",
    "ograve",
    "ocircumflex",
    "odieresis",
    "otilde",
    "uacute",
    "ugrave",
    "ucircumflex",
    "udieresis",
    "dagger",
    "degree",
    "cent",
    "sterling",
    "section",
    "bullet",
    "paragraph",
    "germandbls",
    "registered",
    "copyright",
    "trademark",
    "acute",
    "dieresis",
    "notequal",
    "AE",
    "Oslash",
    "infinity",
    "plusminus",
    "lessequal",
    "greaterequal",
    "yen",
    "mu",
    "partialdiff",
    "summation",
    "product",
    "pi",
    "integral",
    "ordfeminine",
    "ordmasculine",
    "Omega",
    "ae",
    "oslash",
    "questiondown",
    "exclamdown",
    "logicalnot",
    "radical",
    "florin",
    "approxequal",
    "Delta",
    "guillemotleft",
    "guillemotright",
    "ellipsis",
    "nonbreakingspace",
    "Agrave",
    "Atilde",
    "Otilde",
    "OE",
    "oe",
    "endash",
    "emdash",
    "quotedblleft",
    "quotedblright",
    "quoteleft",
    "quoteright",
    "divide",
    "lozenge",
    "ydieresis",
    "Ydieresis",
    "fraction",
    "currency",
    "guilsinglleft",
    "guilsinglright",
    "fi",
    "fl",
    "daggerdbl",
    "periodcentered",
    "quotesinglbase",
    "quotedblbase",
    "perthousand",
    "Acircumflex",
    "Ecircumflex",
    "Aacute",
    "Edieresis",
    "Egrave",
    "Iacute",
    "Icircumflex",
    "Idieresis",
    "Igrave",
    "Oacute",
    "Ocircumflex",
    "apple",
    "Ograve",
    "Uacute",
    "Ucircumflex",
    "Ugrave",
    "dotlessi",
    "circumflex",
    "tilde",
    "macron",
    "breve",
    "dotaccent",
    "ring",
    "cedilla",
    "hungarumlaut",
    "ogonek",
    "caron",
    "Lslash",
    "lslash",
    "Scaron",
    "scaron",
    "Zcaron",
    "zcaron",
    "brokenbar",
    "Eth",
    "eth",
    "Yacute",
    "yacute",
    "Thorn",
    "thorn",
    "minus",
    "multiply",
    "onesuperior",
    "twosuperior",
    "threesuperior",
    "onehalf",
    "onequarter",
    "threequarters",
    "franc",
    "Gbreve",
    "gbreve",
    "Idotaccent",
    "Scedilla",
    "scedilla",
    "Cacute",
    "cacute",
    "Ccaron",
    "ccaron",
    "dcroat",
];


/// A list of glyph names.
#[derive(Clone, Copy, Default)]
pub struct Names<'a> {
    indexes: LazyArray16<'a, u16>,
    data: &'a [u8],
}

// TODO: add low-level iterator
impl<'a> Names<'a> {
    /// Returns a glyph name by ID.
    #[cfg(feature = "glyph-names")]
    pub fn get(&self, glyph_id: GlyphId) -> Option<&'a str> {
        let mut index = self.indexes.get(glyph_id.0)?;

        // 'If the name index is between 0 and 257, treat the name index
        // as a glyph index in the Macintosh standard order.'
        if usize::from(index) < MACINTOSH_NAMES.len() {
            Some(MACINTOSH_NAMES[usize::from(index)])
        } else {
            // 'If the name index is between 258 and 65535, then subtract 258 and use that
            // to index into the list of Pascal strings at the end of the table.'
            index -= MACINTOSH_NAMES.len() as u16;

            let mut s = Stream::new(self.data);
            let mut i = 0;
            while !s.at_end() && i < core::u16::MAX {
                let len = s.read::<u8>()?;

                if i == index {
                    if len == 0 {
                        // Empty name is an error.
                        break;
                    } else {
                        let name = s.read_bytes(usize::from(len))?;
                        return core::str::from_utf8(name).ok();
                    }
                } else {
                    s.advance(usize::from(len));
                }

                i += 1;
            }

            None
        }
    }

    /// Returns names count.
    #[inline]
    pub fn len(&self) -> u16 {
        self.indexes.len()
    }
}

impl core::fmt::Debug for Names<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "Names {{ ... }}")
    }
}


/// A [PostScript Table](https://docs.microsoft.com/en-us/typography/opentype/spec/post).
#[derive(Clone, Copy, Debug)]
pub struct Table<'a> {
    /// Italic angle in counter-clockwise degrees from the vertical.
    pub italic_angle: f32,
    /// Underline metrics.
    pub underline_metrics: LineMetrics,
    /// Flag that indicates that the font is monospaced.
    pub is_monospaced: bool,
    /// A list of glyph names.
    pub names: Names<'a>,
}


impl<'a> Table<'a> {
    /// Parses a table from raw data.
    pub fn parse(data: &'a [u8]) -> Option<Self> {
        if data.len() < TABLE_SIZE {
            return None;
        }

        let version = Stream::new(data).read::<u32>()?;
        if !(version == 0x00010000 || version == 0x00020000 ||
             version == 0x00025000 || version == 0x00030000 ||
             version == 0x00040000)
        {
            return None;
        }

        let italic_angle = Stream::read_at::<Fixed>(data, ITALIC_ANGLE_OFFSET)?.0;

        let underline_metrics = LineMetrics {
            position: Stream::read_at::<i16>(data, UNDERLINE_POSITION_OFFSET)?,
            thickness: Stream::read_at::<i16>(data, UNDERLINE_THICKNESS_OFFSET)?,
        };

        let is_monospaced = Stream::read_at::<u32>(data, IS_FIXED_PITCH_OFFSET)? != 0;

        let mut names = Names::default();
        // Only version 2.0 of the table has data at the end.
        if version == 0x00020000 {
            let mut s = Stream::new_at(data, TABLE_SIZE)?;
            let count = s.read::<u16>()?;
            names.indexes = s.read_array16::<u16>(count)?;
            names.data = s.tail()?;
        }

        Some(Table {
            italic_angle,
            underline_metrics,
            is_monospaced,
            names,
        })
    }
}
