use crate::device;

pub fn read(device: &mut device::Device, _channel: usize, address: u16, data: usize, size: usize) {
    let value: u8;

    if (address >= 0x8000) && (address < 0x9000) {
        value = 0x80;
    } else {
        value = 0x00;
    }
    for i in 0..size {
        device.pif.ram[data + i] = value;
    }
}

pub fn write(device: &mut device::Device, _channel: usize, address: u16, data: usize, size: usize) {
    if address == 0xc000 {
        let rumble = device.pif.ram[data + size - 1];
        println!("Rumble: {}", rumble);
    }
}
