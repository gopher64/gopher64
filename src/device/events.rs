use crate::device;

pub const EVENT_TYPE_NONE: usize = 0;
pub const EVENT_TYPE_AI: usize = 1;
pub const EVENT_TYPE_SI: usize = 2;
pub const EVENT_TYPE_VI: usize = 3;
pub const EVENT_TYPE_PI: usize = 4;
pub const EVENT_TYPE_DP: usize = 5;
pub const EVENT_TYPE_SP: usize = 6;
pub const EVENT_TYPE_INT: usize = 7;
pub const EVENT_TYPE_SPDMA: usize = 8;
pub const EVENT_TYPE_COMPARE: usize = 9;
pub const EVENT_TYPE_VRU: usize = 10;
pub const EVENT_TYPE_PAK: usize = 11;
pub const EVENT_TYPE_COUNT: usize = 12;

#[derive(PartialEq, Copy, Clone, serde::Serialize, serde::Deserialize)]
pub struct Event {
    pub enabled: bool,
    pub count: u64,
}

pub fn create_event(device: &mut device::Device, name: usize, when: u64) {
    device.cpu.events[name] = Event {
        enabled: true,
        count: when,
    };
    set_next_event(device);
}

pub fn get_event(device: &mut device::Device, name: usize) -> Option<&mut Event> {
    if device.cpu.events[name].enabled {
        return Some(&mut device.cpu.events[name]);
    }
    None
}

pub fn remove_event(device: &mut device::Device, name: usize) {
    device.cpu.events[name].enabled = false;
}

fn get_event_handler(name: usize) -> fn(&mut device::Device) {
    match name {
        EVENT_TYPE_AI => device::ai::dma_event,
        EVENT_TYPE_SI => device::si::dma_event,
        EVENT_TYPE_VI => device::vi::vertical_interrupt_event,
        EVENT_TYPE_PI => device::pi::dma_event,
        EVENT_TYPE_DP => device::rdp::rdp_interrupt_event,
        EVENT_TYPE_SP => device::rsp_interface::rsp_event,
        EVENT_TYPE_INT => device::exceptions::interrupt_exception,
        EVENT_TYPE_SPDMA => device::rsp_interface::fifo_pop,
        EVENT_TYPE_COMPARE => device::cop0::compare_event,
        EVENT_TYPE_VRU => device::controller::vru::vru_talking_event,
        EVENT_TYPE_PAK => device::controller::pak_switch_event,
        _ => dummy_event,
    }
}

pub fn trigger_event(device: &mut device::Device) {
    let next_event_name = device.cpu.next_event;
    device.cpu.events[next_event_name].enabled = false;

    let handler = get_event_handler(next_event_name);
    device.cpu.cop0.is_event = true;
    handler(device);
    device.cpu.cop0.is_event = false;
    set_next_event(device);
}

fn set_next_event(device: &mut device::Device) {
    device.cpu.next_event_count = u64::MAX;
    for (pos, i) in device.cpu.events.iter().enumerate() {
        if i.enabled && i.count < device.cpu.next_event_count {
            device.cpu.next_event_count = i.count;
            device.cpu.next_event = pos;
        }
    }
}

pub fn translate_events(device: &mut device::Device, old_count: u64, new_count: u64) {
    for i in device.cpu.events.iter_mut() {
        if i.enabled {
            i.count = i.count - old_count + new_count;
        }
    }
    set_next_event(device);
}

pub fn dummy_event(_device: &mut device::Device) {
    panic!("dummy event")
}
