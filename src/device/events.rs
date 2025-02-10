use crate::{device, savestates};

#[derive(PartialEq, Copy, Clone)]
pub enum EventType {
    AI,
    SI,
    VI,
    PI,
    DP,
    SP,
    Interrupt,
    SPDma,
    Compare,
    Vru,
    PakSwitch,
    Count,
}

#[derive(PartialEq, Copy, Clone, serde::Serialize, serde::Deserialize)]
pub struct Event {
    pub enabled: bool,
    pub count: u64,
    #[serde(skip, default = "savestates::default_event_handler")]
    pub handler: fn(&mut device::Device),
}

pub fn create_event(
    device: &mut device::Device,
    name: EventType,
    when: u64,
    handler: fn(&mut device::Device),
) {
    device.cpu.events[name as usize] = Event {
        enabled: true,
        count: when,
        handler,
    };
    set_next_event(device);
}

pub fn get_event(device: &mut device::Device, name: EventType) -> Option<&mut Event> {
    if device.cpu.events[name as usize].enabled {
        return Some(&mut device.cpu.events[name as usize]);
    }
    None
}

pub fn remove_event(device: &mut device::Device, name: EventType) {
    device.cpu.events[name as usize].enabled = false;
}

pub fn trigger_event(device: &mut device::Device) {
    let next_event_name = device.cpu.next_event;
    device.cpu.events[next_event_name].enabled = false;

    let handler = device.cpu.events[next_event_name].handler;
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
