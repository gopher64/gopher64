use crate::device;

const MI_INIT_MODE_REG: u32 = 0;
const MI_VERSION_REG: u32 = 1;
pub const MI_INTR_REG: u32 = 2;
pub const MI_INTR_MASK_REG: u32 = 3;
pub const MI_REGS_COUNT: u32 = 4;

/* read */
pub const MI_INTR_SP: u32 = 1 << 0;
pub const MI_INTR_SI: u32 = 1 << 1;
pub const MI_INTR_AI: u32 = 1 << 2;
pub const MI_INTR_VI: u32 = 1 << 3;
pub const MI_INTR_PI: u32 = 1 << 4;
pub const MI_INTR_DP: u32 = 1 << 5;

/* write */
const MI_CLR_SP: u32 = 1 << 0;
const MI_SET_SP: u32 = 1 << 1;
const MI_CLR_SI: u32 = 1 << 2;
const MI_SET_SI: u32 = 1 << 3;
const MI_CLR_AI: u32 = 1 << 4;
const MI_SET_AI: u32 = 1 << 5;
const MI_CLR_VI: u32 = 1 << 6;
const MI_SET_VI: u32 = 1 << 7;
const MI_CLR_PI: u32 = 1 << 8;
const MI_SET_PI: u32 = 1 << 9;
const MI_CLR_DP: u32 = 1 << 10;
const MI_SET_DP: u32 = 1 << 11;

/* mode read */
const MI_INIT_MODE: u32 = 1 << 7;
const MI_EBUS_MODE: u32 = 1 << 8;
const MI_RDRAM_MODE: u32 = 1 << 9;

/* mode write */
const MI_CLR_INIT: u32 = 1 << 7;
const MI_SET_INIT: u32 = 1 << 8;
const MI_CLR_EBUS: u32 = 1 << 9;
const MI_SET_EBUS: u32 = 1 << 10;
const MI_CLR_DP_INTR: u32 = 1 << 11;
const MI_CLR_RDRAM: u32 = 1 << 12;
const MI_SET_RDRAM: u32 = 1 << 13;

const MI_INIT_LENGTH_MASK: u32 = 0b1111111;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Mi {
    pub regs: [u32; MI_REGS_COUNT as usize],
}

pub fn read_regs(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    device::cop0::add_cycles(device, 20);
    device.mi.regs[((address & 0xFFFF) >> 2) as usize]
}

pub fn write_regs(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    let reg = (address & 0xFFFF) >> 2;
    match reg as u32 {
        MI_INIT_MODE_REG => update_init_mode(device, value),
        MI_INTR_MASK_REG => update_intr_mask(device, value),
        _ => device::memory::masked_write_32(&mut device.mi.regs[reg as usize], value, mask),
    }

    if device.mi.regs[MI_INTR_REG as usize] & device.mi.regs[MI_INTR_MASK_REG as usize] == 0 {
        device.cpu.cop0.regs[device::cop0::COP0_CAUSE_REG as usize] &=
            !device::cop0::COP0_CAUSE_IP2;
    }
    device::exceptions::check_pending_interrupts(device)
}

fn update_init_mode(device: &mut device::Device, w: u32) {
    device.mi.regs[MI_INIT_MODE_REG as usize] &= !MI_INIT_LENGTH_MASK;
    device.mi.regs[MI_INIT_MODE_REG as usize] |= w & MI_INIT_LENGTH_MASK;

    if w & MI_CLR_INIT != 0 {
        device.mi.regs[MI_INIT_MODE_REG as usize] &= !MI_INIT_MODE
    }
    if w & MI_SET_INIT != 0 {
        panic!("MI_SET_INIT not implemented")
        //device.mi.regs[MI_INIT_MODE_REG as usize] |= MI_INIT_MODE
    }
    if w & MI_CLR_EBUS != 0 {
        device.mi.regs[MI_INIT_MODE_REG as usize] &= !MI_EBUS_MODE
    }
    if w & MI_SET_EBUS != 0 {
        device.mi.regs[MI_INIT_MODE_REG as usize] |= MI_EBUS_MODE
    }
    if w & MI_CLR_RDRAM != 0 {
        device.mi.regs[MI_INIT_MODE_REG as usize] &= !MI_RDRAM_MODE
    }
    if w & MI_SET_RDRAM != 0 {
        device.mi.regs[MI_INIT_MODE_REG as usize] |= MI_RDRAM_MODE
    }

    if w & MI_CLR_DP_INTR != 0 {
        clear_rcp_interrupt(device, MI_INTR_DP)
    }
}

fn update_intr_mask(device: &mut device::Device, w: u32) {
    if w & MI_CLR_SP != 0 {
        device.mi.regs[MI_INTR_MASK_REG as usize] &= !MI_INTR_SP
    }
    if w & MI_SET_SP != 0 {
        device.mi.regs[MI_INTR_MASK_REG as usize] |= MI_INTR_SP
    }
    if w & MI_CLR_SI != 0 {
        device.mi.regs[MI_INTR_MASK_REG as usize] &= !MI_INTR_SI
    }
    if w & MI_SET_SI != 0 {
        device.mi.regs[MI_INTR_MASK_REG as usize] |= MI_INTR_SI
    }
    if w & MI_CLR_AI != 0 {
        device.mi.regs[MI_INTR_MASK_REG as usize] &= !MI_INTR_AI
    }
    if w & MI_SET_AI != 0 {
        device.mi.regs[MI_INTR_MASK_REG as usize] |= MI_INTR_AI
    }
    if w & MI_CLR_VI != 0 {
        device.mi.regs[MI_INTR_MASK_REG as usize] &= !MI_INTR_VI
    }
    if w & MI_SET_VI != 0 {
        device.mi.regs[MI_INTR_MASK_REG as usize] |= MI_INTR_VI
    }
    if w & MI_CLR_PI != 0 {
        device.mi.regs[MI_INTR_MASK_REG as usize] &= !MI_INTR_PI
    }
    if w & MI_SET_PI != 0 {
        device.mi.regs[MI_INTR_MASK_REG as usize] |= MI_INTR_PI
    }
    if w & MI_CLR_DP != 0 {
        device.mi.regs[MI_INTR_MASK_REG as usize] &= !MI_INTR_DP
    }
    if w & MI_SET_DP != 0 {
        device.mi.regs[MI_INTR_MASK_REG as usize] |= MI_INTR_DP
    }
}

pub fn clear_rcp_interrupt(device: &mut device::Device, interrupt: u32) {
    device.mi.regs[MI_INTR_REG as usize] &= !interrupt;

    if device.mi.regs[MI_INTR_REG as usize] & device.mi.regs[MI_INTR_MASK_REG as usize] == 0 {
        device.cpu.cop0.regs[device::cop0::COP0_CAUSE_REG as usize] &=
            !device::cop0::COP0_CAUSE_IP2;
    }
}

pub fn set_rcp_interrupt(device: &mut device::Device, interrupt: u32) {
    device.mi.regs[MI_INTR_REG as usize] |= interrupt;
    device::exceptions::check_pending_interrupts(device)
}

pub fn init(device: &mut device::Device) {
    device.mi.regs[MI_VERSION_REG as usize] = 0x02020102
}
