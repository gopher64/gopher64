# Changelog

Notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## Unreleased - YYYY-MM-DD

## 0.2.0-alpha.6 - 2022-08-28

### Changed
* **BREAKING**: Updated `objc2-encode` to `v2.0.0-pre.2`.
* Updated `ffi` module to `block-sys v0.1.0-beta.1`.

### Fixed
* **BREAKING**: Cleaned up `BlockArguments` trait, it is now sealed and a
  subtrait of `EncodeArguments`.
* **BREAKING**: Cleaned up `IntoConcreteBlock` trait, it is now sealed and the
  associated output type has been renamed to `Output`.

## 0.2.0-alpha.5 - 2022-07-19

### Added
* Implemented `Debug` for `Block`, `ConcreteBlock`, `RcBlock` and
  `GlobalBlock`.

### Changed
* **BREAKING**: Updated `objc2-encode` to `v2.0.0-pre.1`.
* Updated `ffi` module to `block-sys v0.1.0-beta.0`.


## 0.2.0-alpha.4 - 2022-06-13

### Changed
* **BREAKING**: Updated `objc2-encode` to `v2.0.0-pre.0`.
* **BREAKING**: Updated `ffi` module to `block-sys v0.0.4`.

### Removed
* **BREAKING**: Removed `DerefMut` implementation for `ConcreteBlock`.


## 0.2.0-alpha.3 - 2022-01-03

### Changed
* Changed `global_block!` macro to take an optional semicolon at the end.
* Improved documentation.
* **BREAKING**: Updated `ffi` module to `block-sys v0.0.3`.


## 0.2.0-alpha.2 - 2021-12-22

### Added
* `GlobalBlock` and corresponding `global_block!` macro, allowing statically
  creating blocks that don't reference their environment.

### Changed
* **BREAKING**: Updated `ffi` module to `block-sys v0.0.2`


## 0.2.0-alpha.1 - 2021-11-22

### Added
* Proper GNUStep support using `block-sys`. See that crate for usage.
* Export `block-sys` as `ffi` module.

### Removed
* Dependency on `objc_test_utils`.

### Fixed
* `ConcreteBlock` no longer allocates block descriptors on the heap.
* Better unwind safety in `ConcreteBlock::copy`.


## 0.2.0-alpha.0 - 2021-10-28

### Added
* **BREAKING**: Blocks now require that arguments and return type implement
  `objc2_encode::Encode`. This is a safety measure to prevent creating blocks
  with invalid arguments.
* Blocks now implements `objc2_encode::RefEncode` (and as such can be used in
  Objective-C message sends).
* Update to 2018 edition.

### Changed
* **BREAKING**: Forked the project, so it is now available under the name
  `block2`.

### Fixed
* Soundness issues with using empty enums over FFI.


## [0.1.6] (`block` crate) - 2016-05-08

### Added
* Support for linking to `libBlocksRuntime`.


## [0.1.5] (`block` crate) - 2016-04-04

### Changed
* Minor code changes


## [0.1.4] (`block` crate) - 2015-11-12

### Removed
* `libc` dependency.


## [0.1.3] (`block` crate) - 2015-11-07

### Changed
* Updated `libc` dependency.


## [0.1.2] (`block` crate) - 2015-10-10

### Fixed
* `improper_ctypes` warning.


## [0.1.1] (`block` crate) - 2015-09-03

### Fixed
* Missing `Sized` bounds on traits.


## [0.1.0] (`block` crate) - 2015-05-18

### Added
* `Clone` implementation for `RcBlock`.
* Improved documentation.

### Changed
* **BREAKING**: Rename `IdBlock` to `RcBlock`.
* **BREAKING**: Make `Block::call` take self immutably and make it `unsafe`.
* **BREAKING**: Make `BlockArguments::call_block` `unsafe`.

### Removed
* **BREAKING**: `DerefMut` on `RcBlock`.
* `objc` dependency.
* `Foundation` dependency in tests.


## [0.0.2] (`block` crate) - 2015-05-02

### Changed
* Use `objc_id`.


## [0.0.1] (`block` crate) - 2015-04-17

Initial version.


[0.1.6]: https://github.com/madsmtm/objc2/compare/block-0.1.5...block-0.1.6
[0.1.5]: https://github.com/madsmtm/objc2/compare/block-0.1.4...block-0.1.5
[0.1.4]: https://github.com/madsmtm/objc2/compare/block-0.1.3...block-0.1.4
[0.1.3]: https://github.com/madsmtm/objc2/compare/block-0.1.2...block-0.1.3
[0.1.2]: https://github.com/madsmtm/objc2/compare/block-0.1.1...block-0.1.2
[0.1.1]: https://github.com/madsmtm/objc2/compare/block-0.1.0...block-0.1.1
[0.1.0]: https://github.com/madsmtm/objc2/compare/block-0.0.2...block-0.1.0
[0.0.2]: https://github.com/madsmtm/objc2/compare/block-0.0.1...block-0.0.2
[0.0.1]: https://github.com/madsmtm/objc2/releases/tag/block-0.0.1
