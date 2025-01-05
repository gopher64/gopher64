use crate::device;
use crate::ui;

pub const SP_MEM_ADDR_REG: u32 = 0;
pub const SP_DRAM_ADDR_REG: u32 = 1;
pub const SP_RD_LEN_REG: u32 = 2;
pub const SP_WR_LEN_REG: u32 = 3;
pub const SP_STATUS_REG: u32 = 4;
pub const SP_DMA_FULL_REG: u32 = 5;
pub const SP_DMA_BUSY_REG: u32 = 6;
pub const SP_SEMAPHORE_REG: u32 = 7;
pub const SP_REGS_COUNT: u32 = 8;

pub const SP_PC_REG: u32 = 0;
//pub const SP_IBIST_REG: u32 = 1;
pub const SP_REGS2_COUNT: u32 = 2;

/* SP_STATUS - read */
pub const SP_STATUS_HALT: u32 = 1 << 0;
pub const SP_STATUS_BROKE: u32 = 1 << 1;
pub const SP_STATUS_DMA_BUSY: u32 = 1 << 2;
pub const SP_STATUS_DMA_FULL: u32 = 1 << 3;
//pub const SP_STATUS_IO_FULL: u32 = 1 << 4;
pub const SP_STATUS_SSTEP: u32 = 1 << 5;
pub const SP_STATUS_INTR_BREAK: u32 = 1 << 6;
pub const SP_STATUS_SIG0: u32 = 1 << 7;
pub const SP_STATUS_SIG1: u32 = 1 << 8;
pub const SP_STATUS_SIG2: u32 = 1 << 9;
pub const SP_STATUS_SIG3: u32 = 1 << 10;
pub const SP_STATUS_SIG4: u32 = 1 << 11;
pub const SP_STATUS_SIG5: u32 = 1 << 12;
pub const SP_STATUS_SIG6: u32 = 1 << 13;
pub const SP_STATUS_SIG7: u32 = 1 << 14;

/* SP_STATUS - write */
pub const SP_CLR_HALT: u32 = 1 << 0;
pub const SP_SET_HALT: u32 = 1 << 1;
pub const SP_CLR_BROKE: u32 = 1 << 2;
pub const SP_CLR_INTR: u32 = 1 << 3;
pub const SP_SET_INTR: u32 = 1 << 4;
pub const SP_CLR_SSTEP: u32 = 1 << 5;
pub const SP_SET_SSTEP: u32 = 1 << 6;
pub const SP_CLR_INTR_BREAK: u32 = 1 << 7;
pub const SP_SET_INTR_BREAK: u32 = 1 << 8;
pub const SP_CLR_SIG0: u32 = 1 << 9;
pub const SP_SET_SIG0: u32 = 1 << 10;
pub const SP_CLR_SIG1: u32 = 1 << 11;
pub const SP_SET_SIG1: u32 = 1 << 12;
pub const SP_CLR_SIG2: u32 = 1 << 13;
pub const SP_SET_SIG2: u32 = 1 << 14;
pub const SP_CLR_SIG3: u32 = 1 << 15;
pub const SP_SET_SIG3: u32 = 1 << 16;
pub const SP_CLR_SIG4: u32 = 1 << 17;
pub const SP_SET_SIG4: u32 = 1 << 18;
pub const SP_CLR_SIG5: u32 = 1 << 19;
pub const SP_SET_SIG5: u32 = 1 << 20;
pub const SP_CLR_SIG6: u32 = 1 << 21;
pub const SP_SET_SIG6: u32 = 1 << 22;
pub const SP_CLR_SIG7: u32 = 1 << 23;
pub const SP_SET_SIG7: u32 = 1 << 24;

pub const RSP_MEM_MASK: usize = 0x1FFF;

#[derive(PartialEq, Copy, Clone)]
pub enum DmaDir {
    None,
    Write,
    Read,
}

#[derive(Copy, Clone)]
pub struct RspDma {
    pub dir: DmaDir,
    pub length: u32,
    pub memaddr: u32,
    pub dramaddr: u32,
}

pub struct Rsp {
    pub cpu: device::rsp_cpu::Cpu,
    pub regs: [u32; SP_REGS_COUNT as usize],
    pub regs2: [u32; SP_REGS2_COUNT as usize],
    pub mem: [u8; 0x2000],
    pub fifo: [RspDma; 2],
    pub hle_task: bool,
}

pub fn read_mem_fast(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    let masked_address = address as usize & RSP_MEM_MASK;
    u32::from_be_bytes(
        device.rsp.mem[masked_address..masked_address + 4]
            .try_into()
            .unwrap(),
    )
}

pub fn read_mem(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    let masked_address = address as usize & RSP_MEM_MASK;
    u32::from_be_bytes(
        device.rsp.mem[masked_address..masked_address + 4]
            .try_into()
            .unwrap(),
    )
}

pub fn write_mem(device: &mut device::Device, address: u64, value: u32, _mask: u32) {
    let masked_address = address as usize & RSP_MEM_MASK;
    let mut data = u32::from_be_bytes(
        device.rsp.mem[masked_address..masked_address + 4]
            .try_into()
            .unwrap(),
    );
    device::memory::masked_write_32(&mut data, value, 0xFFFFFFFF);
    device.rsp.mem[masked_address..masked_address + 4].copy_from_slice(&data.to_be_bytes());

    if masked_address & 0x1000 != 0 {
        // imem being updated
        device.rsp.cpu.instructions[(masked_address & 0xFFF) / 4].func =
            device::rsp_cpu::decode_opcode(device, data);
        device.rsp.cpu.instructions[(masked_address & 0xFFF) / 4].opcode = data;
    }

    // SH/SB are broken: They overwrite the whole 32 bit, filling everything that isn't written with zeroes
}

pub fn do_dma(device: &mut device::Device, dma: RspDma) {
    let l = dma.length;

    let length = ((l & 0xfff) | 7) + 1;
    let count = ((l >> 12) & 0xff) + 1;
    let skip = (l >> 20) & 0xff8;

    let mut mem_addr = dma.memaddr & 0xff8;
    let mut dram_addr = dma.dramaddr & 0xfffff8;
    let offset = dma.memaddr & 0x1000;

    if dma.dir == DmaDir::Read {
        let mut j = 0;
        while j < count {
            let mut i = 0;
            while i < length {
                let data = u32::from_be_bytes(
                    device.rsp.mem[(offset + (mem_addr & 0xFFF)) as usize
                        ..(offset + (mem_addr & 0xFFF)) as usize + 4]
                        .try_into()
                        .unwrap(),
                );
                device.rdram.mem[dram_addr as usize..dram_addr as usize + 4]
                    .copy_from_slice(&data.to_ne_bytes());
                mem_addr += 4;
                dram_addr += 4;
                i += 4;
            }
            dram_addr += skip;
            j += 1;
        }
    } else {
        let mut j = 0;
        while j < count {
            let mut i = 0;
            while i < length {
                let mut data = 0;
                if dram_addr < device::rdram::RDRAM_SIZE as u32 {
                    data = u32::from_ne_bytes(
                        device.rdram.mem[dram_addr as usize..dram_addr as usize + 4]
                            .try_into()
                            .unwrap(),
                    );
                }
                if offset != 0 {
                    // imem being updated
                    device.rsp.cpu.instructions[((mem_addr & 0xFFF) / 4) as usize].func =
                        device::rsp_cpu::decode_opcode(device, data);
                    device.rsp.cpu.instructions[((mem_addr & 0xFFF) / 4) as usize].opcode = data;
                }
                device.rsp.mem[(offset + (mem_addr & 0xFFF)) as usize
                    ..(offset + (mem_addr & 0xFFF)) as usize + 4]
                    .copy_from_slice(&data.to_be_bytes());
                mem_addr += 4;
                dram_addr += 4;
                i += 4;
            }
            dram_addr += skip;
            j += 1;
        }
    }
    device.rsp.regs[SP_MEM_ADDR_REG as usize] = (mem_addr & 0xfff) + (dma.memaddr & 0x1000);
    device.rsp.regs[SP_DRAM_ADDR_REG as usize] = dram_addr;
    device.rsp.regs[SP_RD_LEN_REG as usize] = 0xff8;
    device.rsp.regs[SP_WR_LEN_REG as usize] = 0xff8;

    device::events::create_event(
        device,
        device::events::EventType::SPDma,
        device.cpu.cop0.regs[device::cop0::COP0_COUNT_REG as usize]
            + device::rdram::rdram_calculate_cycles((count * length) as u64)
            + 9,
        device::rsp_interface::fifo_pop,
    );
}

pub fn fifo_push(device: &mut device::Device, dir: DmaDir) {
    if device.rsp.regs[SP_DMA_FULL_REG as usize] != 0 {
        panic!("RSP DMA already full")
    }

    if device.rsp.regs[SP_DMA_BUSY_REG as usize] != 0 {
        device.rsp.fifo[1].dir = dir;
        if dir == DmaDir::Read {
            device.rsp.fifo[1].length = device.rsp.regs[SP_WR_LEN_REG as usize]
        } else {
            device.rsp.fifo[1].length = device.rsp.regs[SP_RD_LEN_REG as usize]
        }
        device.rsp.fifo[1].memaddr = device.rsp.regs[SP_MEM_ADDR_REG as usize];
        device.rsp.fifo[1].dramaddr = device.rsp.regs[SP_DRAM_ADDR_REG as usize];
        device.rsp.regs[SP_DMA_FULL_REG as usize] = 1;
        device.rsp.regs[SP_STATUS_REG as usize] |= SP_STATUS_DMA_FULL
    } else {
        device.rsp.fifo[0].dir = dir;
        if dir == DmaDir::Read {
            device.rsp.fifo[0].length = device.rsp.regs[SP_WR_LEN_REG as usize]
        } else {
            device.rsp.fifo[0].length = device.rsp.regs[SP_RD_LEN_REG as usize]
        }
        device.rsp.fifo[0].memaddr = device.rsp.regs[SP_MEM_ADDR_REG as usize];
        device.rsp.fifo[0].dramaddr = device.rsp.regs[SP_DRAM_ADDR_REG as usize];
        device.rsp.regs[SP_DMA_BUSY_REG as usize] = 1;
        device.rsp.regs[SP_STATUS_REG as usize] |= SP_STATUS_DMA_BUSY;

        do_dma(device, device.rsp.fifo[0])
    }
}

pub fn fifo_pop(device: &mut device::Device) {
    if device.rsp.regs[SP_DMA_FULL_REG as usize] != 0 {
        device.rsp.fifo[0].dir = device.rsp.fifo[1].dir;
        device.rsp.fifo[0].length = device.rsp.fifo[1].length;
        device.rsp.fifo[0].memaddr = device.rsp.fifo[1].memaddr;
        device.rsp.fifo[0].dramaddr = device.rsp.fifo[1].dramaddr;
        device.rsp.regs[SP_DMA_FULL_REG as usize] = 0;
        device.rsp.regs[SP_STATUS_REG as usize] &= !SP_STATUS_DMA_FULL;

        do_dma(device, device.rsp.fifo[0])
    } else {
        device.rsp.regs[SP_DMA_BUSY_REG as usize] = 0;
        device.rsp.regs[SP_STATUS_REG as usize] &= !SP_STATUS_DMA_BUSY
    }
}

pub fn read_regs(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    let reg = (address & 0xFFFF) >> 2;
    match reg as u32 {
        SP_SEMAPHORE_REG => {
            let value = device.rsp.regs[reg as usize];
            device.rsp.regs[reg as usize] = 1;
            value
        }
        _ => device.rsp.regs[reg as usize],
    }
}

pub fn write_regs(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    let reg = (address & 0xFFFF) >> 2;
    match reg as u32 {
        SP_STATUS_REG => update_sp_status(device, value),
        SP_RD_LEN_REG => {
            device::memory::masked_write_32(&mut device.rsp.regs[reg as usize], value, mask);
            fifo_push(device, DmaDir::Write)
        }
        SP_WR_LEN_REG => {
            device::memory::masked_write_32(&mut device.rsp.regs[reg as usize], value, mask);
            fifo_push(device, DmaDir::Read)
        }
        SP_SEMAPHORE_REG => {
            device::memory::masked_write_32(&mut device.rsp.regs[reg as usize], 0, mask)
        }
        _ => {
            device::memory::masked_write_32(&mut device.rsp.regs[reg as usize], value, mask);
        }
    }
}

pub fn read_regs2(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    device.rsp.regs2[((address & 0xFFFF) >> 2) as usize]
}

pub fn write_regs2(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    let reg = (address & 0xFFFF) >> 2;
    match reg as u32 {
        SP_PC_REG => {
            device::memory::masked_write_32(
                &mut device.rsp.regs2[reg as usize],
                value & 0xFFC,
                mask,
            );
        }
        _ => device::memory::masked_write_32(&mut device.rsp.regs2[reg as usize], value, mask),
    }
}

pub fn update_sp_status(device: &mut device::Device, w: u32) {
    let was_halted = device.rsp.regs[SP_STATUS_REG as usize] & SP_STATUS_HALT != 0;

    /* clear / set halt */
    if w & SP_CLR_HALT != 0 && w & SP_SET_HALT == 0 {
        device.rsp.regs[SP_STATUS_REG as usize] &= !SP_STATUS_HALT
    }
    if w & SP_SET_HALT != 0 && w & SP_CLR_HALT == 0 {
        device::events::remove_event(device, device::events::EventType::SP);
        device.rsp.regs[SP_STATUS_REG as usize] |= SP_STATUS_HALT
    }

    /* clear broke */
    if w & SP_CLR_BROKE != 0 {
        device.rsp.regs[SP_STATUS_REG as usize] &= !SP_STATUS_BROKE
    }

    /* clear SP interrupt */
    if (w & SP_CLR_INTR) != 0 && (w & SP_SET_INTR) == 0 {
        device::mi::clear_rcp_interrupt(device, device::mi::MI_INTR_SP)
    }
    /* set SP interrupt */
    if (w & SP_SET_INTR) != 0 && (w & SP_CLR_INTR) == 0 {
        device::mi::schedule_rcp_interrupt(device, device::mi::MI_INTR_SP);
    }

    /* clear / set single step */
    if w & SP_CLR_SSTEP != 0 && w & SP_SET_SSTEP == 0 {
        device.rsp.regs[SP_STATUS_REG as usize] &= !SP_STATUS_SSTEP
    }
    if w & SP_SET_SSTEP != 0 && w & SP_CLR_SSTEP == 0 {
        device.rsp.regs[SP_STATUS_REG as usize] |= SP_STATUS_SSTEP
    }

    /* clear / set interrupt on break */
    if w & SP_CLR_INTR_BREAK != 0 && w & SP_SET_INTR_BREAK == 0 {
        device.rsp.regs[SP_STATUS_REG as usize] &= !SP_STATUS_INTR_BREAK
    }
    if w & SP_SET_INTR_BREAK != 0 && w & SP_CLR_INTR_BREAK == 0 {
        device.rsp.regs[SP_STATUS_REG as usize] |= SP_STATUS_INTR_BREAK
    }

    /* clear / set signal 0 */
    if w & SP_CLR_SIG0 != 0 && w & SP_SET_SIG0 == 0 {
        device.rsp.regs[SP_STATUS_REG as usize] &= !SP_STATUS_SIG0
    }
    if w & SP_SET_SIG0 != 0 && w & SP_CLR_SIG0 == 0 {
        device.rsp.regs[SP_STATUS_REG as usize] |= SP_STATUS_SIG0
    }

    /* clear / set signal 1 */
    if w & SP_CLR_SIG1 != 0 && w & SP_SET_SIG1 == 0 {
        device.rsp.regs[SP_STATUS_REG as usize] &= !SP_STATUS_SIG1
    }
    if w & SP_SET_SIG1 != 0 && w & SP_CLR_SIG1 == 0 {
        device.rsp.regs[SP_STATUS_REG as usize] |= SP_STATUS_SIG1
    }

    /* clear / set signal 2 */
    if w & SP_CLR_SIG2 != 0 && w & SP_SET_SIG2 == 0 {
        device.rsp.regs[SP_STATUS_REG as usize] &= !SP_STATUS_SIG2
    }
    if w & SP_SET_SIG2 != 0 && w & SP_CLR_SIG2 == 0 {
        device.rsp.regs[SP_STATUS_REG as usize] |= SP_STATUS_SIG2
    }

    /* clear / set signal 3 */
    if w & SP_CLR_SIG3 != 0 && w & SP_SET_SIG3 == 0 {
        device.rsp.regs[SP_STATUS_REG as usize] &= !SP_STATUS_SIG3
    }
    if w & SP_SET_SIG3 != 0 && w & SP_CLR_SIG3 == 0 {
        device.rsp.regs[SP_STATUS_REG as usize] |= SP_STATUS_SIG3
    }

    /* clear / set signal 4 */
    if w & SP_CLR_SIG4 != 0 && w & SP_SET_SIG4 == 0 {
        device.rsp.regs[SP_STATUS_REG as usize] &= !SP_STATUS_SIG4
    }
    if w & SP_SET_SIG4 != 0 && w & SP_CLR_SIG4 == 0 {
        device.rsp.regs[SP_STATUS_REG as usize] |= SP_STATUS_SIG4
    }

    /* clear / set signal 5 */
    if w & SP_CLR_SIG5 != 0 && w & SP_SET_SIG5 == 0 {
        device.rsp.regs[SP_STATUS_REG as usize] &= !SP_STATUS_SIG5
    }
    if w & SP_SET_SIG5 != 0 && w & SP_CLR_SIG5 == 0 {
        device.rsp.regs[SP_STATUS_REG as usize] |= SP_STATUS_SIG5
    }

    /* clear / set signal 6 */
    if w & SP_CLR_SIG6 != 0 && w & SP_SET_SIG6 == 0 {
        device.rsp.regs[SP_STATUS_REG as usize] &= !SP_STATUS_SIG6
    }
    if w & SP_SET_SIG6 != 0 && w & SP_CLR_SIG6 == 0 {
        device.rsp.regs[SP_STATUS_REG as usize] |= SP_STATUS_SIG6
    }

    /* clear / set signal 7 */
    if w & SP_CLR_SIG7 != 0 && w & SP_SET_SIG7 == 0 {
        device.rsp.regs[SP_STATUS_REG as usize] &= !SP_STATUS_SIG7
    }
    if w & SP_SET_SIG7 != 0 && w & SP_CLR_SIG7 == 0 {
        device.rsp.regs[SP_STATUS_REG as usize] |= SP_STATUS_SIG7
    }

    if device.rsp.regs[SP_STATUS_REG as usize] & SP_STATUS_HALT == 0 && was_halted {
        device.rsp.cpu.broken = false;
        device.rsp.cpu.halted = false;
        device.rsp.cpu.sync_point = false;
        do_task(device);
    }
}

pub fn do_task(device: &mut device::Device) {
    let timer;
    if !device.ui.config.video.lle && device.rsp.mem[0xfc0 / 4] == 1 {
        device.rsp.regs[SP_STATUS_REG as usize] |=
            SP_STATUS_SIG2 | SP_STATUS_BROKE | SP_STATUS_HALT;
        timer = ui::video::process_dlist();
        if device.rsp.regs[SP_STATUS_REG as usize] & SP_STATUS_BROKE != 0 {
            device.rsp.cpu.broken = true;
            device.rsp.regs[SP_STATUS_REG as usize] &= !SP_STATUS_BROKE
        }
        if device.rsp.regs[SP_STATUS_REG as usize] & SP_STATUS_HALT != 0 {
            device.rsp.cpu.halted = true;
            device.rsp.regs[SP_STATUS_REG as usize] &= !SP_STATUS_HALT
        }
        device.rsp.regs[SP_STATUS_REG as usize] &= !SP_STATUS_SIG2;
        device.rsp.hle_task = true
    } else {
        timer = device::rsp_cpu::run(device);
        device.rsp.hle_task = false
    }

    device::events::create_event(
        device,
        device::events::EventType::SP,
        device.cpu.cop0.regs[device::cop0::COP0_COUNT_REG as usize] + timer,
        rsp_event,
    )
}

pub fn rsp_event(device: &mut device::Device) {
    if device.rsp.cpu.broken {
        if device.rsp.hle_task {
            device.rsp.regs[SP_STATUS_REG as usize] |= SP_STATUS_SIG2
        }
        device.rsp.regs[SP_STATUS_REG as usize] |= SP_STATUS_HALT | SP_STATUS_BROKE;

        if device.rsp.regs[SP_STATUS_REG as usize] & SP_STATUS_INTR_BREAK != 0 {
            device::mi::set_rcp_interrupt(device, device::mi::MI_INTR_SP)
        }
        return;
    }
    if device.rsp.cpu.halted {
        device.rsp.regs[SP_STATUS_REG as usize] |= SP_STATUS_HALT;
        return;
    }
    device.rsp.cpu.sync_point = false;
    do_task(device)
}

pub fn init(device: &mut device::Device) {
    device.rsp.regs[SP_STATUS_REG as usize] = 1;
    device::rsp_cpu::init(device);
}
