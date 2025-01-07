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

#[derive(PartialEq)]
pub enum DmaDir {
    None,
    Write,
    Read,
}

pub struct Si {
    pub regs: [u32; SI_REGS_COUNT as usize],
    pub dma_dir: DmaDir,
}

pub fn read_regs(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    device.si.regs[((address & 0xFFFF) >> 2) as usize]
}

pub fn dma_read(device: &mut device::Device) {
    device.si.dma_dir = DmaDir::Read;

    let duration = device::pif::update_pif_ram(device);

    device.si.regs[SI_STATUS_REG as usize] |= SI_STATUS_DMA_BUSY;

    device::events::create_event(
        device,
        device::events::EventType::SI,
        device.cpu.cop0.regs[device::cop0::COP0_COUNT_REG as usize] + duration,
        dma_event,
    )
}

pub fn dma_write(device: &mut device::Device) {
    device.si.dma_dir = DmaDir::Write;

    copy_pif_rdram(device);

    device.si.regs[SI_STATUS_REG as usize] |= SI_STATUS_DMA_BUSY;

    device::events::create_event(
        device,
        device::events::EventType::SI,
        device.cpu.cop0.regs[device::cop0::COP0_COUNT_REG as usize] + 6000, //based on https://github.com/rasky/n64-systembench
        dma_event,
    )
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
pub fn copy_pif_rdram(device: &mut device::Device) {
    let dram_addr = device.si.regs[SI_DRAM_ADDR_REG as usize] as usize & device::rdram::RDRAM_MASK;
    if device.si.dma_dir == DmaDir::Write {
        let mut i = 0;
        while i < device::pif::PIF_RAM_SIZE {
            let data = u32::from_ne_bytes(
                device.rdram.mem[dram_addr + i..dram_addr + i + 4]
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
            device.rdram.mem[dram_addr + i..dram_addr + i + 4].copy_from_slice(&data.to_ne_bytes());
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
