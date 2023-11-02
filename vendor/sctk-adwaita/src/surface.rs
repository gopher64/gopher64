use std::{cell::RefCell, rc::Rc, sync::Mutex};

use super::client;
use smithay_client_toolkit as sctk;

use client::{
    protocol::{wl_output, wl_surface},
    Attached, DispatchData, Main,
};
use sctk::output::{add_output_listener, with_output_info, OutputListener};

pub(crate) struct SurfaceUserData {
    scale_factor: i32,
    outputs: Vec<(wl_output::WlOutput, i32, OutputListener)>,
}

impl SurfaceUserData {
    fn new() -> Self {
        SurfaceUserData {
            scale_factor: 1,
            outputs: Vec::new(),
        }
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
        let my_callback = client::UserData::new();
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
                    (*cb.borrow_mut())(new_scale_factor, surface.clone(), ddata);
                }
            }
        });
        self.outputs.push((output, output_scale, listener));
    }

    pub(crate) fn leave(&mut self, output: &wl_output::WlOutput) {
        self.outputs
            .retain(|(ref output2, _, _)| !output.as_ref().equals(output2.as_ref()));
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

pub fn setup_surface<F>(
    surface: Main<wl_surface::WlSurface>,
    callback: Option<F>,
) -> Attached<wl_surface::WlSurface>
where
    F: FnMut(i32, wl_surface::WlSurface, DispatchData) + 'static,
{
    let callback = callback.map(|c| Rc::new(RefCell::new(c)));
    surface.quick_assign(move |surface, event, ddata| {
        let mut user_data = surface
            .as_ref()
            .user_data()
            .get::<Mutex<SurfaceUserData>>()
            .unwrap()
            .lock()
            .unwrap();
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
                (*cb.borrow_mut())(new_scale_factor, surface.detach(), ddata);
            }
        }
    });
    surface
        .as_ref()
        .user_data()
        .set_threadsafe(|| Mutex::new(SurfaceUserData::new()));
    surface.into()
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
