use std::{
    cell::RefCell,
    fmt,
    ops::Deref,
    rc::{Rc, Weak},
};
use wayland_client::{
    protocol::{wl_compositor, wl_pointer, wl_seat, wl_shm, wl_surface},
    Attached, DispatchData,
};
use wayland_cursor::{Cursor, CursorTheme};

/// The specification of a cursor theme to be used by the ThemeManager
#[derive(Debug)]
pub enum ThemeSpec<'a> {
    /// Use this specific theme with given base size
    Precise {
        /// Name of the cursor theme to use
        name: &'a str,
        /// Base size of the cursor images
        ///
        /// This is the size that will be used on monitors with a scale
        /// factor of 1. Cursor images sizes will be multiples of this
        /// base size on HiDPI outputs.
        size: u32,
    },
    /// Use the system provided theme
    ///
    /// In this case SCTK will read the `XCURSOR_THEME` and
    /// `XCURSOR_SIZE` environment variables to figure out the
    /// theme to use.
    System,
}

/// Wrapper managing a system theme for pointer images
///
/// You can use it to initialize new pointers in order
/// to theme them.
///
/// Is is also clone-able in case you need to handle several
/// pointer theming from different places.
///
/// Note that it is however neither `Send` nor `Sync`
#[derive(Debug, Clone)]
pub struct ThemeManager {
    themes: Rc<RefCell<ScaledThemeList>>,
    compositor: Attached<wl_compositor::WlCompositor>,
}

impl ThemeManager {
    /// Load a system pointer theme
    ///
    /// Will use the default theme of the system if name is `None`.
    pub fn init(
        theme: ThemeSpec,
        compositor: Attached<wl_compositor::WlCompositor>,
        shm: Attached<wl_shm::WlShm>,
    ) -> ThemeManager {
        ThemeManager { compositor, themes: Rc::new(RefCell::new(ScaledThemeList::new(theme, shm))) }
    }

    /// Wrap a pointer to theme it
    pub fn theme_pointer(&self, pointer: wl_pointer::WlPointer) -> ThemedPointer {
        let surface = self.compositor.create_surface();
        let inner = Rc::new(RefCell::new(PointerInner {
            surface: surface.detach(),
            themes: self.themes.clone(),
            last_serial: 0,
            current_cursor: "left_ptr".into(),
            scale_factor: 1,
        }));
        let my_pointer = pointer.clone();
        let winner = Rc::downgrade(&inner);
        crate::surface::setup_surface(
            surface,
            Some(move |scale_factor, _, _: DispatchData| {
                if let Some(inner) = Weak::upgrade(&winner) {
                    let mut inner = inner.borrow_mut();
                    inner.scale_factor = scale_factor;
                    // we can't handle errors here, so ignore it
                    // worst that can happen is cursor drawn with the wrong
                    // scale factor
                    let _ = inner.update_cursor(&my_pointer);
                }
            }),
        );
        ThemedPointer { pointer, inner }
    }

    /// Initialize a new pointer as a ThemedPointer with an adapter implementation
    ///
    /// You need to provide an implementation as if implementing a `wl_pointer`, but
    /// it will receive as `meta` argument a `ThemedPointer` wrapping your pointer,
    /// rather than a `WlPointer`.
    pub fn theme_pointer_with_impl<F>(
        &self,
        seat: &Attached<wl_seat::WlSeat>,
        mut callback: F,
    ) -> ThemedPointer
    where
        F: FnMut(wl_pointer::Event, ThemedPointer, DispatchData) + 'static,
    {
        let surface = self.compositor.create_surface();
        let inner = Rc::new(RefCell::new(PointerInner {
            surface: surface.detach(),
            themes: self.themes.clone(),
            last_serial: 0,
            current_cursor: "left_ptr".into(),
            scale_factor: 1,
        }));

        let inner2 = inner.clone();
        let pointer = seat.get_pointer();
        pointer.quick_assign(move |ptr, event, ddata| {
            callback(event, ThemedPointer { pointer: ptr.detach(), inner: inner2.clone() }, ddata)
        });

        let winner = Rc::downgrade(&inner);
        let my_pointer = pointer.clone();
        crate::surface::setup_surface(
            surface,
            Some(move |scale_factor, _, _: DispatchData| {
                if let Some(inner) = Weak::upgrade(&winner) {
                    let mut inner = inner.borrow_mut();
                    inner.scale_factor = scale_factor;
                    // we can't handle errors here, so ignore it
                    // worst that can happen is cursor drawn with the wrong
                    // scale factor
                    let _ = inner.update_cursor(&my_pointer);
                }
            }),
        );

        ThemedPointer { pointer: pointer.detach(), inner }
    }
}

struct ScaledThemeList {
    shm: Attached<wl_shm::WlShm>,
    name: String,
    size: u32,
    themes: Vec<(u32, CursorTheme)>,
}

impl ScaledThemeList {
    fn new(theme: ThemeSpec, shm: Attached<wl_shm::WlShm>) -> ScaledThemeList {
        let (name, size) = match theme {
            ThemeSpec::Precise { name, size } => (name.into(), size),
            ThemeSpec::System => {
                let name = std::env::var("XCURSOR_THEME").ok().unwrap_or_else(|| "default".into());
                let size =
                    std::env::var("XCURSOR_SIZE").ok().and_then(|s| s.parse().ok()).unwrap_or(24);
                (name, size)
            }
        };
        ScaledThemeList { shm, name, size, themes: vec![] }
    }

    fn get_cursor(&mut self, name: &str, scale: u32) -> Option<&Cursor> {
        // Check if we already loaded the theme for this scale factor
        let opt_index = self.themes.iter().position(|&(s, _)| s == scale);
        if let Some(idx) = opt_index {
            self.themes[idx].1.get_cursor(name)
        } else {
            let new_theme = CursorTheme::load_from_name(&self.name, self.size * scale, &self.shm);
            self.themes.push((scale, new_theme));
            self.themes.last_mut().unwrap().1.get_cursor(name)
        }
    }
}

impl fmt::Debug for ScaledThemeList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScaledThemeList")
            .field("shm", &self.shm)
            .field("name", &self.name)
            .field("size", &self.size)
            // Wayland-cursor needs to implement debug
            .field("themes", &"[...]")
            .finish()
    }
}

#[derive(Debug)]
struct PointerInner {
    surface: wl_surface::WlSurface,
    themes: Rc<RefCell<ScaledThemeList>>,
    current_cursor: String,
    last_serial: u32,
    scale_factor: i32,
}

impl PointerInner {
    fn update_cursor(&self, pointer: &wl_pointer::WlPointer) -> Result<(), CursorNotFound> {
        let mut themes = self.themes.borrow_mut();
        let scale = self.scale_factor as u32;
        let cursor = themes.get_cursor(&self.current_cursor, scale).ok_or(CursorNotFound)?;
        let image = &cursor[0];
        let (w, h) = image.dimensions();
        let (hx, hy) = image.hotspot();
        self.surface.set_buffer_scale(scale as i32);
        self.surface.attach(Some(image), 0, 0);
        if self.surface.as_ref().version() >= 4 {
            self.surface.damage_buffer(0, 0, w as i32, h as i32);
        } else {
            // surface is old and does not support damage_buffer, so we damage
            // in surface coordinates and hope it is not rescaled
            self.surface.damage(0, 0, w as i32 / scale as i32, h as i32 / scale as i32);
        }
        self.surface.commit();
        pointer.set_cursor(
            self.last_serial,
            Some(&self.surface),
            hx as i32 / scale as i32,
            hy as i32 / scale as i32,
        );
        Ok(())
    }
}

/// Wrapper of a themed pointer
///
/// You can access the underlying `wl_pointer::WlPointer` via
/// deref. It will *not* release the proxy when dropped.
///
/// Just like `Proxy`, this is a `Rc`-like wrapper. You can clone it
/// to have several handles to the same theming machinery of a pointer.
#[derive(Debug, Clone)]
pub struct ThemedPointer {
    pointer: wl_pointer::WlPointer,
    inner: Rc<RefCell<PointerInner>>,
}

impl ThemedPointer {
    /// Change the cursor to the given cursor name
    ///
    /// Possible names depend on the theme. Does nothing and returns
    /// `Err` if given name is not available.
    ///
    /// If this is done as an answer to an input event, you need to provide
    /// the associated serial otherwise the server may ignore the request.
    pub fn set_cursor(&self, name: &str, serial: Option<u32>) -> Result<(), CursorNotFound> {
        let mut inner = self.inner.borrow_mut();
        if let Some(s) = serial {
            inner.last_serial = s;
        }
        inner.current_cursor = name.into();
        inner.update_cursor(&self.pointer)
    }
}

impl Deref for ThemedPointer {
    type Target = wl_pointer::WlPointer;
    fn deref(&self) -> &wl_pointer::WlPointer {
        &self.pointer
    }
}

impl Drop for PointerInner {
    fn drop(&mut self) {
        self.surface.destroy();
    }
}

/// An error representing the fact that the required cursor was not found
#[derive(Debug, Copy, Clone)]
pub struct CursorNotFound;

impl std::error::Error for CursorNotFound {}

impl std::fmt::Display for CursorNotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("cursor not found")
    }
}
