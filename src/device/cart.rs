use crate::{device, ui};
use device::pif::{JoybusAction, JoybusError, write_reply};

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

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct AfRtc {
    pub control: u16,
}

fn byte2bcd(mut n: u32) -> u8 {
    n %= 100;
    (((n / 10) << 4) | (n % 10)) as u8
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
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

impl Cart {
    /// Clone everything except the ROM, which is large and not part of the savestate.
    /// `self.rom` is left untouched.
    pub fn clone_state_from(&mut self, other: &Cart) {
        // Destructure so adding a field to Cart forces this to be updated.
        let Cart {
            rom: _,
            is_viewer_buffer,
            pal,
            latch,
            cic_seed,
            rtc,
            sc64,
            flashram,
            rtc_timestamp,
        } = other;
        self.is_viewer_buffer.clone_from(is_viewer_buffer);
        self.pal = *pal;
        self.latch = *latch;
        self.cic_seed = *cic_seed;
        self.rtc.clone_from(rtc);
        self.sc64.clone_from(sc64);
        self.flashram.clone_from(flashram);
        self.rtc_timestamp = *rtc_timestamp;
    }
}

pub fn process(
    device: &mut device::Device,
    _channel: usize,
    tx: &[u8],
    rx: &mut [u8],
) -> Result<JoybusAction, JoybusError> {
    match tx[0] {
        JCMD_RESET | JCMD_STATUS => {
            let eeprom_type = if device
                .ui
                .storage
                .save_type
                .contains(&ui::storage::SaveTypes::Eeprom16k)
            {
                JDT_EEPROM_16K
            } else if device
                .ui
                .storage
                .save_type
                .contains(&ui::storage::SaveTypes::Eeprom4k)
            {
                JDT_EEPROM_4K
            } else {
                return Err(JoybusError::NoResponse);
            };
            let [lo, hi] = eeprom_type.to_le_bytes();
            write_reply(rx, &[lo, hi, 0]);
        }
        JCMD_EEPROM_READ => {
            let [_, block, ..] = tx else {
                return Err(JoybusError::Timeout);
            };
            let address = usize::from(*block) * EEPROM_BLOCK_SIZE;
            let block = &device.ui.storage.saves.eeprom.data[address..address + EEPROM_BLOCK_SIZE];
            write_reply(rx, block);
        }
        JCMD_EEPROM_WRITE => {
            let [_, block, data @ ..] = tx else {
                return Err(JoybusError::Timeout);
            };
            let Some(data) = data.first_chunk::<EEPROM_BLOCK_SIZE>() else {
                return Err(JoybusError::Timeout);
            };
            let address = usize::from(*block) * EEPROM_BLOCK_SIZE;
            device.ui.storage.saves.eeprom.data[address..address + EEPROM_BLOCK_SIZE]
                .copy_from_slice(data);
            write_reply(rx, &[0]);
            ui::storage::schedule_save(device, ui::storage::SaveTypes::Eeprom4k);
        }
        JCMD_AF_RTC_STATUS => {
            let [lo, hi] = JDT_AF_RTC.to_le_bytes();
            write_reply(rx, &[lo, hi, 0]);
        }
        JCMD_AF_RTC_READ => {
            let [_, block, ..] = tx else {
                return Err(JoybusError::Timeout);
            };
            af_rtc_read_block(device, *block, rx);
        }
        JCMD_AF_RTC_WRITE => {
            let [_, block, data @ ..] = tx else {
                return Err(JoybusError::Timeout);
            };
            let Some(data) = data.first_chunk::<8>() else {
                return Err(JoybusError::Timeout);
            };
            af_rtc_write_block(device, *block, data, rx);
        }
        _ => {
            return Err(JoybusError::Timeout);
        }
    }
    Ok(JoybusAction::None)
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

fn rtc_time(device: &device::Device) -> [u8; 8] {
    let timestamp = device.cart.rtc_timestamp + device.vi.elapsed_time as i64;
    let utc = time::OffsetDateTime::from_unix_timestamp(timestamp).unwrap();
    let local_offset =
        time::UtcOffset::local_offset_at(utc).expect("could not determine local timezone offset");
    let now = utc.to_offset(local_offset);

    [
        byte2bcd(now.second() as u32),
        byte2bcd(now.minute() as u32),
        0x80 + byte2bcd(now.hour() as u32),
        byte2bcd(now.day() as u32),
        byte2bcd(now.weekday().number_days_from_sunday() as u32),
        byte2bcd(now.month() as u32),
        byte2bcd(now.year() as u32 - 1900),
        byte2bcd((now.year() as u32 - 1900) / 100),
    ]
}

fn af_rtc_read_block(device: &device::Device, block: u8, rx: &mut [u8]) {
    match block {
        0 => {
            write_reply(rx, &device.cart.rtc.control.to_le_bytes());
        }
        1 => {
            panic!("AF-RTC reading block 1 is not implemented !");
        }
        2 => {
            write_reply(rx, &rtc_time(device));
        }
        _ => {
            panic!("AF-RTC read invalid block");
        }
    }
    if let Some(status) = rx.get_mut(8) {
        *status = 0;
    }
}

fn af_rtc_write_block(device: &mut device::Device, block: u8, data: &[u8; 8], rx: &mut [u8]) {
    match block {
        0 => {
            device.cart.rtc.control = u16::from_le_bytes([data[0], data[1]]);
            write_reply(rx, &[0]);
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
