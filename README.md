# gopher64

```
Usage: gopher64 [OPTIONS] [GAME]

Arguments:
  [GAME]  

Options:
  -f, --fullscreen
          
  -c, --configure-input-profile <PROFILE_NAME>
          Create a new input profile (keyboard/gamepad mappings).
  -b, --bind-input-profile <PROFILE_NAME>
          Must also specify --port. Used to bind a previously created profile to a port
  -l, --list-controllers
          Lists connected controllers which can be used in --assign-controller
  -a, --assign-controller <CONTROLLER_NUMBER>
          Must also specify --port. Used to assign a controller listed in --list-controllers to a port
  -p, --port <PORT>
          Valid values: 1-4. To be used alongside --bind-input-profile and --assign-controller
  -h, --help
          Print help
  -V, --version
          Print version
```
## goals

1. Performance. I want to be able to use this emulator on my laptop.
2. Easy to use.
3. Easy to work on. Dynamic recompilers perform well, but they are very hard to read and understand. This emulator will only have interpreters for the CPU and RSP. Additionally, it is completely written in Rust (besides Parallel-RDP), a modern programming language with a growing user base. I've tried to avoid the use of macros, which can reduce some repetitiveness in the code, but also reduce readability.

## building and usage

1. Install rust: https://www.rust-lang.org/tools/install
2. `git clone --recursive https://github.com/gopher64/gopher64.git`
3. `cd gopher64`
4. `cargo build --release`
5. `./target/release/gopher64 /path/to/rom.z64`

## controls

Keys are mapped according to mupen64plus defaults: https://mupen64plus.org/wiki/index.php/KeyboardSetup#2._Default_Key_Mappings_for_SDL-Input_Plugin. Xbox-style controllers also have a default mapping applied.

You can create you own mappings by running `./gopher64 --configure-input-profile my_profile`. You then bind that profile to a port: `./gopher64 --bind-input-profile my_profile --port 1`

In order to use a controller (for example, an Xbox controller), run `./gopher64 --list-controllers` to get a list of attached controllers, and then assign it by doing `./gopher64 --assign-controller <controller_number> --port 1`

## contributing

I am very open to contributions! Please reach out to me via a GitHub issue, or via discord (loganmc10) before doing substantial work on a PR.

## license

Gopher64 is licensed under the GPLv3 license. Many portions of gopher64 have been adapted from mupen64plus and/or ares. The license for mupen64plus can be found here: https://github.com/mupen64plus/mupen64plus-core/blob/master/LICENSES. The license for ares can be found here: https://github.com/ares-emulator/ares/blob/master/LICENSE.

## where to download

Builds can be found as artifacts on [GitHub Actions](https://github.com/gopher64/gopher64/actions?query=branch%3Amain)
