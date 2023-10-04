use crate::device;

pub fn read_mem(
    _device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    let value = (address & 0xFFFF) as u32;
    return value | (value << 16);
}

pub fn write_mem(_device: &mut device::Device, _address: u64, _value: u32, _mask: u32) {}
