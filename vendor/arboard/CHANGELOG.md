# Changelog

## 3.2.1 on 2023-28-11

### Fixed
- Removed all leaks from the macOS clipboard code. Previously, both the `get` and `set` methods leaked data.
- Fixed documentation examples so that they compile on Linux.
- Removed extra whitespace macOS's HTML copying template. This caused unexpected behavior in some apps.

### Changed
- Added a timeout when connecting to the X11 server on UNIX platforms. In situations where the X11 socket is present but unusable, the clipboard
  initialization will no longer hang indefinitely.
- Removed macOS-specific dependency on the `once_cell` crate.

## 3.2.0 on 2022-04-11

### Changed
- The Windows clipboard now behaves consistently with the other
platform implementations again.
- Significantly improve cross-platform documentation of `Clipboard`.
- Remove lingering uses of the dbg! macro in the Wayland backend.

## 3.1.1 on 2022-17-10

### Added
- Implemented the ability to set HTML on the clipboard

### Changed
- Updated minimum `clipboard-win` version to `4.4`.
- Updated `wl-clipboard-rs` to the version `0.7`.

## 3.1 on 2022-20-09

### Changed
- Updated `image` to the version `0.24`.
- Lowered Wayland clipboard initialization log level.

## 3.0 on 2022-19-09

### Added
- Support for clearing the clipboard.
- Spport for excluding Windows clipboard data from cliboard history and OneDrive.
- Support waiting for another process to read clipboard data before returning
from a `write` call to a X11 and Wayland or clipboard

### Changed
- Updated `wl-clipboard-rs` to the version `0.6`.
- Updated `x11rb` to the version `0.10`.
- Cleaned up spelling in documentation
- (Breaking) Functions that used to accept `String` now take `Into<Cow<'a>, str>` instead. 
This avoids cloning the string more times then necessary on platforms that can.
- (Breaking) `Error` is now marked as `#[non_exhaustive]`.
- (Breaking) Removed all platform specific modules and clipboard structures from the public API.
If you were using these directly, the recommended replacement is using `arboard::Clipboard` and 
the new platform-specific extension traits instead.
- (Breaking) On Windows, the clipboard is now opened once per call to `Clipboard::new()` instead of on
each operation. This means that instances of `Clipboard` should be dropped once you're performed the
needed operations to prevent other applications from working with it afterwards.

## v2.1.1 on 2022-18-05

### Changed

- Fix compilation on FreeBSD
- Internal cleanup and documentation fixes
- Remove direct dependency on the `once_cell` crate.
- Fixed crates.io repository link

## v2.1.0 on 2022-09-03

### Changed

- Updated most dependencies
- Removed crate deprecation
- Fixed soundness bug in Windows clipboard

## v2.0.1 on 2021-11-05

### Changed

- On X11, re-assert clipboard ownership every time the data changes.

## v2.0.0 on 2021-08-07

### Changed

- Update dependency on yanked crate versions
- Make the image operations an optional feature

### Added

- Support selecting which linux clipboard is used

## v1.2.1 on 2021-05-04

### Changed

- Fixed a bug that caused the `set_image` function on Windows to distort the
  image colors.

## v1.2.0 on 2021-04-06

### Added

- Optional native wayland support through the `wl-clipboard-rs` crate.

## v1.1.0 on 2020-12-29

### Changed

- The `set_image` function on Windows now also provides the image in
  `CF_BITMAP` format.

## v1.0.2 on 2020-10-29

### Changed

- Fixed the clipboard contents sometimes not being preserved after the program
  exited.
