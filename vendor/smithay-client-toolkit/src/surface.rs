use std::{cell::RefCell, rc::Rc, sync::Mutex};

use wayland_client::{
    protocol::{wl_compositor, wl_output, wl_surface},
    Attached, DispatchData, Main,
};

use crate::output::{add_output_listener, with_output_info, OutputListener};

pub(crate) struct SurfaceUserData {
    scale_factor: i32,
    outputs: Vec<(wl_output::WlOutput, i32, OutputListener)>,
}

impl SurfaceUserData {
    fn new() -> Self {
        SurfaceUserData { scale_factor: 1, outputs: Vec::new() }
    }

    pub(crate) fn enter<F>(
        &mut self,
        output: wl_output::WlOutput,
        surface: wl_surface::WlSurface,
        callback: &Option<Rc<RefCell<F>>>,
    ) where
        F: FnMut(i32, wl_surface::WlSurface, DispatchData) + 'static,
    {
        let output_scale = with_output_info(&output, |info| info.scale_factor).unwrap_or(1);
        let my_surface = surface.clone();
        // Use a UserData to safely share the callback with the other thread
        let my_callback = wayland_client::UserData::new();
        if let Some(ref cb) = callback {
            my_callback.set(|| cb.clone());
        }
        let listener = add_output_listener(&output, move |output, info, ddata| {
            let mut user_data = my_surface
                .as_ref()
                .user_data()
                .get::<Mutex<SurfaceUserData>>()
                .unwrap()
                .lock()
                .unwrap();
            // update the scale factor of the relevant output
            for (ref o, ref mut factor, _) in user_data.outputs.iter_mut() {
                if o.as_ref().equals(output.as_ref()) {
                    if info.obsolete {
                        // an output that no longer exists is marked by a scale factor of -1
                        *factor = -1;
                    } else {
                        *factor = info.scale_factor;
                    }
                    break;
                }
            }
            // recompute the scale factor with the new info
            let callback = my_callback.get::<Rc<RefCell<F>>>().cloned();
            let old_scale_factor = user_data.scale_factor;
            let new_scale_factor = user_data.recompute_scale_factor();
            drop(user_data);
            if let Some(ref cb) = callback {
                if old_scale_factor != new_scale_factor {
                    (cb.borrow_mut())(new_scale_factor, surface.clone(), ddata);
                }
            }
        });
        self.outputs.push((output, output_scale, listener));
    }

    pub(crate) fn leave(&mut self, output: &wl_output::WlOutput) {
        self.outputs.retain(|(ref output2, _, _)| !output.as_ref().equals(output2.as_ref()));
    }

    fn recompute_scale_factor(&mut self) -> i32 {
        let mut new_scale_factor = 1;
        self.outputs.retain(|&(_, output_scale, _)| {
            if output_scale > 0 {
                new_scale_factor = ::std::cmp::max(new_scale_factor, output_scale);
                true
            } else {
                // cleanup obsolete output
                false
            }
        });
        if self.outputs.is_empty() {
            // don't update the scale factor if we are not displayed on any output
            return self.scale_factor;
        }
        self.scale_factor = new_scale_factor;
        new_scale_factor
    }
}

pub(crate) fn setup_surface<F>(
    surface: Main<wl_surface::WlSurface>,
    callback: Option<F>,
) -> Attached<wl_surface::WlSurface>
where
    F: FnMut(i32, wl_surface::WlSurface, DispatchData) + 'static,
{
    let callback = callback.map(|c| Rc::new(RefCell::new(c)));
    surface.quick_assign(move |surface, event, ddata| {
        let mut user_data =
            surface.as_ref().user_data().get::<Mutex<SurfaceUserData>>().unwrap().lock().unwrap();
        match event {
            wl_surface::Event::Enter { output } => {
                // Passing the callback to be added to output listener
                user_data.enter(output, surface.detach(), &callback);
            }
            wl_surface::Event::Leave { output } => {
                user_data.leave(&output);
            }
            _ => unreachable!(),
        };
        let old_scale_factor = user_data.scale_factor;
        let new_scale_factor = user_data.recompute_scale_factor();
        drop(user_data);
        if let Some(ref cb) = callback {
            if old_scale_factor != new_scale_factor {
                (cb.borrow_mut())(new_scale_factor, surface.detach(), ddata);
            }
        }
    });
    surface.as_ref().user_data().set_threadsafe(|| Mutex::new(SurfaceUserData::new()));
    surface.into()
}

impl<E: crate::environment::GlobalHandler<wl_compositor::WlCompositor>>
    crate::environment::Environment<E>
{
    /// Create a DPI-aware surface
    ///
    /// This surface will track the outputs it is being displayed on, and compute the
    /// optimal scale factor for these. You can access them using
    /// [`get_surface_scale_factor`](../fn.get_surface_scale_factor.html) and
    /// [`get_surface_outputs`](../fn.get_surface_outputs.html).
    pub fn create_surface(&self) -> Attached<wl_surface::WlSurface> {
        let compositor = self.require_global::<wl_compositor::WlCompositor>();
        setup_surface(compositor.create_surface(), None::<fn(_, _, DispatchData)>)
    }

    /// Create a DPI-aware surface with callbacks
    ///
    /// This method is like `create_surface`, but the provided callback will also be
    /// notified whenever the scale factor of this surface change, if you don't want to have to
    /// periodically check it.
    pub fn create_surface_with_scale_callback<
        F: FnMut(i32, wl_surface::WlSurface, DispatchData) + 'static,
    >(
        &self,
        f: F,
    ) -> Attached<wl_surface::WlSurface> {
        let compositor = self.require_global::<wl_compositor::WlCompositor>();
        setup_surface(compositor.create_surface(), Some(f))
    }
}

/// Returns the current suggested scale factor of a surface.
///
/// Panics if the surface was not created using `Environment::create_surface` or
/// `Environment::create_surface_with_dpi_callback`.
pub fn get_surface_scale_factor(surface: &wl_surface::WlSurface) -> i32 {
    surface
        .as_ref()
        .user_data()
        .get::<Mutex<SurfaceUserData>>()
        .expect("SCTK: Surface was not created by SCTK.")
        .lock()
        .unwrap()
        .scale_factor
}

/// Returns a list of outputs the surface is displayed on.
///
/// Panics if the surface was not created using `Environment::create_surface` or
/// `Environment::create_surface_with_dpi_callback`.
pub fn get_surface_outputs(surface: &wl_surface::WlSurface) -> Vec<wl_output::WlOutput> {
    surface
        .as_ref()
        .user_data()
        .get::<Mutex<SurfaceUserData>>()
        .expect("SCTK: Surface was not created by SCTK.")
        .lock()
        .unwrap()
        .outputs
        .iter()
        .map(|(ref output, _, _)| output.clone())
        .collect()
}
