use crate::device;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct GbCart {
    pub enabled: bool,
    #[serde(skip)]
    pub rom: Vec<u8>,
    pub ram: Vec<u8>,
    pub cart_type: device::controller::gbcart::CartType,
    pub ram_enabled: bool,
    pub mbc1_mode: bool,
    pub ram_bank: u16,
    pub rom_bank: u32,
    pub set_latch: bool,
    pub latch_second: u8,
    pub latch_minute: u8,
    pub latch_hour: u8,
    pub latch_day: u16,
}

#[derive(Default, Copy, Clone, serde::Serialize, serde::Deserialize)]
pub enum CartType {
    #[default]
    None,
    MBC1RamBatt,
    MBC3RamBatt,
    MBC3RamBattRtc,
    MBC5RamBatt,
}

fn write_mbc1(
    pif_ram: &mut [u8],
    cart: &mut device::controller::gbcart::GbCart,
    address: u16,
    data: usize,
    size: usize,
) {
    let value = pif_ram[data + size - 1];
    if address < 0x2000 {
        cart.ram_enabled = value & 0xf == 0xa;
    } else if address < 0x4000 {
        let bank = value & 0x1f;
        cart.rom_bank = bank as u32;
        if cart.rom_bank == 0 {
            cart.rom_bank = 1;
        }
    } else if address < 0x6000 {
        cart.ram_bank = (value & 0x3) as u16;
    } else if address < 0x8000 {
        cart.mbc1_mode = (value & 0x1) != 0;
    } else if (0xa000..0xc000).contains(&address) {
        if !cart.ram_enabled {
            return;
        }
        if cart.mbc1_mode {
            let banked_address = address - 0xA000 + (cart.ram_bank * 0x2000);
            cart.ram[banked_address as usize..banked_address as usize + size]
                .copy_from_slice(&pif_ram[data..data + size]);
        } else {
            let banked_address = address - 0xA000;
            cart.ram[banked_address as usize..banked_address as usize + size]
                .copy_from_slice(&pif_ram[data..data + size]);
        }
    }
}

fn read_mbc1(
    pif_ram: &mut [u8],
    cart: &mut device::controller::gbcart::GbCart,
    address: u16,
    data: usize,
    size: usize,
) {
    if address < 0x4000 {
        let banked_address = address & 0x3FFF;
        pif_ram[data..data + size]
            .copy_from_slice(&cart.rom[banked_address as usize..banked_address as usize + size]);
    } else if address < 0x8000 {
        let banked_address = address as u32 - 0x4000 + (cart.rom_bank * 0x4000);
        pif_ram[data..data + size]
            .copy_from_slice(&cart.rom[banked_address as usize..banked_address as usize + size]);
    } else if (0xa000..0xc000).contains(&address) {
        if !cart.ram_enabled {
            for i in 0..size {
                pif_ram[data + i] = 0xff;
            }
            return;
        }
        if cart.ram_bank > 3 {
            for i in 0..size {
                pif_ram[data + i] = 0;
            }
        } else {
            let banked_address = address - 0xA000 + (cart.ram_bank * 0x2000);
            pif_ram[data..data + size].copy_from_slice(
                &cart.ram[banked_address as usize..banked_address as usize + size],
            );
        }
    }
}

fn write_mbc3(
    pif_ram: &mut [u8],
    cart: &mut device::controller::gbcart::GbCart,
    address: u16,
    data: usize,
    size: usize,
) {
    let value = pif_ram[data + size - 1];
    if address < 0x2000 {
        cart.ram_enabled = value & 0xf == 0xa;
    } else if address < 0x4000 {
        let bank = value & 0x7f;
        cart.rom_bank = bank as u32;
        if cart.rom_bank == 0 {
            cart.rom_bank = 1;
        }
    } else if address < 0x6000 {
        cart.ram_bank = (value & 0xf) as u16;
    } else if address < 0x8000 {
        if !cart.set_latch && value != 0 {
            // not implemented
        }
        cart.set_latch = value != 0;
    } else if (0xa000..0xc000).contains(&address) {
        if !cart.ram_enabled {
            return;
        }
        if cart.ram_bank < 0x8 {
            let banked_address = address - 0xA000 + (cart.ram_bank * 0x2000);
            cart.ram[banked_address as usize..banked_address as usize + size]
                .copy_from_slice(&pif_ram[data..data + size]);
        } else {
            match cart.ram_bank {
                0x8 => {
                    cart.latch_second = value;
                }
                0x9 => {
                    cart.latch_minute = value;
                }
                0xA => {
                    cart.latch_hour = value;
                }
                0xB => {
                    cart.latch_day &= 0xFF00;
                    cart.latch_day |= value as u16;
                }
                0xC => {
                    cart.latch_day &= 0x00FF;
                    cart.latch_day |= (value as u16) << 8;
                }
                _ => {
                    panic!("Unsupported ram bank {:x}", cart.ram_bank);
                }
            }
        }
    } else {
        panic!("Unsupported write address {:x}", address);
    }
}

fn read_mbc3(
    pif_ram: &mut [u8],
    cart: &mut device::controller::gbcart::GbCart,
    address: u16,
    data: usize,
    size: usize,
) {
    if address < 0x4000 {
        let banked_address = address & 0x3FFF;
        pif_ram[data..data + size]
            .copy_from_slice(&cart.rom[banked_address as usize..banked_address as usize + size]);
    } else if address < 0x8000 {
        let banked_address = address as u32 - 0x4000 + (cart.rom_bank * 0x4000);
        pif_ram[data..data + size]
            .copy_from_slice(&cart.rom[banked_address as usize..banked_address as usize + size]);
    } else if (0xa000..0xc000).contains(&address) {
        if !cart.ram_enabled {
            for i in 0..size {
                pif_ram[data + i] = 0xff;
            }
            return;
        }
        if cart.ram_bank < 0x8 {
            if cart.ram_bank > 3 {
                for i in 0..size {
                    pif_ram[data + i] = 0;
                }
            } else {
                let banked_address = address - 0xA000 + (cart.ram_bank * 0x2000);
                pif_ram[data..data + size].copy_from_slice(
                    &cart.ram[banked_address as usize..banked_address as usize + size],
                );
            }
        } else {
            let latch = match cart.ram_bank {
                0x8 => cart.latch_second,
                0x9 => cart.latch_minute,
                0xA => cart.latch_hour,
                0xB => cart.latch_day as u8,
                0xC => (cart.latch_day >> 8) as u8,
                _ => {
                    panic!("Unsupported ram bank {:x}", cart.ram_bank);
                }
            };
            for i in 0..size {
                pif_ram[data + i] = latch;
            }
        }
    } else {
        panic!("Unsupported read address {:x}", address);
    }
}

fn write_mbc5(
    pif_ram: &mut [u8],
    cart: &mut device::controller::gbcart::GbCart,
    address: u16,
    data: usize,
    size: usize,
) {
    let value = pif_ram[data + size - 1];
    if address < 0x2000 {
        cart.ram_enabled = value & 0xf == 0xa;
    } else if address < 0x3000 {
        cart.rom_bank &= 0xff00;
        cart.rom_bank |= value as u32;
    } else if address < 0x4000 {
        cart.rom_bank &= 0x00ff;
        cart.rom_bank |= ((value & 0x1) as u32) << 8;
    } else if address < 0x6000 {
        cart.ram_bank = (value & 0xf) as u16;
    } else if address < 0xa000 {
        println!("Unknown MBC5 write address {:x}", address);
    } else if (0xa000..0xc000).contains(&address) {
        if !cart.ram_enabled {
            return;
        }

        let banked_address = address - 0xA000 + (cart.ram_bank << 13);
        cart.ram[banked_address as usize..banked_address as usize + size]
            .copy_from_slice(&pif_ram[data..data + size]);
    } else {
        panic!("Unsupported write address {:x}", address);
    }
}

fn read_mbc5(
    pif_ram: &mut [u8],
    cart: &mut device::controller::gbcart::GbCart,
    address: u16,
    data: usize,
    size: usize,
) {
    if address < 0x4000 {
        let banked_address = address & 0x3FFF;
        pif_ram[data..data + size]
            .copy_from_slice(&cart.rom[banked_address as usize..banked_address as usize + size]);
    } else if address < 0x8000 {
        let banked_address = address as u32 - 0x4000 + (cart.rom_bank << 14);
        pif_ram[data..data + size]
            .copy_from_slice(&cart.rom[banked_address as usize..banked_address as usize + size]);
    } else if (0xa000..0xc000).contains(&address) {
        if !cart.ram_enabled {
            for i in 0..size {
                pif_ram[data + i] = 0xff;
            }
            return;
        }

        let banked_address = address - 0xA000 + (cart.ram_bank << 13);
        pif_ram[data..data + size]
            .copy_from_slice(&cart.ram[banked_address as usize..banked_address as usize + size]);
    } else {
        panic!("Unsupported read address {:x}", address);
    }
}

pub fn read(
    pif_ram: &mut [u8],
    cart: &mut device::controller::gbcart::GbCart,
    address: u16,
    data: usize,
    size: usize,
) {
    if !cart.enabled {
        for i in 0..size {
            pif_ram[data + i] = 0x00;
        }
        return;
    }
    match cart.cart_type {
        CartType::MBC1RamBatt => read_mbc1(pif_ram, cart, address, data, size),
        CartType::MBC3RamBatt => read_mbc3(pif_ram, cart, address, data, size),
        CartType::MBC3RamBattRtc => read_mbc3(pif_ram, cart, address, data, size),
        CartType::MBC5RamBatt => read_mbc5(pif_ram, cart, address, data, size),
        _ => panic!("Unsupported cart type"),
    }
}

pub fn write(
    pif_ram: &mut [u8],
    cart: &mut device::controller::gbcart::GbCart,
    address: u16,
    data: usize,
    size: usize,
) {
    if !cart.enabled {
        return;
    }
    match cart.cart_type {
        CartType::MBC1RamBatt => write_mbc1(pif_ram, cart, address, data, size),
        CartType::MBC3RamBatt => write_mbc3(pif_ram, cart, address, data, size),
        CartType::MBC3RamBattRtc => write_mbc3(pif_ram, cart, address, data, size),
        CartType::MBC5RamBatt => write_mbc5(pif_ram, cart, address, data, size),
        _ => panic!("Unsupported cart type"),
    }
}

pub fn get_cart_type(data: u8) -> CartType {
    match data {
        0x03 => CartType::MBC1RamBatt,
        0x10 => CartType::MBC3RamBattRtc,
        0x13 => CartType::MBC3RamBatt,
        0x1b => CartType::MBC5RamBatt,
        _ => panic!("Unsupported cart type {:x}", data),
    }
}
