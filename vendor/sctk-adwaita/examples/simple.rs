extern crate smithay_client_toolkit as sctk;

use sctk::reexports::calloop;
use sctk::reexports::client::protocol::{wl_shm, wl_surface};
use sctk::shm::AutoMemPool;
use sctk::window::Event as WEvent;

sctk::default_environment!(KbdInputExample, desktop);

fn main() {
    let (env, display, queue) = sctk::new_default_environment!(KbdInputExample, desktop)
        .expect("Unable to connect to a Wayland compositor");

    let mut event_loop = calloop::EventLoop::<Option<WEvent>>::try_new().unwrap();

    let mut dimensions = (380u32, 240u32);

    let surface = env.create_surface().detach();

    let mut window = env
        .create_window::<sctk_adwaita::AdwaitaFrame, _>(
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

    window.set_title("/usr/lib/xorg/modules/input".to_string());

    // // uncomment to override automatic theme selection
    // window.set_frame_config(sctk_adwaita::FrameConfig::light());

    let mut pool = env
        .create_auto_pool()
        .expect("Failed to create a memory pool !");

    if !env.get_shell().unwrap().needs_configure() {
        // initial draw to bootstrap on wl_shell
        redraw(&mut pool, window.surface(), dimensions).expect("Failed to draw");
        window.refresh();
    }

    println!("XDG_WINDOW: {:?}", window.surface());

    let mut next_action = None;

    sctk::WaylandSource::new(queue)
        .quick_insert(event_loop.handle())
        .unwrap();

    loop {
        match next_action.take() {
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

        event_loop.dispatch(None, &mut next_action).unwrap();
    }
}

fn redraw(
    pool: &mut AutoMemPool,
    surface: &wl_surface::WlSurface,
    (buf_x, buf_y): (u32, u32),
) -> Result<(), ::std::io::Error> {
    let (canvas, new_buffer) = pool.buffer(
        buf_x as i32,
        buf_y as i32,
        4 * buf_x as i32,
        wl_shm::Format::Argb8888,
    )?;

    for pixel in canvas.chunks_exact_mut(4) {
        pixel[0] = 255;
        pixel[1] = 255;
        pixel[2] = 255;
        pixel[3] = 255;
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
