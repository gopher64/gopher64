use crate::device;

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
}

pub fn create_event(device: &mut device::Device, name: EventType, when: u64) {
    device.cpu.events[name as usize] = Event {
        enabled: true,
        count: when,
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

fn get_event_handler(name: usize) -> fn(&mut device::Device) {
    match name {
        name if name == EventType::AI as usize => device::ai::dma_event,
        name if name == EventType::SI as usize => device::si::dma_event,
        name if name == EventType::VI as usize => device::vi::vertical_interrupt_event,
        name if name == EventType::PI as usize => device::pi::dma_event,
        name if name == EventType::DP as usize => device::rdp::rdp_interrupt_event,
        name if name == EventType::SP as usize => device::rsp_interface::rsp_event,
        name if name == EventType::Interrupt as usize => device::exceptions::interrupt_exception,
        name if name == EventType::SPDma as usize => device::rsp_interface::fifo_pop,
        name if name == EventType::Compare as usize => device::cop0::compare_event,
        name if name == EventType::Vru as usize => device::controller::vru::vru_talking_event,
        name if name == EventType::PakSwitch as usize => device::controller::pak_switch_event,
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
