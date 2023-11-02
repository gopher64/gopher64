//! Tests for the FaceMut trait.
use owned_ttf_parser::{AsFaceRef, FaceMut, OwnedFace};

const VFONT: &[u8] = include_bytes!("../fonts/Cantarell-VF.otf");

#[test]
fn set_variation() {
    let mut face = OwnedFace::from_vec(VFONT.to_vec(), 0).unwrap();
    let axis = face.as_face_ref().variation_axes().get(0).unwrap();
    let def_coord = face.as_face_ref().variation_coordinates()[0];

    // after setting variation on the owned face it should change
    let new_value = axis.def_value + 100.0;
    face.set_variation(axis.tag, new_value)
        .expect("Should be some");

    let coord = face.as_face_ref().variation_coordinates()[0];
    assert!(coord.get() > def_coord.get());
}
