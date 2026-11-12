use crate::{device, ui};

const SRAM_MASK: usize = 0xFFFF;
const SRAM_BANK_SIZE: usize = 0x8000;
pub const SRAM_SIZE: usize = 0x8000;
const SRAM_768K_SIZE: usize = SRAM_BANK_SIZE * 3;
const SRAM_1M_SIZE: usize = 0x20000;
pub const FLASHRAM_SIZE: usize = 0x20000;

// Emulates a Macronix MX29L1100
const FLASHRAM_SILICON_ID: [u8; 8] = [0x11, 0x11, 0x80, 0x01, 0x00, 0xc2, 0x00, 0x1e];
const FLASHRAM_CMD_ADDR: u32 = 0x10000;
pub const FLASHRAM_PAGE_SIZE: usize = 128;
const FLASHRAM_SECTOR_SIZE: usize = 0x4000;
const FLASHRAM_BUS_MASK: u32 = 0x07FFFFFF;
const FLASHRAM_STATUS: u8 = 0x8c;

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum FlashramMode {
    ReadArray,
    Status,
    SiliconId,
    LoadBytePage,
}

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum FlashramEraseSetup {
    None,
    Chip,
    Sector(usize),
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Flashram {
    pub mode: FlashramMode,
    pub erase_setup: FlashramEraseSetup,
    #[serde(with = "serde_big_array::BigArray")]
    pub page_buf: [u8; FLASHRAM_PAGE_SIZE],
}

fn is_sram(device: &device::Device) -> bool {
    device.ui.storage.save_type.iter().any(|save_type| {
        matches!(
            save_type,
            ui::storage::SaveTypes::Sram
                | ui::storage::SaveTypes::Sram768k
                | ui::storage::SaveTypes::Sram1m
        )
    })
}

pub fn sram_size(device: &device::Device) -> usize {
    let save_type = &device.ui.storage.save_type;
    if save_type.contains(&ui::storage::SaveTypes::Sram1m) {
        SRAM_1M_SIZE
    } else if save_type.contains(&ui::storage::SaveTypes::Sram768k) {
        SRAM_768K_SIZE
    } else {
        SRAM_SIZE
    }
}

// translate a PI bus address into an offset in the SRAM save data
fn sram_offset(device: &device::Device, address: u32) -> usize {
    let save_type = &device.ui.storage.save_type;
    if save_type.contains(&ui::storage::SaveTypes::Sram768k) {
        // 3 x 256Kbit banks, selected by PI address bits 18-19
        let bank = (address as usize >> 18) & 0x3;
        bank * SRAM_BANK_SIZE + (address as usize & (SRAM_BANK_SIZE - 1))
    } else if save_type.contains(&ui::storage::SaveTypes::Sram1m) {
        address as usize & (SRAM_1M_SIZE - 1)
    } else {
        address as usize & SRAM_MASK
    }
}

pub fn format_sram(device: &mut device::Device) {
    let size = sram_size(device);
    if device.ui.storage.saves.sram.data.len() < size {
        device.ui.storage.saves.sram.data.resize(size, 0xFF)
    }
}

pub fn format_flash(device: &mut device::Device) {
    if device.ui.storage.saves.flash.data.len() < FLASHRAM_SIZE {
        device
            .ui
            .storage
            .saves
            .flash
            .data
            .resize(FLASHRAM_SIZE, 0xFF)
    }
}

fn read_mem_sram(device: &mut device::Device, address: u64) -> u32 {
    let offset = sram_offset(device, address as u32);

    device
        .ui
        .storage
        .saves
        .sram
        .data
        .get(offset..offset + 4)
        .map_or(0, |data| u32::from_be_bytes(data.try_into().unwrap()))
}

// fills `buf` with data read starting at flash bus offset `offset`
fn flash_read(device: &device::Device, offset: u32, buf: &mut [u8]) {
    match device.cart.flashram.mode {
        FlashramMode::Status => {
            // reads as 00 <status> repeated
            for (i, data) in buf.iter_mut().enumerate() {
                *data = if i & 1 == 0 { 0 } else { FLASHRAM_STATUS };
            }
        }
        FlashramMode::SiliconId => {
            for (data, id) in buf.iter_mut().zip(FLASHRAM_SILICON_ID.iter().cycle()) {
                *data = *id;
            }
        }
        FlashramMode::ReadArray => {
            // word-indexed: each bus address selects a 16-bit word
            let base = offset as usize * 2;
            let flash = &device.ui.storage.saves.flash.data;
            for (i, data) in buf.iter_mut().enumerate() {
                *data = *flash.get(base + i).unwrap_or(&0);
            }
        }
        FlashramMode::LoadBytePage => buf.fill(0),
    }
}

fn read_mem_flash(device: &device::Device, address: u64) -> u32 {
    let mut buf = [0; 4];
    flash_read(device, address as u32 & FLASHRAM_BUS_MASK, &mut buf);
    u32::from_be_bytes(buf)
}

pub fn read_mem(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    let cycles = device::pi::calculate_cycles(device, 2, 4);
    device::cop0::add_cycles(device, cycles);

    if is_sram(device) {
        read_mem_sram(device, address)
    } else {
        read_mem_flash(device, address)
    }
}

fn write_mem_sram(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    let offset = sram_offset(device, address as u32);

    let Some(sram) = device
        .ui
        .storage
        .saves
        .sram
        .data
        .get_mut(offset..offset + 4)
    else {
        return;
    };
    let mut data = u32::from_be_bytes((&*sram).try_into().unwrap());
    device::memory::masked_write_32(&mut data, value, mask);
    sram.copy_from_slice(&data.to_be_bytes());

    ui::storage::schedule_save(device, ui::storage::SaveTypes::Sram);
}

fn write_mem_flash(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    if address as u32 & FLASHRAM_BUS_MASK == FLASHRAM_CMD_ADDR {
        flashram_command(device, value & mask);
    }
}

pub fn write_mem(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    if is_sram(device) {
        write_mem_sram(device, address, value, mask)
    } else {
        write_mem_flash(device, address, value, mask)
    }

    device.pi.regs[device::pi::PI_STATUS_REG] |= device::pi::PI_STATUS_IO_BUSY;

    let cycles = device::pi::calculate_cycles(device, 2, 4);
    device::events::create_event(device, device::events::EVENT_TYPE_PI, cycles);
}

fn dma_read_sram(device: &mut device::Device, cart_addr: u32, mut dram_addr: u32, length: u32) {
    dram_addr &= device::rdram::RDRAM_MASK as u32;
    let mut i = dram_addr;
    let mut j = cart_addr;

    while i < dram_addr + length {
        let offset = sram_offset(device, j);
        *device
            .ui
            .storage
            .saves
            .sram
            .data
            .get_mut(offset)
            .unwrap_or(&mut 0) = *device
            .rdram
            .mem
            .get(i as usize ^ device.byte_swap)
            .unwrap_or(&0);
        i += 1;
        j += 1;
    }

    ui::storage::schedule_save(device, ui::storage::SaveTypes::Sram);
}

fn dma_read_flash(device: &mut device::Device, cart_addr: u32, dram_addr: u32, length: u32) {
    if device.cart.flashram.mode != FlashramMode::LoadBytePage {
        return;
    }
    let dram_addr = dram_addr as usize & device::rdram::RDRAM_MASK;
    for i in 0..length as usize {
        device.cart.flashram.page_buf[(cart_addr as usize + i) & (FLASHRAM_PAGE_SIZE - 1)] =
            *device
                .rdram
                .mem
                .get((dram_addr + i) ^ device.byte_swap)
                .unwrap_or(&0);
    }
}

// cart is big endian, rdram is native endian
pub fn dma_read(device: &mut device::Device, cart_addr: u32, dram_addr: u32, length: u32) -> u64 {
    if is_sram(device) {
        dma_read_sram(device, cart_addr, dram_addr, length)
    } else {
        dma_read_flash(device, cart_addr, dram_addr, length)
    }
    device::pi::calculate_cycles(device, 2, length)
}

fn dma_write_sram(device: &mut device::Device, cart_addr: u32, mut dram_addr: u32, length: u32) {
    dram_addr &= device::rdram::RDRAM_MASK as u32;
    let mut i = dram_addr;
    let mut j = cart_addr;

    while i < dram_addr + length {
        let offset = sram_offset(device, j);
        *device
            .rdram
            .mem
            .get_mut(i as usize ^ device.byte_swap)
            .unwrap_or(&mut 0) = *device.ui.storage.saves.sram.data.get(offset).unwrap_or(&0);
        i += 1;
        j += 1;
    }
}

fn dma_write_flash(device: &mut device::Device, cart_addr: u32, dram_addr: u32, length: u32) {
    let dram_addr = dram_addr as usize & device::rdram::RDRAM_MASK;
    let mut buf = vec![0; length as usize];
    flash_read(device, cart_addr & FLASHRAM_BUS_MASK, &mut buf);
    for (i, value) in buf.into_iter().enumerate() {
        if let Some(data) = device.rdram.mem.get_mut((dram_addr + i) ^ device.byte_swap) {
            *data = value;
        }
    }
}

// cart is big endian, rdram is native endian
pub fn dma_write(device: &mut device::Device, cart_addr: u32, dram_addr: u32, length: u32) -> u64 {
    if is_sram(device) {
        dma_write_sram(device, cart_addr, dram_addr, length)
    } else {
        dma_write_flash(device, cart_addr, dram_addr, length)
    }
    device::pi::calculate_cycles(device, 2, length)
}

fn flashram_command(device: &mut device::Device, command: u32) {
    let flash = &mut device.cart.flashram;
    match command >> 24 {
        0x3c => flash.erase_setup = FlashramEraseSetup::Chip,
        0x4b => flash.erase_setup = FlashramEraseSetup::Sector((command as usize & 0x3ff) >> 7),
        0x78 => {
            let data = &mut device.ui.storage.saves.flash.data;
            match flash.erase_setup {
                FlashramEraseSetup::None => return,
                FlashramEraseSetup::Chip => data.fill(0xff),
                FlashramEraseSetup::Sector(sector) => {
                    let base = sector * FLASHRAM_SECTOR_SIZE;
                    data[base..base + FLASHRAM_SECTOR_SIZE].fill(0xff)
                }
            }
            flash.erase_setup = FlashramEraseSetup::None;
            flashram_finish_operation(device);
        }
        0xa5 => {
            let base = (command as usize & 0x3ff) * FLASHRAM_PAGE_SIZE;
            device.ui.storage.saves.flash.data[base..base + FLASHRAM_PAGE_SIZE]
                .copy_from_slice(&flash.page_buf);
            // The internal page buffer resets to all ones after a program.
            flash.page_buf = [0xff; FLASHRAM_PAGE_SIZE];
            flashram_finish_operation(device);
        }
        0xb4 => flash.mode = FlashramMode::LoadBytePage,
        0xd2 => flash.mode = FlashramMode::Status,
        0xe1 => flash.mode = FlashramMode::SiliconId,
        0xf0 => flash.mode = FlashramMode::ReadArray,
        _ => {}
    }
}

fn flashram_finish_operation(device: &mut device::Device) {
    device.cart.flashram.mode = FlashramMode::Status;
    ui::storage::schedule_save(device, ui::storage::SaveTypes::Flash);
}
