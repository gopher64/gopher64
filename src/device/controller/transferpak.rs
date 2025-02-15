use core::panic;

use crate::device;

pub fn read(
    _device: &mut device::Device,
    _channel: usize,
    mut address: u16,
    _data: usize,
    _size: usize,
) {
    address &= 0x7FFF;
    match address {
        _ => {
            panic!("Transfer Pak: Unimplemented read at address: {:X}", address)
        }
    }
}

pub fn write(
    _device: &mut device::Device,
    _channel: usize,
    mut address: u16,
    _data: usize,
    _size: usize,
) {
    address &= 0x7FFF;
    match address {
        _ => {
            panic!(
                "Transfer Pak: Unimplemented write at address: {:X}",
                address
            )
        }
    }
}
