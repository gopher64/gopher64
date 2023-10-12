use crate::device;

pub fn read(device: &mut device::Device, channel: usize, address: u16, data: usize, size: usize) {
    if address < 0x8000 {}
}

pub fn write(device: &mut device::Device, channel: usize, address: u16, data: usize, size: usize) {}
