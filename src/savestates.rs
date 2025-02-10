#[cfg(target_arch = "aarch64")]
use device::__m128i;
use serde::ser::{Serialize, SerializeSeq, Serializer};
#[cfg(target_arch = "aarch64")]
include!(concat!(env!("OUT_DIR"), "/simd_bindings.rs"));
use crate::{device, ui};
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

pub fn serialize_m128i<S>(data: &__m128i, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let bytes: [u8; 16] = unsafe { std::mem::transmute(*data) };
    bytes.serialize(serializer)
}

pub fn serialize_m128i_array<S>(value: &[__m128i], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    // Create a sequence serializer for the array
    let mut seq = serializer.serialize_seq(Some(value.len()))?;

    // Serialize each __m128i as 16 bytes
    for item in value {
        // Safety: __m128i is 16 bytes
        let bytes: [u8; 16] = unsafe { std::mem::transmute(*item) };
        seq.serialize_element(&bytes)?;
    }

    seq.end()
}

pub fn create_savestate(device: &device::Device) {
    let compressed_file = ui::storage::compress_file(&postcard::to_stdvec(device).unwrap());
    std::fs::write(device.ui.paths.savestate_file_path.clone(), compressed_file).unwrap();
}
