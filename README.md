# gopher64

## download

* **Windows:** [Download](https://github.com/gopher64/gopher64/releases/latest/download/gopher64-windows-x86_64.exe)
* **Linux:** [Flathub](https://flathub.org/apps/io.github.gopher64.gopher64)

## wiki

[gopher64 Wiki](https://github.com/gopher64/gopher64/wiki)

## discord

[Join the Discord](https://discord.gg/9RGXq8W8JQ)

## controls

Keys are mapped according to [these defaults](https://github.com/gopher64/gopher64/wiki/Default-Keyboard-Setup). Xbox-style controllers also have a [default mapping applied](https://github.com/gopher64/gopher64/wiki/Default-Gamepad-Setup).

## netplay

Gopher64 supports netplay (online play with others) via cloud hosted servers. You can also run the server ([server code](https://github.com/gopher64/gopher64-netplay-server)) yourself on a LAN.

## portable mode

If you would like to keep all the game data in the same folder as the executable, you just need to create a file called "portable.txt" in the same directory as the executable.

## flatpak

If you want to run the flatpak from the command line, you need to add the `--filesystem=host:ro` option, for example:

```sh
flatpak run --filesystem=host:ro io.github.gopher64.gopher64 /path/to/rom.z64
````

## building and usage

1. **Windows/Linux:**

    a) **Prerequisites:**

      * **Linux only:** Install the SDL3 dependencies: [SDL3 Linux Dependencies](https://wiki.libsdl.org/SDL3/README-linux#build-dependencies)

      * Install Rust (including Cargo): [Install Rust](https://www.rust-lang.org/tools/install)

      * Install `cargo-bundle`:

        ```sh
        cargo install cargo-bundle
        ```

    b) **Clone the repository and navigate to it:**

    ```sh
    git clone --recursive https://github.com/gopher64/gopher64.git && cd gopher64
    ```

    c) **Build the project:**

    ```sh
    cargo build --release
    ```

    d) **Run the emulator with your ROM:**

    ```sh
    ./target/release/gopher64 /path/to/rom.z64
    ```

    e) **(Optional) Create application bundle:**

    ```sh
    cargo bundle --release
    ```

2. **macOS:**

    a) **Install prerequisites:**

      * Install Xcode Command Line Tools:

        ```sh
        xcode-select --install
        ```

      * Install Homebrew:

        ```sh
        /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
        ```

      * Install Rust (including Cargo):

        ```sh
        brew install rust
        ```

      * Install `cargo-bundle`:

        ```sh
        cargo install cargo-bundle
        ```

      * Install required dependencies**

        ```sh
        brew install llvm pkg-config freetype sdl3 sdl3_ttf
        ```

    b) **Clone the repository and navigate to it:**

    ```sh
    git clone --recursive https://github.com/gopher64/gopher64.git && cd gopher64
    ```

    c) **Build the project:**

    ```sh
    cargo build --release
    ```

    d) **Run the emulator with a ROM:**

    ```sh
    ./target/release/gopher64 /path/to/rom.z64
    ```

    e) **(Optional) Create .app bundle:**

    ```sh
    cargo bundle --release
    ```

    *The finished .app bundle will be in `target/release/bundle/osx/Gopher64.app` and you can move it to Applications folder using `mv /target/release/bundle/osx/gopher64.app /Applications`.*

## contributing

I am very open to contributions\! Please contact me via a GitHub issue or Discord (loganmc10) before doing substantial work on a PR.

## license

Gopher64 is licensed under the GPLv3 license. Many portions of gopher64 have been adapted from mupen64plus and/or ares. The license for mupen64plus can be found here: [mupen64plus license](https://github.com/mupen64plus/mupen64plus-core/blob/master/LICENSES). The license for ares can be found here: [ares license](https://github.com/ares-emulator/ares/blob/master/LICENSE).

## privacy and code signing policy

Free code signing for the Windows release is provided by [SignPath.io](https://about.signpath.io), certificate by [SignPath Foundation](https://signpath.org).

During online netplay sessions, the server logs your IP address and basic session information (game title and session name) for operational purposes. No additional personal data is collected or stored.
