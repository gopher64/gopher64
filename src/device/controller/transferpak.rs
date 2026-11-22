use crate::device;
use device::controller::PAK_CHUNK_SIZE;

#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransferPak {
    pub enabled: bool,
    pub reset_state: u8,
    pub bank: u16,
    pub cart: device::controller::gbcart::GbCart,
}

impl TransferPak {
    /// Clone everything except the GB cart ROM; `self.cart.rom` is left untouched.
    pub fn clone_state_from(&mut self, other: &TransferPak) {
        let TransferPak {
            enabled,
            reset_state,
            bank,
            cart,
        } = other;
        self.enabled = *enabled;
        self.reset_state = *reset_state;
        self.bank = *bank;
        self.cart.clone_state_from(cart);
    }
}

pub fn read(
    device: &mut device::Device,
    channel: usize,
    address: u16,
    data: &mut [u8; PAK_CHUNK_SIZE],
) {
    let pak = &mut device.transferpaks[channel];

    match address >> 12 {
        0x8 => {
            let value = if pak.enabled { 0x84 } else { 0x00 };
            data.fill(value);
            return;
        }
        _ => {
            if !pak.enabled {
                data.fill(0);
                return;
            }
        }
    }

    match address >> 12 {
        0xB => {
            let mut value = 0;
            if pak.cart.enabled {
                value |= 1 << 0;
            }
            value |= (pak.reset_state & 3) << 2;
            if pak.enabled {
                value |= 1 << 7;
            }

            if pak.cart.enabled && pak.reset_state == 3 {
                pak.reset_state = 2
            } else if !pak.cart.enabled && pak.reset_state == 2 {
                pak.reset_state = 1
            } else if !pak.cart.enabled && pak.reset_state == 1 {
                pak.reset_state = 0
            }
            data.fill(value);
        }
        0xC..=0xF => {
            device::controller::gbcart::read(
                data,
                &mut pak.cart,
                0x4000 * pak.bank + (address & 0x3fff),
                0,
                PAK_CHUNK_SIZE,
            );
        }
        _ => {
            eprintln!("unknown transfer pak read {:x}", address >> 12);
        }
    }
}

pub fn write(device: &mut device::Device, channel: usize, address: u16, data: &[u8]) {
    let pak = &mut device.transferpaks[channel];

    let value = *data.last().unwrap();
    match address >> 12 {
        0x8 => {
            match value {
                0xFE => {
                    pak.enabled = false;
                }
                0x84 => {
                    if !pak.enabled {
                        pak.bank = 3;
                        pak.cart.enabled = false;
                        pak.reset_state = 0;
                    }
                    pak.enabled = true;
                }
                _ => eprintln!("unknown transfer pak value"),
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
                if !pak.cart.enabled {
                    pak.reset_state = 3;
                    pak.cart.rom_bank = 1;
                    pak.cart.ram_bank = 0;
                    pak.cart.ram_enabled = false;
                    pak.cart.cart_type =
                        device::controller::gbcart::get_cart_type(pak.cart.rom[0x147]);
                }
                pak.cart.enabled = true;
            } else {
                pak.cart.enabled = false;
            }
        }
        0xC..=0xF => {
            device::controller::gbcart::write(
                data,
                &mut pak.cart,
                0x4000 * pak.bank + (address & 0x3fff),
                0,
                data.len(),
                device.vi.elapsed_time as i64,
            );
        }
        _ => {
            eprintln!("unknown transfer pak write {:x}", address >> 12);
        }
    }
}
