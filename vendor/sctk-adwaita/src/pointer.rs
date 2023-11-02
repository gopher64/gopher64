use log::error;
use smithay_client_toolkit::{
    reexports::{
        client::{
            protocol::{wl_pointer, wl_seat::WlSeat},
            DispatchData,
        },
        protocols::xdg_shell::client::xdg_toplevel::ResizeEdge,
    },
    seat::pointer::ThemedPointer,
    window::FrameRequest,
};

use crate::{
    buttons::{ButtonKind, Buttons},
    parts::DecorationPartKind,
    precise_location,
    theme::{BORDER_SIZE, HEADER_SIZE},
    Inner, Location,
};

pub(crate) struct PointerUserData {
    pub location: Location,
    current_surface: DecorationPartKind,

    position: (f64, f64),
    pub seat: WlSeat,
    last_click: Option<std::time::Instant>,

    lpm_grab: Option<ButtonKind>,
}

impl PointerUserData {
    pub fn new(seat: WlSeat) -> Self {
        Self {
            location: Location::None,
            current_surface: DecorationPartKind::None,
            position: (0.0, 0.0),
            seat,
            last_click: None,
            lpm_grab: None,
        }
    }

    pub fn event(
        &mut self,
        event: wl_pointer::Event,
        inner: &mut Inner,
        buttons: &Buttons,
        pointer: &ThemedPointer,
        ddata: DispatchData<'_>,
    ) {
        use wl_pointer::Event;
        match event {
            Event::Enter {
                serial,
                surface,
                surface_x,
                surface_y,
            } => {
                self.location = precise_location(
                    buttons,
                    inner.parts.find_surface(&surface),
                    inner.size.0,
                    surface_x,
                    surface_y,
                );
                self.current_surface = inner.parts.find_decoration_part(&surface);
                self.position = (surface_x, surface_y);
                change_pointer(pointer, inner, self.location, Some(serial))
            }
            Event::Leave { serial, .. } => {
                self.current_surface = DecorationPartKind::None;

                self.location = Location::None;
                change_pointer(pointer, inner, self.location, Some(serial));
                (inner.implem)(FrameRequest::Refresh, 0, ddata);
            }
            Event::Motion {
                surface_x,
                surface_y,
                ..
            } => {
                self.position = (surface_x, surface_y);
                let newpos =
                    precise_location(buttons, self.location, inner.size.0, surface_x, surface_y);
                if newpos != self.location {
                    match (newpos, self.location) {
                        (Location::Button(_), _) | (_, Location::Button(_)) => {
                            // pointer movement involves a button, request refresh
                            (inner.implem)(FrameRequest::Refresh, 0, ddata);
                        }
                        _ => (),
                    }
                    // we changed of part of the decoration, pointer image
                    // may need to be changed
                    self.location = newpos;
                    change_pointer(pointer, inner, self.location, None)
                }
            }
            Event::Button {
                serial,
                button,
                state,
                ..
            } => {
                let request = if state == wl_pointer::ButtonState::Pressed {
                    match button {
                        // Left mouse button.
                        0x110 => lmb_press(self, inner.maximized, inner.resizable),
                        // Right mouse button.
                        0x111 => rmb_press(self),
                        _ => None,
                    }
                } else {
                    // Left mouse button.
                    if button == 0x110 {
                        lmb_release(self, inner.maximized)
                    } else {
                        None
                    }
                };

                if let Some(request) = request {
                    (inner.implem)(request, serial, ddata);
                }
            }
            _ => {}
        }
    }
}

fn lmb_press(
    pointer_data: &mut PointerUserData,
    maximized: bool,
    resizable: bool,
) -> Option<FrameRequest> {
    match pointer_data.location {
        Location::Top if resizable => Some(FrameRequest::Resize(
            pointer_data.seat.clone(),
            ResizeEdge::Top,
        )),
        Location::TopLeft if resizable => Some(FrameRequest::Resize(
            pointer_data.seat.clone(),
            ResizeEdge::TopLeft,
        )),
        Location::Left if resizable => Some(FrameRequest::Resize(
            pointer_data.seat.clone(),
            ResizeEdge::Left,
        )),
        Location::BottomLeft if resizable => Some(FrameRequest::Resize(
            pointer_data.seat.clone(),
            ResizeEdge::BottomLeft,
        )),
        Location::Bottom if resizable => Some(FrameRequest::Resize(
            pointer_data.seat.clone(),
            ResizeEdge::Bottom,
        )),
        Location::BottomRight if resizable => Some(FrameRequest::Resize(
            pointer_data.seat.clone(),
            ResizeEdge::BottomRight,
        )),
        Location::Right if resizable => Some(FrameRequest::Resize(
            pointer_data.seat.clone(),
            ResizeEdge::Right,
        )),
        Location::TopRight if resizable => Some(FrameRequest::Resize(
            pointer_data.seat.clone(),
            ResizeEdge::TopRight,
        )),
        Location::Head => {
            let last_click = pointer_data.last_click.replace(std::time::Instant::now());

            if let Some(last) = last_click {
                if last.elapsed() < std::time::Duration::from_millis(400) {
                    pointer_data.last_click = None;

                    if maximized {
                        Some(FrameRequest::UnMaximize)
                    } else {
                        Some(FrameRequest::Maximize)
                    }
                } else {
                    Some(FrameRequest::Move(pointer_data.seat.clone()))
                }
            } else {
                Some(FrameRequest::Move(pointer_data.seat.clone()))
            }
        }
        Location::Button(btn) => {
            pointer_data.lpm_grab = Some(btn);
            None
        }
        _ => None,
    }
}

fn lmb_release(pointer_data: &mut PointerUserData, maximized: bool) -> Option<FrameRequest> {
    let lpm_grab = pointer_data.lpm_grab.take();

    match pointer_data.location {
        Location::Button(btn) => {
            if lpm_grab == Some(btn) {
                let req = match btn {
                    ButtonKind::Close => FrameRequest::Close,
                    ButtonKind::Maximize => {
                        if maximized {
                            FrameRequest::UnMaximize
                        } else {
                            FrameRequest::Maximize
                        }
                    }
                    ButtonKind::Minimize => FrameRequest::Minimize,
                };

                Some(req)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn rmb_press(pointer_data: &PointerUserData) -> Option<FrameRequest> {
    match pointer_data.location {
        Location::Head | Location::Button(_) => Some(FrameRequest::ShowMenu(
            pointer_data.seat.clone(),
            pointer_data.position.0 as i32 - BORDER_SIZE as i32,
            // We must offset it by header size for precise position.
            pointer_data.position.1 as i32 - (HEADER_SIZE as i32 + BORDER_SIZE as i32),
        )),
        _ => None,
    }
}

fn change_pointer(pointer: &ThemedPointer, inner: &Inner, location: Location, serial: Option<u32>) {
    // Prevent theming of the surface if it was requested.
    if !inner.theme_over_surface && location == Location::None {
        return;
    }

    let name = match location {
        // If we can't resize a frame we shouldn't show resize cursors.
        _ if !inner.resizable => "left_ptr",
        Location::Top => "top_side",
        Location::TopRight => "top_right_corner",
        Location::Right => "right_side",
        Location::BottomRight => "bottom_right_corner",
        Location::Bottom => "bottom_side",
        Location::BottomLeft => "bottom_left_corner",
        Location::Left => "left_side",
        Location::TopLeft => "top_left_corner",
        _ => "left_ptr",
    };

    if pointer.set_cursor(name, serial).is_err() {
        error!("Failed to set cursor");
    }
}
