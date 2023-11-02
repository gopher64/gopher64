# Changelog

Changes to the `objc2::foundation` module will be documented in this file.
This previously existed as a separate crate `objc2_foundation`, hence the
separation.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## Unreleased - YYYY-MM-DD

## objc2 0.3.0-beta.3 - 2022-09-01

### Added
* Added `NSSet`.
* Added `NSMutableSet`.
* Added `NSMutableDictionary`.
* Added `NSNotFound`.
* Added `NSBundle`.
* Added `NSTimeInterval`.
* Added `NSString::len_utf16` and `NSAttributedString::len_utf16`.
* Added `NSString::concat` and `NSString::join_path`.
* Added `CGSize`, `CGPoint` and `CGRect` (just aliases to equivalent
  `NS`-types, but helps readability).

### Changed
* **BREAKING**: `NSSize::new` no longer requires it's arguments to be
  non-negative. Use `NSSize::abs` or `NSRect::standardize` if the API you're
  binding to requires a non-negative size.


## objc2 0.3.0-beta.2 - 2022-08-28

### Added
* Added `NSNumber`.
* Added `NSError`.
* Implement `UnwindSafe` and `RefUnwindSafe` for all objects.
* Implemented `IntoIterator` for references to `NSArray`, `NSMutableArray`,
  `NSData` and `NSMutableData`.
* Implemented `Extend` for `NSMutableArray`.
* Add extra `Extend<&u8>` impl for `NSMutableData`.
* Added function `NSValue::contains_encoding` for determining if the encoding
  of the `NSValue` matches the encoding of the given type.
* Added functions `get_range`, `get_point`, `get_size` and `get_rect` to
  `NSValue` to help safely returning various types it will commonly contain.
* `NSArray` and `NSMutableArray` now have sensible defaults for the ownership
  of the objects they contain.

### Changed
* **BREAKING**: Moved from external crate `objc2_foundation` into
  `objc2::foundation`.
* **BREAKING**: Made `NSValue` not generic any more. While we loose some
  type-safety from this, it makes `NSValue` much more useful in the real
  world!
* **BREAKING**: Made `NSArray::new` generic over ownership.
* **BREAKING**: Made `NSObject::is_kind_of` take a generic `T: ClassType`
  instead of a `runtime::Class`.

### Fixed
* Made `Debug` impls for all objects print something useful.

### Removed
* `NSObject::hash_code`, `NSObject::is_equal` and `NSObject::description` in
  favour of just having the trait implementations `Hash`, `PartiqalEq` and
  `Debug`.


## objc2-foundation 0.2.0-alpha.6 - 2022-07-19

### Added
* Added `MainThreadMarker` to help with designing APIs where a method is only
  safe to call on the main thread.
* Added `NSException` object.
* Added `extern_class!` macro to help with defining interfaces to classes.
  Further changelog for this can be found in `CHANGELOG.md`.
* Added `declare_class!` macro to help with declaring custom classes.
  Further changelog for this can be found in `CHANGELOG.md`.
* Expose the `objc2` version that this uses in the crate root.
* Added `NSZone`.

### Changed
* Changed a few `Debug` impls.


## objc2-foundation 0.2.0-alpha.5 - 2022-06-13

### Added
* Objects now `Deref` to their superclasses. E.g. `NSMutableArray` derefs to
  `NSArray`, which derefs to `NSObject`, which derefs to `Object`.

  This allows more ergonomic usage.
* Implement `PartialOrd` and `Ord` for `NSString` and `NSRange`.
* Added `NSString::has_prefix` and `NSString::has_suffix`.
* Added `NSRange` methods `new`, `is_empty`, `contains` and `end`.
* Added `NSThread` object.
* Added `is_multi_threaded` and `is_main_thread` helper functions.
* Added `NSProcessInfo` object.
* Added `NSMutableData` methods `from_data`, `with_capacity` and `push`.
* Added `io::Write` and `iter::Extend` implementation for `NSMutableData`.
* Added `NSUUID` object.
* Added `NSMutableString` object.
* Added basic `NSAttributedString` object.
* Added basic `NSMutableAttributedString` object.
* Added `NSInteger` and `NSUInteger` (type aliases to `isize` and `usize`).
* Added `CGFloat`.
* Added `NSPoint`.
* Added `NSSize`.
* Added `NSRect`.
* Implement `Borrow` and `BorrowMut` for all objects.
* Implement `ToOwned` for copyable types.

### Changed
* **BREAKING**: Removed the following helper traits in favor of inherent
  methods on the objects themselves:
  - `INSMutableArray`
  - `INSArray`
  - `INSMutableData`
  - `INSData`
  - `INSDictionary`
  - `INSString`
  - `INSValue`
  - `INSObject`

  This changed because objects now deref to their superclasses.
* **BREAKING**: Relaxed a lot of bounds from `INSObject` to `Message`. At some
  point in the future a new trait will be introduced which remedies this
  change.
* **BREAKING**: Removed the `I` prefix from:
  - `INSCopying` (now `NSCopying`)
  - `INSMutableCopying` (now `NSMutableCopying`)
  - `INSFastEnumeration` (now `NSFastEnumeration`)
* **BREAKING**: Renamed `NSMutableData::append` to `extend_from_slice`.


## 0.2.0-alpha.4 - 2022-01-03

### Added
* Implement `PartialOrd` and `Ord` for `NSComparisonResult` and `NSValue`.
* Implement `fmt::Display` for `NSValue`.
* Implement `DefaultId` for relevant objects.
* Implement `AsRef` and `Index` for `NSData` and `NSMutableData`.
* Implement `AsMut` and `IndexMut` for `NSMutableData`.

### Changed
* **BREAKING**: Renamed `INSFastEnumeration::enumerator` to
  `INSFastEnumeration::iter_fast`.

### Removed
* **BREAKING**: Removed `Deref` and `DerefMut` from `NSData` and
  `NSMutableData`, since these invoke a non-trivial amount of code, and could
  easily lead to hard-to-diagnose performance issues.


## objc2-foundation 0.2.0-alpha.3 - 2021-12-22

### Added
* **BREAKING**: Added associated `Ownership` type to `NSCopying`.
* **BREAKING**: Added associated `Ownership` type to `INSData`.
* **BREAKING**: Added associated `Ownership` type to `INSArray`.
* Added common trait impls (`PartialEq`, `Eq`, `Hash` and `Debug`) to
  `NSValue`, `NSDictionary`, `NSArray` and `NSMutableArray`.

### Changed
* **BREAKING**: Made some creation methods a bit less generic (e.g.
  `INSDictionary::from_keys_and_objects` now always returns `Id<_, Shared>`).
* Relax bounds on generic `INSObject` impls.

### Removed
* **BREAKING**: Removed associated `Ownership` type from `INSObject`; instead,
  it is present on the types that actually need it (for example `NSCopying`).
* **BREAKING**: Removed `Sized` bound on `INSObject`.

### Fixed
* Soundness issue with `NSValue`, `NSDictionary`, `NSArray` and
  `NSMutableArray` not being `#[repr(C)]`.
* **BREAKING**: `NSObject` is no longer `Send` and `Sync` (because its
  subclasses may not be).


## objc2-foundation 0.2.0-alpha.2 - 2021-11-22

### Added
* **BREAKING**: Added associated `Ownership` type to `INSObject` to specify
  whether the type can be mutated or not. `NSString` is a prime example of a
  type that you may never get a `Owned/&mut` reference to, since it is very
  easy to create two `NSString`s with the same underlying allocation.
* Added helper `is_empty` methods.
* Added `INSArray::first_mut` and `INSArray::last_mut`.

### Changed
* **BREAKING**: Renamed a lot of methods to better match Rusts naming scheme:
  - `INSArray`
    - `count` -> `len`
    - `object_at` -> `get`
    - `mut_object_at` -> `get_mut`
    - `shared_object_at` -> `get_retained`
    - `first_object` -> `first`
    - `last_object` -> `last`
    - `object_enumerator` -> `iter`
  - `INSMutableArray`
    - `add_object` -> `push`
    - `insert_object_at` -> `insert`
    - `replace_object_at` -> `replace`
    - `remove_object_at` -> `remove`
    - `remove_last_object` -> `pop`
    - `remove_all_objects` -> `clear`
  - `INSDictionary`
    - `count` -> `len`
    - `object_for` -> `get`
    - `key_enumerator` -> `iter_keys`
    - `object_enumerator` -> `iter_values`
  - `INSValue`
    - `value` -> `get`
    - `from_value` -> `new`
  - `NSComparisonResult`
    - `from_ordering` -> `from`
    - `as_ordering` -> `into`
  - `NSRange`
    - `from_range` -> `from`
    - `as_range` -> `into`
* Use `SliceId` for better performance when creating arrays and dictionaries.

### Removed
* **BREAKING**: Removed the `object_struct!` macro. It may be re-added in
  another form in the future.
* **BREAKING**: Removed `NSMutableSharedArray<T>` and `NSSharedArray<T>` type
  aliases. Use `NSMutableArray<T, Shared>` and `NSArray<T, Shared>` instead.
* **BREAKING**: Removed `Any / 'static` bound on `INSObject`. This allows
  implementing it for objects that contain lifetimes from the outer scope.

### Fixed
* **BREAKING**: Marked `INS...` traits as `unsafe` to implement.
* **BREAKING**: Removed `new` method from `INSObject` since some classes don't
  want this called. It has been re-added to other `INS...` traits on a case by
  case basis (in particular not `NSValue`).
* **BREAKING**: `INSString::as_str` now takes an a reference to
  `objc2::rc::AutoreleasePool`. This ensure that the returned `&str` is only
  used while the current autorelease pool is valid.
* Fixed `NSData::from_vec` on GNUStep.


## objc2-foundation 0.2.0-alpha.1 - 2021-10-28

### Added
* Implement new `RefEncode` trait for objects.
* Implement `Encode` for `NSComparisonResult` and `NSFastEnumerationState`.
* Implement `RefEncode` for objects and `NSFastEnumerationState`.

### Changed
* **BREAKING**: Uses `Id` from `objc2::rc` module instead of `objc_id` crate.
* **BREAKING**: `INSValue::encoding` now returns `&str` instead of `Encoding`.

### Fixed
* Use proper `#[repr(C)]` structs to represent Objective-C objects.
* `INSString::from_str` on GNUStep (`UTF8_ENCODING` was the wrong type).


## objc2-foundation 0.2.0-alpha.0 - 2021-08-29

Note: This is the version that is, as of this writing, available on the
`master` branch in the original `objc-foundation` project.

### Added
* Implement `Display` for `NSString`.
* Make `INSObject::class` faster using the `objc::class!` macro.

### Changed
* **BREAKING**: Forked the project, the crate name is now `objc2-foundation`.

### Fixed
* Fixed types in various calls to `objc::msg_send!` for better verification.


## objc-foundation [0.1.1] - 2016-06-19

### Fixed
* An issue with passing functions (instead of function pointers) in
  `INSMutableArray::sort_by`.


## objc-foundation [0.1.0] - 2016-03-20

### Changed
* Update `objc` to `v0.2`.
* Update `objc_id` to `v0.1`.


## objc-foundation [0.0.4] - 2015-12-09

### Removed
* `libc` dependency.


## objc-foundation [0.0.3] - 2015-11-07

### Added
* `object_struct!` macro.

### Changed
* `libc` version can both be `0.1` and `0.2`.


## objc-foundation [0.0.2] - 2015-09-03

### Added
* `Any` bound on `INSObject`, because of a change in `objc` `v0.1.6`.


## objc-foundation [0.0.1] - 2015-06-13

Initial release.


[0.1.1]: https://github.com/madsmtm/objc2/compare/objc-foundation-0.1.0...objc-foundation-0.1.1
[0.1.0]: https://github.com/madsmtm/objc2/compare/objc-foundation-0.0.4...objc-foundation-0.1.0
[0.0.4]: https://github.com/madsmtm/objc2/compare/objc-foundation-0.0.3...objc-foundation-0.0.4
[0.0.3]: https://github.com/madsmtm/objc2/compare/objc-foundation-0.0.2...objc-foundation-0.0.3
[0.0.2]: https://github.com/madsmtm/objc2/compare/objc-foundation-0.0.1...objc-foundation-0.0.2
[0.0.1]: https://github.com/madsmtm/objc2/releases/tag/objc-foundation-0.0.1
