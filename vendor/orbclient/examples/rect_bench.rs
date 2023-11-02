// SPDX-License-Identifier: MIT

use orbclient::{Color, EventOption, Renderer, Window};

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
    let mut window = Window::new(10, 10, 800, 600, "RECTANGLE BENCHMARK").unwrap();

    time!("set", { window.set(Color::rgb(255, 255, 255)) });

    time!("rect 400x400", {
        window.rect(0, 0, 400, 400, Color::rgb(0, 0, 255))
    });

    time!("rect 200x200", {
        window.rect(0, 0, 200, 200, Color::rgb(0, 255, 0))
    });

    time!("rect 100x100", {
        window.rect(0, 0, 100, 100, Color::rgb(255, 0, 0))
    });

    time!("sync", {
        window.sync();
    });

    'events: loop {
        for event in window.events() {
            #[allow(clippy::single_match)]
            match event.to_option() {
                EventOption::Quit(_quit_event) => break 'events,
                _ => (),
            }
        }
    }
}
