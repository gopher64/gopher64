use crate::{cheats, device, retroachievements, ui};
#[cfg(target_arch = "aarch64")]
use device::__m128i;
use serde::de::{Deserialize, Deserializer, SeqAccess, Visitor};
use serde::ser::{Serialize, SerializeSeq, Serializer};
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;
use std::ops::Deref;

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

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Savestate {
    pub save_state: bool,
    pub load_state: bool,
    pub save_rewind: bool,
    pub load_rewind: bool,
    pub last_rewind_saved: f64,
    #[serde(skip)]
    pub rewind_pool:
        std::sync::Arc<std::sync::Mutex<std::collections::BTreeMap<i32, SavestateData>>>,
}

pub struct SavestateData {
    device: Box<device::Device>,
    saves: ui::storage::Saves,
    rdp_state: Vec<u8>,
    ra_state: Vec<u8>,
}

static DEVICE_CLONE: std::sync::LazyLock<std::sync::Mutex<Box<device::Device>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(device::Device::new(false)));

static SAVES_CLONE: std::sync::LazyLock<std::sync::Mutex<ui::storage::Saves>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(ui::storage::Saves::default()));

pub fn process_savestates(device: &mut device::Device) {
    if device.savestate.save_state || device.savestate.save_rewind {
        let rewind = device.savestate.save_rewind && !device.savestate.save_state;
        if rewind {
            device.savestate.save_rewind = false;
        }
        device.savestate.save_state = false;
        create_savestate(device, rewind, None);
    } else if device.savestate.load_state || device.savestate.load_rewind {
        device.savestate.load_state = false;
        let rewind = device.savestate.load_rewind;
        device.savestate.load_rewind = false;
        load_savestate(device, rewind, None);
    }
}

pub fn create_savestate(device: &mut device::Device, rewind: bool, rewind_frame: Option<i32>) {
    if !rewind {
        ui::video::check_framebuffers(0, device.rdram.size);
    }

    let mut rdp_state: Vec<u8> = vec![0; ui::video::state_size()];
    ui::video::save_state(rdp_state.as_mut_ptr());

    let mut ra_state: Vec<u8> = vec![0; retroachievements::state_size()];
    retroachievements::save_state(ra_state.as_mut_ptr(), ra_state.len());

    if let Ok(mut device_clone) = DEVICE_CLONE.lock()
        && let Ok(mut saves_clone) = SAVES_CLONE.lock()
    {
        device_clone.clone_state(device);
        saves_clone.clone_from(&device.ui.storage.saves);
    } else {
        ui::video::onscreen_message(
            "Failed to create savestate",
            ui::video::MESSAGE_LENGTH_MESSAGE_SHORT,
        );
        return;
    }

    let save_path = device.ui.storage.paths.savestate_file_path.clone();
    let save_state_slot = device.ui.storage.save_state_slot;
    let rewind_pool = device.savestate.rewind_pool.clone();

    tokio::spawn(async move {
        let mut error = false;

        if rewind {
            if let Ok(device_clone) = DEVICE_CLONE.lock()
                && let Ok(saves_clone) = SAVES_CLONE.lock()
                && let Ok(mut pool) = rewind_pool.lock()
            {
                let mut state = device::Device::new(false);
                state.clone_state(device_clone.deref());
                let key = if let Some(key) = rewind_frame {
                    key
                } else if let Some(key) = pool.keys().last() {
                    key + 1
                } else {
                    0
                };
                pool.insert(
                    key,
                    SavestateData {
                        device: state,
                        saves: saves_clone.clone(),
                        rdp_state,
                        ra_state,
                    },
                );
                if pool.len() > 30 {
                    pool.pop_first();
                }
            } else {
                error = true;
            }
        } else {
            let compressed_file = if let Ok(device_clone) = DEVICE_CLONE.lock()
                && let Ok(saves_clone) = SAVES_CLONE.lock()
                && let Ok(device_data) = postcard::to_stdvec(device_clone.deref())
                && let Ok(saves_data) = postcard::to_stdvec(saves_clone.deref())
                && let Ok(compressed_file) = ui::storage::compress_file(&[
                    (&device_data, "device"),
                    (&saves_data, "saves"),
                    (&rdp_state, "rdp_state"),
                    (&ra_state, "ra_state"),
                ]) {
                Some(compressed_file)
            } else {
                None
            };
            if let Some(compressed_file) = compressed_file {
                if let Err(e) = tokio::fs::write(save_path, compressed_file).await {
                    eprintln!("Error writing savestate: {}", e);
                    error = true;
                }
            } else {
                error = true;
                eprintln!("Error compressing savestate");
            }
        }

        if error {
            ui::video::onscreen_message(
                &format!("Failed to create savestate in slot {}", save_state_slot),
                ui::video::MESSAGE_LENGTH_MESSAGE_SHORT,
            );
        } else if !rewind {
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

    let state_data = if rewind {
        if let Some(rewind_frame) = rewind_frame {
            let timeout = std::time::Duration::from_secs(1);
            let now = std::time::Instant::now();
            loop {
                if let Ok(mut pool) = device.savestate.rewind_pool.lock()
                    && pool.contains_key(&rewind_frame)
                {
                    break pool.remove(&rewind_frame);
                }
                if now.elapsed() > timeout {
                    break None;
                }
            }
        } else if let Ok(mut pool) = device.savestate.rewind_pool.lock()
            && let Some((_key, state)) = pool.pop_last()
        {
            Some(state)
        } else {
            None
        }
    } else if let savestate = std::fs::read(&device.ui.storage.paths.savestate_file_path)
        && let Ok(savestate) = &savestate
        && let Ok(device_data) = ui::storage::decompress_file(savestate, "device")
        && let Ok(saves_data) = ui::storage::decompress_file(savestate, "saves")
        && let Ok(rdp_state) = ui::storage::decompress_file(savestate, "rdp_state")
        && let Ok(ra_state) = ui::storage::decompress_file(savestate, "ra_state")
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

    if let Some(mut state) = state_data
        && device.rdram.size == state.device.rdram.size
    {
        if device.netplay.is_none() {
            ui::video::check_framebuffers(0, device.rdram.size);
        }

        device.savestate.last_rewind_saved = state.device.vi.elapsed_time;

        std::mem::swap(&mut device.rng, &mut state.device.rng);

        std::mem::swap(&mut device.ui.storage.saves, &mut state.saves);

        std::mem::swap(&mut device.cpu, &mut state.device.cpu);
        std::mem::swap(&mut device.pif, &mut state.device.pif);

        std::mem::swap(&mut device.cart, &mut state.device.cart);
        std::mem::swap(&mut device.cart.rom, &mut state.device.cart.rom); // ROM is not included in the savestate

        std::mem::swap(&mut device.memory, &mut state.device.memory);
        std::mem::swap(&mut device.rsp, &mut state.device.rsp);
        std::mem::swap(&mut device.rdp, &mut state.device.rdp);

        device.rdram.mem.copy_from_slice(&state.device.rdram.mem); // RDRAM address should not change
        std::mem::swap(&mut device.rdram.regs, &mut state.device.rdram.regs);

        std::mem::swap(&mut device.mi, &mut state.device.mi);
        std::mem::swap(&mut device.pi, &mut state.device.pi);
        std::mem::swap(&mut device.vi, &mut state.device.vi);
        std::mem::swap(&mut device.ai, &mut state.device.ai);
        std::mem::swap(&mut device.si, &mut state.device.si);
        std::mem::swap(&mut device.ri, &mut state.device.ri);
        std::mem::swap(&mut device.vru, &mut state.device.vru);
        std::mem::swap(&mut device.cheats, &mut state.device.cheats);

        std::mem::swap(&mut device.transferpaks, &mut state.device.transferpaks);
        for (i, item) in device.transferpaks.iter_mut().enumerate() {
            std::mem::swap(
                &mut item.cart.rom,
                &mut state.device.transferpaks[i].cart.rom,
            ); // ROM is not included in the savestate
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
                device.memory.icache[line_index].instruction[i] =
                    device::cpu::decode_opcode(device, device.memory.icache[line_index].words[i]);
            }
        }

        device::pif::connect_pif_channels(device);
        for i in 0..4 {
            if let Some(handler) = device.pif.channels[i].pak_handler {
                if handler.pak_type == device::controller::PakType::RumblePak {
                    let rumblepak_handler = device::controller::PakHandler {
                        read: device::controller::rumble::read,
                        write: device::controller::rumble::write,
                        pak_type: device::controller::PakType::RumblePak,
                    };
                    device.pif.channels[i].pak_handler = Some(rumblepak_handler);
                } else if handler.pak_type == device::controller::PakType::MemPak {
                    let mempak_handler = device::controller::PakHandler {
                        read: device::controller::mempak::read,
                        write: device::controller::mempak::write,
                        pak_type: device::controller::PakType::MemPak,
                    };
                    device.pif.channels[i].pak_handler = Some(mempak_handler);
                } else if handler.pak_type == device::controller::PakType::TransferPak {
                    let tpak_handler = device::controller::PakHandler {
                        read: device::controller::transferpak::read,
                        write: device::controller::transferpak::write,
                        pak_type: device::controller::PakType::TransferPak,
                    };
                    device.pif.channels[i].pak_handler = Some(tpak_handler);
                }
            }
        }

        ui::audio::update_freq(device);
        ui::video::load_state(device, state.rdp_state.as_ptr());

        if !state.ra_state.is_empty() {
            retroachievements::load_state(state.ra_state.as_ptr(), state.ra_state.len());
        } else {
            retroachievements::load_state(std::ptr::null(), 0);
        }

        if device.cheats.enabled {
            cheats::execute_cheats(device, device.cheats.cheats.clone());
        }

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
