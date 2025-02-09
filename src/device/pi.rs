use crate::device;

const PI_DRAM_ADDR_REG: u32 = 0;
const PI_CART_ADDR_REG: u32 = 1;
const PI_RD_LEN_REG: u32 = 2;
const PI_WR_LEN_REG: u32 = 3;
pub const PI_STATUS_REG: u32 = 4;
const PI_BSD_DOM1_LAT_REG: u32 = 5;
const PI_BSD_DOM1_PWD_REG: u32 = 6;
const PI_BSD_DOM1_PGS_REG: u32 = 7;
const PI_BSD_DOM1_RLS_REG: u32 = 8;
const PI_BSD_DOM2_LAT_REG: u32 = 9;
const PI_BSD_DOM2_PWD_REG: u32 = 10;
const PI_BSD_DOM2_PGS_REG: u32 = 11;
const PI_BSD_DOM2_RLS_REG: u32 = 12;
//const UNKNOWN_REG: u32 = 13; //LibDragon
pub const PI_REGS_COUNT: u32 = 14;

/* PI_STATUS - read */
const PI_STATUS_DMA_BUSY: u32 = 1 << 0;
pub const PI_STATUS_IO_BUSY: u32 = 1 << 1;
//const PI_STATUS_ERROR: u32 = 1 << 2;
const PI_STATUS_INTERRUPT: u32 = 1 << 3;

/* PI_STATUS - write */
const PI_STATUS_RESET: u32 = 1 << 0;
const PI_STATUS_CLR_INTR: u32 = 1 << 1;

pub struct Pi {
    pub regs: [u32; PI_REGS_COUNT as usize],
}

struct PiHandler {
    read: fn(&mut device::Device, u32, u32, u32) -> u64,
    write: fn(&mut device::Device, u32, u32, u32) -> u64,
}

pub fn read_regs(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    device::cop0::add_cycles(device, 20);
    let reg = (address & 0xFFFF) >> 2;
    match reg as u32 {
        PI_WR_LEN_REG | PI_RD_LEN_REG => 0x7F,
        PI_CART_ADDR_REG => device.pi.regs[reg as usize] & 0xFFFFFFFE,
        PI_DRAM_ADDR_REG => device.pi.regs[reg as usize] & 0xFFFFFE,
        _ => device.pi.regs[reg as usize],
    }
}

fn dma_read(device: &mut device::Device) {
    let handler = get_handler(device.pi.regs[PI_CART_ADDR_REG as usize]);

    let cart_addr = device.pi.regs[PI_CART_ADDR_REG as usize] & !1;
    let dram_addr = device.pi.regs[PI_DRAM_ADDR_REG as usize] & 0xFFFFFE;
    let mut length = (device.pi.regs[PI_RD_LEN_REG as usize] & 0xFFFFFF) + 1;

    /* PI seems to treat the first 128 bytes differently, see https://n64brew.dev/wiki/Peripheral_Interface#Unaligned_DMA_transfer */
    if length >= 0x7f && (length & 1) != 0 {
        length += 1;
    }

    let cycles = (handler.read)(device, cart_addr, dram_addr, length);

    device::events::create_event(
        device,
        device::events::EventType::PI,
        device.cpu.cop0.regs[device::cop0::COP0_COUNT_REG as usize] + cycles,
        dma_event,
    );

    /* Update PI_DRAM_ADDR_REG and PI_CART_ADDR_REG */
    device.pi.regs[PI_DRAM_ADDR_REG as usize] =
        (device.pi.regs[PI_DRAM_ADDR_REG as usize] + length + 7) & !7;
    device.pi.regs[PI_CART_ADDR_REG as usize] =
        (device.pi.regs[PI_CART_ADDR_REG as usize] + length + 1) & !1;

    device.pi.regs[PI_STATUS_REG as usize] |= PI_STATUS_DMA_BUSY
}

fn dma_write(device: &mut device::Device) {
    let handler = get_handler(device.pi.regs[PI_CART_ADDR_REG as usize]);

    let cart_addr = device.pi.regs[PI_CART_ADDR_REG as usize] & !1;
    let dram_addr = device.pi.regs[PI_DRAM_ADDR_REG as usize] & 0xFFFFFE;
    let mut length = (device.pi.regs[PI_WR_LEN_REG as usize] & 0xFFFFFF) + 1;

    /* PI seems to treat the first 128 bytes differently, see https://n64brew.dev/wiki/Peripheral_Interface#Unaligned_DMA_transfer */
    if length >= 0x7f && (length & 1) != 0 {
        length += 1;
    }
    if length <= 0x80 {
        length -= dram_addr & 0x7;
    }

    let cycles = (handler.write)(device, cart_addr, dram_addr, length);

    device::events::create_event(
        device,
        device::events::EventType::PI,
        device.cpu.cop0.regs[device::cop0::COP0_COUNT_REG as usize] + cycles,
        dma_event,
    );

    /* Update PI_DRAM_ADDR_REG and PI_CART_ADDR_REG */
    device.pi.regs[PI_DRAM_ADDR_REG as usize] =
        (device.pi.regs[PI_DRAM_ADDR_REG as usize] + length + 7) & !7;
    device.pi.regs[PI_CART_ADDR_REG as usize] =
        (device.pi.regs[PI_CART_ADDR_REG as usize] + length + 1) & !1;

    device.pi.regs[PI_STATUS_REG as usize] |= PI_STATUS_DMA_BUSY
}

fn get_handler(address: u32) -> PiHandler {
    let mut handler = PiHandler {
        read: device::cart::rom::dma_read,
        write: device::cart::rom::dma_write,
    };
    if address >= device::memory::MM_SC64_BUFFER as u32
        && address < (device::memory::MM_SC64_BUFFER + 0x2000) as u32
    {
        handler.read = device::cart::sc64::dma_read;
        handler.write = device::cart::sc64::dma_write;
    } else if address >= device::memory::MM_CART_ROM as u32
        && address < device::memory::MM_PIF_MEM as u32
    {
        handler.read = device::cart::rom::dma_read;
        handler.write = device::cart::rom::dma_write;
    } else if address >= device::memory::MM_DOM2_ADDR2 as u32
        && address < device::memory::MM_CART_ROM as u32
    {
        handler.read = device::cart::sram::dma_read;
        handler.write = device::cart::sram::dma_write;
    } else {
        handler.read = unknown_dma_read;
        handler.write = unknown_dma_write;
    }
    handler
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
        PI_BSD_DOM1_LAT_REG | PI_BSD_DOM1_PWD_REG | PI_BSD_DOM2_LAT_REG | PI_BSD_DOM2_PWD_REG => {
            device::memory::masked_write_32(&mut device.pi.regs[reg as usize], value & 0xFF, mask)
        }
        PI_BSD_DOM1_PGS_REG | PI_BSD_DOM2_PGS_REG => {
            device::memory::masked_write_32(&mut device.pi.regs[reg as usize], value & 0xF, mask)
        }
        PI_BSD_DOM1_RLS_REG | PI_BSD_DOM2_RLS_REG => {
            device::memory::masked_write_32(&mut device.pi.regs[reg as usize], value & 0x3, mask)
        }
        _ => device::memory::masked_write_32(&mut device.pi.regs[reg as usize], value, mask),
    }
}

pub fn calculate_cycles(device: &device::Device, domain: i32, length: u32) -> u64 {
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
    (cycles * 1.5) as u64 // Converting RCP clock speed to CPU clock speed
}

pub fn dma_event(device: &mut device::Device) {
    device.pi.regs[PI_STATUS_REG as usize] &= !(PI_STATUS_DMA_BUSY | PI_STATUS_IO_BUSY);
    device.pi.regs[PI_STATUS_REG as usize] |= PI_STATUS_INTERRUPT;

    device::mi::set_rcp_interrupt(device, device::mi::MI_INTR_PI)
}

fn unknown_dma_read(
    device: &mut device::Device,
    mut _cart_addr: u32,
    mut _dram_addr: u32,
    length: u32,
) -> u64 {
    device::pi::calculate_cycles(device, 1, length)
}

fn unknown_dma_write(
    device: &mut device::Device,
    mut _cart_addr: u32,
    mut dram_addr: u32,
    length: u32,
) -> u64 {
    dram_addr &= device::rdram::RDRAM_MASK as u32;

    for i in 0..length {
        device.rdram.mem[(dram_addr + i) as usize ^ device.byte_swap] = 0;
    }
    device::pi::calculate_cycles(device, 1, length)
}
