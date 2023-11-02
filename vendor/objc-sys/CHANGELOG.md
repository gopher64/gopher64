# Changelog

Notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## Unreleased - YYYY-MM-DD

## 0.2.0-beta.2 - 2022-08-28

### Fixed
* Fixed `docs.rs` setup.


## 0.2.0-beta.1 - 2022-07-19

### Added
* Added `unstable-c-unwind` feature.
* Use `doc_auto_cfg` to improve documentation output.


## 0.2.0-beta.0 - 2022-06-13

### Changed
* **BREAKING**: Changed `links` key from `objc` to `objc_0_2` for better
  future compatibility, until we reach 1.0 (so `DEP_OBJC_CC_ARGS` in build
  scripts becomes `DEP_OBJC_0_2_CC_ARGS`).
* **BREAKING**: Apple's runtime is now always the default.

### Removed
* **BREAKING**: Removed type aliases `Class`, `Ivar`, `Method` and `Protocol`
  since they could easily be mistaken for the `objc2::runtime` structs with
  the same name.
* **BREAKING**: Removed `objc_property_t`.
* **BREAKING**: Removed `objc_hook_getClass` and `objc_hook_lazyClassNamer`
  type aliases (for now).
* **BREAKING**: Removed `DEP_OBJC_RUNTIME` build script output.


## 0.2.0-alpha.1 - 2022-01-03

### Added
* Added `objc_exception_try_enter` and `objc_exception_try_exit` on macOS x86.

### Changed
* **BREAKING**: Correctly `cfg`-guarded the following types and methods to not
  be available on macOS x86:
  - `objc_exception_matcher`
  - `objc_exception_preprocessor`
  - `objc_uncaught_exception_handler`
  - `objc_exception_handler`
  - `objc_begin_catch`
  - `objc_end_catch`
  - `objc_exception_rethrow`
  - `objc_setExceptionMatcher`
  - `objc_setExceptionPreprocessor`
  - `objc_setUncaughtExceptionHandler`
  - `objc_addExceptionHandler`
  - `objc_removeExceptionHandler`

### Removed
* **BREAKING**: Removed`objc_set_apple_compatible_objcxx_exceptions` since it
  is only available when `libobjc2` is compiled with the correct flags.
* **BREAKING**: Removed `object_setInstanceVariableWithStrongDefault` since it
  is only available since macOS 10.12.
* **BREAKING**: Removed `objc_setHook_getClass` since it is only available
  since macOS 10.14.4.
* **BREAKING**: Removed `objc_setHook_lazyClassNamer` since it is only
  available since macOS 11.

## Fixed
* `docs.rs` configuration.


## 0.2.0-alpha.0 - 2021-12-22

## Added
* `NSInteger` and `NSUInteger` (type aliases of `isize`/`usize`).
* `NSIntegerMax`, `NSIntegerMin` and `NSUIntegerMax`.

### Changed
* **BREAKING**: `cfg`-guarded `class_getImageName` to only appear on Apple
  platforms.

### Fixed
* **BREAKING**: Opaque types are now also `!UnwindSafe`.


## 0.1.0 - 2021-11-22

### Changed
* **BREAKING**: Use feature flags `apple`, `gnustep-X-Y` or `winobjc` to
  specify the runtime you're using, instead of the `RUNTIME_VERSION`
  environment variable.
* **BREAKING**: `DEP_OBJC_RUNTIME` now returns `gnustep` on WinObjC.


## 0.0.1 - 2021-10-28

Initial release.
