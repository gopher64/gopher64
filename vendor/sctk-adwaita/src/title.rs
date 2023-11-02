use tiny_skia::{Color, Pixmap};

#[cfg(any(feature = "crossfont", feature = "ab_glyph"))]
mod config;
#[cfg(any(feature = "crossfont", feature = "ab_glyph"))]
mod font_preference;

#[cfg(feature = "crossfont")]
mod crossfont_renderer;

#[cfg(all(not(feature = "crossfont"), feature = "ab_glyph"))]
mod ab_glyph_renderer;

#[cfg(all(not(feature = "crossfont"), not(feature = "ab_glyph")))]
mod dumb;

#[derive(Debug)]
pub struct TitleText {
    #[cfg(feature = "crossfont")]
    imp: crossfont_renderer::CrossfontTitleText,
    #[cfg(all(not(feature = "crossfont"), feature = "ab_glyph"))]
    imp: ab_glyph_renderer::AbGlyphTitleText,
    #[cfg(all(not(feature = "crossfont"), not(feature = "ab_glyph")))]
    imp: dumb::DumbTitleText,
}

impl TitleText {
    pub fn new(color: Color) -> Option<Self> {
        #[cfg(feature = "crossfont")]
        return crossfont_renderer::CrossfontTitleText::new(color)
            .ok()
            .map(|imp| Self { imp });

        #[cfg(all(not(feature = "crossfont"), feature = "ab_glyph"))]
        return Some(Self {
            imp: ab_glyph_renderer::AbGlyphTitleText::new(color),
        });

        #[cfg(all(not(feature = "crossfont"), not(feature = "ab_glyph")))]
        {
            let _ = color;
            return None;
        }
    }

    pub fn update_scale(&mut self, scale: u32) {
        self.imp.update_scale(scale)
    }

    pub fn update_title<S: Into<String>>(&mut self, title: S) {
        self.imp.update_title(title)
    }

    pub fn update_color(&mut self, color: Color) {
        self.imp.update_color(color)
    }

    pub fn pixmap(&self) -> Option<&Pixmap> {
        self.imp.pixmap()
    }
}
