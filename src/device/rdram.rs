use crate::device;
use crate::retroachievements;
use crate::ui;
use std::alloc::{Layout, alloc_zeroed};

//const RDRAM_CONFIG_REG: usize = 0;
//const RDRAM_DEVICE_ID_REG: usize = 1;
//const RDRAM_DELAY_REG: usize = 2;
const RDRAM_MODE_REG: usize = 3;
//const RDRAM_REF_INTERVAL_REG: usize = 4;
//const RDRAM_REF_ROW_REG: usize = 5;
//const RDRAM_RAS_INTERVAL_REG: usize = 6;
//const RDRAM_MIN_INTERVAL_REG: usize = 7;
//const RDRAM_ADDR_SELECT_REG: usize = 8;
//const RDRAM_DEVICE_MANUF_REG: usize = 9;
pub const RDRAM_REGS_COUNT: usize = 10;

pub const RDRAM_MASK: usize = 0xFFFFFF;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Rdram {
    pub mem: Vec<u8>,
    pub size: u32,
    pub regs: [[u32; RDRAM_REGS_COUNT]; 4],
}

impl Rdram {
    /// Clone `other` into `self`, reusing `self.mem`'s allocation so it is not reallocated
    /// (and keeps its address, which the RDP holds) when the sizes match.
    pub fn clone_state_from(&mut self, other: &Rdram) {
        // Destructure so adding a field to Rdram forces this to be updated.
        let Rdram { mem, size, regs } = other;
        self.mem.clone_from(mem);
        self.size = *size;
        self.regs = *regs;
    }
}

pub fn read_mem_fast(
    device: &device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    let masked_address = address as usize & RDRAM_MASK;
    u32::from_ne_bytes(
        device
            .rdram
            .mem
            .get(masked_address..masked_address + 4)
            .unwrap_or(&[0; 4])
            .try_into()
            .unwrap_or_default(),
    )
}

pub fn read_mem(
    device: &mut device::Device,
    address: u64,
    access_size: device::memory::AccessSize,
) -> u32 {
    device::cop0::add_cycles(
        device,
        match access_size {
            device::memory::AccessSize::Word => const { rdram_cycles_per_word(4) },
            device::memory::AccessSize::Dword => const { rdram_cycles_per_word(8) },
            device::memory::AccessSize::Dcache => const { rdram_cycles_per_word(16) },
            device::memory::AccessSize::Icache => const { rdram_cycles_per_word(32) },
            device::memory::AccessSize::None => unreachable!(),
        },
    );
    let masked_address = address as usize & RDRAM_MASK;

    ui::video::check_rdram_read(masked_address as u32, 4);

    u32::from_ne_bytes(
        device
            .rdram
            .mem
            .get(masked_address..masked_address + 4)
            .unwrap_or(&[0; 4])
            .try_into()
            .unwrap_or_default(),
    )
}

pub fn write_mem(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    let masked_address = address as usize & RDRAM_MASK;

    ui::video::check_rdram_write(masked_address as u32, 4);

    let mut data = u32::from_ne_bytes(
        device
            .rdram
            .mem
            .get(masked_address..masked_address + 4)
            .unwrap_or(&[0; 4])
            .try_into()
            .unwrap_or_default(),
    );
    device::memory::masked_write_32(&mut data, value, mask);
    device
        .rdram
        .mem
        .get_mut(masked_address..masked_address + 4)
        .unwrap_or(&mut [0; 4])
        .copy_from_slice(&data.to_ne_bytes());
}

pub fn set_write_handler(
    device: &mut device::Device,
    handler: fn(&mut device::Device, u64, u32, u32),
) {
    for item in device
        .memory
        .memory_map_write
        .iter_mut()
        .take(0x3F00000 >> 16)
    {
        *item = handler;
    }
}

pub fn write_mem_repeat(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    if mask != 0xFFFFFFFF {
        panic!("RDRAM write_mem_repeat called with mask {:#x}", mask);
    }

    let length = ((device.mi.regs[device::mi::MI_INIT_MODE_REG] & device::mi::MI_INIT_LENGTH_MASK)
        + 1) as usize;
    let masked_address = address as usize & RDRAM_MASK;

    ui::video::check_rdram_write(masked_address as u32, length.next_multiple_of(4) as u32);

    for offset in (0..length).step_by(4) {
        let remaining = length - offset;
        let word_mask = if remaining >= 4 {
            0xFFFFFFFF
        } else {
            0xFFFFFFFF << (8 * (4 - remaining))
        };
        if let Some(word) = device
            .rdram
            .mem
            .get_mut(masked_address + offset..masked_address + offset + 4)
        {
            let mut data = u32::from_ne_bytes(word.try_into().unwrap());
            device::memory::masked_write_32(&mut data, value, word_mask);
            word.copy_from_slice(&data.to_ne_bytes());
        }
    }

    device.mi.regs[device::mi::MI_INIT_MODE_REG] &= !device::mi::MI_INIT_MODE;
    set_write_handler(device, write_mem);
}

pub fn read_regs(
    device: &mut device::Device,
    address: u64,
    access_size: device::memory::AccessSize,
) -> u32 {
    // register reads use the same RDRAM transfer as memory (n64-systembench: RDRAM U64R)
    device::cop0::add_cycles(
        device,
        match access_size {
            device::memory::AccessSize::Dword => const { rdram_cycles_per_word(8) },
            _ => const { rdram_cycles_per_word(4) },
        },
    );
    let chip_id = (address >> 13) & 3;
    let reg = (address & 0x3FF) >> 2;
    match reg as usize {
        RDRAM_MODE_REG => device.pi.regs[reg as usize] ^ 0xc0c0c0c0,
        0x80 => 0x00000000, //Row, needed for libdragon
        _ => device.rdram.regs[chip_id as usize][reg as usize],
    }
}

pub fn write_regs(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    let chip_id = (address >> 13) & 3;
    let reg = (address & 0x3FF) >> 2;
    device::memory::masked_write_32(
        &mut device.rdram.regs[chip_id as usize][reg as usize],
        value,
        mask,
    )
}

pub fn init(device: &mut device::Device) {
    let alignment = 64 * 1024;
    let layout =
        Layout::from_size_align(device.rdram.size as usize, alignment).expect("Invalid layout");
    let ptr = unsafe { alloc_zeroed(layout) };
    device.rdram.mem =
        unsafe { Vec::from_raw_parts(ptr, device.rdram.size as usize, device.rdram.size as usize) };

    retroachievements::set_rdram(device.rdram.mem.as_ptr(), device.rdram.size as usize);

    // hack, skip RDRAM initialization
    device
        .rdram
        .mem
        .get_mut(0x318..0x318 + 4)
        .unwrap_or(&mut [0; 4])
        .copy_from_slice(&device.rdram.size.to_ne_bytes());
    // hack, skip RDRAM initialization
    device
        .rdram
        .mem
        .get_mut(0x3f0..0x3f0 + 4)
        .unwrap_or(&mut [0; 4])
        .copy_from_slice(&device.rdram.size.to_ne_bytes());

    device.ri.regs[device::ri::RI_MODE_REG] = 0x0e;
    device.ri.regs[device::ri::RI_CONFIG_REG] = 0x40;
}

pub const fn rdram_calculate_cycles(length: u64) -> u64 {
    31 + (length / 3) // https://hcs64.com/dma.html, https://github.com/rasky/n64-systembench
}

const fn rdram_cycles_per_word(access_size: u64) -> u64 {
    rdram_calculate_cycles(access_size) / (access_size / 4)
}
