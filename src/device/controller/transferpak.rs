use crate::device;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct TransferPak {
    pub enabled: bool,
    pub cart_enabled: bool,
    pub reset_state: u8,
    pub bank: u16,
    #[serde(skip)]
    pub rom: Vec<u8>,
    pub ram: Vec<u8>,
    pub cart_type: device::controller::gbcart::CartType,
    pub ram_enabled: bool,
    pub ram_bank: u16,
}

pub fn read(device: &mut device::Device, channel: usize, address: u16, data: usize, size: usize) {
    let pak = &mut device.transferpaks[channel];

    match address >> 12 {
        0x8 => {
            let value = if pak.enabled { 0x84 } else { 0x00 };
            for i in 0..size {
                device.pif.ram[data + i] = value;
            }
            return;
        }
        _ => {
            if !pak.enabled {
                for i in 0..size {
                    device.pif.ram[data + i] = 0x00;
                }
                return;
            }
        }
    }

    match address >> 12 {
        0xB => {
            let mut value = 0;
            if pak.cart_enabled {
                value |= 1 << 0;
            }
            value |= (pak.reset_state & 3) << 2;
            if pak.enabled {
                value |= 1 << 7;
            }

            if pak.cart_enabled && pak.reset_state == 3 {
                pak.reset_state = 2
            } else if !pak.cart_enabled && pak.reset_state == 2 {
                pak.reset_state = 1
            } else if !pak.cart_enabled && pak.reset_state == 1 {
                pak.reset_state = 0
            }
            for i in 0..size {
                device.pif.ram[data + i] = value;
            }
        }
        0xC..=0xF => {
            device::controller::gbcart::read(
                &mut device.pif.ram,
                pak,
                (address & 0x3fff) | ((pak.bank & 0x3) * 0x4000),
                data,
                size,
            );
        }
        _ => {
            panic!("unknown transfer pak read {:x}", address >> 12);
        }
    }
}

pub fn write(device: &mut device::Device, channel: usize, address: u16, data: usize, size: usize) {
    let pak = &mut device.transferpaks[channel];

    let value = device.pif.ram[data + size - 1];
    match address >> 12 {
        0x8 => {
            match value {
                0xFE => {
                    pak.enabled = false;
                }
                0x84 => {
                    if !pak.enabled {
                        pak.bank = 0;
                        pak.cart_enabled = false;
                        pak.reset_state = 0;
                    }
                    pak.enabled = true;
                }
                _ => panic!("unknown transfer pak value"),
            }
            return;
        }
        _ => {
            if !pak.enabled {
                return;
            }
        }
    }

    match address >> 12 {
        0xA => {
            pak.bank = value as u16;
            if pak.bank > 3 {
                pak.bank = 0;
            }
        }
        0xB => {
            if value & 1 != 0 {
                if !pak.cart_enabled {
                    pak.reset_state = 3;
                    pak.cart_type = device::controller::gbcart::get_cart_type(pak.rom[0x147]);
                }
                pak.cart_enabled = true;
            } else {
                pak.cart_enabled = false;
            }
        }
        0xC..=0xF => {
            device::controller::gbcart::write(
                &mut device.pif.ram,
                pak,
                (address & 0x3fff) | ((pak.bank & 0x3) * 0x4000),
                data,
                size,
            );
        }
        _ => {
            panic!("unknown transfer pak write {:x}", address >> 12);
        }
    }
}
