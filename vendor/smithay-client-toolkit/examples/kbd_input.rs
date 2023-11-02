extern crate smithay_client_toolkit as sctk;

use std::cmp::min;

use sctk::reexports::calloop;
use sctk::reexports::client::protocol::{wl_keyboard, wl_shm, wl_surface};
use sctk::seat::keyboard::{map_keyboard_repeat, Event as KbEvent, RepeatKind};
use sctk::shm::AutoMemPool;
use sctk::window::{Event as WEvent, FallbackFrame};

sctk::default_environment!(KbdInputExample, desktop);

fn main() {
    /*
     * Initial setup
     */
    let (env, display, queue) = sctk::new_default_environment!(KbdInputExample, desktop)
        .expect("Unable to connect to a Wayland compositor");

    /*
     * Prepare a calloop event loop to handle key repetion
     */
    // Here `Option<WEvent>` is the type of a global value that will be shared by
    // all callbacks invoked by the event loop.
    let mut event_loop = calloop::EventLoop::<Option<WEvent>>::try_new().unwrap();

    /*
     * Create a buffer with window contents
     */

    let mut dimensions = (320u32, 240u32);

    /*
     * Init wayland objects
     */

    let surface = env.create_surface().detach();

    let mut window = env
        .create_window::<FallbackFrame, _>(
            surface,
            None,
            dimensions,
            move |evt, mut dispatch_data| {
                let next_action = dispatch_data.get::<Option<WEvent>>().unwrap();
                // Keep last event in priority order : Close > Configure > Refresh
                let replace = matches!(
                    (&evt, &*next_action),
                    (_, &None)
                        | (_, &Some(WEvent::Refresh))
                        | (&WEvent::Configure { .. }, &Some(WEvent::Configure { .. }))
                        | (&WEvent::Close, _)
                );
                if replace {
                    *next_action = Some(evt);
                }
            },
        )
        .expect("Failed to create a window !");

    window.set_title("Kbd Input".to_string());

    let mut pool = env.create_auto_pool().expect("Failed to create a memory pool !");

    /*
     * Keyboard initialization
     */

    let mut seats = Vec::<(String, Option<wl_keyboard::WlKeyboard>)>::new();

    // first process already existing seats
    for seat in env.get_all_seats() {
        if let Some((has_kbd, name)) = sctk::seat::with_seat_data(&seat, |seat_data| {
            (seat_data.has_keyboard && !seat_data.defunct, seat_data.name.clone())
        }) {
            if has_kbd {
                let seat_name = name.clone();
                match map_keyboard_repeat(
                    event_loop.handle(),
                    &seat,
                    None,
                    RepeatKind::System,
                    move |event, _, _| print_keyboard_event(event, &seat_name),
                ) {
                    Ok(kbd) => {
                        seats.push((name, Some(kbd)));
                    }
                    Err(e) => {
                        eprintln!("Failed to map keyboard on seat {} : {:?}.", name, e);
                        seats.push((name, None));
                    }
                }
            } else {
                seats.push((name, None));
            }
        }
    }

    // then setup a listener for changes
    let loop_handle = event_loop.handle();
    let _seat_listener = env.listen_for_seats(move |seat, seat_data, _| {
        // find the seat in the vec of seats, or insert it if it is unknown
        let idx = seats.iter().position(|(name, _)| name == &seat_data.name);
        let idx = idx.unwrap_or_else(|| {
            seats.push((seat_data.name.clone(), None));
            seats.len() - 1
        });

        let (_, ref mut opt_kbd) = &mut seats[idx];
        // we should map a keyboard if the seat has the capability & is not defunct
        if seat_data.has_keyboard && !seat_data.defunct {
            if opt_kbd.is_none() {
                // we should initalize a keyboard
                let seat_name = seat_data.name.clone();
                match map_keyboard_repeat(
                    loop_handle.clone(),
                    &seat,
                    None,
                    RepeatKind::System,
                    move |event, _, _| print_keyboard_event(event, &seat_name),
                ) {
                    Ok(kbd) => {
                        *opt_kbd = Some(kbd);
                    }
                    Err(e) => {
                        eprintln!("Failed to map keyboard on seat {} : {:?}.", seat_data.name, e)
                    }
                }
            }
        } else if let Some(kbd) = opt_kbd.take() {
            // the keyboard has been removed, cleanup
            kbd.release();
        }
    });

    if !env.get_shell().unwrap().needs_configure() {
        // initial draw to bootstrap on wl_shell
        redraw(&mut pool, window.surface(), dimensions).expect("Failed to draw");
        window.refresh();
    }

    let mut next_action = None;

    sctk::WaylandSource::new(queue).quick_insert(event_loop.handle()).unwrap();

    loop {
        match next_action.take() {
            Some(WEvent::Close) => break,
            Some(WEvent::Refresh) => {
                window.refresh();
                window.surface().commit();
            }
            Some(WEvent::Configure { new_size, states }) => {
                if let Some((w, h)) = new_size {
                    window.resize(w, h);
                    dimensions = (w, h)
                }
                println!("Window states: {:?}", states);
                window.refresh();
                redraw(&mut pool, window.surface(), dimensions).expect("Failed to draw");
            }
            None => {}
        }

        // always flush the connection before going to sleep waiting for events
        display.flush().unwrap();

        event_loop.dispatch(None, &mut next_action).unwrap();
    }
}

fn print_keyboard_event(event: KbEvent, seat_name: &str) {
    match event {
        KbEvent::Enter { keysyms, .. } => {
            println!("Gained focus on seat '{}' while {} keys pressed.", seat_name, keysyms.len(),);
        }
        KbEvent::Leave { .. } => {
            println!("Lost focus on seat '{}'.", seat_name);
        }
        KbEvent::Key { keysym, state, utf8, .. } => {
            println!("Key {:?}: {:x} on seat '{}'.", state, keysym, seat_name);
            if let Some(txt) = utf8 {
                println!(" -> Received text \"{}\".", txt);
            }
        }
        KbEvent::Modifiers { modifiers } => {
            println!("Modifiers changed to {:?} on seat '{}'.", modifiers, seat_name);
        }
        KbEvent::Repeat { keysym, utf8, .. } => {
            println!("Key repetition {:x} on seat '{}'.", keysym, seat_name);
            if let Some(txt) = utf8 {
                println!(" -> Received text \"{}\".", txt);
            }
        }
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
