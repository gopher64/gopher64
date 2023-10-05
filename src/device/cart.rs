use crate::device;
use crate::ui;

pub const JCMD_STATUS: u8 = 0x00;
pub const JCMD_EEPROM_READ: u8 = 0x04;
pub const JCMD_EEPROM_WRITE: u8 = 0x05;
pub const JCMD_RESET: u8 = 0xff;

//pub const JDT_AF_RTC: u16 = 0x1000; /* RTC */
pub const JDT_EEPROM_4K: u16 = 0x8000; /* 4k EEPROM */
//pub const JDT_EEPROM_16K: u16 = 0xc000; /* 16k EEPROM */
pub const EEPROM_TYPE: u16 = JDT_EEPROM_4K; // todo, support 16k eeprom
pub const EEPROM_BLOCK_SIZE: usize = 8;

pub fn process(device: &mut device::Device, channel: usize) {
    let cmd = device.pif.ram[device.pif.channels[channel].tx_buf.unwrap()];

    match cmd {
        JCMD_RESET => { /* reset  */ }
        JCMD_STATUS => {
            device.pif.ram[device.pif.channels[channel].rx_buf.unwrap()] = EEPROM_TYPE as u8;
            device.pif.ram[device.pif.channels[channel].rx_buf.unwrap() + 1] =
                (EEPROM_TYPE >> 8) as u8;
            device.pif.ram[device.pif.channels[channel].rx_buf.unwrap() + 2] = 0;
        }
        JCMD_EEPROM_READ => {
            eeprom_read_block(
                device,
                device.pif.channels[channel].tx_buf.unwrap() + 1,
                device.pif.channels[channel].rx_buf.unwrap(),
            );
        }
        JCMD_EEPROM_WRITE => eeprom_write_block(
            device,
            device.pif.channels[channel].tx_buf.unwrap() + 1,
            device.pif.channels[channel].tx_buf.unwrap() + 2,
            device.pif.channels[channel].rx_buf.unwrap(),
        ),
        _ => {
            panic!("unknown cart command")
        }
    }
}

pub fn eeprom_read_block(device: &mut device::Device, block: usize, offset: usize) {
    let address = device.pif.ram[block as usize] as usize * EEPROM_BLOCK_SIZE;

    if address < device.ui.saves.eeprom.len() {
        device.pif.ram[offset..offset + EEPROM_BLOCK_SIZE]
            .copy_from_slice(&device.ui.saves.eeprom[address..address + EEPROM_BLOCK_SIZE]);
    } else {
        for i in 0..EEPROM_BLOCK_SIZE {
            device.pif.ram[offset + i] = 0;
        }
    }
}

pub fn eeprom_write_block(device: &mut device::Device, block: usize, offset: usize, status: usize) {
    let address = device.pif.ram[block as usize] as usize * EEPROM_BLOCK_SIZE;

    if address + EEPROM_BLOCK_SIZE > device.ui.saves.eeprom.len() {
        device
            .ui
            .saves
            .eeprom
            .resize(address + EEPROM_BLOCK_SIZE, 0)
    }

    device.ui.saves.eeprom[address..address + EEPROM_BLOCK_SIZE]
        .copy_from_slice(&device.pif.ram[offset..offset + EEPROM_BLOCK_SIZE]);

    device.pif.ram[status as usize] = 0x00;

    ui::storage::write_save(&mut device.ui, ui::storage::SaveTypes::Eeprom);
}
