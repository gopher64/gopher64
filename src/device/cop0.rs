use crate::{device, savestates};

pub const COP0_INDEX_REG: u32 = 0;
const COP0_RANDOM_REG: u32 = 1;
pub const COP0_ENTRYLO0_REG: u32 = 2;
pub const COP0_ENTRYLO1_REG: u32 = 3;
pub const COP0_CONTEXT_REG: u32 = 4;
pub const COP0_PAGEMASK_REG: u32 = 5;
const COP0_WIRED_REG: u32 = 6;
//const COP0_UNUSED_7: u32 = 7;
pub const COP0_BADVADDR_REG: u32 = 8;
pub const COP0_COUNT_REG: u32 = 9;
pub const COP0_ENTRYHI_REG: u32 = 10;
const COP0_COMPARE_REG: u32 = 11;
pub const COP0_STATUS_REG: u32 = 12;
pub const COP0_CAUSE_REG: u32 = 13;
pub const COP0_EPC_REG: u32 = 14;
const COP0_PREVID_REG: u32 = 15;
const COP0_CONFIG_REG: u32 = 16;
pub const COP0_LLADDR_REG: u32 = 17;
//const COP0_WATCHLO_REG: u32 = 18;
//const COP0_WATCHHI_REG: u32 = 19;
pub const COP0_XCONTEXT_REG: u32 = 20;
//const COP0_UNUSED_21: u32 = 21;
//const COP0_UNUSED_22: u32 = 22;
//const COP0_UNUSED_23: u32 = 23;
//const COP0_UNUSED_24: u32 = 24;
//const COP0_UNUSED_25: u32 = 25;
//const COP0_PARITYERR_REG: u32 = 26;
//const COP0_CACHEERR_REG: u32 = 27;
pub const COP0_TAGLO_REG: u32 = 28;
//const COP0_TAGHI_REG: u32 = 29;
const COP0_ERROREPC_REG: u32 = 30;
//const COP0_UNUSED_31: u32 = 31;
pub const COP0_REGS_COUNT: u32 = 32;

pub const COP0_STATUS_IE: u64 = 1 << 0;
pub const COP0_STATUS_EXL: u64 = 1 << 1;
pub const COP0_STATUS_ERL: u64 = 1 << 2;
pub const COP0_STATUS_BEV: u64 = 1 << 22;
pub const COP0_STATUS_FR: u64 = 1 << 26;
pub const COP0_STATUS_CU1: u64 = 1 << 29;
pub const COP0_STATUS_CU2: u64 = 1 << 30;

//const COP0_CAUSE_EXCCODE_INTR: u64 = 0 << 2;
pub const COP0_CAUSE_EXCCODE_MOD: u64 = 1 << 2;
pub const COP0_CAUSE_EXCCODE_TLBL: u64 = 2 << 2;
pub const COP0_CAUSE_EXCCODE_TLBS: u64 = 3 << 2;
//const COP0_CAUSE_EXCCODE_ADEL: u64 = 4 << 2;
//const COP0_CAUSE_EXCCODE_ADES: u64 = 5 << 2;
pub const COP0_CAUSE_EXCCODE_SYS: u64 = 8 << 2;
pub const COP0_CAUSE_EXCCODE_BP: u64 = 9 << 2;
pub const COP0_CAUSE_EXCCODE_RI: u64 = 10 << 2;
pub const COP0_CAUSE_EXCCODE_CPU: u64 = 11 << 2;
pub const COP0_CAUSE_EXCCODE_TR: u64 = 13 << 2;
pub const COP0_CAUSE_EXCCODE_FPE: u64 = 15 << 2;
pub const COP0_CAUSE_IP2: u64 = 1 << 10;
pub const COP0_CAUSE_IP7: u64 = 1 << 15;
pub const COP0_CAUSE_BD: u64 = 1 << 31;

pub const COP0_CAUSE_CE1: u64 = 1 << 28;
pub const COP0_CAUSE_CE2: u64 = 1 << 29;

pub const COP0_CAUSE_IP_MASK: u64 = 0b00000000000000001111111100000000;
//pub const COP0_CAUSE_EXCCODE_MASK: u64 = 0x1F << 2;
pub const COP0_CONTEXT_BADVPN2_MASK: u64 = 0b00000000011111111111111111110000;
pub const COP0_XCONTEXT_BADVPN2_MASK: u64 = 0b01111111111111111111111111110000;
pub const COP0_XCONTEXT_REGION_MASK: u64 = 0b110000000000000000000000000000000;

const COP0_INDEX_REG_MASK: u64 = 0b10000000000000000000000000111111;
//const COP0_RANDOM_REG_MASK: u64 = 0b00000000000000000000000000111111;
const COP0_ENTRYLO_REG_MASK: u64 =
    0b0000000000000000000000000000000000111111111111111111111111111111;
const COP0_CONTEXT_REG_MASK: u64 =
    0b1111111111111111111111111111111111111111100000000000000000000000;
const COP0_PAGEMASK_REG_MASK: u64 = 0b00000001111111111110000000000000;
const COP0_WIRED_REG_MASK: u64 = 0b00000000000000000000000000111111;
const COP0_ENTRYHI_REG_MASK: u64 =
    0b1100000000000000000000001111111111111111111111111110000011111111;
const COP0_STATUS_REG_MASK: u64 = 0b11111111010101111111111111111111;
const COP0_CAUSE_REG_MASK: u64 = 0b00000000000000000000001100000000;
const COP0_CONFIG_REG_MASK: u64 = 0b00001111000000001000000000001111;
const COP0_LLADDR_REG_MASK: u64 = 0b11111111111111111111111111111111;
const COP0_WATCHLO_REG_MASK: u64 = 0b11111111111111111111111111111011;
const COP0_WATCHHI_REG_MASK: u64 = 0b00000000000000000000000000001111;
const COP0_XCONTEXT_REG_MASK: u64 =
    0b1111111111111111111111111111111000000000000000000000000000000000;
const COP0_PARITYERR_REG_MASK: u64 = 0b00000000000000000000000011111111;
const COP0_TAGLO_REG_MASK: u64 = 0b00001111111111111111111111000000;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Cop0 {
    pub reg_latch: u64,
    pub regs: [u64; COP0_REGS_COUNT as usize],
    pub reg_write_masks: [u64; COP0_REGS_COUNT as usize],
    #[serde(skip, default = "savestates::default_instructions")]
    pub instrs: [fn(&mut device::Device, u32); 32],
    #[serde(skip, default = "savestates::default_instructions")]
    pub instrs2: [fn(&mut device::Device, u32); 32],
    pub tlb_lut_r: Vec<device::tlb::TlbLut>,
    pub tlb_lut_w: Vec<device::tlb::TlbLut>,
    pub tlb_entries: [device::tlb::TlbEntry; 32],
    pub is_event: bool,
    pub pending_compare_interrupt: bool,
}

fn mfc0(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[device::cpu_instructions::rt(opcode) as usize] = device::cpu_instructions::se32(
        (get_control_registers(device, device::cpu_instructions::rd(opcode))) as u32 as i32,
    )
}

fn dmfc0(device: &mut device::Device, opcode: u32) {
    device.cpu.gpr[device::cpu_instructions::rt(opcode) as usize] =
        get_control_registers(device, device::cpu_instructions::rd(opcode))
}

fn mtc0(device: &mut device::Device, opcode: u32) {
    device::cop0::set_control_registers(
        device,
        device::cpu_instructions::rd(opcode),
        device::cpu_instructions::se32(
            device.cpu.gpr[device::cpu_instructions::rt(opcode) as usize] as u32 as i32,
        ),
    )
}

fn dmtc0(device: &mut device::Device, opcode: u32) {
    device::cop0::set_control_registers(
        device,
        device::cpu_instructions::rd(opcode),
        device.cpu.gpr[device::cpu_instructions::rt(opcode) as usize],
    )
}

fn tlbr(device: &mut device::Device, _opcode: u32) {
    device::tlb::read(device, device.cpu.cop0.regs[COP0_INDEX_REG as usize])
}

fn tlbwi(device: &mut device::Device, _opcode: u32) {
    device::tlb::write(device, device.cpu.cop0.regs[COP0_INDEX_REG as usize])
}

fn tlbwr(device: &mut device::Device, _opcode: u32) {
    let random = set_random_register(device);
    device::tlb::write(device, random)
}

fn tlbp(device: &mut device::Device, _opcode: u32) {
    device::tlb::probe(device)
}

fn eret(device: &mut device::Device, _opcode: u32) {
    if device.cpu.cop0.regs[COP0_STATUS_REG as usize] & COP0_STATUS_ERL != 0 {
        device.cpu.pc = device.cpu.cop0.regs[COP0_ERROREPC_REG as usize];
        device.cpu.cop0.regs[COP0_STATUS_REG as usize] &= !COP0_STATUS_ERL
    } else {
        device.cpu.pc = device.cpu.cop0.regs[COP0_EPC_REG as usize];
        device.cpu.cop0.regs[COP0_STATUS_REG as usize] &= !COP0_STATUS_EXL
    }
    device.cpu.branch_state.state = device::cpu::State::Exception;
    device.cpu.llbit = false;
    device::exceptions::check_pending_interrupts(device)
}

fn execute_cp0(device: &mut device::Device, opcode: u32) {
    device.cpu.cop0.instrs2[(opcode & 0x3F) as usize](device, opcode)
}

pub fn syscall(device: &mut device::Device, _opcode: u32) {
    device::exceptions::syscall_exception(device)
}

pub fn break_(device: &mut device::Device, _opcode: u32) {
    device::exceptions::break_exception(device)
}

pub fn reserved(device: &mut device::Device, _opcode: u32) {
    device::exceptions::reserved_exception(device, 0);
}

fn get_control_registers(device: &device::Device, index: u32) -> u64 {
    match index {
        COP0_COUNT_REG => device.cpu.cop0.regs[index as usize] >> 1,
        COP0_RANDOM_REG => set_random_register(device),
        7 | 21 | 22 | 23 | 24 | 25 | 31 => device.cpu.cop0.reg_latch,
        _ => device.cpu.cop0.regs[index as usize],
    }
}

fn set_control_registers(device: &mut device::Device, index: u32, mut data: u64) {
    device.cpu.cop0.reg_latch = data;
    match index {
        COP0_COUNT_REG => {
            data &= 0xFFFFFFFF;
            data <<= 1;
            device::events::translate_events(
                device,
                device.cpu.cop0.regs[COP0_COUNT_REG as usize],
                data,
            );
            device.cpu.cop0.regs[COP0_COUNT_REG as usize] = data;
            return;
        }
        COP0_WIRED_REG => device.cpu.cop0.regs[COP0_RANDOM_REG as usize] = 31,
        COP0_COMPARE_REG => {
            data &= 0xFFFFFFFF;
            let current_count = (device.cpu.cop0.regs[COP0_COUNT_REG as usize] >> 1) & 0xFFFFFFFF;
            let mut compare_event_diff = (data as u32).wrapping_sub(current_count as u32);

            if compare_event_diff == 0 {
                compare_event_diff += u32::MAX;
            }

            device::events::create_event(
                device,
                device::events::EVENT_TYPE_COMPARE,
                device.cpu.cop0.regs[COP0_COUNT_REG as usize] + ((compare_event_diff as u64) << 1),
            );
            device.cpu.cop0.regs[COP0_CAUSE_REG as usize] &= !COP0_CAUSE_IP7;
            device.cpu.cop0.pending_compare_interrupt = false;
        }
        COP0_STATUS_REG => {
            if data & COP0_STATUS_FR != device.cpu.cop0.regs[index as usize] & COP0_STATUS_FR {
                device::cop1::set_fgr_registers(device, data)
            }
        }
        _ => {}
    }
    device::memory::masked_write_64(
        &mut device.cpu.cop0.regs[index as usize],
        data,
        device.cpu.cop0.reg_write_masks[index as usize],
    );
    device::exceptions::check_pending_interrupts(device);
}

pub fn compare_event(device: &mut device::Device) {
    device.cpu.cop0.pending_compare_interrupt = true;
    device::events::create_event(
        device,
        device::events::EVENT_TYPE_COMPARE,
        device.cpu.next_event_count + (u32::MAX as u64),
    );
    device::exceptions::check_pending_interrupts(device);
}

fn set_random_register(device: &device::Device) -> u64 {
    if device.cpu.cop0.regs[COP0_WIRED_REG as usize] > 31 {
        (u64::MAX - device.cpu.cop0.regs[COP0_COUNT_REG as usize]) & 0x3F
    } else {
        (u64::MAX - device.cpu.cop0.regs[COP0_COUNT_REG as usize])
            % (32 - device.cpu.cop0.regs[COP0_WIRED_REG as usize])
            + device.cpu.cop0.regs[COP0_WIRED_REG as usize]
    }
}

pub fn add_cycles(device: &mut device::Device, cycles: u64) {
    device.cpu.cop0.regs[COP0_COUNT_REG as usize] += cycles // COUNT_REG is shifted right 1 bit when read by MFC0
}

pub fn map_instructions(device: &mut device::Device) {
    device.cpu.cop0.instrs = [
        device::cop0::mfc0,        // 0
        device::cop0::dmfc0,       // 1
        device::cop0::reserved,    // 2
        device::cop0::reserved,    // 3
        device::cop0::mtc0,        // 4
        device::cop0::dmtc0,       // 5
        device::cop0::reserved,    // 6
        device::cop0::reserved,    // 7
        device::cop0::reserved,    // 8
        device::cop0::reserved,    // 9
        device::cop0::reserved,    // 10
        device::cop0::reserved,    // 11
        device::cop0::reserved,    // 12
        device::cop0::reserved,    // 13
        device::cop0::reserved,    // 14
        device::cop0::reserved,    // 15
        device::cop0::execute_cp0, // 16
        device::cop0::reserved,    // 17
        device::cop0::reserved,    // 18
        device::cop0::reserved,    // 19
        device::cop0::reserved,    // 20
        device::cop0::reserved,    // 21
        device::cop0::reserved,    // 22
        device::cop0::reserved,    // 23
        device::cop0::reserved,    // 24
        device::cop0::reserved,    // 25
        device::cop0::reserved,    // 26
        device::cop0::reserved,    // 27
        device::cop0::reserved,    // 28
        device::cop0::reserved,    // 29
        device::cop0::reserved,    // 30
        device::cop0::reserved,    // 31
    ];

    device.cpu.cop0.instrs2 = [
        device::cop0::reserved, // 0
        device::cop0::tlbr,     // 1
        device::cop0::tlbwi,    // 2
        device::cop0::reserved, // 3
        device::cop0::reserved, // 4
        device::cop0::reserved, // 5
        device::cop0::tlbwr,    // 6
        device::cop0::reserved, // 7
        device::cop0::tlbp,     // 8
        device::cop0::reserved, // 9
        device::cop0::reserved, // 10
        device::cop0::reserved, // 11
        device::cop0::reserved, // 12
        device::cop0::reserved, // 13
        device::cop0::reserved, // 14
        device::cop0::reserved, // 15
        device::cop0::reserved, // 16
        device::cop0::reserved, // 17
        device::cop0::reserved, // 18
        device::cop0::reserved, // 19
        device::cop0::reserved, // 20
        device::cop0::reserved, // 21
        device::cop0::reserved, // 22
        device::cop0::reserved, // 23
        device::cop0::eret,     // 24
        device::cop0::reserved, // 25
        device::cop0::reserved, // 26
        device::cop0::reserved, // 27
        device::cop0::reserved, // 28
        device::cop0::reserved, // 29
        device::cop0::reserved, // 30
        device::cop0::reserved, // 31
    ];
}

pub fn init(device: &mut device::Device) {
    device.cpu.cop0.reg_write_masks = [
        COP0_INDEX_REG_MASK,
        0, // Random, read only
        COP0_ENTRYLO_REG_MASK,
        COP0_ENTRYLO_REG_MASK,
        COP0_CONTEXT_REG_MASK,
        COP0_PAGEMASK_REG_MASK,
        COP0_WIRED_REG_MASK,
        0,               // 7
        0,               // BadVAddr, read only
        u32::MAX as u64, // count
        COP0_ENTRYHI_REG_MASK,
        u32::MAX as u64, // compare
        COP0_STATUS_REG_MASK,
        COP0_CAUSE_REG_MASK,
        u64::MAX, // EPC
        0,        // previd, read only
        COP0_CONFIG_REG_MASK,
        COP0_LLADDR_REG_MASK,
        COP0_WATCHLO_REG_MASK,
        COP0_WATCHHI_REG_MASK,
        COP0_XCONTEXT_REG_MASK,
        0, // 21
        0, // 22
        0, // 23
        0, // 24
        0, // 25
        COP0_PARITYERR_REG_MASK,
        0, // cache error
        COP0_TAGLO_REG_MASK,
        0,        // taghi
        u64::MAX, // ErrorPC
        0,        // 31
    ];

    map_instructions(device);

    // taken from VR4300 manual
    device.cpu.cop0.regs[COP0_RANDOM_REG as usize] = 0b00000000000000000000000000011111;
    device.cpu.cop0.regs[COP0_CONFIG_REG as usize] = 0b01110000000001101110010001100000;
    device.cpu.cop0.regs[COP0_STATUS_REG as usize] = 0b00000000010000000000000000000100;
    device.cpu.cop0.regs[COP0_PREVID_REG as usize] = 0b00000000000000000000101100100010;
    device.cpu.cop0.regs[COP0_EPC_REG as usize] = 0b11111111111111111111111111111111;
    device.cpu.cop0.regs[COP0_ERROREPC_REG as usize] = 0b11111111111111111111111111111111;
    device.cpu.cop0.regs[COP0_BADVADDR_REG as usize] = 0xFFFFFFFF;
    device.cpu.cop0.regs[COP0_CONTEXT_REG as usize] = 0x7FFFF0;

    device::events::create_event(device, device::events::EVENT_TYPE_COMPARE, u32::MAX as u64)
}
