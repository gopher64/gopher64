use crate::device;
use device::controller::PAK_CHUNK_SIZE;

pub fn read(address: u16, data: &mut [u8; PAK_CHUNK_SIZE]) {
    let value: u8 = if (0x8000..0x9000).contains(&address) {
        0x80
    } else {
        0x00
    };

    data.fill(value);
}

pub fn write(device: &device::Device, channel: usize, address: u16, rumble: u8) {
    if address == 0xc000 {
        if let Some(netplay) = &device.netplay {
            if netplay.player_number == channel {
                device::ui::input::set_rumble(&device.ui, 0, rumble);
            }
        } else {
            device::ui::input::set_rumble(&device.ui, channel, rumble);
        }
    }
}
