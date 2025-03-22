use crate::{device, ui};
#[cfg(target_arch = "aarch64")]
use device::__m128i;
use serde::de::{Deserialize, Deserializer, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeSeq, Serializer};
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
        let mut arr: [__m128i; N] = [device::zero_m128i(); N];
        for (index, item) in arr.iter_mut().enumerate().take(N) {
            match seq.next_element::<u128>()? {
                Some(value) => *item = unsafe { std::mem::transmute::<u128, __m128i>(value) },
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
    Ok(unsafe { std::mem::transmute::<u128, __m128i>(bytes) })
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
    let rdp_state_size = ui::video::state_size();
    let rdp_state: Vec<u8> = vec![0; rdp_state_size as usize];
    ui::video::save_state(rdp_state.as_ptr() as *mut u8);

    let data: &[(&[u8], &str)] = &[
        (&postcard::to_stdvec(device).unwrap(), "device"),
        (
            &postcard::to_stdvec(&device.ui.storage.saves).unwrap(),
            "saves",
        ),
        (&postcard::to_stdvec(&rdp_state).unwrap(), "rdp_state"),
    ];
    let compressed_file = ui::storage::compress_file(data);
    std::fs::write(
        device.ui.storage.paths.savestate_file_path.clone(),
        compressed_file,
    )
    .unwrap();
}

pub fn load_savestate(device: &mut device::Device) {
    let savestate = std::fs::read(&device.ui.storage.paths.savestate_file_path);
    if savestate.is_ok() {
        let device_bytes = ui::storage::decompress_file(savestate.as_ref().unwrap(), "device");
        let save_bytes = ui::storage::decompress_file(savestate.as_ref().unwrap(), "saves");
        let rdp_bytes = ui::storage::decompress_file(savestate.as_ref().unwrap(), "rdp_state");
        if let Ok(state) = postcard::from_bytes::<device::Device>(&device_bytes) {
            device.ui.storage.saves = postcard::from_bytes(&save_bytes).unwrap();

            device.cpu = state.cpu;
            device.pif = state.pif;

            let rom = device.cart.rom.clone();
            device.cart = state.cart;
            device.cart.rom = rom;

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
            device.vru = state.vru;

            let mut tpak_rom = [vec![], vec![], vec![], vec![]];
            for (i, item) in tpak_rom.iter_mut().enumerate() {
                *item = device.transferpaks[i].cart.rom.clone();
            }
            device.transferpaks = state.transferpaks;
            for (i, item) in tpak_rom.iter().enumerate() {
                device.transferpaks[i].cart.rom = item.clone();
            }

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
                device.rsp.cpu.instructions[(mem_addr & 0xFFF) / 4].func =
                    device::rsp_cpu::decode_opcode(device, data);
                device.rsp.cpu.instructions[(mem_addr & 0xFFF) / 4].opcode = data;
                mem_addr += 4;
            }

            for line_index in 0..512 {
                for i in 0..8 {
                    device.memory.icache[line_index].instruction[i] = device::cpu::decode_opcode(
                        device,
                        device.memory.icache[line_index].words[i],
                    );
                }
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
                    } else if device.pif.channels[i].pak_handler.unwrap().pak_type
                        == device::controller::PakType::TransferPak
                    {
                        let tpak_handler = device::controller::PakHandler {
                            read: device::controller::transferpak::read,
                            write: device::controller::transferpak::write,
                            pak_type: device::controller::PakType::TransferPak,
                        };
                        device.pif.channels[i].pak_handler = Some(tpak_handler);
                    }
                }
            }

            ui::audio::close(&mut device.ui);
            ui::audio::init(&mut device.ui, device.ai.freq);
            ui::video::load_state(device, rdp_bytes.as_ptr() as *mut u8);
        } else {
            println!("Failed to load savestate");
        }
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

pub fn default_memory_read_fast()
-> [fn(&device::Device, u64, device::memory::AccessSize) -> u32; 0x2000] {
    [device::unmapped::read_mem_fast; 0x2000]
}

pub fn default_memory_read()
-> [fn(&mut device::Device, u64, device::memory::AccessSize) -> u32; 0x2000] {
    [device::unmapped::read_mem; 0x2000]
}

pub fn default_memory_write() -> [fn(&mut device::Device, u64, u32, u32); 0x2000] {
    [device::unmapped::write_mem; 0x2000]
}
