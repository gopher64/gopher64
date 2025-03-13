# gopher64
## download

https://github.com/gopher64/gopher64/releases

## wiki

https://github.com/gopher64/gopher64/wiki

## discord

https://discord.gg/9RGXq8W8JQ

## controls

Keys are mapped according to [these defaults](https://github.com/gopher64/gopher64/wiki/Default-Keyboard-Setup). Xbox-style controllers also have a [default mapping applied](https://github.com/gopher64/gopher64/wiki/Default-Gamepad-Setup).

You can create you own mappings by running `./gopher64 --configure-input-profile my_profile`. You then bind that profile to a port: `./gopher64 --bind-input-profile my_profile --port 1`

In order to use a controller (for example, an Xbox controller), run `./gopher64 --list-controllers` to get a list of attached controllers, and then assign it by doing `./gopher64 --assign-controller <controller_number> --port 1`

## netplay

Gopher64 supports netplay (online play with others). It has a few public netplay servers. If you are interested in running a public netplay server, please let me know (open an issue or discussion, or contact me on Discord). You can also run the server (https://github.com/gopher64/gopher64-netplay-server) yourself on a LAN.

## portable mode

If you would like to keep all the game data in the same folder as executable, you just need to create a file called "portable.txt" in the same directory as the executable.

## flatpak

If you want to run the flatpak from the command line, you need to add the `--filesystem=host:ro` option, for example:

```
flatpak run --filesystem=host:ro io.github.gopher64.gopher64 /path/to/rom.z64
```

## goals

1. Performance. I want to be able to use this emulator on my laptop.
2. Easy to use.
3. Easy to work on. Dynamic recompilers perform well, but they are very hard to read and understand. This emulator will only have interpreters for the CPU and RSP. Additionally, it is completely written in Rust (besides Parallel-RDP), a modern programming language with a growing user base. I've tried to avoid the use of macros, which can reduce some repetitiveness in the code, but also reduce readability.

## building and usage

1. Linux only: install the SDL3 dependencies: https://wiki.libsdl.org/SDL3/README/linux
2. Install rust: https://www.rust-lang.org/tools/install
3. `git clone --recursive https://github.com/gopher64/gopher64.git`
4. `cd gopher64`
5. `cargo build --release`
6. `./target/release/gopher64 /path/to/rom.z64`

## contributing

I am very open to contributions! Please reach out to me via a GitHub issue, or via discord (loganmc10) before doing substantial work on a PR.

## license

Gopher64 is licensed under the GPLv3 license. Many portions of gopher64 have been adapted from mupen64plus and/or ares. The license for mupen64plus can be found here: https://github.com/mupen64plus/mupen64plus-core/blob/master/LICENSES. The license for ares can be found here: https://github.com/ares-emulator/ares/blob/master/LICENSE.
