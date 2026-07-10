//! Pure GameCube adapter report parsing, stick calibration, and GameCube button
//! decoding.
//!
//! No I/O lives here: every function is deterministic and unit-testable without
//! hardware. This module knows the adapter's byte layout; the GameCube -> N64
//! button mapping itself lives in `input.rs`, next to the N64 bit definitions.

const REPORT_LEN: usize = 37;
const REPORT_HEADER: u8 = 0x21;
const PORT_STRIDE: usize = 9;

// Controller-type byte: bit 4 set = wired, bit 5 set = wireless, else absent.
const TYPE_CONNECTED: u8 = 0x10 | 0x20;

// `b1` button bits.
const B1_A: u8 = 1 << 0;
const B1_B: u8 = 1 << 1;
const B1_X: u8 = 1 << 2;
// GC Y (bit 3) has no N64 equivalent and is intentionally not decoded.
const B1_LEFT: u8 = 1 << 4;
const B1_RIGHT: u8 = 1 << 5;
const B1_DOWN: u8 = 1 << 6;
const B1_UP: u8 = 1 << 7;

// `b2` button bits (R/L are the analog triggers' digital click).
const B2_START: u8 = 1 << 0;
const B2_Z: u8 = 1 << 1;
const B2_R: u8 = 1 << 2;
const B2_L: u8 = 1 << 3;

/// GameCube stick travel from center to a cardinal extreme, in raw byte units
/// (center `0x80`, radius `0x7f`). Used to normalize sticks to `[-1.0, 1.0]`.
const GC_STICK_RADIUS: f64 = 0x7f as f64;

/// A single adapter port's most recent raw report bytes plus its connection
/// state. Calibration is applied at map time (see [`gc_stick`]), so this stays a
/// faithful copy of what the device sent.
#[derive(Default, Clone, Copy)]
pub struct GcPortState {
    pub connected: bool,
    pub b1: u8,
    pub b2: u8,
    pub stick_x: u8,
    pub stick_y: u8,
    pub cstick_x: u8,
    pub cstick_y: u8,
}

impl GcPortState {
    pub fn a(&self) -> bool {
        self.b1 & B1_A != 0
    }
    pub fn b(&self) -> bool {
        self.b1 & B1_B != 0
    }
    pub fn x(&self) -> bool {
        self.b1 & B1_X != 0
    }
    pub fn dpad_left(&self) -> bool {
        self.b1 & B1_LEFT != 0
    }
    pub fn dpad_right(&self) -> bool {
        self.b1 & B1_RIGHT != 0
    }
    pub fn dpad_down(&self) -> bool {
        self.b1 & B1_DOWN != 0
    }
    pub fn dpad_up(&self) -> bool {
        self.b1 & B1_UP != 0
    }
    pub fn start(&self) -> bool {
        self.b2 & B2_START != 0
    }
    pub fn z(&self) -> bool {
        self.b2 & B2_Z != 0
    }
    /// Digital click of the analog R trigger.
    pub fn r(&self) -> bool {
        self.b2 & B2_R != 0
    }
    /// Digital click of the analog L trigger.
    pub fn l(&self) -> bool {
        self.b2 & B2_L != 0
    }
}

/// Per-port neutral-position calibration, captured the first time a controller
/// is seen on a port (Dolphin-style origin).
#[derive(Default, Clone, Copy)]
pub struct GcCalibration {
    pub origin_x: u8,
    pub origin_y: u8,
    pub origin_cx: u8,
    pub origin_cy: u8,
}

/// Parse a 37-byte adapter input report into the four port states. Returns
/// `None` if the buffer length or header byte is invalid (the boundary check for
/// the untrusted USB payload).
pub fn parse_report(report: &[u8]) -> Option<[GcPortState; 4]> {
    if report.len() != REPORT_LEN || report[0] != REPORT_HEADER {
        return None;
    }

    let mut ports = [GcPortState::default(); 4];
    for (port, state) in ports.iter_mut().enumerate() {
        let base = 1 + PORT_STRIDE * port;
        state.connected = report[base] & TYPE_CONNECTED != 0;
        state.b1 = report[base + 1];
        state.b2 = report[base + 2];
        state.stick_x = report[base + 3];
        state.stick_y = report[base + 4];
        state.cstick_x = report[base + 5];
        state.cstick_y = report[base + 6];
        // Bytes 7-8 are the analog L/R trigger pressures, unused by the N64 mapping.
    }
    Some(ports)
}

/// Capture the neutral origin for a port from a valid report: its current stick
/// positions become the calibrated center.
pub fn capture_origin(state: &GcPortState) -> GcCalibration {
    GcCalibration {
        origin_x: state.stick_x,
        origin_y: state.stick_y,
        origin_cx: state.cstick_x,
        origin_cy: state.cstick_y,
    }
}

fn normalize(value: u8, origin: u8) -> f64 {
    (value as f64 - origin as f64) / GC_STICK_RADIUS
}

/// Origin-corrected, radius-scaled main stick as normalized axes in roughly
/// `[-1.0, 1.0]` (up = positive). The caller scales to the N64 range and applies
/// the deadzone and bounding.
pub fn gc_stick(state: &GcPortState, cal: &GcCalibration) -> (f64, f64) {
    (
        normalize(state.stick_x, cal.origin_x),
        normalize(state.stick_y, cal.origin_y),
    )
}

/// Origin-corrected, radius-scaled C-stick as normalized axes in roughly
/// `[-1.0, 1.0]` (up = positive). The caller thresholds these into C-buttons.
pub fn gc_cstick(state: &GcPortState, cal: &GcCalibration) -> (f64, f64) {
    (
        normalize(state.cstick_x, cal.origin_cx),
        normalize(state.cstick_y, cal.origin_cy),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // Build a 37-byte report with one port populated at the given offset bytes.
    fn report_with_port(port: usize, bytes: [u8; PORT_STRIDE]) -> Vec<u8> {
        let mut r = vec![0u8; REPORT_LEN];
        r[0] = REPORT_HEADER;
        let base = 1 + PORT_STRIDE * port;
        r[base..base + PORT_STRIDE].copy_from_slice(&bytes);
        r
    }

    #[test]
    fn rejects_bad_length() {
        assert!(parse_report(&[REPORT_HEADER; 10]).is_none());
        assert!(parse_report(&[]).is_none());
    }

    #[test]
    fn rejects_bad_header() {
        let mut r = vec![0u8; REPORT_LEN];
        r[0] = 0x00;
        assert!(parse_report(&r).is_none());
    }

    #[test]
    fn parses_each_port_field() {
        // Port 2: wired, A+Start pressed, sticks set; trailing trigger bytes ignored.
        let bytes = [0x10, B1_A, B2_START, 0x90, 0x70, 0xa0, 0x60, 0x11, 0x22];
        let ports = parse_report(&report_with_port(2, bytes)).unwrap();

        assert!(!ports[0].connected);
        let p = ports[2];
        assert!(p.connected);
        assert!(p.a() && p.start());
        assert!(!p.b());
        assert_eq!(p.stick_x, 0x90);
        assert_eq!(p.stick_y, 0x70);
        assert_eq!(p.cstick_x, 0xa0);
        assert_eq!(p.cstick_y, 0x60);
    }

    #[test]
    fn decodes_every_button_bit() {
        let all_b1 = GcPortState {
            b1: 0xff,
            ..Default::default()
        };
        assert!(
            all_b1.a()
                && all_b1.b()
                && all_b1.x()
                && all_b1.dpad_left()
                && all_b1.dpad_right()
                && all_b1.dpad_down()
                && all_b1.dpad_up()
        );
        let all_b2 = GcPortState {
            b2: 0xff,
            ..Default::default()
        };
        assert!(all_b2.start() && all_b2.z() && all_b2.r() && all_b2.l());
    }

    #[test]
    fn disconnected_when_type_byte_zero() {
        let bytes = [0x00, B1_A, 0, 0x80, 0x80, 0x80, 0x80, 0, 0];
        let ports = parse_report(&report_with_port(0, bytes)).unwrap();
        assert!(!ports[0].connected);
    }

    #[test]
    fn stick_centered_at_origin_is_zero() {
        let state = GcPortState {
            stick_x: 0x80,
            stick_y: 0x80,
            ..Default::default()
        };
        let cal = capture_origin(&state);
        let (x, y) = gc_stick(&state, &cal);
        assert!(x.abs() < 1e-9 && y.abs() < 1e-9);
    }

    #[test]
    fn full_deflection_reaches_unit_magnitude() {
        let cal = GcCalibration {
            origin_x: 0x80,
            origin_y: 0x80,
            ..Default::default()
        };
        // origin + radius (0x80 + 0x7f = 0xff) -> +1.0 exactly.
        let up_right = GcPortState {
            stick_x: 0xff,
            stick_y: 0xff,
            ..Default::default()
        };
        let (x, y) = gc_stick(&up_right, &cal);
        assert!((x - 1.0).abs() < 1e-9);
        assert!((y - 1.0).abs() < 1e-9);
        // Bottom-left clamps slightly past -1.0 (caller bounds it).
        let down_left = GcPortState {
            stick_x: 0x00,
            stick_y: 0x00,
            ..Default::default()
        };
        let (x, y) = gc_stick(&down_left, &cal);
        assert!(x < -1.0 && y < -1.0);
    }

    #[test]
    fn off_center_origin_is_subtracted() {
        // A pad whose neutral rests at 0x88 should read zero there, not at 0x80.
        let neutral = GcPortState {
            stick_x: 0x88,
            stick_y: 0x78,
            ..Default::default()
        };
        let cal = capture_origin(&neutral);
        let (x, y) = gc_stick(&neutral, &cal);
        assert!(x.abs() < 1e-9 && y.abs() < 1e-9);
    }
}
