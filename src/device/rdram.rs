use crate::device;
use crate::ui;
use std::alloc::{Layout, alloc_zeroed};

//const RDRAM_CONFIG_REG: u32 = 0;
//const RDRAM_DEVICE_ID_REG: u32 = 1;
//const RDRAM_DELAY_REG: u32 = 2;
const RDRAM_MODE_REG: u32 = 3;
//const RDRAM_REF_INTERVAL_REG: u32 = 4;
//const RDRAM_REF_ROW_REG: u32 = 5;
//const RDRAM_RAS_INTERVAL_REG: u32 = 6;
//const RDRAM_MIN_INTERVAL_REG: u32 = 7;
//const RDRAM_ADDR_SELECT_REG: u32 = 8;
//const RDRAM_DEVICE_MANUF_REG: u32 = 9;
pub const RDRAM_REGS_COUNT: u32 = 10;

pub const RDRAM_MASK: usize = 0xFFFFFF;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Rdram {
    pub mem: Vec<u8>,
    pub size: u32,
    pub regs: [[u32; RDRAM_REGS_COUNT as usize]; 4],
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
        rdram_calculate_cycles(access_size as u64) / (access_size as u64 / 4),
    );
    let masked_address = address as usize & RDRAM_MASK;

    ui::video::check_framebuffers(masked_address as u32, 4);

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
    ui::video::check_framebuffers(address as u32, 4);

    let mut data = u32::from_ne_bytes(
        device
            .rdram
            .mem
            .get(address as usize..(address + 4) as usize)
            .unwrap_or(&[0; 4])
            .try_into()
            .unwrap_or_default(),
    );
    device::memory::masked_write_32(&mut data, value, mask);
    device
        .rdram
        .mem
        .get_mut(address as usize..(address + 4) as usize)
        .unwrap_or(&mut [0; 4])
        .copy_from_slice(&data.to_ne_bytes());
}

pub fn write_mem_repeat(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    if mask != 0xFFFFFFFF {
        panic!("RDRAM write_mem_repeat called with mask {:#x}", mask);
    }

    let repeat_length = (device.mi.regs[device::mi::MI_INIT_MODE_REG as usize]
        & device::mi::MI_INIT_LENGTH_MASK)
        + 1;

    if !repeat_length.is_multiple_of(4) {
        panic!(
            "RDRAM write_mem_repeat called with non-word-aligned length {}",
            repeat_length
        );
    }

    ui::video::check_framebuffers(address as u32, repeat_length);

    for i in 0..(repeat_length / 4) {
        device
            .rdram
            .mem
            .get_mut(
                (address as usize + (i * 4) as usize)..(address as usize + (i * 4) as usize + 4),
            )
            .unwrap_or(&mut [0; 4])
            .copy_from_slice(&value.to_ne_bytes());
    }

    device.mi.regs[device::mi::MI_INIT_MODE_REG as usize] &= !device::mi::MI_INIT_MODE;
    for i in 0..(0x3F00000 >> 16) {
        device.memory.memory_map_write[i] = device::rdram::write_mem;
    }
}

pub fn read_regs(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    device::cop0::add_cycles(device, 20);
    let chip_id = (address >> 13) & 3;
    let reg = (address & 0x3FF) >> 2;
    match reg as u32 {
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

    device.ri.regs[device::ri::RI_MODE_REG as usize] = 0x0e;
    device.ri.regs[device::ri::RI_CONFIG_REG as usize] = 0x40;
}

pub fn rdram_calculate_cycles(length: u64) -> u64 {
    31 + (length / 3) // https://hcs64.com/dma.html, https://github.com/rasky/n64-systembench
}
