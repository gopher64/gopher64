# `objc-sys`

[![Latest version](https://badgen.net/crates/v/objc-sys)](https://crates.io/crates/objc-sys)
[![License](https://badgen.net/badge/license/MIT/blue)](../LICENSE.txt)
[![Documentation](https://docs.rs/objc-sys/badge.svg)](https://docs.rs/objc-sys/)
[![CI](https://github.com/madsmtm/objc2/actions/workflows/ci.yml/badge.svg)](https://github.com/madsmtm/objc2/actions/workflows/ci.yml)

Raw Rust bindings to the Objective-C runtime and ABI.

This crate is part of the [`objc2` project](https://github.com/madsmtm/objc2),
see that for related crates.


## Runtime Support

Objective-C has a runtime, different implementations of said runtime exist,
and they act in slightly different ways. By default, Apple platforms link to
Apple's runtime, but if you're using another runtime you must tell it to this
library using feature flags (you might have to disable the default `apple`
feature first).

One could ask, why even bother supporting other runtimes? To that, there's a
simple answer: _Robustness_. By testing with these alternative runtimes in CI,
we become by extension much more confident that our implementation doesn't
rely on brittle unspecified behaviour, and works across different macOS and
iOS versions.


### Apple's [`objc4`](https://github.com/apple-oss-distributions/objc4)

- Feature flag: `apple`.

This is used by default.

The supported runtime version (higher versions lets the compiler enable newer
optimizations, at the cost of not supporting older operating systems) can be
chosen using the standard `X_DEPLOYMENT_TARGET` environment variables:
- macOS: `MACOSX_DEPLOYMENT_TARGET`
  - Default: `10.7` ([same as Rust](https://github.com/rust-lang/rust/blob/1.56.0/compiler/rustc_target/src/spec/apple_base.rs#L67))
  - Minimum: `10.7`
- iOS: `IPHONEOS_DEPLOYMENT_TARGET`
  - Default: `7.0` ([same as Rust](https://github.com/rust-lang/rust/blob/1.56.0/compiler/rustc_target/src/spec/apple_base.rs#L92))
  - Minimum: `5.0` (theoretically)
- tvOS: `TVOS_DEPLOYMENT_TARGET`
  - Default: TODO
  - Minimum: `9.0` (theoretically)
- watchOS: `WATCHOS_DEPLOYMENT_TARGET`
  - Default: TODO
  - Minimum: `1.0` (theoretically)


### GNUStep's [`libobjc2`](https://github.com/gnustep/libobjc2)

- Feature flag: `gnustep-1-7`, `gnustep-1-8`, `gnustep-1-9`, `gnustep-2-0` and
  `gnustep-2-1` depending on the version you're using. Recommended default is
  `gnustep-1-8`.


### Microsoft's [`WinObjC`](https://github.com/microsoft/WinObjC)

- Feature flag: `unstable-winobjc`.

**Unstable: Hasn't been tested on Windows yet!**

This is essentially just [a fork](https://github.com/microsoft/libobjc2) based
on GNUStep's `libobjc2` version 1.8, with very few user-facing changes.


### [`ObjFW`](https://github.com/ObjFW/ObjFW)

- Feature flag: `unstable-objfw`.

**Unstable: Doesn't work yet!**

TODO.


### Other runtimes

This library will probably only ever support ["Modern"][modern] Objective-C
runtimes, since support for reference-counting primitives like `objc_retain`
and `objc_autoreleasePoolPop` is a vital requirement for most applications.

Just so we're being clear, this rules out the GCC [`libobjc`][gcc-libobjc]
runtime (see [this][gcc-objc-support]), the [`mulle-objc`] runtime and
[cocotron]. (But support for [`darling`] may be added).
More information on different runtimes can be found in GNUStep's
[Objective-C Compiler and Runtime FAQ][gnustep-faq].

[modern]: https://en.wikipedia.org/wiki/Objective-C#Modern_Objective-C
[gcc-libobjc]: https://github.com/gcc-mirror/gcc/tree/master/libobjc
[gcc-objc-support]: https://gcc.gnu.org/onlinedocs/gcc/Standards.html#Objective-C-and-Objective-C_002b_002b-Languages
[`mulle-objc`]: https://github.com/mulle-objc/mulle-objc-runtime
[cocotron]: https://cocotron.org/
[`darling`]: https://github.com/darlinghq/darling-objc4
[gnustep-faq]: http://wiki.gnustep.org/index.php/Objective-C_Compiler_and_Runtime_FAQ


## Advanced linking configuration

This crate defines the `links` key in `Cargo.toml` so it's possible to
change the linking to `libobjc`, see [the relevant cargo docs][overriding].

In the future, this crate may vendor the required source code to automatically
build and link to the runtimes. Choosing static vs. dynamic linking here may
also become an option.

[overriding]: https://doc.rust-lang.org/cargo/reference/build-scripts.html#overriding-build-scripts


## Objective-C Compiler configuration

Objective-C compilers like `clang` and `gcc` requires configuring the calling
ABI to the runtime you're using:
- `clang` uses the [`-fobjc-runtime`] flag, of which there are a few different
  [options][clang-objc-kinds].
- `gcc` uses the [`-fgnu-runtime` or `-fnext-runtime`][gcc-flags] options.
  Note that Modern Objective-C features are ill supported.

This is relevant if you're building and linking to custom Objective-C sources
in a build script. To assist in compiling Objective-C sources, this crate's
build script expose the `DEP_OBJC_0_2_CC_ARGS` environment variable to
downstream build scripts.

Example usage in your `build.rs` (using the `cc` crate) would be as follows:

```rust , ignore
fn main() {
    let mut builder = cc::Build::new();
    builder.compiler("clang");
    builder.file("my_objective_c_script.m");

    for flag in std::env::var("DEP_OBJC_0_2_CC_ARGS").unwrap().split(' ') {
        builder.flag(flag);
    }

    builder.compile("libmy_objective_c_script.a");
}
```

[`-fobjc-runtime`]: https://clang.llvm.org/docs/ClangCommandLineReference.html#cmdoption-clang-fobjc-runtime
[clang-objc-kinds]: https://clang.llvm.org/doxygen/classclang_1_1ObjCRuntime.html#af19fe070a7073df4ecc666b44137c4e5
[gcc-flags]: https://gcc.gnu.org/onlinedocs/gcc/Objective-C-and-Objective-C_002b_002b-Dialect-Options.html


## Design choices

It is recognized that the most primary consumer of this library will be macOS
and secondly iOS applications. Therefore it was chosen not to use `bindgen` in
our build script to not add compilation cost to those targets.<sup>1</sup>

Deprecated functions are also not included for future compability, since they
could be removed in any macOS release, and then our code would break. If you
have a need for these, please open an issue and we can discuss it!

Some items (in particular the `objc_msgSend_X` family) have `cfg`s that prevent
their usage on different platforms; these are **semver-stable** in the sense
that they will only get less restrictive, never more.

<sup>1: That said, most of this is created with the help of `bindgen`'s
commandline interface, so huge thanks to them!</sup>
