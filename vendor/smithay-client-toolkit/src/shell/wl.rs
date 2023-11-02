use wayland_client::{
    protocol::{wl_output, wl_seat, wl_shell, wl_shell_surface, wl_surface},
    DispatchData,
};

use wayland_protocols::xdg_shell::client::xdg_toplevel;

use super::{Event, ShellSurface};

#[derive(Debug)]
pub(crate) struct Wl {
    shell_surface: wl_shell_surface::WlShellSurface,
}

impl Wl {
    pub(crate) fn create<Impl>(
        surface: &wl_surface::WlSurface,
        shell: &wl_shell::WlShell,
        mut implementation: Impl,
    ) -> Wl
    where
        Impl: FnMut(Event, DispatchData) + 'static,
    {
        let shell_surface = shell.get_shell_surface(surface);
        shell_surface.quick_assign(move |shell_surface, event, ddata| match event {
            wl_shell_surface::Event::Ping { serial } => {
                shell_surface.pong(serial);
            }
            wl_shell_surface::Event::Configure { width, height, .. } => {
                use std::cmp::max;
                implementation(
                    Event::Configure {
                        new_size: Some((max(width, 1) as u32, max(height, 1) as u32)),
                        states: Vec::new(),
                    },
                    ddata,
                );
            }
            wl_shell_surface::Event::PopupDone => {
                unreachable!();
            }
            _ => unreachable!(),
        });
        shell_surface.set_toplevel();
        Wl { shell_surface: shell_surface.detach() }
    }
}

impl ShellSurface for Wl {
    fn resize(&self, seat: &wl_seat::WlSeat, serial: u32, edges: xdg_toplevel::ResizeEdge) {
        let edges = match edges {
            xdg_toplevel::ResizeEdge::None => wl_shell_surface::Resize::None,
            xdg_toplevel::ResizeEdge::Top => wl_shell_surface::Resize::Top,
            xdg_toplevel::ResizeEdge::Left => wl_shell_surface::Resize::Left,
            xdg_toplevel::ResizeEdge::Right => wl_shell_surface::Resize::Right,
            xdg_toplevel::ResizeEdge::Bottom => wl_shell_surface::Resize::Bottom,
            xdg_toplevel::ResizeEdge::TopLeft => wl_shell_surface::Resize::TopLeft,
            xdg_toplevel::ResizeEdge::TopRight => wl_shell_surface::Resize::TopRight,
            xdg_toplevel::ResizeEdge::BottomLeft => wl_shell_surface::Resize::BottomLeft,
            xdg_toplevel::ResizeEdge::BottomRight => wl_shell_surface::Resize::BottomRight,
            _ => unreachable!(),
        };
        self.shell_surface.resize(seat, serial, edges);
    }

    fn move_(&self, seat: &wl_seat::WlSeat, serial: u32) {
        self.shell_surface._move(seat, serial);
    }

    fn set_title(&self, title: String) {
        self.shell_surface.set_title(title);
    }

    fn set_app_id(&self, app_id: String) {
        self.shell_surface.set_class(app_id);
    }
    fn set_fullscreen(&self, output: Option<&wl_output::WlOutput>) {
        self.shell_surface.set_fullscreen(wl_shell_surface::FullscreenMethod::Default, 0, output)
    }

    fn unset_fullscreen(&self) {
        self.shell_surface.set_toplevel();
    }

    fn set_maximized(&self) {
        self.shell_surface.set_maximized(None);
    }

    fn unset_maximized(&self) {
        self.shell_surface.set_toplevel();
    }

    fn show_window_menu(&self, _: &wl_seat::WlSeat, _: u32, _: i32, _: i32) {
        /* not available */
    }

    fn set_minimized(&self) {
        /* not available */
    }

    fn set_geometry(&self, _: i32, _: i32, _: i32, _: i32) {
        /* not available */
    }

    fn set_min_size(&self, _: Option<(i32, i32)>) {
        /* not available */
    }

    fn set_max_size(&self, _: Option<(i32, i32)>) {
        /* not available */
    }

    fn get_xdg(&self) -> Option<&xdg_toplevel::XdgToplevel> {
        None
    }
}
