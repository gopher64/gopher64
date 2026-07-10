# gopher64
Gopher64 is a cross-platform N64 emulator. Some notable features:
* Netplay
* Homebrew support
* Upscaling
* CRT shader
* Emulate CPU overclocking
* Cheats
* Savestates
* RetroAchievements

## download

<a href="https://loganmc10.itch.io/gopher64"><img src="https://static.itch.io/images/badge.svg" width="200" ></a>

Windows:
* Standalone executable: [gopher64-windows-x86_64.exe](https://github.com/gopher64/gopher64/releases/latest/download/gopher64-windows-x86_64.exe)

MacOS:
* App Bundle: [gopher64-macos-aarch64.zip](https://github.com/gopher64/gopher64/releases/latest/download/gopher64-macos-aarch64.zip)
* Homebrew: `brew install --cask gopher64`

Linux:
* Standalone executable: [gopher64-linux-x86_64](https://github.com/gopher64/gopher64/releases/latest/download/gopher64-linux-x86_64)
* Flatpak: `flatpak install flathub io.github.gopher64.gopher64`

Android:
* APK: [gopher64-android.apk](https://github.com/gopher64/gopher64/releases/latest/download/gopher64-android.apk)

## wiki

https://github.com/gopher64/gopher64/wiki

## discord

[Discord invite link](https://discord.gg/9RGXq8W8JQ)

## controls

Keys are mapped according to [these defaults](https://github.com/gopher64/gopher64/wiki/Default-Keyboard-Setup). Xbox-style controllers also have a [default mapping applied](https://github.com/gopher64/gopher64/wiki/Default-Gamepad-Setup).

## GameCube controller (Wii U USB adapter)

Gopher64 can read GameCube controllers through the Nintendo Wii U / Switch USB GameCube adapter (and compatible clones that enumerate as VID `057E` PID `0337`), the same hardware Dolphin uses.

Setup:
* **Windows:** install the WinUSB driver with [Zadig](https://zadig.akeo.ie/). Select the adapter (`057E:0337`) and install **WinUSB** specifically (not libusbK or libusb-win32). The adapter ports only appear in the controller configuration once this driver is installed.
* **Linux:** grant access with a udev rule, e.g. put `SUBSYSTEM=="usb", ATTRS{idVendor}=="057e", ATTRS{idProduct}=="0337", MODE="0666"` in `/etc/udev/rules.d/51-gcadapter.rules`, then `sudo udevadm control --reload-rules && sudo udevadm trigger`.
* **macOS:** no driver needed.

Usage:
* In the controller configuration, assign one of the "GameCube Adapter Port 1-4" entries to an N64 port and enable that port.
* The mapping is fixed: A→A, B→B, Start→Start, D-pad→D-pad, GameCube L trigger→N64 Z, R trigger→N64 R, Z→N64 L, C-stick→C-buttons, main stick→N64 stick. The bound input profile contributes only its deadzone.
* GameCube **X** is the hotkey modifier (like the gamepad Back button): hold it with another button for save/load state and other shortcuts. Hold **X + B** to cycle the controller pak; switch to the Rumble Pak this way to feel rumble (the default pak does not rumble).
* Rumble on the official adapter requires its second (black) USB plug to be connected to a powered port; wireless WaveBird controllers never rumble.

## netplay

Gopher64 supports P2P netplay (online play with others).

Please read the [guide](https://github.com/gopher64/gopher64/wiki/Netplay-Guide) before trying out netplay.

## portable mode

If you would like to keep all the game data in the same folder as the executable, you just need to create a file called "portable.txt" in the same directory as the executable.

## flatpak

If you want to run the flatpak from the command line, you need to add the `--filesystem=host:ro` option, for example:

```
flatpak run --filesystem=host:ro io.github.gopher64.gopher64 /path/to/rom.z64
```

## building and usage

1. Linux only: [install the SDL3 dependencies](https://wiki.libsdl.org/SDL3/README-linux#build-dependencies)
2. [Install rust](https://www.rust-lang.org/tools/install)
3. `git clone --recursive https://github.com/gopher64/gopher64.git`
4. `cd gopher64`
5. `cargo build --release`
6. `./target/release/gopher64 /path/to/rom.z64`

## contributing

PRs that are vibe coded and/or co-authored by AI agents (Plan/Agent mode) are not permitted. If you are going to submit a PR, it should be scoped to a single feature/improvement.

Please contact me via a GitHub issue or Discord (loganmc10) before doing substantial work on a PR.

## license

Gopher64 is licensed under the GPLv3 license. Many portions of gopher64 have been adapted from mupen64plus and/or ares. The license for mupen64plus can be found [here](https://github.com/mupen64plus/mupen64plus-core/blob/master/LICENSES). The license for ares can be found [here](https://github.com/ares-emulator/ares/blob/master/LICENSE).

## privacy and code signing policy

Free code signing for the Windows release is provided by [SignPath.io](https://about.signpath.io), certificate by [SignPath Foundation](https://signpath.org).

During online netplay sessions, the server logs your IP address and basic session information (game title and session name) for operational purposes. No additional personal data is collected or stored.

If you enable the RetroAchievements feature, some data is sent to their systems. Please see their terms [here](https://retroachievements.org/terms).
