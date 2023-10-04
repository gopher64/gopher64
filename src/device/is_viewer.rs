use crate::device;
use std::str;

pub const IS_VIEWER_MASK: usize = 0xFFFF;

pub fn read_mem(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    let masked_address = address as usize & IS_VIEWER_MASK;
    return u32::from_be_bytes(
        device.cart.is_viewer_buffer[masked_address..masked_address + 4]
            .try_into()
            .unwrap(),
    );
}

pub fn write_mem(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    let masked_address = address as usize & IS_VIEWER_MASK;
    if masked_address == 0x14 {
        let length = (value & mask) as u64;
        let data =
            str::from_utf8(&device.cart.is_viewer_buffer[0x20 as usize..(0x20 + length) as usize])
                .unwrap();
        print!("{}", data);
    } else {
        let mut data = u32::from_be_bytes(
            device.cart.is_viewer_buffer[masked_address..masked_address + 4]
                .try_into()
                .unwrap(),
        );
        device::memory::masked_write_32(&mut data, value, mask);
        device.cart.is_viewer_buffer[masked_address..masked_address + 4]
            .copy_from_slice(&data.to_be_bytes());
    }
}
