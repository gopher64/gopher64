use crate::device;

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct GbCart {
    pub enabled: bool,
    #[serde(skip)]
    pub rom: Vec<u8>,
    pub ram: Vec<u8>,
    pub cart_type: device::controller::gbcart::CartType,
    pub ram_enabled: bool,
    pub ram_bank: u16,
    pub rom_bank: u16,
}

#[derive(Default, Copy, Clone, serde::Serialize, serde::Deserialize)]
pub enum CartType {
    #[default]
    None,
    MBC3RamBatt,
    MBC3RamBattRtc,
    MBC5RamBatt,
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
        if bank == 0 {
            cart.rom_bank = 1;
        } else {
            cart.rom_bank = bank as u16;
        }
    } else if address < 0x6000 {
        cart.ram_bank = value as u16;
    } else if address < 0x8000 {
        println!("MBC3 RTC latch")
    } else if (0xa000..0xc000).contains(&address) {
        if !cart.ram_enabled {
            return;
        }
        if cart.ram_bank < 0x8 {
            let banked_address = (cart.ram_bank << 13) | (address & 0x1FFF);
            for i in 0..size {
                cart.ram[banked_address as usize + i] = pif_ram[data + i];
            }
        } else {
            panic!("Unsupported ram bank {:x}", cart.ram_bank);
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
        for i in 0..size {
            pif_ram[data + i] = cart.rom[address as usize + i];
        }
    } else if address < 0x8000 {
        let banked_address = (cart.rom_bank << 14) | (address & 0x3FFF);
        for i in 0..size {
            pif_ram[data + i] = cart.rom[banked_address as usize + i];
        }
    } else if address < 0xc000 {
        if !cart.ram_enabled {
            for i in 0..size {
                pif_ram[data + i] = 0xff;
            }
            return;
        }
        if cart.ram_bank < 0x8 {
            let banked_address = (cart.ram_bank << 13) | (address & 0x1FFF);
            for i in 0..size {
                pif_ram[data + i] = cart.ram[banked_address as usize + i];
            }
        } else {
            panic!("Unsupported ram bank {:x}", cart.ram_bank);
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
    } else if address < 0x4000 {
        println!("Unknown MBC5 write address {:x}", address);
    } else if address < 0x6000 {
        cart.ram_bank = (value & 0xf) as u16;
    } else if address < 0xa000 {
        println!("Unknown MBC5 write address {:x}", address);
    } else if (0xa000..0xc000).contains(&address) {
        if !cart.ram_enabled {
            return;
        }

        let banked_address = (cart.ram_bank << 13) | (address & 0x1FFF);
        for i in 0..size {
            cart.ram[banked_address as usize + i] = pif_ram[data + i];
        }
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
        for i in 0..size {
            pif_ram[data + i] = cart.rom[address as usize + i];
        }
    } else if address < 0x8000 {
        let banked_address = (cart.rom_bank << 14) | (address & 0x3FFF);
        for i in 0..size {
            pif_ram[data + i] = cart.rom[banked_address as usize + i];
        }
    } else if (0xa000..0xc000).contains(&address) {
        if !cart.ram_enabled {
            for i in 0..size {
                pif_ram[data + i] = 0xff;
            }
            return;
        }

        let banked_address = (cart.ram_bank << 13) | (address & 0x1FFF);
        for i in 0..size {
            pif_ram[data + i] = cart.ram[banked_address as usize + i];
        }
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
        CartType::MBC3RamBatt => write_mbc3(pif_ram, cart, address, data, size),
        CartType::MBC3RamBattRtc => write_mbc3(pif_ram, cart, address, data, size),
        CartType::MBC5RamBatt => write_mbc5(pif_ram, cart, address, data, size),
        _ => panic!("Unsupported cart type"),
    }
}

pub fn get_cart_type(data: u8) -> CartType {
    match data {
        0x10 => CartType::MBC3RamBattRtc,
        0x13 => CartType::MBC3RamBatt,
        0x1b => CartType::MBC5RamBatt,
        _ => panic!("Unsupported cart type {:x}", data),
    }
}
