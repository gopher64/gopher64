use crate::ui;

pub mod ai;
pub mod cache;
pub mod cart;
pub mod cart_rom;
pub mod controller;
pub mod cop0;
pub mod cop1;
pub mod cop2;
pub mod cpu;
pub mod cpu_instructions;
pub mod events;
pub mod exceptions;
pub mod fpu_instructions;
pub mod is_viewer;
pub mod memory;
pub mod mempak;
pub mod mi;
pub mod pi;
pub mod pif;
pub mod rdp;
pub mod rdram;
pub mod ri;
pub mod rsp_cpu;
pub mod rsp_interface;
pub mod rsp_su_instructions;
pub mod rsp_vu_instructions;
pub mod si;
pub mod sram;
pub mod tlb;
pub mod unmapped;
pub mod vi;

pub struct Device {
    pub ui: ui::Ui,
    byte_swap: usize,
    cpu: cpu::Cpu,
    pif: pif::Pif,
    cart: cart_rom::Cart,
    memory: memory::Memory,
    rsp: rsp_interface::Rsp,
    rdp: rdp::Rdp,
    pub rdram: rdram::Rdram,
    mi: mi::Mi,
    pi: pi::Pi,
    vi: vi::Vi,
    ai: ai::Ai,
    si: si::Si,
    ri: ri::Ri,
    flashram: sram::Flashram,
}

impl Device {
    pub fn new() -> Device {
        let mut byte_swap: usize = 0;
        let test: [u8; 4] = [1, 2, 3, 4];
        // if the host computer is little endian, that means the RDRAM will be stored as little endian
        // when accessing bytes in RDRAM, we need to swap them by XORing with the byte_swap value (3)
        if u32::from_le_bytes(test) == u32::from_ne_bytes(test) {
            byte_swap = 3;
        }
        Device {
            ui: ui::Ui::new(),
            byte_swap: byte_swap,
            cpu: cpu::Cpu {
                cop0: cop0::Cop0 {
                    regs: [0; cop0::COP0_REGS_COUNT as usize],
                    reg_write_masks: [0; cop0::COP0_REGS_COUNT as usize],
                    reg_latch: 0,
                    instrs: [cop0::reserved; 32],
                    instrs2: [cop0::reserved; 32],
                    tlb_lut_w: vec![
                        tlb::TlbLut {
                            address: 0,
                            cached: false,
                        };
                        0x100000
                    ],
                    tlb_lut_r: vec![
                        tlb::TlbLut {
                            address: 0,
                            cached: false,
                        };
                        0x100000
                    ],
                    tlb_entries: [tlb::TlbEntry {
                        mask: 0,
                        vpn2: 0,
                        region: 0,
                        g: 0,
                        asid: 0,
                        pfn_even: 0,
                        c_even: 0,
                        d_even: 0,
                        v_even: 0,
                        pfn_odd: 0,
                        c_odd: 0,
                        d_odd: 0,
                        v_odd: 0,
                        start_even: 0,
                        end_even: 0,
                        phys_even: 0,
                        start_odd: 0,
                        end_odd: 0,
                        phys_odd: 0,
                    }; 32],
                },
                cop1: cop1::Cop1 {
                    fcr0: 0,
                    fcr31: 0,
                    flush_mode: 0,
                    fgr32: [[0; 4]; 32],
                    fgr64: [[0; 8]; 32],
                    instrs: [cop1::reserved; 32],
                    b_instrs: [cop1::reserved; 4],
                    s_instrs: [cop1::reserved; 64],
                    d_instrs: [cop1::reserved; 64],
                    w_instrs: [cop1::reserved; 64],
                    l_instrs: [cop1::reserved; 64],
                },
                cop2: cop2::Cop2 {
                    instrs: [cop2::reserved; 32],
                    reg_latch: 0,
                },
                branch_state: cpu::BranchState {
                    state: cpu::State::Step,
                    pc: 0,
                },
                gpr: [0; 32],
                clock_rate: 0,
                pc: 0xBFC00000,
                pc_phys: 0,
                llbit: false,
                lo: 0,
                hi: 0,
                running: 0,
                instrs: [cop0::reserved; 64],
                special_instrs: [cop0::reserved; 64],
                regimm_instrs: [cop0::reserved; 32],
                events: [events::Event {
                    enabled: false,
                    count: u64::MAX,
                    handler: events::dummy_event,
                }; events::EventType::EventTypeCount as usize],
                next_event_count: u64::MAX,
                next_event: 0,
            },
            pif: pif::Pif {
                rom: [0; 1984],
                ram: [0; 64],
                channels: [pif::PifChannel {
                    tx: None,
                    tx_buf: None,
                    rx: None,
                    rx_buf: None,
                    process: None,
                    pak_handler: None,
                }; 5],
            },
            cart: cart_rom::Cart {
                rom: Vec::new(),
                is_viewer_buffer: [0; 0xFFFF],
                pal: false,
                latch: 0,
                cic_seed: 0,
                cic_type: cart_rom::CicType::CicNus6102,
                rdram_size_offset: 0,
                rtc: cart::AfRtc { control: 0x0200 },
            },
            memory: memory::Memory {
                fast_read: [unmapped::read_mem; 0x2000],
                memory_map_read: [unmapped::read_mem; 0x2000],
                memory_map_write: [unmapped::write_mem; 0x2000],
                icache: [cache::ICache {
                    valid: false,
                    index: 0,
                    tag: 0,
                    words: [0; 8],
                    instruction: [cop0::reserved; 8],
                }; 512],
                dcache: [cache::DCache {
                    valid: false,
                    dirty: false,
                    tag: 0,
                    index: 0,
                    words: [0; 4],
                }; 512],
            },
            rdram: rdram::Rdram { mem: vec![] },
            rsp: rsp_interface::Rsp {
                cpu: rsp_cpu::Cpu {
                    instructions: [rsp_cpu::Instructions {
                        func: rsp_su_instructions::reserved,
                        opcode: 0,
                    }; 0x1000 / 4],
                    last_instruction_type: rsp_cpu::InstructionType::Su,
                    instruction_type: rsp_cpu::InstructionType::Su,
                    pipeline_full: false,
                    branch_state: rsp_cpu::BranchState {
                        state: cpu::State::Step,
                        pc: 0,
                    },
                    broken: false,
                    halted: false,
                    sync_point: false,
                    cycle_counter: 0,
                    gpr: [0; 32],
                    vpr: [0; 32],
                    reciprocals: [0; 512],
                    inverse_square_roots: [0; 512],
                    divdp: false,
                    divin: 0,
                    divout: 0,
                    shuffle: unsafe { [std::arch::x86_64::_mm_setzero_si128(); 16] },
                    vcol: unsafe { std::arch::x86_64::_mm_setzero_si128() },
                    vcoh: unsafe { std::arch::x86_64::_mm_setzero_si128() },
                    vccl: unsafe { std::arch::x86_64::_mm_setzero_si128() },
                    vcch: unsafe { std::arch::x86_64::_mm_setzero_si128() },
                    vce: unsafe { std::arch::x86_64::_mm_setzero_si128() },
                    accl: unsafe { std::arch::x86_64::_mm_setzero_si128() },
                    accm: unsafe { std::arch::x86_64::_mm_setzero_si128() },
                    acch: unsafe { std::arch::x86_64::_mm_setzero_si128() },
                    special_instrs: [rsp_su_instructions::reserved; 64],
                    regimm_instrs: [rsp_su_instructions::reserved; 32],
                    cop0_instrs: [rsp_su_instructions::reserved; 32],
                    cop2_instrs: [rsp_su_instructions::reserved; 32],
                    lwc2_instrs: [rsp_su_instructions::reserved; 32],
                    swc2_instrs: [rsp_su_instructions::reserved; 32],
                    instrs: [rsp_su_instructions::reserved; 64],
                    vec_instrs: [rsp_vu_instructions::reserved; 64],
                },
                regs: [0; rsp_interface::SP_REGS_COUNT as usize],
                regs2: [0; rsp_interface::SP_REGS2_COUNT as usize],
                mem: [0; 0x2000],
                fifo: [rsp_interface::RspDma {
                    dir: rsp_interface::DmaDir::None,
                    length: 0,
                    memaddr: 0,
                    dramaddr: 0,
                }; 2],
            },
            rdp: rdp::Rdp {
                regs_dpc: [0; rdp::DPC_REGS_COUNT as usize],
                regs_dps: [0; rdp::DPS_REGS_COUNT as usize],
                wait_frozen: false,
            },
            mi: mi::Mi {
                regs: [0; mi::MI_REGS_COUNT as usize],
            },
            pi: pi::Pi {
                regs: [0; pi::PI_REGS_COUNT as usize],
            },
            ai: ai::Ai {
                regs: [0; ai::AI_REGS_COUNT as usize],
                last_read: 0,
                delayed_carry: false,
                fifo: [ai::AiDma {
                    address: 0,
                    length: 0,
                    duration: 0,
                }; 2],
            },
            si: si::Si {
                regs: [0; si::SI_REGS_COUNT as usize],
                dma_dir: si::DmaDir::None,
            },
            ri: ri::Ri {
                regs: [0; ri::RI_REGS_COUNT as usize],
            },
            vi: vi::Vi {
                regs: [0; vi::VI_REGS_COUNT as usize],
                clock: 0,
                field: 0,
                delay: 0,
                count_per_scanline: 0,
                limiter: None,
            },
            flashram: sram::Flashram {
                status: 0,
                erase_page: 0,
                page_buf: [0xff; 128],
                silicon_id: [sram::FLASHRAM_TYPE_ID, sram::MX29L1100_ID],
                mode: sram::FlashramMode::ReadArray,
            },
        }
    }
}
