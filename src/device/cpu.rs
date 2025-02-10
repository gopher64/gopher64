use crate::device;
use crate::savestates;

#[derive(PartialEq, serde::Serialize, serde::Deserialize)]
pub enum State {
    Step,
    Take,
    NotTaken,
    DelaySlotTaken,
    DelaySlotNotTaken,
    Discard,
    Exception,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct BranchState {
    pub state: State,
    pub pc: u64,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Cpu {
    pub cop0: device::cop0::Cop0,
    pub cop1: device::cop1::Cop1,
    pub cop2: device::cop2::Cop2,
    pub branch_state: BranchState,
    pub gpr: [u64; 32],
    pub pc: u64,
    pub pc_phys: u64,
    pub lo: u64,
    pub hi: u64,
    pub running: bool,
    pub llbit: bool,
    pub clock_rate: u64,
    #[serde(skip)]
    #[serde(default = "savestates::default_instruction_64")]
    pub instrs: [fn(&mut device::Device, u32); 64],
    #[serde(skip)]
    #[serde(default = "savestates::default_instruction_64")]
    pub special_instrs: [fn(&mut device::Device, u32); 64],
    #[serde(skip)]
    #[serde(default = "savestates::default_instruction_32")]
    pub regimm_instrs: [fn(&mut device::Device, u32); 32],
    pub events: [device::events::Event; device::events::EventType::Count as usize],
    pub next_event_count: u64,
    pub next_event: usize,
}

pub fn decode_opcode(device: &device::Device, opcode: u32) -> fn(&mut device::Device, u32) {
    match opcode >> 26 {
        0 => device.cpu.special_instrs[(opcode & 0x3F) as usize], // SPECIAL
        1 => device.cpu.regimm_instrs[((opcode >> 16) & 0x1F) as usize], // REGIMM
        16 => device.cpu.cop0.instrs[((opcode >> 21) & 0x1F) as usize], // COP0
        17 => device.cpu.cop1.instrs[((opcode >> 21) & 0x1F) as usize], // COP1
        18 => device.cpu.cop2.instrs[((opcode >> 21) & 0x1F) as usize], // COP2
        _ => device.cpu.instrs[(opcode >> 26) as usize],
    }
}

pub fn init(device: &mut device::Device) {
    device.cpu.clock_rate = 93750000;

    device.cpu.instrs = [
        device::cop0::reserved,           // SPECIAL
        device::cop0::reserved,           // REGIMM
        device::cpu_instructions::j,      // 2
        device::cpu_instructions::jal,    // 3
        device::cpu_instructions::beq,    // 4
        device::cpu_instructions::bne,    // 5
        device::cpu_instructions::blez,   // 6
        device::cpu_instructions::bgtz,   // 7
        device::cpu_instructions::addi,   // 8
        device::cpu_instructions::addiu,  // 9
        device::cpu_instructions::slti,   // 10
        device::cpu_instructions::sltiu,  // 11
        device::cpu_instructions::andi,   // 12
        device::cpu_instructions::ori,    // 13
        device::cpu_instructions::xori,   // 14
        device::cpu_instructions::lui,    // 15
        device::cop0::reserved,           // COP0
        device::cop0::reserved,           // COP1
        device::cop0::reserved,           // COP2
        device::cop0::reserved,           // 19
        device::cpu_instructions::beql,   // 20
        device::cpu_instructions::bnel,   // 21
        device::cpu_instructions::blezl,  // 22
        device::cpu_instructions::bgtzl,  // 23
        device::cpu_instructions::daddi,  // 24
        device::cpu_instructions::daddiu, // 25
        device::cpu_instructions::ldl,    // 26
        device::cpu_instructions::ldr,    // 27
        device::cop0::reserved,           // 28
        device::cop0::reserved,           // 29
        device::cop0::reserved,           // 30
        device::cop0::reserved,           // 31
        device::cpu_instructions::lb,     // 32
        device::cpu_instructions::lh,     // 33
        device::cpu_instructions::lwl,    // 34
        device::cpu_instructions::lw,     // 35
        device::cpu_instructions::lbu,    // 36
        device::cpu_instructions::lhu,    // 37
        device::cpu_instructions::lwr,    // 38
        device::cpu_instructions::lwu,    // 39
        device::cpu_instructions::sb,     // 40
        device::cpu_instructions::sh,     // 41
        device::cpu_instructions::swl,    // 42
        device::cpu_instructions::sw,     // 43
        device::cpu_instructions::sdl,    // 44
        device::cpu_instructions::sdr,    // 45
        device::cpu_instructions::swr,    // 46
        device::cpu_instructions::cache,  // 47
        device::cpu_instructions::ll,     // 48
        device::cop1::lwc1,               // 49
        device::cop0::reserved,           // 50
        device::cop0::reserved,           // 51
        device::cpu_instructions::lld,    // 52
        device::cop1::ldc1,               // 53
        device::cop0::reserved,           // 54
        device::cpu_instructions::ld,     // 55
        device::cpu_instructions::sc,     // 56
        device::cop1::swc1,               // 57
        device::cop0::reserved,           // 58
        device::cop0::reserved,           // 59
        device::cpu_instructions::scd,    // 60
        device::cop1::sdc1,               // 61
        device::cop0::reserved,           // 62
        device::cpu_instructions::sd,     // 63
    ];
    device.cpu.special_instrs = [
        device::cpu_instructions::sll,    // 0
        device::cop0::reserved,           // 1
        device::cpu_instructions::srl,    // 2
        device::cpu_instructions::sra,    // 3
        device::cpu_instructions::sllv,   // 4
        device::cop0::reserved,           // 5
        device::cpu_instructions::srlv,   // 6
        device::cpu_instructions::srav,   // 7
        device::cpu_instructions::jr,     // 8
        device::cpu_instructions::jalr,   // 9
        device::cop0::reserved,           // 10
        device::cop0::reserved,           // 11
        device::cop0::syscall,            // 12
        device::cop0::break_,             // 13
        device::cop0::reserved,           // 14
        device::cpu_instructions::sync,   // 15
        device::cpu_instructions::mfhi,   // 16
        device::cpu_instructions::mthi,   // 17
        device::cpu_instructions::mflo,   // 18
        device::cpu_instructions::mtlo,   // 19
        device::cpu_instructions::dsllv,  // 20
        device::cop0::reserved,           // 21
        device::cpu_instructions::dsrlv,  // 22
        device::cpu_instructions::dsrav,  // 23
        device::cpu_instructions::mult,   // 24
        device::cpu_instructions::multu,  // 25
        device::cpu_instructions::div,    // 26
        device::cpu_instructions::divu,   // 27
        device::cpu_instructions::dmult,  // 28
        device::cpu_instructions::dmultu, // 29
        device::cpu_instructions::ddiv,   // 30
        device::cpu_instructions::ddivu,  // 31
        device::cpu_instructions::add,    // 32
        device::cpu_instructions::addu,   // 33
        device::cpu_instructions::sub,    // 34
        device::cpu_instructions::subu,   // 35
        device::cpu_instructions::and,    // 36
        device::cpu_instructions::or,     // 37
        device::cpu_instructions::xor,    // 38
        device::cpu_instructions::nor,    // 39
        device::cop0::reserved,           // 40
        device::cop0::reserved,           // 41
        device::cpu_instructions::slt,    // 42
        device::cpu_instructions::sltu,   // 43
        device::cpu_instructions::dadd,   // 44
        device::cpu_instructions::daddu,  // 45
        device::cpu_instructions::dsub,   // 46
        device::cpu_instructions::dsubu,  // 47
        device::cpu_instructions::tge,    // 48
        device::cpu_instructions::tgeu,   // 49
        device::cpu_instructions::tlt,    // 50
        device::cpu_instructions::tltu,   // 51
        device::cpu_instructions::teq,    // 52
        device::cop0::reserved,           // 53
        device::cpu_instructions::tne,    // 54
        device::cop0::reserved,           // 55
        device::cpu_instructions::dsll,   // 56
        device::cop0::reserved,           // 57
        device::cpu_instructions::dsrl,   // 58
        device::cpu_instructions::dsra,   // 59
        device::cpu_instructions::dsll32, // 60
        device::cop0::reserved,           // 61
        device::cpu_instructions::dsrl32, // 62
        device::cpu_instructions::dsra32, // 63
    ];
    device.cpu.regimm_instrs = [
        device::cpu_instructions::bltz,    // 0
        device::cpu_instructions::bgez,    // 1
        device::cpu_instructions::bltzl,   // 2
        device::cpu_instructions::bgezl,   // 3
        device::cop0::reserved,            // 4
        device::cop0::reserved,            // 5
        device::cop0::reserved,            // 6
        device::cop0::reserved,            // 7
        device::cpu_instructions::tgei,    // 8
        device::cpu_instructions::tgeiu,   // 9
        device::cpu_instructions::tlti,    // 10
        device::cpu_instructions::tltiu,   // 11
        device::cpu_instructions::teqi,    // 12
        device::cop0::reserved,            // 13
        device::cpu_instructions::tnei,    // 14
        device::cop0::reserved,            // 15
        device::cpu_instructions::bltzal,  // 16
        device::cpu_instructions::bgezal,  // 17
        device::cpu_instructions::bltzall, // 18
        device::cpu_instructions::bgezall, // 19
        device::cop0::reserved,            // 20
        device::cop0::reserved,            // 21
        device::cop0::reserved,            // 22
        device::cop0::reserved,            // 23
        device::cop0::reserved,            // 24
        device::cop0::reserved,            // 25
        device::cop0::reserved,            // 26
        device::cop0::reserved,            // 27
        device::cop0::reserved,            // 28
        device::cop0::reserved,            // 29
        device::cop0::reserved,            // 30
        device::cop0::reserved,            // 31
    ];
    device::cop0::init(device);
    device::cop1::init(device);
    device::cop2::init(device);
}

pub fn in_delay_slot(device: &device::Device) -> bool {
    device.cpu.branch_state.state == State::DelaySlotTaken
        || device.cpu.branch_state.state == State::DelaySlotNotTaken
}

pub fn in_delay_slot_taken(device: &device::Device) -> bool {
    device.cpu.branch_state.state == State::DelaySlotTaken
}

pub fn run(device: &mut device::Device) {
    device.cpu.running = true;
    while device.cpu.running {
        device.cpu.gpr[0] = 0; // gpr 0 is read only
        let (cached, err);
        (device.cpu.pc_phys, cached, err) = device::memory::translate_address(
            device,
            device.cpu.pc,
            device::memory::AccessType::Read,
        );
        if err {
            continue; // TLB exception
        }

        if cached {
            device::cache::icache_fetch(device, device.cpu.pc_phys)
        } else {
            let opcode = device.memory.memory_map_read[(device.cpu.pc_phys >> 16) as usize](
                device,
                device.cpu.pc_phys,
                device::memory::AccessSize::Word,
            );
            device::cpu::decode_opcode(device, opcode)(device, opcode);
        }

        match device.cpu.branch_state.state {
            State::Step => device.cpu.pc += 4,
            State::Take => {
                device.cpu.pc += 4;
                device.cpu.branch_state.state = State::DelaySlotTaken
            }
            State::NotTaken => {
                device.cpu.pc += 4;
                device.cpu.branch_state.state = State::DelaySlotNotTaken
            }
            State::DelaySlotTaken => {
                device.cpu.pc = device.cpu.branch_state.pc;
                device.cpu.branch_state.state = State::Step
            }
            State::DelaySlotNotTaken => {
                device.cpu.pc += 4;
                device.cpu.branch_state.state = State::Step
            }
            State::Discard => {
                device.cpu.pc += 8;
                device.cpu.branch_state.state = State::Step
            }
            State::Exception => device.cpu.branch_state.state = State::Step,
        }
        device::cop0::add_cycles(device, 1);
        if device.cpu.cop0.regs[device::cop0::COP0_COUNT_REG as usize] > device.cpu.next_event_count
        {
            device::events::trigger_event(device)
        }
    }
}
