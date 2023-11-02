extern crate image;
extern crate smithay_client_toolkit as sctk;

use std::env;

use sctk::reexports::client::protocol::{wl_shm, wl_surface};
use sctk::shm::AutoMemPool;
use sctk::window::{Event as WEvent, FallbackFrame, State};

sctk::default_environment!(ImViewerExample, desktop);

fn main() {
    // First of all, retrieve the path from the program arguments:
    let path = match env::args_os().nth(1) {
        Some(p) => p,
        None => {
            println!("USAGE: ./image_wiewer <PATH>");
            return;
        }
    };
    // now, try to open the image
    // the image crate will take care of auto-detecting the file format
    let image = match image::open(&path) {
        Ok(i) => i,
        Err(e) => {
            println!("Failed to open image {}.", path.to_string_lossy());
            println!("Error was: {:?}", e);
            return;
        }
    };
    // We'll need the image in RGBA for drawing it
    let image = image.to_rgba8();

    /*
     * Initalize the wayland connection
     */
    let (env, _display, mut queue) = sctk::new_default_environment!(ImViewerExample, desktop)
        .expect("Unable to connect to a Wayland compositor");

    // Use the compositor global to create a new surface
    let surface = env
        .create_surface_with_scale_callback(|dpi, _surface, _dispatch_data| {
            println!("dpi changed to {}", dpi);
        })
        .detach();

    /*
     * Init the window
     */

    // First of all, this Option<WEvent> will store
    // any event from the window that we'll need to process. We
    // store them and will process them later in the event loop
    // rather that process them directly because in a batch of
    // generated events, often only the last one needs to actually
    // be processed, and some events may render other obsoletes.
    // See the closure a few lines below for details
    let mut next_action = None::<WEvent>;

    // Now we actually create the window. The type parameter `ConceptFrame` here
    // specifies the type we want to use to draw the borders. To create your own
    // decorations you just need an object to implement the `Frame` trait.
    let mut window = env
        .create_window::<FallbackFrame, _>(
            surface,            // the wl_surface that serves as the basis of this window
            None,               // None for theme_manager, since we don't theme pointer outself
            image.dimensions(), // the initial internal dimensions of the window
            move |evt, mut dispatch_data| {
                // This is the closure that process the Window events.
                // There are 3 possible events:
                // - Close: the user requested the window to be closed, we'll then quit
                // - Configure: the server suggested a new state for the window (possibly
                //   a new size if a resize is in progress). We'll likely need to redraw
                //   our contents
                // - Refresh: the frame itself needs to be redrawn. SCTK does not do this
                //   automatically because it has a cost and should only be done in periods
                //   of the event loop where the client actually wants to draw
                // Here we actually only keep the last event receive according to a priority
                // order of Close > Configure > Refresh.
                // Indeed, if we received a Close, there is not point drawing anything more as
                // we will exit. A new Configure overrides a previous one, and if we received
                // a Configure we will refresh the frame anyway.

                // We access the next_action Option via the dispatch_data provided by wayland-rs.
                let next_action = dispatch_data.get::<Option<WEvent>>().unwrap();
                // Check if we need to replace the old event by the new one
                let replace = matches!(
                    (&evt, &*next_action),
                    // replace if there is no old event
                    (_, &None)
                    // or the old event is refresh
                    | (_, &Some(WEvent::Refresh))
                    // or we had a configure and received a new one
                    | (&WEvent::Configure { .. }, &Some(WEvent::Configure { .. }))
                    // or the new event is close
                    | (&WEvent::Close, _)
                );
                if replace {
                    *next_action = Some(evt);
                }
            },
            // creating the window may fail if the code drawing the frame
            // fails to initialize itself. For ConceptFrame this should not happen
            // unless the system is utterly broken, though.
        )
        .expect("Failed to create a window !");

    // Setting the windows title allows the compositor to know what your
    // window should be called and the title will be display on the header bar
    // of the windows decorations
    window.set_title("Image Viewer".to_string());

    /*
     * Initialization of the memory pool
     */
    let mut pool = env.create_auto_pool().expect("Failed to create the memory pool.");

    /*
     * Event Loop preparation and running
     */

    // First, we initialize a few boolean flags that we'll use to track our state:
    // - the window needs to be redrawn
    let mut need_redraw = false;
    // - are we currently in the process of being resized? (to draw the image or
    //   black content)
    let mut resizing = false;
    // - the size of our contents
    let mut dimensions = image.dimensions();

    // if our shell does not need to wait for a configure event, we draw right away.
    //
    // Note that this is only the case for the old wl_shell protocol, which is now
    // deprecated. This code is only for compatibility with old server that do not
    // support the new standard xdg_shell protocol.
    //
    // But if we have fallbacked to wl_shell, we need to draw right away because we'll
    // never receive a configure event if we don't draw something...
    if !env.get_shell().unwrap().needs_configure() {
        // initial draw to bootstrap on wl_shell
        redraw(&mut pool, window.surface(), dimensions, if resizing { None } else { Some(&image) })
            .expect("Failed to draw");
        window.refresh();
    }

    // We can now actually enter the event loop!
    loop {
        // First, check if any pending action was received by the
        // Window implementation:
        match next_action.take() {
            // We received a Close event, just break from the loop
            // and let the app quit
            Some(WEvent::Close) => break,
            // We receive a Refresh event, store that we need to refresh the
            // frame
            Some(WEvent::Refresh) => {
                window.refresh();
                window.surface().commit();
            }
            // We received a configure event, our action depends on its
            // contents
            Some(WEvent::Configure { new_size, states }) => {
                // the configure event contains a suggested size,
                // if it is different from our current size, we need to
                // update it and redraw
                if let Some((w, h)) = new_size {
                    if dimensions != (w, h) {
                        dimensions = (w, h);
                    }
                }
                window.resize(dimensions.0, dimensions.1);
                window.refresh();
                // Are we currently resizing ?
                // We check if a resizing just started or stopped,
                // because in this case we'll swap between drawing black
                // and drawing the window (or the reverse), and thus we need to
                // redraw
                let new_resizing = states.contains(&State::Resizing);
                resizing = new_resizing;

                need_redraw = true;
            }
            // No event, nothing new to do.
            None => {}
        }

        if need_redraw {
            // We don't need to redraw or refresh anymore =)
            need_redraw = false;
            redraw(
                &mut pool,
                window.surface(),
                dimensions,
                if resizing { None } else { Some(&image) },
            )
            .expect("Failed to draw")
        }

        // Finally, dispatch the event queue. This method blocks until a message
        // sends all our request to the server, then blocks until an event arrives
        // from it. It then processes all events by calling the implementation of
        // the target object for each, and only return once all pending messages
        // have been processed.
        queue.dispatch(&mut next_action, |_, _, _| {}).unwrap();
    }
}

// The draw function, which drawn `base_image` in the provided `MemPool`,
// at given dimensions.
//
// If `base_image` is `None`, it'll just draw black contents. This is to
// improve performance during resizing: we need to redraw the window frequently
// so that its dimensions follow the pointer during the resizing, but resizing the
// image is costly and long. So during an interactive resize of the window we'll
// just draw black contents to not feel laggy.
fn redraw(
    pool: &mut AutoMemPool,
    surface: &wl_surface::WlSurface,
    (buf_x, buf_y): (u32, u32),
    base_image: Option<&image::ImageBuffer<image::Rgba<u8>, Vec<u8>>>,
) -> Result<(), ::std::io::Error> {
    // We allocate a new buffer from the memory pool with the appropriate dimensions
    // This function automatically finds an unused space of the correct size in the memory
    // pool and returns it as a `&mut [u8]`, as well as a `wl_buffer` matching it.
    let (canvas, new_buffer) = pool.buffer(
        buf_x as i32,     // width of the buffer, in pixels
        buf_y as i32,     // height of the buffer, in pixels
        4 * buf_x as i32, // stride: number of bytes between the start of two
        //   consecutive rows of pixels
        wl_shm::Format::Argb8888, // the pixel format we wrote in
    )?;

    if let Some(base_image) = base_image {
        // We have an image to draw

        // first, resize it to the requested size. We just use the function provided
        // by the image crate here.
        let image =
            image::imageops::resize(base_image, buf_x, buf_y, image::imageops::FilterType::Nearest);

        // Now, we'll write the pixels of the image to the MemPool.
        //
        // We do this in an horribly inefficient manner, for the sake of simplicity.
        // We'll send pixels to the server in ARGB8888 format (this is one of the only
        // formats that are guaranteed to be supported), but image provides it in
        // RGBA8888, so we need to do the conversion.
        //
        // Additionally, if the image has some transparent parts, we'll blend them into
        // a white background, otherwise the server will draw our window with a
        // transparent background!
        for (src_pixel, dst_pixel) in image.pixels().zip(canvas.chunks_exact_mut(4)) {
            // retrieve the pixel values
            let r = src_pixel.0[0] as u32;
            let g = src_pixel.0[1] as u32;
            let b = src_pixel.0[2] as u32;
            let a = src_pixel.0[3] as u32;
            // blend them
            let r = ::std::cmp::min(0xFF, (0xFF * (0xFF - a) + a * r) / 0xFF);
            let g = ::std::cmp::min(0xFF, (0xFF * (0xFF - a) + a * g) / 0xFF);
            let b = ::std::cmp::min(0xFF, (0xFF * (0xFF - a) + a * b) / 0xFF);
            // write the pixel
            let pixel: [u8; 4] = ((0xFF << 24) + (r << 16) + (g << 8) + b).to_ne_bytes();
            dst_pixel[0] = pixel[0];
            dst_pixel[1] = pixel[1];
            dst_pixel[2] = pixel[2];
            dst_pixel[3] = pixel[3];
        }
    } else {
        // We do not have any image to draw, so we draw black contents
        for dst_pixel in canvas.chunks_exact_mut(4) {
            dst_pixel[0] = 0x00;
            dst_pixel[1] = 0x00;
            dst_pixel[2] = 0x00;
            dst_pixel[3] = 0xFF;
        }
    }

    surface.attach(Some(&new_buffer), 0, 0);
    // damage the surface so that the compositor knows it needs to redraw it
    if surface.as_ref().version() >= 4 {
        // If our server is recent enough and supports at least version 4 of the
        // wl_surface interface, we can specify the damage in buffer coordinates.
        // This is obviously the best and do that if possible.
        surface.damage_buffer(0, 0, buf_x as i32, buf_y as i32);
    } else {
        // Otherwise, we fallback to compatilibity mode. Here we specify damage
        // in surface coordinates, which would have been different if we had drawn
        // our buffer at HiDPI resolution. We didn't though, so it is ok.
        // Using `damage_buffer` in general is better though.
        surface.damage(0, 0, buf_x as i32, buf_y as i32);
    }
    surface.commit();
    Ok(())
}
