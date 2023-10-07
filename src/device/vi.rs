use crate::device;
use crate::ui;

pub const VI_STATUS_REG: u32 = 0;
//pub const VI_ORIGIN_REG: u32 = 1;
//pub const VI_WIDTH_REG: u32 = 2;
pub const VI_V_INTR_REG: u32 = 3;
pub const VI_CURRENT_REG: u32 = 4;
//pub const VI_BURST_REG: u32 = 5;
pub const VI_V_SYNC_REG: u32 = 6;
pub const VI_H_SYNC_REG: u32 = 7;
//pub const VI_LEAP_REG: u32 = 8;
//pub const VI_H_START_REG: u32 = 9;
//pub const VI_V_START_REG: u32 = 10;
//pub const VI_V_BURST_REG: u32 = 11;
//pub const VI_X_SCALE_REG: u32 = 12;
//pub const VI_Y_SCALE_REG: u32 = 13;
pub const VI_REGS_COUNT: u32 = 14;

pub struct Vi {
    pub regs: [u32; VI_REGS_COUNT as usize],
    pub clock: u64,
    pub delay: u64,
    pub field: u32,
    pub holdover: std::time::Duration,
    pub count_per_scanline: u64,
    pub last_vi_time: std::time::Instant,
    pub vi_period: std::time::Duration,
}

//static mut FRAME_COUNTER: u64 = 0;

pub fn set_expected_refresh_rate(device: &mut device::Device) {
    let expected_refresh_rate = device.vi.clock as f64
        / (device.vi.regs[VI_V_SYNC_REG as usize] + 1) as f64
        / ((device.vi.regs[VI_H_SYNC_REG as usize] & 0xFFF) + 1) as f64
        * 2.0;
    device.vi.delay = (device.cpu.clock_rate as f64 / expected_refresh_rate) as u64;
    device.vi.count_per_scanline =
        device.vi.delay / (device.vi.regs[VI_V_SYNC_REG as usize] + 1) as u64;

    device.vi.vi_period = std::time::Duration::from_secs_f64(1.0 / expected_refresh_rate);
}

pub fn set_vertical_interrupt(device: &mut device::Device) {
    if device::events::get_event(device, device::events::EventType::VI) == None
        && device.vi.regs[VI_V_INTR_REG as usize] < device.vi.regs[VI_V_SYNC_REG as usize]
    {
        device::events::create_event(
            device,
            device::events::EventType::VI,
            device.cpu.cop0.regs[device::cop0::COP0_COUNT_REG as usize] + device.vi.delay,
            vertical_interrupt_event,
        )
    }
}

pub fn set_current_line(device: &mut device::Device) {
    let delay = device.vi.delay;
    let next_vi = device::events::get_event(device, device::events::EventType::VI);
    if next_vi != None {
        device.vi.regs[VI_CURRENT_REG as usize] = ((delay
            - (next_vi.unwrap().count
                - device.cpu.cop0.regs[device::cop0::COP0_COUNT_REG as usize]))
            / device.vi.count_per_scanline)
            as u32;

        // wrap around VI_CURRENT_REG if needed
        if device.vi.regs[VI_CURRENT_REG as usize] >= device.vi.regs[VI_V_SYNC_REG as usize] {
            device.vi.regs[VI_CURRENT_REG as usize] -= device.vi.regs[VI_V_SYNC_REG as usize]
        }
    }
    /* update current field */
    device.vi.regs[VI_CURRENT_REG as usize] =
        (device.vi.regs[VI_CURRENT_REG as usize] & !1) | device.vi.field
}

pub fn read_regs(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    let reg = (address & 0xFFFF) >> 2;
    if reg as u32 == VI_CURRENT_REG {
        set_current_line(device)
    }
    return device.vi.regs[reg as usize];
}

pub fn write_regs(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    let reg = (address & 0xFFFF) >> 2;
    match reg as u32 {
        VI_CURRENT_REG => device::mi::clear_rcp_interrupt(device, device::mi::MI_INTR_VI),
        VI_V_INTR_REG => {
            if device.vi.regs[reg as usize] != value & mask {
                device::memory::masked_write_32(&mut device.vi.regs[reg as usize], value, mask);
                set_vertical_interrupt(device);
                ui::video::set_register(reg as u32, device.vi.regs[reg as usize])
            }
        }
        VI_V_SYNC_REG => {
            if device.vi.regs[reg as usize] != value & mask {
                device::memory::masked_write_32(&mut device.vi.regs[reg as usize], value, mask);
                set_vertical_interrupt(device);
                set_expected_refresh_rate(device);
                ui::video::set_register(reg as u32, device.vi.regs[reg as usize])
            }
        }
        VI_H_SYNC_REG => {
            if device.vi.regs[reg as usize] != value & mask {
                device::memory::masked_write_32(&mut device.vi.regs[reg as usize], value, mask);
                set_expected_refresh_rate(device);
                ui::video::set_register(reg as u32, device.vi.regs[reg as usize])
            }
        }
        _ => {
            device::memory::masked_write_32(&mut device.vi.regs[reg as usize], value, mask);
            ui::video::set_register(reg as u32, device.vi.regs[reg as usize])
        }
    }
}

pub fn vertical_interrupt_event(device: &mut device::Device) {
    device.cpu.running = ui::video::update_screen();

    /*
        unsafe {
            FRAME_COUNTER += 1;
        }
    */

    let elapsed = device.vi.last_vi_time.elapsed();
    if elapsed <= device.vi.vi_period {
        let sleep_time = device.vi.vi_period - elapsed;
        let remaining_holdover_space = device.vi.vi_period - device.vi.holdover; // holdover can't exceed the vi period
        if sleep_time <= remaining_holdover_space {
            device.vi.holdover += sleep_time; // donate all time to the holdover
        } else {
            device.vi.holdover += remaining_holdover_space; // max out holdover
            std::thread::sleep(sleep_time - remaining_holdover_space); // sleep the rest of the time
        }
    } else {
        let over_time = elapsed - device.vi.vi_period; // this is how much we overshot the vi period
        if over_time <= device.vi.holdover {
            device.vi.holdover -= over_time; // consume some holdover
        } else {
            device.vi.holdover -= device.vi.holdover; // consume all holdover
        }
    }
    device.vi.last_vi_time = std::time::Instant::now();

    /* toggle vi field if in interlaced mode */
    device.vi.field ^= (device.vi.regs[VI_STATUS_REG as usize] >> 6) & 0x1;

    device::mi::set_rcp_interrupt(device, device::mi::MI_INTR_VI);

    device::events::create_event(
        device,
        device::events::EventType::VI,
        device.cpu.next_event_count + device.vi.delay,
        vertical_interrupt_event,
    )
}

pub fn init(device: &mut device::Device) {
    if device.cart.pal {
        device.vi.clock = 49656530
    } else {
        device.vi.clock = 48681812
    }
    /*
    std::thread::spawn(move || {
        let mut last_count = 0;
        loop {
            unsafe {
                println!("{:?}", FRAME_COUNTER - last_count);
                last_count = FRAME_COUNTER;
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    });
    */
}
