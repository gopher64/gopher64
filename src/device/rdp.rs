use crate::{device, savestates, ui};

pub const DPC_START_REG: u32 = 0;
pub const DPC_END_REG: u32 = 1;
pub const DPC_CURRENT_REG: u32 = 2;
pub const DPC_STATUS_REG: u32 = 3;
const DPC_CLOCK_REG: u32 = 4;
const DPC_BUFBUSY_REG: u32 = 5;
const DPC_PIPEBUSY_REG: u32 = 6;
const DPC_TMEM_REG: u32 = 7;
pub const DPC_REGS_COUNT: u32 = 8;

//const DPS_TBIST_REG: u32 = 0;
//const DPS_TEST_MODE_REG: u32 = 1;
//const DPS_BUFTEST_ADDR_REG: u32 = 2;
//const DPS_BUFTEST_DATA_REG: u32 = 3;
pub const DPS_REGS_COUNT: u32 = 4;

/* DPC status - read */
const DPC_STATUS_XBUS_DMEM_DMA: u32 = 1 << 0;
const DPC_STATUS_FREEZE: u32 = 1 << 1;
const DPC_STATUS_FLUSH: u32 = 1 << 2;
const DPC_STATUS_START_GCLK: u32 = 1 << 3;
const DPC_STATUS_TMEM_BUSY: u32 = 1 << 4;
const DPC_STATUS_PIPE_BUSY: u32 = 1 << 5;
const DPC_STATUS_CMD_BUSY: u32 = 1 << 6;
const DPC_STATUS_CBUF_READY: u32 = 1 << 7;
//const DPC_STATUS_DMA_BUSY: u32 = 1 << 8;
//const DPC_STATUS_END_VALID: u32 = 1 << 9;
const DPC_STATUS_START_VALID: u32 = 1 << 10;
/* DPC status - write */
const DPC_CLR_XBUS_DMEM_DMA: u32 = 1 << 0;
const DPC_SET_XBUS_DMEM_DMA: u32 = 1 << 1;
const DPC_CLR_FREEZE: u32 = 1 << 2;
const DPC_SET_FREEZE: u32 = 1 << 3;
const DPC_CLR_FLUSH: u32 = 1 << 4;
const DPC_SET_FLUSH: u32 = 1 << 5;
const DPC_CLR_TMEM_CTR: u32 = 1 << 6;
const DPC_CLR_PIPE_CTR: u32 = 1 << 7;
const DPC_CLR_CMD_CTR: u32 = 1 << 8;
const DPC_CLR_CLOCK_CTR: u32 = 1 << 9;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Rdp {
    pub regs_dpc: [u32; DPC_REGS_COUNT as usize],
    pub regs_dps: [u32; DPS_REGS_COUNT as usize],
    pub wait_frozen: bool,
    pub last_status_value: u32,
}

pub fn read_regs_dpc(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    if !device.rsp.cpu.running {
        device::cop0::add_cycles(device, 20);
    }
    let reg = (address & 0xFFFF) >> 2;
    match reg as u32 {
        DPC_STATUS_REG => {
            let value =
                device.rdp.regs_dpc[reg as usize] & (DPC_STATUS_PIPE_BUSY | DPC_STATUS_CMD_BUSY);
            if value == device.rdp.last_status_value && value != 0 {
                device.rsp.cpu.sync_point = true;
            }
            device.rdp.last_status_value = value;
            device.rdp.regs_dpc[reg as usize]
        }
        DPC_CURRENT_REG => {
            if device.rdp.wait_frozen {
                device.rsp.cpu.sync_point = true;
            }
            device.rdp.regs_dpc[reg as usize]
        }
        _ => device.rdp.regs_dpc[reg as usize],
    }
}

pub fn write_regs_dpc(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    let reg = (address & 0xFFFF) >> 2;
    match reg as u32 {
        DPC_CURRENT_REG | DPC_CLOCK_REG | DPC_BUFBUSY_REG | DPC_PIPEBUSY_REG | DPC_TMEM_REG => {}
        DPC_STATUS_REG => update_dpc_status(device, value & mask),
        DPC_START_REG => {
            if (device.rdp.regs_dpc[DPC_STATUS_REG as usize] & DPC_STATUS_START_VALID) == 0 {
                device::memory::masked_write_32(
                    &mut device.rdp.regs_dpc[reg as usize],
                    value & 0xFFFFF8,
                    mask,
                )
            }
            device.rdp.regs_dpc[DPC_STATUS_REG as usize] |= DPC_STATUS_START_VALID
        }
        DPC_END_REG => {
            device::memory::masked_write_32(
                &mut device.rdp.regs_dpc[reg as usize],
                value & 0xFFFFF8,
                mask,
            );
            if (device.rdp.regs_dpc[DPC_STATUS_REG as usize] & DPC_STATUS_START_VALID) != 0 {
                device.rdp.regs_dpc[DPC_CURRENT_REG as usize] =
                    device.rdp.regs_dpc[DPC_START_REG as usize];
                device.rdp.regs_dpc[DPC_STATUS_REG as usize] &= !DPC_STATUS_START_VALID
            }
            if device.rdp.regs_dpc[DPC_STATUS_REG as usize] & DPC_STATUS_FREEZE == 0 {
                run_rdp(device)
            } else {
                device.rdp.wait_frozen = true;
            }
        }
        _ => device::memory::masked_write_32(&mut device.rdp.regs_dpc[reg as usize], value, mask),
    }
}

fn run_rdp(device: &mut device::Device) {
    let timer = ui::video::process_rdp_list();
    device.rdp.regs_dpc[DPC_STATUS_REG as usize] |=
        DPC_STATUS_START_GCLK | DPC_STATUS_PIPE_BUSY | DPC_STATUS_CMD_BUSY;
    device.rdp.regs_dpc[DPC_CLOCK_REG as usize] = 0xFFFFFF;
    device.rdp.regs_dpc[DPC_PIPEBUSY_REG as usize] = 0xFFFFFF;

    if timer != 0 {
        if device.save_state {
            // Right after full sync, good time for save state
            device.save_state = false;
            savestates::create_savestate(device);
        } else if device.load_state {
            // Right after full sync, good time for save state
            device.load_state = false;
            savestates::load_savestate(device);
        }
        device::events::create_event(
            device,
            device::events::EVENT_TYPE_DP,
            device.cpu.cop0.regs[device::cop0::COP0_COUNT_REG as usize] + timer,
        )
    }
}

pub fn read_regs_dps(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    device::cop0::add_cycles(device, 20);
    device.rdp.regs_dps[((address & 0xFFFF) >> 2) as usize]
}

pub fn write_regs_dps(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    device::memory::masked_write_32(
        &mut device.rdp.regs_dps[((address & 0xFFFF) >> 2) as usize],
        value,
        mask,
    )
}

fn update_dpc_status(device: &mut device::Device, w: u32) {
    /* clear / set xbus_dmem_dma */
    if w & DPC_CLR_XBUS_DMEM_DMA != 0 {
        device.rdp.regs_dpc[DPC_STATUS_REG as usize] &= !DPC_STATUS_XBUS_DMEM_DMA
    }
    if w & DPC_SET_XBUS_DMEM_DMA != 0 {
        device.rdp.regs_dpc[DPC_STATUS_REG as usize] |= DPC_STATUS_XBUS_DMEM_DMA
    }

    /* clear / set freeze */
    if w & DPC_CLR_FREEZE != 0 {
        device.rdp.regs_dpc[DPC_STATUS_REG as usize] &= !DPC_STATUS_FREEZE;
        if device.rdp.wait_frozen {
            run_rdp(device);
            device.rdp.wait_frozen = false;
        }
    }
    if w & DPC_SET_FREEZE != 0 {
        device.rdp.regs_dpc[DPC_STATUS_REG as usize] |= DPC_STATUS_FREEZE;
    }

    /* clear / set flush */
    if w & DPC_CLR_FLUSH != 0 {
        device.rdp.regs_dpc[DPC_STATUS_REG as usize] &= !DPC_STATUS_FLUSH
    }
    if w & DPC_SET_FLUSH != 0 {
        device.rdp.regs_dpc[DPC_STATUS_REG as usize] |= DPC_STATUS_FLUSH
    }

    if w & DPC_CLR_TMEM_CTR != 0 {
        device.rdp.regs_dpc[DPC_STATUS_REG as usize] &= !DPC_STATUS_TMEM_BUSY;
        device.rdp.regs_dpc[DPC_TMEM_REG as usize] = 0
    }
    if w & DPC_CLR_PIPE_CTR != 0 {
        device.rdp.regs_dpc[DPC_STATUS_REG as usize] &= !DPC_STATUS_PIPE_BUSY;
        device.rdp.regs_dpc[DPC_PIPEBUSY_REG as usize] = 0
    }
    if w & DPC_CLR_CMD_CTR != 0 {
        device.rdp.regs_dpc[DPC_STATUS_REG as usize] &= !DPC_STATUS_CMD_BUSY;
        device.rdp.regs_dpc[DPC_BUFBUSY_REG as usize] = 0
    }

    /* clear clock counter */
    if w & DPC_CLR_CLOCK_CTR != 0 {
        device.rdp.regs_dpc[DPC_CLOCK_REG as usize] = 0
    }
}

pub fn init(device: &mut device::Device) {
    device.rdp.regs_dpc[DPC_STATUS_REG as usize] |=
        DPC_STATUS_START_GCLK | DPC_STATUS_PIPE_BUSY | DPC_STATUS_CBUF_READY;
    device.rdp.regs_dpc[DPC_CLOCK_REG as usize] = 0xFFFFFF;
    device.rdp.regs_dpc[DPC_PIPEBUSY_REG as usize] = 0xFFFFFF;
}

pub fn rdp_interrupt_event(device: &mut device::Device) {
    device.rdp.regs_dpc[DPC_STATUS_REG as usize] &=
        !(DPC_STATUS_START_GCLK | DPC_STATUS_PIPE_BUSY | DPC_STATUS_CMD_BUSY);

    device::mi::set_rcp_interrupt(device, device::mi::MI_INTR_DP)
}
