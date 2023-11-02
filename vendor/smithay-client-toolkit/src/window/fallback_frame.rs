use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use wayland_client::protocol::{
    wl_compositor, wl_pointer, wl_seat, wl_shm, wl_subcompositor, wl_subsurface, wl_surface,
};
use wayland_client::{Attached, DispatchData};

use log::error;

use super::{ButtonState, Frame, FrameRequest, State, WindowState};
use crate::seat::pointer::{ThemeManager, ThemeSpec, ThemedPointer};
use crate::shm::AutoMemPool;

/*
 * Drawing theme definitions
 */

const BORDER_SIZE: u32 = 4;
const HEADER_SIZE: u32 = 24;

const BTN_ICON_COLOR: u32 = 0xFF1E1E1E;
const BTN_HOVER_BG: u32 = 0xFFA8A8A8;

const PRIMARY_COLOR_ACTIVE: u32 = 0xFFE6E6E6;
const PRIMARY_COLOR_INACTIVE: u32 = 0xFFDCDCDC;

/*
 * Utilities
 */

const HEAD: usize = 0;
const TOP: usize = 1;
const BOTTOM: usize = 2;
const LEFT: usize = 3;
const RIGHT: usize = 4;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Location {
    None,
    Head,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
    TopLeft,
    Button(UIButton),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum UIButton {
    Minimize,
    Maximize,
    Close,
}

#[derive(Debug)]
struct Part {
    surface: wl_surface::WlSurface,
    subsurface: wl_subsurface::WlSubsurface,
}

impl Part {
    fn new(
        parent: &wl_surface::WlSurface,
        compositor: &Attached<wl_compositor::WlCompositor>,
        subcompositor: &Attached<wl_subcompositor::WlSubcompositor>,
        inner: Option<Rc<RefCell<Inner>>>,
    ) -> Part {
        let surface = if let Some(inner) = inner {
            crate::surface::setup_surface(
                compositor.create_surface(),
                Some(move |dpi, surface: wl_surface::WlSurface, ddata: DispatchData| {
                    surface.set_buffer_scale(dpi);
                    surface.commit();
                    (inner.borrow_mut().implem)(FrameRequest::Refresh, 0, ddata);
                }),
            )
        } else {
            crate::surface::setup_surface(
                compositor.create_surface(),
                Some(move |dpi, surface: wl_surface::WlSurface, _ddata: DispatchData| {
                    surface.set_buffer_scale(dpi);
                    surface.commit();
                }),
            )
        };

        let surface = surface.detach();

        let subsurface = subcompositor.get_subsurface(&surface, parent);

        Part { surface, subsurface: subsurface.detach() }
    }
}

impl Drop for Part {
    fn drop(&mut self) {
        self.subsurface.destroy();
        self.surface.destroy();
    }
}

struct PointerUserData {
    location: Location,
    position: (f64, f64),
    seat: wl_seat::WlSeat,
}

/*
 * The core frame
 */

struct Inner {
    parts: Vec<Part>,
    size: (u32, u32),
    resizable: bool,
    theme_over_surface: bool,
    implem: Box<dyn FnMut(FrameRequest, u32, DispatchData)>,
    maximized: bool,
    fullscreened: bool,
}

impl Inner {
    fn find_surface(&self, surface: &wl_surface::WlSurface) -> Location {
        if self.parts.is_empty() {
            return Location::None;
        }

        if surface.as_ref().equals(self.parts[HEAD].surface.as_ref()) {
            Location::Head
        } else if surface.as_ref().equals(self.parts[TOP].surface.as_ref()) {
            Location::Top
        } else if surface.as_ref().equals(self.parts[BOTTOM].surface.as_ref()) {
            Location::Bottom
        } else if surface.as_ref().equals(self.parts[LEFT].surface.as_ref()) {
            Location::Left
        } else if surface.as_ref().equals(self.parts[RIGHT].surface.as_ref()) {
            Location::Right
        } else {
            Location::None
        }
    }
}

impl fmt::Debug for Inner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Inner")
            .field("parts", &self.parts)
            .field("size", &self.size)
            .field("resizable", &self.resizable)
            .field("theme_over_surface", &self.theme_over_surface)
            .field("implem", &"FnMut(FrameRequest, u32, DispatchData) -> { ... }")
            .field("maximized", &self.maximized)
            .field("fullscreened", &self.fullscreened)
            .finish()
    }
}

fn precise_location(old: Location, width: u32, x: f64, y: f64) -> Location {
    match old {
        Location::Head | Location::Button(_) => find_button(x, y, width),

        Location::Top | Location::TopLeft | Location::TopRight => {
            if x <= f64::from(BORDER_SIZE) {
                Location::TopLeft
            } else if x >= f64::from(width + BORDER_SIZE) {
                Location::TopRight
            } else {
                Location::Top
            }
        }

        Location::Bottom | Location::BottomLeft | Location::BottomRight => {
            if x <= f64::from(BORDER_SIZE) {
                Location::BottomLeft
            } else if x >= f64::from(width + BORDER_SIZE) {
                Location::BottomRight
            } else {
                Location::Bottom
            }
        }

        other => other,
    }
}

fn find_button(x: f64, y: f64, w: u32) -> Location {
    if (w >= HEADER_SIZE)
        && (x >= f64::from(w - HEADER_SIZE))
        && (x <= f64::from(w))
        && (y <= f64::from(HEADER_SIZE))
        && (y >= f64::from(0))
    {
        // first button
        Location::Button(UIButton::Close)
    } else if (w >= 2 * HEADER_SIZE)
        && (x >= f64::from(w - 2 * HEADER_SIZE))
        && (x <= f64::from(w - HEADER_SIZE))
        && (y <= f64::from(HEADER_SIZE))
        && (y >= f64::from(0))
    {
        // second button
        Location::Button(UIButton::Maximize)
    } else if (w >= 3 * HEADER_SIZE)
        && (x >= f64::from(w - 3 * HEADER_SIZE))
        && (x <= f64::from(w - 2 * HEADER_SIZE))
        && (y <= f64::from(HEADER_SIZE))
        && (y >= f64::from(0))
    {
        // third button
        Location::Button(UIButton::Minimize)
    } else {
        Location::Head
    }
}

/// A simple set of decorations that can be used as a fallback
///
/// This class drawn some simple and minimalistic decorations around
/// a window so that it remains possible to interact with the window
/// even when server-side decorations are not available.
///
/// `FallbackFrame` is hiding its `ClientSide` decorations
/// in a `Fullscreen` state and brings them back if those are
/// visible when unsetting `Fullscreen` state.
#[derive(Debug)]
pub struct FallbackFrame {
    base_surface: wl_surface::WlSurface,
    compositor: Attached<wl_compositor::WlCompositor>,
    subcompositor: Attached<wl_subcompositor::WlSubcompositor>,
    inner: Rc<RefCell<Inner>>,
    pool: AutoMemPool,
    active: WindowState,
    hidden: bool,
    pointers: Vec<ThemedPointer>,
    themer: ThemeManager,
    surface_version: u32,
}

impl Frame for FallbackFrame {
    type Error = ::std::io::Error;
    type Config = ();
    fn init(
        base_surface: &wl_surface::WlSurface,
        compositor: &Attached<wl_compositor::WlCompositor>,
        subcompositor: &Attached<wl_subcompositor::WlSubcompositor>,
        shm: &Attached<wl_shm::WlShm>,
        theme_manager: Option<ThemeManager>,
        implementation: Box<dyn FnMut(FrameRequest, u32, DispatchData)>,
    ) -> Result<FallbackFrame, ::std::io::Error> {
        let (themer, theme_over_surface) = if let Some(theme_manager) = theme_manager {
            (theme_manager, false)
        } else {
            (ThemeManager::init(ThemeSpec::System, compositor.clone(), shm.clone()), true)
        };

        let inner = Rc::new(RefCell::new(Inner {
            parts: vec![],
            size: (1, 1),
            resizable: true,
            implem: implementation,
            theme_over_surface,
            maximized: false,
            fullscreened: false,
        }));

        let pool = AutoMemPool::new(shm.clone())?;

        Ok(FallbackFrame {
            base_surface: base_surface.clone(),
            compositor: compositor.clone(),
            subcompositor: subcompositor.clone(),
            inner,
            pool,
            active: WindowState::Inactive,
            hidden: true,
            pointers: Vec::new(),
            themer,
            surface_version: compositor.as_ref().version(),
        })
    }

    fn new_seat(&mut self, seat: &Attached<wl_seat::WlSeat>) {
        use self::wl_pointer::Event;
        let inner = self.inner.clone();
        let pointer = self.themer.theme_pointer_with_impl(
            seat,
            move |event, pointer: ThemedPointer, ddata: DispatchData| {
                let data: &RefCell<PointerUserData> = pointer.as_ref().user_data().get().unwrap();
                let mut data = data.borrow_mut();
                let mut inner = inner.borrow_mut();
                match event {
                    Event::Enter { serial, surface, surface_x, surface_y } => {
                        data.location = precise_location(
                            inner.find_surface(&surface),
                            inner.size.0,
                            surface_x,
                            surface_y,
                        );
                        data.position = (surface_x, surface_y);
                        change_pointer(&pointer, &inner, data.location, Some(serial))
                    }
                    Event::Leave { serial, .. } => {
                        data.location = Location::None;
                        change_pointer(&pointer, &inner, data.location, Some(serial));
                        (inner.implem)(FrameRequest::Refresh, 0, ddata);
                    }
                    Event::Motion { surface_x, surface_y, .. } => {
                        data.position = (surface_x, surface_y);
                        let newpos =
                            precise_location(data.location, inner.size.0, surface_x, surface_y);
                        if newpos != data.location {
                            match (newpos, data.location) {
                                (Location::Button(_), _) | (_, Location::Button(_)) => {
                                    // pointer movement involves a button, request refresh
                                    (inner.implem)(FrameRequest::Refresh, 0, ddata);
                                }
                                _ => (),
                            }
                            // we changed of part of the decoration, pointer image
                            // may need to be changed
                            data.location = newpos;
                            change_pointer(&pointer, &inner, data.location, None)
                        }
                    }
                    Event::Button { serial, button, state, .. } => {
                        if state == wl_pointer::ButtonState::Pressed {
                            let request = match button {
                                // Left mouse button.
                                0x110 => request_for_location_on_lmb(
                                    &data,
                                    inner.maximized,
                                    inner.resizable,
                                ),
                                // Right mouse button.
                                0x111 => request_for_location_on_rmb(&data),
                                _ => None,
                            };

                            if let Some(request) = request {
                                (inner.implem)(request, serial, ddata);
                            }
                        }
                    }
                    _ => {}
                }
            },
        );
        pointer.as_ref().user_data().set(|| {
            RefCell::new(PointerUserData {
                location: Location::None,
                position: (0.0, 0.0),
                seat: seat.detach(),
            })
        });
        self.pointers.push(pointer);
    }

    fn remove_seat(&mut self, seat: &wl_seat::WlSeat) {
        self.pointers.retain(|pointer| {
            let user_data = pointer.as_ref().user_data().get::<RefCell<PointerUserData>>().unwrap();
            let guard = user_data.borrow_mut();
            if &guard.seat == seat {
                pointer.release();
                false
            } else {
                true
            }
        });
    }

    fn set_states(&mut self, states: &[State]) -> bool {
        let mut inner = self.inner.borrow_mut();
        let mut need_redraw = false;

        // Process active.
        let new_active = if states.contains(&State::Activated) {
            WindowState::Active
        } else {
            WindowState::Inactive
        };
        need_redraw |= new_active != self.active;
        self.active = new_active;

        // Process maximized.
        let new_maximized = states.contains(&State::Maximized);
        need_redraw |= new_maximized != inner.maximized;
        inner.maximized = new_maximized;

        // Process fullscreened.
        let new_fullscreened = states.contains(&State::Fullscreen);
        need_redraw |= new_fullscreened != inner.fullscreened;
        inner.fullscreened = new_fullscreened;

        need_redraw
    }

    fn set_hidden(&mut self, hidden: bool) {
        self.hidden = hidden;
        let mut inner = self.inner.borrow_mut();
        if !self.hidden {
            if inner.parts.is_empty() {
                inner.parts = vec![
                    Part::new(
                        &self.base_surface,
                        &self.compositor,
                        &self.subcompositor,
                        Some(Rc::clone(&self.inner)),
                    ),
                    Part::new(&self.base_surface, &self.compositor, &self.subcompositor, None),
                    Part::new(&self.base_surface, &self.compositor, &self.subcompositor, None),
                    Part::new(&self.base_surface, &self.compositor, &self.subcompositor, None),
                    Part::new(&self.base_surface, &self.compositor, &self.subcompositor, None),
                ];
            }
        } else {
            inner.parts.clear();
        }
    }

    fn set_resizable(&mut self, resizable: bool) {
        self.inner.borrow_mut().resizable = resizable;
    }

    fn resize(&mut self, newsize: (u32, u32)) {
        self.inner.borrow_mut().size = newsize;
    }

    fn redraw(&mut self) {
        let inner = self.inner.borrow_mut();

        // Don't draw borders if the frame explicitly hidden or fullscreened.
        if self.hidden || inner.fullscreened {
            // Don't draw the borders.
            for p in inner.parts.iter() {
                p.surface.attach(None, 0, 0);
                p.surface.commit();
            }
            return;
        }

        // `parts` can't be empty here, since the initial state for `self.hidden` is true, and
        // they will be created once `self.hidden` will become `false`.
        let parts = &inner.parts;

        let scales: Vec<u32> = parts
            .iter()
            .map(|part| crate::surface::get_surface_scale_factor(&part.surface) as u32)
            .collect();

        let (width, height) = inner.size;

        // Use header scale for all the thing.
        let header_scale = scales[HEAD];

        let scaled_header_height = HEADER_SIZE * header_scale;
        let scaled_header_width = width * header_scale;

        {
            // Create the buffers and draw
            let color = if self.active == WindowState::Active {
                PRIMARY_COLOR_ACTIVE.to_ne_bytes()
            } else {
                PRIMARY_COLOR_INACTIVE.to_ne_bytes()
            };

            // -> head-subsurface
            if let Ok((canvas, buffer)) = self.pool.buffer(
                scaled_header_width as i32,
                scaled_header_height as i32,
                4 * scaled_header_width as i32,
                wl_shm::Format::Argb8888,
            ) {
                for pixel in canvas.chunks_exact_mut(4) {
                    pixel[0] = color[0];
                    pixel[1] = color[1];
                    pixel[2] = color[2];
                    pixel[3] = color[3];
                }

                draw_buttons(
                    canvas,
                    width,
                    header_scale,
                    inner.resizable,
                    self.active,
                    &self
                        .pointers
                        .iter()
                        .flat_map(|p| {
                            if p.as_ref().is_alive() {
                                let data: &RefCell<PointerUserData> =
                                    p.as_ref().user_data().get().unwrap();
                                Some(data.borrow().location)
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<Location>>(),
                );

                parts[HEAD].subsurface.set_position(0, -(HEADER_SIZE as i32));
                parts[HEAD].surface.attach(Some(&buffer), 0, 0);
                if self.surface_version >= 4 {
                    parts[HEAD].surface.damage_buffer(
                        0,
                        0,
                        scaled_header_width as i32,
                        scaled_header_height as i32,
                    );
                } else {
                    // surface is old and does not support damage_buffer, so we damage
                    // in surface coordinates and hope it is not rescaled
                    parts[HEAD].surface.damage(0, 0, width as i32, HEADER_SIZE as i32);
                }
                parts[HEAD].surface.commit();
            }

            // -> top-subsurface
            if let Ok((canvas, buffer)) = self.pool.buffer(
                ((width + 2 * BORDER_SIZE) * scales[TOP]) as i32,
                (BORDER_SIZE * scales[TOP]) as i32,
                (4 * scales[TOP] * (width + 2 * BORDER_SIZE)) as i32,
                wl_shm::Format::Argb8888,
            ) {
                for pixel in canvas.chunks_exact_mut(4) {
                    pixel[0] = color[0];
                    pixel[1] = color[1];
                    pixel[2] = color[2];
                    pixel[3] = color[3];
                }
                parts[TOP].subsurface.set_position(
                    -(BORDER_SIZE as i32),
                    -(HEADER_SIZE as i32 + BORDER_SIZE as i32),
                );
                parts[TOP].surface.attach(Some(&buffer), 0, 0);
                if self.surface_version >= 4 {
                    parts[TOP].surface.damage_buffer(
                        0,
                        0,
                        ((width + 2 * BORDER_SIZE) * scales[TOP]) as i32,
                        (BORDER_SIZE * scales[TOP]) as i32,
                    );
                } else {
                    // surface is old and does not support damage_buffer, so we damage
                    // in surface coordinates and hope it is not rescaled
                    parts[TOP].surface.damage(
                        0,
                        0,
                        (width + 2 * BORDER_SIZE) as i32,
                        BORDER_SIZE as i32,
                    );
                }
                parts[TOP].surface.commit();
            }

            // -> bottom-subsurface
            if let Ok((canvas, buffer)) = self.pool.buffer(
                ((width + 2 * BORDER_SIZE) * scales[BOTTOM]) as i32,
                (BORDER_SIZE * scales[BOTTOM]) as i32,
                (4 * scales[BOTTOM] * (width + 2 * BORDER_SIZE)) as i32,
                wl_shm::Format::Argb8888,
            ) {
                for pixel in canvas.chunks_exact_mut(4) {
                    pixel[0] = color[0];
                    pixel[1] = color[1];
                    pixel[2] = color[2];
                    pixel[3] = color[3];
                }
                parts[BOTTOM].subsurface.set_position(-(BORDER_SIZE as i32), height as i32);
                parts[BOTTOM].surface.attach(Some(&buffer), 0, 0);
                if self.surface_version >= 4 {
                    parts[BOTTOM].surface.damage_buffer(
                        0,
                        0,
                        ((width + 2 * BORDER_SIZE) * scales[BOTTOM]) as i32,
                        (BORDER_SIZE * scales[BOTTOM]) as i32,
                    );
                } else {
                    // surface is old and does not support damage_buffer, so we damage
                    // in surface coordinates and hope it is not rescaled
                    parts[BOTTOM].surface.damage(
                        0,
                        0,
                        (width + 2 * BORDER_SIZE) as i32,
                        BORDER_SIZE as i32,
                    );
                }
                parts[BOTTOM].surface.commit();
            }

            // -> left-subsurface
            if let Ok((canvas, buffer)) = self.pool.buffer(
                (BORDER_SIZE * scales[LEFT]) as i32,
                ((height + HEADER_SIZE) * scales[LEFT]) as i32,
                4 * (BORDER_SIZE * scales[LEFT]) as i32,
                wl_shm::Format::Argb8888,
            ) {
                for pixel in canvas.chunks_exact_mut(4) {
                    pixel[0] = color[0];
                    pixel[1] = color[1];
                    pixel[2] = color[2];
                    pixel[3] = color[3];
                }
                parts[LEFT].subsurface.set_position(-(BORDER_SIZE as i32), -(HEADER_SIZE as i32));
                parts[LEFT].surface.attach(Some(&buffer), 0, 0);
                if self.surface_version >= 4 {
                    parts[LEFT].surface.damage_buffer(
                        0,
                        0,
                        (BORDER_SIZE * scales[LEFT]) as i32,
                        ((height + HEADER_SIZE) * scales[LEFT]) as i32,
                    );
                } else {
                    // surface is old and does not support damage_buffer, so we damage
                    // in surface coordinates and hope it is not rescaled
                    parts[LEFT].surface.damage(
                        0,
                        0,
                        BORDER_SIZE as i32,
                        (height + HEADER_SIZE) as i32,
                    );
                }
                parts[LEFT].surface.commit();
            }

            // -> right-subsurface
            if let Ok((canvas, buffer)) = self.pool.buffer(
                (BORDER_SIZE * scales[RIGHT]) as i32,
                ((height + HEADER_SIZE) * scales[RIGHT]) as i32,
                4 * (BORDER_SIZE * scales[RIGHT]) as i32,
                wl_shm::Format::Argb8888,
            ) {
                for pixel in canvas.chunks_exact_mut(4) {
                    pixel[0] = color[0];
                    pixel[1] = color[1];
                    pixel[2] = color[2];
                    pixel[3] = color[3];
                }
                parts[RIGHT].subsurface.set_position(width as i32, -(HEADER_SIZE as i32));
                parts[RIGHT].surface.attach(Some(&buffer), 0, 0);
                if self.surface_version >= 4 {
                    parts[RIGHT].surface.damage_buffer(
                        0,
                        0,
                        (BORDER_SIZE * scales[RIGHT]) as i32,
                        ((height + HEADER_SIZE) * scales[RIGHT]) as i32,
                    );
                } else {
                    // surface is old and does not support damage_buffer, so we damage
                    // in surface coordinates and hope it is not rescaled
                    parts[RIGHT].surface.damage(
                        0,
                        0,
                        BORDER_SIZE as i32,
                        (height + HEADER_SIZE) as i32,
                    );
                }
                parts[RIGHT].surface.commit();
            }
        }
    }

    fn subtract_borders(&self, width: i32, height: i32) -> (i32, i32) {
        if self.hidden || self.inner.borrow().fullscreened {
            (width, height)
        } else {
            (width - 2 * BORDER_SIZE as i32, height - HEADER_SIZE as i32 - 2 * BORDER_SIZE as i32)
        }
    }

    fn add_borders(&self, width: i32, height: i32) -> (i32, i32) {
        if self.hidden || self.inner.borrow().fullscreened {
            (width, height)
        } else {
            (width + 2 * BORDER_SIZE as i32, height + HEADER_SIZE as i32 + 2 * BORDER_SIZE as i32)
        }
    }

    fn location(&self) -> (i32, i32) {
        if self.hidden || self.inner.borrow().fullscreened {
            (0, 0)
        } else {
            (-(BORDER_SIZE as i32), -(HEADER_SIZE as i32 + BORDER_SIZE as i32))
        }
    }

    fn set_config(&mut self, _config: ()) {}

    fn set_title(&mut self, _title: String) {}
}

impl Drop for FallbackFrame {
    fn drop(&mut self) {
        for ptr in self.pointers.drain(..) {
            if ptr.as_ref().version() >= 3 {
                ptr.release();
            }
        }
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

fn request_for_location_on_lmb(
    pointer_data: &PointerUserData,
    maximized: bool,
    resizable: bool,
) -> Option<FrameRequest> {
    use wayland_protocols::xdg_shell::client::xdg_toplevel::ResizeEdge;
    match pointer_data.location {
        Location::Top if resizable => {
            Some(FrameRequest::Resize(pointer_data.seat.clone(), ResizeEdge::Top))
        }
        Location::TopLeft if resizable => {
            Some(FrameRequest::Resize(pointer_data.seat.clone(), ResizeEdge::TopLeft))
        }
        Location::Left if resizable => {
            Some(FrameRequest::Resize(pointer_data.seat.clone(), ResizeEdge::Left))
        }
        Location::BottomLeft if resizable => {
            Some(FrameRequest::Resize(pointer_data.seat.clone(), ResizeEdge::BottomLeft))
        }
        Location::Bottom if resizable => {
            Some(FrameRequest::Resize(pointer_data.seat.clone(), ResizeEdge::Bottom))
        }
        Location::BottomRight if resizable => {
            Some(FrameRequest::Resize(pointer_data.seat.clone(), ResizeEdge::BottomRight))
        }
        Location::Right if resizable => {
            Some(FrameRequest::Resize(pointer_data.seat.clone(), ResizeEdge::Right))
        }
        Location::TopRight if resizable => {
            Some(FrameRequest::Resize(pointer_data.seat.clone(), ResizeEdge::TopRight))
        }
        Location::Head => Some(FrameRequest::Move(pointer_data.seat.clone())),
        Location::Button(UIButton::Close) => Some(FrameRequest::Close),
        Location::Button(UIButton::Maximize) => {
            if maximized {
                Some(FrameRequest::UnMaximize)
            } else {
                Some(FrameRequest::Maximize)
            }
        }
        Location::Button(UIButton::Minimize) => Some(FrameRequest::Minimize),
        _ => None,
    }
}

fn request_for_location_on_rmb(pointer_data: &PointerUserData) -> Option<FrameRequest> {
    match pointer_data.location {
        Location::Head | Location::Button(_) => Some(FrameRequest::ShowMenu(
            pointer_data.seat.clone(),
            pointer_data.position.0 as i32,
            // We must offset it by header size for precise position.
            pointer_data.position.1 as i32 - HEADER_SIZE as i32,
        )),
        _ => None,
    }
}

fn draw_buttons(
    canvas: &mut [u8],
    width: u32,
    scale: u32,
    maximizable: bool,
    state: WindowState,
    mouses: &[Location],
) {
    let scale = scale as usize;

    if width >= HEADER_SIZE {
        // Draw the close button
        let btn_state = if mouses.iter().any(|&l| l == Location::Button(UIButton::Close)) {
            ButtonState::Hovered
        } else {
            ButtonState::Idle
        };

        if state == WindowState::Active && btn_state == ButtonState::Hovered {
            draw_button(canvas, 0, scale, width as usize, BTN_HOVER_BG.to_ne_bytes());
        }
        draw_icon(canvas, width as usize, 0, scale, BTN_ICON_COLOR.to_ne_bytes(), Icon::Close);
    }

    if width as usize >= 2 * HEADER_SIZE as usize {
        let btn_state = if !maximizable {
            ButtonState::Disabled
        } else if mouses.iter().any(|&l| l == Location::Button(UIButton::Maximize)) {
            ButtonState::Hovered
        } else {
            ButtonState::Idle
        };

        if state == WindowState::Active && btn_state == ButtonState::Hovered {
            draw_button(
                canvas,
                HEADER_SIZE as usize,
                scale,
                width as usize,
                BTN_HOVER_BG.to_ne_bytes(),
            );
        }
        draw_icon(
            canvas,
            width as usize,
            HEADER_SIZE as usize,
            scale,
            BTN_ICON_COLOR.to_ne_bytes(),
            Icon::Maximize,
        );
    }

    if width as usize >= 3 * HEADER_SIZE as usize {
        let btn_state = if mouses.iter().any(|&l| l == Location::Button(UIButton::Minimize)) {
            ButtonState::Hovered
        } else {
            ButtonState::Idle
        };

        if state == WindowState::Active && btn_state == ButtonState::Hovered {
            draw_button(
                canvas,
                2 * HEADER_SIZE as usize,
                scale,
                width as usize,
                BTN_HOVER_BG.to_ne_bytes(),
            );
        }
        draw_icon(
            canvas,
            width as usize,
            2 * HEADER_SIZE as usize,
            scale,
            BTN_ICON_COLOR.to_ne_bytes(),
            Icon::Minimize,
        );
    }
}

enum Icon {
    Close,
    Maximize,
    Minimize,
}

fn draw_button(canvas: &mut [u8], x_offset: usize, scale: usize, width: usize, btn_color: [u8; 4]) {
    let h = HEADER_SIZE as usize;
    let x_start = width - h - x_offset;
    // main square
    for y in 0..h * scale {
        let canvas =
            &mut canvas[(x_start + y * width) * 4 * scale..(x_start + y * width + h) * scale * 4];
        for pixel in canvas.chunks_exact_mut(4) {
            pixel[0] = btn_color[0];
            pixel[1] = btn_color[1];
            pixel[2] = btn_color[2];
            pixel[3] = btn_color[3];
        }
    }
}

fn draw_icon(
    canvas: &mut [u8],
    width: usize,
    x_offset: usize,
    scale: usize,
    icon_color: [u8; 4],
    icon: Icon,
) {
    let h = HEADER_SIZE as usize;
    let sh = scale * h;
    let x_start = width - h - x_offset;

    match icon {
        Icon::Close => {
            // Draw black rectangle
            for y in sh / 4..3 * sh / 4 {
                let line = &mut canvas[(x_start + y * width + h / 4) * 4 * scale
                    ..(x_start + y * width + 3 * h / 4) * 4 * scale];
                for pixel in line.chunks_exact_mut(4) {
                    pixel[0] = icon_color[0];
                    pixel[1] = icon_color[1];
                    pixel[2] = icon_color[2];
                    pixel[3] = icon_color[3];
                }
            }
        }
        Icon::Maximize => {
            // Draw an empty rectangle
            for y in 2 * sh / 8..3 * sh / 8 {
                let line = &mut canvas[(x_start + y * width + h / 4) * 4 * scale
                    ..(x_start + y * width + 3 * h / 4) * 4 * scale];
                for pixel in line.chunks_exact_mut(4) {
                    pixel[0] = icon_color[0];
                    pixel[1] = icon_color[1];
                    pixel[2] = icon_color[2];
                    pixel[3] = icon_color[3];
                }
            }
            for y in 3 * sh / 8..5 * sh / 8 {
                let line = &mut canvas[(x_start + y * width + 2 * h / 8) * 4 * scale
                    ..(x_start + y * width + 3 * h / 8) * 4 * scale];
                for pixel in line.chunks_exact_mut(4) {
                    pixel[0] = icon_color[0];
                    pixel[1] = icon_color[1];
                    pixel[2] = icon_color[2];
                    pixel[3] = icon_color[3];
                }
                let line = &mut canvas[(x_start + y * width + 5 * h / 8) * 4 * scale
                    ..(x_start + y * width + 6 * h / 8) * 4 * scale];
                for pixel in line.chunks_exact_mut(4) {
                    pixel[0] = icon_color[0];
                    pixel[1] = icon_color[1];
                    pixel[2] = icon_color[2];
                    pixel[3] = icon_color[3];
                }
            }
            for y in 5 * sh / 8..6 * sh / 8 {
                let line = &mut canvas[(x_start + y * width + h / 4) * 4 * scale
                    ..(x_start + y * width + 3 * h / 4) * 4 * scale];
                for pixel in line.chunks_exact_mut(4) {
                    pixel[0] = icon_color[0];
                    pixel[1] = icon_color[1];
                    pixel[2] = icon_color[2];
                    pixel[3] = icon_color[3];
                }
            }
        }
        Icon::Minimize => {
            // Draw an underline
            for y in 5 * sh / 8..3 * sh / 4 {
                let line = &mut canvas[(x_start + y * width + h / 4) * 4 * scale
                    ..(x_start + y * width + 3 * h / 4) * 4 * scale];
                for pixel in line.chunks_exact_mut(4) {
                    pixel[0] = icon_color[0];
                    pixel[1] = icon_color[1];
                    pixel[2] = icon_color[2];
                    pixel[3] = icon_color[3];
                }
            }
        }
    }
}
