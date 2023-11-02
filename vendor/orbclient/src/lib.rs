// SPDX-License-Identifier: MIT

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(all(feature = "std", not(target_os = "redox")))]
#[path = "sys/sdl2.rs"]
mod sys;

#[cfg(all(feature = "std", target_os = "redox"))]
#[path = "sys/orbital.rs"]
mod sys;

#[cfg(feature = "std")]
pub use sys::{get_display_size, EventIter, Window};

#[cfg(feature = "unifont")]
pub static FONT: &[u8] = include_bytes!("../res/unifont.font");

pub use color::Color;
pub use event::*;
pub use graphicspath::GraphicsPath;
pub use renderer::Renderer;

#[cfg(feature = "std")]
mod blur;
pub mod color;
pub mod event;
pub mod graphicspath;
pub mod renderer;

#[derive(Clone, Copy, Debug)]
pub enum WindowFlag {
    Async,
    Back,
    Front,
    Borderless,
    Resizable,
    Transparent,
    Unclosable,
}

#[derive(Clone, Copy, Debug)]
pub enum Mode {
    Blend,     //Composite
    Overwrite, //Replace
}
