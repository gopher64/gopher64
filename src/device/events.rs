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
    COMPARE,
}

#[derive(PartialEq, Copy, Clone)]
pub struct Event {
    pub name: EventType,
    pub count: u64,
    pub handler: fn(&mut device::Device),
}

pub fn create_event(
    device: &mut device::Device,
    name: EventType,
    when: u64,
    handler: fn(&mut device::Device),
) {
    let event = Event {
        name: name,
        count: when,
        handler: handler,
    };
    if get_event(device, name) != None {
        panic! {"duplicate event {}", name as usize}
    }
    device.cpu.events.push(event);
    set_next_event(device);
}

pub fn get_event(device: &mut device::Device, name: EventType) -> Option<&mut Event> {
    for i in device.cpu.events.iter_mut() {
        if i.name == name {
            return Some(i);
        }
    }
    return None;
}

pub fn remove_event(device: &mut device::Device, name: EventType) {
    let mut remove_index: usize = 0;
    let mut found = false;
    for (pos, i) in device.cpu.events.iter_mut().enumerate() {
        if i.name == name {
            found = true;
            remove_index = pos;
            break;
        }
    }
    if found {
        device.cpu.events.remove(remove_index);
    }
}

pub fn trigger_event(device: &mut device::Device) {
    let next_event_name = device.cpu.next_event.unwrap().name;
    remove_event(device, next_event_name);

    let handler = device.cpu.next_event.unwrap().handler;
    handler(device);
    set_next_event(device);
}

pub fn set_next_event(device: &mut device::Device) {
    device.cpu.next_event_count = u64::MAX;
    for i in device.cpu.events.iter() {
        if i.count < device.cpu.next_event_count {
            device.cpu.next_event_count = i.count;
            device.cpu.next_event = Some(*i);
        }
    }
}

pub fn translate_events(device: &mut device::Device, old_count: u64, new_count: u64) {
    for i in device.cpu.events.iter_mut() {
        i.count = i.count - old_count + new_count
    }
    set_next_event(device);
}
