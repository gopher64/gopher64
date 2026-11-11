use crate::{cheats, device, retroachievements, ui};
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
        formatter.write_str(&format!("an array of {N} 128-bit integers"))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut arr: [__m128i; N] = [device::zero_m128i(); N];
        for (index, item) in arr.iter_mut().enumerate() {
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

/// Serialize an `FxHashMap` in sorted-key order so the output is deterministic regardless
/// of insertion history. Without this, two logically-identical maps (e.g. the TLB lookup
/// tables after a savestate round-trip) can serialize to different bytes, which would make
/// the serialized device state — and any hash of it — non-canonical.
pub fn serialize_sorted_map<S, K, V>(
    map: &rustc_hash::FxHashMap<K, V>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
    K: serde::Serialize + Ord,
    V: serde::Serialize,
{
    use serde::ser::SerializeMap;
    let mut entries: Vec<(&K, &V)> = map.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    let mut map_ser = serializer.serialize_map(Some(entries.len()))?;
    for (k, v) in entries {
        map_ser.serialize_entry(k, v)?;
    }
    map_ser.end()
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

#[derive(Default)]
pub struct Savestate {
    pub save_state: bool,
    pub load_state: bool,
    pub load_rewind: bool,
    pub last_rewind_saved: f64,
    pub rewind_pool: std::collections::BTreeMap<i32, SavestateData>,
}

const REWIND_POOL_SIZE: usize = 30;

pub struct SavestateData {
    device: Box<device::Device>,
    saves: ui::storage::Saves,
    rdp_state: Vec<u8>,
    ra_state: Vec<u8>,
}

impl SavestateData {
    fn new() -> SavestateData {
        SavestateData {
            device: device::Device::new(false),
            saves: ui::storage::Saves::default(),
            rdp_state: Vec::new(),
            ra_state: Vec::new(),
        }
    }
}

pub fn process_savestates(device: &mut device::Device) {
    let save_rewind = if device.netplay.is_none()
        && device.ui.config.emulation.rewind
        && device.vi.elapsed_time - device.ui.savestate.last_rewind_saved > 1.0
    {
        device.ui.savestate.last_rewind_saved = device.vi.elapsed_time;
        true
    } else {
        false
    };

    if device.ui.savestate.save_state || save_rewind {
        create_savestate(device, !device.ui.savestate.save_state, None);
        device.ui.savestate.save_state = false;
    } else if device.ui.savestate.load_state || device.ui.savestate.load_rewind {
        load_savestate(device, device.ui.savestate.load_rewind, None);
        device.ui.savestate.load_state = false;
        device.ui.savestate.load_rewind = false;
    }
}

pub fn create_savestate(device: &mut device::Device, rewind: bool, rewind_frame: Option<i32>) {
    ui::video::idle();

    // Rewind snapshots reuse the buffers of the pool entry they replace or evict.
    let rewind_key = rewind.then(|| {
        let pool = &device.ui.savestate.rewind_pool;
        rewind_frame.unwrap_or_else(|| pool.keys().last().map_or(0, |key| key + 1))
    });
    let mut state = if let Some(key) = rewind_key {
        let pool = &mut device.ui.savestate.rewind_pool;
        pool.remove(&key)
            .or_else(|| {
                if pool.len() >= REWIND_POOL_SIZE {
                    pool.pop_first().map(|(_, state)| state)
                } else {
                    None
                }
            })
            .unwrap_or_else(SavestateData::new)
    } else {
        SavestateData::new()
    };

    state.rdp_state.resize(ui::video::state_size(), 0);
    ui::video::save_state(state.rdp_state.as_mut_ptr());

    state.ra_state.resize(retroachievements::state_size(), 0);
    retroachievements::save_state(state.ra_state.as_mut_ptr(), state.ra_state.len());

    state.device.clone_state(device);
    state.saves.copy_from(&device.ui.storage.saves);

    if let Some(key) = rewind_key {
        device.ui.savestate.rewind_pool.insert(key, state);
        return;
    }

    let save_path = device.ui.storage.paths.savestate_file_path.clone();
    let save_state_slot = device.ui.storage.save_state_slot;

    tokio::spawn(async move {
        let compressed_file = if let Ok(device_data) = postcard::to_stdvec(&state.device)
            && let Ok(saves_data) = postcard::to_stdvec(&state.saves)
            && let Ok(compressed_file) = ui::storage::compress_file(&[
                (&device_data, "device"),
                (&saves_data, "saves"),
                (&state.rdp_state, "rdp_state"),
                (&state.ra_state, "ra_state"),
            ]) {
            Some(compressed_file)
        } else {
            None
        };
        let error = if let Some(compressed_file) = compressed_file {
            if let Err(e) = tokio::fs::write(save_path, compressed_file).await {
                eprintln!("Error writing savestate: {}", e);
                true
            } else {
                false
            }
        } else {
            eprintln!("Error compressing savestate");
            true
        };

        if error {
            ui::video::onscreen_message(
                &format!("Failed to create savestate in slot {}", save_state_slot),
                ui::video::MESSAGE_LENGTH_MESSAGE_SHORT,
            );
        } else {
            ui::video::onscreen_message(
                &format!("Savestate created in slot {}", save_state_slot),
                ui::video::MESSAGE_LENGTH_MESSAGE_VERY_SHORT,
            );
        }
    });
}

pub fn load_savestate(device: &mut device::Device, rewind: bool, rewind_frame: Option<i32>) {
    if retroachievements::get_hardcore() {
        ui::video::onscreen_message(
            "Cannot load savestate in RA hardcore mode",
            ui::video::MESSAGE_LENGTH_MESSAGE_SHORT,
        );
        return;
    }

    let loaded = if rewind && let Some(rewind_frame) = rewind_frame {
        // Keep the entry: GGRS does not re-save the frame it rolls back to, and may
        // roll back to the same frame again (e.g. late inputs from another player).
        if let Some(state) = device.ui.savestate.rewind_pool.remove(&rewind_frame) {
            let loaded = apply_state(device, &state);
            device.ui.savestate.rewind_pool.insert(rewind_frame, state);
            loaded
        } else {
            false
        }
    } else {
        let state_data = if rewind {
            device
                .ui
                .savestate
                .rewind_pool
                .pop_last()
                .map(|(_key, state)| state)
        } else if let Ok(savestate) = std::fs::read(&device.ui.storage.paths.savestate_file_path)
            && let Ok(device_data) = ui::storage::decompress_file(&savestate, "device")
            && let Ok(saves_data) = ui::storage::decompress_file(&savestate, "saves")
            && let Ok(rdp_state) = ui::storage::decompress_file(&savestate, "rdp_state")
            && let Ok(ra_state) = ui::storage::decompress_file(&savestate, "ra_state")
            && let Ok(state) = postcard::from_bytes::<Box<device::Device>>(&device_data)
            && let Ok(saves) = postcard::from_bytes(&saves_data)
        {
            Some(SavestateData {
                device: state,
                saves,
                rdp_state,
                ra_state,
            })
        } else {
            None
        };
        state_data.is_some_and(|state| apply_state(device, &state))
    };

    if loaded {
        if !rewind {
            ui::video::onscreen_message(
                &format!(
                    "Savestate loaded from slot {}",
                    device.ui.storage.save_state_slot
                ),
                ui::video::MESSAGE_LENGTH_MESSAGE_VERY_SHORT,
            );
        }
    } else {
        let (message, length) = if !rewind {
            (
                format!(
                    "Failed to load savestate from slot {}",
                    device.ui.storage.save_state_slot
                ),
                ui::video::MESSAGE_LENGTH_MESSAGE_VERY_SHORT,
            )
        } else if device.netplay.is_none() {
            (
                "Failed to rewind".to_string(),
                ui::video::MESSAGE_LENGTH_MESSAGE_VERY_SHORT,
            )
        } else {
            (
                "Failed to rollback".to_string(),
                ui::video::MESSAGE_LENGTH_MESSAGE_SHORT,
            )
        };
        ui::video::onscreen_message(&message, length);
    }
}

/// Restore emulator state from `state`. The ROMs are not part of the savestate and are kept.
fn apply_state(device: &mut device::Device, state: &SavestateData) -> bool {
    if device.rdram.size != state.device.rdram.size {
        return false;
    }

    ui::video::idle();

    device.ui.savestate.last_rewind_saved = state.device.vi.elapsed_time;

    // clone_from keeps the RDRAM allocation (same size), so its address does not change.
    device.clone_state(&state.device);
    device.ui.storage.saves.copy_from(&state.saves);

    device::memory::init(device);
    if device.mi.regs[device::mi::MI_INIT_MODE_REG] & device::mi::MI_INIT_MODE != 0 {
        device::rdram::set_write_handler(device, device::rdram::write_mem_repeat);
    }
    device::vi::set_expected_refresh_rate(device);
    device::cpu::map_instructions(device);
    device::cop0::map_instructions(device);
    device::cop1::map_instructions(device);
    device::cop1::set_host_flush_mode(device);
    device::cop2::map_instructions(device);
    device::rsp_cpu::map_instructions(device);

    let mut mem_addr = 0x1000;
    while mem_addr < 0x2000 {
        let data = u32::from_be_bytes(device.rsp.mem[mem_addr..mem_addr + 4].try_into().unwrap());
        device.rsp.cpu.instructions[(mem_addr & 0xFFF) / 4].func =
            device::rsp_cpu::decode_opcode(device, data);
        device.rsp.cpu.instructions[(mem_addr & 0xFFF) / 4].opcode = data;
        mem_addr += 4;
    }

    for line_index in 0..512 {
        for i in 0..8 {
            device.memory.icache[line_index].instruction[i] =
                device::cpu::decode_opcode(device, device.memory.icache[line_index].words[i]);
        }
    }

    device::pif::connect_pif_channels(device);
    for item in device.pif.channels.iter_mut().take(4) {
        if let Some(handler) = item.pak_handler.as_mut() {
            use device::controller::{PakType, mempak, rumble, transferpak};
            type Handler = fn(&mut device::Device, usize, u16, usize, usize);
            let (read, write): (Handler, Handler) = match handler.pak_type {
                PakType::RumblePak => (rumble::read, rumble::write),
                PakType::MemPak => (mempak::read, mempak::write),
                PakType::TransferPak => (transferpak::read, transferpak::write),
                PakType::None => continue,
            };
            handler.read = read;
            handler.write = write;
        }
    }

    ui::audio::update_freq(device);
    ui::video::load_state(device, state.rdp_state.as_ptr(), state.rdp_state.len());

    if !state.ra_state.is_empty() {
        retroachievements::load_state(state.ra_state.as_ptr(), state.ra_state.len());
    } else {
        retroachievements::load_state(std::ptr::null(), 0);
    }

    if device.cheats.enabled {
        cheats::execute_cheats(device, device.cheats.cheats.clone());
    }

    true
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
