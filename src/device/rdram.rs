use crate::device;
use std::alloc::{alloc_zeroed, Layout};

pub const RDRAM_MASK: usize = 0xFFFFFF;

pub const RDRAM_SIZE: usize = 0x800000; // 8MB

pub struct Rdram {
    pub mem: Vec<u8>,
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
    if address < RDRAM_SIZE as u64 {
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
    if address < RDRAM_SIZE as u64 {
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
    _device: &mut device::Device,
    _address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    panic!("rdram read reg");
}

pub fn write_regs(_device: &mut device::Device, _address: u64, _value: u32, _mask: u32) {
    panic!("rdram write reg");
}

pub fn init(device: &mut device::Device) -> (*mut u8, usize) {
    let alignment = 64 * 1024;
    let layout = Layout::from_size_align(RDRAM_SIZE, alignment).expect("Invalid layout");
    let ptr = unsafe { alloc_zeroed(layout) };
    device.rdram.mem = unsafe { Vec::from_raw_parts(ptr, RDRAM_SIZE, RDRAM_SIZE) };

    // hack, skip RDRAM initialization
    let data: u32 = RDRAM_SIZE as u32;
    device.rdram.mem[device.cart.rdram_size_offset..device.cart.rdram_size_offset + 4]
        .copy_from_slice(&data.to_ne_bytes());
    (ptr, RDRAM_SIZE)
}

pub fn rdram_calculate_cycles(length: u64) -> u64 {
    31 + (length / 3)// https://hcs64.com/dma.html, https://github.com/rasky/n64-systembench
}
