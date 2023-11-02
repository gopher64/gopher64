use owned_ttf_parser::{AsFaceRef, GlyphId, PreParsedSubtables};

const FONT: &[u8] = include_bytes!("../fonts/font.ttf");
const EMOJI_FONT: &[u8] = include_bytes!("../fonts/NotoColorEmoji-Partial.ttf");

#[test]
fn preparse_glyph_index() {
    let face = owned_ttf_parser::Face::parse(FONT, 0).unwrap();

    let pre_parse = PreParsedSubtables::from(face.clone());

    assert_eq!(pre_parse.glyph_index('x'), face.glyph_index('x'));

    assert_eq!(
        pre_parse.as_face_ref().glyph_index('x'),
        face.glyph_index('x')
    );
}

#[test]
fn preparse_glyph_variation_index() {
    let face = owned_ttf_parser::Face::parse(EMOJI_FONT, 0).unwrap();

    let pre_parse = PreParsedSubtables::from(face.clone());

    assert_eq!(
        pre_parse.glyph_variation_index('#', '\u{FE0F}'),
        face.glyph_variation_index('#', '\u{FE0F}')
    );

    assert_eq!(
        pre_parse
            .as_face_ref()
            .glyph_variation_index('#', '\u{FE0F}'),
        face.glyph_variation_index('#', '\u{FE0F}')
    );
}

#[test]
fn preparse_glyphs_kerning() {
    let face = owned_ttf_parser::Face::parse(FONT, 0).unwrap();

    let pre_parse = PreParsedSubtables::from(face.clone());

    let (a, b) = (GlyphId(92), GlyphId(93));

    assert_eq!(
        pre_parse.glyphs_hor_kerning(a, b),
        face.tables()
            .kern
            .iter()
            .flat_map(|c| c.subtables)
            .filter(|st| st.horizontal && !st.variable)
            .find_map(|st| st.glyphs_kerning(a, b))
    );
}
