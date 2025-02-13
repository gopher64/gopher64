use crate::{device, netplay, ui};
use governor::clock::Clock;

const VI_STATUS_REG: u32 = 0;
const VI_ORIGIN_REG: u32 = 1;
//const VI_WIDTH_REG: u32 = 2;
//const VI_V_INTR_REG: u32 = 3;
const VI_CURRENT_REG: u32 = 4;
//const VI_BURST_REG: u32 = 5;
const VI_V_SYNC_REG: u32 = 6;
const VI_H_SYNC_REG: u32 = 7;
//const VI_LEAP_REG: u32 = 8;
//const VI_H_START_REG: u32 = 9;
//const VI_V_START_REG: u32 = 10;
//const VI_V_BURST_REG: u32 = 11;
//const VI_X_SCALE_REG: u32 = 12;
//const VI_Y_SCALE_REG: u32 = 13;
pub const VI_REGS_COUNT: u32 = 14;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Vi {
    pub regs: [u32; VI_REGS_COUNT as usize],
    pub clock: u64,
    pub delay: u64,
    pub field: u32,
    #[serde(skip)]
    pub limiter: Option<governor::DefaultDirectRateLimiter>,
    pub count_per_scanline: u64,
    pub vi_counter: u64,
    pub last_origin: u32,
    pub internal_frame_counter: u64,
    pub min_wait_time: std::time::Duration,
    pub frame_time: f64,
    pub limit_freq: u64,
    #[serde(skip)]
    #[serde(default = "std::time::Instant::now")]
    pub limit_freq_check: std::time::Instant,
}

const MAX_LIMIT_FREQ: u64 = 3;

pub fn set_expected_refresh_rate(device: &mut device::Device) {
    let expected_refresh_rate = device.vi.clock as f64
        / (device.vi.regs[VI_V_SYNC_REG as usize] + 1) as f64
        / ((device.vi.regs[VI_H_SYNC_REG as usize] & 0xFFF) + 1) as f64
        * 2.0;
    device.vi.frame_time = 1.0 / expected_refresh_rate;
    device.vi.delay = (device.cpu.clock_rate as f64 / expected_refresh_rate) as u64;
    device.vi.count_per_scanline =
        device.vi.delay / (device.vi.regs[VI_V_SYNC_REG as usize] + 1) as u64;

    create_limiter(device);
}

fn create_limiter(device: &mut device::Device) {
    let quota = governor::Quota::with_period(std::time::Duration::from_secs_f64(
        device.vi.frame_time * device.vi.limit_freq as f64,
    ))
    .unwrap();
    device.vi.limiter = Some(governor::RateLimiter::direct(quota));
    let _ = device.vi.limiter.as_ref().unwrap().check();
    //println!("new limit freq: {}", device.vi.limit_freq);
}

fn set_vertical_interrupt(device: &mut device::Device) {
    if device::events::get_event(device, device::events::EVENT_TYPE_VI).is_none() {
        device::events::create_event(
            device,
            device::events::EVENT_TYPE_VI,
            device.cpu.cop0.regs[device::cop0::COP0_COUNT_REG as usize] + device.vi.delay,
        )
    }
}

fn set_current_line(device: &mut device::Device) {
    let delay = device.vi.delay;
    let next_vi = device::events::get_event(device, device::events::EVENT_TYPE_VI);
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
    device::cop0::add_cycles(device, 20);
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
        VI_ORIGIN_REG => {
            device::memory::masked_write_32(&mut device.vi.regs[reg as usize], value, mask);
            if device.vi.regs[reg as usize] != device.vi.last_origin {
                device.vi.last_origin = device.vi.regs[reg as usize];
                device.vi.internal_frame_counter += 1;
            }
        }
        _ => {
            device::memory::masked_write_32(&mut device.vi.regs[reg as usize], value, mask);
        }
    }
    ui::video::set_register(reg as u32, device.vi.regs[reg as usize])
}

pub fn vertical_interrupt_event(device: &mut device::Device) {
    ui::video::check_callback(device);

    let mut enable_speed_limiter = true;
    if device.netplay.is_some() {
        netplay::send_sync_check(device);
        enable_speed_limiter = !device.netplay.as_ref().unwrap().fast_forward;
    }

    device.vi.vi_counter += 1;
    if device.vi.vi_counter % device.vi.limit_freq == 0 && enable_speed_limiter {
        speed_limiter(device);
    }
    ui::video::update_screen();

    /*
    let vis = if device.cart.pal { 50 } else { 60 };
    if device.vi.vi_counter % vis == 0 {
        println!("FPS: {}", device.vi.internal_frame_counter);
        device.vi.internal_frame_counter = 0;
    }
    */

    /* toggle vi field if in interlaced mode */
    device.vi.field ^= (device.vi.regs[VI_STATUS_REG as usize] >> 6) & 0x1;

    device::mi::set_rcp_interrupt(device, device::mi::MI_INTR_VI);

    device::events::create_event(
        device,
        device::events::EVENT_TYPE_VI,
        device.cpu.next_event_count + device.vi.delay,
    )
}

pub fn init(device: &mut device::Device) {
    if device.cart.pal {
        device.vi.clock = 49656530
    } else {
        device.vi.clock = 48681812
    }
}

fn speed_limiter(device: &mut device::Device) {
    let result = device.vi.limiter.as_ref().unwrap().check();
    if result.is_err() {
        let outcome = result.unwrap_err();
        let dur = outcome.wait_time_from(governor::clock::DefaultClock::default().now());
        spin_sleep::sleep(dur);
        if dur < device.vi.min_wait_time {
            device.vi.min_wait_time = dur;
        }

        let _ = device.vi.limiter.as_ref().unwrap().check();
    } else {
        device.vi.min_wait_time = std::time::Duration::from_secs(0);
    }

    if std::time::Instant::now()
        .duration_since(device.vi.limit_freq_check)
        .as_secs_f64()
        > 1.0
    {
        if device.vi.min_wait_time.as_secs_f64() == 0.0 && device.vi.limit_freq < MAX_LIMIT_FREQ {
            device.vi.limit_freq += 1;
            create_limiter(device);
        } else if device.vi.min_wait_time.as_secs_f64() > device.vi.frame_time
            && device.vi.limit_freq > 1
        {
            device.vi.limit_freq -= 1;
            create_limiter(device);
        }

        //println!("limit freq: {}", device.vi.limit_freq);
        device.vi.min_wait_time = std::time::Duration::from_secs(1);
        device.vi.limit_freq_check = std::time::Instant::now();
    }
}
