use crate::device;

pub fn read(device: &mut device::Device, _channel: usize, address: u16, data: usize, size: usize) {
    let value: u8 = if (0x8000..0x9000).contains(&address) {
        0x80
    } else {
        0x00
    };

    for i in 0..size {
        device.pif.ram[data + i] = value;
    }
}

pub fn write(device: &mut device::Device, channel: usize, address: u16, data: usize, size: usize) {
    if address == 0xc000 {
        let rumble = device.pif.ram[data + size - 1];
        if device.netplay.is_none() {
            device::ui::input::set_rumble(&mut device.ui, channel, rumble);
        } else if device.netplay.as_ref().unwrap().player_number as usize == channel {
            device::ui::input::set_rumble(&mut device.ui, 0, rumble);
        }
    }
}
