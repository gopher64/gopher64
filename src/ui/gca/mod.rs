//! GameCube controller support via the Nintendo Wii U USB adapter.
//!
//! [`protocol`] holds the pure parsing, calibration, and decoding logic. This
//! module owns the USB transport: a background thread that opens the adapter,
//! streams input reports into a shared snapshot, and writes rumble.
//!
//! The transport is nusb-backed and desktop-only; Android keeps the no-op stub
//! below so the rest of the UI compiles unconditionally without scattering `cfg`
//! through the input pipeline. The public surface is the same on every target:
//! [`adapter_present`] and [`Adapter`].

pub mod protocol;

pub use protocol::{GcCalibration, GcPortState};

#[cfg(not(target_os = "android"))]
pub use desktop::{Adapter, adapter_present};

#[cfg(target_os = "android")]
pub use stub::{Adapter, adapter_present};

#[cfg(target_os = "android")]
mod stub {
    use super::{GcCalibration, GcPortState};

    pub fn adapter_present() -> bool {
        false
    }

    #[derive(Default)]
    pub struct Adapter {}

    impl Adapter {
        pub fn start() -> Adapter {
            Adapter::default()
        }
        pub fn port_state(&self, _port: usize) -> (GcPortState, GcCalibration) {
            (GcPortState::default(), GcCalibration::default())
        }
        pub fn set_rumble(&self, _port: usize, _on: bool) {}
    }
}

#[cfg(not(target_os = "android"))]
mod desktop {
    use super::protocol::{self, GcCalibration, GcPortState};
    use nusb::descriptors::TransferType;
    use nusb::transfer::TransferError;
    use nusb::transfer::{ControlOut, ControlType, Direction, In, Interrupt, Out, Recipient};
    use nusb::{DeviceInfo, Endpoint, Interface, MaybeFuture};
    use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread::JoinHandle;
    use std::time::Duration;

    const VID: u16 = 0x057e;
    const PID: u16 = 0x0337;
    const INTERFACE: u8 = 0;
    const INIT_BYTE: u8 = 0x13;
    const RUMBLE_CMD: u8 = 0x11;
    // Class control request some third-party adapters (Nyko) need before they
    // start streaming; the official adapter and Mayflash reject it (ignored).
    const NYKO_INIT_REQUEST: u8 = 11;
    const NYKO_INIT_VALUE: u16 = 0x0001;
    // Endpoint addresses used if descriptor discovery somehow finds nothing.
    const FALLBACK_IN: u8 = 0x81;
    const FALLBACK_OUT: u8 = 0x02;

    const READ_TIMEOUT: Duration = Duration::from_millis(16);
    const WRITE_TIMEOUT: Duration = Duration::from_millis(100);
    const CONTROL_TIMEOUT: Duration = Duration::from_millis(1000);
    const RECONNECT_DELAY: Duration = Duration::from_millis(500);
    // Keep a few reads in flight so the host controller always has work pending.
    const PENDING_READS: usize = 4;
    const PORTS: usize = 4;

    type Shared = Arc<Mutex<([GcPortState; PORTS], [GcCalibration; PORTS])>>;
    type EpIn = Endpoint<Interrupt, In>;
    type EpOut = Endpoint<Interrupt, Out>;

    /// Owns the background USB reader thread and the shared per-port snapshot.
    /// The single owner lives in [`crate::ui::Input`]; dropping it stops and
    /// joins the thread.
    pub struct Adapter {
        shared: Shared,
        rumble: Arc<[AtomicU8; PORTS]>,
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }

    impl Adapter {
        /// Start the reader thread. Returns even when no adapter is present yet;
        /// the thread retries connecting, giving basic hotplug support.
        pub fn start() -> Adapter {
            let shared: Shared = Arc::new(Mutex::new((
                [GcPortState::default(); PORTS],
                [GcCalibration::default(); PORTS],
            )));
            let rumble = Arc::new([
                AtomicU8::new(0),
                AtomicU8::new(0),
                AtomicU8::new(0),
                AtomicU8::new(0),
            ]);
            let stop = Arc::new(AtomicBool::new(false));

            let thread = {
                let shared = shared.clone();
                let rumble = rumble.clone();
                let stop = stop.clone();
                std::thread::spawn(move || run(&shared, &rumble, &stop))
            };

            Adapter {
                shared,
                rumble,
                stop,
                thread: Some(thread),
            }
        }

        /// Latest snapshot and calibration for an adapter port (`0..=3`).
        pub fn port_state(&self, port: usize) -> (GcPortState, GcCalibration) {
            debug_assert!(port < PORTS);
            // Recover from a poisoned lock: a panic in the USB thread must never
            // take down the emulation thread, and the bytes are still valid.
            let guard = self.shared.lock().unwrap_or_else(|e| e.into_inner());
            (guard.0[port], guard.1[port])
        }

        /// Set the rumble motor on or off for an adapter port (`0..=3`).
        pub fn set_rumble(&self, port: usize, on: bool) {
            debug_assert!(port < PORTS);
            self.rumble[port].store(on as u8, Ordering::Relaxed);
        }
    }

    impl Drop for Adapter {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    /// True when a usable adapter is connected. On Windows this additionally
    /// requires the WinUSB driver to be bound, so ports are only offered once
    /// they can actually be opened.
    pub fn adapter_present() -> bool {
        nusb::list_devices()
            .wait()
            .map(|mut devices| devices.any(|d| is_adapter(&d)))
            .unwrap_or(false)
    }

    fn is_adapter(info: &DeviceInfo) -> bool {
        if info.vendor_id() != VID || info.product_id() != PID {
            return false;
        }
        // nusb enumerates every USB device on Windows regardless of driver, but
        // only a WinUSB-bound device can be opened; require it so the config UI
        // never offers ports that would fail to open.
        #[cfg(target_os = "windows")]
        {
            info.driver()
                .is_some_and(|d| d.eq_ignore_ascii_case("winusb"))
        }
        #[cfg(not(target_os = "windows"))]
        {
            true
        }
    }

    fn run(shared: &Shared, rumble: &Arc<[AtomicU8; PORTS]>, stop: &Arc<AtomicBool>) {
        let mut announced_failure = false;
        while !stop.load(Ordering::Relaxed) {
            match connect() {
                Some((interface, mut ep_in, mut ep_out)) => {
                    announced_failure = false;
                    run_io(&mut ep_in, &mut ep_out, shared, rumble, stop);
                    // Endpoints/interface keep the device open; drop them and
                    // reset the snapshot before attempting to reconnect.
                    drop((ep_in, ep_out, interface));
                    clear_connection(shared);
                }
                None => {
                    if !announced_failure {
                        eprintln!(
                            "GameCube adapter: not available (check it is plugged in and, on \
                             Windows, that the WinUSB driver is installed with Zadig)"
                        );
                        announced_failure = true;
                    }
                    std::thread::sleep(RECONNECT_DELAY);
                }
            }
        }
    }

    fn connect() -> Option<(Interface, EpIn, EpOut)> {
        let info = nusb::list_devices().wait().ok()?.find(is_adapter)?;
        let device = info.open().wait().ok()?;
        // Detaches the kernel driver on Linux; a plain claim elsewhere.
        let interface = device.detach_and_claim_interface(INTERFACE).wait().ok()?;
        let (in_addr, out_addr) = endpoints(&interface);
        let ep_in = interface.endpoint::<Interrupt, In>(in_addr).ok()?;
        let mut ep_out = interface.endpoint::<Interrupt, Out>(out_addr).ok()?;
        init(&interface, &mut ep_out);
        Some((interface, ep_in, ep_out))
    }

    fn endpoints(interface: &Interface) -> (u8, u8) {
        let mut in_addr = None;
        let mut out_addr = None;
        if let Some(desc) = interface.descriptor() {
            for ep in desc.endpoints() {
                if ep.transfer_type() != TransferType::Interrupt {
                    continue;
                }
                match ep.direction() {
                    Direction::In => in_addr = Some(ep.address()),
                    Direction::Out => out_addr = Some(ep.address()),
                }
            }
        }
        (
            in_addr.unwrap_or(FALLBACK_IN),
            out_addr.unwrap_or(FALLBACK_OUT),
        )
    }

    fn init(interface: &Interface, ep_out: &mut EpOut) {
        // Best-effort init some third-party adapters need (Nyko); the official
        // adapter and Mayflash error here, which we ignore, matching Dolphin.
        let _ = interface
            .control_out(
                ControlOut {
                    control_type: ControlType::Class,
                    recipient: Recipient::Interface,
                    request: NYKO_INIT_REQUEST,
                    value: NYKO_INIT_VALUE,
                    index: 0,
                    data: &[],
                },
                CONTROL_TIMEOUT,
            )
            .wait();
        // Required: ask the adapter to start streaming input reports.
        let _ = ep_out.transfer_blocking(vec![INIT_BYTE].into(), WRITE_TIMEOUT);
    }

    fn run_io(
        ep_in: &mut EpIn,
        ep_out: &mut EpOut,
        shared: &Shared,
        rumble: &Arc<[AtomicU8; PORTS]>,
        stop: &Arc<AtomicBool>,
    ) {
        let read_len = ep_in.max_packet_size();
        while ep_in.pending() < PENDING_READS {
            let buffer = ep_in.allocate(read_len);
            ep_in.submit(buffer);
        }

        let mut last_rumble = [0u8; PORTS];
        while !stop.load(Ordering::Relaxed) {
            // `None` means the read timed out with the transfer still pending; we
            // just loop to re-check the stop flag and service rumble.
            if let Some(completion) = ep_in.wait_next_complete(READ_TIMEOUT) {
                match completion.status {
                    Ok(()) => {
                        let len = completion.actual_len;
                        update_state(shared, &completion.buffer[..len]);
                        ep_in.submit(completion.buffer);
                    }
                    Err(TransferError::Cancelled) => ep_in.submit(completion.buffer),
                    Err(TransferError::Stall) => {
                        let _ = ep_in.clear_halt().wait();
                        ep_in.submit(completion.buffer);
                    }
                    Err(_) => return, // disconnected or fault: reconnect
                }
            }
            send_rumble(ep_out, rumble, &mut last_rumble);
        }
    }

    fn update_state(shared: &Shared, report: &[u8]) {
        let Some(ports) = protocol::parse_report(report) else {
            return;
        };
        let mut guard = shared.lock().unwrap_or_else(|e| e.into_inner());
        let (snapshots, calibrations) = &mut *guard;
        for ((snapshot, calibration), port) in snapshots
            .iter_mut()
            .zip(calibrations.iter_mut())
            .zip(ports.iter())
        {
            let was_connected = snapshot.connected;
            *snapshot = *port;
            if port.connected && !was_connected {
                *calibration = protocol::capture_origin(port);
            }
        }
    }

    fn clear_connection(shared: &Shared) {
        let mut guard = shared.lock().unwrap_or_else(|e| e.into_inner());
        guard.0 = [GcPortState::default(); PORTS];
        guard.1 = [GcCalibration::default(); PORTS];
    }

    fn send_rumble(ep_out: &mut EpOut, rumble: &Arc<[AtomicU8; PORTS]>, last: &mut [u8; PORTS]) {
        let current = [
            rumble[0].load(Ordering::Relaxed),
            rumble[1].load(Ordering::Relaxed),
            rumble[2].load(Ordering::Relaxed),
            rumble[3].load(Ordering::Relaxed),
        ];
        if current == *last {
            return;
        }
        *last = current;
        let payload = vec![RUMBLE_CMD, current[0], current[1], current[2], current[3]];
        let _ = ep_out.transfer_blocking(payload.into(), WRITE_TIMEOUT);
    }
}
