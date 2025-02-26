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
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    let masked_address = address as usize & RDRAM_MASK;
    u32::from_ne_bytes(
        device.rdram.mem[masked_address..masked_address + 4]
            .try_into()
            .unwrap(),
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
    if address < device.rdram.size as u64 {
        ui::video::check_framebuffers(masked_address as u32);
        u32::from_ne_bytes(
            device.rdram.mem[masked_address..masked_address + 4]
                .try_into()
                .unwrap(),
        )
    } else {
        0
    }
}

pub fn write_mem(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    if address < device.rdram.size as u64 {
        ui::video::check_framebuffers(address as u32);
        let mut data = u32::from_ne_bytes(
            device.rdram.mem[address as usize..(address + 4) as usize]
                .try_into()
                .unwrap(),
        );
        device::memory::masked_write_32(&mut data, value, mask);
        device.rdram.mem[address as usize..(address + 4) as usize]
            .copy_from_slice(&data.to_ne_bytes());
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
    device.rdram.mem[0x318..0x318 + 4].copy_from_slice(&device.rdram.size.to_ne_bytes());
    // hack, skip RDRAM initialization
    device.rdram.mem[0x3f0..0x3f0 + 4].copy_from_slice(&device.rdram.size.to_ne_bytes());

    device.ri.regs[device::ri::RI_MODE_REG as usize] = 0x0e;
    device.ri.regs[device::ri::RI_CONFIG_REG as usize] = 0x40;
}

pub fn rdram_calculate_cycles(length: u64) -> u64 {
    31 + (length / 3) // https://hcs64.com/dma.html, https://github.com/rasky/n64-systembench
}
