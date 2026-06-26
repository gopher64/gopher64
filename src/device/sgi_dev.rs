/// SGI N64 Development Board — N64-side driver
///
/// Compiled only when the `ultra64` feature is enabled.
/// Models the three N64-side control registers the development board
/// exposes in the PI cartridge domain upper range:
///
///   0x18000000  W   GIO_INT   — N64 writes 6-bit value → signals Indy host
///   0x18000400  W   GIO_SYNC  — polling register, no interrupt raised
///   0x18000800  R   CART_INT_CLR — ACK the host→N64 cart interrupt (clears CAUSE.IP4)
///
/// The 16 MB RAMROM SRAM appears as normal cartridge ROM at 0x10000000 and is
/// handled entirely by the existing cart::rom paths — this module does not
/// touch that range.
///
/// On startup gopher64 waits (with a timeout) for IRIS to create the shm object
/// "iris_n64_bridge".  Once it appears and the magic word is valid, we open the
/// shm with shared_memory::ShmemConf::open(), attach to the two raw_sync Events
/// embedded at the start, and point `device.cart.rom` at the 16 MB RAMROM region
/// inside the mapping so every existing cart::rom read path transparently reads
/// from shm without further modification.

use crate::device;
use crate::device::memory::AccessSize;
use raw_sync::events::{Event, EventImpl, EventInit, EventState};
use raw_sync::Timeout;
use shared_memory::ShmemConf;
use std::fs::File;
use std::sync::{Arc, Mutex, OnceLock};
use std::sync::atomic::{AtomicBool, Ordering};

// All ultra64 log output (debug + trace) goes here, never to stderr.
pub static ULTRA_LOG: OnceLock<Mutex<File>> = OnceLock::new();

// Set by the listener thread on RESET_DEASSERT; survives Device reconstruction
// so the next sgi_dev::init knows it can skip the RESET_DEASSERT wait.
static HARD_RESET_PENDING: AtomicBool = AtomicBool::new(false);

fn open_ultra_log() {
    let f = File::create("/tmp/ultra64.log").expect("ultra64: cannot open /tmp/ultra64.log");
    let _ = ULTRA_LOG.set(Mutex::new(f));
}

macro_rules! ulog {
    ($($arg:tt)*) => {
        if let Some(m) = $crate::device::sgi_dev::ULTRA_LOG.get() {
            if let Ok(mut f) = m.lock() {
                let _ = std::io::Write::write_fmt(&mut *f, format_args!("{}\n", format_args!($($arg)*)));
            }
        }
    };
}
pub(crate) use ulog;

/// Wraps a `Box<dyn EventImpl>` and asserts Send + Sync.
/// SAFETY: raw_sync Events use POSIX pthread primitives (or Windows Event objects),
/// both of which are safe to use from any thread. Internal synchronization is
/// provided by the mutex embedded in the shm region.
struct SendEvent(Box<dyn EventImpl>);
unsafe impl Send for SendEvent {}
unsafe impl Sync for SendEvent {}
impl SendEvent {
    fn wait(&self, t: Timeout) -> Result<(), Box<dyn std::error::Error>> { self.0.wait(t) }
    fn set(&self, s: EventState) -> Result<(), Box<dyn std::error::Error>> { self.0.set(s) }
}

// --------------------------------------------------------------------------
// Shared-memory layout — single source of truth in ultra_proto (symlinked)
// --------------------------------------------------------------------------

use super::ultra_proto::{h2n, n2h, ShmHeader,
    SHM_MAGIC, SHM_HEADER_OFFSET, SHM_RAMROM_OFFSET};

pub const SHM_OS_ID:    &str  = "iris_n64_bridge";
pub const RAMROM_SIZE:  usize = super::ultra_proto::RAMROM_TOTAL;

/// Byte offset of the first Event (h2n) inside the shm region.
pub const EVT_H2N_OFFSET: usize = 0;

// --------------------------------------------------------------------------
// SgiDev state (attached to Device)
// --------------------------------------------------------------------------

pub struct SgiDev {
    // shared_memory::Shmem owns the mapping for its lifetime.
    shmem:            Option<shared_memory::Shmem>,
    // Raw pointers into shmem; valid as long as shmem is Some(…).
    hdr_ptr:          *mut ShmHeader,
    ramrom_ptr:       *mut u8,
    // Events in Arc so they can be cloned into background threads.
    evt_h2n:          Option<Arc<SendEvent>>, // Indy sets  → N64 wakes
    evt_n2h:          Option<Arc<SendEvent>>, // N64 sets   → Indy wakes
    pub cart_int_pending: bool,
    // Written by h2n listener thread via raw pointer (same pattern as cause_ptr).
    // Safe: h2n thread is the sole writer; CPU thread is sole reader.
    pub rdb_h2n:      u32,  // last Indy→N64 RDB packet
    pub rdb_n2h:      u32,  // last N64→Indy RDB packet
    pub rdb_n2h_ack:  u32,  // running ACK counter from Indy
    // Set by the listener thread; consumed by the CPU thread each iteration.
    pub nmi_pending:        Arc<AtomicBool>,
    pub hard_reset_pending: Arc<AtomicBool>,
    // Shutdown signal for the h2n listener thread; set before dropping shmem.
    listener_stop:    Arc<AtomicBool>,
    listener_thread:  Option<std::thread::JoinHandle<()>>,
}

// SAFETY: ShmHeader accesses are single-threaded on the N64 CPU path;
// the background thread only writes cause_ptr which the N64 CPU reads
// atomically at the top of each step — same pattern as gopher64's MI.
unsafe impl Send for SgiDev {}
unsafe impl Sync for SgiDev {}

impl Default for SgiDev {
    fn default() -> Self {
        SgiDev {
            shmem:              None,
            hdr_ptr:            std::ptr::null_mut(),
            ramrom_ptr:         std::ptr::null_mut(),
            evt_h2n:            None,
            evt_n2h:            None,
            cart_int_pending:   false,
            rdb_h2n:            0,
            rdb_n2h:            0,
            rdb_n2h_ack:        0,
            nmi_pending:        Arc::new(AtomicBool::new(false)),
            hard_reset_pending: Arc::new(AtomicBool::new(false)),
            listener_stop:      Arc::new(AtomicBool::new(false)),
            listener_thread:    None,
        }
    }
}

impl SgiDev {
    pub fn hdr_mut(&self) -> &mut ShmHeader { unsafe { &mut *self.hdr_ptr } }
}

// --------------------------------------------------------------------------
// Startup
// --------------------------------------------------------------------------

/// Wait up to `timeout_secs` seconds for IRIS to create the shm object,
/// then open it, attach Events, and swap cart.rom to the RAMROM region.
/// Panics on timeout (we cannot run without the shared region).
pub fn init(device: &mut device::Device, timeout_secs: u64) {
    open_ultra_log();

    let deadline = std::time::Instant::now()
        + std::time::Duration::from_secs(timeout_secs);

    ulog!("[ultra64] waiting for IRIS shm \"{}\"...", SHM_OS_ID);

    // ---- wait for IRIS to create and initialise the shm ----
    let shmem = loop {
        match ShmemConf::new().os_id(SHM_OS_ID).open() {
            Ok(shm) => {
                let base = shm.as_ptr();
                let hdr = unsafe {
                    &*(base.add(SHM_HEADER_OFFSET) as *const ShmHeader)
                };
                if hdr.magic == SHM_MAGIC {
                    ulog!("[ultra64] shm found, magic={:#010x} version={}", hdr.magic, hdr.version);
                    break shm;
                }
                ulog!("[ultra64] shm exists but magic={:#010x} (not ready yet)", hdr.magic);
            }
            Err(e) => ulog!("[ultra64] shm not yet available: {}", e),
        }
        if std::time::Instant::now() >= deadline {
            panic!(
                "ultra64: timed out waiting for IRIS to create \"{SHM_OS_ID}\" \
                 (is iris running with [ultra64] enabled = true?)"
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    };

    let base = shmem.as_ptr();

    // ---- attach to the two Events embedded in the shm ----
    ulog!("[ultra64] attaching to h2n event at offset {:#x}", EVT_H2N_OFFSET);
    let (evt_h2n_box, h2n_used) = unsafe {
        Event::from_existing(base.add(EVT_H2N_OFFSET))
            .expect("ultra64: failed to attach to h2n event")
    };
    let evt_n2h_offset = EVT_H2N_OFFSET + h2n_used;
    ulog!("[ultra64] attaching to n2h event at offset {:#x}", evt_n2h_offset);
    let (evt_n2h_box, _) = unsafe {
        Event::from_existing(base.add(evt_n2h_offset))
            .expect("ultra64: failed to attach to n2h event")
    };
    ulog!("[ultra64] events attached OK");

    let evt_h2n = Arc::new(SendEvent(evt_h2n_box));
    let evt_n2h = Arc::new(SendEvent(evt_n2h_box));

    let hdr_ptr    = unsafe { base.add(SHM_HEADER_OFFSET) as *mut ShmHeader };
    let ramrom_ptr = unsafe { base.add(SHM_RAMROM_OFFSET) };

    ulog!("[ultra64] initial header: magic={:#010x} version={}", unsafe { (*hdr_ptr).magic }, unsafe { (*hdr_ptr).version });

    device.sgi_dev.shmem      = Some(shmem);
    device.sgi_dev.hdr_ptr    = hdr_ptr;
    device.sgi_dev.ramrom_ptr = ramrom_ptr;
    device.sgi_dev.evt_h2n    = Some(evt_h2n);
    device.sgi_dev.evt_n2h    = Some(evt_n2h);

    // ---- point cart.rom at the shm RAMROM region ----
    let old = std::mem::replace(&mut device.cart.rom, Vec::new());
    drop(old);
    let shm_vec = unsafe {
        Vec::from_raw_parts(ramrom_ptr, RAMROM_SIZE, RAMROM_SIZE)
    };
    device.cart.rom = shm_vec;
    ulog!("[ultra64] cart.rom -> shm RAMROM ({} MB)", RAMROM_SIZE >> 20);

    // ---- wait for IRIS to release reset before starting the CPU ----
    // On a hard reset restart the static HARD_RESET_PENDING is already set
    // (the previous listener thread set it on RESET_DEASSERT before exiting).
    // Clear it and skip the wait — gload already deasserted reset for this run.
    // On first boot, block until gload pushes RESET_DEASSERT into the ring.
    if HARD_RESET_PENDING.swap(false, Ordering::AcqRel) {
        ulog!("[ultra64] hard reset restart — skipping RESET_DEASSERT wait");
    } else {
        ulog!("[ultra64] waiting for RESET_DEASSERT...");
        let evt = device.sgi_dev.evt_h2n.as_ref().unwrap();
        loop {
            let hdr = unsafe { &mut *hdr_ptr };
            let mut got_deassert = false;
            while let Some((kind, _)) = hdr.h2n_ring.pop() {
                if kind == h2n::RESET_DEASSERT { got_deassert = true; }
            }
            if got_deassert { break; }
            let _ = evt.wait(Timeout::Val(std::time::Duration::from_millis(50)));
        }
        ulog!("[ultra64] RESET_DEASSERT received, starting CPU");
    }

    // Reset stop flag in case we're restarting after a hard reset.
    device.sgi_dev.listener_stop.store(false, Ordering::Release);

    // ---- spawn host→N64 interrupt listener ----
    let handle = spawn_cart_int_listener(device);
    device.sgi_dev.listener_thread = Some(handle);
    ulog!("[ultra64] h2n listener thread spawned, ready");
}

/// Background thread: blocks on h2n event, drains the h2n ring, dispatches messages.
/// Ring-based delivery guarantees no events are lost even if IRIS posts faster than
/// the thread can re-enter wait (the old edge-detection approach would drop those).
fn spawn_cart_int_listener(device: &mut device::Device) -> std::thread::JoinHandle<()> {
    let evt_h2n            = Arc::clone(device.sgi_dev.evt_h2n.as_ref().unwrap());
    let hdr_ptr            = device.sgi_dev.hdr_ptr as usize;
    let cause_ptr          = &raw mut device.cpu.cop0.regs[device::cop0::COP0_CAUSE_REG] as usize;
    let rdb_h2n_ptr        = &raw mut device.sgi_dev.rdb_h2n as usize;
    let rdb_n2h_ack_ptr    = &raw mut device.sgi_dev.rdb_n2h_ack as usize;
    let nmi_pending        = Arc::clone(&device.sgi_dev.nmi_pending);
    let hard_reset_pending = Arc::clone(&device.sgi_dev.hard_reset_pending);
    let stop               = Arc::clone(&device.sgi_dev.listener_stop);

    std::thread::Builder::new()
        .name("ultra64-h2n".into())
        .spawn(move || {
            let _ = evt_h2n.set(EventState::Clear);
            ulog!("[ultra64/h2n] listener started (ring-based)");
            loop {
                if evt_h2n.wait(Timeout::Infinite).is_err() {
                    ulog!("[ultra64/h2n] wait error, exiting");
                    break;
                }
                if stop.load(Ordering::Acquire) {
                    ulog!("[ultra64/h2n] stop flag set, exiting");
                    break;
                }
                // Drain every message queued since the last wake.
                // A single h2n event post is just a wakeup hint; the ring
                // holds the authoritative record of what happened.
                let hdr = unsafe { &mut *(hdr_ptr as *mut ShmHeader) };
                while let Some((kind, payload)) = hdr.h2n_ring.pop() {
                    match kind {
                        h2n::RESET_ASSERT => {
                            ulog!("[ultra64/h2n] RESET_ASSERT");
                            // Nothing to do — we just wait for DEASSERT.
                        }
                        h2n::RESET_DEASSERT => {
                            ulog!("[ultra64/h2n] RESET_DEASSERT → hard_reset_pending, exiting");
                            HARD_RESET_PENDING.store(true, Ordering::Release);
                            hard_reset_pending.store(true, Ordering::Release);
                            return;
                        }
                        h2n::NMI_ASSERT => {
                            ulog!("[ultra64/h2n] NMI_ASSERT → nmi_pending");
                            nmi_pending.store(true, Ordering::Release);
                        }
                        h2n::CART_INT => {
                            ulog!("[ultra64/h2n] CART_INT payload={:#x} → CAUSE.IP4", payload);
                            unsafe { *(cause_ptr as *mut u64) |= 1 << 12; }
                        }
                        h2n::RDB_WRITE => {
                            ulog!("[ultra64/h2n] RDB_WRITE {:#010x} → CAUSE bit 14 (IP7)", payload);
                            unsafe { *(rdb_h2n_ptr as *mut u32) = payload; }
                            unsafe { *(cause_ptr as *mut u64) |= 1 << 14; }
                        }
                        h2n::RDB_ACK => {
                            ulog!("[ultra64/h2n] RDB_ACK {} → CAUSE bit 13 (IP6)", payload);
                            unsafe { *(rdb_n2h_ack_ptr as *mut u32) = payload; }
                            unsafe { *(cause_ptr as *mut u64) |= 1 << 13; }
                        }
                        _ => {
                            ulog!("[ultra64/h2n] unknown msg kind={} payload={}", kind, payload);
                        }
                    }
                }
            }
        })
        .expect("ultra64: failed to spawn h2n listener thread")
}

// --------------------------------------------------------------------------
// N64-side control register handlers (memory_map_read/write at 0x18000000)
// --------------------------------------------------------------------------

pub fn read_mem_fast(
    _device: &device::Device,
    _address: u64,
    _access_size: AccessSize,
) -> u32 {
    0
}

pub fn read_mem(
    device: &mut device::Device,
    address: u64,
    _access_size: AccessSize,
) -> u32 {
    let offset = (address as u32) & 0xFFF;
    let val = match offset {
        // 0x18000800: CART_INT_CLR — ACK host→N64 interrupt
        0x800 => {
            device.sgi_dev.cart_int_pending = false;
            device.cpu.cop0.regs[device::cop0::COP0_CAUSE_REG] &= !(1u64 << 12);
            0
        }
        _ => 0,
    };
    ulog!("[ultra64] ctrl read  addr={:#010x} offset={:#05x} -> {:#010x}", address, offset, val);
    val
}

pub fn write_mem(
    device: &mut device::Device,
    address: u64,
    value: u32,
    _mask: u32,
) {
    let offset = (address as u32) & 0xFFF;
    // log every write so we can see what libultra/RDB actually accesses
    ulog!("[ultra64] ctrl write addr={:#010x} offset={:#05x} val={:#010x}", address, offset, value);
    match offset {
        // 0x18000000: GIO_INT — signal host with 6-bit payload, wake Indy
        // 0x000 => { ... } // handled below with logging
        // 0x18000400: GIO_SYNC — polling only
        // 0x400 => { ... } // handled below with logging
        0x000 => {
            let payload = value & 0x3F;
            device.sgi_dev.hdr_mut().n2h_ring.push(n2h::GIO_INT, payload);
            if let Some(evt) = &device.sgi_dev.evt_n2h {
                let _ = evt.set(EventState::Signaled);
            }
        }
        0x400 => {
            let v = value & 0x3F;
            device.sgi_dev.hdr_mut().n2h_ring.push(n2h::GIO_SYNC, v);
        }
        _ => {} // unknown — logged above
    }
}

// --------------------------------------------------------------------------
// RDB register handlers (physical 0xC0000000)
// --------------------------------------------------------------------------

pub fn read_mem_rdb(
    device: &mut device::Device,
    address: u64,
    _access_size: AccessSize,
) -> u32 {
    // Physical 0x80000000: osMapTLBRdb maps virtual 0xC0000000 here (uncached).
    let offset = address & 0xF;
    let val = match offset as u32 {
        0x0 => {
            // N64 read Indy's RDB packet — notify Indy so it can send the next one,
            // and clear the RDB_WRITE interrupt bit so the ISR doesn't re-fire.
            let v = device.sgi_dev.rdb_h2n;
            device.cpu.cop0.regs[device::cop0::COP0_CAUSE_REG] &= !device::cop0::COP0_CAUSE_IP6;
            device.sgi_dev.hdr_mut().n2h_ring.push(n2h::RDB_READ, 0);
            if let Some(evt) = &device.sgi_dev.evt_n2h {
                let _ = evt.set(EventState::Signaled);
            }
            v
        }
        _ => 0,
    };
    ulog!("[ultra64] RDB read  phys={:#010x} offset={:#x} -> {:#010x}", address, offset, val);
    val
}

pub fn write_mem_rdb(
    device: &mut device::Device,
    address: u64,
    value: u32,
    _mask: u32,
) {
    // Physical 0x80000000: osMapTLBRdb maps virtual 0xC0000000 here (uncached).
    let offset = address & 0xF;
    ulog!("[ultra64] RDB write phys={:#010x} offset={:#x} val={:#010x}", address, offset, value);
    match offset as u32 {
        0x0 => {
            // N64 writes RDB packet to Indy — push onto n2h ring, wake Indy.
            device.sgi_dev.rdb_n2h = value;
            device.sgi_dev.hdr_mut().n2h_ring.push(n2h::RDB_WRITE, value);
            if let Some(evt) = &device.sgi_dev.evt_n2h {
                let _ = evt.set(EventState::Signaled);
            }
        }
        0x8 => {
            device.cpu.cop0.regs[device::cop0::COP0_CAUSE_REG] &= !device::cop0::COP0_CAUSE_IP6;
        }
        0xC => {
            device.cpu.cop0.regs[device::cop0::COP0_CAUSE_REG] &= !device::cop0::COP0_CAUSE_RDB_READ_ACK;
        }
        _ => {}
    }
}

// --------------------------------------------------------------------------
// PI DMA handlers for the 0x18000000 control register range
// (actual RAMROM DMA goes through 0x10000000 via cart::rom::dma_write)
// --------------------------------------------------------------------------

pub fn dma_read(
    _device: &mut device::Device,
    _cart_addr: u32,
    _dram_addr: u32,
    length: u32,
) -> u64 {
    length as u64 * 6 // nominal cycle cost, control regs don't support DMA
}

pub fn dma_write(
    _device: &mut device::Device,
    _cart_addr: u32,
    _dram_addr: u32,
    length: u32,
) -> u64 {
    length as u64 * 6
}

// --------------------------------------------------------------------------
// Cleanup
// --------------------------------------------------------------------------

pub fn close(device: &mut device::Device) {
    if device.sgi_dev.shmem.is_none() {
        return;
    }

    // Wait for the listener thread to exit before unmapping the shm.
    // The thread self-exits on RESET_DEASSERT (hard reset path) or on wait
    // error; we also set the stop flag and signal in case it's still running
    // (e.g. clean shutdown without a hard reset).
    device.sgi_dev.listener_stop.store(true, Ordering::Release);
    if let Some(evt) = &device.sgi_dev.evt_h2n {
        let _ = evt.set(EventState::Signaled);
    }
    if let Some(handle) = device.sgi_dev.listener_thread.take() {
        let _ = handle.join();
    }
    // The events point into shm — drop our Arcs after the thread exits.
    device.sgi_dev.evt_h2n = None;
    device.sgi_dev.evt_n2h = None;

    // Prevent the shm-backed Vec from being freed through the allocator.
    let v = std::mem::replace(&mut device.cart.rom, Vec::new());
    std::mem::forget(v);

    // Drop shmem last so the mapping stays valid until after the Vec is forgotten.
    device.sgi_dev.shmem.take();
    device.sgi_dev.hdr_ptr    = std::ptr::null_mut();
    device.sgi_dev.ramrom_ptr = std::ptr::null_mut();
}
