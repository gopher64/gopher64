use crate::device;
use crate::ui;
use rand::Rng;

const SI_DRAM_ADDR_REG: usize = 0;
const SI_PIF_ADDR_RD64B_REG: usize = 1;
//const SI_R2_REG: usize = 2;
//const SI_R3_REG: usize = 3;
const SI_PIF_ADDR_WR64B_REG: usize = 4;
//const SI_R5_REG: usize = 5;
pub const SI_STATUS_REG: usize = 6;
pub const SI_REGS_COUNT: usize = 7;

const SI_STATUS_DMA_BUSY: u32 = 1 << 0;
const SI_STATUS_IO_BUSY: u32 = 1 << 1;
//const SI_STATUS_DMA_ERROR: u32 = 1 << 3;
const SI_STATUS_INTERRUPT: u32 = 1 << 12;
// Approximate SI transport timings from n64-systembench, in CPU cycles.
// Joybus setup runs at write completion without charging extra PIF firmware time.
const SI_WRITE_CYCLES: u64 = 6000;
const SI_WRITE_ROM_CYCLES: u64 = 3100;
const SI_IO_WRITE_CYCLES: u64 = 3200;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum Transfer {
    Read(usize),
    Write,
    WriteRom,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Si {
    pub regs: [u32; SI_REGS_COUNT],
    pub transfer: Option<Transfer>,
}

pub fn read_regs(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    device::cop0::add_cycles(device, 20);
    device.si.regs[((address & 0xFFFF) >> 2) as usize]
}

fn randomize_interrupt_time(rng: &mut rand::rngs::Xoshiro256PlusPlus) -> u64 {
    rng.next_u64() % 0x100
}

fn start_transfer(device: &mut device::Device, transfer: Transfer, cycles: u64, io_status: u32) {
    device.si.transfer = Some(transfer);
    device.si.regs[SI_STATUS_REG] |= SI_STATUS_DMA_BUSY | io_status;
    device::events::create_event(device, device::events::EVENT_TYPE_SI, cycles);
}

fn dram_addr(device: &device::Device) -> usize {
    device.si.regs[SI_DRAM_ADDR_REG] as usize & device::rdram::RDRAM_MASK
}

fn dma_read(device: &mut device::Device) {
    let addr = dram_addr(device);
    let cycles = device::pif::update_pif_ram(device) + randomize_interrupt_time(&mut device.rng);
    start_transfer(device, Transfer::Read(addr), cycles, 0);
}

fn dma_write(device: &mut device::Device, pif_addr: u32) {
    let addr = dram_addr(device);
    let (transfer, cycles) = if let Some((source_offset, ram)) =
        device::pif::dma_write_target(&mut device.pif, pif_addr)
    {
        // Snapshot the packet now so direct PIF reads already see the new data.
        let addr = addr + source_offset;
        ui::video::check_rdram_read(addr as u32, ram.len() as u32);
        copy_rdram_to_pif(&device.rdram.mem, addr, ram);
        (Transfer::Write, SI_WRITE_CYCLES)
    } else {
        (Transfer::WriteRom, SI_WRITE_ROM_CYCLES)
    };
    let cycles = cycles + randomize_interrupt_time(&mut device.rng);
    start_transfer(device, transfer, cycles, 0);
}

pub fn io_write(device: &mut device::Device) {
    start_transfer(
        device,
        Transfer::Write,
        SI_IO_WRITE_CYCLES,
        SI_STATUS_IO_BUSY,
    );
}

pub fn write_regs(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    let reg = (address & 0xFFFF) >> 2;
    match reg as usize {
        SI_STATUS_REG => {
            device.si.regs[reg as usize] &= !SI_STATUS_INTERRUPT;
            device::mi::clear_rcp_interrupt(device, device::mi::MI_INTR_SI)
        }
        SI_PIF_ADDR_RD64B_REG => dma_read(device),
        // The RCP sees the entire bus word even for a byte or halfword store.
        SI_PIF_ADDR_WR64B_REG => dma_write(device, value),
        _ => device::memory::masked_write_32(&mut device.si.regs[reg as usize], value, mask),
    }
}

// RDRAM is in native endian format; PIF RAM is big endian.
fn copy_rdram_to_pif(rdram: &[u8], addr: usize, ram: &mut [u8]) {
    let (words, _) = ram.as_chunks_mut::<4>();
    for (i, word) in words.iter_mut().enumerate() {
        let offset = addr + i * 4;
        let data = rdram.get(offset..offset + 4).unwrap_or(&[0; 4]);
        *word = u32::from_ne_bytes(data.try_into().unwrap()).to_be_bytes();
    }
}

fn copy_pif_to_rdram(device: &mut device::Device, addr: usize) {
    ui::video::check_rdram_write(addr as u32, device::pif::PIF_RAM_SIZE as u32);
    let (words, _) = device.pif.ram.as_chunks::<4>();
    for (i, word) in words.iter().enumerate() {
        let offset = addr + i * 4;
        if let Some(dst) = device.rdram.mem.get_mut(offset..offset + 4) {
            dst.copy_from_slice(&u32::from_be_bytes(*word).to_ne_bytes());
        }
    }
}

pub fn dma_event(device: &mut device::Device) {
    match device
        .si
        .transfer
        .take()
        .expect("SI event without a transfer")
    {
        Transfer::Read(addr) => copy_pif_to_rdram(device, addr),
        Transfer::Write => device::pif::process_ram(device),
        Transfer::WriteRom => {}
    }
    device.si.regs[SI_STATUS_REG] &= !(SI_STATUS_DMA_BUSY | SI_STATUS_IO_BUSY);
    device.si.regs[SI_STATUS_REG] |= SI_STATUS_INTERRUPT;

    device::mi::set_rcp_interrupt(device, device::mi::MI_INTR_SI)
}
