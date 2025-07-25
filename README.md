# gopher64
## download

Windows: https://github.com/gopher64/gopher64/releases/latest/download/gopher64-windows-x86_64.exe

Linux: https://flathub.org/apps/io.github.gopher64.gopher64

## wiki

https://github.com/gopher64/gopher64/wiki

## discord

https://discord.gg/9RGXq8W8JQ

## controls

Keys are mapped according to [these defaults](https://github.com/gopher64/gopher64/wiki/Default-Keyboard-Setup). Xbox-style controllers also have a [default mapping applied](https://github.com/gopher64/gopher64/wiki/Default-Gamepad-Setup).

## netplay

Gopher64 supports netplay (online play with others). It has a few public netplay servers. If you are interested in running a public netplay server, please let me know (open an issue or discussion or contact me on Discord). You can also run the server (https://github.com/gopher64/gopher64-netplay-server) yourself on a LAN.

## portable mode

If you would like to keep all the game data in the same folder as the executable, you just need to create a file called "portable.txt" in the same directory as the executable.

## flatpak

If you want to run the flatpak from the command line, you need to add the `--filesystem=host:ro` option, for example:

```
flatpak run --filesystem=host:ro io.github.gopher64.gopher64 /path/to/rom.z64
```

## building and usage

1. Linux only: install the SDL3 dependencies: https://wiki.libsdl.org/SDL3/README-linux#build-dependencies
2. Install rust: https://www.rust-lang.org/tools/install
3. `git clone --recursive https://github.com/gopher64/gopher64.git`
4. `cd gopher64`
5. `cargo build --release`
6. `./target/release/gopher64 /path/to/rom.z64`

## contributing

I am very open to contributions! Please contact me via a GitHub issue or Discord (loganmc10) before doing substantial work on a PR.

## license

Gopher64 is licensed under the GPLv3 license. Many portions of gopher64 have been adapted from mupen64plus and/or ares. The license for mupen64plus can be found here: https://github.com/mupen64plus/mupen64plus-core/blob/master/LICENSES. The license for ares can be found here: https://github.com/ares-emulator/ares/blob/master/LICENSE.

## privacy and code signing policy

Free code signing for the Windows release is provided by [SignPath.io](https://about.signpath.io), certificate by [SignPath Foundation](https://signpath.org).

During online netplay sessions, the server logs your IP address and basic session information (game title and session name) for operational purposes. No additional personal data is collected or stored.
