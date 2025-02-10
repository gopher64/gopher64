use crate::device;

pub const SDCARD_SIZE: usize = 0x4000000;

const SC64_SCR_REG: u32 = 0;
const SC64_DATA0_REG: u32 = 1;
const SC64_DATA1_REG: u32 = 2;
const SC64_IDENTIFIER_REG: u32 = 3;
const SC64_KEY_REG: u32 = 4;
//const SC64_IRQ_REG: u32 = 5;
//const SC64_AUX_REG: u32 = 6;
pub const SC64_REGS_COUNT: u32 = 7;

pub const SC64_ROM_WRITE_ENABLE: u32 = 1;
pub const SC64_CFG_COUNT: u32 = 15;

const SC64_BUFFER_MASK: usize = 0x1FFF;

#[derive(serde::Serialize)]
pub struct Sc64 {
    #[serde(serialize_with = "<[_]>::serialize")]
    pub buffer: [u8; 8192],
    pub regs: [u32; SC64_REGS_COUNT as usize],
    pub regs_locked: bool,
    pub cfg: [u32; SC64_CFG_COUNT as usize],
    pub sector: u32,
}

fn format_sdcard(device: &mut device::Device) {
    if device.ui.saves.sdcard.0.is_empty() {
        device.ui.saves.sdcard.0.resize(SDCARD_SIZE, 0);
        let buf = std::io::Cursor::new(&mut device.ui.saves.sdcard.0);
        fatfs::format_volume(buf, fatfs::FormatVolumeOptions::new()).unwrap();
    }
}

pub fn read_regs(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    device::cop0::add_cycles(device, 20);
    if device.sc64.regs_locked {
        return 0;
    }
    let reg = (address & 0xFFFF) >> 2;
    match reg as u32 {
        SC64_SCR_REG | SC64_DATA0_REG | SC64_DATA1_REG => device.sc64.regs[reg as usize],
        SC64_IDENTIFIER_REG => 0x53437632,
        _ => panic!("unknown read reg {} address {:#x}", reg, address),
    }
}

pub fn write_regs(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    let reg = (address & 0xFFFF) >> 2;
    match reg as u32 {
        SC64_KEY_REG => {
            device::memory::masked_write_32(&mut device.sc64.regs[reg as usize], value, mask);
            if device.sc64.regs[SC64_KEY_REG as usize] == 0x4F434B5F {
                device.sc64.regs_locked = false;
            } else if device.sc64.regs[SC64_KEY_REG as usize] == 0xFFFFFFFF {
                device.sc64.regs_locked = true;
            }
        }
        SC64_DATA0_REG | SC64_DATA1_REG => {
            if !device.sc64.regs_locked {
                device::memory::masked_write_32(&mut device.sc64.regs[reg as usize], value, mask);
            }
        }
        SC64_SCR_REG => {
            if !device.sc64.regs_locked {
                match char::from_u32(value & mask).unwrap() {
                    'c' => {
                        // get config
                        device.sc64.regs[SC64_DATA1_REG as usize] =
                            device.sc64.cfg[device.sc64.regs[SC64_DATA0_REG as usize] as usize]
                    }
                    'C' => {
                        // set config
                        std::mem::swap(
                            &mut device.sc64.cfg
                                [device.sc64.regs[SC64_DATA0_REG as usize] as usize],
                            &mut device.sc64.regs[SC64_DATA1_REG as usize],
                        );
                    }
                    'i' => {
                        // sd card operation
                        match device.sc64.regs[SC64_DATA1_REG as usize] {
                            0 => { //Init SD card
                            }
                            1 => { //Deinit SD card
                            }
                            _ => {
                                panic!(
                                    "unknown sc64 sd card operation: {}",
                                    device.sc64.regs[SC64_DATA1_REG as usize]
                                )
                            }
                        }
                    }
                    'I' => {
                        // set sd sector
                        device.sc64.sector = device.sc64.regs[SC64_DATA0_REG as usize];
                    }
                    's' => {
                        format_sdcard(device);
                        // read sd card
                        let address = device.sc64.regs[SC64_DATA0_REG as usize] as u64 & 0x1FFFFFFF;
                        let offset = (device.sc64.sector * 512) as usize;
                        let length = (device.sc64.regs[SC64_DATA1_REG as usize] * 512) as usize;
                        let mut i = 0;

                        while i < length {
                            if offset + i < device.ui.saves.sdcard.0.len() {
                                let data = u32::from_be_bytes(
                                    device.ui.saves.sdcard.0[(offset + i)..(offset + i + 4)]
                                        .try_into()
                                        .unwrap(),
                                );

                                device::memory::data_write(
                                    device,
                                    address + i as u64,
                                    data,
                                    0xFFFFFFFF,
                                    false,
                                );
                            } else {
                                panic!("sd card read out of bounds")
                            }
                            i += 4;
                        }
                    }
                    'S' => {
                        format_sdcard(device);
                        // write sd card
                        let address = device.sc64.regs[SC64_DATA0_REG as usize] as u64 & 0x1FFFFFFF;
                        let offset = (device.sc64.sector * 512) as usize;
                        let length = (device.sc64.regs[SC64_DATA1_REG as usize] * 512) as usize;
                        let mut i = 0;

                        while i < length {
                            if offset + i < device.ui.saves.sdcard.0.len() {
                                let data = device::memory::data_read(
                                    device,
                                    address + i as u64,
                                    device::memory::AccessSize::Word,
                                    false,
                                )
                                .to_be_bytes();
                                device.ui.saves.sdcard.0[(offset + i)..(offset + i + 4)]
                                    .copy_from_slice(&data);
                            } else {
                                panic!("sd card write out of bounds")
                            }
                            i += 4;
                        }
                        device.ui.saves.sdcard.1 = true;
                    }
                    'U' => {} // USB_WRITE_STATUS, ignored
                    'M' => {} // USB_WRITE, ignored
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
    let masked_address = address as usize & SC64_BUFFER_MASK;
    u32::from_be_bytes(
        device.sc64.buffer[masked_address..masked_address + 4]
            .try_into()
            .unwrap(),
    )
}

pub fn write_mem(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    let masked_address = address as usize & SC64_BUFFER_MASK;
    let mut data = u32::from_be_bytes(
        device.sc64.buffer[masked_address..masked_address + 4]
            .try_into()
            .unwrap(),
    );
    device::memory::masked_write_32(&mut data, value, mask);
    device.sc64.buffer[masked_address..masked_address + 4].copy_from_slice(&data.to_be_bytes());
}

pub fn dma_read(
    device: &mut device::Device,
    mut cart_addr: u32,
    mut dram_addr: u32,
    length: u32,
) -> u64 {
    dram_addr &= device::rdram::RDRAM_MASK as u32;
    cart_addr &= SC64_BUFFER_MASK as u32;
    let mut i = dram_addr;
    let mut j = cart_addr;

    while i < dram_addr + length && i < device.rdram.size {
        device.sc64.buffer[j as usize] = device.rdram.mem[i as usize ^ device.byte_swap];
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
    cart_addr &= SC64_BUFFER_MASK as u32;
    let mut i = dram_addr;
    let mut j = cart_addr;

    while i < dram_addr + length && i < device.rdram.size {
        device.rdram.mem[i as usize ^ device.byte_swap] = device.sc64.buffer[j as usize];
        i += 1;
        j += 1;
    }
    device::pi::calculate_cycles(device, 1, length)
}
