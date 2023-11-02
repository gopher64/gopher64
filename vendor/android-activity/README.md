# `android-activity`

[![ci](https://github.com/rust-mobile/android-activity/actions/workflows/ci.yml/badge.svg)](https://github.com/rust-mobile/android-activity/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/android-activity.svg)](https://crates.io/crates/android-activity)
[![Docs](https://docs.rs/android-activity/badge.svg)](https://docs.rs/android-activity)
[![MSRV](https://img.shields.io/badge/rustc-1.64.0+-ab6000.svg)](https://blog.rust-lang.org/2022/09/22/Rust-1.64.0.html)

## Overview

`android-activity` provides a "glue" layer for building native Rust
applications on Android, supporting multiple [`Activity`] base classes.
It's comparable to [`android_native_app_glue.c`][ndk_concepts]
for C/C++ applications and is an alternative to the [ndk-glue] crate.

`android-activity` provides a way to load your crate as a `cdylib` library via
the `onCreate` method of your Android `Activity` class; run an `android_main()`
function in a separate thread from the Java main thread and marshal events (such
as lifecycle events and input events) between Java and your native thread.

So far it supports [`NativeActivity`] or [`GameActivity`] (from the
[Android Game Development Kit][agdk]) and there's also interest in supporting a first-party
`RustActivity` base class that could be better tailored to the needs of Rust
applications.

[`Activity`]: https://developer.android.com/reference/android/app/Activity
[`NativeActivity`]: https://developer.android.com/reference/android/app/NativeActivity
[ndk_concepts]: https://developer.android.com/ndk/guides/concepts#naa
[`GameActivity`]: https://developer.android.com/games/agdk/integrate-game-activity
[ndk-glue]: https://crates.io/crates/ndk-glue
[agdk]: https://developer.android.com/games/agdk

## Example

Cargo.toml

```toml
[dependencies]
log = "0.4"
android_logger = "0.11"
android-activity = { version = "0.4", features = [ "native-activity" ] }

[lib]
crate_type = ["cdylib"]
```

_Note: that you will need to either specify the **"native-activity"** feature or **"game-activity"** feature to identify which `Activity` base class your application is based on_

lib.rs

```rust
use android_activity::{AndroidApp, InputStatus, MainEvent, PollEvent};

#[no_mangle]
fn android_main(app: AndroidApp) {
    android_logger::init_once(android_logger::Config::default().with_min_level(log::Level::Info));

    loop {
        app.poll_events(Some(std::time::Duration::from_millis(500)) /* timeout */, |event| {
            match event {
                PollEvent::Wake => { log::info!("Early wake up"); },
                PollEvent::Timeout => { log::info!("Hello, World!"); },
                PollEvent::Main(main_event) => {
                    log::info!("Main event: {:?}", main_event);
                    match main_event {
                        MainEvent::Destroy => { return; }
                        _ => {}
                    }
                },
                _ => {}
            }

            app.input_events(|event| {
                log::info!("Input Event: {event:?}");
                InputStatus::Unhandled
            });
        });
    }
}
```

```sh
rustup target add aarch64-linux-android
cargo install cargo-apk
cargo apk run
adb logcat example:V *:S
```

## Full Examples

See [this collection of examples](https://github.com/rust-mobile/rust-android-examples) (based on both `GameActivity` and `NativeActivity`).

Each example is a standalone project that may also be a convenient templates for starting a new project.

For the examples based on middleware frameworks (Winit and or Egui) they also aim to demonstrate how it's possible to write portable code that will run on Android and other systems.

## Should I use NativeActivity or GameActivity?

To learn more about the `NativeActivity` class that's shipped with Android see [here](https://developer.android.com/ndk/guides/concepts#naa).

To learn more about the `GameActivity` class that's part of the [Android Game Developer's Kit][agdk] and also see a comparison with `NativeActivity` see [here](https://developer.android.com/games/agdk/game-activity)

Generally speaking, if unsure, `NativeActivity` may be more convenient to start with since you may not need to compile/link any Java or Kotlin code.

It's expected that the `GameActivity` backend will gain more sophisticated input handling features over time (such as for supporting input via onscreen keyboards or game controllers) and only `GameActivity` is based on the [`AppCompatActivity`] subclass which you may want in some situations to help with compatibility across devices.

Even if you start out using `NativeActivity` for the convenience, it's likely that most moderately complex applications will eventually need to define their own `Activity` subclass (either subclassing `NativeActivity` or `GameActivity`) which will require compiling at least a small amount of Java or Kotlin code. This is generally due to Android's design which directs numerous events via the `Activity` class which can only be processed by overloading some associated Activity method.

## Switching from ndk-glue to android-activity

### Winit-based applications

Firstly; if you have a [Winit](https://crates.io/crates/winit) based application and also have an explicit dependency on `ndk-glue` your application will need to remove its dependency on `ndk-glue` for the 0.28 release of Winit which will be based on android-activity (Since glue crates, due to their nature, can't be compatible with alternative glue crates).

Winit-based applications can follow the [Android README](https://github.com/rust-windowing/winit#android) guidance for advice on how to switch over. Most Winit-based applications should aim to remove any explicit dependency on a specific glue crate (so not depend directly on `ndk-glue` or `android-activity` and instead rely on Winit to pull in the right glue crate). The main practical change will then be to add a `#[no_mangle]fn android_main(app: AndroidApp)` entry point.

See the [Android README](https://github.com/rust-windowing/winit#android) for more details and also see the [Winit-based examples here](https://github.com/rust-mobile/rust-android-examples).

### Middleware crates (i.e. not applications)

If you have a crate that would be considered a middleware library (for example using JNI to support access to Bluetooth, or Android's Camera APIs) then the crate should almost certainly remove any dependence on a specific glue crate because this imposes a strict compatibility constraint that means the crate can only be used by applications that use that exact same glue crate version.

Middleware libraries can instead look at using the [ndk-context](https://crates.io/crates/ndk-context) crate as a means for being able to use JNI without making any assumptions about the applications overall architecture. This way a middleware crate can work with alternative glue crates (including `ndk-glue` and `android-activity`) as well as work with embedded use cases (i.e. non-native applications that may just embed a dynamic library written in Rust to implement certain native functions).

### Other, non-Winit-based applications

The steps to switch a simple standalone application over from `ndk-glue` to `android-activity` (still based on `NativeActivity`) should be:

1. Remove `ndk-glue` from your Cargo.toml
2. Add a dependency on `android-activity`, like `android-activity = { version="0.4", features = [ "native-activity" ] }`
3. Optionally add a dependency on `android_logger = "0.11.0"`
4. Update the `main` entry point to look like this:

```rust
use android_activity::AndroidApp;

#[no_mangle]
fn android_main(app: AndroidApp) {
    android_logger::init_once(android_logger::Config::default().with_min_level(log::Level::Info));
}
```

See this minimal [NativeActivity Mainloop](https://github.com/rust-mobile/android-activity/tree/main/examples/na-mainloop) for more details about how to poll for events.

There is is no `#[ndk_glue::main]` replacement considering that `android_main()` entry point needs to be passed an `AndroidApp` argument which isn't compatible with a traditional `main()` function. Having an Android specific entry point also gives a place to initialize Android logging and handle other Android specific details (such as building an event loop based on the `app` argument)

### Design Summary / Motivation behind android-activity

Prior to working on android-activity, the existing glue crates available for building standalone Rust applications on Android were found to have a number of technical limitations that this crate aimed to solve:

1. **Support alternative Activity classes**: Prior glue crates were based on `NativeActivity` and their API precluded supporting alternatives. In particular there was an interest in the [`GameActivity`] class in conjunction with it's [`GameTextInput`] library that can facilitate onscreen keyboard support. This also allows building applications based on the standard [`AppCompatActivity`] base class which isn't possible with `NativeActivity`. Finally there was interest in paving the way towards supporting a first-party `RustActivity` that could be best tailored towards the needs of Rust applications on Android.
2. **Encapsulate IPC + synchronization between the native thread and the JVM thread**: For example with `ndk-glue` the application itself needs to avoid race conditions between the native and Java thread by following a locking convention) and it wasn't clear how this would extend to support other requests (like state saving) that also require synchronization.
3. **Avoid static global state**: Keeping in mind the possibility of supporting applications with multiple native activities there was interest in having an API that didn't rely on global statics to track top-level state. Instead of having global getters for state then `android-activity` passes an explicit `app: AndroidApp` argument to the entry point that encapsulates the state connected with a single `Activity`.

[`GameTextInput`]: https://developer.android.com/games/agdk/add-support-for-text-input
[`AppCompatActivity`]: https://developer.android.com/reference/androidx/appcompat/app/AppCompatActivity

## MSRV

We aim to (at least) support stable releases of Rust from the last three months. Rust has a 6 week release cycle which means we will support the last three stable releases.
For example, when Rust 1.69 is released we would limit our `rust_version` to 1.67.

We will only bump the `rust_version` at the point where we either depend on a new features or a dependency has increased its MSRV, and we won't be greedy. In other words we will only set the MSRV to the lowest version that's _needed_.

MSRV updates are not considered to be inherently semver breaking (unless a new feature is exposed in the public API) and so a `rust_version` change may happen in patch releases.
