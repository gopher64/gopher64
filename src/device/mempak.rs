use crate::device;
use crate::ui;

pub const MEMPAK_SIZE: usize = 0x8000;

pub fn format_mempak(device: &mut device::Device) {
    if device.ui.saves.mempak.len() < MEMPAK_SIZE * 4 {}
}

pub fn read(device: &mut device::Device, channel: usize, address: u16, data: usize, size: usize) {
    if (address as usize) < MEMPAK_SIZE {
        format_mempak(device);

        let offset = (channel * MEMPAK_SIZE) + address as usize;
        device.pif.ram[data..data + size]
            .copy_from_slice(&device.ui.saves.mempak[offset..offset + size])
    } else {
        for i in 0..size {
            device.pif.ram[data + i] = 0;
        }
    }
}

pub fn write(device: &mut device::Device, channel: usize, address: u16, data: usize, size: usize) {
    if (address as usize) < MEMPAK_SIZE {
        format_mempak(device);

        let offset = (channel * MEMPAK_SIZE) + address as usize;
        device.ui.saves.mempak[offset..offset + size]
            .copy_from_slice(&device.pif.ram[data..data + size]);

        ui::storage::write_save(&mut device.ui, ui::storage::SaveTypes::Mempak);
    }
}
