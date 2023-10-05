use crate::device;

pub const PI_DRAM_ADDR_REG: u32 = 0;
pub const PI_CART_ADDR_REG: u32 = 1;
pub const PI_RD_LEN_REG: u32 = 2;
pub const PI_WR_LEN_REG: u32 = 3;
pub const PI_STATUS_REG: u32 = 4;
pub const PI_BSD_DOM1_LAT_REG: u32 = 5;
pub const PI_BSD_DOM1_PWD_REG: u32 = 6;
pub const PI_BSD_DOM1_PGS_REG: u32 = 7;
pub const PI_BSD_DOM1_RLS_REG: u32 = 8;
pub const PI_BSD_DOM2_LAT_REG: u32 = 9;
pub const PI_BSD_DOM2_PWD_REG: u32 = 10;
pub const PI_BSD_DOM2_PGS_REG: u32 = 11;
pub const PI_BSD_DOM2_RLS_REG: u32 = 12;
pub const PI_REGS_COUNT: u32 = 13;

/* PI_STATUS - read */
pub const PI_STATUS_DMA_BUSY: u32 = 1 << 0;
pub const PI_STATUS_IO_BUSY: u32 = 1 << 1;
//pub const PI_STATUS_ERROR: u32 = 1 << 2;
pub const PI_STATUS_INTERRUPT: u32 = 1 << 3;

/* PI_STATUS - write */
pub const PI_STATUS_RESET: u32 = 1 << 0;
pub const PI_STATUS_CLR_INTR: u32 = 1 << 1;

pub struct Pi {
    pub regs: [u32; PI_REGS_COUNT as usize],
}

pub struct PiHandler {
    read: fn(&mut device::Device, u32, u32, u32) -> u64,
    write: fn(&mut device::Device, u32, u32, u32) -> u64,
}

pub fn read_regs(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    return device.pi.regs[((address & 0xFFFF) >> 2) as usize];
}

pub fn dma_read(device: &mut device::Device) {
    let handler = get_handler(device.pi.regs[PI_CART_ADDR_REG as usize]);
    let cycles = (handler.read)(
        device,
        device.pi.regs[PI_CART_ADDR_REG as usize],
        device.pi.regs[PI_DRAM_ADDR_REG as usize],
        device.pi.regs[PI_RD_LEN_REG as usize] + 1,
    );

    device::events::create_event(
        device,
        device::events::EventType::PI,
        device.cpu.cop0.regs[device::cop0::COP0_COUNT_REG as usize] + cycles,
        dma_event,
    );

    device.pi.regs[PI_STATUS_REG as usize] |= PI_STATUS_DMA_BUSY
}

pub fn dma_write(device: &mut device::Device) {
    let handler = get_handler(device.pi.regs[PI_CART_ADDR_REG as usize]);
    let cycles = (handler.write)(
        device,
        device.pi.regs[PI_CART_ADDR_REG as usize],
        device.pi.regs[PI_DRAM_ADDR_REG as usize],
        device.pi.regs[PI_WR_LEN_REG as usize] + 1,
    );

    device::events::create_event(
        device,
        device::events::EventType::PI,
        device.cpu.cop0.regs[device::cop0::COP0_COUNT_REG as usize] + cycles,
        dma_event,
    );

    device.pi.regs[PI_STATUS_REG as usize] |= PI_STATUS_DMA_BUSY
}

pub fn get_handler(address: u32) -> PiHandler {
    let mut handler = PiHandler {
        read: device::cart_rom::dma_read,
        write: device::cart_rom::dma_write,
    };
    if address >= device::memory::MM_CART_ROM as u32 {
        if address >= device::memory::MM_DOM1_ADDR3 as u32 {
            panic!("unimplemented handler")
        //RW(cart, cart_dom3);
        } else {
            handler.read = device::cart_rom::dma_read;
            handler.write = device::cart_rom::dma_write;
        }
    } else if address >= device::memory::MM_DOM2_ADDR2 as u32 {
        handler.read = device::sram::dma_read;
        handler.write = device::sram::dma_write;
    } else {
        panic!("unknown pi handler")
    }
    return handler;
}

pub fn write_regs(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    let reg = (address & 0xFFFF) >> 2;
    match reg as u32 {
        PI_RD_LEN_REG => {
            device::memory::masked_write_32(&mut device.pi.regs[reg as usize], value, mask);
            device::pi::dma_read(device)
        }
        PI_WR_LEN_REG => {
            device::memory::masked_write_32(&mut device.pi.regs[reg as usize], value, mask);
            device::pi::dma_write(device)
        }
        PI_STATUS_REG => {
            if value & mask & PI_STATUS_CLR_INTR != 0 {
                device.pi.regs[reg as usize] &= !PI_STATUS_INTERRUPT;
                device::mi::clear_rcp_interrupt(device, device::mi::MI_INTR_PI);
            }
            if value & mask & PI_STATUS_RESET != 0 {
                device.pi.regs[PI_STATUS_REG as usize] = 0
            }
        }
        PI_BSD_DOM1_LAT_REG | PI_BSD_DOM1_PWD_REG | PI_BSD_DOM1_PGS_REG | PI_BSD_DOM1_RLS_REG
        | PI_BSD_DOM2_LAT_REG | PI_BSD_DOM2_PWD_REG | PI_BSD_DOM2_PGS_REG | PI_BSD_DOM2_RLS_REG => {
            device::memory::masked_write_32(&mut device.pi.regs[reg as usize], value & 0xFF, mask)
        }
        _ => device::memory::masked_write_32(&mut device.pi.regs[reg as usize], value, mask),
    }
}

pub fn calculate_cycles(device: &mut device::Device, domain: i32, length: u32) -> u64 {
    let mut cycles: f64 = 0.0;
    let (page_size, latency, pulse_width, release, pages);
    let page_size_base: f64 = 2.0;

    if domain == 1 {
        latency = (device.pi.regs[PI_BSD_DOM1_LAT_REG as usize] + 1) as f64;
        pulse_width = (device.pi.regs[PI_BSD_DOM1_PWD_REG as usize] + 1) as f64;
        release = (device.pi.regs[PI_BSD_DOM1_RLS_REG as usize] + 1) as f64;
        page_size = page_size_base.powf((device.pi.regs[PI_BSD_DOM1_PGS_REG as usize] + 2) as f64);
    } else if domain == 2 {
        latency = (device.pi.regs[PI_BSD_DOM2_LAT_REG as usize] + 1) as f64;
        pulse_width = (device.pi.regs[PI_BSD_DOM2_PWD_REG as usize] + 1) as f64;
        release = (device.pi.regs[PI_BSD_DOM2_RLS_REG as usize] + 1) as f64;
        page_size = page_size_base.powf((device.pi.regs[PI_BSD_DOM2_PGS_REG as usize] + 2) as f64);
    } else {
        panic!("unknown pi dma")
    }
    pages = (length as f64 / page_size).ceil();

    cycles += (14.0 + latency) * pages;
    cycles += (pulse_width + release) * (length as f64 / 2.0);
    cycles += 5.0 * pages;
    return (cycles * 1.5) as u64; // Converting RCP clock speed to CPU clock speed
}

pub fn dma_event(device: &mut device::Device) {
    device.pi.regs[PI_STATUS_REG as usize] &= !(PI_STATUS_DMA_BUSY | PI_STATUS_IO_BUSY);
    device.pi.regs[PI_STATUS_REG as usize] |= PI_STATUS_INTERRUPT;

    device::mi::set_rcp_interrupt(device, device::mi::MI_INTR_PI)
}
