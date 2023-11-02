// SPDX-License-Identifier: MIT

use core::ops::{Deref, DerefMut};
use core::{char, mem, slice};

pub const EVENT_NONE: i64 = 0;
pub const EVENT_KEY: i64 = 1;
pub const EVENT_MOUSE: i64 = 2;
pub const EVENT_BUTTON: i64 = 3;
pub const EVENT_SCROLL: i64 = 4;
pub const EVENT_QUIT: i64 = 5;
pub const EVENT_FOCUS: i64 = 6;
pub const EVENT_MOVE: i64 = 7;
pub const EVENT_RESIZE: i64 = 8;
pub const EVENT_SCREEN: i64 = 9;
pub const EVENT_CLIPBOARD: i64 = 10;
pub const EVENT_MOUSE_RELATIVE: i64 = 11;
pub const EVENT_DROP: i64 = 12;
pub const EVENT_TEXT_INPUT: i64 = 13;
pub const EVENT_CLIPBOARD_UPDATE: i64 = 14;
pub const EVENT_HOVER: i64 = 15;

/// An optional event
#[derive(Copy, Clone, Debug)]
pub enum EventOption {
    /// A key event
    Key(KeyEvent),
    /// A text input event
    TextInput(TextInputEvent),
    /// A mouse event (absolute)
    Mouse(MouseEvent),
    /// A mouse event (relative)
    MouseRelative(MouseRelativeEvent),
    /// A mouse button event
    Button(ButtonEvent),
    /// A mouse scroll event
    Scroll(ScrollEvent),
    /// A quit request event
    Quit(QuitEvent),
    /// A focus event
    Focus(FocusEvent),
    /// A move event
    Move(MoveEvent),
    /// A resize event
    Resize(ResizeEvent),
    /// A screen report event
    Screen(ScreenEvent),
    /// A clipboard event
    Clipboard(ClipboardEvent),
    /// A clipboard update event
    ClipboardUpdate(ClipboardUpdateEvent),
    /// A drop file / text event (available on linux, windows and macOS)
    Drop(DropEvent),
    /// A hover event
    Hover(HoverEvent),
    /// An unknown event
    Unknown(Event),
    /// No event
    None,
}

/// An event
#[derive(Copy, Clone, Debug)]
#[repr(packed)]
pub struct Event {
    pub code: i64,
    pub a: i64,
    pub b: i64,
}

#[allow(clippy::new_without_default)]
impl Event {
    /// Create a null event
    pub fn new() -> Event {
        Event {
            code: 0,
            a: 0,
            b: 0,
        }
    }

    /// Convert the event ot an optional event
    // TODO: Consider doing this via a From trait.
    pub fn to_option(self) -> EventOption {
        match self.code {
            EVENT_NONE => EventOption::None,
            EVENT_KEY => EventOption::Key(KeyEvent::from_event(self)),
            EVENT_TEXT_INPUT => EventOption::TextInput(TextInputEvent::from_event(self)),
            EVENT_MOUSE => EventOption::Mouse(MouseEvent::from_event(self)),
            EVENT_MOUSE_RELATIVE => {
                EventOption::MouseRelative(MouseRelativeEvent::from_event(self))
            }
            EVENT_BUTTON => EventOption::Button(ButtonEvent::from_event(self)),
            EVENT_SCROLL => EventOption::Scroll(ScrollEvent::from_event(self)),
            EVENT_QUIT => EventOption::Quit(QuitEvent::from_event(self)),
            EVENT_FOCUS => EventOption::Focus(FocusEvent::from_event(self)),
            EVENT_MOVE => EventOption::Move(MoveEvent::from_event(self)),
            EVENT_RESIZE => EventOption::Resize(ResizeEvent::from_event(self)),
            EVENT_SCREEN => EventOption::Screen(ScreenEvent::from_event(self)),
            EVENT_CLIPBOARD => EventOption::Clipboard(ClipboardEvent::from_event(self)),
            EVENT_CLIPBOARD_UPDATE => {
                EventOption::ClipboardUpdate(ClipboardUpdateEvent::from_event(self))
            }
            EVENT_DROP => EventOption::Drop(DropEvent::from_event(self)),
            EVENT_HOVER => EventOption::Hover(HoverEvent::from_event(self)),
            _ => EventOption::Unknown(self),
        }
    }
}

impl Deref for Event {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(self as *const Event as *const u8, mem::size_of::<Event>())
                as &[u8]
        }
    }
}

impl DerefMut for Event {
    fn deref_mut(&mut self) -> &mut [u8] {
        unsafe {
            slice::from_raw_parts_mut(self as *mut Event as *mut u8, mem::size_of::<Event>())
                as &mut [u8]
        }
    }
}

pub const K_A: u8 = 0x1E;
pub const K_B: u8 = 0x30;
pub const K_C: u8 = 0x2E;
pub const K_D: u8 = 0x20;
pub const K_E: u8 = 0x12;
pub const K_F: u8 = 0x21;
pub const K_G: u8 = 0x22;
pub const K_H: u8 = 0x23;
pub const K_I: u8 = 0x17;
pub const K_J: u8 = 0x24;
pub const K_K: u8 = 0x25;
pub const K_L: u8 = 0x26;
pub const K_M: u8 = 0x32;
pub const K_N: u8 = 0x31;
pub const K_O: u8 = 0x18;
pub const K_P: u8 = 0x19;
pub const K_Q: u8 = 0x10;
pub const K_R: u8 = 0x13;
pub const K_S: u8 = 0x1F;
pub const K_T: u8 = 0x14;
pub const K_U: u8 = 0x16;
pub const K_V: u8 = 0x2F;
pub const K_W: u8 = 0x11;
pub const K_X: u8 = 0x2D;
pub const K_Y: u8 = 0x15;
pub const K_Z: u8 = 0x2C;
pub const K_0: u8 = 0x0B;
pub const K_1: u8 = 0x02;
pub const K_2: u8 = 0x03;
pub const K_3: u8 = 0x04;
pub const K_4: u8 = 0x05;
pub const K_5: u8 = 0x06;
pub const K_6: u8 = 0x07;
pub const K_7: u8 = 0x08;
pub const K_8: u8 = 0x09;
pub const K_9: u8 = 0x0A;

// Numpad keys (codes 0x70-0x79)
pub const K_NUM_0: u8 = 0x70;
pub const K_NUM_1: u8 = 0x71;
pub const K_NUM_2: u8 = 0x72;
pub const K_NUM_3: u8 = 0x73;
pub const K_NUM_4: u8 = 0x74;
pub const K_NUM_5: u8 = 0x75;
pub const K_NUM_6: u8 = 0x76;
pub const K_NUM_7: u8 = 0x77;
pub const K_NUM_8: u8 = 0x78;
pub const K_NUM_9: u8 = 0x79;

/// Tick/tilde key
pub const K_TICK: u8 = 0x29;
/// Minus/underline key
pub const K_MINUS: u8 = 0x0C;
/// Equals/plus key
pub const K_EQUALS: u8 = 0x0D;
/// Backslash/pipe key
pub const K_BACKSLASH: u8 = 0x2B;
/// Bracket open key
pub const K_BRACE_OPEN: u8 = 0x1A;
/// Bracket close key
pub const K_BRACE_CLOSE: u8 = 0x1B;
/// Semicolon key
pub const K_SEMICOLON: u8 = 0x27;
/// Quote key
pub const K_QUOTE: u8 = 0x28;
/// Comma key
pub const K_COMMA: u8 = 0x33;
/// Period key
pub const K_PERIOD: u8 = 0x34;
/// Slash key
pub const K_SLASH: u8 = 0x35;
/// Backspace key
pub const K_BKSP: u8 = 0x0E;
/// Space key
pub const K_SPACE: u8 = 0x39;
/// Tab key
pub const K_TAB: u8 = 0x0F;
/// Capslock
pub const K_CAPS: u8 = 0x3A;
/// Left shift
pub const K_LEFT_SHIFT: u8 = 0x2A;
/// Right shift
pub const K_RIGHT_SHIFT: u8 = 0x36;
/// Control key
pub const K_CTRL: u8 = 0x1D;
/// Alt key
pub const K_ALT: u8 = 0x38;
/// AltGr key
pub const K_ALT_GR: u8 = 0x64;
/// Enter key
pub const K_ENTER: u8 = 0x1C;
/// Escape key
pub const K_ESC: u8 = 0x01;
/// F1 key
pub const K_F1: u8 = 0x3B;
/// F2 key
pub const K_F2: u8 = 0x3C;
/// F3 key
pub const K_F3: u8 = 0x3D;
/// F4 key
pub const K_F4: u8 = 0x3E;
/// F5 key
pub const K_F5: u8 = 0x3F;
/// F6 key
pub const K_F6: u8 = 0x40;
/// F7 key
pub const K_F7: u8 = 0x41;
/// F8 key
pub const K_F8: u8 = 0x42;
/// F9 key
pub const K_F9: u8 = 0x43;
/// F10 key
pub const K_F10: u8 = 0x44;
/// Home key
pub const K_HOME: u8 = 0x47;
/// Up key
pub const K_UP: u8 = 0x48;
/// Page up key
pub const K_PGUP: u8 = 0x49;
/// Left key
pub const K_LEFT: u8 = 0x4B;
/// Right key
pub const K_RIGHT: u8 = 0x4D;
/// End key
pub const K_END: u8 = 0x4F;
/// Down key
pub const K_DOWN: u8 = 0x50;
/// Page down key
pub const K_PGDN: u8 = 0x51;
/// Delete key
pub const K_DEL: u8 = 0x53;
/// F11 key
pub const K_F11: u8 = 0x57;
/// F12 key
pub const K_F12: u8 = 0x58;
/// SUPER/META/WIN Key
pub const K_SUPER : u8 = 0x5B;
/// Media Key for Volume toggle (mute/unmute)
pub const K_VOLUME_TOGGLE : u8 = 0x80 + 0x20;
/// Media Key for Volume Down
pub const K_VOLUME_DOWN : u8 = 0x80 + 0x2E;
/// Media Key for Volume Up
pub const K_VOLUME_UP : u8 = 0x80 + 0x30;

/// A key event (such as a pressed key)
#[derive(Copy, Clone, Debug)]
pub struct KeyEvent {
    /// The character of the key
    pub character: char,
    /// The scancode of the key
    pub scancode: u8,
    /// Was it pressed?
    pub pressed: bool,
}

impl KeyEvent {
    /// Convert to an `Event`
    pub fn to_event(&self) -> Event {
        Event {
            code: EVENT_KEY,
            a: self.character as i64,
            b: self.scancode as i64 | (self.pressed as i64) << 8,
        }
    }

    /// Convert from an `Event`
    pub fn from_event(event: Event) -> KeyEvent {
        KeyEvent {
            character: char::from_u32(event.a as u32).unwrap_or('\0'),
            scancode: event.b as u8,
            pressed: event.b & 1 << 8 == 1 << 8,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct TextInputEvent {
    pub character: char,
}

impl TextInputEvent {
    /// Convert to an `Event`
    pub fn to_event(&self) -> Event {
        Event {
            code: EVENT_TEXT_INPUT,
            a: self.character as i64,
            b: 0,
        }
    }

    /// Convert from an `Event`
    pub fn from_event(event: Event) -> TextInputEvent {
        TextInputEvent {
            character: char::from_u32(event.a as u32).unwrap_or('\0'),
        }
    }
}

/// A event related to the mouse (absolute position)
#[derive(Copy, Clone, Debug)]
pub struct MouseEvent {
    /// The x coordinate of the mouse
    pub x: i32,
    /// The y coordinate of the mouse
    pub y: i32,
}

impl MouseEvent {
    /// Convert to an `Event`
    pub fn to_event(&self) -> Event {
        Event {
            code: EVENT_MOUSE,
            a: self.x as i64,
            b: self.y as i64,
        }
    }

    /// Convert an `Event` to a `MouseEvent`
    pub fn from_event(event: Event) -> MouseEvent {
        MouseEvent {
            x: event.a as i32,
            y: event.b as i32,
        }
    }
}

/// A event related to the mouse (relative position)
#[derive(Copy, Clone, Debug)]
pub struct MouseRelativeEvent {
    /// The x coordinate of the mouse
    pub dx: i32,
    /// The y coordinate of the mouse
    pub dy: i32,
}

impl MouseRelativeEvent {
    /// Convert to an `Event`
    pub fn to_event(&self) -> Event {
        Event {
            code: EVENT_MOUSE_RELATIVE,
            a: self.dx as i64,
            b: self.dy as i64,
        }
    }

    /// Convert an `Event` to a `MouseRelativeEvent`
    pub fn from_event(event: Event) -> MouseRelativeEvent {
        MouseRelativeEvent {
            dx: event.a as i32,
            dy: event.b as i32,
        }
    }
}

/// A event for clicking the mouse
#[derive(Copy, Clone, Debug)]
pub struct ButtonEvent {
    /// Was the left button pressed?
    pub left: bool,
    /// Was the middle button pressed?
    pub middle: bool,
    /// Was the right button pressed?
    pub right: bool,
}

impl ButtonEvent {
    /// Convert to an `Event`
    pub fn to_event(&self) -> Event {
        Event {
            code: EVENT_BUTTON,
            a: self.left as i64 | (self.middle as i64) << 1 | (self.right as i64) << 2,
            b: 0,
        }
    }

    /// Convert an `Event` to a `ButtonEvent`
    pub fn from_event(event: Event) -> ButtonEvent {
        ButtonEvent {
            left: event.a & 1 == 1,
            middle: event.a & 2 == 2,
            right: event.a & 4 == 4,
        }
    }
}

/// A event for scrolling the mouse
#[derive(Copy, Clone, Debug)]
pub struct ScrollEvent {
    /// The x distance of the scroll
    pub x: i32,
    /// The y distance of the scroll
    pub y: i32,
}

impl ScrollEvent {
    /// Convert to an `Event`
    pub fn to_event(&self) -> Event {
        Event {
            code: EVENT_SCROLL,
            a: self.x as i64,
            b: self.y as i64,
        }
    }

    /// Convert an `Event` to a `ScrollEvent`
    pub fn from_event(event: Event) -> ScrollEvent {
        ScrollEvent {
            x: event.a as i32,
            y: event.b as i32,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct QuitEvent;

impl QuitEvent {
    pub fn to_event(&self) -> Event {
        Event {
            code: EVENT_QUIT,
            a: 0,
            b: 0,
        }
    }

    pub fn from_event(_: Event) -> QuitEvent {
        QuitEvent
    }
}

/// A focus event
#[derive(Copy, Clone, Debug)]
pub struct FocusEvent {
    /// True if window has been focused, false if not
    pub focused: bool,
}

impl FocusEvent {
    pub fn to_event(&self) -> Event {
        Event {
            code: EVENT_FOCUS,
            a: self.focused as i64,
            b: 0,
        }
    }

    pub fn from_event(event: Event) -> FocusEvent {
        FocusEvent {
            focused: event.a > 0,
        }
    }
}

/// A move event
#[derive(Copy, Clone, Debug)]
pub struct MoveEvent {
    pub x: i32,
    pub y: i32,
}

impl MoveEvent {
    pub fn to_event(&self) -> Event {
        Event {
            code: EVENT_MOVE,
            a: self.x as i64,
            b: self.y as i64,
        }
    }

    pub fn from_event(event: Event) -> MoveEvent {
        MoveEvent {
            x: event.a as i32,
            y: event.b as i32,
        }
    }
}

/// A resize event
#[derive(Copy, Clone, Debug)]
pub struct ResizeEvent {
    pub width: u32,
    pub height: u32,
}

impl ResizeEvent {
    pub fn to_event(&self) -> Event {
        Event {
            code: EVENT_RESIZE,
            a: self.width as i64,
            b: self.height as i64,
        }
    }

    pub fn from_event(event: Event) -> ResizeEvent {
        ResizeEvent {
            width: event.a as u32,
            height: event.b as u32,
        }
    }
}

/// A screen report event
#[derive(Copy, Clone, Debug)]
pub struct ScreenEvent {
    pub width: u32,
    pub height: u32,
}

impl ScreenEvent {
    pub fn to_event(&self) -> Event {
        Event {
            code: EVENT_SCREEN,
            a: self.width as i64,
            b: self.height as i64,
        }
    }

    pub fn from_event(event: Event) -> ScreenEvent {
        ScreenEvent {
            width: event.a as u32,
            height: event.b as u32,
        }
    }
}

pub const CLIPBOARD_COPY: u8 = 0;
pub const CLIPBOARD_CUT: u8 = 1;
pub const CLIPBOARD_PASTE: u8 = 2;

/// A clipboard event
#[derive(Copy, Clone, Debug)]
pub struct ClipboardUpdateEvent;

impl ClipboardUpdateEvent {
    pub fn to_event(&self) -> Event {
        Event {
            code: EVENT_CLIPBOARD_UPDATE,
            a: 0,
            b: 0,
        }
    }

    pub fn from_event(_: Event) -> ClipboardUpdateEvent {
        ClipboardUpdateEvent
    }
}

/// A clipboard event
#[derive(Copy, Clone, Debug)]
pub struct ClipboardEvent {
    pub kind: u8,
    pub size: usize,
}

impl ClipboardEvent {
    pub fn to_event(&self) -> Event {
        Event {
            code: EVENT_CLIPBOARD,
            a: self.kind as i64,
            b: self.size as i64,
        }
    }

    pub fn from_event(event: Event) -> ClipboardEvent {
        ClipboardEvent {
            kind: event.a as u8,
            size: event.b as usize,
        }
    }
}

pub const DROP_FILE: u8 = 0;
pub const DROP_TEXT: u8 = 1;

/// A drop file event.
#[derive(Copy, Clone, Debug)]
pub struct DropEvent {
    pub kind: u8,
}

impl DropEvent {
    pub fn to_event(&self) -> Event {
        Event {
            code: EVENT_DROP,
            a: self.kind as i64,
            b: 0,
        }
    }

    pub fn from_event(event: Event) -> DropEvent {
        DropEvent {
            kind: event.a as u8,
        }
    }
}

/// A hover event
#[derive(Copy, Clone, Debug)]
pub struct HoverEvent {
    /// True if window has been entered, false if exited
    pub entered: bool,
}

impl HoverEvent {
    pub fn to_event(&self) -> Event {
        Event {
            code: EVENT_HOVER,
            a: self.entered as i64,
            b: 0,
        }
    }

    pub fn from_event(event: Event) -> HoverEvent {
        HoverEvent {
            entered: event.a > 0,
        }
    }
}
