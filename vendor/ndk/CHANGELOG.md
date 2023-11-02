# Unreleased

# 0.7.0 (2022-07-24)

- hardware_buffer: Make `HardwareBuffer::as_ptr()` public for interop with Vulkan. (#213)
- **Breaking:** `Configuration::country()` now returns `None` when the country is unset (akin to `Configuration::language()`). (#220)
- Add `MediaCodec` and `MediaFormat` bindings. (#216)
- **Breaking:** Upgrade to [`ndk-sys 0.4.0`](../ndk-sys/CHANGELOG.md#040-2022-07-XXXX) and use new `enum` newtype wrappers. (#245)
- native_window: Use `release`/`acquire` for `Drop` and `Clone` respectively. (#207)
- **Breaking:** audio: Rename from `aaudio` to `audio` and drop `A` prefix. (#273)
- Implement `HasRawWindowHandle` directly on `NativeWindow`. (#274, #319)
- **Breaking:** native_activity: Replace `CStr` return types with `Path`. (#279)
- native_window: Add `format()` getter and `set_buffers_geometry()` setter. (#276)
- native_activity: Add `set_window_format()` setter. (#277)
- native_activity: Add `set_window_flags()` to change window behavior. (#278)
- Add `SurfaceTexture` bindings. (#267)
- Improve library and structure documentation, linking back to the NDK docs more rigorously. (#290)
- **Breaking:** input_queue: `get_event()` now returns a `Result` with `std::io::Error`; `InputQueueError` has been removed. (#292)
- **Breaking:** input_queue: `has_events()` now returns a `bool` directly without being wrapped in `Result`. (#294)
- **Breaking:** hardware_buffer: `HardwareBufferError` has been removed and replaced with `std::io::Error` in return types. (#295)
- Fixed `HardwareBuffer` leak on buffers returned from `AndroidBitmap::get_hardware_buffer()`. (#296)
- **Breaking:** Update `jni` crate (used in public API) from `0.18` to `0.19`. (#300)
- hardware_buffer: Made `HardwareBufferDesc` fields `pub`. (#313)
- **Breaking:** Remove `hardware_buffer` and `trace` features in favour of using `api-level-26` or `api-level-23` directly. (#320)

# 0.6.0 (2022-01-05)

- **Breaking:** Upgrade to [`ndk-sys 0.3.0`](../ndk-sys/CHANGELOG.md#030-2022-01-05) and migrate to `jni-sys` types that it now directly uses in its bindings. (#209 / #214)

# 0.5.0 (2021-11-22)

- **Breaking:** Replace `add_fd_with_callback` `ident` with constant value `ALOOPER_POLL_CALLBACK`,
  as per https://developer.android.com/ndk/reference/group/looper#alooper_addfd.
- **Breaking:** Accept unboxed closure in `add_fd_with_callback`.
- aaudio: Replace "Added in" comments with missing `#[cfg(feature)]`.
- aaudio: Add missing `fn get_allowed_capture_policy()`.
- configuration: Add missing `api-level-30` feature to `fn screen_round()`.

# 0.4.0 (2021-08-02)

- **Breaking:** Model looper file descriptor events integer as `bitflags`.

# 0.3.0 (2021-01-30)

- **Breaking:** Looper `ident` not passed in `data` pointer anymore.
  `attach_looper` now only sets the `ident` field when attaching an
  `InputQueue` to a `ForeignLooper`.
  If you are relying on `Poll::Event::data` to tell event fd and
  input queue apart, please use `Poll::Event::ident` and the new
  constants introduced in `ndk-glue`!

# 0.2.1 (2020-10-15)

- Fix documentation build on docs.rs

# 0.2.0 (2020-09-15)

- **Breaking:** Updated to use [ndk-sys 0.2.0](../ndk-sys/CHANGELOG.md#020-2020-09-15)
- Added `media` bindings
- Added `bitmap` and `hardware_buffer` bindings
- Added `aaudio` bindings
- Fixed assets directory path to be relative to the manifest
- Added `trace` feature for native tracing

# 0.1.0 (2020-04-22)

- Initial release! 🎉
