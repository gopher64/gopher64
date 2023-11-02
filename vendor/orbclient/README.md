# orbclient
The Orbital Client Library. Compatible with Redox and SDL2 (on Linux and Macos).

[![Build status](https://gitlab.redox-os.org/redox-os/orbclient/badges/master/pipeline.svg)](https://gitlab.redox-os.org/redox-os/orbclient/pipelines)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![crates.io](http://meritbadge.herokuapp.com/orbclient)](https://crates.io/crates/orbclient)
[![docs.rs](https://docs.rs/orbclient/badge.svg)](https://docs.rs/orbclient)

## Dependencies
If you are *NOT* using the "bundled" feature (which is off by default) then you need SDL (sdl2)
installed on your system. 

### macos
On macos you can install the SDL2 library using `brew install sdl2`

## Features
The `"serde"`feature can be used to include code for `Color` deserialization using the `serde` crate (which is an 
optional dependency). This is not enabled by default. To enable, either build using the `--features "serde"` command
line option, or use `features = ["serde"]` in your crate, where it declares a dependency on orbclient.

The `std` feature is used to allow building `orbclient` with our without rust `std`. 
This is to enable use by some UEFI apps (e.g. System76 firmware setup, System76 firmware updater) that don't have `std`.

The `"unifont` feature (on by default is used to include the "unifont" font).

The `bundled` feature removes the need to have SDL2 installed locally. The SDL library is compiled from source
as part of the crate build and bundled with it.

### Troubleshooting

* Make sure that you work with the current ```nightly``` version of Rust
  * To make sure of that, please use [rustup](https://github.com/rust-lang-nursery/rustup.rs)
  * Don't forget to override your work directory with ```rustup override set nightly```
  * Don't forget to update the ```nightly``` version of Rust with ```rustup update nightly```
* SDL2 should be automatically with orbclient if you have trouble try to install it ```libsdl2-dev``` manually   
  * For example, with Ubuntu, please to type ```sudo apt-get install libsdl2-dev``` in your console
* On fedora please type ```sudo dnf install SDL2-devel SDL2-static``` in your console before building.
  * if during building, this message comes up ```could not find native static library `SDL2main`, perhaps an -L flag is missing?```.
   Providing the path to the static library might help. You can provide this path via ```RUSTFLAGS='-L <path-to-folder-with-libSDL2.a>' cargo b ...```.
   At the moment of writing, the SDL2 library is stored under **/usr/lib64** on fedora. In this case you would type ```RUSTFLAGS='-L /usr/lib64' cargo r --example simple``` 
   to start the simple example.
* Other problem? Do not hesitate to create a new issue!
