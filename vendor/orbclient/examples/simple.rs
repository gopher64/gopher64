// SPDX-License-Identifier: MIT

use orbclient::{Color, EventOption, GraphicsPath, Mode, Renderer, Window};

fn main() {
    let (width, height) = orbclient::get_display_size().unwrap();

    let mut window = Window::new(
        (width as i32) / 4,
        (height as i32) / 4,
        width / 2,
        height / 2,
        "TITLE",
    )
    .unwrap();

    let (win_w, win_h) = (width / 2, height / 2);

    // top left -> bottom rigth
    window.linear_gradient(
        0,
        0,
        win_w / 3,
        win_h,
        0,
        0,
        (win_w / 3) as i32,
        (win_h / 2) as i32,
        Color::rgb(128, 128, 128),
        Color::rgb(255, 255, 255),
    );
    // horizontal gradient
    window.linear_gradient(
        (win_w / 3) as i32,
        0,
        win_w / 3,
        win_h,
        (win_w / 3) as i32,
        0,
        (2 * win_w / 3) as i32,
        0,
        Color::rgb(128, 255, 255),
        Color::rgb(255, 255, 255),
    );
    // vertical gradient
    window.linear_gradient(
        (2 * win_w / 3) as i32,
        0,
        win_w / 3,
        win_h,
        (2 * win_w / 3) as i32,
        0,
        (2 * win_w / 3) as i32,
        win_h as i32,
        Color::rgb(0, 128, 0),
        Color::rgb(255, 255, 255),
    );
    window.arc(100, 100, -25, 1 << 0 | 1 << 2, Color::rgb(0, 0, 255));
    window.arc(100, 100, -25, 1 << 1 | 1 << 3, Color::rgb(0, 255, 255));
    window.arc(100, 100, -25, 1 << 4 | 1 << 6, Color::rgb(255, 0, 255));
    window.arc(100, 100, -25, 1 << 5 | 1 << 7, Color::rgb(255, 255, 0));
    window.circle(100, 100, 25, Color::rgb(0, 0, 0));
    window.circle(100, 101, -25, Color::rgb(0, 255, 0));
    window.circle(220, 220, -100, Color::rgba(128, 128, 128, 80));
    window.wu_circle(150, 220, 100, Color::rgba(255, 0, 0, 255));
    window.line(0, 0, 200, 200, Color::rgb(255, 0, 0));
    window.line(0, 200, 200, 0, Color::rgb(128, 255, 0));
    // vertical and horizontal line test
    window.line(100, 0, 100, 200, Color::rgb(0, 0, 255));
    window.line(0, 100, 200, 100, Color::rgb(255, 255, 0));
    window.wu_line(100, 220, 400, 250, Color::rgba(255, 0, 0, 255));
    window.line(100, 230, 400, 260, Color::rgba(255, 0, 0, 255));

    // path and bezier curve example draw a cloud
    let mut cloud_path = GraphicsPath::new();
    cloud_path.move_to(170, 80);
    cloud_path.bezier_curve_to(130, 100, 130, 150, 230, 150);
    cloud_path.bezier_curve_to(250, 180, 320, 180, 340, 150);
    cloud_path.bezier_curve_to(420, 150, 420, 120, 390, 100);
    cloud_path.bezier_curve_to(430, 40, 370, 30, 340, 50);
    cloud_path.bezier_curve_to(320, 5, 250, 20, 250, 50);
    cloud_path.bezier_curve_to(200, 5, 150, 20, 170, 80);
    window.draw_path_stroke(cloud_path, Color::rgb(0, 0, 255));

    // path and quadratic curve example draw a balloon
    let mut balloon_path = GraphicsPath::new();
    balloon_path.move_to(75, 25);
    balloon_path.quadratic_curve_to(25, 25, 25, 62);
    balloon_path.quadratic_curve_to(25, 100, 50, 100);
    balloon_path.quadratic_curve_to(50, 120, 30, 125);
    balloon_path.quadratic_curve_to(60, 120, 65, 100);
    balloon_path.quadratic_curve_to(125, 100, 125, 62);
    balloon_path.quadratic_curve_to(125, 25, 75, 25);
    window.draw_path_stroke(balloon_path, Color::rgb(0, 0, 255));

    window.char(200, 200, '═', Color::rgb(0, 0, 0));
    window.char(208, 200, '═', Color::rgb(0, 0, 0));

    // testing for non existent x,y position : does not panic but returns Color(0,0,0,0)
    let _non_existent_pixel = window.getpixel(width as i32 + 10, height as i32 + 10);

    // testing PartialEq for Color
    if Color::rgb(11, 2, 3) == Color::rgba(1, 2, 3, 100) {
        println!("Testing colors: they are the same!")
    } else {
        println!("Testing colors: they are NOT the same!")
    }

    //Draw a transparent rectangle over window content
    // default mode is Blend
    window.rect(250, 200, 80, 80, Color::rgba(100, 100, 100, 100));

    //Draw an opaque rectangle replacing window content
    window.mode().set(Mode::Overwrite); // set window drawing mode to Overwrite from now on
    window.rect(300, 220, 80, 80, Color::rgb(100, 100, 100));

    //Draw a hole in the window replacing alpha channel (Only in Orbital, not in SDL2)
    window.rect(300, 100, 80, 80, Color::rgba(10, 10, 10, 1));

    //Draw a transparent rectangle over window content
    window.mode().set(Mode::Blend); //set mode to Blend fron now on
    window.rect(200, 230, 80, 80, Color::rgba(100, 100, 100, 100));

    //Draw a blured box over window content
    window.box_blur(170, 100, 150, 150, 10);

    //Draw a shadow around a box
    window.box_shadow(170, 100, 150, 150, 0, 0, 20, Color::rgba(0, 0, 0, 255));

    window.sync();

    'events: loop {
        for event in window.events() {
            match event.to_option() {
                EventOption::Quit(_quit_event) => break 'events,
                EventOption::Mouse(evt) => println!(
                    "At position {:?} pixel color is : {:?}",
                    (evt.x, evt.y),
                    window.getpixel(evt.x, evt.y)
                ),
                event_option => println!("{:?}", event_option),
            }
        }
    }
}
