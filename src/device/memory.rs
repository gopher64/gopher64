use crate::device;

pub const MM_RDRAM_DRAM: usize = 0x00000000;
pub const MM_RDRAM_REGS: usize = 0x03f00000;
pub const MM_RSP_MEM: usize = 0x04000000;
pub const MM_RSP_REGS: usize = 0x04040000;
pub const MM_RSP_REGS_PC: usize = 0x04080000;
pub const MM_DPC_REGS: usize = 0x04100000;
pub const MM_DPS_REGS: usize = 0x04200000;
pub const MM_MI_REGS: usize = 0x04300000;
pub const MM_VI_REGS: usize = 0x04400000;
pub const MM_AI_REGS: usize = 0x04500000;
pub const MM_PI_REGS: usize = 0x04600000;
pub const MM_RI_REGS: usize = 0x04700000;
pub const MM_SI_REGS: usize = 0x04800000;
//const MM_DOM2_ADDR1: usize = 0x05000000;
pub const MM_DOM2_ADDR2: usize = 0x08000000;
pub const MM_CART_ROM: usize = 0x10000000;
pub const MM_PIF_MEM: usize = 0x1fc00000;
pub const MM_DOM1_ADDR3: usize = 0x1fd00000;
pub const MM_IS_VIEWER: usize = 0x13ff0000;

#[derive(PartialEq)]
pub enum AccessType {
    Write,
    Read,
    Lookup,
}

#[derive(Copy, Clone)]
pub enum AccessSize {
    // None = 0,
    Word = 4,
    Dword = 8,
    Dcache = 16,
    Icache = 32,
}

pub struct Memory {
    pub fast_read: [fn(&mut device::Device, u64, AccessSize) -> u32; 0x2000], // fast_read is used for lookups that try to detect idle loops
    pub memory_map_read: [fn(&mut device::Device, u64, AccessSize) -> u32; 0x2000],
    pub memory_map_write: [fn(&mut device::Device, u64, u32, u32); 0x2000],
    pub icache: [device::cache::ICache; 512],
    pub dcache: [device::cache::DCache; 512],
}

pub fn masked_write_32(dst: &mut u32, value: u32, mask: u32) {
    *dst = (*dst & !mask) | (value & mask);
}

pub fn masked_write_64(dst: &mut u64, value: u64, mask: u64) {
    *dst = (*dst & !mask) | (value & mask);
}

pub fn translate_address(
    device: &mut device::Device,
    address: u64,
    access_type: AccessType,
) -> (u64, bool, bool) {
    let mut cached = false;
    if (address & 0xc0000000) != 0x80000000 {
        return device::tlb::get_physical_address(device, address, access_type);
    } else if address & 0x20000000 == 0 {
        cached = true;
    }
    return (address & 0x1FFFFFFF, cached, false);
}

pub fn data_read(
    device: &mut device::Device,
    phys_address: u64,
    access_size: AccessSize,
    cached: bool,
) -> u32 {
    if cached {
        device::cache::dcache_read(device, phys_address)
    } else {
        return device.memory.memory_map_read[(phys_address >> 16) as usize](
            device,
            phys_address,
            access_size,
        );
    }
}

pub fn data_write(
    device: &mut device::Device,
    phys_address: u64,
    value: u32,
    mask: u32,
    cached: bool,
) {
    if cached {
        device::cache::dcache_write(device, phys_address, value, mask)
    } else {
        device.memory.memory_map_write[(phys_address >> 16) as usize](
            device,
            phys_address,
            value,
            mask,
        )
    }
}

pub fn init(device: &mut device::Device) {
    for i in 0..0x2000 {
        if i >= MM_RDRAM_DRAM >> 16 && i <= (MM_RDRAM_DRAM + 0x03EFFFFF) >> 16 {
            device.memory.fast_read[i] = device::rdram::read_mem_fast;
            device.memory.memory_map_read[i] = device::rdram::read_mem;
            device.memory.memory_map_write[i] = device::rdram::write_mem;
        } else if i >= MM_RDRAM_REGS >> 16 && i <= (MM_RDRAM_REGS + 0xFFFFF) >> 16 {
            device.memory.memory_map_read[i] = device::rdram::read_regs;
            device.memory.memory_map_write[i] = device::rdram::write_regs;
        } else if i >= MM_RSP_MEM >> 16 && i <= (MM_RSP_MEM + 0x3FFFF) >> 16 {
            device.memory.fast_read[i] = device::rsp_interface::read_mem_fast;
            device.memory.memory_map_read[i] = device::rsp_interface::read_mem;
            device.memory.memory_map_write[i] = device::rsp_interface::write_mem;
        } else if i >= MM_RSP_REGS >> 16 && i <= (MM_RSP_REGS + 0xFFFF) >> 16 {
            device.memory.memory_map_read[i] = device::rsp_interface::read_regs;
            device.memory.memory_map_write[i] = device::rsp_interface::write_regs;
        } else if i >= MM_RSP_REGS_PC >> 16 && i <= (MM_RSP_REGS_PC + 0xFFFF) >> 16 {
            device.memory.memory_map_read[i] = device::rsp_interface::read_regs2;
            device.memory.memory_map_write[i] = device::rsp_interface::write_regs2;
        } else if i >= MM_DPC_REGS >> 16 && i <= (MM_DPC_REGS + 0xFFFF) >> 16 {
            device.memory.memory_map_read[i] = device::rdp::read_regs_dpc;
            device.memory.memory_map_write[i] = device::rdp::write_regs_dpc;
        } else if i >= MM_DPS_REGS >> 16 && i <= (MM_DPS_REGS + 0xFFFF) >> 16 {
            device.memory.memory_map_read[i] = device::rdp::read_regs_dps;
            device.memory.memory_map_write[i] = device::rdp::write_regs_dps;
        } else if i >= MM_MI_REGS >> 16 && i <= (MM_MI_REGS + 0xFFFF) >> 16 {
            device.memory.memory_map_read[i] = device::mi::read_regs;
            device.memory.memory_map_write[i] = device::mi::write_regs;
        } else if i >= MM_VI_REGS >> 16 && i <= (MM_VI_REGS + 0xFFFF) >> 16 {
            device.memory.memory_map_read[i] = device::vi::read_regs;
            device.memory.memory_map_write[i] = device::vi::write_regs;
        } else if i >= MM_AI_REGS >> 16 && i <= (MM_AI_REGS + 0xFFFF) >> 16 {
            device.memory.memory_map_read[i] = device::ai::read_regs;
            device.memory.memory_map_write[i] = device::ai::write_regs;
        } else if i >= MM_PI_REGS >> 16 && i <= (MM_PI_REGS + 0xFFFF) >> 16 {
            device.memory.memory_map_read[i] = device::pi::read_regs;
            device.memory.memory_map_write[i] = device::pi::write_regs;
        } else if i >= MM_RI_REGS >> 16 && i <= (MM_RI_REGS + 0xFFFF) >> 16 {
            device.memory.memory_map_read[i] = device::ri::read_regs;
            device.memory.memory_map_write[i] = device::ri::write_regs;
        } else if i >= MM_SI_REGS >> 16 && i <= (MM_SI_REGS + 0xFFFF) >> 16 {
            device.memory.memory_map_read[i] = device::si::read_regs;
            device.memory.memory_map_write[i] = device::si::write_regs;
        } else if i >= MM_DOM2_ADDR2 >> 16 && i <= (MM_DOM2_ADDR2 + 0x1FFFF) >> 16 {
            device.memory.memory_map_read[i] = device::sram::read_mem;
            device.memory.memory_map_write[i] = device::sram::write_mem;
        } else if i >= MM_CART_ROM >> 16 && i <= (MM_CART_ROM + device.cart.rom.len() - 1) >> 16 {
            device.memory.fast_read[i] = device::cart_rom::read_mem_fast;
            device.memory.memory_map_read[i] = device::cart_rom::read_mem;
            device.memory.memory_map_write[i] = device::cart_rom::write_mem;
        } else if i >= MM_IS_VIEWER >> 16 && i <= (MM_IS_VIEWER + 0xFFFF) >> 16 {
            device.memory.memory_map_read[i] = device::is_viewer::read_mem;
            device.memory.memory_map_write[i] = device::is_viewer::write_mem;
        } else if i >= MM_PIF_MEM >> 16 && i <= (MM_PIF_MEM + 0xFFFF) >> 16 {
            device.memory.memory_map_read[i] = device::pif::read_mem;
            device.memory.memory_map_write[i] = device::pif::write_mem;
        }
    }
    device::cache::init(device)
}
