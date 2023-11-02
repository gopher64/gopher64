#[cfg(target_arch = "wasm32")]
extern crate wasm_bindgen;
#[cfg(feature = "aes256")]
mod aes256sha256;
mod bcj2;
#[cfg(feature = "aes256")]
pub use aes256sha256::*;
#[cfg(target_arch = "wasm32")]
mod wasm;
extern crate filetime_creation as ft;
pub(crate) mod archive;
mod bcj;
#[cfg(not(target_arch = "wasm32"))]
mod de_funcs;
pub(crate) mod decoders;
mod delta;
#[cfg(feature = "compress")]
mod en_funcs;
#[cfg(feature = "compress")]
mod encoders;
mod error;
pub(crate) mod folder;
mod method_options;
pub use method_options::*;
mod password;
mod reader;
#[cfg(feature = "compress")]
mod writer;
pub use archive::*;
#[cfg(not(target_arch = "wasm32"))]
pub use de_funcs::*;
#[cfg(feature = "compress")]
pub use en_funcs::*;
pub use error::Error;
pub use lzma_rust as lzma;
pub use nt_time;
pub use password::Password;
pub use reader::SevenZReader;
pub use reader::FolderDecoder;
#[cfg(feature = "compress")]
pub use writer::*;
