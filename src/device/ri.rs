use crate::device;

//pub const RI_MODE_REG: u32 = 0;
//pub const RI_CONFIG_REG: u32 = 1;
//pub const RI_CURRENT_LOAD_REG: u32 = 2;
pub const RI_SELECT_REG: u32 = 3;
//pub const RI_REFRESH_REG: u32 = 4;
//pub const RI_LATENCY_REG: u32 = 5;
//pub const RI_ERROR_REG: u32 = 6;
//pub const RI_WERROR_REG: u32 = 7;
pub const RI_REGS_COUNT: u32 = 8;

pub struct Ri {
    pub regs: [u32; RI_REGS_COUNT as usize],
}

pub fn read_regs(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    if ((address & 0xFFFF) >> 2) as u32 == RI_SELECT_REG {
        return 0x14; // hack, skip RDRAM initialization
    }
    device.ri.regs[((address & 0xFFFF) >> 2) as usize]
}

pub fn write_regs(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    device::memory::masked_write_32(
        &mut device.ri.regs[((address & 0xFFFF) >> 2) as usize],
        value,
        mask,
    );
}
