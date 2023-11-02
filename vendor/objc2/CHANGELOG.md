# Changelog

Notable changes to this crate will be documented in this file. See the
`CHANGELOG_FOUNDATION.md` file for changes to the `objc2::foundation` module!

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## Unreleased - YYYY-MM-DD

## 0.3.0-beta.3 - 2022-09-01

### Added
* Added `Ivar::write`, `Ivar::as_ptr` and `Ivar::as_mut_ptr` for safely
  querying and modifying instance variables inside `init` methods.
* Added `IvarDrop<T>` to allow storing complex `Drop` values in ivars
  (currently `rc::Id<T, O>`, `Box<T>`, `Option<rc::Id<T, O>>` or
  `Option<Box<T>>`).
* **BREAKING**: Added required `ClassType::NAME` constant for statically
  determining the name of a specific class.
* Allow directly specifying class name in declare_class! macro.

### Removed
* **BREAKING**: `MaybeUninit` no longer implements `IvarType` directly; use
  `Ivar::write` instead.


## 0.3.0-beta.2 - 2022-08-28

### Added
* Added the `"unstable-static-class"` and `"unstable-static-class-inlined"`
  feature flags to make the `class!` macro zero cost.
* Moved the external crate `objc2_foundation` into `objc2::foundation` under
  (default) feature flag `"foundation"`.
* Added `declare_class!`, `extern_class!` and `ns_string!` macros from
  `objc2-foundation`.
* Added helper method `ClassBuilder::add_static_ivar`.
* **BREAKING**: Added `ClassType` trait, and moved the associated `class`
  methods that `extern_class!` and `declare_class!` generated to that. This
  means you'll have to `use objc2::ClassType` whenever you want to use e.g.
  `NSData::class()`.
* Added `Id::into_super`.
* Added `extern_methods!` macro.
* Added ability to call `msg_send![super(obj), ...]` without explicitly
  specifying the superclass.
* Added automatic conversion of `bool` to/from the Objective-C `BOOL` in
  `msg_send!`, `msg_send_id!`, `extern_methods!` and `declare_class!`.

  Example:
  ```rust
  // Before
  use objc2::{msg_send, msg_send_bool};
  use objc2::rc::{Id, Shared};
  use objc2::runtime::{Bool, Object};

  let obj: Id<Object, Shared>;
  let _: () = unsafe { msg_send![&obj, setArg: Bool::YES] };
  let is_equal = unsafe { msg_send_bool![&obj, isEqual: &*obj] };

  // After
  use objc2::msg_send;
  use objc2::rc::{Id, Shared};
  use objc2::runtime::Object;

  let obj: Id<Object, Shared>;
  let _: () = unsafe { msg_send![&obj, setArg: true] };
  let is_equal: bool = unsafe { msg_send![&obj, isEqual: &*obj] };
  ```

### Changed
* **BREAKING**: Change syntax in `extern_class!` macro to be more Rust-like.
* **BREAKING**: Change syntax in `declare_class!` macro to be more Rust-like.
* **BREAKING**: Renamed `Id::from_owned` to `Id::into_shared`.
* **BREAKING**: The return type of `msg_send_id!` is now more generic; it can
  now either be `Option<Id<_, _>>` or `Id<_, _>` (if the latter, it'll simply
  panic).

  Example:
  ```rust
  // Before
  let obj: Id<Object, Shared> = unsafe {
      msg_send_id![msg_send_id![class!(MyObject), alloc], init].unwrap()
  };

  // After
  let obj: Id<Object, Shared> = unsafe {
      msg_send_id![msg_send_id![class!(MyObject), alloc], init]
  };
  ```
* Updated `ffi` module to `objc-sys v0.2.0-beta.2`.
* **BREAKING**: Updated `encode` module `objc2-encode v2.0.0-pre.2`.

  In particular, `Encoding` no longer has a lifetime parameter:
  ```rust
  // Before
  #[repr(C)]
  pub struct NSRange {
      pub location: usize,
      pub length: usize,
  }
  unsafe impl Encode for NSRange {
      const ENCODING: Encoding<'static> = Encoding::Struct(
          "_NSRange", // This is how the struct is defined in C header files
          &[usize::ENCODING, usize::ENCODING]
      );
  }
  unsafe impl RefEncode for NSRange {
      const ENCODING_REF: Encoding<'static> = Encoding::Pointer(&Self::ENCODING);
  }

  // After
  #[repr(C)]
  pub struct NSRange {
      pub location: usize,
      pub length: usize,
  }
  unsafe impl Encode for NSRange {
      const ENCODING: Encoding = Encoding::Struct(
          "_NSRange", // This is how the struct is defined in C header files
          &[usize::ENCODING, usize::ENCODING]
      );
  }
  unsafe impl RefEncode for NSRange {
      const ENCODING_REF: Encoding = Encoding::Pointer(&Self::ENCODING);
  }
  ```

### Deprecated
* Depreacted `msg_send_bool!` in favour of new functionality on `msg_send!`
  that allows seamlessly handling `bool`.


## 0.3.0-beta.1 - 2022-07-19

### Added
* Added `msg_send_id!` to help with following Objective-C's memory management
  rules. **It is highly recommended that you use this instead of doing memory
  management yourself!**

  Example:
  ```rust
  // Before
  let obj: Id<Object, Shared> = unsafe {
      let obj: *mut Object = msg_send![class!(MyObject), alloc];
      let obj: *mut Object = msg_send![obj, init];
      Id::new(obj).unwrap()
  };

  // After
  let obj: Id<Object, Shared> = unsafe {
      msg_send_id![msg_send_id![class!(MyObject), alloc], init].unwrap()
  };
  ```
* Added the `"unstable-static-sel"` and `"unstable-static-sel-inlined"`
  feature flags to make the `sel!` macro (and by extension, the `msg_send!`
  macros) faster.
* Added `"unstable-c-unwind"` feature.
* Added unsafe function `Id::cast` for converting between different types of
  objects.
* Added `Object::ivar_ptr` to allow direct access to instance variables
  through `&Object`.
* Added `VerificationError` as more specific return type from
  `Class::verify_sel`.
* Added `rc::Allocated` struct which is used within `msg_send_id!`.
* Added `Class::responds_to`.
* Added `exception::Exception` object to improve error messages from caught
  exceptions.
* Added `declare::Ivar<T>` helper struct. This is useful for building safe
  abstractions that access instance variables.
* Added `Id::from_owned` helper function.

### Changed
* **BREAKING**: `Sel` is now required to be non-null, which means that you
  have to ensure that any selectors you receive from method calls are
  non-null before using them.
* **BREAKING**: `ClassBuilder::root` is now generic over the function pointer,
  meaning you will have to coerce initializer functions to pointers like in
  `ClassBuilder::add_method` before you can use it.
* **BREAKING**: Moved `MessageReceiver::verify_message` to `Class::verify_sel`
  and changed return type.
* Improved debug output with `verify_message` feature enabled.
* **BREAKING**: Changed `MessageReceiver::send_message` to panic instead of
  returning an error.
* **BREAKING**: Renamed `catch_all` feature to `catch-all`.
* **BREAKING**: Made passing the function pointer argument to
  `ClassBuilder::add_method`, `ClassBuilder::add_class_method` and similar
  more ergonomic.

  Let's say you have the following code:
  ```rust
  // Before
  let init: extern "C" fn(&mut Object, Sel) -> *mut Object = init;
  builder.add_method(sel!(init), init);
  ```

  Unfortunately, you will now encounter a very confusing error:
  ```
    |
  2 | builder.add_method(sel!(init), init);
    |         ^^^^^^^^^^ implementation of `MethodImplementation` is not general enough
    |
     = note: `MethodImplementation` would have to be implemented for the type `for<'r> extern "C" fn(&'r mut Object, Sel) -> *mut Object`
     = note: ...but `MethodImplementation` is actually implemented for the type `extern "C" fn(&'0 mut Object, Sel) -> *mut Object`, for some specific lifetime `'0`
  ```

  Fret not, the fix is easy! Just let the compiler infer the argument and
  return types:
  ```rust
  // After
  let init: extern "C" fn(_, _) -> _ = init;
  builder.add_method(sel!(init), init);
  ```
* Updated `ffi` module to `objc-sys v0.2.0-beta.1`.
* **BREAKING**: Updated `encode` module `objc2-encode v2.0.0-pre.1`.

### Fixed
* **BREAKING**: Disallow throwing `nil` exceptions in `exception::throw`.

### Removed
* **BREAKING**: Removed the `Sel::from_ptr` method.
* **BREAKING**: Removed `MessageError`.


## 0.3.0-beta.0 - 2022-06-13

### Added
* Added deprecated `Object::get_ivar` and `Object::get_mut_ivar` to make
  upgrading easier.
* Allow using `From`/`TryFrom` to convert between `rc::Id` and `rc::WeakId`.
* Added `Bool::as_bool` (more descriptive name than `Bool::is_true`).
* Added convenience method `Id::as_ptr` and `Id::as_mut_ptr`.
* The `objc2-encode` dependency is now exposed as `objc2::encode`.
* Added `Id::retain_autoreleased` to allow following Cocoas memory management
  rules more efficiently.
* Consistently allow trailing commas in `msg_send!`.
* Added `msg_send_bool!`, a less error-prone version of `msg_send!` for
  Objective-C methods that return `BOOL`.
* Implemented `MethodImplementation` for `unsafe` function pointers.

### Changed
* **BREAKING**: Changed signature of `Id::new` and `Id::retain` from
  `fn(NonNull<T>) -> Id<T>` to `fn(*mut T) -> Option<Id<T>>`.

  Concretely, you will have to change your code as follows.
  ```rust
  // Before
  let obj: *mut Object = unsafe { msg_send![class!(NSObject), new] };
  let obj = NonNull::new(obj).expect("Failed to allocate object.");
  let obj = unsafe { Id::new(obj) };
  // After
  let obj: *mut Object = unsafe { msg_send![class!(NSObject), new] };
  let obj = unsafe { Id::new(obj) }.expect("Failed to allocate object.");
  ```
* Allow specifying any receiver `T: Message` for methods added with
  `ClassBuilder::add_method`.
* Renamed `ClassDecl` and `ProtocolDecl` to `ClassBuilder` and
  `ProtocolBuilder`. The old names are kept as deprecated aliases.
* **BREAKING**: Changed how `msg_send!` works wrt. capturing its arguments.

  This will require changes to your code wherever you used `Id`, for example:
  ```rust
  // Before
  let obj: Id<Object, Owned> = ...;
  let p: i32 = unsafe { msg_send![obj, parameter] };
  let _: () = unsafe { msg_send![obj, setParameter: p + 1] };
  // After
  let mut obj: Id<Object, Owned> = ...;
  let p: i32 = unsafe { msg_send![&obj, parameter] };
  let _: () = unsafe { msg_send![&mut obj, setParameter: p + 1] };
  ```

  Notice that we now clearly pass `obj` by reference, and therein also
  communicate the mutability of the object (in the first case, immutable, and
  in the second, mutable).

  If you previously used `*mut Object` or `&Object` as the receiver, message
  sending should work exactly as before.
* **BREAKING**: `Class` no longer implements `Message` (but it can still be
  used as the receiver in `msg_send!`, so this is unlikely to break anything
  in practice).
* **BREAKING**: Sealed the `MethodImplementation` trait, and made its `imp`
  method privat.
* **BREAKING**: Updated `ffi` module to `objc-sys v0.2.0-beta.0`.
* **BREAKING**: Updated `objc2-encode` (`Encoding`, `Encode`, `RefEncode` and
  `EncodeArguments`) to `v2.0.0-pre.0`.

### Fixed
* Properly sealed the `MessageArguments` trait (it already had a hidden
  method, so this is not really a breaking change).

### Removed
* **BREAKING**: `ManuallyDrop` no longer implements `Message` directly.
* **BREAKING**: `MessageReceiver::as_raw_receiver` is no longer public.


## 0.3.0-alpha.6 - 2022-01-03

### Added
* Implement `Hash` for `Sel`, `Ivar`, `Class`, `Method` and `MessageError`.
* Implement `PartialEq` and `Eq` for `Ivar`, `Method` and `MessageError`.
* Implement `fmt::Pointer` for `Sel` and `rc::AutoreleasePool`.
* Implement `fmt::Debug` for `ClassDecl`, `ProtocolDecl` and `rc::AutoreleasePool`.

### Changed
* **BREAKING**: Renamed:
  - `Object::get_ivar` -> `Object::ivar`
  - `Object::get_mut_ivar` -> `Object::ivar_mut`
* Vastly improved documentation.
* **BREAKING**: Updated `ffi` module to `objc-sys v0.2.0-alpha.1`.
* **BREAKING**: Updated `objc2-encode` (`Encoding`, `Encode`, `RefEncode` and
  `EncodeArguments`) to `v2.0.0-beta.2`.


## 0.3.0-alpha.5 - 2021-12-22

### Added
* Export `objc-sys` as `ffi` module.
* Added common trait impls on `rc::Owned` and `rc::Shared` (useful in generic
  contexts).
* Implement `RefEncode` for `runtime::Protocol`.
* Added `Message` and `MessageReceiver` implementation for `ManuallyDrop<T>`
  (where `T` is appropriately bound). This allows patterns like:
  ```rust
  let obj = Id::new(msg_send![class!(MyObject), alloc]);
  let obj = ManuallyDrop::new(obj);
  // `init` takes ownership and possibly returns a new object.
  let obj = Id::new(msg_send![obj, init]);
  ```
* New cargo feature `"malloc"`, which allows cutting down on dependencies,
  most crates don't need the introspection features that this provides.

### Changed
* Deprecated `runtime::BOOL`, `runtime::YES` and `runtime::NO`. Use the
  newtype `Bool` instead, or low-level `ffi::BOOL`, `ffi::YES` and `ffi::NO`.
* **BREAKING**: The following methods now require the new `"malloc"` feature
  flag to be enabled:
  - `MessageReceiver::verify_message` (temporarily)
  - `Method::return_type`
  - `Method::argument_type`
  - `Class::classes`
  - `Class::instance_methods`
  - `Class::adopted_protocols`
  - `Class::instance_variables`
  - `Protocol::protocols`
  - `Protocol::adopted_protocols`
* Relaxed `Sized` bound on `rc::Id` and `rc::WeakId` to prepare for
  `extern type` support.
* **BREAKING**: Relaxed `Sized` bound on `rc::SliceId` and `rc::DefaultId`.
* **BREAKING**: Updated `objc-sys` to `v0.2.0-alpha.0`.
* Updated `objc2-encode` (`Encoding`, `Encode`, `RefEncode` and
  `EncodeArguments`) to `v2.0.0-beta.1`.

### Removed
* **BREAKING**: Removed the raw FFI functions from the `runtime` module. These
  are available in the new `ffi` module instead.

### Fixed
* An issue with inlining various `rc` methods.
* Most types (e.g. `Class` and `Method`) are now `Send`, `Sync`, `UnwindSafe`
  and `RefUnwindSafe` again.
  Notable exception is `Object`, because that depends on the specific
  subclass.


## 0.3.0-alpha.4 - 2021-11-22

Note: To use this version, specify `objc2-encode = "=2.0.0-beta.0"` in your
`Cargo.toml` as well.

### Added
* **BREAKING**: GNUStep users must depend on, and specify the appropriate
  feature flag on `objc-sys` for the version they're using.
* Moved `objc_exception` crate into `exception` module (under feature flag).
* Added support for `_Complex` types.
* Added `rc::SliceId`, `rc::SliceIdMut` and `rc::DefaultId` helper traits for
  extra functionality on `rc::Id`.

### Changed
* **BREAKING**: The `exception` feature now just enables the `exception`
  module for general use. Use the new `catch_all` feature to wrap all message
  sends in a `@try/@catch`.
* **BREAKING**: Updated `objc-sys` to `v0.1.0`.
* **BREAKING**: Updated `objc2-encode` (`Encoding`, `Encode`, `RefEncode` and
  `EncodeArguments`) to `v2.0.0-beta.0`.


## 0.3.0-alpha.3 - 2021-09-05

Note: To use this version, specify `objc2-encode = "=2.0.0-alpha.1"` in your
`Cargo.toml` as well.

### Added
* Now uses the `objc-sys` (`v0.0.1`) crate for possibly better
  interoperability with other crates that link to `libobjc`.
* Added newtype `runtime::Bool` to fix soundness issues with using
  `runtime::BOOL` or `bool`.
* Moved `objc_id` crate into `rc` module. Notable changes:
  - Vastly improved documentation
  - Added `Id::autorelease`
  - Added `Id::from_shared`
  - Added a lot of forwarding implementations on `Id` for easier use
  - `Id` and `WeakId` are now able to use the null-pointer optimization
  - **BREAKING**: Added `T: Message` bounds on `Id`
  - **BREAKING**: Remove `ShareId` type alias
  - **BREAKING**: `Id` no longer have a default `Ownership`, you must specify
    it everywhere as either `Id<T, Shared>` or `Id<T, Owned>`
  - **BREAKING**: Sealed the `Ownership` trait
  - **BREAKING**: Renamed `Id::from_ptr` to `Id::retain`
  - **BREAKING**: Renamed `Id::from_retained_ptr` to `Id::new`
  - **BREAKING**: Changed `Id::share` to a `From` implementation (usage of
    `obj.share()` can be changed to `obj.into()`)
  - **BREAKING**: Fixed soundness issues with missing `Send` and `Sync` bounds
    on `Id` and `WeakId`
* Added sealed (for now) trait `MessageReceiver` to specify types that can
  be used as the receiver of a message (instead of only allowing pointer
  types).
* Add `MessageReceiver::send_super_message` method for dynamic selectors.

### Changed
* **BREAKING**: Change types of parameters to FFI functions exported in the
  `runtime` module.
* **BREAKING**: Most types are now `!UnwindSafe`, to discourage trying to use
  them after an unwind. This restriction may be lifted in the future.
* **BREAKING**: Most types are now `!Send` and `!Sync`. This was an oversight
  that is fixed in a later version.
* A lot of smaller things.
* **BREAKING**: Dynamic message sending with `Message::send_message` is moved
  to `MessageReceiver`.
* **BREAKING** Make `MessageArguments` a subtrait of `EncodeArguments`.
* Allow an optional comma after each argument to `msg_send!`.

### Removed
* **BREAKING**: Removed `rc::StrongPtr`. Use `Option<rc::Id<Object, Shared>>`
  instead (beware: This has stronger safety invariants!).
* **BREAKING**: Removed `rc::WeakPtr`. Use `rc::WeakId<Object>` instead.

### Fixed
* **BREAKING**: Stop unsafely dereferencing `msg_send!`s first argument. The
  `MessageReceiver` trait was introduced to avoid most breakage from this
  change.


## 0.3.0-alpha.2 - 2021-09-05

### Added
* Added `rc::AutoreleasePool` and `rc::AutoreleaseSafe` to make accessing
  autoreleased objects safe, by binding references to it using the
  `ptr_as_ref` and `ptr_as_mut` methods.

### Changed
* **BREAKING**: The closure in `rc::autoreleasepool` now takes an argument
  `&rc::AutoreleasePool`. This reference can be given to functions like
  `INSString::as_str` so that it knows which lifetime to bound the returned
  `&str` with.

  Simple migration:
  ```rust
  // Change
  autoreleasepool(|| {
      // Some code that autoreleases objects
  });
  // To
  autoreleasepool(|_pool| {
      // Some code that autoreleases objects
  });
  ```

### Fixed
* The encoding of `BOOL` on `GNUStep`.


## 0.3.0-alpha.1 - 2021-09-02

### Added
* More documentation of safety requirements, and in general.

### Changed
* **BREAKING**: Change `objc-encode` dependency to `objc2-encode` version
  `2.0.0-alpha.1`, and re-export the new `RefEncode` trait from that.
* **BREAKING**: Require that the receiver, arguments and return types of
  messages always implement `Encode`. This helps ensuring that only types made
  to go across the FFI boundary (`repr(C)`, ...) may. These requirements were
  already present when the `verify_message` feature was enabled.

  This is a very _disruptive change_, since libraries are now required to
  implement `Encode` and `RefEncode` for all types intended to go across the
  FFI-boundary to Objective-C. The change is justified because it helps
  ensuring that users only pass valid types to `msg_send!` (for example, this
  prevents users from accidentally passing `Drop` types to `msg_send`).

  See the following examples for how to implement these traits, and otherwise
  refer to the documentation of `objc2-encode` (`v2.0.0-alpha.1` or above).
  ```rust
  use objc2::{Encode, Encoding, RefEncode};

  /// Example struct.
  #[repr(C)]
  pub struct NSRange {
      pub location: usize,
      pub length: usize,
  }
  unsafe impl Encode for NSRange {
      const ENCODING: Encoding<'static> = Encoding::Struct(
          "_NSRange", // This is how the struct is defined in C header files
          &[usize::ENCODING, usize::ENCODING]
      );
  }
  unsafe impl RefEncode for NSRange {
      const ENCODING_REF: Encoding<'static> = Encoding::Pointer(&Self::ENCODING);
  }

  /// Example object.
  #[repr(C)]
  pub struct __CFString(c_void);

  pub type CFStringRef = *const __CFString;

  unsafe impl RefEncode for __CFString {
      const ENCODING_REF: Encoding<'static> = Encoding::Object;
  }
  ```
* Temporarily disabled iOS tests.

### Fixed
* Statically find the correct `objc_msgSend[_X]` function to use based on the
  `Encode` implementation of the return type. This fixes using functions that
  return e.g. `type CGFloat = f32 / f64;`.
* Documentation links.


## 0.3.0-alpha.0 - 2021-08-29

Note: This is the version that is, as of this writing, available on the
`master` branch in the original `objc` project.

### Added
* Improve macro hygiene.
  ```rust
  // You can now do
  use objc2::{sel, class, msg_send};
  // Instead of
  #[macro_use]
  extern crate objc2;
  ```
* Update to Rust 2018.
* Other internal improvements.

### Changed
* **BREAKING**: Forked the project, so the crate name is now `objc2`.
* **BREAKING**: Updated encoding utilities to use `objc-encode`. See that for
  how to use the updated type `Encoding` and trait `Encode`.

  In short, you will likely need to change your implementations of `Encode`
  like this:
  ```rust
  use objc2::{Encode, Encoding};

  pub type CGFloat = ...; // Varies based on target_pointer_width

  #[repr(C)]
  pub struct NSPoint {
      pub x: CGFloat,
      pub y: CGFloat,
  }

  // Before
  unsafe impl Encode for NSPoint {
      fn encode() -> Encoding {
          let encoding = format!(
              "{{CGPoint={}{}}}",
              CGFloat::encode().as_str(),
              CGFloat::encode().as_str(),
          );
          unsafe { Encoding::from_str(&encoding) }
      }
  }

  // After
  unsafe impl Encode for NSPoint {
      const ENCODING: Encoding<'static> = Encoding::Struct(
          "CGPoint",
          &[CGFloat::ENCODING, CGFloat::ENCODING]
      );
  }
  ```
* **BREAKING**: Updated public dependency `malloc_buf` to `1.0`.
* **BREAKING**: `Method::return_type` and `Method::argument_type` now return
  `Malloc<str>` instead of `Encoding`.
* **BREAKING**: `Ivar::type_encoding` now return `&str` instead of `Encoding`.

### Removed
* **BREAKING**: Removed hidden `sel_impl!` macro.


## [0.2.7] (`objc` crate) - 2019-10-19

### Fixed
* **BREAKING**: Uses of `msg_send!` will now correctly fail to compile if no
  return type can be inferred, instead of relying on an edge case of the
  compiler that will soon change and silently cause undefined behavior.


## [0.2.6] (`objc` crate) - 2019-03-25

### Fixed
* Suppressed a deprecation warning in `sel!`, `msg_send!`, and `class!`.


## [0.2.5] (`objc` crate) - 2018-07-24

### Added
* **BREAKING**: `autoreleasepool` returns the value returned by its body
  closure.


## [0.2.4] (`objc` crate) - 2018-07-22

### Added
* Added an `rc` module with reference counting utilities:
  `StrongPtr`, `WeakPtr`, and `autoreleasepool`.
* Added some reference counting ABI foreign functions to the `runtime` module.

### Fixed
* Messaging nil under GNUstep now correctly returns zeroed results for all
  return types.


## [0.2.3] (`objc` crate) - 2018-07-07

### Added
* Added a `class!` macro for getting statically-known classes. The result is
  non-optional (avoiding a need to unwrap) and cached so each usage will only
  look up the class once.
* Added caching to the `sel!` macro so that each usage will only register the
  selector once.

### Fixed
* Implementation of `objc2::runtime` structs so there can't be unsound
  references to uninhabited types.


## [0.2.2] (`objc` crate) - 2016-10-30

### Added
* Implemented `Sync` and `Send` for `Sel`.


## [0.2.1] (`objc` crate) - 2016-04-23

### Added
* Added support for working with protocols with the `Protocol` struct.
  The protocols a class conforms to can be examined with the new
  `Class::adopted_protocols` and `Class::conforms_to` methods.
* Protocols can be declared using the new `ProtocolDecl` struct.


## [0.2.0] (`objc` crate) - 2016-03-20

### Added
* Added verification for the types used when sending messages.
  This can be enabled for all messages with the `"verify_message"` feature,
  or you can test before sending specific messages with the
  `Message::verify_message` method. Verification errors are reported using the
  new `MessageError` struct.
* Added support for the GNUstep runtime!
  Operating systems besides OSX and iOS will fall back to the GNUstep runtime.
* Root classes can be declared by using the `ClassDecl::root` constructor.

### Changed
* **BREAKING**: C types are now used from `std::os::raw` rather than `libc`.
  This means `Encode` may not be implemented for `libc` types; switch them to
  the `std::os::raw` equivalents instead. This avoids an issue that would
  arise from simultaneously using different versions of the libc crate.
* **BREAKING**: Dynamic messaging was moved into the `Message` trait; instead
  of `().send(obj, sel!(description))`, use
  `obj.send_message(sel!(description), ())`.
* **BREAKING**: Rearranged the parameters to `ClassDecl::new` for consistency;
  instead of `ClassDecl::new(superclass, "MyObject")`, use
  `ClassDecl::new("MyObject", superclass)`.
* **BREAKING**: Overhauled the `MethodImplementation` trait. Encodings are now
  accessed through the `MethodImplementation::Args` associated type. The
  `imp_for` method was replaced with `imp` and no longer takes a selector or
  returns an `UnequalArgsError`, although `ClassDecl::add_method` still
  validates the number of arguments.
* **BREAKING**: Updated the definition of `Imp` to not use the old dispatch
  prototypes. To invoke an `Imp`, it must first be transmuted to the correct
  type.

### Removed
  **BREAKING**: `objc_msgSend` functions from the `runtime` module; the
  availability of these functions varies and they shouldn't be called without
  trasmuting, so they are now hidden as an implementation detail of messaging.

### Fixed
* Corrected alignment of ivars in `ClassDecl`; declared classes may now have a
  smaller size.
* With the `"exception"` or `"verify_message"` feature enabled, panics from
  `msg_send!` will now be triggered from the line and file where the macro is
  used, rather than from within the implementation of messaging.

## [0.1.8] (`objc` crate) - 2015-11-06

### Changed
* Updated `libc` dependency.


## [0.1.7] (`objc` crate) - 2015-09-23

### Fixed
* `improper_ctypes` warning.


## [0.1.6] (`objc` crate) - 2015-08-08

### Added
* Added `"exception"` feature which catches Objective-C exceptions and turns
  them into Rust panics.
* Added support for `ARM`, `ARM64` and `x86` architectures.
* **BREAKING**: Added `Any` bound on message return types. In practice this
  probably won't break anything.
* Start testing on iOS.


## [0.1.5] (`objc` crate) - 2015-05-02

### Changed
* **BREAKING**: Renamed `IntoMethodImp` to `MethodImplementation`.
* **BREAKING**: Renamed `MethodImplementation::into_imp` to `::imp_for`.
* **BREAKING**: Relaxed `Sized` bounds on `Encode` and `Message`. In practice
  this probably won't break anything.

### Removed
* **BREAKING**: Removed `Id`, `Owned`, `Ownership`, `Shared`, `ShareId` and
  `WeakId`. Use them from the `objc_id` crate instead.
* **BREAKING**: Removed `Method::set_implementation` and
  `Method::exchange_implementation`.


## [0.1.4] (`objc` crate) - 2015-04-17

### Removed
* **BREAKING**: Removed `block` module. Use them from the `block` crate
  instead.


## [0.1.3] (`objc` crate) - 2015-04-11

### Added
* Implement `fmt::Pointer` for `Id`.

### Fixed
* Odd lifetime bug.


## [0.1.2] (`objc` crate) - 2015-04-04

### Fixed
* **BREAKING**: Replace uses of `PhantomFn` with `Sized`.


## [0.1.1] (`objc` crate) - 2015-03-27

### Added
* Implement `Error` for `UnequalArgsError`.

### Removed
* **BREAKING**: Move `objc::foundation` into new crate `objc_foundation`.


[0.2.7]: https://github.com/madsmtm/objc2/compare/objc-0.2.6...objc-0.2.7
[0.2.6]: https://github.com/madsmtm/objc2/compare/objc-0.2.5...objc-0.2.6
[0.2.5]: https://github.com/madsmtm/objc2/compare/objc-0.2.4...objc-0.2.5
[0.2.4]: https://github.com/madsmtm/objc2/compare/objc-0.2.3...objc-0.2.4
[0.2.3]: https://github.com/madsmtm/objc2/compare/objc-0.2.2...objc-0.2.3
[0.2.2]: https://github.com/madsmtm/objc2/compare/objc-0.2.1...objc-0.2.2
[0.2.1]: https://github.com/madsmtm/objc2/compare/objc-0.2.0...objc-0.2.1
[0.2.0]: https://github.com/madsmtm/objc2/compare/objc-0.1.8...objc-0.2.0
[0.1.8]: https://github.com/madsmtm/objc2/compare/objc-0.1.7...objc-0.1.8
[0.1.7]: https://github.com/madsmtm/objc2/compare/objc-0.1.6...objc-0.1.7
[0.1.6]: https://github.com/madsmtm/objc2/compare/objc-0.1.5...objc-0.1.6
[0.1.5]: https://github.com/madsmtm/objc2/compare/objc-0.1.4...objc-0.1.5
[0.1.4]: https://github.com/madsmtm/objc2/compare/objc-0.1.3...objc-0.1.4
[0.1.3]: https://github.com/madsmtm/objc2/compare/objc-0.1.2...objc-0.1.3
[0.1.2]: https://github.com/madsmtm/objc2/compare/objc-0.1.1...objc-0.1.2
[0.1.1]: https://github.com/madsmtm/objc2/compare/objc-0.1.0...objc-0.1.1
