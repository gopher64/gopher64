# Rust on Android

[![Rust](https://github.com/rust-windowing/android-ndk-rs/workflows/Rust/badge.svg)](https://github.com/rust-windowing/android-ndk-rs/actions) ![MIT license](https://img.shields.io/badge/License-MIT-green.svg) ![APACHE2 license](https://img.shields.io/badge/License-APACHE2-green.svg)

Libraries and tools for Rust programming on Android targets:

Name | Description | Badges
--- | --- | ---
[`ndk-sys`](./ndk-sys) | Raw FFI bindings to the NDK | [![crates.io](https://img.shields.io/crates/v/ndk-sys.svg)](https://crates.io/crates/ndk-sys) [![crates.io](https://docs.rs/ndk-sys/badge.svg)](https://docs.rs/ndk-sys)
[`ndk`](./ndk) | Safe abstraction of the bindings | [![crates.io](https://img.shields.io/crates/v/ndk.svg)](https://crates.io/crates/ndk) [![crates.io](https://docs.rs/ndk/badge.svg)](https://docs.rs/ndk)
[`ndk-context`](./ndk-context) | Android handles | [![crates.io](https://img.shields.io/crates/v/ndk-context.svg)](https://crates.io/crates/ndk-context)
[`ndk-glue`](./ndk-glue) | Startup code | [![crates.io](https://img.shields.io/crates/v/ndk-glue.svg)](https://crates.io/crates/ndk-glue) [![crates.io](https://docs.rs/ndk-glue/badge.svg)](https://docs.rs/ndk-glue)
[`ndk-build`](./ndk-build) | Everything for building apk's | [![crates.io](https://img.shields.io/crates/v/ndk-build.svg)](https://crates.io/crates/ndk-build) [![crates.io](https://docs.rs/ndk-build/badge.svg)](https://docs.rs/ndk-build)
[`cargo-apk`](./cargo-apk) | Build tool | [![crates.io](https://img.shields.io/crates/v/cargo-apk.svg)](https://crates.io/crates/cargo-apk) [![crates.io](https://docs.rs/cargo-apk/badge.svg)](https://docs.rs/cargo-apk)

See [`ndk-examples`](./ndk-examples) for examples using the NDK and the README files of the crates for more details.

## Supported NDK versions

`android-ndk-rs` aims to support at least the `LTS` and `Rolling Release` branches of the NDK, as described on [their wiki](https://github.com/android/ndk/wiki#supported-downloads). Additionally the `Beta Release` might be supported to prepare for an upcoming release.

As of writing (2021-07-24) the following NDKs are tested:

Branch | Version | Status | Working
-|-|:-:|:-:
r18 | 18.1.5063045 | _Deprecated_ | :x:
r19 | 19.2.5345600 | _Deprecated_ | :heavy_check_mark:
r20 | 20.1.5948944 | _Deprecated_ | :heavy_check_mark:
r21 | 21.4.7075529 | _Deprecated_ | :heavy_check_mark:
r22 | 22.1.7171670 | _Deprecated_ | :heavy_check_mark:
r23 | beta 1/2 | _Deprecated_ | :heavy_check_mark:
r23 | 23.0.7272597-beta3 | _Deprecated_ | :heavy_check_mark: Workaround in [#189](https://github.com/rust-windowing/android-ndk-rs/pull/189)
r23 | 23.1.7779620 | LTS | :heavy_check_mark: Workaround in [#189](https://github.com/rust-windowing/android-ndk-rs/pull/189)
r24 | 24.0.7856742-beta1 | Rolling Release | :heavy_check_mark: Workaround in [#189](https://github.com/rust-windowing/android-ndk-rs/pull/189)


## Quick start: `Hello World` crate on Android

Quick start setting up a new project with support for Android, using the `ndk-glue` layer for communicating with the Android framework through [`NativeActivity`](https://developer.android.com/reference/android/app/NativeActivity) and `cargo-apk` for packaging a crate in an Android `.apk` file.

This short guide can also be used as a reference for converting existing crates to be runnable on Android.

### 1. Install the Android NDK and SDK

Make sure the Android NDK is installed, together with a target platform (`30` by default), `build-tools` and `platform-tools`, using either the [`sdkmanager`](https://developer.android.com/studio/command-line/sdkmanager) or [Android Studio](https://developer.android.com/studio/projects/install-ndk).

### 2. Create a new library crate

```console
$ cargo new hello_world_android --lib
```

Never name your project `android` as this results in a target binary named `libandroid.so` which is also the name of Android's framework library: this will fail to link.

### 3. Configure crate for use on Android

Add the `ndk-glue` dependency to your crate.

`Cargo.toml`
```toml
# This dependency will only be included when targeting Android
[target.'cfg(target_os = "android")'.dependencies]
ndk-glue = "xxx" # Substitute this with the latest ndk-glue version you wish to use
```

Then configure the library target to be compiled to a Rust `lib` (for use in an executable on desktop) and a `cdylib` to create a native binary that can be bundled in the final `.apk` and loaded by Android.

`Cargo.toml`
```toml
[lib]
crate-type = ["lib", "cdylib"]
```

### 4. Wrap entry point with `ndk-glue`

Create a `main` function holding your code in the library portion of the crate, and wrap it in the `ndk_glue::main` attribute macro when targeting Android.

`src/lib.rs`
```rust
#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
pub fn main() {
    println!("Hello World");
}
```

See the [`ndk-macro` documentation](./ndk-macro/README.md) for more options.

Additionally, to make this crate runnable outside of Android, create a binary that calls the main function in the library.

`src/main.rs`
```rust
fn main() {
    hello_world_android::main()
}
```

As a sanity check, run this binary to make sure everything is set up correctly:

```console
$ cargo run
```

### 5. Run the crate on your Android device

Install `cargo apk` for building, running and debugging your application:
```console
$ cargo install cargo-apk
```

We can now directly execute our `Hello World` application on a real connected device or an emulator:
```console
$ cargo apk run
```

If the crate includes a runnable binary as suggested above, you will likely be greeted by the following error:

```console
$ cargo apk run
error: extra arguments to `rustc` can only be passed to one target, consider filtering
the package by passing, e.g., `--lib` or `--bin NAME` to specify a single target
Error: Command `cargo rustc --target aarch64-linux-android -- -L hello_world_android/target/cargo-apk-temp-extra-link-libraries` had a non-zero exit code.
```

To solve this, add `--lib` to the run invocation, like so:
```console
$ cargo apk run --lib
```

### 6. Inspect the output

`ndk-glue` redirects stdout and stderr to Android `logcat`, including the `println!("Hello World")` by the example above. See [Logging and stdout](##Logging-and-stdout) below how to access it.

## Rendering to the window

Android native apps have no easy access to [Android's User Interface](https://developer.android.com/guide/topics/ui) functionality (bar [JNI](##jni) interop). Applications can instead draw pixels directly to the window using [`ANativeWindow_lock`](https://developer.android.com/ndk/reference/group/a-native-window#group___a_native_window_1ga0b0e3b7d442dee83e1a1b42e5b0caee6), or use a graphics API like OpenGL or Vulkan for high performance rendering.

## Logging and stdout
Stdout is redirected to the android log api when using `ndk-glue`. Any logger that logs to
stdout, like `println!`, should therefore work.

To filter on this output in `logcat`:
```console
$ adb logcat RustStdoutStderr:D *:S
```

### Android logger
Enable the `"logger"` feature on the `ndk-glue` macro and configure its log tag and debug level through the attribute macro:

`src/lib.rs`
```rust
#[cfg_attr(target_os = "android", ndk_glue::main(logger(level = "debug", tag = "my-tag")))]
pub fn main() {
    log!("hello world");
}
```

## App/APK configuration
Android APKs contain a file called `AndroidManifest.xml`, which has things like permission requests and feature declarations, plus configuration of activities, intents, resource folders and more. This file is autogenerated by `cargo-apk`. To control what goes in it through Cargo.toml, refer to [`cargo-apk`'s README](./cargo-apk/README.md).

## Overriding crate paths
The macro `ndk_glue::main` tries to determine crate names from current _Cargo.toml_.
You can override this names with specific paths like so:
```rust
#[ndk_glue::main(
  ndk_glue = "path::to::ndk_glue",
)]
fn main() {}
```

## JNI
Java Native Interface (JNI) allows executing Java code in a VM from native applications. To access
the JNI use the `AndroidContext` from the `ndk-context` crate. `ndk-examples` contains a `jni_audio`
example which will print out all output audio devices in the log.

- [`jni`](https://crates.io/crates/jni), JNI bindings for Rust
