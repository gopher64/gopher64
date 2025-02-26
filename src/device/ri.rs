use crate::device;

pub const RI_MODE_REG: u32 = 0;
pub const RI_CONFIG_REG: u32 = 1;
//const RI_CURRENT_LOAD_REG: u32 = 2;
pub const RI_SELECT_REG: u32 = 3;
pub const RI_REFRESH_REG: u32 = 4;
//const RI_LATENCY_REG: u32 = 5;
//const RI_ERROR_REG: u32 = 6;
//const RI_WERROR_REG: u32 = 7;
pub const RI_REGS_COUNT: u32 = 8;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Ri {
    pub regs: [u32; RI_REGS_COUNT as usize],
    pub ram_init: bool,
}

pub fn read_regs(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    device::cop0::add_cycles(device, 20);
    let reg = (address & 0xFFFF) >> 2;
    match reg as u32 {
        RI_SELECT_REG => {
            if !device.ri.ram_init {
                device::cop0::add_cycles(device, device.cpu.clock_rate / 2); // hack, simulate RDRAM initialization
                device.ri.ram_init = true;
            }
            0x14 // hack, skip RDRAM initialization
        }
        RI_REFRESH_REG => 0x00063634, // hack, skip RDRAM initialization
        _ => device.ri.regs[reg as usize],
    }
}

pub fn write_regs(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    let reg = (address & 0xFFFF) >> 2;
    if reg as u32 == RI_SELECT_REG {
        device.ri.ram_init = false;
    }
    device::memory::masked_write_32(&mut device.ri.regs[reg as usize], value, mask);
}
