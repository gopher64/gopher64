use crate::{device, ui};

const SRAM_MASK: usize = 0xFFFF;
pub const SRAM_SIZE: usize = 0x8000;
pub const FLASHRAM_SIZE: usize = 0x20000;
pub const FLASHRAM_TYPE_ID: u32 = 0x11118001;
pub const MX29L1100_ID: u32 = 0x00c2001e;
const MX29L0000_ID: u32 = 0x00c20000;
const MX29L0001_ID: u32 = 0x00c20001;

#[derive(PartialEq, serde::Serialize, serde::Deserialize)]
pub enum FlashramMode {
    ReadArray,
    ReadSiliconId,
    Status,
    SectorErase,
    ChipErase,
    PageProgram,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Flashram {
    pub status: u32,
    pub mode: FlashramMode,
    pub erase_page: u16,
    #[serde(with = "serde_big_array::BigArray")]
    pub page_buf: [u8; 128],
    pub silicon_id: [u32; 2],
}

fn format_sram(device: &mut device::Device) {
    if device.ui.storage.saves.sram.data.len() < SRAM_SIZE {
        device.ui.storage.saves.sram.data.resize(SRAM_SIZE, 0xFF)
    }
}

fn format_flash(device: &mut device::Device) {
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
    let masked_address = address as usize & SRAM_MASK;

    format_sram(device);

    u32::from_be_bytes(
        device.ui.storage.saves.sram.data[masked_address..masked_address + 4]
            .try_into()
            .unwrap(),
    )
}

fn read_mem_flash(device: &device::Device, address: u64) -> u32 {
    if (address & 0x1ffff) == 0x00000 && device.cart.flashram.mode == FlashramMode::Status {
        /* read Status register */
        device.cart.flashram.status
    } else if (address & 0x1ffff) == 0x0000 && device.cart.flashram.mode == FlashramMode::ReadArray
    {
        /* flashram MMIO read are not supported except for the "dummy" read @0x0000 done before DMA.
         * returns a "dummy" value. */
        0
    } else {
        /* other accesses are not implemented */
        panic!("unknown flashram read")
    }
}

pub fn read_mem(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    let cycles = device::pi::calculate_cycles(device, 2, 4);
    device::cop0::add_cycles(device, cycles);

    if device
        .ui
        .storage
        .save_type
        .contains(&ui::storage::SaveTypes::Sram)
    {
        read_mem_sram(device, address)
    } else {
        read_mem_flash(device, address)
    }
}

fn write_mem_sram(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    let masked_address = address as usize & SRAM_MASK;

    format_sram(device);

    let mut data = u32::from_be_bytes(
        device.ui.storage.saves.sram.data[masked_address..masked_address + 4]
            .try_into()
            .unwrap(),
    );
    device::memory::masked_write_32(&mut data, value, mask);
    device.ui.storage.saves.sram.data[masked_address..masked_address + 4]
        .copy_from_slice(&data.to_be_bytes());

    device.ui.storage.saves.sram.written = true
}

fn write_mem_flash(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    if (address & 0x1ffff) == 0x00000 && device.cart.flashram.mode == FlashramMode::Status {
        /* clear/set Status register */
        device.cart.flashram.status = (value & mask) & 0xff;
    } else if (address & 0x1ffff) == 0x10000 {
        /* set command */
        format_flash(device);
        flashram_command(device, value & mask);
    } else {
        /* other accesses are not implemented */
        panic!("unknown flashram write")
    }
}

pub fn write_mem(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    if device
        .ui
        .storage
        .save_type
        .contains(&ui::storage::SaveTypes::Sram)
    {
        write_mem_sram(device, address, value, mask)
    } else {
        write_mem_flash(device, address, value, mask)
    }

    device.pi.regs[device::pi::PI_STATUS_REG as usize] |= device::pi::PI_STATUS_IO_BUSY;

    let cycles = device::pi::calculate_cycles(device, 2, 4);
    device::events::create_event(device, device::events::EVENT_TYPE_PI, cycles);
}

fn dma_read_sram(device: &mut device::Device, mut cart_addr: u32, mut dram_addr: u32, length: u32) {
    dram_addr &= device::rdram::RDRAM_MASK as u32;
    cart_addr &= SRAM_MASK as u32;
    let mut i = dram_addr;
    let mut j = cart_addr;

    format_sram(device);

    while i < dram_addr + length {
        *device
            .ui
            .storage
            .saves
            .sram
            .data
            .get_mut(j as usize)
            .unwrap_or(&mut 0) = *device
            .rdram
            .mem
            .get(i as usize ^ device.byte_swap)
            .unwrap_or(&0);
        i += 1;
        j += 1;
    }

    device.ui.storage.saves.sram.written = true
}

fn dma_read_flash(device: &mut device::Device, cart_addr: u32, dram_addr: u32, length: u32) {
    format_flash(device);

    if (cart_addr & 0x1ffff) == 0x00000
        && length == 128
        && device.cart.flashram.mode == FlashramMode::PageProgram
    {
        /* load page buf using DMA */
        for i in 0..length {
            device.cart.flashram.page_buf[i as usize] = *device
                .rdram
                .mem
                .get((dram_addr + i) as usize ^ device.byte_swap)
                .unwrap_or(&0);
        }
    } else {
        /* other accesses are not implemented */
        panic!("unknown flash dma read")
    }
}

// cart is big endian, rdram is native endian
pub fn dma_read(device: &mut device::Device, cart_addr: u32, dram_addr: u32, length: u32) -> u64 {
    if device
        .ui
        .storage
        .save_type
        .contains(&ui::storage::SaveTypes::Sram)
    {
        dma_read_sram(device, cart_addr, dram_addr, length)
    } else {
        dma_read_flash(device, cart_addr, dram_addr, length)
    }
    device::pi::calculate_cycles(device, 2, length)
}

fn dma_write_sram(
    device: &mut device::Device,
    mut cart_addr: u32,
    mut dram_addr: u32,
    length: u32,
) {
    dram_addr &= device::rdram::RDRAM_MASK as u32;
    cart_addr &= SRAM_MASK as u32;
    let mut i = dram_addr;
    let mut j = cart_addr;

    format_sram(device);

    while i < dram_addr + length {
        *device
            .rdram
            .mem
            .get_mut(i as usize ^ device.byte_swap)
            .unwrap_or(&mut 0) = *device
            .ui
            .storage
            .saves
            .sram
            .data
            .get(j as usize)
            .unwrap_or(&0);
        i += 1;
        j += 1;
    }
}

fn dma_write_flash(
    device: &mut device::Device,
    mut cart_addr: u32,
    mut dram_addr: u32,
    length: u32,
) {
    dram_addr &= device::rdram::RDRAM_MASK as u32;

    if (cart_addr & 0x1ffff) == 0x00000
        && length == 8
        && device.cart.flashram.mode == FlashramMode::ReadSiliconId
    {
        /* read Silicon ID using DMA */
        device
            .rdram
            .mem
            .get_mut(dram_addr as usize..dram_addr as usize + 4)
            .unwrap_or(&mut [0; 4])
            .copy_from_slice(&device.cart.flashram.silicon_id[0].to_ne_bytes());
        dram_addr += 4;
        device
            .rdram
            .mem
            .get_mut(dram_addr as usize..dram_addr as usize + 4)
            .unwrap_or(&mut [0; 4])
            .copy_from_slice(&device.cart.flashram.silicon_id[1].to_ne_bytes());
    } else if (cart_addr & 0x1ffff) < 0x10000
        && device.cart.flashram.mode == FlashramMode::ReadArray
    {
        format_flash(device);
        /* adjust flashram address before starting DMA. */
        if device.cart.flashram.silicon_id[1] == MX29L1100_ID
            || device.cart.flashram.silicon_id[1] == MX29L0000_ID
            || device.cart.flashram.silicon_id[1] == MX29L0001_ID
        {
            /* "old" flash needs special address adjusting */
            cart_addr = (cart_addr & 0xffff) * 2;
        } else {
            /* "new" flash doesn't require special address adjusting at DMA start. */
            cart_addr &= 0xffff;
        }

        /* do actual DMA */
        for i in 0..length {
            *device
                .rdram
                .mem
                .get_mut((dram_addr + i) as usize ^ device.byte_swap)
                .unwrap_or(&mut 0) = device.ui.storage.saves.flash.data[(cart_addr + i) as usize];
        }
    } else {
        /* other accesses are not implemented */
        panic!("unknown flash dma write")
    }
}

// cart is big endian, rdram is native endian
pub fn dma_write(device: &mut device::Device, cart_addr: u32, dram_addr: u32, length: u32) -> u64 {
    if device
        .ui
        .storage
        .save_type
        .contains(&ui::storage::SaveTypes::Sram)
    {
        dma_write_sram(device, cart_addr, dram_addr, length)
    } else {
        dma_write_flash(device, cart_addr, dram_addr, length)
    }
    device::pi::calculate_cycles(device, 2, length)
}

fn flashram_command(device: &mut device::Device, command: u32) {
    match command & 0xff000000 {
        0x3c000000 => {
            /* set chip erase mode */
            device.cart.flashram.mode = FlashramMode::ChipErase;
        }

        0x4b000000 => {
            /* set sector erase mode, set erase sector */
            device.cart.flashram.mode = FlashramMode::SectorErase;
            device.cart.flashram.erase_page = command as u16;
        }

        0x78000000 => {
            /* set erase busy flag */
            device.cart.flashram.status |= 0x02;

            /* do chip/sector erase */
            if device.cart.flashram.mode == FlashramMode::SectorErase {
                let offset: usize = (device.cart.flashram.erase_page & 0xff80) as usize * 128;
                for i in 0..128 * 128 {
                    device.ui.storage.saves.flash.data[offset + i] = 0xFF;
                }
                device.ui.storage.saves.flash.written = true
            } else if device.cart.flashram.mode == FlashramMode::ChipErase {
                for i in 0..FLASHRAM_SIZE {
                    device.ui.storage.saves.flash.data[i] = 0xFF;
                }
                device.ui.storage.saves.flash.written = true
            } else {
                panic!("Unexpected flash erase command")
            }

            /* clear erase busy flag, set erase success flag, transition to status mode */
            device.cart.flashram.status &= !0x02;
            device.cart.flashram.status |= 0x08;
            device.cart.flashram.mode = FlashramMode::Status;
        }

        0xa5000000 => {
            /* set program busy flag */
            device.cart.flashram.status |= 0x01;

            /* program selected page */
            let offset: usize = (command & 0xffff) as usize * 128;
            for i in 0..128 {
                device.ui.storage.saves.flash.data[offset + i] = device.cart.flashram.page_buf[i];
            }
            device.ui.storage.saves.flash.written = true;

            /* clear program busy flag, set program success flag, transition to status mode */
            device.cart.flashram.status &= !0x01;
            device.cart.flashram.status |= 0x04;
            device.cart.flashram.mode = FlashramMode::Status;
        }

        0xb4000000 => {
            /* set page program mode */
            device.cart.flashram.mode = FlashramMode::PageProgram;
        }

        0xd2000000 => {
            /* set status mode */
            device.cart.flashram.mode = FlashramMode::Status;
        }

        0xe1000000 => {
            /* set silicon_id mode */
            device.cart.flashram.mode = FlashramMode::ReadSiliconId;
            device.cart.flashram.status |= 0x01; /* Needed for Pokemon Puzzle League */
        }

        0xf0000000 => {
            /* set read mode */
            device.cart.flashram.mode = FlashramMode::ReadArray;
        }

        _ => {
            panic!("unknown flash command")
        }
    }
}
