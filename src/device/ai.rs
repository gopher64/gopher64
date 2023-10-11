use crate::device;
use crate::ui;

pub const AI_DRAM_ADDR_REG: u32 = 0;
pub const AI_LEN_REG: u32 = 1;
//pub const AI_CONTROL_REG: u32 = 2;
pub const AI_STATUS_REG: u32 = 3;
pub const AI_DACRATE_REG: u32 = 4;
//pub const AI_BITRATE_REG: u32 = 5;
pub const AI_REGS_COUNT: u32 = 6;

pub const AI_STATUS_BUSY: u32 = 0x40000000;
pub const AI_STATUS_FULL: u32 = 0x80000000;

pub struct Ai {
    pub regs: [u32; AI_REGS_COUNT as usize],
    pub fifo: [AiDma; 2],
    pub last_read: u64,
    pub delayed_carry: bool,
}

#[derive(Copy, Clone)]
pub struct AiDma {
    pub address: u64,
    pub length: u64,
    pub duration: u64,
}

pub fn get_remaining_dma_length(device: &mut device::Device) -> u64 {
    if device.ai.fifo[0].duration == 0 {
        return 0;
    }

    let next_ai_event = device::events::get_event(device, device::events::EventType::AI);
    if next_ai_event == None {
        return 0;
    }

    let remaining_dma_duration =
        next_ai_event.unwrap().count - device.cpu.cop0.regs[device::cop0::COP0_COUNT_REG as usize];

    let dma_length = remaining_dma_duration * device.ai.fifo[0].length / device.ai.fifo[0].duration;
    return dma_length & !7;
}

pub fn get_dma_duration(device: &mut device::Device) -> u64 {
    let samples_per_sec = device.vi.clock / (1 + device.ai.regs[AI_DACRATE_REG as usize]) as u64;
    let bytes_per_sample = 4; /* XXX: assume 16bit stereo - should depends on bitrate instead */
    let length = (device.ai.regs[AI_LEN_REG as usize] & !7) as u64;

    return length * (device.cpu.clock_rate / (bytes_per_sample * samples_per_sec));
}

pub fn do_dma(device: &mut device::Device) {
    device.ai.last_read = device.ai.fifo[0].length;

    if device.ai.delayed_carry {
        device.ai.fifo[0].address += 0x2000;
    }

    if ((device.ai.fifo[0].address + device.ai.fifo[0].length) & 0x1FFF) == 0 {
        device.ai.delayed_carry = true;
    } else {
        device.ai.delayed_carry = false;
    }

    /* schedule end of dma event */
    device::events::create_event(
        device,
        device::events::EventType::AI,
        device.cpu.cop0.regs[device::cop0::COP0_COUNT_REG as usize] + device.ai.fifo[0].duration,
        dma_event,
    );
    device::mi::schedule_rcp_interrupt(device, device::mi::MI_INTR_AI);
}

pub fn fifo_push(device: &mut device::Device) {
    let duration = get_dma_duration(device);

    if (device.ai.regs[AI_STATUS_REG as usize] & AI_STATUS_BUSY) != 0 {
        device.ai.fifo[1].address =
            device.ai.regs[AI_DRAM_ADDR_REG as usize] as u64 & device::rdram::RDRAM_MASK as u64;
        device.ai.fifo[1].length = (device.ai.regs[AI_LEN_REG as usize] & !7) as u64;
        device.ai.fifo[1].duration = duration;
        device.ai.regs[AI_STATUS_REG as usize] |= AI_STATUS_FULL;
    } else {
        device.ai.fifo[0].address =
            device.ai.regs[AI_DRAM_ADDR_REG as usize] as u64 & device::rdram::RDRAM_MASK as u64;
        device.ai.fifo[0].length = (device.ai.regs[AI_LEN_REG as usize] & !7) as u64;
        device.ai.fifo[0].duration = duration;
        device.ai.regs[AI_STATUS_REG as usize] |= AI_STATUS_BUSY;

        do_dma(device);
    }
}

pub fn fifo_pop(device: &mut device::Device) {
    if device.ai.regs[AI_STATUS_REG as usize] & AI_STATUS_FULL != 0 {
        device.ai.fifo[0].address = device.ai.fifo[1].address;
        device.ai.fifo[0].length = device.ai.fifo[1].length;
        device.ai.fifo[0].duration = device.ai.fifo[1].duration;
        device.ai.regs[AI_STATUS_REG as usize] &= !AI_STATUS_FULL;

        do_dma(device);
    } else {
        device.ai.regs[AI_STATUS_REG as usize] &= !AI_STATUS_BUSY;
        device.ai.delayed_carry = false;
    }
}

pub fn read_regs(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    let reg = (address & 0xFFFF) >> 2;
    match reg as u32 {
        AI_LEN_REG => {
            let value = get_remaining_dma_length(device);
            if value < device.ai.last_read {
                let diff = device.ai.fifo[0].length - device.ai.last_read;

                ui::audio::play_audio(
                    device,
                    (device.ai.fifo[0].address + diff) as usize,
                    device.ai.last_read - value,
                );

                device.ai.last_read = value;
            }
            return value as u32;
        }
        _ => return device.ai.regs[reg as usize],
    }
}

pub fn write_regs(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    let reg = (address & 0xFFFF) >> 2;
    match reg as u32 {
        AI_LEN_REG => {
            device::memory::masked_write_32(&mut device.ai.regs[reg as usize], value, mask);
            if device.ai.regs[reg as usize] != 0 {
                fifo_push(device)
            }
        }
        AI_STATUS_REG => device::mi::clear_rcp_interrupt(device, device::mi::MI_INTR_AI),
        AI_DACRATE_REG => {
            if device.ai.regs[reg as usize] != value & mask {
                let frequency = device.vi.clock / (1 + (value & mask)) as u64;
                ui::audio::init(&mut device.ui, frequency)
            }
            device::memory::masked_write_32(&mut device.ai.regs[reg as usize], value, mask)
        }
        _ => device::memory::masked_write_32(&mut device.ai.regs[reg as usize], value, mask),
    }
}

pub fn dma_event(device: &mut device::Device) {
    if device.ai.last_read != 0 {
        let diff = device.ai.fifo[0].length - device.ai.last_read;
        ui::audio::play_audio(
            device,
            (device.ai.fifo[0].address + diff) as usize,
            device.ai.last_read,
        );
        device.ai.last_read = 0;
    }

    fifo_pop(device);
}
