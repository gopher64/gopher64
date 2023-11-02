<!-- markdownlint-disable MD022 MD024 MD032  -->

# Changelog
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.3] - 2022-07-30
### Fixed
- Fixed a deadlock in the `native-activity` backend while waiting for the native thread after getting an `onDestroy` callback from Java ([#94](https://github.com/rust-mobile/android-activity/pull/94))

- Fixed numerous deadlocks in the `game-activity` backend with how it would wait for the native thread in various Java callbacks, after the app has returned from `android_main` ([#98](https://github.com/rust-mobile/android-activity/pull/98))

## [0.4.2] - 2022-06-17
### Changed
- The `Activity.finish()` method is now called when `android_main` returns so the `Activity` will be destroyed ([#67](https://github.com/rust-mobile/android-activity/issues/67))
- The `native-activity` backend now propagates `NativeWindow` redraw/resize and `ContentRectChanged` callbacks to main loop ([#70](https://github.com/rust-mobile/android-activity/pull/70))
- The `game-activity` implementation of `pointer_index()` was fixed to not always return `0` ([#80](https://github.com/rust-mobile/android-activity/pull/84))
- Added `panic` guards around application's `android_main()` and native code that could potentially unwind across a Java FFI boundary ([#68](https://github.com/rust-mobile/android-activity/pull/68))

## [0.4.1] - 2022-02-16
### Added
- Added `AndroidApp::vm_as_ptr()` to expose JNI `JavaVM` pointer ([#60](https://github.com/rust-mobile/android-activity/issues/60))
- Added `AndroidApp::activity_as_ptr()` to expose Android `Activity` JNI reference as pointer ([#60](https://github.com/rust-mobile/android-activity/issues/60))
### Changed
- Removed some overly-verbose logging in the `native-activity` backend ([#49](https://github.com/rust-mobile/android-activity/pull/49))
### Removed
- Most of the examples were moved to https://github.com/rust-mobile/rust-android-examples ([#50](https://github.com/rust-mobile/android-activity/pull/50))

## [0.4] - 2022-11-10
### Changed
- *Breaking*: `input_events` callback now return whether an event was handled or not to allow for fallback handling ([#31](https://github.com/rust-mobile/android-activity/issues/31))
- The native-activity backend is now implemented in Rust only, without building on `android_native_app_glue.c` ([#35](https://github.com/rust-mobile/android-activity/pull/35))
### Added
- Added `Pointer::tool_type()` API to `GameActivity` backend for compatibility with `ndk` events API ([#38](https://github.com/rust-mobile/android-activity/pull/38))

## [0.3] - 2022-09-15
### Added
- `show/hide_sot_input` API for being able to show/hide a soft keyboard (other IME still pending)
- `set_window_flags()` API for setting WindowManager params
### Changed
- *Breaking*: Created extensible, `#[non_exhaustive]` `InputEvent` wrapper enum instead of exposing `ndk` type directly

## [0.2] - 2022-08-25
### Added
- Emit an `InputAvailable` event for new input with `NativeActivity` and `GameActivity`
  enabling gui apps that don't render continuously
- Oboe and Cpal audio examples added
- `AndroidApp` is now `Send` + `Sync`
### Changed
- *Breaking*: updates to `ndk 0.7` and `ndk-sys 0.4`
- *Breaking*: `AndroidApp::config()` now returns a clonable `ConfigurationRef` instead of a deep `Configuration` copy
### Removed
- The `NativeWindowRef` wrapper struct was removed since `NativeWindow` now implements `Clone` and `Drop` in `ndk 0.7`
- *Breaking*: The `FdEvent` and `Error` enum values were removed from `PollEvents`

## [0.1.1] - 2022-07-04
### Changed
- Documentation fixes

## [0.1] - 2022-07-04
### Added
- Initial release