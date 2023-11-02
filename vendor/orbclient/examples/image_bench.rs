// SPDX-License-Identifier: MIT

use orbclient::{Color, EventOption, Renderer, Window};

const TIMES: usize = 10;

macro_rules! time {
    ($msg:tt, $block: block) => ({
        let _time_instant = ::std::time::Instant::now();
        $block
        let _time_duration = _time_instant.elapsed();
        let _time_fractional = _time_duration.as_secs() as f64
                             + (_time_duration.subsec_nanos() as f64)/1000000000.0;
        println!(
            "{}: {} ms",
            $msg,
            _time_fractional * 1000.0
        );
    });
}

fn main() {
    //let (width, height) = orbclient::get_display_size().unwrap();

    let mut window = Window::new(10, 10, 800, 600, "IMAGE BENCHMARK").unwrap();

    window.set(Color::rgb(255, 255, 255));

    //create image data : a green square
    let data = vec![Color::rgba(100, 200, 10, 20); 412500];
    let data2 = vec![Color::rgba(200, 100, 10, 20); 412500];
    let data3 = vec![Color::rgba(10, 100, 100, 20); 412500];
    let data4 = vec![Color::rgba(10, 100, 200, 20); 480000];

    //draw image benchmarking
    println!("Benchmarking implementations to draw an image on window:");

    time!("image_legacy", {
        for _i in 0..TIMES {
            window.image_legacy(15, 15, 750, 550, &data[..]);
        }
    });

    time!("image_fast", {
        for _i in 0..TIMES {
            window.image_fast(20, 20, 750, 550, &data2[..]);
        }
    });

    time!("image_opaque", {
        for _i in 0..TIMES {
            window.image_opaque(50, 50, 750, 550, &data3[..]);
        }
    });

    time!("image_over", {
        for _i in 0..TIMES {
            window.image_over(50, &data4[..360000]);
        }
    });

    println!("------------------------------------------------");

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
