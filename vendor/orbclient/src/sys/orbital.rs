// SPDX-License-Identifier: MIT

use std::cell::Cell;
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
use std::{env, mem, slice, thread};

use crate::color::Color;
use crate::event::{Event, EVENT_RESIZE};
use crate::renderer::Renderer;
use crate::Mode;
use crate::WindowFlag;

pub fn get_display_size() -> Result<(u32, u32), String> {
    let display_path = env::var("DISPLAY").or(Err("DISPLAY not set"))?;
    match File::open(&display_path) {
        Ok(display) => {
            let mut buf: [u8; 4096] = [0; 4096];
            let count = syscall::fpath(display.as_raw_fd() as usize, &mut buf)
                .map_err(|err| format!("{}", err))?;
            let path = unsafe { String::from_utf8_unchecked(Vec::from(&buf[..count])) };
            let res = path.split(":").nth(1).unwrap_or("");
            let width = res
                .split("/")
                .nth(1)
                .unwrap_or("")
                .parse::<u32>()
                .unwrap_or(0);
            let height = res
                .split("/")
                .nth(2)
                .unwrap_or("")
                .parse::<u32>()
                .unwrap_or(0);
            Ok((width, height))
        }
        Err(err) => Err(format!("{}", err)),
    }
}

/// A window
pub struct Window {
    /// The x coordinate of the window
    x: i32,
    /// The y coordinate of the window
    y: i32,
    /// The width of the window
    w: u32,
    /// The height of the window
    h: u32,
    /// The title of the window
    t: String,
    /// True if the window should not wait for events
    window_async: bool,
    /// True if the window can be resized
    resizable: bool,
    /// Drawing mode
    mode: Cell<Mode>,
    /// The input scheme
    file_opt: Option<File>,
    /// Window data
    data_opt: Option<&'static mut [Color]>,
}

impl Renderer for Window {
    /// Get width
    fn width(&self) -> u32 {
        self.w
    }

    /// Get height
    fn height(&self) -> u32 {
        self.h
    }

    /// Access pixel buffer
    fn data(&self) -> &[Color] {
        self.data_opt.as_ref().unwrap()
    }

    /// Access pixel buffer mutably
    fn data_mut(&mut self) -> &mut [Color] {
        self.data_opt.as_mut().unwrap()
    }

    /// Flip the buffer
    fn sync(&mut self) -> bool {
        self.file_mut().sync_data().is_ok()
    }

    /// Set/get mode
    fn mode(&self) -> &Cell<Mode> {
        &self.mode
    }
}

impl Window {
    /// Create a new window
    pub fn new(x: i32, y: i32, w: u32, h: u32, title: &str) -> Option<Self> {
        Window::new_flags(x, y, w, h, title, &[])
    }

    /// Create a new window with flags
    pub fn new_flags(
        x: i32,
        y: i32,
        w: u32,
        h: u32,
        title: &str,
        flags: &[WindowFlag],
    ) -> Option<Self> {
        let mut flag_str = String::new();

        let mut window_async = false;
        let mut resizable = false;
        for &flag in flags.iter() {
            match flag {
                WindowFlag::Async => {
                    window_async = true;
                    flag_str.push('a');
                }
                WindowFlag::Back => flag_str.push('b'),
                WindowFlag::Front => flag_str.push('f'),
                WindowFlag::Borderless => flag_str.push('l'),
                WindowFlag::Resizable => {
                    resizable = true;
                    flag_str.push('r');
                }
                WindowFlag::Transparent => flag_str.push('t'),
                WindowFlag::Unclosable => flag_str.push('u'),
            }
        }

        if let Ok(file) = File::open(&format!(
            "orbital:{}/{}/{}/{}/{}/{}",
            flag_str, x, y, w, h, title
        )) {
            let mut window = Window {
                x: x,
                y: y,
                w: w,
                h: h,
                t: title.to_string(),
                window_async,
                resizable: resizable,
                mode: Cell::new(Mode::Blend),
                file_opt: Some(file),
                data_opt: None,
            };
            unsafe { window.remap(); }
            Some(window)
        } else {
            None
        }
    }

    pub fn clipboard(&self) -> String {
        let mut text = String::new();
        let window_fd = self.file().as_raw_fd();
        if let Ok(clipboard_fd) = syscall::dup(window_fd as usize, b"clipboard") {
            let mut clipboard_file = unsafe { File::from_raw_fd(clipboard_fd as RawFd) };
            let _ = clipboard_file.read_to_string(&mut text);
        }
        text
    }

    pub fn set_clipboard(&mut self, text: &str) {
        let window_fd = self.file().as_raw_fd();
        if let Ok(clipboard_fd) = syscall::dup(window_fd as usize, b"clipboard") {
            let mut clipboard_file = unsafe { File::from_raw_fd(clipboard_fd as RawFd) };
            let _ = clipboard_file.write(text.as_bytes());
        }
    }

    /// Not yet available on Redox OS.
    pub fn pop_drop_content(&self) -> Option<String> {
        None
    }

    // TODO: Replace with smarter mechanism, maybe a move event?
    pub fn sync_path(&mut self) {
        let mut buf: [u8; 4096] = [0; 4096];
        if let Ok(count) = syscall::fpath(self.file().as_raw_fd() as usize, &mut buf) {
            let path = unsafe { String::from_utf8_unchecked(Vec::from(&buf[..count])) };
            // orbital:/x/y/w/h/t
            let mut parts = path.split('/');
            if let Some(flags) = parts.next() {
                self.window_async = flags.contains('a');
                self.resizable = flags.contains('r');
            }
            if let Some(x) = parts.next() {
                self.x = x.parse::<i32>().unwrap_or(0);
            }
            if let Some(y) = parts.next() {
                self.y = y.parse::<i32>().unwrap_or(0);
            }
            if let Some(w) = parts.next() {
                self.w = w.parse::<u32>().unwrap_or(0);
            }
            if let Some(h) = parts.next() {
                self.h = h.parse::<u32>().unwrap_or(0);
            }
            if let Some(t) = parts.next() {
                self.t = t.to_string();
            }
        }
    }

    /// Get x
    // TODO: Sync with window movements
    pub fn x(&self) -> i32 {
        self.x
    }

    /// Get y
    // TODO: Sync with window movements
    pub fn y(&self) -> i32 {
        self.y
    }

    /// Get title
    pub fn title(&self) -> String {
        self.t.clone()
    }

    /// Get async
    pub fn is_async(&self) -> bool {
        self.window_async
    }

    /// Set async
    pub fn set_async(&mut self, is_async: bool) {
        self.window_async = is_async;
        let _ = self.file_mut().write(if is_async { b"A,1" } else { b"A,0" });
    }

    /// Set cursor visibility
    pub fn set_mouse_cursor(&mut self, visible: bool) {
        let _ = self.file_mut().write(if visible { b"M,C,1" } else { b"M,C,0" });
    }

    /// Set mouse grabbing
    pub fn set_mouse_grab(&mut self, grab: bool) {
        let _ = self.file_mut().write(if grab { b"M,G,1" } else { b"M,G,0" });
    }

    /// Set mouse relative mode
    pub fn set_mouse_relative(&mut self, relative: bool) {
        let _ = self.file_mut().write(if relative { b"M,R,1" } else { b"M,R,0" });
    }

    /// Set position
    pub fn set_pos(&mut self, x: i32, y: i32) {
        let _ = self.file_mut().write(&format!("P,{},{}", x, y).as_bytes());
        self.sync_path();
    }

    /// Set size
    pub fn set_size(&mut self, width: u32, height: u32) {
        //TODO: Improve safety and reliability
        unsafe { self.unmap(); }

        let _ = self
            .file_mut()
            .write(&format!("S,{},{}", width, height).as_bytes());
        self.sync_path();

        unsafe { self.remap(); }
    }

    /// Set title
    pub fn set_title(&mut self, title: &str) {
        let _ = self.file_mut().write(&format!("T,{}", title).as_bytes());
        self.sync_path();
    }

    /// Blocking iterator over events
    pub fn events(&mut self) -> EventIter {
        let mut iter = EventIter {
            extra: None,
            events: [Event::new(); 16],
            i: 0,
            count: 0,
        };

        'blocking: loop {
            if iter.count == iter.events.len() {
                if iter.extra.is_none() {
                    iter.extra = Some(Vec::with_capacity(32));
                }
                iter.extra.as_mut().unwrap().extend_from_slice(&iter.events);
                iter.count = 0;
            }
            let bytes = unsafe {
                slice::from_raw_parts_mut(
                    iter.events[iter.count..].as_mut_ptr() as *mut u8,
                    iter.events[iter.count..].len() * mem::size_of::<Event>(),
                )
            };
            match self.file_mut().read(bytes) {
                Ok(0) => {
                    if !self.window_async && iter.extra.is_none() && iter.count == 0 {
                        thread::yield_now();
                    } else {
                        break 'blocking;
                    }
                }
                Ok(count) => {
                    let count = count / mem::size_of::<Event>();
                    let events = &iter.events[iter.count..][..count];
                    iter.count += count;

                    if self.resizable {
                        let mut resize = None;
                        for event in events {
                            let event = *event;
                            if event.code == EVENT_RESIZE {
                                resize = Some((event.a as u32, event.b as u32));
                            }
                        }
                        if let Some((w, h)) = resize {
                            self.set_size(w, h);
                        }
                    }
                    if !self.window_async {
                        // Synchronous windows are blocking, can't attempt another read
                        break 'blocking;
                    }
                }
                Err(_) => break 'blocking,
            }
        }

        iter
    }

    fn file(&self) -> &File {
        self.file_opt.as_ref().unwrap()
    }

    fn file_mut(&mut self) -> &mut File {
        self.file_opt.as_mut().unwrap()
    }

    unsafe fn remap(&mut self) {
        self.unmap();

        let size = (self.w * self.h) as usize;
        let address = syscall::fmap(
            self.file().as_raw_fd() as usize,
            &syscall::Map {
                offset: 0,
                size: next_multiple_of(size * mem::size_of::<Color>(), syscall::PAGE_SIZE),
                flags: syscall::PROT_READ | syscall::PROT_WRITE | syscall::MAP_SHARED,
                address: 0,
            },
        )
        .expect("orbclient: failed to map memory");

        self.data_opt = Some(
            slice::from_raw_parts_mut(address as *mut Color, size)
        );
    }

    unsafe fn unmap(&mut self) {
        if let Some(data) = self.data_opt.take() {
            syscall::funmap(
                data.as_ptr() as usize,
                next_multiple_of(data.len() * mem::size_of::<Color>(), syscall::PAGE_SIZE),
            ).expect("orbclient: failed to unmap memory");
        }
    }

}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe { self.unmap(); }
    }
}

impl AsRawFd for Window {
    fn as_raw_fd(&self) -> RawFd {
        self.file().as_raw_fd()
    }
}

impl FromRawFd for Window {
    unsafe fn from_raw_fd(fd: RawFd) -> Window {
        let mut window = Window {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
            t: String::new(),
            window_async: false,
            resizable: false,
            mode: Cell::new(Mode::Blend),
            file_opt: Some(File::from_raw_fd(fd)),
            data_opt: None,
        };
        window.sync_path();
        window.remap();
        window
    }
}

impl IntoRawFd for Window {
    fn into_raw_fd(mut self) -> RawFd {
        self.file_opt.take().unwrap().into_raw_fd()
    }
}

/// Event iterator
pub struct EventIter {
    extra: Option<Vec<Event>>,
    events: [Event; 16],
    i: usize,
    count: usize,
}

impl Iterator for EventIter {
    type Item = Event;
    fn next(&mut self) -> Option<Event> {
        let mut i = self.i;
        if let Some(ref mut extra) = self.extra {
            if i < extra.len() {
                self.i += 1;
                return Some(extra[i]);
            }
            i -= extra.len();
        }
        if i < self.count {
            self.i += 1;
            return Some(self.events[i]);
        }
        None
    }
}

// TODO: Use libcore API once #![feature(int_roundings)] is stabilized.
fn next_multiple_of(value: usize, size: usize) -> usize {
    ((value + size - 1) / size) * size
}
