#![allow(dead_code)]
/// Shared IPC protocol between IRIS (Indy) and gopher64 (N64).
/// This file is symlinked into gopher64/src/device/ultra_proto.rs — keep it
/// dependency-free (no crate imports) so it compiles in both crates.

use std::sync::atomic::{AtomicU32, Ordering};

// ---------------------------------------------------------------------------
// Ring buffer
// ---------------------------------------------------------------------------

pub const H2N_RING_SIZE: usize = 64;

/// Message kinds: Indy → N64
pub mod h2n {
    pub const RESET_ASSERT:   u32 = 1;
    pub const RESET_DEASSERT: u32 = 2;
    pub const NMI_ASSERT:     u32 = 3;
    pub const CART_INT:       u32 = 4; // payload = 6-bit value
    pub const RDB_WRITE:      u32 = 5; // payload = rdb packet word
    pub const RDB_ACK:        u32 = 6; // payload = new ack counter value
}

/// Message kinds: N64 → Indy
pub mod n2h {
    pub const GIO_INT:   u32 = 1; // payload = 5-bit value
    pub const GIO_SYNC:  u32 = 2; // payload = 5-bit value
    pub const RDB_WRITE: u32 = 3; // payload = rdb packet word (N64→Indy)
    pub const RDB_READ:  u32 = 4; // N64 read Indy's packet; payload = unused
}

#[repr(C)]
pub struct IpcMsg {
    pub kind:    u32,
    pub payload: u32,
}

#[repr(C)]
pub struct IpcRing {
    pub head: u32,  // consumer reads and increments
    pub tail: u32,  // producer writes and increments
    pub msgs: [IpcMsg; H2N_RING_SIZE],
}

impl IpcRing {
    /// Push a message. Returns false if ring is full (dropped).
    pub fn push(&mut self, kind: u32, payload: u32) -> bool {
        let head = unsafe {
            (std::ptr::addr_of!(self.head) as *const AtomicU32)
                .as_ref().unwrap().load(Ordering::Acquire)
        };
        let tail = self.tail;
        if tail.wrapping_sub(head) >= H2N_RING_SIZE as u32 {
            return false;
        }
        let slot = (tail as usize) % H2N_RING_SIZE;
        self.msgs[slot].kind    = kind;
        self.msgs[slot].payload = payload;
        unsafe {
            (std::ptr::addr_of_mut!(self.tail) as *mut AtomicU32)
                .as_ref().unwrap().store(tail.wrapping_add(1), Ordering::Release)
        };
        true
    }

    /// Pop a message. Returns None if empty.
    pub fn pop(&mut self) -> Option<(u32, u32)> {
        let tail = unsafe {
            (std::ptr::addr_of!(self.tail) as *const AtomicU32)
                .as_ref().unwrap().load(Ordering::Acquire)
        };
        let head = self.head;
        if head == tail { return None; }
        let slot = (head as usize) % H2N_RING_SIZE;
        let kind    = self.msgs[slot].kind;
        let payload = self.msgs[slot].payload;
        unsafe {
            (std::ptr::addr_of_mut!(self.head) as *mut AtomicU32)
                .as_ref().unwrap().store(head.wrapping_add(1), Ordering::Release)
        };
        Some((kind, payload))
    }
}

// ---------------------------------------------------------------------------
// Shared memory header
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct ShmHeader {
    pub magic:    u32,     // 0x4E36344D ("N64M")
    pub version:  u32,     // 1
    pub h2n_ring: IpcRing, // Indy → N64
    pub n2h_ring: IpcRing, // N64 → Indy
}

pub const SHM_MAGIC:   u32   = 0x4E36_344D;
pub const SHM_VERSION: u32   = 1;

/// Fixed area at the start of shm reserved for the two raw_sync Events.
pub const EVENT_AREA_SIZE:   usize = 256;
pub const SHM_HEADER_OFFSET: usize = EVENT_AREA_SIZE;
pub const SHM_RAMROM_OFFSET: usize = SHM_HEADER_OFFSET + std::mem::size_of::<ShmHeader>();
pub const RAMROM_TOTAL:      usize = 0x100_0000; // 16 MB
pub const SHM_TOTAL_SIZE:    usize = SHM_RAMROM_OFFSET + RAMROM_TOTAL;

// ---------------------------------------------------------------------------
// RDB packet encoding (PR/rdb.h)
// Packet is one u32: bits[31:26]=type(6), bits[25:18]=length(8), bits[17:0]=data
// ---------------------------------------------------------------------------

pub mod rdb_type {
    pub const GTOH_PRINT:          u32 = 1;
    pub const GTOH_FAULT:          u32 = 2;
    pub const GTOH_LOG_CT:         u32 = 3;
    pub const GTOH_LOG:            u32 = 4;
    pub const GTOH_READY_FOR_DATA: u32 = 5;
    pub const GTOH_DATA_CT:        u32 = 6;
    pub const GTOH_DATA:           u32 = 7;
    pub const GTOH_DEBUG:          u32 = 8;
    pub const GTOH_RAMROM:         u32 = 9;
    pub const GTOH_DEBUG_DONE:     u32 = 10;
    pub const GTOH_DEBUG_READY:    u32 = 11;
    pub const GTOH_KDEBUG:         u32 = 12;
    pub const GTOH_PROF_DATA:      u32 = 22;

    pub const HTOG_LOG_DONE:       u32 = 13;
    pub const HTOG_DEBUG:          u32 = 14;
    pub const HTOG_DEBUG_CT:       u32 = 15;
    pub const HTOG_DATA:           u32 = 16;
    pub const HTOG_DATA_DONE:      u32 = 17;
    pub const HTOG_REQ_RAMROM:     u32 = 18;
    pub const HTOG_FREE_RAMROM:    u32 = 19;
    pub const HTOG_KDEBUG:         u32 = 20;
    pub const HTOG_PROF_SIGNAL:    u32 = 21;
}

#[inline] pub fn rdb_type(pkt: u32)   -> u32 { pkt >> 26 }
#[inline] pub fn rdb_length(pkt: u32) -> u32 { (pkt >> 18) & 0xFF }
#[inline] pub fn rdb_data(pkt: u32)   -> u32 { pkt & 0x3FFFF }

/// Extract up to 3 ASCII bytes from the data field (MSB first).
pub fn rdb_bytes(pkt: u32) -> [u8; 3] {
    let d = rdb_data(pkt);
    [((d >> 16) & 0xFF) as u8, ((d >> 8) & 0xFF) as u8, (d & 0xFF) as u8]
}
