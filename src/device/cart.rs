use chrono::Datelike;
use chrono::Timelike;

use crate::{device, ui};

pub mod rom;
pub mod sc64;
pub mod sram;

const JCMD_STATUS: u8 = 0x00;
const JCMD_EEPROM_READ: u8 = 0x04;
const JCMD_EEPROM_WRITE: u8 = 0x05;
const JCMD_AF_RTC_STATUS: u8 = 0x06;
const JCMD_AF_RTC_READ: u8 = 0x07;
const JCMD_AF_RTC_WRITE: u8 = 0x08;
const JCMD_RESET: u8 = 0xff;

const JDT_AF_RTC: u16 = 0x1000; /* RTC */
const JDT_EEPROM_4K: u16 = 0x8000; /* 4k EEPROM */
const JDT_EEPROM_16K: u16 = 0xc000; /* 16k EEPROM */
const EEPROM_BLOCK_SIZE: usize = 8;
pub const EEPROM_MAX_SIZE: usize = 0x800;

#[derive(serde::Serialize, serde::Deserialize)]
pub enum CicType {
    CicNus6101,
    CicNus6102,
    CicNus6103,
    CicNus6105,
    CicNus6106,
    CicNus5167,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct AfRtc {
    pub control: u16,
}

fn byte2bcd(mut n: u32) -> u8 {
    n %= 100;
    (((n / 10) << 4) | (n % 10)) as u8
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Cart {
    #[serde(skip)]
    pub rom: Vec<u8>,
    pub is_viewer_buffer: Vec<u8>,
    pub pal: bool,
    pub latch: u32,
    pub cic_seed: u8,
    pub rtc: AfRtc,
    pub sc64: sc64::Sc64,
    pub flashram: sram::Flashram,
    pub rtc_timestamp: i64,
}

pub fn process(device: &mut device::Device, channel: usize) {
    let cmd = device.pif.ram[device.pif.channels[channel].tx_buf.unwrap()];

    match cmd {
        JCMD_RESET | JCMD_STATUS => {
            let eeprom_type;
            if device
                .ui
                .storage
                .save_type
                .contains(&ui::storage::SaveTypes::Eeprom16k)
            {
                eeprom_type = JDT_EEPROM_16K;
            } else if device
                .ui
                .storage
                .save_type
                .contains(&ui::storage::SaveTypes::Eeprom4k)
            {
                eeprom_type = JDT_EEPROM_4K;
            } else {
                device.pif.ram[device.pif.channels[channel].rx.unwrap()] |= 0x80;
                return;
            }

            device.pif.ram[device.pif.channels[channel].rx_buf.unwrap()] = eeprom_type as u8;
            device.pif.ram[device.pif.channels[channel].rx_buf.unwrap() + 1] =
                (eeprom_type >> 8) as u8;
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
        JCMD_AF_RTC_STATUS => {
            /* set type and status */
            device.pif.ram[device.pif.channels[channel].rx_buf.unwrap()] = JDT_AF_RTC as u8;
            device.pif.ram[device.pif.channels[channel].rx_buf.unwrap() + 1] =
                (JDT_AF_RTC >> 8) as u8;
            device.pif.ram[device.pif.channels[channel].rx_buf.unwrap() + 2] = 0x00;
        }
        JCMD_AF_RTC_READ => {
            af_rtc_read_block(
                device,
                device.pif.channels[channel].tx_buf.unwrap() + 1,
                device.pif.channels[channel].rx_buf.unwrap(),
                device.pif.channels[channel].rx_buf.unwrap() + 8,
            );
        }
        JCMD_AF_RTC_WRITE => af_rtc_write_block(
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

pub fn format_eeprom(device: &mut device::Device) {
    if device.ui.storage.saves.eeprom.data.len() < EEPROM_MAX_SIZE {
        device
            .ui
            .storage
            .saves
            .eeprom
            .data
            .resize(EEPROM_MAX_SIZE, 0xFF)
    }
}

fn eeprom_read_block(device: &mut device::Device, block: usize, offset: usize) {
    let address = device.pif.ram[block] as usize * EEPROM_BLOCK_SIZE;

    format_eeprom(device);

    device.pif.ram[offset..offset + EEPROM_BLOCK_SIZE].copy_from_slice(
        &device.ui.storage.saves.eeprom.data[address..address + EEPROM_BLOCK_SIZE],
    );
}

fn eeprom_write_block(device: &mut device::Device, block: usize, offset: usize, status: usize) {
    let address = device.pif.ram[block] as usize * EEPROM_BLOCK_SIZE;

    format_eeprom(device);

    device.ui.storage.saves.eeprom.data[address..address + EEPROM_BLOCK_SIZE]
        .copy_from_slice(&device.pif.ram[offset..offset + EEPROM_BLOCK_SIZE]);

    device.pif.ram[status] = 0x00;

    device.ui.storage.saves.eeprom.written = true
}

fn time2data(device: &mut device::Device, offset: usize) {
    let timestamp =
        device.cart.rtc_timestamp + (device.vi.frame_time * device.vi.vi_counter as f64) as i64;
    let now = chrono::DateTime::from_timestamp(timestamp, 0).unwrap();

    device.pif.ram[offset] = byte2bcd(now.second());
    device.pif.ram[offset + 1] = byte2bcd(now.minute());
    device.pif.ram[offset + 2] = 0x80 + byte2bcd(now.hour());
    device.pif.ram[offset + 3] = byte2bcd(now.day());
    device.pif.ram[offset + 4] = byte2bcd(now.weekday().num_days_from_sunday());
    device.pif.ram[offset + 5] = byte2bcd(now.month());
    device.pif.ram[offset + 6] = byte2bcd(now.year() as u32 - 1900);
    device.pif.ram[offset + 7] = byte2bcd((now.year() as u32 - 1900) / 100);
}

fn af_rtc_read_block(device: &mut device::Device, block: usize, offset: usize, status: usize) {
    match device.pif.ram[block] {
        0 => {
            device.pif.ram[offset] = device.cart.rtc.control as u8;
            device.pif.ram[offset + 1] = (device.cart.rtc.control >> 8) as u8;
            device.pif.ram[status] = 0x00;
        }
        1 => {
            panic!("AF-RTC reading block 1 is not implemented !");
        }
        2 => {
            time2data(device, offset);
            device.pif.ram[status] = 0x00;
        }
        _ => {
            panic!("AF-RTC read invalid block");
        }
    }
}
fn af_rtc_write_block(device: &mut device::Device, block: usize, offset: usize, status: usize) {
    match device.pif.ram[block] {
        0 => {
            device.cart.rtc.control =
                ((device.pif.ram[offset + 1] as u16) << 8) | device.pif.ram[offset] as u16;
            device.pif.ram[status] = 0x00;
        }
        1 => {
            /* block 1 read-only when control[0] is set */
            if (device.cart.rtc.control & 0x01) != 0 {
                return;
            }
            panic!("AF-RTC writing block 1 is not implemented !");
        }
        2 => {
            /* block 2 read-only when control[1] is set */
            if (device.cart.rtc.control & 0x02) != 0 {
                return;
            }

            /* TODO: implement block 2 writes */
            panic!("AF-RTC writing block 2 is not implemented !");
        }

        _ => {
            panic!("AF-RTC write invalid block");
        }
    }
}
