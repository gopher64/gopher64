extern crate x11rb;

use x11rb::connection::Connection;
use x11rb::cursor::Handle as CursorHandle;
use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;
use x11rb::resource_manager::new_from_default;
use x11rb::wrapper::ConnectionExt as _;

fn main() {
    let (conn, screen_num) = x11rb::connect(None).unwrap();

    // The following is only needed for start_timeout_thread(), which is used for 'tests'
    let conn1 = std::sync::Arc::new(conn);
    let conn = &*conn1;

    let screen = &conn.setup().roots[screen_num];
    let win_id = conn.generate_id().unwrap();
    let gc_id = conn.generate_id().unwrap();
    let resource_db = new_from_default(conn).unwrap();
    let cursor_handle = CursorHandle::new(conn, screen_num, &resource_db).unwrap();

    let wm_protocols = conn.intern_atom(false, b"WM_PROTOCOLS").unwrap();
    let wm_delete_window = conn.intern_atom(false, b"WM_DELETE_WINDOW").unwrap();
    let net_wm_name = conn.intern_atom(false, b"_NET_WM_NAME").unwrap();
    let utf8_string = conn.intern_atom(false, b"UTF8_STRING").unwrap();
    let wm_protocols = wm_protocols.reply().unwrap().atom;
    let wm_delete_window = wm_delete_window.reply().unwrap().atom;
    let net_wm_name = net_wm_name.reply().unwrap().atom;
    let utf8_string = utf8_string.reply().unwrap().atom;
    let cursor_handle = cursor_handle.reply().unwrap();

    let win_aux = CreateWindowAux::new()
        .event_mask(EventMask::EXPOSURE | EventMask::STRUCTURE_NOTIFY | EventMask::NO_EVENT)
        .background_pixel(screen.white_pixel)
        .win_gravity(Gravity::NORTH_WEST)
        // Just because, we set the cursor to "wait"
        .cursor(cursor_handle.load_cursor(conn, "wait").unwrap());

    let gc_aux = CreateGCAux::new().foreground(screen.black_pixel);

    let (mut width, mut height) = (100, 100);

    conn.create_window(
        screen.root_depth,
        win_id,
        screen.root,
        0,
        0,
        width,
        height,
        0,
        WindowClass::INPUT_OUTPUT,
        0,
        &win_aux,
    )
    .unwrap();

    util::start_timeout_thread(conn1.clone(), win_id);

    let title = "Simple Window";
    conn.change_property8(
        PropMode::REPLACE,
        win_id,
        AtomEnum::WM_NAME,
        AtomEnum::STRING,
        title.as_bytes(),
    )
    .unwrap();
    conn.change_property8(
        PropMode::REPLACE,
        win_id,
        net_wm_name,
        utf8_string,
        title.as_bytes(),
    )
    .unwrap();
    conn.change_property32(
        PropMode::REPLACE,
        win_id,
        wm_protocols,
        AtomEnum::ATOM,
        &[wm_delete_window],
    )
    .unwrap();
    conn.change_property8(
        PropMode::REPLACE,
        win_id,
        AtomEnum::WM_CLASS,
        AtomEnum::STRING,
        b"simple_window\0simple_window\0",
    )
    .unwrap();
    conn.change_property8(
        PropMode::REPLACE,
        win_id,
        AtomEnum::WM_CLIENT_MACHINE,
        AtomEnum::STRING,
        // Text encoding in X11 is complicated. Let's use UTF-8 and hope for the best.
        gethostname::gethostname()
            .to_str()
            .unwrap_or("[Invalid]")
            .as_bytes(),
    )
    .unwrap();

    let reply = conn
        .get_property(false, win_id, AtomEnum::WM_NAME, AtomEnum::STRING, 0, 1024)
        .unwrap();
    let reply = reply.reply().unwrap();
    assert_eq!(reply.value, title.as_bytes());

    conn.create_gc(gc_id, win_id, &gc_aux).unwrap();

    conn.map_window(win_id).unwrap();

    conn.flush().unwrap();

    loop {
        let event = conn.wait_for_event().unwrap();
        println!("{:?})", event);
        match event {
            Event::Expose(event) => {
                if event.count == 0 {
                    // There already is a white background because we set background_pixel to white
                    // when creating the window.
                    let (width, height): (i16, i16) = (width as _, height as _);
                    let points = [
                        Point {
                            x: width,
                            y: height,
                        },
                        Point { x: -10, y: -10 },
                        Point {
                            x: -10,
                            y: height + 10,
                        },
                        Point {
                            x: width + 10,
                            y: -10,
                        },
                    ];
                    conn.poly_line(CoordMode::ORIGIN, win_id, gc_id, &points)
                        .unwrap();
                    conn.flush().unwrap();
                }
            }
            Event::ConfigureNotify(event) => {
                width = event.width;
                height = event.height;
            }
            Event::ClientMessage(event) => {
                let data = event.data.as_data32();
                if event.format == 32 && event.window == win_id && data[0] == wm_delete_window {
                    println!("Window was asked to close");
                    return;
                }
            }
            Event::Error(_) => println!("Got an unexpected error"),
            _ => println!("Got an unknown event"),
        }
    }
}

include!("integration_test_util/util.rs");
