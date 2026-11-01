use crate::device;
#[cfg(target_os = "android")]
use crate::ui;

use std::io::{Read, Write};
const MBC3_RTC_SECONDS: usize = 0;
const MBC3_RTC_MINUTES: usize = 1;
const MBC3_RTC_HOURS: usize = 2;
const MBC3_RTC_DAYS_L: usize = 3;
const MBC3_RTC_DAYS_H: usize = 4;
const MBC3_RTC_REGS_COUNT: usize = 5;

#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
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
    pub rtc_regs: [u8; MBC3_RTC_REGS_COUNT],
    pub rtc_regs_latch: [u8; MBC3_RTC_REGS_COUNT],
    pub last_elapsed_time: i64,
    pub rtc_base_time: i64,
}

#[derive(Default, Copy, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum CartType {
    #[default]
    None,
    MBC1RamBatt,
    MBC3RamBatt,
    MBC3RamBattRtc,
    MBC5RamBatt,
}

pub fn read_file(path: &str) -> Option<Vec<u8>> {
    #[cfg(target_os = "android")]
    let mut file = ui::android::get_file_from_uri(&std::path::PathBuf::from(path))?;
    #[cfg(not(target_os = "android"))]
    let mut file = std::fs::File::open(path).ok()?;

    let mut contents = vec![];
    file.read_to_end(&mut contents).ok()?;
    Some(contents)
}

pub fn init(gb_cart: &mut device::controller::gbcart::GbCart, rom: &[u8], ram: &[u8]) {
    gb_cart.rom = rom.to_vec();
    gb_cart.rtc_base_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let offset = match ram.len() % 8192 {
        44 => {
            gb_cart.ram = ram[..ram.len() - 44].to_vec();
            ram.len() - 44
        }
        48 => {
            gb_cart.ram = ram[..ram.len() - 48].to_vec();
            ram.len() - 48
        }
        _ => {
            gb_cart.ram = ram.to_vec();
            return;
        }
    };

    gb_cart.rtc_regs[MBC3_RTC_SECONDS] =
        u32::from_le_bytes(ram[offset..offset + 4].try_into().unwrap()) as u8;
    gb_cart.rtc_regs[MBC3_RTC_MINUTES] =
        u32::from_le_bytes(ram[offset + 4..offset + 8].try_into().unwrap()) as u8;
    gb_cart.rtc_regs[MBC3_RTC_HOURS] =
        u32::from_le_bytes(ram[offset + 8..offset + 12].try_into().unwrap()) as u8;
    gb_cart.rtc_regs[MBC3_RTC_DAYS_L] =
        u32::from_le_bytes(ram[offset + 12..offset + 16].try_into().unwrap()) as u8;
    gb_cart.rtc_regs[MBC3_RTC_DAYS_H] =
        u32::from_le_bytes(ram[offset + 16..offset + 20].try_into().unwrap()) as u8;

    gb_cart.rtc_regs_latch[MBC3_RTC_SECONDS] =
        u32::from_le_bytes(ram[offset + 20..offset + 24].try_into().unwrap()) as u8;
    gb_cart.rtc_regs_latch[MBC3_RTC_MINUTES] =
        u32::from_le_bytes(ram[offset + 24..offset + 28].try_into().unwrap()) as u8;
    gb_cart.rtc_regs_latch[MBC3_RTC_HOURS] =
        u32::from_le_bytes(ram[offset + 28..offset + 32].try_into().unwrap()) as u8;
    gb_cart.rtc_regs_latch[MBC3_RTC_DAYS_L] =
        u32::from_le_bytes(ram[offset + 32..offset + 36].try_into().unwrap()) as u8;
    gb_cart.rtc_regs_latch[MBC3_RTC_DAYS_H] =
        u32::from_le_bytes(ram[offset + 36..offset + 40].try_into().unwrap()) as u8;

    let saved_time = if ram.len() - offset == 48 {
        i64::from_le_bytes(ram[offset + 40..offset + 48].try_into().unwrap())
    } else {
        u32::from_le_bytes(ram[offset + 40..offset + 44].try_into().unwrap()) as i64
    };
    if saved_time > 0 {
        advance_rtc(&mut gb_cart.rtc_regs, gb_cart.rtc_base_time - saved_time);
    }
}

pub fn save(
    gb_cart: &mut device::controller::gbcart::GbCart,
    elapsed_time: i64,
    ram_path: &std::string::String,
) {
    if cfg!(target_os = "android") {
        return;
    }
    update_rtc_regs(gb_cart, elapsed_time);

    if let Ok(file) = std::fs::File::create(ram_path) {
        let mut f = std::io::BufWriter::new(file);
        f.write_all(&gb_cart.ram).unwrap();

        if gb_cart.cart_type == CartType::MBC3RamBattRtc {
            f.write_all(&(gb_cart.rtc_regs[MBC3_RTC_SECONDS] as u32).to_le_bytes())
                .unwrap();
            f.write_all(&(gb_cart.rtc_regs[MBC3_RTC_MINUTES] as u32).to_le_bytes())
                .unwrap();
            f.write_all(&(gb_cart.rtc_regs[MBC3_RTC_HOURS] as u32).to_le_bytes())
                .unwrap();
            f.write_all(&(gb_cart.rtc_regs[MBC3_RTC_DAYS_L] as u32).to_le_bytes())
                .unwrap();
            f.write_all(&(gb_cart.rtc_regs[MBC3_RTC_DAYS_H] as u32).to_le_bytes())
                .unwrap();

            f.write_all(&(gb_cart.rtc_regs_latch[MBC3_RTC_SECONDS] as u32).to_le_bytes())
                .unwrap();
            f.write_all(&(gb_cart.rtc_regs_latch[MBC3_RTC_MINUTES] as u32).to_le_bytes())
                .unwrap();
            f.write_all(&(gb_cart.rtc_regs_latch[MBC3_RTC_HOURS] as u32).to_le_bytes())
                .unwrap();
            f.write_all(&(gb_cart.rtc_regs_latch[MBC3_RTC_DAYS_L] as u32).to_le_bytes())
                .unwrap();
            f.write_all(&(gb_cart.rtc_regs_latch[MBC3_RTC_DAYS_H] as u32).to_le_bytes())
                .unwrap();

            // rtc_regs are now in sync with rtc_base_time + elapsed_time
            let timestamp = gb_cart.rtc_base_time + elapsed_time;
            f.write_all(&timestamp.to_le_bytes()).unwrap();

            f.flush().unwrap();
        }
    } else {
        eprintln!("Error saving TransferPak RAM to {ram_path}");
    }
}

fn update_rtc_regs(cart: &mut device::controller::gbcart::GbCart, elapsed_time: i64) {
    let diff = elapsed_time - cart.last_elapsed_time;
    cart.last_elapsed_time = elapsed_time;
    advance_rtc(&mut cart.rtc_regs, diff);
}

fn advance_rtc(regs: &mut [u8; MBC3_RTC_REGS_COUNT], diff: i64) {
    // bit 6 of DAYS_H is the halt flag
    if diff <= 0 || regs[MBC3_RTC_DAYS_H] & 0x40 != 0 {
        return;
    }

    let mut total = regs[MBC3_RTC_SECONDS] as i64
        + regs[MBC3_RTC_MINUTES] as i64 * 60
        + regs[MBC3_RTC_HOURS] as i64 * 3600
        + diff;
    regs[MBC3_RTC_SECONDS] = (total % 60) as u8;
    total /= 60;
    regs[MBC3_RTC_MINUTES] = (total % 60) as u8;
    total /= 60;
    regs[MBC3_RTC_HOURS] = (total % 24) as u8;
    total /= 24;

    let days =
        ((((regs[MBC3_RTC_DAYS_H] & 0x01) as i64) << 8) | regs[MBC3_RTC_DAYS_L] as i64) + total;

    regs[MBC3_RTC_DAYS_L] = (days & 0xff) as u8;
    regs[MBC3_RTC_DAYS_H] &= !0x01;
    regs[MBC3_RTC_DAYS_H] |= ((days >> 8) & 1) as u8;

    // set carry bit if days overflow
    if days > 511 {
        regs[MBC3_RTC_DAYS_H] |= 0x80;
    }
}

fn write_mbc1(
    pif_ram: &[u8],
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
    cart: &device::controller::gbcart::GbCart,
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
    pif_ram: &[u8],
    cart: &mut device::controller::gbcart::GbCart,
    address: u16,
    data: usize,
    size: usize,
    elapsed_time: i64,
) {
    let value = pif_ram[data + size - 1];
    if address < 0x2000 {
        cart.ram_enabled = value & 0xf == 0xa;
    } else if address < 0x4000 {
        let bank = value & if cart.rom.len() > 2097152 { 0xff } else { 0x7f };
        cart.rom_bank = bank as u32;
        if cart.rom_bank == 0 {
            cart.rom_bank = 1;
        }
    } else if address < 0x6000 {
        cart.ram_bank = (value & 0xf) as u16;
    } else if address < 0x8000 {
        if cart.cart_type == CartType::MBC3RamBattRtc && (value & 0x1) != 0 {
            update_rtc_regs(cart, elapsed_time);
            cart.rtc_regs_latch = cart.rtc_regs;
        }
    } else if (0xa000..0xc000).contains(&address) {
        if !cart.ram_enabled {
            return;
        }
        if cart.ram_bank < 0x8 {
            let banked_address = address - 0xA000 + (cart.ram_bank * 0x2000);
            cart.ram[banked_address as usize..banked_address as usize + size]
                .copy_from_slice(&pif_ram[data..data + size]);
        } else if cart.cart_type == CartType::MBC3RamBattRtc {
            cart.rtc_regs[cart.ram_bank as usize - 0x8] = value;
        }
    } else {
        panic!("Unsupported write address {address:x}");
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
            let banked_address = address - 0xA000 + (cart.ram_bank * 0x2000);
            pif_ram[data..data + size].copy_from_slice(
                cart.ram
                    .get(banked_address as usize..banked_address as usize + size)
                    .unwrap_or(&vec![0; size]),
            );
        } else if cart.cart_type == CartType::MBC3RamBattRtc {
            for i in 0..size {
                pif_ram[data + i] = cart.rtc_regs_latch[cart.ram_bank as usize - 0x8];
            }
        }
    } else {
        panic!("Unsupported read address {address:x}");
    }
}

fn write_mbc5(
    pif_ram: &[u8],
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
        eprintln!("Unknown MBC5 write address {address:x}");
    } else if (0xa000..0xc000).contains(&address) {
        if !cart.ram_enabled {
            return;
        }

        let banked_address = address - 0xA000 + (cart.ram_bank << 13);
        cart.ram[banked_address as usize..banked_address as usize + size]
            .copy_from_slice(&pif_ram[data..data + size]);
    } else {
        panic!("Unsupported write address {address:x}");
    }
}

fn read_mbc5(
    pif_ram: &mut [u8],
    cart: &device::controller::gbcart::GbCart,
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
        panic!("Unsupported read address {address:x}");
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
    pif_ram: &[u8],
    cart: &mut device::controller::gbcart::GbCart,
    address: u16,
    data: usize,
    size: usize,
    elapsed_time: i64,
) {
    if !cart.enabled {
        return;
    }
    match cart.cart_type {
        CartType::MBC1RamBatt => write_mbc1(pif_ram, cart, address, data, size),
        CartType::MBC3RamBatt => write_mbc3(pif_ram, cart, address, data, size, elapsed_time),
        CartType::MBC3RamBattRtc => write_mbc3(pif_ram, cart, address, data, size, elapsed_time),
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
        _ => panic!("Unsupported cart type {data:x}"),
    }
}
