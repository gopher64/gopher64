#[cfg(target_arch = "aarch64")]
use device::__m128i;
use serde::de::{Deserialize, Deserializer, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeSeq, Serializer};
#[cfg(target_arch = "aarch64")]
include!(concat!(env!("OUT_DIR"), "/simd_bindings.rs"));
use crate::{device, ui};
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

struct M128iArrayVisitor16;
struct M128iArrayVisitor32;

impl<'de> Visitor<'de> for M128iArrayVisitor16 {
    type Value = [__m128i; 16];

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("an array of 128-bit integers")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut arr: [__m128i; 16] = unsafe { [_mm_setzero_si128(); 16] };
        let mut index = 0;
        while let Some(value) = seq.next_element::<u128>()? {
            arr[index] = unsafe { std::mem::transmute(value) };
            index += 1;
        }
        Ok(arr)
    }
}

pub fn deserialize_m128i_array16<'de, D>(deserializer: D) -> Result<[__m128i; 16], D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserializer.deserialize_seq(M128iArrayVisitor16)
}

impl<'de> Visitor<'de> for M128iArrayVisitor32 {
    type Value = [__m128i; 32];

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("an array of 128-bit integers")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut arr: [__m128i; 32] = unsafe { [_mm_setzero_si128(); 32] };
        let mut index = 0;
        while let Some(value) = seq.next_element::<u128>()? {
            arr[index] = unsafe { std::mem::transmute(value) };
            index += 1;
        }
        Ok(arr)
    }
}

pub fn deserialize_m128i_array32<'de, D>(deserializer: D) -> Result<[__m128i; 32], D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserializer.deserialize_seq(M128iArrayVisitor32)
}

pub fn serialize_m128i<S>(data: &__m128i, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let bytes: u128 = unsafe { std::mem::transmute(*data) };
    bytes.serialize(serializer)
}

pub fn deserialize_m128i<'de, D>(deserializer: D) -> Result<__m128i, D::Error>
where
    D: Deserializer<'de>,
{
    let bytes = u128::deserialize(deserializer)?;
    Ok(unsafe { std::mem::transmute(bytes) })
}

pub fn serialize_m128i_array<S>(value: &[__m128i], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let mut seq = serializer.serialize_seq(Some(value.len()))?;

    for item in value {
        let bytes: u128 = unsafe { std::mem::transmute(*item) };
        seq.serialize_element(&bytes)?;
    }

    seq.end()
}

pub fn create_savestate(device: &device::Device) {
    let compressed_file = ui::storage::compress_file(&postcard::to_stdvec(device).unwrap());
    std::fs::write(device.ui.paths.savestate_file_path.clone(), compressed_file).unwrap();
}

pub fn load_savestate(device: &mut device::Device) {
    let savestate = std::fs::read(&mut device.ui.paths.savestate_file_path);
    if savestate.is_ok() {
        let savestate_bytes = ui::storage::decompress_file(&savestate.unwrap());
        let _state: device::Device = postcard::from_bytes(&savestate_bytes).unwrap();
    }
}

pub fn default_pak_handler() -> fn(&mut device::Device, usize, u16, usize, usize) {
    device::controller::mempak::read
}

pub fn default_instruction() -> fn(&mut device::Device, u32) {
    device::cop0::reserved
}

pub fn default_instructions<const N: usize>() -> [fn(&mut device::Device, u32); N]
where
    [fn(&mut device::Device, u32); N]: Sized,
{
    [device::cop0::reserved; N]
}

pub fn default_event_handler() -> fn(&mut device::Device) {
    device::pi::dma_event
}

pub fn default_memory_read(
) -> [fn(&mut device::Device, u64, device::memory::AccessSize) -> u32; 0x2000] {
    [device::rdram::read_mem; 0x2000]
}

pub fn default_memory_write() -> [fn(&mut device::Device, u64, u32, u32); 0x2000] {
    [device::rdram::write_mem; 0x2000]
}
