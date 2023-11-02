//! Parsing encodings from their string representation.
#![deny(unsafe_code)]

use crate::helper::{Helper, NestingLevel};
use crate::Encoding;

pub(crate) const QUALIFIERS: &[char] = &[
    'r', // const
    'n', // in
    'N', // inout
    'o', // out
    'O', // bycopy
    'R', // byref
    'V', // oneway
];

pub(crate) fn rm_enc_prefix<'a>(
    s: &'a str,
    enc: &Encoding,
    level: NestingLevel,
) -> Option<&'a str> {
    use Helper::*;
    match Helper::new(enc) {
        Primitive(primitive) => s.strip_prefix(primitive.to_str()),
        BitField(b, _type) => {
            // TODO: Use the type on GNUStep (nesting level?)
            let s = s.strip_prefix('b')?;
            rm_int_prefix(s, b as usize)
        }
        Indirection(kind, t) => {
            let s = s.strip_prefix(kind.prefix())?;
            rm_enc_prefix(s, t, level.indirection(kind))
        }
        Array(len, item) => {
            let mut s = s;
            s = s.strip_prefix('[')?;
            s = rm_int_prefix(s, len)?;
            s = rm_enc_prefix(s, item, level.array())?;
            s.strip_prefix(']')
        }
        Container(kind, name, fields) => {
            let mut s = s;
            s = s.strip_prefix(kind.start())?;
            s = s.strip_prefix(name)?;
            if let Some(level) = level.container() {
                s = s.strip_prefix('=')?;
                for field in fields {
                    s = rm_enc_prefix(s, field, level)?;
                }
            }
            s.strip_prefix(kind.end())
        }
    }
}

fn chomp_int(s: &str) -> Option<(usize, &str)> {
    // Chomp until we hit a non-digit
    let (num, t) = match s.find(|c: char| !c.is_ascii_digit()) {
        Some(i) => s.split_at(i),
        None => (s, ""),
    };
    num.parse().map(|n| (n, t)).ok()
}

fn rm_int_prefix(s: &str, other: usize) -> Option<&str> {
    chomp_int(s).and_then(|(n, t)| if other == n { Some(t) } else { None })
}
