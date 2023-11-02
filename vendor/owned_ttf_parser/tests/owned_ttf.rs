use owned_ttf_parser::{AsFaceRef, OwnedFace};

const FONT: &[u8] = include_bytes!("../fonts/font.ttf");

#[test]
fn move_and_use() {
    let owned_data = FONT.to_vec();
    let pin_face = OwnedFace::from_vec(owned_data, 0).unwrap();

    let ascent = pin_face.as_face_ref().ascender();

    // force a move
    let moved = Box::new(pin_face);

    assert_eq!(moved.as_face_ref().ascender(), ascent);
}
