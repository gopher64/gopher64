//! # Android NDK
//!
//! Bindings to the [Android NDK].
//!
//! [Android NDK]: https://developer.android.com/ndk/reference
#![warn(missing_debug_implementations, trivial_casts)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

pub mod asset;
pub mod audio;
pub mod bitmap;
pub mod configuration;
pub mod event;
pub mod hardware_buffer;
pub mod hardware_buffer_format;
pub mod input_queue;
pub mod looper;
pub mod media;
pub mod native_activity;
pub mod native_window;
pub mod surface_texture;
pub mod trace;
mod utils;
