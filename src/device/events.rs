use crate::device;

#[derive(PartialEq, Copy, Clone)]
pub enum EventType {
    AI,
    SI,
    VI,
    PI,
    DP,
    SP,
    SPDma,
    Compare,
    EventTypeCount,
}

#[derive(PartialEq, Copy, Clone)]
pub struct Event {
    pub enabled: bool,
    pub count: u64,
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
        handler: handler,
    };
    set_next_event(device);
}

pub fn get_event(device: &mut device::Device, name: EventType) -> Option<&mut Event> {
    if device.cpu.events[name as usize].enabled {
        return Some(&mut device.cpu.events[name as usize]);
    }
    return None;
}

pub fn remove_event(device: &mut device::Device, name: EventType) {
    device.cpu.events[name as usize].enabled = false;
}

pub fn trigger_event(device: &mut device::Device) {
    let next_event_name = device.cpu.next_event;
    device.cpu.events[next_event_name].enabled = false;

    let handler = device.cpu.events[next_event_name].handler;
    handler(device);
    set_next_event(device);
}

pub fn set_next_event(device: &mut device::Device) {
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
        i.count = i.count.wrapping_sub(old_count).wrapping_add(new_count)
    }
    set_next_event(device);
}

pub fn dummy_event(_device: &mut device::Device) {
    panic!("dummy event")
}
