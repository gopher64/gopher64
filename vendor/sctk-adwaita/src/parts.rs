use std::{cell::RefCell, rc::Rc};

use smithay_client_toolkit::{
    reexports::client::{
        protocol::{
            wl_compositor::WlCompositor, wl_subcompositor::WlSubcompositor,
            wl_subsurface::WlSubsurface, wl_surface::WlSurface,
        },
        Attached, DispatchData,
    },
    window::FrameRequest,
};

use crate::{surface, Inner, Location};

pub enum DecorationPartKind {
    Header,
    Top,
    Left,
    Right,
    Bottom,
    None,
}

#[derive(Debug)]
pub struct Decoration {
    pub header: Part,

    pub top: Part,
    pub left: Part,
    pub right: Part,
    pub bottom: Part,
}

impl Decoration {
    pub fn iter(&self) -> [&Part; 5] {
        [
            &self.header,
            &self.top,
            &self.left,
            &self.right,
            &self.bottom,
        ]
    }

    pub fn hide_decoration(&self) {
        for p in self.iter() {
            p.surface.attach(None, 0, 0);
            p.surface.commit();
        }
    }

    pub fn hide_borders(&self) {
        for p in self.iter().iter().skip(1) {
            p.surface.attach(None, 0, 0);
            p.surface.commit();
        }
    }
}

#[derive(Default, Debug)]
pub(crate) struct Parts {
    decoration: Option<Decoration>,
}

impl Parts {
    pub fn add_decorations(
        &mut self,
        parent: &WlSurface,
        compositor: &Attached<WlCompositor>,
        subcompositor: &Attached<WlSubcompositor>,
        inner: Rc<RefCell<Inner>>,
    ) {
        if self.decoration.is_none() {
            let header = Part::new(parent, compositor, subcompositor, Some(inner));
            let top = Part::new(parent, compositor, subcompositor, None);
            let left = Part::new(parent, compositor, subcompositor, None);
            let right = Part::new(parent, compositor, subcompositor, None);
            let bottom = Part::new(parent, compositor, subcompositor, None);

            self.decoration = Some(Decoration {
                header,
                top,
                left,
                right,
                bottom,
            });
        }
    }

    pub fn remove_decorations(&mut self) {
        self.decoration = None;
    }

    pub fn hide_decorations(&self) {
        if let Some(decor) = self.decoration.as_ref() {
            decor.hide_decoration();
        }
    }

    pub fn decoration(&self) -> Option<&Decoration> {
        self.decoration.as_ref()
    }

    pub fn find_decoration_part(&self, surface: &WlSurface) -> DecorationPartKind {
        if let Some(decor) = self.decoration() {
            if surface.as_ref().equals(decor.header.surface.as_ref()) {
                DecorationPartKind::Header
            } else if surface.as_ref().equals(decor.top.surface.as_ref()) {
                DecorationPartKind::Top
            } else if surface.as_ref().equals(decor.bottom.surface.as_ref()) {
                DecorationPartKind::Bottom
            } else if surface.as_ref().equals(decor.left.surface.as_ref()) {
                DecorationPartKind::Left
            } else if surface.as_ref().equals(decor.right.surface.as_ref()) {
                DecorationPartKind::Right
            } else {
                DecorationPartKind::None
            }
        } else {
            DecorationPartKind::None
        }
    }

    pub fn find_surface(&self, surface: &WlSurface) -> Location {
        if let Some(decor) = self.decoration() {
            if surface.as_ref().equals(decor.header.surface.as_ref()) {
                Location::Head
            } else if surface.as_ref().equals(decor.top.surface.as_ref()) {
                Location::Top
            } else if surface.as_ref().equals(decor.bottom.surface.as_ref()) {
                Location::Bottom
            } else if surface.as_ref().equals(decor.left.surface.as_ref()) {
                Location::Left
            } else if surface.as_ref().equals(decor.right.surface.as_ref()) {
                Location::Right
            } else {
                Location::None
            }
        } else {
            Location::None
        }
    }
}

#[derive(Debug)]
pub struct Part {
    pub surface: WlSurface,
    pub subsurface: WlSubsurface,
}

impl Part {
    fn new(
        parent: &WlSurface,
        compositor: &Attached<WlCompositor>,
        subcompositor: &Attached<WlSubcompositor>,
        inner: Option<Rc<RefCell<Inner>>>,
    ) -> Part {
        let surface = if let Some(inner) = inner {
            surface::setup_surface(
                compositor.create_surface(),
                Some(move |dpi, surface: WlSurface, ddata: DispatchData| {
                    surface.set_buffer_scale(dpi);
                    surface.commit();
                    (inner.borrow_mut().implem)(FrameRequest::Refresh, 0, ddata);
                }),
            )
        } else {
            surface::setup_surface(
                compositor.create_surface(),
                Some(move |dpi, surface: WlSurface, _ddata: DispatchData| {
                    surface.set_buffer_scale(dpi);
                    surface.commit();
                }),
            )
        };

        let surface = surface.detach();

        let subsurface = subcompositor.get_subsurface(&surface, parent);

        Part {
            surface,
            subsurface: subsurface.detach(),
        }
    }

    pub fn scale(&self) -> u32 {
        surface::get_surface_scale_factor(&self.surface) as u32
    }
}

impl Drop for Part {
    fn drop(&mut self) {
        self.subsurface.destroy();
        self.surface.destroy();
    }
}
