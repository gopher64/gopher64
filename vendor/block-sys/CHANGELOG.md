# Changelog

Notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## Unreleased - YYYY-MM-DD

## 0.1.0-beta.1 - 2022-08-28

### Fixed
* Fixed `docs.rs` setup.


## 0.1.0-beta.0 - 2022-07-19

### Added
* Use `doc_auto_cfg` to improve documentation output.

### Changed
* **BREAKING**: Changed `links` key from `block_0_0` to `block_0_1` (so
  `DEP_BLOCK_0_0_CC_ARGS` in build scripts becomes `DEP_BLOCK_0_1_CC_ARGS`).


## 0.0.4 - 2022-06-13

### Changed
* **BREAKING**: Changed `links` key from `block` to `block_0_0` for better
  future compatibility, until we reach 1.0 (so `DEP_BLOCK_CC_ARGS` in build
  scripts becomes `DEP_BLOCK_0_0_CC_ARGS`).
* **BREAKING**: Apple's runtime is now always the default.
* **BREAKING**: Updated `objc-sys` to `v0.2.0-beta.0`.

### Fixed
* **BREAKING**: Tweak the types of a lot of fields and arguments.


## 0.0.3 - 2022-01-03

### Changed
* **BREAKING**: Updated `objc-sys` to `v0.2.0-alpha.1`.


## 0.0.2 - 2021-12-22

### Changed
* **BREAKING**: Updated `objc-sys` to `v0.2.0-alpha.0`.

### Fixed
* **BREAKING**: `Class` is now `!UnwindSafe`.


## 0.0.1 - 2021-11-22

Initial release.
