use crate::device;

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
    pak: &mut device::controller::transferpak::TransferPak,
    address: u16,
    data: usize,
    size: usize,
) {
    let value = pif_ram[data + size - 1];
    if address < 0x2000 {
        pak.ram_enabled = value & 0xf == 0xa;
    } else if address < 0x4000 {
        let bank = value & 0x7f;
        if bank == 0 {
            pak.bank = 1;
        } else {
            pak.bank = bank as u16;
        }
    } else if address < 0x6000 {
        pak.ram_bank = value as u16;
    } else if address < 0x8000 {
        println!("MBC3 RTC latch")
    } else if (0xa000..0xc000).contains(&address) {
        if !pak.ram_enabled {
            return;
        }
        if pak.ram_bank < 0x8 {
            let banked_address = (pak.ram_bank << 13) | (address & 0x1FFF);
            for i in 0..size {
                pak.ram[banked_address as usize + i] = pif_ram[data + i];
            }
        } else {
            panic!("Unsupported ram bank {:x}", pak.ram_bank);
        }
    } else {
        panic!("Unsupported write address {:x}", address);
    }
}

fn read_mbc3(
    pif_ram: &mut [u8],
    pak: &mut device::controller::transferpak::TransferPak,
    address: u16,
    data: usize,
    size: usize,
) {
    if address < 0x4000 {
        for i in 0..size {
            pif_ram[data + i] = pak.rom[address as usize + i];
        }
    } else if address < 0x8000 {
        let banked_address = (pak.bank << 14) | (address & 0x3FFF);
        for i in 0..size {
            pif_ram[data + i] = pak.rom[banked_address as usize + i];
        }
    } else if address < 0xc000 {
        if !pak.ram_enabled {
            for i in 0..size {
                pif_ram[data + i] = 0xff;
            }
            return;
        }
        if pak.ram_bank < 0x8 {
            let banked_address = (pak.ram_bank << 13) | (address & 0x1FFF);
            for i in 0..size {
                pif_ram[data + i] = pak.ram[banked_address as usize + i];
            }
        } else {
            panic!("Unsupported ram bank {:x}", pak.ram_bank);
        }
    } else {
        panic!("Unsupported read address {:x}", address);
    }
}

fn write_mbc5(
    pif_ram: &mut [u8],
    pak: &mut device::controller::transferpak::TransferPak,
    address: u16,
    data: usize,
    size: usize,
) {
    let value = pif_ram[data + size - 1];
    if address < 0x2000 {
        pak.ram_enabled = value & 0xf == 0xa;
    } else if address < 0x4000 {
        println!("Unknown MBC5 write address {:x}", address);
    } else if address < 0x6000 {
        pak.ram_bank = (value & 0xf) as u16;
    } else if address < 0xa000 {
        println!("Unknown MBC5 write address {:x}", address);
    } else if (0xa000..0xc000).contains(&address) {
        if !pak.ram_enabled {
            return;
        }

        let banked_address = (pak.ram_bank << 13) | (address & 0x1FFF);
        for i in 0..size {
            pak.ram[banked_address as usize + i] = pif_ram[data + i];
        }
    } else {
        panic!("Unsupported write address {:x}", address);
    }
}

fn read_mbc5(
    pif_ram: &mut [u8],
    pak: &mut device::controller::transferpak::TransferPak,
    address: u16,
    data: usize,
    size: usize,
) {
    if address < 0x4000 {
        for i in 0..size {
            pif_ram[data + i] = pak.rom[address as usize + i];
        }
    } else if address < 0x8000 {
        let banked_address = (pak.bank << 14) | (address & 0x3FFF);
        for i in 0..size {
            pif_ram[data + i] = pak.rom[banked_address as usize + i];
        }
    } else if (0xa000..0xc000).contains(&address) {
        if !pak.ram_enabled {
            for i in 0..size {
                pif_ram[data + i] = 0xff;
            }
            return;
        }

        let banked_address = (pak.ram_bank << 13) | (address & 0x1FFF);
        for i in 0..size {
            pif_ram[data + i] = pak.ram[banked_address as usize + i];
        }
    } else {
        panic!("Unsupported read address {:x}", address);
    }
}

pub fn read(
    pif_ram: &mut [u8],
    pak: &mut device::controller::transferpak::TransferPak,
    address: u16,
    data: usize,
    size: usize,
) {
    if !pak.cart_enabled {
        for i in 0..size {
            pif_ram[data + i] = 0x00;
        }
        return;
    }
    match pak.cart_type {
        CartType::MBC3RamBatt => read_mbc3(pif_ram, pak, address, data, size),
        CartType::MBC3RamBattRtc => read_mbc3(pif_ram, pak, address, data, size),
        CartType::MBC5RamBatt => read_mbc5(pif_ram, pak, address, data, size),
        _ => panic!("Unsupported cart type"),
    }
}

pub fn write(
    pif_ram: &mut [u8],
    pak: &mut device::controller::transferpak::TransferPak,
    address: u16,
    data: usize,
    size: usize,
) {
    if !pak.cart_enabled {
        return;
    }
    match pak.cart_type {
        CartType::MBC3RamBatt => write_mbc3(pif_ram, pak, address, data, size),
        CartType::MBC3RamBattRtc => write_mbc3(pif_ram, pak, address, data, size),
        CartType::MBC5RamBatt => write_mbc5(pif_ram, pak, address, data, size),
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
