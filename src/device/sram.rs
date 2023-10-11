use crate::device;
use crate::ui;

pub const SRAM_MASK: usize = 0xFFFF;
pub const SRAM_SIZE: usize = 0x8000;
//pub const FLASHRAM_SIZE: usize = 0x20000;

pub fn read_mem(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    let cycles = device::pi::calculate_cycles(device, 2, 4);
    device::cop0::add_cycles(device, cycles);

    if device.ui.save_type.contains(&ui::storage::SaveTypes::Sram) {
        let masked_address = address as usize & SRAM_MASK;

        if device.ui.saves.sram.len() < SRAM_SIZE {
            device.ui.saves.sram.resize(SRAM_SIZE, 0xFF)
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

        if device.ui.saves.sram.len() < SRAM_SIZE {
            device.ui.saves.sram.resize(SRAM_SIZE, 0xFF)
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

    device.pi.regs[device::pi::PI_STATUS_REG as usize] |= device::pi::PI_STATUS_IO_BUSY;

    let cycles = device::pi::calculate_cycles(device, 2, 4);
    device::events::create_event(
        device,
        device::events::EventType::PI,
        device.cpu.cop0.regs[device::cop0::COP0_COUNT_REG as usize] + cycles,
        device::pi::dma_event,
    );
}

// cart is big endian, rdram is native endian
pub fn dma_read(
    device: &mut device::Device,
    mut cart_addr: u32,
    mut dram_addr: u32,
    length: u32,
) -> u64 {
    if device.ui.save_type.contains(&ui::storage::SaveTypes::Sram) {
        dram_addr &= device::rdram::RDRAM_MASK as u32;
        cart_addr &= SRAM_MASK as u32;
        let mut i = dram_addr;
        let mut j = cart_addr;

        if device.ui.saves.sram.len() < SRAM_SIZE {
            device.ui.saves.sram.resize(SRAM_SIZE, 0xFF)
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
pub fn dma_write(
    device: &mut device::Device,
    mut cart_addr: u32,
    mut dram_addr: u32,
    length: u32,
) -> u64 {
    if device.ui.save_type.contains(&ui::storage::SaveTypes::Sram) {
        dram_addr &= device::rdram::RDRAM_MASK as u32;
        cart_addr &= SRAM_MASK as u32;
        let mut i = dram_addr;
        let mut j = cart_addr;

        if device.ui.saves.sram.len() < SRAM_SIZE {
            device.ui.saves.sram.resize(SRAM_SIZE, 0xFF)
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
