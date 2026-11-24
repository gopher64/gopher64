use crate::{cheats, device, netplay, retroachievements, savestates, ui};

const VI_STATUS_REG: usize = 0;
const VI_ORIGIN_REG: usize = 1;
//const VI_WIDTH_REG: usize = 2;
const VI_V_INTR_REG: usize = 3;
const VI_CURRENT_REG: usize = 4;
//const VI_BURST_REG: usize = 5;
const VI_V_SYNC_REG: usize = 6;
const VI_H_SYNC_REG: usize = 7;
//const VI_LEAP_REG: usize = 8;
//const VI_H_START_REG: usize = 9;
const VI_V_START_REG: usize = 10;
//const VI_V_BURST_REG: usize = 11;
//const VI_X_SCALE_REG: usize = 12;
//const VI_Y_SCALE_REG: usize = 13;
const VI_REGS_COUNT: usize = 14;
const VI_V_MASK: u32 = 0x3ff;
const VI_TYPE_MASK: u32 = 3;
const MAX_LIMIT_FREQ: u64 = 3;

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Vi {
    pub regs: [u32; VI_REGS_COUNT],
    pub clock: u64,
    // Timing for the current field. A zero half-line count means the VI is off.
    half_lines: u64,
    field_cycles: u64,
    scanout_field: u64,
    pub elapsed_time: f64,
}

impl Vi {
    fn configured_half_lines(&self) -> u64 {
        let v_sync = self.regs[VI_V_SYNC_REG] & VI_V_MASK;
        if self.regs[VI_STATUS_REG] & VI_TYPE_MASK == 0 || v_sync == 0 {
            0
        } else {
            u64::from(v_sync) + 1
        }
    }

    fn current_field(&self) -> u64 {
        // The scanline counter alternates fields only with an odd half-line total.
        self.scanout_field & self.half_lines
    }

    fn cycles_at_line(&self, line: u64) -> u64 {
        (line * self.field_cycles).div_ceil(self.half_lines)
    }
}

pub fn frame_time(device: &device::Device) -> f64 {
    device.vi.field_cycles as f64 / device.cpu.clock_rate as f64
}

fn schedule_field(device: &mut device::Device, start: u64) {
    device.vi.elapsed_time +=
        elapsed_field_cycles(device, start) as f64 / device.cpu.clock_rate as f64;

    let vi = &mut device.vi;
    let was_active = vi.half_lines != 0;
    vi.half_lines = vi.configured_half_lines();
    vi.scanout_field = if was_active && vi.half_lines != 0 {
        vi.scanout_field ^ 1
    } else {
        0
    };
    vi.field_cycles = if vi.half_lines == 0 {
        // Keep screen updates and host callbacks running with the VI off.
        device.cpu.clock_rate / if device.cart.pal { 50 } else { 60 }
    } else {
        let h_total = u64::from((vi.regs[VI_H_SYNC_REG] & 0xfff) + 1);
        vi.half_lines * h_total * device.cpu.clock_rate / (2 * vi.clock)
    };
    let end = start + vi.field_cycles;
    device::events::create_event_at(device, device::events::EVENT_TYPE_VI_FIELD, end);
    schedule_vertical_interrupt(device, start);
}

fn field_end(device: &device::Device) -> u64 {
    // The scheduler also translates this deadline when the guest writes COUNT.
    device.cpu.events[device::events::EVENT_TYPE_VI_FIELD].count
}

fn elapsed_field_cycles(device: &device::Device, now: u64) -> u64 {
    device
        .vi
        .field_cycles
        .saturating_sub(field_end(device).saturating_sub(now))
}

fn schedule_vertical_interrupt(device: &mut device::Device, earliest: u64) {
    let vi = &device.vi;
    let compare = u64::from(vi.regs[VI_V_INTR_REG] & VI_V_MASK);
    let field = vi.current_field();
    let mut line = compare & !1;
    // An even compare (including the SDKs' line 2) fires one scanline early in odd fields.
    if field == 1 && compare == line {
        line = line.saturating_sub(2);
    }
    if line + field < vi.half_lines {
        let count = field_end(device).saturating_sub(vi.field_cycles - vi.cycles_at_line(line));
        if count >= earliest {
            device::events::create_event_at(device, device::events::EVENT_TYPE_VI_INTERRUPT, count);
            return;
        }
    }
    device::events::remove_event(device, device::events::EVENT_TYPE_VI_INTERRUPT);
}

fn reprogram_vertical_interrupt(device: &mut device::Device, now: u64) {
    // An earlier event can leave an already-matched VI interrupt undispatched.
    if device::events::get_event(device, device::events::EVENT_TYPE_VI_INTERRUPT)
        .is_some_and(|event| event.count <= now)
    {
        vertical_interrupt_event(device);
    }
    schedule_vertical_interrupt(device, now + 1);
}

fn current_line(device: &device::Device) -> u32 {
    let vi = &device.vi;
    if vi.half_lines == 0 {
        return 0;
    }
    let now = device.cpu.cop0.regs[device::cop0::COP0_COUNT_REG];
    let elapsed = elapsed_field_cycles(device, now);
    let field = vi.current_field();
    let line = elapsed * vi.half_lines / vi.field_cycles;
    ((line.min(vi.half_lines - 1 - field) & !1) | field) as u32
}

pub fn read_regs(
    device: &mut device::Device,
    address: u64,
    _access_size: device::memory::AccessSize,
) -> u32 {
    let reg = ((address & 0xffff) >> 2) as usize;
    let value = if reg == VI_CURRENT_REG {
        current_line(device)
    } else {
        device.vi.regs[reg]
    };
    device::cop0::add_cycles(device, 20);
    value
}

pub fn write_regs(device: &mut device::Device, address: u64, value: u32, mask: u32) {
    let reg = ((address & 0xffff) >> 2) as usize;
    if reg == VI_CURRENT_REG {
        device::mi::clear_rcp_interrupt(device, device::mi::MI_INTR_VI);
        return;
    }

    let old = device.vi.regs[reg];
    device::memory::masked_write_32(&mut device.vi.regs[reg], value, mask);
    let value = device.vi.regs[reg];
    let changed = old ^ value;
    let now = device.cpu.cop0.regs[device::cop0::COP0_COUNT_REG];
    match reg {
        VI_STATUS_REG | VI_V_SYNC_REG => {
            // Latch timing at the field boundary unless the VI starts or stops.
            let active = device.vi.configured_half_lines() != 0;
            if active != (device.vi.half_lines != 0) {
                schedule_field(device, now);
            }
        }
        VI_V_INTR_REG if changed & VI_V_MASK != 0 => {
            reprogram_vertical_interrupt(device, now);
        }
        VI_ORIGIN_REG if changed != 0 && !netplay::in_rollback(device.netplay.as_ref()) => {
            let _ = device.ui.video.fps_tx.as_ref().unwrap().send(true);
        }
        _ => {}
    }
    ui::video::set_register(reg as u32, value);
}

pub fn update_screen(device: &mut device::Device) {
    let in_rollback = netplay::in_rollback(device.netplay.as_ref());
    if !in_rollback {
        // Weave serrated output in the renderer even if the guest counter is progressive.
        ui::video::set_register(VI_CURRENT_REG as u32, device.vi.scanout_field as u32);
        ui::video::render_frame();
    }

    let (speed_limiter_toggled, paused) = ui::video::check_callback(device);

    if speed_limiter_toggled {
        reset_pace_deadline(device);
    }

    if !in_rollback
        && device
            .ui
            .speed_limiter
            .frame_counter
            .is_multiple_of(device.ui.speed_limiter.limit_freq)
        && device.ui.speed_limiter.enabled
    {
        speed_limiter(device);
    }

    if !in_rollback {
        ui::video::update_screen();
        device.ui.speed_limiter.frame_counter += 1;
        let _ = device.ui.video.vis_tx.as_ref().unwrap().send(true);
    }

    // Loading restores CPU state, so don't let a pending load undo a stop.
    if device.cpu.running && device.netplay.is_none() && device.bk2_playback.is_none() {
        savestates::process_savestates(device);
    }

    if device.netplay.is_some() {
        device.netplay.as_mut().unwrap().inputs = netplay::process_requests(device);
    } else if paused {
        if retroachievements::get_hardcore() {
            ui::video::onscreen_message(
                "Cannot pause in RA hardcore mode",
                ui::video::MESSAGE_LENGTH_MESSAGE_SHORT,
            );
        } else {
            let seconds = frame_time(device);
            ui::video::pause_loop(&mut device.ui, seconds);
        }
    }
}

pub fn field_event(device: &mut device::Device) {
    let start = device.cpu.next_event_count;
    if device.vi.regs[VI_V_SYNC_REG] & VI_V_MASK != 0
        && let Some(playback) = &mut device.bk2_playback
    {
        playback.advance();
    }

    if device.vi.half_lines != 0 {
        if device.cheats.enabled {
            cheats::execute_cheats(device);
        }
        retroachievements::do_frame();
    }

    schedule_field(device, start);
    let vi = &device.vi;
    let v_start = u64::from((vi.regs[VI_V_START_REG] >> 16) & VI_V_MASK);
    // Let the vblank handler update the framebuffer before scanout.
    let offset = if v_start < vi.half_lines {
        vi.cycles_at_line(v_start)
    } else {
        0
    };
    device::events::create_event_at(
        device,
        device::events::EVENT_TYPE_UPDATE_SCREEN,
        start + offset,
    );
}

pub fn vertical_interrupt_event(device: &mut device::Device) {
    device::mi::set_rcp_interrupt(device, device::mi::MI_INTR_VI);
}

pub fn init(device: &mut device::Device) {
    device.vi.clock = if device.cart.pal { 49656530 } else { 48681812 };
    device.vi.regs[VI_V_INTR_REG] = VI_V_MASK;
    device.vi.regs[VI_H_SYNC_REG] = 0x7ff;
    schedule_field(device, device.cpu.cop0.regs[device::cop0::COP0_COUNT_REG]);
}

fn reset_pace_deadline(device: &mut device::Device) {
    device.ui.speed_limiter.next_pace_deadline = None;
}

fn speed_limiter(device: &mut device::Device) {
    let seconds = frame_time(device);
    let mut speed_limiter_toggled = false;
    let mut interval =
        std::time::Duration::from_secs_f64(seconds * device.ui.speed_limiter.limit_freq as f64);

    if let Some(netplay) = &device.netplay {
        let ahead = netplay.session.frames_ahead();
        if ahead > 0 {
            interval = interval.mul_f64(1.0 + 0.05 * ahead.min(2) as f64);
        } else if ahead < 0 {
            interval = interval.mul_f64(1.0 - 0.05 * (-ahead).min(2) as f64);
        }
    }

    let now = std::time::Instant::now();
    match device.ui.speed_limiter.next_pace_deadline {
        None => {
            device.ui.speed_limiter.next_pace_deadline = Some(now + interval);
            speed_limiter_toggled = true;
        }
        Some(deadline) => {
            if now < deadline {
                let dur = deadline - now;
                spin_sleep::sleep(dur);

                if dur < device.ui.speed_limiter.min_wait_time {
                    device.ui.speed_limiter.min_wait_time = dur;
                }
            } else {
                //println!("did not sleep");
                device.ui.speed_limiter.min_wait_time = std::time::Duration::from_secs(0);
            }
            let mut next = deadline + interval;
            let t = std::time::Instant::now();
            while next <= t {
                next += interval;
            }
            device.ui.speed_limiter.next_pace_deadline = Some(next);
        }
    }

    if std::time::Instant::now().duration_since(device.ui.speed_limiter.limit_freq_check)
        > std::time::Duration::from_secs(1)
    {
        if !speed_limiter_toggled {
            if device.ui.speed_limiter.min_wait_time == std::time::Duration::from_secs(0)
                && device.ui.speed_limiter.limit_freq < MAX_LIMIT_FREQ
            {
                device.ui.speed_limiter.limit_freq += 1;
                reset_pace_deadline(device);
            } else if device.ui.speed_limiter.min_wait_time
                > std::time::Duration::from_secs_f64(seconds)
                && device.ui.speed_limiter.limit_freq > 1
            {
                device.ui.speed_limiter.limit_freq -= 1;
                reset_pace_deadline(device);
            }
        }

        //println!("limit freq: {}", device.vi.limit_freq);
        device.ui.speed_limiter.min_wait_time = std::time::Duration::from_secs(1);
        device.ui.speed_limiter.limit_freq_check = std::time::Instant::now();
    }
}
