#[cfg(target_arch = "aarch64")]
use device::__m128i;
use serde::de::{Deserialize, Deserializer, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeSeq, Serializer};
#[cfg(target_arch = "aarch64")]
include!(concat!(env!("OUT_DIR"), "/simd_bindings.rs"));
use crate::{device, ui};
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

struct M128iArrayVisitor<const N: usize>;

impl<'de, const N: usize> Visitor<'de> for M128iArrayVisitor<N> {
    type Value = [__m128i; N];

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str(&format!("an array of {} 128-bit integers", N))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut arr: [__m128i; N] = unsafe { [_mm_setzero_si128(); N] };
        for index in 0..N {
            match seq.next_element::<u128>()? {
                Some(value) => arr[index] = unsafe { std::mem::transmute(value) },
                None => return Err(serde::de::Error::invalid_length(index, &self)),
            }
        }
        Ok(arr)
    }
}

pub fn deserialize_m128i_array<'de, D, const N: usize>(
    deserializer: D,
) -> Result<[__m128i; N], D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserializer.deserialize_seq(M128iArrayVisitor)
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
    let data: &[(&[u8], &str)] = &[
        (&postcard::to_stdvec(device).unwrap(), "device"),
        (&postcard::to_stdvec(&device.ui.saves).unwrap(), "saves"),
    ];
    let compressed_file = ui::storage::compress_file(data);
    std::fs::write(device.ui.paths.savestate_file_path.clone(), compressed_file).unwrap();
}

pub fn load_savestate(device: &mut device::Device) {
    let savestate = std::fs::read(&mut device.ui.paths.savestate_file_path);
    if savestate.is_ok() {
        let device_bytes = ui::storage::decompress_file(savestate.as_ref().unwrap(), "device");
        let save_bytes = ui::storage::decompress_file(savestate.as_ref().unwrap(), "saves");
        let state: device::Device = postcard::from_bytes(&device_bytes).unwrap();

        device.ui.saves = postcard::from_bytes(&save_bytes).unwrap();

        device.cpu = state.cpu;
        device.pif = state.pif;
        device.cart = state.cart;
        device.memory = state.memory;
        device.rsp = state.rsp;
        device.rdp = state.rdp;
        device.rdram = state.rdram;
        device.mi = state.mi;
        device.pi = state.pi;
        device.vi = state.vi;
        device.ai = state.ai;
        device.si = state.si;
        device.ri = state.ri;

        // don't want to copy window_notifier, word_receiver, gui_ctx
        device.vru.status = state.vru.status;
        device.vru.voice_state = state.vru.voice_state;
        device.vru.load_offset = state.vru.load_offset;
        device.vru.voice_init = state.vru.voice_init;
        device.vru.word_buffer = state.vru.word_buffer;
        device.vru.words = state.vru.words;
        device.vru.talking = state.vru.talking;
        device.vru.word_mappings = state.vru.word_mappings;

        device::memory::init(device);
        device::vi::set_expected_refresh_rate(device);
        device::cpu::map_instructions(device);
        device::cop0::map_instructions(device);
        device::cop1::map_instructions(device);
        device::cop2::map_instructions(device);
        device::rsp_cpu::map_instructions(device);

        let mut mem_addr = 0x1000;
        while mem_addr < 0x2000 {
            let data =
                u32::from_be_bytes(device.rsp.mem[mem_addr..mem_addr + 4].try_into().unwrap());
            device.rsp.cpu.instructions[((mem_addr & 0xFFF) / 4) as usize].func =
                device::rsp_cpu::decode_opcode(device, data);
            device.rsp.cpu.instructions[((mem_addr & 0xFFF) / 4) as usize].opcode = data;
            mem_addr += 4;
        }

        for line_index in 0..512 {
            device.memory.icache[line_index].instruction[0] =
                device::cpu::decode_opcode(device, device.memory.icache[line_index].words[0]);
            device.memory.icache[line_index].instruction[1] =
                device::cpu::decode_opcode(device, device.memory.icache[line_index].words[1]);
            device.memory.icache[line_index].instruction[2] =
                device::cpu::decode_opcode(device, device.memory.icache[line_index].words[2]);
            device.memory.icache[line_index].instruction[3] =
                device::cpu::decode_opcode(device, device.memory.icache[line_index].words[3]);
            device.memory.icache[line_index].instruction[4] =
                device::cpu::decode_opcode(device, device.memory.icache[line_index].words[4]);
            device.memory.icache[line_index].instruction[5] =
                device::cpu::decode_opcode(device, device.memory.icache[line_index].words[5]);
            device.memory.icache[line_index].instruction[6] =
                device::cpu::decode_opcode(device, device.memory.icache[line_index].words[6]);
            device.memory.icache[line_index].instruction[7] =
                device::cpu::decode_opcode(device, device.memory.icache[line_index].words[7]);
        }

        device::pif::connect_pif_channels(device);
        for i in 0..4 {
            if device.pif.channels[i].pak_handler.is_some() {
                if device.pif.channels[i].pak_handler.unwrap().pak_type
                    == device::controller::PakType::RumblePak
                {
                    let rumblepak_handler = device::controller::PakHandler {
                        read: device::controller::rumble::read,
                        write: device::controller::rumble::write,
                        pak_type: device::controller::PakType::RumblePak,
                    };
                    device.pif.channels[i].pak_handler = Some(rumblepak_handler);
                } else if device.pif.channels[i].pak_handler.unwrap().pak_type
                    == device::controller::PakType::MemPak
                {
                    let mempak_handler = device::controller::PakHandler {
                        read: device::controller::mempak::read,
                        write: device::controller::mempak::write,
                        pak_type: device::controller::PakType::MemPak,
                    };
                    device.pif.channels[i].pak_handler = Some(mempak_handler);
                }
            }
        }

        ui::audio::close(&mut device.ui);
        ui::audio::init(&mut device.ui, device.ai.freq);
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

pub fn default_memory_read(
) -> [fn(&mut device::Device, u64, device::memory::AccessSize) -> u32; 0x2000] {
    [device::rdram::read_mem; 0x2000]
}

pub fn default_memory_write() -> [fn(&mut device::Device, u64, u32, u32); 0x2000] {
    [device::rdram::write_mem; 0x2000]
}
