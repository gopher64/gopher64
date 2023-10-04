use crate::device;

pub fn read_mem(
    _device: &mut device::Device,
    _address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    panic!("sream read");
}

pub fn write_mem(_device: &mut device::Device, _address: u64, _value: u32, _mask: u32) {
    panic!("sram write");
}
