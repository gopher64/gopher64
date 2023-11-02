use crate::{FontRef, FontVec, VariableFont, VariationAxis};
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
use owned_ttf_parser::{self as ttfp, AsFaceRef, FaceMut};

impl VariableFont for FontRef<'_> {
    fn set_variation(&mut self, axis: &[u8; 4], value: f32) -> bool {
        self.0
            .set_variation(ttfp::Tag::from_bytes(axis), value)
            .is_some()
    }

    fn variations(&self) -> Vec<VariationAxis> {
        variations(self.0.as_face_ref())
    }
}

impl VariableFont for FontVec {
    fn set_variation(&mut self, axis: &[u8; 4], value: f32) -> bool {
        self.0
            .set_variation(ttfp::Tag::from_bytes(axis), value)
            .is_some()
    }

    fn variations(&self) -> Vec<VariationAxis> {
        variations(self.0.as_face_ref())
    }
}

fn variations(face: &ttfp::Face<'_>) -> Vec<VariationAxis> {
    face.variation_axes()
        .into_iter()
        .map(|axis| {
            #[cfg(feature = "std")]
            let name = face.names().into_iter().find_map(|n| {
                if n.name_id == axis.name_id {
                    n.to_string()
                } else {
                    None
                }
            });
            #[cfg(not(feature = "std"))]
            let name = None;
            VariationAxis {
                tag: axis.tag.to_bytes(),
                name,
                min_value: axis.min_value,
                default_value: axis.def_value,
                max_value: axis.max_value,
                hidden: axis.hidden,
            }
        })
        .collect()
}
