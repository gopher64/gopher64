use crate::device;
use crate::ui;

pub const SRAM_MASK: usize = 0xFFFF;

pub fn read_mem(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    if device.ui.save_type.contains(&ui::storage::SaveTypes::Sram) {
        let masked_address = address as usize & SRAM_MASK;

        if masked_address + 4 > device.ui.saves.sram.len() {
            device.ui.saves.sram.resize(masked_address + 4, 0)
        }

        return u32::from_be_bytes(
            device.ui.saves.sram[masked_address..masked_address + 4]
                .try_into()
                .unwrap(),
        );
    } else {
        panic!("flash read")
    }
}

pub fn write_mem(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    if device.ui.save_type.contains(&ui::storage::SaveTypes::Sram) {
        let masked_address = address as usize & SRAM_MASK;

        if masked_address + 4 > device.ui.saves.sram.len() {
            device.ui.saves.sram.resize(masked_address + 4, 0)
        }

        let mut data = u32::from_be_bytes(
            device.ui.saves.sram[masked_address..masked_address + 4]
                .try_into()
                .unwrap(),
        );
        device::memory::masked_write_32(&mut data, value, mask);
        device.ui.saves.sram[masked_address..masked_address + 4]
            .copy_from_slice(&data.to_be_bytes());

        ui::storage::write_save(&mut device.ui, ui::storage::SaveTypes::Sram);
    } else {
        panic!("flash write")
    }
}

// cart is big endian, rdram is native endian
pub fn dma_read(device: &mut device::Device, cart_addr: u32, dram_addr: u32, length: u32) -> u64 {
    if device.ui.save_type.contains(&ui::storage::SaveTypes::Sram) {
        let mut i = dram_addr & device::rdram::RDRAM_MASK as u32;
        let mut j = cart_addr & SRAM_MASK as u32;

        if (cart_addr + length) as usize > device.ui.saves.sram.len() {
            device
                .ui
                .saves
                .sram
                .resize((cart_addr + length) as usize, 0)
        }

        while i < dram_addr + length {
            device.ui.saves.sram[j as usize] = device.rdram.mem[i as usize ^ device.byte_swap];
            i += 1;
            j += 1;
        }

        ui::storage::write_save(&mut device.ui, ui::storage::SaveTypes::Sram);
        return device::pi::calculate_cycles(device, 2, length);
    } else {
        panic!("flash dma read")
    }
}

// cart is big endian, rdram is native endian
pub fn dma_write(device: &mut device::Device, cart_addr: u32, dram_addr: u32, length: u32) -> u64 {
    if device.ui.save_type.contains(&ui::storage::SaveTypes::Sram) {
        let mut i = dram_addr & device::rdram::RDRAM_MASK as u32;
        let mut j = cart_addr & SRAM_MASK as u32;

        if (cart_addr + length) as usize > device.ui.saves.sram.len() {
            device
                .ui
                .saves
                .sram
                .resize((cart_addr + length) as usize, 0)
        }

        while i < dram_addr + length {
            device.rdram.mem[i as usize ^ device.byte_swap] = device.ui.saves.sram[j as usize];
            i += 1;
            j += 1;
        }
        return device::pi::calculate_cycles(device, 2, length);
    } else {
        panic!("flash dma write")
    }
}
