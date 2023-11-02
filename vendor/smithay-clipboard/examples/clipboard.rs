use std::io::{BufWriter, Seek, SeekFrom, Write};

use sctk::seat;
use sctk::seat::keyboard::{self, Event as KeyboardEvent, KeyState, RepeatKind};
use sctk::shm::MemPool;
use sctk::window::{Event as WindowEvent, FallbackFrame};

use sctk::reexports::client::protocol::wl_shm;
use sctk::reexports::client::protocol::wl_surface::WlSurface;

use smithay_clipboard::Clipboard;

sctk::default_environment!(ClipboardExample, desktop);

/// Our dispatch data for simple clipboard access and processing frame events.
struct DispatchData {
    /// Pending event from SCTK to update window.
    pub pending_frame_event: Option<WindowEvent>,
    /// Clipboard handler.
    pub clipboard: Clipboard,
}

impl DispatchData {
    fn new(clipboard: Clipboard) -> Self {
        Self { pending_frame_event: None, clipboard }
    }
}

fn main() {
    // Setup default desktop environment
    let (env, display, queue) = sctk::new_default_environment!(ClipboardExample, desktop)
        .expect("unable to connect to a Wayland compositor.");

    // Create event loop
    let mut event_loop = sctk::reexports::calloop::EventLoop::<DispatchData>::try_new().unwrap();

    // Initial window dimentions
    let mut dimentions = (320u32, 240u32);

    // Create surface
    let surface = env.create_surface().detach();

    // Create window
    let mut window = env
        .create_window::<FallbackFrame, _>(
            surface,
            None,
            dimentions,
            move |event, mut dispatch_data| {
                // Get our dispath data
                let dispatch_data = dispatch_data.get::<DispatchData>().unwrap();

                // Keep last event in priority order : Close > Configure > Refresh
                let should_replace_event = match (&event, &dispatch_data.pending_frame_event) {
                    (_, &None)
                    | (_, &Some(WindowEvent::Refresh))
                    | (&WindowEvent::Configure { .. }, &Some(WindowEvent::Configure { .. }))
                    | (&WindowEvent::Close, _) => true,
                    _ => false,
                };

                if should_replace_event {
                    dispatch_data.pending_frame_event = Some(event);
                }
            },
        )
        .expect("failed to create a window.");

    // Set title and app id
    window.set_title(String::from("smithay-clipboard example. Press C/P to copy/paste"));
    window.set_app_id(String::from("smithay-clipboard-example"));

    // Create memory pool
    let mut pools = env.create_double_pool(|_| {}).expect("failed to create a memory pool.");

    // Structure to track seats
    let mut seats = Vec::new();

    // Process existing seats
    for seat in env.get_all_seats() {
        let seat_data = match seat::with_seat_data(&seat, |seat_data| seat_data.clone()) {
            Some(seat_data) => seat_data,
            _ => continue,
        };

        if seat_data.has_keyboard && !seat_data.defunct {
            // Suply event_loop's handle to handle key repeat
            let event_loop_handle = event_loop.handle();

            // Map keyboard for exising seats
            let keyboard_mapping_result = keyboard::map_keyboard_repeat(
                event_loop_handle,
                &seat,
                None,
                RepeatKind::System,
                move |event, _, mut dispatch_data| {
                    let dispatch_data = dispatch_data.get::<DispatchData>().unwrap();
                    process_keyboard_event(event, dispatch_data);
                },
            );

            // Insert repeat rate handling source
            match keyboard_mapping_result {
                Ok(keyboard) => {
                    seats.push((seat.detach(), Some(keyboard)));
                }
                Err(err) => {
                    eprintln!("Failed to map keyboard on seat {:?} : {:?}", seat_data.name, err);
                    seats.push((seat.detach(), None));
                }
            }
        } else {
            // Handle seats without keyboard, since they can gain keyboard later
            seats.push((seat.detach(), None));
        }
    }

    // Implement event listener for seats to handle capability change, etc
    let event_loop_handle = event_loop.handle();
    let _listener = env.listen_for_seats(move |seat, seat_data, _| {
        // find the seat in the vec of seats or insert it
        let idx = seats.iter().position(|(st, _)| st == &seat.detach());
        let idx = idx.unwrap_or_else(|| {
            seats.push((seat.detach(), None));
            seats.len() - 1
        });

        let (_, mapped_keyboard) = &mut seats[idx];

        if seat_data.has_keyboard && !seat_data.defunct {
            // Map keyboard if it's not mapped already
            if mapped_keyboard.is_none() {
                let keyboard_mapping_result = keyboard::map_keyboard_repeat(
                    event_loop_handle.clone(),
                    &seat,
                    None,
                    RepeatKind::System,
                    move |event, _, mut dispatch_data| {
                        let dispatch_data = dispatch_data.get::<DispatchData>().unwrap();
                        process_keyboard_event(event, dispatch_data);
                    },
                );

                // Insert repeat rate source
                match keyboard_mapping_result {
                    Ok(keyboard) => {
                        *mapped_keyboard = Some(keyboard);
                    }
                    Err(err) => {
                        eprintln!("Failed to map keyboard on seat {} : {:?}", seat_data.name, err);
                    }
                }
            }
        } else if let Some(keyboard) = mapped_keyboard.take() {
            if keyboard.as_ref().version() >= 3 {
                keyboard.release();
            }
        }
    });

    if !env.get_shell().unwrap().needs_configure() {
        if let Some(pool) = pools.pool() {
            draw(pool, window.surface().clone(), dimentions).expect("failed to draw.")
        }
        // Refresh our frame
        window.refresh();
    }

    sctk::WaylandSource::new(queue).quick_insert(event_loop.handle()).unwrap();

    let clipboard = unsafe { Clipboard::new(display.get_display_ptr() as *mut _) };
    let mut dispatch_data = DispatchData::new(clipboard);

    loop {
        if let Some(frame_event) = dispatch_data.pending_frame_event.take() {
            match frame_event {
                WindowEvent::Close => break,
                WindowEvent::Refresh => {
                    window.refresh();
                    window.surface().commit();
                }
                WindowEvent::Configure { new_size, .. } => {
                    if let Some((w, h)) = new_size {
                        window.resize(w, h);
                        dimentions = (w, h)
                    }
                    window.refresh();
                    if let Some(pool) = pools.pool() {
                        draw(pool, window.surface().clone(), dimentions).expect("failed to draw.")
                    }
                }
            }
        }

        display.flush().unwrap();

        event_loop.dispatch(None, &mut dispatch_data).unwrap();
    }
}

fn process_keyboard_event(event: KeyboardEvent, dispatch_data: &mut DispatchData) {
    let text = match event {
        KeyboardEvent::Key { state, utf8: Some(text), .. } if state == KeyState::Pressed => text,
        KeyboardEvent::Repeat { utf8: Some(text), .. } => text,
        _ => return,
    };

    match text.as_str() {
        // Paste primary.
        "P" => {
            let contents = dispatch_data
                .clipboard
                .load_primary()
                .unwrap_or_else(|_| String::from("Failed to load primary selection"));
            println!("Paste from primary clipboard: {}", contents);
        }
        // Paste.
        "p" => {
            let contents = dispatch_data
                .clipboard
                .load()
                .unwrap_or_else(|_| String::from("Failed to load selection"));
            println!("Paste: {}", contents);
        }
        // Copy primary.
        "C" => {
            let text = String::from("Copy primary");
            dispatch_data.clipboard.store_primary(text.clone());
            println!("Copied string into primary selection buffer: {}", text);
        }
        // Copy.
        "c" => {
            let text = String::from("Copy");
            dispatch_data.clipboard.store(text.clone());
            println!("Copied string: {}", text);
        }
        _ => (),
    }
}

fn draw(
    pool: &mut MemPool,
    surface: WlSurface,
    dimensions: (u32, u32),
) -> Result<(), std::io::Error> {
    pool.resize((4 * dimensions.0 * dimensions.1) as usize).expect("failed to resize memory pool");

    {
        pool.seek(SeekFrom::Start(0))?;
        let mut writer = BufWriter::new(&mut *pool);
        for _ in 0..dimensions.0 * dimensions.1 {
            // ARGB color written in LE, so it's #FF1C1C1C
            writer.write_all(&[0x1c, 0x1c, 0x1c, 0xff])?;
        }
        writer.flush()?;
    }

    let new_buffer = pool.buffer(
        0,
        dimensions.0 as i32,
        dimensions.1 as i32,
        4 * dimensions.0 as i32,
        wl_shm::Format::Argb8888,
    );
    surface.attach(Some(&new_buffer), 0, 0);
    surface.commit();

    Ok(())
}
