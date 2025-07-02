use rand_chacha::rand_core::RngCore;

use crate::device;

const SI_DRAM_ADDR_REG: u32 = 0;
const SI_PIF_ADDR_RD64B_REG: u32 = 1;
//const SI_R2_REG: u32 = 2;
//const SI_R3_REG: u32 = 3;
const SI_PIF_ADDR_WR64B_REG: u32 = 4;
//const SI_R5_REG: u32 = 5;
pub const SI_STATUS_REG: u32 = 6;
pub const SI_REGS_COUNT: u32 = 7;

pub const SI_STATUS_DMA_BUSY: u32 = 1 << 0;
pub const SI_STATUS_IO_BUSY: u32 = 1 << 1;
//const SI_STATUS_DMA_ERROR: u32 = 1 << 3;
const SI_STATUS_INTERRUPT: u32 = 1 << 12;

#[derive(PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DmaDir {
    None,
    Write,
    Read,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Si {
    pub regs: [u32; SI_REGS_COUNT as usize],
    pub dma_dir: DmaDir,
}

pub fn read_regs(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    device::cop0::add_cycles(device, 20);
    device.si.regs[((address & 0xFFFF) >> 2) as usize]
}

fn randomize_interrupt_time(rng: &mut rand_chacha::ChaCha8Rng) -> u64 {
    rng.next_u64() % 0x100
}

fn dma_read(device: &mut device::Device) {
    device.si.dma_dir = DmaDir::Read;

    let duration = device::pif::update_pif_ram(device);

    device.si.regs[SI_STATUS_REG as usize] |= SI_STATUS_DMA_BUSY;

    let length = duration + randomize_interrupt_time(&mut device.rng);

    device::events::create_event(device, device::events::EVENT_TYPE_SI, length)
}

fn dma_write(device: &mut device::Device) {
    device.si.dma_dir = DmaDir::Write;

    copy_pif_rdram(device);

    device.si.regs[SI_STATUS_REG as usize] |= SI_STATUS_DMA_BUSY;

    let length = 6000 + randomize_interrupt_time(&mut device.rng); //based on https://github.com/rasky/n64-systembench

    device::events::create_event(device, device::events::EVENT_TYPE_SI, length)
}

pub fn write_regs(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    let reg = (address & 0xFFFF) >> 2;
    match reg as u32 {
        SI_STATUS_REG => {
            device.si.regs[reg as usize] &= !SI_STATUS_INTERRUPT;
            device::mi::clear_rcp_interrupt(device, device::mi::MI_INTR_SI)
        }
        SI_PIF_ADDR_RD64B_REG => dma_read(device),
        SI_PIF_ADDR_WR64B_REG => dma_write(device),
        _ => device::memory::masked_write_32(&mut device.si.regs[reg as usize], value, mask),
    }
}

//rdram is in native endian format, and pif memory is in big endian format
fn copy_pif_rdram(device: &mut device::Device) {
    let dram_addr = device.si.regs[SI_DRAM_ADDR_REG as usize] as usize & device::rdram::RDRAM_MASK;
    if device.si.dma_dir == DmaDir::Write {
        let mut i = 0;
        while i < device::pif::PIF_RAM_SIZE {
            let data = u32::from_ne_bytes(
                device
                    .rdram
                    .mem
                    .get(dram_addr + i..dram_addr + i + 4)
                    .unwrap_or_default()
                    .try_into()
                    .unwrap(),
            );
            device.pif.ram[i..i + 4].copy_from_slice(&data.to_be_bytes());
            i += 4;
        }
    } else if device.si.dma_dir == DmaDir::Read {
        let mut i = 0;
        while i < device::pif::PIF_RAM_SIZE {
            let data = u32::from_be_bytes(device.pif.ram[i..i + 4].try_into().unwrap());
            device
                .rdram
                .mem
                .get_mut(dram_addr + i..dram_addr + i + 4)
                .unwrap_or_default()
                .copy_from_slice(&data.to_ne_bytes());
            i += 4;
        }
    } else {
        panic!("si dma unknown")
    }
}

pub fn dma_event(device: &mut device::Device) {
    if device.si.dma_dir == DmaDir::Write {
        device::pif::process_ram(device);
    } else if device.si.dma_dir == DmaDir::Read {
        device::si::copy_pif_rdram(device);
    } else {
        panic!("si dma unknown")
    }
    device.si.dma_dir = DmaDir::None;
    device.si.regs[SI_STATUS_REG as usize] &= !(SI_STATUS_DMA_BUSY | SI_STATUS_IO_BUSY);
    device.si.regs[SI_STATUS_REG as usize] |= SI_STATUS_INTERRUPT;

    device::mi::set_rcp_interrupt(device, device::mi::MI_INTR_SI)
}
