use crate::device;
use crate::ui;

pub const SDCARD_SIZE: usize = 0x4000000;

const SC64_SCR_REG: usize = 0;
const SC64_DATA0_REG: usize = 1;
const SC64_DATA1_REG: usize = 2;
const SC64_IDENTIFIER_REG: usize = 3;
const SC64_KEY_REG: usize = 4;
//const SC64_IRQ_REG: usize = 5;
//const SC64_AUX_REG: usize = 6;
pub const SC64_REGS_COUNT: usize = 7;

pub const SC64_BOOTLOADER_SWITCH: u32 = 0;
pub const SC64_ROM_WRITE_ENABLE: u32 = 1;
pub const SC64_SAVE_TYPE: u32 = 6;
pub const SC64_CFG_COUNT: usize = 15;

const SC64_BUFFER_MASK: usize = 0x1FFF;
const SC64_EEPROM_MASK: usize = 0xFFF;
const USB_PACKET_DELAY: u64 = 93750 * 2; // ~2ms of COUNT (CPU cycles at 93.75 MHz)

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Sc64 {
    pub buffer: Vec<u8>,
    pub regs: [u32; SC64_REGS_COUNT],
    pub regs_locked: bool,
    pub cfg: [u32; SC64_CFG_COUNT],
    pub sector: u32,
    pub writeback_sector: Vec<u32>,
    pub usb_buffer: Vec<u8>,
    pub usb_read_type: u32,
    pub usb_read_size: u32, // non-zero while a received packet is waiting to be read by 'm'
    pub usb_next_read: u64, // COUNT at which the next received packet becomes visible
}

fn format_sdcard(device: &mut device::Device) {
    if device.ui.storage.saves.sdcard.data.is_empty() {
        device.ui.storage.saves.sdcard.data.resize(SDCARD_SIZE, 0);
        let buf = std::io::Cursor::new(&mut device.ui.storage.saves.sdcard.data);
        fatfs::format_volume(buf, fatfs::FormatVolumeOptions::new()).unwrap();
    }
}

pub fn read_regs(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    device::cop0::add_cycles(device, 20);
    if device.cart.sc64.regs_locked {
        return 0;
    }
    let reg = (address & 0xFFFF) >> 2;
    match reg as usize {
        SC64_SCR_REG | SC64_DATA0_REG | SC64_DATA1_REG => device.cart.sc64.regs[reg as usize],
        SC64_IDENTIFIER_REG => 0x53437632,
        _ => panic!("unknown read reg {reg} address {address:#x}"),
    }
}

pub fn write_regs(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    let reg = (address & 0xFFFF) >> 2;
    match reg as usize {
        SC64_KEY_REG => {
            device::memory::masked_write_32(&mut device.cart.sc64.regs[reg as usize], value, mask);
            if device.cart.sc64.regs[SC64_KEY_REG] == 0x4F434B5F {
                device.cart.sc64.regs_locked = false;
            } else if device.cart.sc64.regs[SC64_KEY_REG] == 0xFFFFFFFF {
                device.cart.sc64.regs_locked = true;
            }
        }
        SC64_DATA0_REG | SC64_DATA1_REG => {
            if !device.cart.sc64.regs_locked {
                device::memory::masked_write_32(
                    &mut device.cart.sc64.regs[reg as usize],
                    value,
                    mask,
                );
            }
        }
        SC64_SCR_REG => {
            if !device.cart.sc64.regs_locked {
                match char::from_u32(value & mask).unwrap() {
                    'V' => {
                        // get version
                        device.cart.sc64.regs[SC64_DATA0_REG] = (2 << 16) | 20;
                        device.cart.sc64.regs[SC64_DATA1_REG] = 2;
                    }
                    'c' => {
                        // get config
                        device.cart.sc64.regs[SC64_DATA1_REG] =
                            device.cart.sc64.cfg[device.cart.sc64.regs[SC64_DATA0_REG] as usize]
                    }
                    'C' => {
                        // set config
                        if device.cart.sc64.regs[SC64_DATA0_REG] == SC64_SAVE_TYPE {
                            // if save type is being written, we are probably booting a game using the flash cart menu
                            // we shouldn't write saves to disk in this case (they are written to the SD card)
                            device.ui.storage.saves.write_to_disk = false;
                            device.ui.storage.save_type =
                                match device.cart.sc64.regs[SC64_DATA1_REG] {
                                    0 => {
                                        vec![]
                                    }
                                    1 => {
                                        vec![ui::storage::SaveTypes::Eeprom4k]
                                    }
                                    2 => {
                                        vec![ui::storage::SaveTypes::Eeprom16k]
                                    }
                                    3 => {
                                        vec![ui::storage::SaveTypes::Sram]
                                    }
                                    4 => {
                                        vec![ui::storage::SaveTypes::Flash]
                                    }
                                    5 => {
                                        vec![ui::storage::SaveTypes::Sram768k]
                                    }
                                    6 => {
                                        vec![ui::storage::SaveTypes::Sram1m]
                                    }
                                    _ => {
                                        panic!(
                                            "unknown sc64 save type: {}",
                                            device.cart.sc64.regs[SC64_DATA1_REG]
                                        )
                                    }
                                };
                            device::cart::sram::format_sram(device);
                        }
                        std::mem::swap(
                            &mut device.cart.sc64.cfg
                                [device.cart.sc64.regs[SC64_DATA0_REG] as usize],
                            &mut device.cart.sc64.regs[SC64_DATA1_REG],
                        );
                    }
                    'i' => {
                        // sd card operation
                        match device.cart.sc64.regs[SC64_DATA1_REG] {
                            0 => { //Init SD card
                            }
                            1 => { //Deinit SD card
                            }
                            _ => {
                                panic!(
                                    "unknown sc64 sd card operation: {}",
                                    device.cart.sc64.regs[SC64_DATA1_REG]
                                )
                            }
                        }
                    }
                    'I' => {
                        // set sd sector
                        device.cart.sc64.sector = device.cart.sc64.regs[SC64_DATA0_REG];
                    }
                    's' => {
                        format_sdcard(device);
                        // read sd card
                        let address = device.cart.sc64.regs[SC64_DATA0_REG] as u64 & 0x1FFFFFFF;
                        let offset = (device.cart.sc64.sector * 512) as usize;
                        let length = (device.cart.sc64.regs[SC64_DATA1_REG] * 512) as usize;
                        let mut i = 0;

                        while i < length {
                            let data = u32::from_be_bytes(
                                device
                                    .ui
                                    .storage
                                    .saves
                                    .sdcard
                                    .data
                                    .get((offset + i)..(offset + i + 4))
                                    .unwrap_or(&[0; 4])
                                    .try_into()
                                    .unwrap_or_default(),
                            );

                            device::memory::data_write(
                                device,
                                address + i as u64,
                                data,
                                0xFFFFFFFF,
                                None,
                            );
                            i += 4;
                        }
                    }
                    'S' => {
                        format_sdcard(device);
                        // write sd card
                        let address = device.cart.sc64.regs[SC64_DATA0_REG] as u64 & 0x1FFFFFFF;
                        let offset = (device.cart.sc64.sector * 512) as usize;
                        let length = (device.cart.sc64.regs[SC64_DATA1_REG] * 512) as usize;
                        let mut i = 0;

                        while i < length {
                            let data = device::memory::data_read(
                                device,
                                address + i as u64,
                                device::memory::AccessSize::Word,
                                None,
                            )
                            .to_be_bytes();
                            device
                                .ui
                                .storage
                                .saves
                                .sdcard
                                .data
                                .get_mut((offset + i)..(offset + i + 4))
                                .unwrap_or(&mut [0; 4])
                                .copy_from_slice(&data);
                            i += 4;
                        }
                        ui::storage::schedule_save(device, ui::storage::SaveTypes::Sdcard);
                    }
                    'U' => {
                        device.cart.sc64.regs[SC64_DATA0_REG] = 0;
                    }
                    'u' => {
                        // used to notify the game that there is data to read.
                        // Status reads don't consume the packet, only 'm' does.
                        // Packets from the PC arrive with some USB latency in between, so don't make
                        // the next one visible right away (games may rely on this, NetLib examples do)
                        let count = device.cpu.cop0.regs[device::cop0::COP0_COUNT_REG];
                        if device.cart.sc64.usb_read_size == 0
                            && (count >= device.cart.sc64.usb_next_read
                                || device.cart.sc64.usb_next_read - count > USB_PACKET_DELAY)
                            && let Some(cart_rx) = device.ui.usb.cart_rx.as_mut()
                            && let Ok(data) = cart_rx.try_recv()
                        {
                            device.cart.sc64.usb_read_type = data.data_type & 0xFF;
                            device.cart.sc64.usb_read_size = data.data_size & 0xFFFFFF;
                            device.cart.sc64.usb_buffer = data.data; // store the data to be read
                        }
                        device.cart.sc64.regs[SC64_DATA0_REG] = device.cart.sc64.usb_read_type; // read_status/type
                        device.cart.sc64.regs[SC64_DATA1_REG] = device.cart.sc64.usb_read_size; // length
                    }
                    'M' => {
                        // Send data from from flashcart to USB
                        let address = device.cart.sc64.regs[SC64_DATA0_REG] as u64 & 0x1FFFFFFF;
                        let length = device.cart.sc64.regs[SC64_DATA1_REG] as usize & 0xFFFFFF;

                        if let Some(usb_tx) = device.ui.usb.usb_tx.as_mut() {
                            let mut usb_buffer = vec![0; length];

                            let mut i = 0;

                            if address < device.rdram.size as u64 {
                                while i < length {
                                    *usb_buffer.get_mut(i).unwrap_or(&mut 0) = *device
                                        .rdram
                                        .mem
                                        .get((address as usize + i) ^ device.byte_swap)
                                        .unwrap_or(&0);
                                    i += 1;
                                }
                            } else if address >= device::memory::MM_CART_ROM as u64
                                && address < device::memory::MM_PIF_MEM as u64
                            {
                                while i < length {
                                    *usb_buffer.get_mut(i).unwrap_or(&mut 0) = *device
                                        .ui
                                        .storage
                                        .saves
                                        .romsave
                                        .data
                                        .get(
                                            &(((address as usize + i)
                                                & device::cart::rom::CART_MASK)
                                                as u32),
                                        )
                                        .unwrap_or(
                                            device
                                                .cart
                                                .rom
                                                .get(
                                                    (address as usize + i)
                                                        & device::cart::rom::CART_MASK,
                                                )
                                                .unwrap_or(&0),
                                        );
                                    i += 1;
                                }
                            } else {
                                panic!("Unknown address {address:#x} for SC64 M command");
                            }

                            ui::usb::send_to_usb(
                                usb_tx,
                                ui::usb::UsbData {
                                    data: usb_buffer,
                                    data_type: device.cart.sc64.regs[SC64_DATA1_REG] >> 24,
                                    data_size: device.cart.sc64.regs[SC64_DATA1_REG] & 0xFFFFFF,
                                },
                            );
                        }
                    }
                    'm' => {
                        // Receive data from USB to flashcart
                        let address = device.cart.sc64.regs[SC64_DATA0_REG] as u64 & 0x1FFFFFFF;
                        let length = device.cart.sc64.regs[SC64_DATA1_REG] as usize & 0xFFFFFF;
                        let usb_buffer = std::mem::take(&mut device.cart.sc64.usb_buffer);

                        let mut i = 0;

                        if address < device.rdram.size as u64 {
                            while i < length {
                                *device
                                    .rdram
                                    .mem
                                    .get_mut((address as usize + i) ^ device.byte_swap)
                                    .unwrap_or(&mut 0) = *usb_buffer.get(i).unwrap_or(&0);
                                i += 1;
                            }
                        } else if address >= device::memory::MM_CART_ROM as u64
                            && address < device::memory::MM_PIF_MEM as u64
                        {
                            while i < length {
                                device.ui.storage.saves.romsave.data.insert(
                                    ((address as usize + i) & device::cart::rom::CART_MASK) as u32,
                                    *usb_buffer.get(i).unwrap_or(&0),
                                );
                                i += 1;
                            }
                        } else {
                            panic!("Unknown address {address:#x} for SC64 m command");
                        }
                        // packet consumed
                        if device.cart.sc64.usb_read_size != 0 {
                            device.cart.sc64.usb_read_type = 0;
                            device.cart.sc64.usb_read_size = 0;
                            device.cart.sc64.usb_next_read = device.cpu.cop0.regs
                                [device::cop0::COP0_COUNT_REG]
                                + USB_PACKET_DELAY;
                        }
                    }
                    'w' => {
                        // SD card writeback pending
                        device.cart.sc64.regs[SC64_DATA0_REG] = 0;
                    }
                    'W' => {
                        let writeback_sectors_address =
                            device.cart.sc64.regs[SC64_DATA0_REG] as u64;
                        for i in 0..256 {
                            device.cart.sc64.writeback_sector[i] = device::memory::data_read(
                                device,
                                writeback_sectors_address + (i * 4) as u64,
                                device::memory::AccessSize::Word,
                                None,
                            );
                        }
                    }
                    '%' => {
                        match device.cart.sc64.regs[SC64_DATA0_REG] {
                            0 => {
                                // DIAGNOSTIC_ID_VOLTAGE_TEMPERATURE
                                device.cart.sc64.regs[SC64_DATA1_REG] = 3300 << 16;
                            }
                            _ => {
                                panic!(
                                    "unknown sc64 diagnostic command: {}",
                                    device.cart.sc64.regs[SC64_DATA0_REG]
                                );
                            }
                        }
                    }
                    _ => {
                        panic!(
                            "unknown sc64 command: {}",
                            char::from_u32(value & mask).unwrap()
                        )
                    }
                }
            }
        }
        _ => panic!(
            "unknown write reg {} address {:#x} value {}",
            reg,
            address,
            char::from_u32(value & mask).unwrap()
        ),
    }
}

pub fn read_mem(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    if address & 0x2000 != 0 {
        let masked_address = address as usize & SC64_EEPROM_MASK;
        u32::from_be_bytes(
            device.ui.storage.saves.eeprom.data[masked_address..masked_address + 4]
                .try_into()
                .unwrap(),
        )
    } else {
        let masked_address = address as usize & SC64_BUFFER_MASK;
        u32::from_be_bytes(
            device.cart.sc64.buffer[masked_address..masked_address + 4]
                .try_into()
                .unwrap(),
        )
    }
}

pub fn write_mem(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    if address & 0x2000 != 0 {
        let masked_address = address as usize & SC64_EEPROM_MASK;
        let mut data = u32::from_be_bytes(
            device.ui.storage.saves.eeprom.data[masked_address..masked_address + 4]
                .try_into()
                .unwrap(),
        );
        device::memory::masked_write_32(&mut data, value, mask);
        device.ui.storage.saves.eeprom.data[masked_address..masked_address + 4]
            .copy_from_slice(&data.to_be_bytes());
        ui::storage::schedule_save(device, ui::storage::SaveTypes::Eeprom4k);
    } else {
        let masked_address = address as usize & SC64_BUFFER_MASK;
        let mut data = u32::from_be_bytes(
            device.cart.sc64.buffer[masked_address..masked_address + 4]
                .try_into()
                .unwrap(),
        );
        device::memory::masked_write_32(&mut data, value, mask);
        device.cart.sc64.buffer[masked_address..masked_address + 4]
            .copy_from_slice(&data.to_be_bytes());
    }
}

pub fn dma_read(
    device: &mut device::Device,
    mut cart_addr: u32,
    mut dram_addr: u32,
    length: u32,
) -> u64 {
    dram_addr &= device::rdram::RDRAM_MASK as u32;
    let buffer = if cart_addr & 0x2000 != 0 {
        cart_addr &= SC64_EEPROM_MASK as u32;
        ui::storage::schedule_save(device, ui::storage::SaveTypes::Eeprom4k);
        &mut device.ui.storage.saves.eeprom.data
    } else {
        cart_addr &= SC64_BUFFER_MASK as u32;
        &mut device.cart.sc64.buffer
    };
    let mut i = dram_addr;
    let mut j = cart_addr;

    while i < dram_addr + length {
        *buffer.get_mut(j as usize).unwrap_or(&mut 0) = *device
            .rdram
            .mem
            .get(i as usize ^ device.byte_swap)
            .unwrap_or(&0);
        i += 1;
        j += 1;
    }

    device::pi::calculate_cycles(device, 1, length)
}

pub fn dma_write(
    device: &mut device::Device,
    mut cart_addr: u32,
    mut dram_addr: u32,
    length: u32,
) -> u64 {
    dram_addr &= device::rdram::RDRAM_MASK as u32;
    let buffer = if cart_addr & 0x2000 != 0 {
        cart_addr &= SC64_EEPROM_MASK as u32;
        &device.ui.storage.saves.eeprom.data
    } else {
        cart_addr &= SC64_BUFFER_MASK as u32;
        &device.cart.sc64.buffer
    };
    let mut i = dram_addr;
    let mut j = cart_addr;

    while i < dram_addr + length {
        *device
            .rdram
            .mem
            .get_mut(i as usize ^ device.byte_swap)
            .unwrap_or(&mut 0) = *buffer.get(j as usize).unwrap_or(&0);
        i += 1;
        j += 1;
    }
    device::pi::calculate_cycles(device, 1, length)
}
