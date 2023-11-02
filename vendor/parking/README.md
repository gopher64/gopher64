# parking

[![Build](https://github.com/smol-rs/parking/workflows/Build%20and%20test/badge.svg)](
https://github.com/smol-rs/parking/actions)
[![License](https://img.shields.io/badge/license-Apache--2.0_OR_MIT-blue.svg)](
https://github.com/smol-rs/parking)
[![Cargo](https://img.shields.io/crates/v/parking.svg)](
https://crates.io/crates/parking)
[![Documentation](https://docs.rs/parking/badge.svg)](
https://docs.rs/parking)

Thread parking and unparking.

A parker is in either notified or unnotified state. Method `park()` blocks
the current thread until the parker becomes notified and then puts it back into unnotified
state. Method `unpark()` puts it into notified state.

## Examples

```rust
use std::thread;
use std::time::Duration;
use parking::Parker;

let p = Parker::new();
let u = p.unparker();

// Notify the parker.
u.unpark();

// Wakes up immediately because the parker is notified.
p.park();

thread::spawn(move || {
    thread::sleep(Duration::from_millis(500));
    u.unpark();
});

// Wakes up when `u.unpark()` notifies and then goes back into unnotified state.
p.park();
```

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

#### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
