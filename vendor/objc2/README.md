# `objc2`

[![Latest version](https://badgen.net/crates/v/objc2)](https://crates.io/crates/objc2)
[![License](https://badgen.net/badge/license/MIT/blue)](../LICENSE.txt)
[![Documentation](https://docs.rs/objc2/badge.svg)](https://docs.rs/objc2/)
[![CI](https://github.com/madsmtm/objc2/actions/workflows/ci.yml/badge.svg)](https://github.com/madsmtm/objc2/actions/workflows/ci.yml)

Objective-C interface and bindings to the `Foundation` framework in Rust.

Most of the core libraries and frameworks that are in use on Apple systems are
written in Objective-C; this crate enables you to interract with those, and
provides ready-made bindings for the `Foundation` framework in particular.

## Example

```rust
use objc2::{class, msg_send, msg_send_id};
use objc2::ffi::NSUInteger;
use objc2::rc::{Id, Owned};
use objc2::runtime::Object;

let cls = class!(NSObject);
let obj: Id<Object, Owned> = unsafe { msg_send_id![cls, new] };

let hash: NSUInteger = unsafe { msg_send![&obj, hash] };
println!("NSObject hash: {}", hash);
```

See [the docs](https://docs.rs/objc2/) for a more thorough overview, or jump
right into the [examples].

This crate is part of the [`objc2` project](https://github.com/madsmtm/objc2),
see that for related crates.

[examples]: https://github.com/madsmtm/objc2/tree/master/objc2/examples
