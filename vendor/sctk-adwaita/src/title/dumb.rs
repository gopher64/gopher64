use tiny_skia::{Color, Pixmap};

#[derive(Debug)]
pub struct DumbTitleText {}

impl DumbTitleText {
    pub fn update_scale(&mut self, _scale: u32) {}

    pub fn update_title<S: Into<String>>(&mut self, _title: S) {}

    pub fn update_color(&mut self, _color: Color) {}

    pub fn pixmap(&self) -> Option<&Pixmap> {
        None
    }
}
