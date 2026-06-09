use crate::{cheats, device, netplay, retroachievements, savestates, ui};

const VI_STATUS_REG: usize = 0;
const VI_ORIGIN_REG: usize = 1;
//const VI_WIDTH_REG: usize = 2;
//const VI_V_INTR_REG: usize = 3;
const VI_CURRENT_REG: usize = 4;
//const VI_BURST_REG: usize = 5;
const VI_V_SYNC_REG: usize = 6;
const VI_H_SYNC_REG: usize = 7;
//const VI_LEAP_REG: usize = 8;
//const VI_H_START_REG: usize = 9;
//const VI_V_START_REG: usize = 10;
//const VI_V_BURST_REG: usize = 11;
//const VI_X_SCALE_REG: usize = 12;
//const VI_Y_SCALE_REG: usize = 13;
pub const VI_REGS_COUNT: usize = 14;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Vi {
    pub regs: [u32; VI_REGS_COUNT],
    pub clock: u64,
    pub delay: u64,
    pub field: u32,
    pub count_per_scanline: u64,
    pub frame_time: f64,
    pub elapsed_time: f64,
}

const MAX_LIMIT_FREQ: u64 = 3;

pub fn set_expected_refresh_rate(device: &mut device::Device) {
    let expected_refresh_rate = device.vi.clock as f64
        / (device.vi.regs[VI_V_SYNC_REG] + 1) as f64
        / ((device.vi.regs[VI_H_SYNC_REG] & 0xFFF) + 1) as f64
        * 2.0;
    device.vi.frame_time = 1.0 / expected_refresh_rate;
    device.vi.delay = (device.cpu.clock_rate as f64 / expected_refresh_rate) as u64;
    device.vi.count_per_scanline = device.vi.delay / (device.vi.regs[VI_V_SYNC_REG] + 1) as u64;
}

fn reset_pace_deadline(device: &mut device::Device) {
    device.speed_limiter.next_pace_deadline = None;
}

fn set_vertical_interrupt(device: &mut device::Device) {
    if device::events::get_event(device, device::events::EVENT_TYPE_VI).is_none() {
        device::events::create_event(device, device::events::EVENT_TYPE_VI, device.vi.delay)
    }
}

fn set_current_line(device: &mut device::Device) {
    if let Some(next_vi) = device::events::get_event(device, device::events::EVENT_TYPE_VI) {
        device.vi.regs[VI_CURRENT_REG] =
            ((device.vi.delay.saturating_sub(
                next_vi.count - device.cpu.cop0.regs[device::cop0::COP0_COUNT_REG],
            )) / device.vi.count_per_scanline) as u32;

        // wrap around VI_CURRENT_REG if needed
        if device.vi.regs[VI_CURRENT_REG] >= device.vi.regs[VI_V_SYNC_REG] {
            device.vi.regs[VI_CURRENT_REG] -= device.vi.regs[VI_V_SYNC_REG]
        }
    }
    /* update current field */
    device.vi.regs[VI_CURRENT_REG] = (device.vi.regs[VI_CURRENT_REG] & !1) | device.vi.field;
    ui::video::set_register(VI_CURRENT_REG as u32, device.vi.regs[VI_CURRENT_REG])
}

pub fn read_regs(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    let reg = (address & 0xFFFF) >> 2;
    if reg as usize == VI_CURRENT_REG {
        set_current_line(device)
    }
    device::cop0::add_cycles(device, 20);
    device.vi.regs[reg as usize]
}

pub fn write_regs(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    let reg = (address & 0xFFFF) >> 2;
    match reg as usize {
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
            let current_origin = device.vi.regs[reg as usize];
            device::memory::masked_write_32(&mut device.vi.regs[reg as usize], value, mask);
            if current_origin != device.vi.regs[reg as usize] {
                if device.netplay.is_none() {
                    savestates::process_savestates(device);
                }
                let _ = device.ui.video.fps_tx.as_ref().unwrap().try_send(true);
            }
        }
        _ => {
            device::memory::masked_write_32(&mut device.vi.regs[reg as usize], value, mask);
        }
    }
    ui::video::set_register(reg as u32, device.vi.regs[reg as usize])
}

pub fn vertical_interrupt_event(device: &mut device::Device) {
    device.vi.elapsed_time += device.vi.frame_time;

    if device.cheats.enabled {
        cheats::execute_cheats(device, device.cheats.cheats.clone());
    }

    ui::video::render_frame();
    let _ = device.ui.video.vis_tx.as_ref().unwrap().try_send(true);

    retroachievements::do_frame();

    let (speed_limiter_toggled, paused) = ui::video::check_callback(device);

    if device.netplay.is_none()
        && device.ui.config.emulation.rewind
        && device.vi.elapsed_time - device.savestate.last_rewind_saved > 1.0
    {
        device.savestate.save_rewind = true;
        device.savestate.last_rewind_saved = device.vi.elapsed_time;
    }

    if speed_limiter_toggled {
        reset_pace_deadline(device);
    }

    if device
        .speed_limiter
        .frame_counter
        .is_multiple_of(device.speed_limiter.limit_freq)
        && device.speed_limiter.enabled
    {
        speed_limiter(device);
    }

    unsafe { sdl3_sys::events::SDL_PumpEvents() };
    ui::video::update_screen();
    device.speed_limiter.frame_counter += 1;

    if device.netplay.is_some() {
        device.netplay.as_mut().unwrap().inputs = netplay::process_requests(device);
    }

    if device.netplay.is_none() && paused {
        if retroachievements::get_hardcore() {
            ui::video::onscreen_message(
                "Cannot pause in RA hardcore mode",
                ui::video::MESSAGE_LENGTH_MESSAGE_SHORT,
            );
        } else {
            ui::video::pause_loop(&mut device.ui, device.vi.frame_time);
        }
    }

    /* toggle vi field if in interlaced mode */
    device.vi.field ^= (device.vi.regs[VI_STATUS_REG] >> 6) & 0x1;

    device::mi::set_rcp_interrupt(device, device::mi::MI_INTR_VI);

    device::events::create_event_at(
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
    let mut speed_limiter_toggled = false;
    let mut interval = std::time::Duration::from_secs_f64(
        device.vi.frame_time * device.speed_limiter.limit_freq as f64,
    );

    if let Some(netplay) = &device.netplay {
        let ahead = netplay.session.frames_ahead();
        if ahead > 0 {
            interval = interval.mul_f64(1.0 + 0.05 * ahead.min(2) as f64);
        } else if ahead < 0 {
            interval = interval.mul_f64(1.0 - 0.05 * (-ahead).min(2) as f64);
        }
    }

    let now = std::time::Instant::now();
    match device.speed_limiter.next_pace_deadline {
        None => {
            device.speed_limiter.next_pace_deadline = Some(now + interval);
            speed_limiter_toggled = true;
        }
        Some(deadline) => {
            if now < deadline {
                let dur = deadline - now;
                spin_sleep::sleep(dur);
                if dur < device.speed_limiter.min_wait_time {
                    device.speed_limiter.min_wait_time = dur;
                }
            } else {
                //println!("did not sleep");
                device.speed_limiter.min_wait_time = std::time::Duration::from_secs(0);
            }
            let mut next = deadline + interval;
            let t = std::time::Instant::now();
            while next <= t {
                next += interval;
            }
            device.speed_limiter.next_pace_deadline = Some(next);
        }
    }

    if std::time::Instant::now().duration_since(device.speed_limiter.limit_freq_check)
        > std::time::Duration::from_secs(1)
    {
        if !speed_limiter_toggled {
            if device.speed_limiter.min_wait_time == std::time::Duration::from_secs(0)
                && device.speed_limiter.limit_freq < MAX_LIMIT_FREQ
            {
                device.speed_limiter.limit_freq += 1;
                reset_pace_deadline(device);
            } else if device.speed_limiter.min_wait_time
                > std::time::Duration::from_secs_f64(device.vi.frame_time)
                && device.speed_limiter.limit_freq > 1
            {
                device.speed_limiter.limit_freq -= 1;
                reset_pace_deadline(device);
            }
        }

        //println!("limit freq: {}", device.vi.limit_freq);
        device.speed_limiter.min_wait_time = std::time::Duration::from_secs(1);
        device.speed_limiter.limit_freq_check = std::time::Instant::now();
    }
}
