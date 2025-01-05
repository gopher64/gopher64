use crate::device;
use crate::ui;
use governor::clock::Clock;

pub const VI_STATUS_REG: u32 = 0;
//pub const VI_ORIGIN_REG: u32 = 1;
//pub const VI_WIDTH_REG: u32 = 2;
//pub const VI_V_INTR_REG: u32 = 3;
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
    pub limiter: Option<governor::DefaultDirectRateLimiter>,
    pub count_per_scanline: u64,
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

    let quota = governor::Quota::with_period(std::time::Duration::from_secs_f64(
        1.0 / expected_refresh_rate,
    ))
    .unwrap();
    device.vi.limiter = Some(governor::RateLimiter::direct(quota))
}

pub fn set_vertical_interrupt(device: &mut device::Device) {
    if device::events::get_event(device, device::events::EventType::VI).is_none() {
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
    if next_vi.is_some() {
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
    device.vi.regs[reg as usize]
}

pub fn write_regs(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    let reg = (address & 0xFFFF) >> 2;
    match reg as u32 {
        VI_CURRENT_REG => device::mi::clear_rcp_interrupt(device, device::mi::MI_INTR_VI),
        VI_V_SYNC_REG => {
            if device.vi.regs[reg as usize] != value & mask {
                device::memory::masked_write_32(&mut device.vi.regs[reg as usize], value, mask);
                set_vertical_interrupt(device);
                set_expected_refresh_rate(device);
            }
        }
        VI_H_SYNC_REG => {
            if device.vi.regs[reg as usize] != value & mask {
                device::memory::masked_write_32(&mut device.vi.regs[reg as usize], value, mask);
                set_expected_refresh_rate(device);
            }
        }
        _ => {
            device::memory::masked_write_32(&mut device.vi.regs[reg as usize], value, mask);
        }
    }
    if device.ui.config.video.lle {
        ui::video::set_register(reg as u32, device.vi.regs[reg as usize])
    }
}

pub fn vertical_interrupt_event(device: &mut device::Device) {
    device.cpu.running = ui::video::update_screen(device.ui.config.video.lle);

    /*
        unsafe {
            FRAME_COUNTER += 1;
        }
    */

    speed_limiter(device);

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

pub fn speed_limiter(device: &device::Device) {
    let result = device.vi.limiter.as_ref().unwrap().check();
    if result.is_err() {
        let outcome = result.unwrap_err();
        let dur = outcome.wait_time_from(governor::clock::DefaultClock::default().now());
        spin_sleep::sleep(dur);

        device.vi.limiter.as_ref().unwrap().check().unwrap();
    }
}
