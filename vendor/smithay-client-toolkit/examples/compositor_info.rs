extern crate smithay_client_toolkit as sctk;

use sctk::shell::Shell;

// This is a small program that queries the compositor for
// various information and prints them on the console before exiting.

sctk::default_environment!(CompInfo, desktop);

fn main() -> Result<(), ()> {
    let (env, _display, _queue) = sctk::new_default_environment!(CompInfo, desktop)
        .expect("Unable to connect to a Wayland compositor");

    println!("== Smithay's compositor info tool ==\n");

    // print the best supported shell
    println!(
        "-> Most recent shell supported by the compositor is {}.",
        match env.get_shell() {
            Some(Shell::Wl(_)) => "the legacy wl_shell",
            Some(Shell::Zxdg(_)) => "the old unstable xdg_shell (zxdg_shell_v6)",
            Some(Shell::Xdg(_)) => "the current xdg_shell",
            None => "nothing",
        }
    );
    println!();

    // print the outputs
    let outputs = env.get_all_outputs();
    println!("-> Compositor advertised {} outputs:", outputs.len());
    for output in outputs {
        sctk::output::with_output_info(&output, |info| {
            println!(
                "  -> #{}: {} ({}), with scale factor of {}",
                info.id, info.model, info.make, info.scale_factor
            );
            println!("     Possible modes are:");
            for mode in &info.modes {
                println!(
                    "     -> [{}{}] {} x {} @ {}.{} Hz",
                    if mode.is_preferred { "p" } else { " " },
                    if mode.is_current { "c" } else { " " },
                    mode.dimensions.0,
                    mode.dimensions.1,
                    mode.refresh_rate / 1000,
                    mode.refresh_rate % 1000
                );
            }
        });
    }
    println!();

    // print the seats
    let seats = env.get_all_seats();
    println!("-> Compositor advertised {} seats:", seats.len());
    for seat in seats {
        sctk::seat::with_seat_data(&seat, |data| {
            print!("  -> {} with capabilities: ", data.name);
            if data.has_pointer {
                print!("pointer ");
            }
            if data.has_keyboard {
                print!("keyboard ");
            }
            if data.has_touch {
                print!("touch ");
            }
            println!();
        });
    }

    /*
        if env.decorations_mgr.is_some() {
            println!("-> Compositor supports server-side decorations.")
        } else {
            println!("-> Compositor does not support server-side decorations.")
        }
    */
    Ok(())
}
