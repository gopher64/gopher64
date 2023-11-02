extern crate smithay_client_toolkit as sctk;

use std::io::{Read, Write};

use sctk::{
    data_device::DataSourceEvent,
    environment::Environment,
    primary_selection::PrimarySelectionSourceEvent,
    seat::keyboard::{map_keyboard_repeat, Event as KbEvent, KeyState, RepeatKind},
    shm::AutoMemPool,
    window::{Event as WEvent, FallbackFrame},
};

use sctk::reexports::{
    calloop::{LoopHandle, RegistrationToken},
    client::{
        protocol::{wl_keyboard, wl_seat, wl_shm, wl_surface},
        DispatchData,
    },
};

sctk::default_environment!(SelectionExample, desktop);

// Here the type parameter is a global value that will be shared by
// all callbacks invoked by the event loop.
type DData = (Environment<SelectionExample>, Option<WEvent>, Option<RegistrationToken>);

fn main() {
    /*
     * Initial setup
     */
    let (env, display, queue) = sctk::new_default_environment!(SelectionExample, desktop)
        .expect("Unable to connect to a Wayland compositor");

    /*
     * Prepare a calloop event loop to handle clipboard reading
     */
    let mut event_loop = sctk::reexports::calloop::EventLoop::<DData>::try_new().unwrap();

    // we need a window to receive things actually
    let mut dimensions = (320u32, 240u32);
    let surface = env.create_surface().detach();

    let mut window = env
        .create_window::<FallbackFrame, _>(
            surface,
            None,
            dimensions,
            move |evt, mut dispatch_data| {
                let (_, next_action, _) = dispatch_data.get::<DData>().unwrap();
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
    window.set_title("Selection example".to_string());

    println!("Press c/C p/P to copy/paste from selection/primary clipboard respectively.");

    let mut pool = env.create_auto_pool().expect("Failed to create a memory pool !");

    let mut seats = Vec::<(String, Option<wl_keyboard::WlKeyboard>)>::new();

    // first process already existing seats
    for seat in env.get_all_seats() {
        if let Some((has_kbd, name)) = sctk::seat::with_seat_data(&seat, |seat_data| {
            (seat_data.has_keyboard && !seat_data.defunct, seat_data.name.clone())
        }) {
            if has_kbd {
                let my_seat = seat.clone();
                let handle = event_loop.handle();
                match map_keyboard_repeat(
                    event_loop.handle(),
                    &seat,
                    None,
                    RepeatKind::System,
                    move |event, _, ddata| process_keyboard_event(event, &my_seat, &handle, ddata),
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
                let my_seat = seat.clone();
                let handle = loop_handle.clone();
                match map_keyboard_repeat(
                    handle.clone(),
                    &seat,
                    None,
                    RepeatKind::System,
                    move |event, _, ddata| process_keyboard_event(event, &my_seat, &handle, ddata),
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

    // the data that will be shared to all callbacks
    let mut data: DData = (env, None, None);

    sctk::WaylandSource::new(queue).quick_insert(event_loop.handle()).unwrap();

    loop {
        match data.1.take() {
            Some(WEvent::Close) => break,
            Some(WEvent::Refresh) => {
                window.refresh();
                window.surface().commit();
            }
            Some(WEvent::Configure { new_size, .. }) => {
                if let Some((w, h)) = new_size {
                    window.resize(w, h);
                    dimensions = (w, h)
                }
                window.refresh();
                redraw(&mut pool, window.surface(), dimensions).expect("Failed to draw");
            }
            None => {}
        }

        // always flush the connection before going to sleep waiting for events
        display.flush().unwrap();

        event_loop.dispatch(None, &mut data).unwrap();
    }
}

fn process_keyboard_event(
    event: KbEvent,
    seat: &wl_seat::WlSeat,
    handle: &LoopHandle<DData>,
    mut ddata: DispatchData,
) {
    let (env, _, opt_source) = ddata.get::<DData>().unwrap();
    if let KbEvent::Key { state, utf8: Some(text), serial, .. } = event {
        if text == "p" && state == KeyState::Pressed {
            // pressed the 'p' key, try to read contents !
            env.with_data_device(seat, |device| {
                device.with_selection(|offer| {
                    let offer = match offer {
                        Some(offer) => offer,
                        None => {
                            println!("No current selection buffer!");
                            return;
                        }
                    };

                    let seat_name =
                        sctk::seat::with_seat_data(seat, |data| data.name.clone()).unwrap();
                    print!("Current selection buffer mime types on seat '{}': [ ", seat_name);
                    let mut has_text = false;
                    offer.with_mime_types(|types| {
                        for t in types {
                            print!("\"{}\", ", t);
                            if t == "text/plain;charset=utf-8" {
                                has_text = true;
                            }
                        }
                    });
                    println!("]");
                    if has_text {
                        println!("Buffer contains text, going to read it...");
                        let reader = offer.receive("text/plain;charset=utf-8".into()).unwrap();
                        let src_handle = handle.clone();
                        let source = handle
                            .insert_source(reader, move |(), file, ddata| {
                                let mut txt = String::new();
                                file.read_to_string(&mut txt).unwrap();
                                println!("Selection contents are: \"{}\"", txt);
                                if let Some(src) = ddata.2.take() {
                                    src_handle.remove(src);
                                }
                            })
                            .unwrap();
                        *opt_source = Some(source);
                    }
                });
            })
            .unwrap();
        }

        if text == "P" && state == KeyState::Pressed {
            env.with_primary_selection(seat, |primary_selection| {
                println!("In primary selection closure");
                primary_selection.with_selection(|offer| {
                    let offer = match offer {
                        Some(offer) => offer,
                        None => {
                            println!("No current primary selection buffer!");
                            return;
                        }
                    };

                    let seat_name =
                        sctk::seat::with_seat_data(seat, |data| data.name.clone()).unwrap();
                    print!(
                        "Current primary selection buffer mime type on seat '{}': [ ",
                        seat_name
                    );

                    let mut has_text = false;
                    offer.with_mime_types(|types| {
                        for t in types {
                            print!("\"{}\", ", t);
                            if t == "text/plain;charset=utf-8" {
                                has_text = true;
                            }
                        }
                    });
                    println!("]");
                    if has_text {
                        println!("Buffer contains text, going to read it...");
                        let reader = offer.receive("text/plain;charset=utf-8".into()).unwrap();
                        let src_handle = handle.clone();
                        let source = handle
                            .insert_source(reader, move |(), file, ddata| {
                                let mut txt = String::new();
                                file.read_to_string(&mut txt).unwrap();
                                println!("Selection contents are: \"{}\"", txt);
                                if let Some(src) = ddata.2.take() {
                                    src_handle.remove(src);
                                }
                            })
                            .unwrap();

                        *opt_source = Some(source);
                    }
                })
            })
            .unwrap()
        }

        if text == "c" && state == KeyState::Pressed {
            let data_source =
                env.new_data_source(vec!["text/plain;charset=utf-8".into()], move |event, _| {
                    if let DataSourceEvent::Send { mut pipe, .. } = event {
                        let contents = "Hello from clipboard";
                        println!("Setting clipboard to: {}", &contents);
                        write!(pipe, "{}", contents).unwrap();
                    }
                });

            env.with_data_device(seat, |device| {
                println!("Set selection source");
                device.set_selection(&Some(data_source), serial);
            })
            .unwrap();
        }

        if text == "C" && state == KeyState::Pressed {
            let data_source = env.new_primary_selection_source(
                vec!["text/plain;charset=utf-8".into()],
                move |event, _| {
                    if let PrimarySelectionSourceEvent::Send { mut pipe, .. } = event {
                        let contents = "Hello from primary selection";
                        println!("Setting clipboard primary clipboard to {}", &contents);
                        write!(pipe, "{}", contents).unwrap();
                    }
                },
            );

            env.with_primary_selection(seat, |device| {
                println!("Set primary selection source");
                device.set_selection(&Some(data_source), serial);
            })
            .unwrap();
        }
    }
}

fn redraw(
    pool: &mut AutoMemPool,
    surface: &wl_surface::WlSurface,
    (buf_x, buf_y): (u32, u32),
) -> Result<(), ::std::io::Error> {
    let (canvas, new_buffer) =
        pool.buffer(buf_x as i32, buf_y as i32, 4 * buf_x as i32, wl_shm::Format::Argb8888)?;
    for dst_pixel in canvas.chunks_exact_mut(4) {
        dst_pixel[0] = 0x00;
        dst_pixel[1] = 0x00;
        dst_pixel[2] = 0x00;
        dst_pixel[3] = 0xFF;
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
