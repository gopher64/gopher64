extern crate smithay_client_toolkit as sctk;

use std::cmp::min;

use sctk::reexports::client::protocol::{wl_pointer, wl_shm, wl_surface};
use sctk::shm::AutoMemPool;
use sctk::window::{Event as WEvent, FallbackFrame};

#[derive(Debug)]
enum NextAction {
    Refresh,
    Redraw,
    Exit,
}

struct WindowConfig {
    width: u32,
    height: u32,
    dpi_scale: i32,
    next_action: Option<NextAction>,
    has_drawn_once: bool,
}

impl WindowConfig {
    pub fn new() -> Self {
        WindowConfig {
            width: 320,
            height: 240,
            dpi_scale: 1,
            next_action: None,
            has_drawn_once: false,
        }
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width * self.dpi_scale as u32, self.height * self.dpi_scale as u32)
    }

    pub fn handle_action(&mut self, new_action: NextAction) {
        let replace = matches!(
            (&self.next_action, &new_action),
            (&None, _)
                | (&Some(NextAction::Refresh), _)
                | (&Some(NextAction::Redraw), &NextAction::Exit)
        );
        if replace {
            self.next_action = Some(new_action);
        }
    }
}

sctk::default_environment!(PtrInputExample, desktop);

fn main() {
    /*
     * Initial setup
     */
    let (env, _display, mut queue) = sctk::new_default_environment!(PtrInputExample, desktop)
        .expect("Unable to connect to a Wayland compositor");

    /*
     * Init wayland objects
     */

    let mut window_config = WindowConfig::new();

    let surface = env
        .create_surface_with_scale_callback(move |dpi, surface, mut dispatch_data| {
            let config = dispatch_data.get::<WindowConfig>().unwrap();
            surface.set_buffer_scale(dpi);
            config.dpi_scale = dpi;
            config.handle_action(NextAction::Redraw);
        })
        .detach();

    let mut window = env
        .create_window::<FallbackFrame, _>(
            surface,
            None,
            window_config.dimensions(),
            move |event, mut dispatch_data| {
                let mut config = dispatch_data.get::<WindowConfig>().unwrap();
                match event {
                    WEvent::Refresh => config.handle_action(NextAction::Refresh),
                    WEvent::Configure { new_size: Some((w, h)), .. } => {
                        if config.dimensions() != (w, h) || !config.has_drawn_once {
                            config.width = w;
                            config.height = h;
                            config.handle_action(NextAction::Redraw);
                        } else {
                            config.handle_action(NextAction::Refresh);
                        }
                    }
                    WEvent::Configure { new_size: None, .. } => {
                        if config.has_drawn_once {
                            config.handle_action(NextAction::Refresh)
                        } else {
                            config.handle_action(NextAction::Redraw)
                        }
                    }
                    WEvent::Close => config.handle_action(NextAction::Exit),
                }
            },
        )
        .expect("Failed to create a window !");

    let mut pool = env.create_auto_pool().expect("Failed to create a memory pool !");

    /*
     * Pointer initialization
     */
    let mut seats = Vec::<(String, Option<wl_pointer::WlPointer>)>::new();

    // first process already existing seats
    for seat in env.get_all_seats() {
        if let Some((has_ptr, name)) = sctk::seat::with_seat_data(&seat, |seat_data| {
            (seat_data.has_pointer && !seat_data.defunct, seat_data.name.clone())
        }) {
            if has_ptr {
                let seat_name = name.clone();
                let pointer = seat.get_pointer();
                let surface = window.surface().clone();
                pointer.quick_assign(move |_, event, _| {
                    print_pointer_event(event, &seat_name, &surface)
                });
            } else {
                seats.push((name, None));
            }
        }
    }

    // then setup a listener for changes
    let main_surface = window.surface().clone();
    let _seat_listener = env.listen_for_seats(move |seat, seat_data, _| {
        // find the seat in the vec of seats, or insert it if it is unknown
        let idx = seats.iter().position(|(name, _)| name == &seat_data.name);
        let idx = idx.unwrap_or_else(|| {
            seats.push((seat_data.name.clone(), None));
            seats.len() - 1
        });

        let (_, ref mut opt_ptr) = &mut seats[idx];
        // we should map a keyboard if the seat has the capability & is not defunct
        if seat_data.has_keyboard && !seat_data.defunct {
            if opt_ptr.is_none() {
                // we should initalize a keyboard
                let seat_name = seat_data.name.clone();
                let pointer = seat.get_pointer();
                let surface = main_surface.clone();
                pointer.quick_assign(move |_, event, _| {
                    print_pointer_event(event, &seat_name, &surface)
                });
                *opt_ptr = Some(pointer.detach());
            }
        } else if let Some(ptr) = opt_ptr.take() {
            // the pointer has been removed, cleanup
            ptr.release();
        }
    });

    if !env.get_shell().unwrap().needs_configure() {
        window_config.handle_action(NextAction::Redraw);
    }

    loop {
        let next_action = window_config.next_action.take();
        println!("{:?}", next_action);
        match next_action {
            Some(NextAction::Exit) => break,
            Some(NextAction::Refresh) => {
                window.refresh();
                window.surface().commit();
            }
            Some(NextAction::Redraw) => {
                window_config.has_drawn_once = true;
                let (w, h) = window_config.dimensions();
                window.resize(w, h);
                window.refresh();
                redraw(&mut pool, window.surface(), window_config.dimensions())
                    .expect("Failed to draw");
            }
            None => {}
        }

        queue.dispatch(&mut window_config, |_, _, _| {}).unwrap();
    }
}

#[allow(clippy::many_single_char_names)]
fn redraw(
    pool: &mut AutoMemPool,
    surface: &wl_surface::WlSurface,
    (buf_x, buf_y): (u32, u32),
) -> Result<(), ::std::io::Error> {
    let (canvas, new_buffer) =
        pool.buffer(buf_x as i32, buf_y as i32, 4 * buf_x as i32, wl_shm::Format::Argb8888)?;
    for (i, dst_pixel) in canvas.chunks_exact_mut(4).enumerate() {
        let x = i as u32 % buf_x;
        let y = i as u32 / buf_x;
        let r: u32 = min(((buf_x - x) * 0xFF) / buf_x, ((buf_y - y) * 0xFF) / buf_y);
        let g: u32 = min((x * 0xFF) / buf_x, ((buf_y - y) * 0xFF) / buf_y);
        let b: u32 = min(((buf_x - x) * 0xFF) / buf_x, (y * 0xFF) / buf_y);
        let pixel: [u8; 4] = ((0xFF << 24) + (r << 16) + (g << 8) + b).to_ne_bytes();
        dst_pixel[0] = pixel[0];
        dst_pixel[1] = pixel[1];
        dst_pixel[2] = pixel[2];
        dst_pixel[3] = pixel[3];
    }
    surface.attach(Some(&new_buffer), 0, 0);
    if surface.as_ref().version() >= 4 {
        surface.damage_buffer(0, 0, buf_x as i32, buf_y as i32);
    } else {
        surface.damage(0, 0, buf_x as i32, buf_y as i32);
    }
    surface.commit();
    Ok(())
}

fn print_pointer_event(
    event: wl_pointer::Event,
    seat_name: &str,
    main_surface: &wl_surface::WlSurface,
) {
    match event {
        wl_pointer::Event::Enter { surface, surface_x, surface_y, .. } => {
            if main_surface == &surface {
                println!(
                    "Pointer of seat '{}' entered at ({}, {})",
                    seat_name, surface_x, surface_y
                );
            }
        }
        wl_pointer::Event::Leave { surface, .. } => {
            if main_surface == &surface {
                println!("Pointer of seat '{}' left", seat_name);
            }
        }
        wl_pointer::Event::Button { button, state, .. } => {
            println!("Button {:?} of seat '{}' was {:?}", button, seat_name, state);
        }
        wl_pointer::Event::Motion { surface_x, surface_y, .. } => {
            println!("Pointer motion to ({}, {}) on seat '{}'", surface_x, surface_y, seat_name)
        }
        _ => {}
    }
}
