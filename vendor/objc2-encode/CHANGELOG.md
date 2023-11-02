# Changelog

Notable changes to this crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## Unreleased - YYYY-MM-DD

## 2.0.0-pre.2 - 2022-08-28

### Added
* Added `EncodeConvert` trait to help with correctly handling `BOOL`/`bool`.

### Changed
* **BREAKING**: Remove the lifetime specifier from `Encoding`, since the non
  -`'static` version was essentially useless.

### Fixed
* Fixed the encoding output and comparison of structs behind pointers.

### Removed
* **BREAKING**: `bool` (and `AtomicBool`) no longer implements `Encode`, since
  that was difficult to use correctly. See the `EncodeConvert` trait, or use
  `objc2::runtime::Bool` instead.


## 2.0.0-pre.1 - 2022-07-19

### Added
* Added `Encoding::Atomic`.
* Implement `Encode` and `RefEncode` for `std::sync::atomic` types.

### Changed
* **BREAKING**: Renamed `Encoding::C_U_LONG` to `Encoding::C_ULONG`.


## 2.0.0-pre.0 - 2022-06-13

### Added
* Added `Encoding::C_LONG` and `Encoding::C_U_LONG` to help with platform
  compatibility; use these instead of `c_long::ENCODING` and
  `c_ulong::ENCODING`.
* Implement `Encode` and `RefEncode` for `MaybeUninit<T>`, where `T` is
  properly bound.

### Changed
* **BREAKING**: Sealed the `EncodeArguments` trait.
* **BREAKING**: Add type argument to `Encoding::BitField`.

### Removed
* **BREAKING**: Removed `PartialEq` impl between `str` and `Encoding` since it
  was incorrect (it violated the trait requirements).
* **BREAKING**: Removed `Encode` and `RefEncode` implementations for `Pin`
  since it may not be sound.


## 2.0.0-beta.2 - 2022-01-03

### Added
* Implement `Hash` for `Encoding`.

### Changed
* Improved documentation.


## 2.0.0-beta.1 - 2021-12-22

### Added
* `Encoding::equivalent_to`, `Encoding::equivalent_to_str` and
  `Encoding::equivalent_to_start_of_str` methods for more precise comparison
  semantics.
* Added `Encode` and `RefEncode` implementations for `Option` function
  pointers.

### Changed
* Discourage comparing `str` with `Encoding` using `PartialEq`. This trait
  impl might get removed in a future version.


## 2.0.0-beta.0 - 2021-11-22

### Added
* **BREAKING**: Add `Encoding::LongDouble`, `Encoding::FloatComplex`,
  `Encoding::DoubleComplex` and `Encoding::LongDoubleComplex`.
* Implement `RefEncode` for all number types that implement `Encode` (`bool`,
  `i8`, `usize`, `f32`, `NonZeroU32`, and so on).
* Implement `RefEncode` for `*const c_void` and `*mut c_void` (allowing
  `void**` in C).
* Implement `Encode` and `RefEncode` for `Wrapping<T>`, where `T` is properly
  bound.

### Changed
* **BREAKING**: Make `Encoding` `#[non_exhaustive]`. This will help us in
  evolving the API while minimizing further breaking changes.
* Discourage using `bool::ENCODING`; use `objc2::Bool::ENCODING` instead.
* Discourage using `()::ENCODING` for anything other than as a function return
  type.


## 2.0.0-alpha.1 - 2021-09-01

### Added
* Improved documentation.
* Add `RefEncode` trait, which represents types whoose pointers has an
  encoding. This means you now only have to implement `RefEncode`, and not
  both `&Encode` and `&mut Encode`.
  Additionally, encodings of pointers to pointers (to pointers, and so on) are
  now supported.
* Implement `Encode` for `NonZeroX` and `Option<NonZeroX>` integer types.
* Implement `RefEncode` for arrays.
* Implement `Encode` and `RefEncode` for (where `T` is properly bound):
  - `ManuallyDrop<T>`
  - `Pin<T>`
  - `NonNull<T>`
  - `Option<NonNull<T>>`
* Add `EncodeArguments` trait, to represent an ordered group of functions
  arguments, where each argument has an Objective-C type-encoding.
  Previously in the `objc` crate.
* Implement `Encode` and `RefEncode` for some `extern "C" fn` pointers.

### Removed
* **BREAKING**: Removed automatic `*const T: Encode` and `*mut T: Encode`
  impls when when `&T: Encode` and `&mut T: Encode` was implemented.

  Implement `T: RefEncode` instead!


## 2.0.0-alpha.0 - 2021-09-01

### Added
* Improved documentation.
* Support for targets with pointer-width 16
* Implement `Encode` for all array lengths using const-generics.
* Implement `Encode` for unsized pointer types as well.

### Changed
* **BREAKING**: Forked the project, so it is now available under the name
  `objc2-encode`.
* **BREAKING**: Changed type in `Encoding::BitField` from `u32` to `u8`.
* **BREAKING**: Changed type in `Encoding::Array` from `u32` to `usize`.
* **BREAKING**: Loosen `'static` bounds on references implementing `Encode`.


## [1.1.0] (`objc-encode` crate) - 2019-10-16

### Added
* Implement `Encode` for arrays with up to 32 elements.

### Changed
* Simplify internal encoding comparison.


## [1.0.0] (`objc-encode` crate) - 2019-03-25

### Added
* Implement `PartialEq` between `Encoding` and `&str`.

### Changed
* **BREAKING**: Make `Encoding` an enum instead of a trait, yielding a vastly
  different design. This makes use of associated constants.
* **BREAKING**: Rename `Encode::CODE` to `Encode::ENCODING`.
* Update to Rust 2018.

### Removed
* `libc` dependency.


## [0.0.3] (`objc-encode` crate) - 2017-04-30

### Fixed
* Compilation on versions prior to Rust `1.15`.


## [0.0.2] (`objc-encode` crate) - 2017-02-20

### Added
* **BREAKING**: `Display` requirement for encodings.
* Implement `PartialEq` for encodings.
* Implement `Encode` for pointers when references do.

### Fixed
* `IndexEncodingsComparator`.
* Compilation with older Rust versions.


## [0.0.1] (`objc-encode` crate) - 2017-02-19

Initial version.


[1.1.0]: https://github.com/madsmtm/objc2/compare/objc-encode-1.0.0...objc-encode-1.1.0
[1.0.0]: https://github.com/madsmtm/objc2/compare/objc-encode-0.0.3...objc-encode-1.0.0
[0.0.3]: https://github.com/madsmtm/objc2/compare/objc-encode-0.0.2...objc-encode-0.0.3
[0.0.2]: https://github.com/madsmtm/objc2/compare/objc-encode-0.0.1...objc-encode-0.0.2
[0.0.1]: https://github.com/madsmtm/objc2/releases/tag/objc-encode-0.0.1
