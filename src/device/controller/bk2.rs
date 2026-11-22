use std::collections::HashSet;
use std::io;
use std::path::Path;

use crate::ui;

enum Control {
    Console,
    Button(usize, usize),
    Axis(usize, usize),
}

pub struct Playback {
    frames: Vec<[u32; 4]>,
    connected: [bool; 4],
    frame: usize,
}

impl Playback {
    pub fn open(path: &Path) -> io::Result<Self> {
        let result = std::fs::read_to_string(path).and_then(|text| Self::parse(&text));
        result.map_err(|err| {
            io::Error::new(
                err.kind(),
                format!("Could not load BK2 input log {}: {err}", path.display()),
            )
        })
    }

    fn parse(text: &str) -> io::Result<Self> {
        let mut controls = None;
        let mut connected = [false; 4];
        let mut frames = Vec::new();
        let mut ended = false;

        for (i, line) in text.trim_start_matches('\u{feff}').lines().enumerate() {
            let line_number = i + 1;
            if line.trim().is_empty() {
                continue;
            }
            let result = if ended {
                Err("unexpected content after [/Input]".into())
            } else if line == "[Input]" && controls.is_none() {
                Ok(())
            } else if line == "[/Input]" && controls.is_some() {
                ended = true;
                Ok(())
            } else if let Some(key) = line.strip_prefix("LogKey:")
                && controls.is_none()
            {
                parse_log_key(key, &mut connected).map(|layout| controls = Some(layout))
            } else if line.starts_with('|') {
                match &controls {
                    Some(controls) => parse_frame(line, controls).map(|frame| frames.push(frame)),
                    None => Err("expected a LogKey before the first frame".into()),
                }
            } else {
                Err("expected a LogKey or a frame beginning with '|'".into())
            };
            result.map_err(|err| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("line {line_number}: {err}"),
                )
            })?;
        }

        if frames.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "no input frames",
            ));
        }
        Ok(Self {
            frames,
            connected,
            frame: 0,
        })
    }

    pub fn connected(&self, port: usize) -> bool {
        self.connected[port]
    }

    pub fn input(&self, port: usize) -> ui::input::InputData {
        ui::input::InputData {
            data: self.frames.get(self.frame).map_or(0, |frame| frame[port]),
            pak_change_pressed: false,
        }
    }

    pub fn advance(&mut self) {
        if self.frame < self.frames.len() {
            self.frame += 1;
        }
    }
}

fn parse_log_key(key: &str, connected: &mut [bool; 4]) -> Result<Vec<Control>, String> {
    let mut names = HashSet::new();
    let mut controls = Vec::new();
    let mut axes = [[false; 2]; 4];
    for name in key.split(['#', '|']).filter(|name| !name.is_empty()) {
        if !names.insert(name) {
            return Err(format!("duplicate control '{name}'"));
        }
        if matches!(name, "Reset" | "Power") {
            controls.push(Control::Console);
            continue;
        }
        let Some((player, name)) = name.split_once(' ') else {
            return Err(format!("unknown N64 control '{name}'"));
        };
        let port = match player {
            "P1" => 0,
            "P2" => 1,
            "P3" => 2,
            "P4" => 3,
            _ => return Err(format!("unknown controller '{player}'")),
        };
        connected[port] = true;

        use ui::input_profile::*;
        let control = match name {
            "X Axis" | "Y Axis" => {
                let axis = usize::from(name == "Y Axis");
                axes[port][axis] = true;
                Control::Axis(port, axis)
            }
            "DPad R" => Control::Button(port, R_DPAD),
            "DPad L" => Control::Button(port, L_DPAD),
            "DPad D" => Control::Button(port, D_DPAD),
            "DPad U" => Control::Button(port, U_DPAD),
            "Start" => Control::Button(port, START_BUTTON),
            "Z" => Control::Button(port, Z_TRIG),
            "B" => Control::Button(port, B_BUTTON),
            "A" => Control::Button(port, A_BUTTON),
            "C Right" => Control::Button(port, R_CBUTTON),
            "C Left" => Control::Button(port, L_CBUTTON),
            "C Down" => Control::Button(port, D_CBUTTON),
            "C Up" => Control::Button(port, U_CBUTTON),
            "R" => Control::Button(port, R_TRIG),
            "L" => Control::Button(port, L_TRIG),
            _ => return Err(format!("unknown N64 control '{player} {name}'")),
        };
        controls.push(control);
    }
    if !connected.iter().any(|&present| present) {
        return Err("LogKey has no N64 controllers".into());
    }
    for (port, &present) in connected.iter().enumerate() {
        if present && axes[port] != [true; 2] {
            return Err(format!("P{} must have both X Axis and Y Axis", port + 1));
        }
    }
    Ok(controls)
}

fn parse_frame(mut line: &str, controls: &[Control]) -> Result<[u32; 4], String> {
    let mut buttons = [0u16; 4];
    let mut axes = [[0i8; 2]; 4];

    for control in controls {
        line = line.trim_start_matches('|');
        match *control {
            Control::Axis(port, axis) => {
                let Some((value, rest)) = line.split_once(',') else {
                    return Err("expected a stick value followed by a comma".into());
                };
                axes[port][axis] = match value.trim().parse::<i8>() {
                    Ok(number) if value.len() == 5 => number,
                    _ => {
                        return Err(format!(
                            "invalid stick value '{value}' (expected a 5-character field from -128 to 127)"
                        ));
                    }
                };
                line = rest;
            }
            Control::Console => {
                line = line
                    .strip_prefix('.')
                    .ok_or("expected '.' for an unused Reset or Power control")?;
            }
            Control::Button(port, bit) => {
                let Some(value) = line.chars().next() else {
                    return Err("missing button value".into());
                };
                if value != '.' {
                    buttons[port] |= 1 << bit;
                }
                line = &line[value.len_utf8()..];
            }
        }
    }
    if !line.trim_matches('|').is_empty() {
        return Err("extra values after the last control".into());
    }

    Ok(std::array::from_fn(|port| {
        let [x, y] = axes[port];
        let [lo, hi] = buttons[port].to_le_bytes();
        u32::from_le_bytes([lo, hi, x as u8, y as u8])
    }))
}
